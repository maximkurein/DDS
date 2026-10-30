"""Minimal KiCad schematic/symbol writer used by the DDS project generator.

All geometry is in millimetres.  Library symbol coordinates are Y-up (KiCad
convention); schematic coordinates are Y-down.  Everything the generator
places is snapped to the 1.27 mm connection grid.
"""
import math
import uuid

GRID = 1.27
NS = uuid.UUID('6f1c3a52-6d1e-4a8e-9a43-1508e1887ad5')  # stable namespace


def suid(key):
    """Deterministic UUID so that regenerated files keep stable identities."""
    return str(uuid.uuid5(NS, key))


def q(s):
    return '"' + str(s).replace('\\', '\\\\').replace('"', '\\"') + '"'


def f(v):
    v = round(float(v), 4)
    if v == int(v):
        return str(int(v))
    return ('%.4f' % v).rstrip('0').rstrip('.')


def snap(v, g=GRID):
    return round(round(v / g) * g, 4)


def font(size, bold=False, italic=False):
    s = f'(font (size {f(size)} {f(size)})'
    if bold:
        s += ' (bold yes)'
    if italic:
        s += ' (italic yes)'
    return s + ')'


def effects(size=1.27, justify=None, hide=False, bold=False):
    s = f'(effects {font(size, bold)}'
    if justify:
        s += f' (justify {justify})'
    if hide:
        s += ' (hide yes)'
    return s + ')'


# ---------------------------------------------------------------- symbols ---

class Pin:
    def __init__(self, num, name, x, y, angle, length, etype='passive',
                 shape='line', name_hidden=False, num_hidden=False):
        self.num, self.name, self.x, self.y = str(num), name, x, y
        self.angle, self.length, self.etype, self.shape = angle, length, etype, shape
        self.name_hidden, self.num_hidden = name_hidden, num_hidden

    def sexpr(self, name_size=1.27, num_size=1.0):
        nm = effects(name_size, hide=self.name_hidden)
        nu = effects(num_size, hide=self.num_hidden)
        return (f'(pin {self.etype} {self.shape} (at {f(self.x)} {f(self.y)} {self.angle}) '
                f'(length {f(self.length)}) (name {q(self.name)} {nm}) (number {q(self.num)} {nu}))')


class Symbol:
    """A library symbol.  units: {unit_no: {'pins': [Pin], 'gfx': [str]}}"""

    def __init__(self, name, ref_prefix, value='', power=False, pin_name_offset=0.508,
                 hide_pin_names=False, hide_pin_numbers=False, ref_at=(0, 0, 'left'),
                 val_at=(0, 0, 'left'), in_bom=True, on_board=True, unit_names=None,
                 desc=''):
        self.name, self.ref_prefix, self.value = name, ref_prefix, value or name
        self.power = power
        self.pin_name_offset = pin_name_offset
        self.hide_pin_names, self.hide_pin_numbers = hide_pin_names, hide_pin_numbers
        self.ref_at, self.val_at = ref_at, val_at
        self.in_bom, self.on_board = in_bom, on_board
        self.units = {}
        self.common_gfx = []
        self.unit_names = unit_names or {}
        self.desc = desc
        self.name_size, self.num_size = 1.27, 1.0

    def unit(self, n):
        return self.units.setdefault(n, {'pins': [], 'gfx': []})

    def pin(self, unit, *a, **k):
        p = Pin(*a, **k)
        self.unit(unit)['pins'].append(p)
        return p

    def gfx(self, unit, s):
        if unit == 0:
            self.common_gfx.append(s)
        else:
            self.unit(unit)['gfx'].append(s)

    def pins_of(self, unit):
        return {p.num: p for p in self.units[unit]['pins']}

    def all_pins(self):
        out = {}
        for u, d in self.units.items():
            for p in d['pins']:
                out[p.num] = (u, p)
        return out

    def sexpr(self, libname=None):
        full = f'{libname}:{self.name}' if libname else self.name
        s = [f'(symbol {q(full)}']
        if self.power:
            s.append('(power global)')
        if self.hide_pin_numbers:
            s.append('(pin_numbers (hide yes))')
        pn = f'(pin_names (offset {f(self.pin_name_offset)})'
        if self.hide_pin_names:
            pn += ' (hide yes)'
        s.append(pn + ')')
        s.append('(exclude_from_sim no)')
        s.append(f'(in_bom {"yes" if self.in_bom else "no"}) (on_board {"yes" if self.on_board else "no"})')
        rx, ry, rj = self.ref_at
        vx, vy, vj = self.val_at
        s.append(f'(property "Reference" {q(self.ref_prefix)} (at {f(rx)} {f(ry)} 0) '
                 f'{effects(1.778, rj, hide=self.power)})')
        s.append(f'(property "Value" {q(self.value)} (at {f(vx)} {f(vy)} 0) {effects(1.524, vj)})')
        s.append(f'(property "Footprint" "" (at 0 0 0) {effects(1.27, hide=True)})')
        s.append(f'(property "Datasheet" "" (at 0 0 0) {effects(1.27, hide=True)})')
        s.append(f'(property "Description" {q(self.desc)} (at 0 0 0) {effects(1.27, hide=True)})')
        if self.common_gfx:
            s.append(f'(symbol {q(self.name + "_0_1")} ' + ' '.join(self.common_gfx) + ')')
        for u in sorted(self.units):
            d = self.units[u]
            body = ' '.join(d['gfx'] + [p.sexpr(self.name_size, self.num_size) for p in d['pins']])
            un = ''
            if u in self.unit_names:
                un = f'(unit_name {q(self.unit_names[u])}) '
            s.append(f'(symbol {q(f"{self.name}_{u}_1")} {un}{body})')
        s.append('(embedded_fonts no))')
        return ' '.join(s)


