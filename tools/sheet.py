"""Sheet builder: places library symbols, wires, labels and power ports and
writes a KiCad 10 compatible .kicad_sch (format 20250114; the project then
normalises it with `kicad-cli sch upgrade`)."""
from kisch import (q, f, snap, suid, effects, lib_to_sch, pin_dir_sch)

POWER_NETS = {'+15V', '+5V', '+3V3', '+3V3_A', '+3V3_OSC', '+1V8', '+1V8_Q', 'GND'}
# rotation of a power symbol for a given outward direction (schematic vector)
PWR_ROT = {(0, -1): 0, (-1, 0): 90, (0, 1): 180, (1, 0): 270}
GND_ROT = {(0, 1): 0, (1, 0): 90, (0, -1): 180, (-1, 0): 270}
LBL_ROT = {(1, 0): (0, 'left'), (-1, 0): (180, 'right'), (0, -1): (90, 'left'), (0, 1): (270, 'right')}


class Inst:
    def __init__(self, sheet, key, sym, unit, x, y, ang, mirror=None):
        self.sheet, self.key, self.sym, self.unit = sheet, key, sym, unit
        self.x, self.y, self.ang, self.mirror = snap(x), snap(y), ang, mirror
        self.fields = {}          # name -> (value, x, y, justify, hide, size)
        self.ref = None           # filled in by annotation
        self.value = ''
        self.footprint = ''
        self.extra = {}           # extra visible/hidden properties
        self.dnp = False
        self.in_bom = sym.in_bom
        self.ref_pos = None
        self.val_pos = None
        self.pinmap = {}          # new pin -> net (filled later)

    def pin_xy(self, num):
        u, p = self.sym.all_pins()[str(num)]
        assert u == self.unit, f'{self.key}: pin {num} belongs to unit {u}, not {self.unit}'
        return lib_to_sch(self.x, self.y, self.ang, p.x, p.y, self.mirror)

    def pin_out(self, num):
        u, p = self.sym.all_pins()[str(num)]
        return pin_dir_sch(p, self.ang, self.mirror)

    def pins(self):
        return [p.num for p in self.sym.units[self.unit]['pins']]