def poly(pts, width=0.254, fill='none'):
    p = ' '.join(f'(xy {f(x)} {f(y)})' for x, y in pts)
    return f'(polyline (pts {p}) (stroke (width {f(width)}) (type default)) (fill (type {fill})))'


def rect(x1, y1, x2, y2, width=0.254, fill='none'):
    return (f'(rectangle (start {f(x1)} {f(y1)}) (end {f(x2)} {f(y2)}) '
            f'(stroke (width {f(width)}) (type default)) (fill (type {fill})))')


def circle(x, y, r, width=0.254, fill='none'):
    return (f'(circle (center {f(x)} {f(y)}) (radius {f(r)}) '
            f'(stroke (width {f(width)}) (type default)) (fill (type {fill})))')


def arc(sx, sy, mx, my, ex, ey, width=0.254):
    return (f'(arc (start {f(sx)} {f(sy)}) (mid {f(mx)} {f(my)}) (end {f(ex)} {f(ey)}) '
            f'(stroke (width {f(width)}) (type default)) (fill (type none)))')


def text(s, x, y, size=1.27, angle=0, justify=None, bold=False):
    return f'(text {q(s)} (at {f(x)} {f(y)} {angle}) {effects(size, justify, bold=bold)})'


# ------------------------------------------------------------ transforms ---

def rot(x, y, ang):
    """Rotate a library point CCW by ang degrees (library, Y-up)."""
    a = ang % 360
    if a == 0:
        return x, y
    if a == 90:
        return -y, x
    if a == 180:
        return -x, -y
    if a == 270:
        return y, -x
    r = math.radians(a)
    return x * math.cos(r) - y * math.sin(r), x * math.sin(r) + y * math.cos(r)


def lib_to_sch(X, Y, ang, x, y, mirror=None):
    if mirror == 'y':      # mirror around Y axis (left-right)
        x = -x
    elif mirror == 'x':
        y = -y
    rx, ry = rot(x, y, ang)
    return round(X + rx, 4), round(Y - ry, 4)


def pin_end(pin):
    """Connection point of a pin in library coordinates (the 'at' point)."""
    return pin.x, pin.y


def pin_dir_sch(pin, ang, mirror=None):
    """Unit vector (schematic, Y-down) pointing from the body outwards along the pin."""
    # pin.angle is the direction from the connection point towards the body
    a = pin.angle % 360
    dx, dy = {0: (1, 0), 90: (0, 1), 180: (-1, 0), 270: (0, -1)}[a]
    dx, dy = -dx, -dy                    # outward direction (library)
    if mirror == 'y':
        dx = -dx
    elif mirror == 'x':
        dy = -dy
    rx, ry = rot(dx, dy, ang)
    return round(rx), round(-ry)