class Sheet:
    def __init__(self, name, file, paper, title, page, uuid_key):
        self.name, self.file, self.paper, self.title, self.page = name, file, paper, title, page
        self.uuid = suid('sheet:' + uuid_key)
        self.insts = []
        self.wires = []           # [(x1,y1,x2,y2)]
        self.labels = []          # (kind, net, x, y, dir)
        self.powers = []          # (net, x, y, dir)
        self.flags = []           # (x, y, dir) PWR_FLAG
        self.ncs = []             # (x, y)
        self.texts = []           # (s, x, y, size, justify)
        self.graphics = []        # raw sexpr strings (polylines, rectangles)
        self.sheet_syms = []      # hierarchical sheet symbols (on root)
        self.counter = 0
        self.wired_pins = set()   # (key, pin) that are explicitly wired
        self.done_pins = set()    # pins already terminated (label/power/nc)
        self.resolve = None       # text placeholder resolver {KEY} -> designator

    # ------------------------------------------------------------ placing --
    def place(self, key, sym, x, y, ang=0, unit=1, mirror=None):
        i = Inst(self, key, sym, unit, x, y, ang, mirror)
        self.insts.append(i)
        return i

    def wire(self, *pts):
        pts = [(snap(a), snap(b)) for a, b in pts]
        for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
            if (x1, y1) != (x2, y2):
                assert x1 == x2 or y1 == y2, f'non-orthogonal wire {pts}'
                self.wires.append((x1, y1, x2, y2))

    def connect(self, a, pa, b, pb, via=None, first='h'):
        """Wire pin pa of inst a to pin pb of inst b with an L or straight path."""
        x1, y1 = a.pin_xy(pa)
        x2, y2 = b.pin_xy(pb)
        self.wired_pins.update({(a.key, str(pa)), (b.key, str(pb))})
        if via:
            self.wire((x1, y1), *via, (x2, y2))
        elif x1 == x2 or y1 == y2:
            self.wire((x1, y1), (x2, y2))
        elif first == 'h':
            self.wire((x1, y1), (x2, y1), (x2, y2))
        else:
            self.wire((x1, y1), (x1, y2), (x2, y2))

    def pin_wire(self, inst, pin, *pts):
        """Wire from a pin through the given points."""
        x, y = inst.pin_xy(pin)
        self.wired_pins.add((inst.key, str(pin)))
        self.wire((x, y), *pts)
        return pts[-1] if pts else (x, y)

    def stub(self, inst, pin, length=2.54):
        x, y = inst.pin_xy(pin)
        dx, dy = inst.pin_out(pin)
        ex, ey = x + dx * length, y + dy * length
        if length:
            self.wire((x, y), (ex, ey))
        self.wired_pins.add((inst.key, str(pin)))
        return (snap(ex), snap(ey)), (dx, dy)

    def power(self, net, x, y, d):
        self.powers.append((net, snap(x), snap(y), d))

    def label(self, net, x, y, d, kind='label'):
        self.labels.append((kind, net, snap(x), snap(y), d))

    def flag(self, x, y, d=(0, -1)):
        self.flags.append((snap(x), snap(y), d))

    def nc(self, inst, pin):
        x, y = inst.pin_xy(pin)
        self.ncs.append((x, y))
        self.done_pins.add((inst.key, str(pin)))

    def text(self, s, x, y, size=1.778, justify='left'):
        self.texts.append((s, x, y, size, justify))

    def line(self, pts, width=0.3):
        p = ' '.join(f'(xy {f(a)} {f(b)})' for a, b in pts)
        self.graphics.append(f'(polyline (pts {p}) (stroke (width {f(width)}) (type default)) '
                             f'(uuid "{self.uid()}"))')

    def uid(self):
        self.counter += 1
        return suid(f'{self.name}:obj:{self.counter}')

    # ----------------------------------------------------------- finishing --
    def split_wires(self):
        """Split wires where an endpoint/pin lies on another wire's interior."""
        pts = set()
        for x1, y1, x2, y2 in self.wires:
            pts.add((x1, y1)); pts.add((x2, y2))
        for i in self.insts:
            for p in i.pins():
                pts.add(i.pin_xy(p))
        for _, _, x, y, _ in self.labels:
            pts.add((x, y))
        for _, x, y, _ in self.powers:
            pts.add((x, y))
        for x, y, _ in self.flags:
            pts.add((x, y))
        out = []
        for x1, y1, x2, y2 in self.wires:
            cuts = [(x1, y1), (x2, y2)]
            for (px, py) in pts:
                if x1 == x2 == px and min(y1, y2) < py < max(y1, y2):
                    cuts.append((px, py))
                elif y1 == y2 == py and min(x1, x2) < px < max(x1, x2):
                    cuts.append((px, py))
            cuts = sorted(set(cuts))
            for a, b in zip(cuts, cuts[1:]):
                out.append((a[0], a[1], b[0], b[1]))
        # de-duplicate (including reversed copies)
        seen, res = set(), []
        for w in out:
            k = tuple(sorted([(w[0], w[1]), (w[2], w[3])]))
            if k not in seen:
                seen.add(k); res.append(w)
        self.wires = res

    def junctions(self):
        deg = {}
        for x1, y1, x2, y2 in self.wires:
            for p in ((x1, y1), (x2, y2)):
                deg[p] = deg.get(p, 0) + 1
        for i in self.insts:
            for p in i.pins():
                xy = i.pin_xy(p)
                if xy in deg:
                    deg[xy] += 1
        return sorted(p for p, n in deg.items() if n >= 3)

    # ------------------------------------------------------------- output --
    def sexpr(self, project, root_uuid, libs, title_block):
        self.split_wires()
        out = [f'(kicad_sch (version 20250114) (generator "eeschema") (generator_version "9.0")',
               f'(uuid "{self.uuid}")', f'(paper "{self.paper}")', title_block]
        used = {}
        for i in self.insts:
            used[i.sym.name] = i.sym
        for net, *_ in self.powers:
            used[net] = libs[net]
        if self.flags:
            used['PWR_FLAG'] = libs['PWR_FLAG']
        out.append('(lib_symbols ' + ' '.join(s.sexpr('DDS') for s in used.values()) + ')')
        for x, y in self.junctions():
            out.append(f'(junction (at {f(x)} {f(y)}) (diameter 0) (color 0 0 0 0) (uuid "{self.uid()}"))')
        for x, y in self.ncs:
            out.append(f'(no_connect (at {f(x)} {f(y)}) (uuid "{self.uid()}"))')
        for x1, y1, x2, y2 in self.wires:
            out.append(f'(wire (pts (xy {f(x1)} {f(y1)}) (xy {f(x2)} {f(y2)})) '
                       f'(stroke (width 0) (type default)) (uuid "{self.uid()}"))')
        out += self.graphics
        for s, x, y, size, j in self.texts:
            if self.resolve:
                s = self.resolve(s)
            out.append(f'(text {q(s)} (exclude_from_sim no) (at {f(x)} {f(y)} 0) '
                       f'{effects(size, j + " top")} (uuid "{self.uid()}"))')
        for kind, net, x, y, d in self.labels:
            ang, j = LBL_ROT[d]
            if kind == 'global':
                out.append(f'(global_label {q(net)} (shape bidirectional) (at {f(x)} {f(y)} {ang}) '
                           f'(fields_autoplaced yes) {effects(1.27, j)} (uuid "{self.uid()}") '
                           f'(property "Intersheetrefs" "${{INTERSHEET_REFS}}" (at {f(x)} {f(y)} 0) '
                           f'{effects(1.0, hide=True)}))')
            else:
                out.append(f'(label {q(net)} (at {f(x)} {f(y)} {ang}) {effects(1.27, j + " bottom")} '
                           f'(uuid "{self.uid()}"))')
        path = f'/{root_uuid}' if self.page == '1' else f'/{root_uuid}/{self.sheet_path_uuid}'
        npwr = 0
        for net, x, y, d in self.powers:
            npwr += 1
            rot = (GND_ROT if net == 'GND' else PWR_ROT)[d]
            ref = f'#PWR{self.page}{npwr:03d}'
            sym = libs[net]
            vx, vy = self._power_value_pos(net, x, y, d)
            out.append(
                f'(symbol (lib_id "DDS:{net}") (at {f(x)} {f(y)} {rot}) (unit 1) (exclude_from_sim no) '
                f'(in_bom no) (on_board yes) (dnp no) (uuid "{self.uid()}") '
                f'(property "Reference" {q(ref)} (at {f(x)} {f(y)} 0) {effects(1.27, hide=True)}) '
                f'(property "Value" {q(net)} (at {f(vx)} {f(vy)} 0) '
                f'{effects(1.27, hide=(net == "GND"))}) '
                f'(property "Footprint" "" (at {f(x)} {f(y)} 0) {effects(1.27, hide=True)}) '
                f'(property "Datasheet" "" (at {f(x)} {f(y)} 0) {effects(1.27, hide=True)}) '
                f'(pin "1" (uuid "{self.uid()}")) '
                f'(instances (project {q(project)} (path {q(path)} (reference {q(ref)}) (unit 1)))))')
        nflg = 0
        for x, y, d in self.flags:
            nflg += 1
            ref = f'#FLG{self.page}{nflg:02d}'
            rot = PWR_ROT[d]
            out.append(
                f'(symbol (lib_id "DDS:PWR_FLAG") (at {f(x)} {f(y)} {rot}) (unit 1) (exclude_from_sim no) '
                f'(in_bom no) (on_board yes) (dnp no) (uuid "{self.uid()}") '
                f'(property "Reference" {q(ref)} (at {f(x)} {f(y)} 0) {effects(1.27, hide=True)}) '
                f'(property "Value" "PWR_FLAG" (at {f(x)} {f(y)} 0) {effects(1.0, hide=True)}) '
                f'(property "Footprint" "" (at {f(x)} {f(y)} 0) {effects(1.27, hide=True)}) '
                f'(pin "1" (uuid "{self.uid()}")) '
                f'(instances (project {q(project)} (path {q(path)} (reference {q(ref)}) (unit 1)))))')
        for i in self.insts:
            out.append(self._inst_sexpr(i, project, path))
        out += self.sheet_syms
        if self.page == '1':
            out.append('(sheet_instances (path "/" (page "1")))')
        out.append('(embedded_fonts no))')
        return '\n'.join(out) + '\n'

    def _power_value_pos(self, net, x, y, d):
        dx, dy = d
        if dy == -1:
            return x, y - 3.302
        if dy == 1:
            return x, y + 4.318
        return x + dx * 3.81, y - 1.27

    def _inst_sexpr(self, i, project, path):
        fld = []
        rx, ry, rj = i.ref_pos
        vx, vy, vj = i.val_pos
        # KiCad applies the symbol rotation to field text and keeps it readable
        # by flipping justification; compensate so fields read horizontally.
        fa = 90 if i.ang % 180 == 90 else 0
        flip = (i.ang + fa) % 360 == 180
        mir = i.mirror == 'y'

        def jfix(j):
            if not j:
                return j
            sw = {'left': 'right', 'right': 'left', 'top': 'bottom', 'bottom': 'top'}
            out = []
            for w in j.split():
                if flip:
                    w = sw[w]
                if mir and w in ('left', 'right'):
                    w = sw[w]
                out.append(w)
            return ' '.join(out)
        rj, vj = jfix(rj), jfix(vj)
        fld.append(f'(property "Reference" {q(i.ref)} (at {f(rx)} {f(ry)} {fa}) {effects(1.778, rj)})')
        fld.append(f'(property "Value" {q(i.value)} (at {f(vx)} {f(vy)} {fa}) '
                   f'{effects(1.524, vj, hide=i.extra.get("_hide_value", False))})')
        fld.append(f'(property "Footprint" {q(i.footprint)} (at {f(i.x)} {f(i.y)} 0) {effects(1.27, hide=True)})')
        fld.append(f'(property "Datasheet" "" (at {f(i.x)} {f(i.y)} 0) {effects(1.27, hide=True)})')
        for k, v in i.extra.items():
            if k.startswith('_'):
                continue
            if isinstance(v, tuple):     # visible field: (value, x, y, justify)
                fld.append(f'(property {q(k)} {q(v[0])} (at {f(v[1])} {f(v[2])} 0) {effects(1.27, v[3])})')
            else:
                fld.append(f'(property {q(k)} {q(v)} (at {f(i.x)} {f(i.y)} 0) {effects(1.27, hide=True)})')
        mirror = f' (mirror {i.mirror})' if i.mirror else ''
        pins = ' '.join(f'(pin {q(p.num)} (uuid "{suid(i.key + ":pin:" + p.num)}"))'
                        for p in i.sym.units[i.unit]['pins'])
        return (f'(symbol (lib_id "DDS:{i.sym.name}") (at {f(i.x)} {f(i.y)} {i.ang}){mirror} '
                f'(unit {i.unit}) (exclude_from_sim no) (in_bom {"yes" if i.in_bom else "no"}) '
                f'(on_board yes) (dnp {"yes" if i.dnp else "no"}) (uuid "{suid("sym:" + i.key + ":" + str(i.unit))}") '
                + ' '.join(fld) + ' ' + pins +
                f' (instances (project {q(project)} (path {q(path)} (reference {q(i.ref)}) (unit {i.unit})))))')
