#!/usr/bin/env python3
"""KiCad schematic -> Visio (.vsdx) for printing / reading (demonstration copy).

    python3 tools/sch2vsdx.py <project_dir> <out.vsdx>

The KiCad project stays the single source of truth: every page is rebuilt
from the .kicad_sch files (symbols from the embedded lib_symbols, wires,
junctions, labels, texts) and the ГОСТ frame from lib/GOST_E3.kicad_wks.
Each component becomes a Visio group (graphics + its texts), each wire a
separate line shape; all text is real Arial text, editable in Visio.
The package skeleton (styles) comes from the blank drawing shipped with the
`vsdx` python package, so the file opens in Visio and in LibreOffice Draw.
"""
import math
import re
import sys
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape

MM = 1 / 25.4                       # mm -> inch
FONT = 'Arial'
PAPER = {'A4': (297, 210), 'A3': (420, 297), 'A2': (594, 420), 'A1': (841, 594)}
W_SYM, W_WIRE, W_THIN = 0.25, 0.25, 0.18     # line weights, mm (as in the samples, ГОСТ 2.303)

# ------------------------------------------------------------------ s-expr --
TOK = re.compile(r'\s*(\(|\)|"(?:[^"\\]|\\.)*"|[^\s()"]+)')


def parse(text):
    stack = [[]]
    for m in TOK.finditer(text):
        t = m.group(1)
        if t == '(':
            stack.append([])
        elif t == ')':
            n = stack.pop()
            stack[-1].append(n)
        elif t.startswith('"'):
            stack[-1].append(t[1:-1].replace('\\"', '"').replace('\\\\', '\\'))
        else:
            stack[-1].append(t)
    return stack[0][0]


def kids(n, key):
    return [c for c in n[1:] if isinstance(c, list) and c and c[0] == key]


def kid(n, key):
    for c in n[1:]:
        if isinstance(c, list) and c and c[0] == key:
            return c
    return None


def flag(n, key):
    """(hide yes) / bare 'hide' atom / (hide)."""
    if n is None:
        return False
    for c in n[1:]:
        if c == key:
            return True
        if isinstance(c, list) and c and c[0] == key:
            return len(c) == 1 or c[1] == 'yes'
    return False


def xy(n):
    return float(n[1]), float(n[2])


def at(n):
    a = kid(n, 'at')
    return float(a[1]), float(a[2]), (float(a[3]) if len(a) > 3 else 0.0)


def effects(n):
    e = kid(n, 'effects')
    size, just, hide = 1.27, [], False
    if e is not None:
        f = kid(e, 'font')
        if f is not None and kid(f, 'size') is not None:
            size = float(kid(f, 'size')[1])
        j = kid(e, 'justify')
        if j is not None:
            just = [x for x in j[1:] if isinstance(x, str)]
        hide = flag(e, 'hide')
    return size, just, hide or flag(n, 'hide')


# ------------------------------------------------------------- text metrics --
try:
    from PIL import ImageFont
    for _name in ('/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf', 'arial.ttf', 'Arial.ttf'):
        try:
            _F = ImageFont.truetype(_name, 100)
            break
        except OSError:
            continue
    else:
        raise OSError('no Arial-metric font')

    def text_w(s, size):            # size = cap height, mm (ГОСТ meaning)
        return _F.getlength(s) / 100 * size / 0.716
except Exception:                   # pragma: no cover
    def text_w(s, size):
        return len(s) * size * 0.75


def clean(s):
    """KiCad markup -> plain text (+ overline flag)."""
    over = s.startswith('~{') and s.endswith('}')
    s = re.sub(r'~\{([^}]*)\}', r'\1', s)
    return s, over


# -------------------------------------------------------------- Visio shapes --
class Page:
    def __init__(self, name, w, h):
        self.name, self.w, self.h = name, w, h
        self.shapes = []
        self.nid = 1

    def id(self):
        self.nid += 1
        return self.nid

    def P(self, x, y):              # KiCad mm (Y down) -> Visio inch (Y up)
        return x * MM, (self.h - y) * MM


NAMES = set()


def uname(name, sid):
    """Visio wants unique shape names on a page: 'Связь' -> 'Связь.17'."""
    n = name if name not in NAMES else f'{name}.{sid}'
    NAMES.add(n)
    return n


def cell(n, v, u=None, f=None):
    s = f"<Cell N='{n}' V='{v}'"
    if u:
        s += f" U='{u}'"
    if f:
        s += f" F='{f}'"
    return s + '/>'


def fmt(v):
    return ('%.6f' % v).rstrip('0').rstrip('.') or '0'


class Geo:
    """Collects geometry in page inches; emits a shape with local coordinates."""

    def __init__(self, weight=W_SYM):
        self.paths = []             # (kind, data, filled)
        self.weight = weight

    def poly(self, pts, filled=False, closed=False):
        if len(pts) >= 2:
            self.paths.append(('poly', (pts + [pts[0]]) if closed else pts, filled))

    def circle(self, c, r, filled=False):
        self.paths.append(('circle', (c, r), filled))

    def bbox(self):
        xs, ys = [], []
        for k, d, _ in self.paths:
            if k == 'poly':
                xs += [p[0] for p in d]; ys += [p[1] for p in d]
            else:
                (cx, cy), r = d
                xs += [cx - r, cx + r]; ys += [cy - r, cy + r]
        return min(xs), min(ys), max(xs), max(ys)

    def xml(self, sid, ox=0.0, oy=0.0, name=None):
        x0, y0, x1, y1 = self.bbox()
        w, h = max(x1 - x0, 0.001), max(y1 - y0, 0.001)
        secs = []
        for i, (k, d, filled) in enumerate(self.paths):
            rows = [cell('NoFill', 0 if filled else 1), cell('NoLine', 0), cell('NoShow', 0), cell('NoSnap', 0)]
            if k == 'poly':
                for j, (px, py) in enumerate(d):
                    t = 'MoveTo' if j == 0 else 'LineTo'
                    rows.append(f"<Row T='{t}' IX='{j + 1}'>{cell('X', fmt(px - x0))}{cell('Y', fmt(py - y0))}</Row>")
            else:
                (cx, cy), r = d
                cx, cy = cx - x0, cy - y0
                rows.append(f"<Row T='Ellipse' IX='1'>{cell('X', fmt(cx))}{cell('Y', fmt(cy))}"
                            f"{cell('A', fmt(cx + r))}{cell('B', fmt(cy))}{cell('C', fmt(cx))}{cell('D', fmt(cy + r))}</Row>")
            secs.append(f"<Section N='Geometry' IX='{i}'>{''.join(rows)}</Section>")
        if name:
            name = uname(name, sid)
        nm = f" NameU='{escape(name)}' Name='{escape(name)}'" if name else ''
        return (f"<Shape ID='{sid}'{nm} Type='Shape' LineStyle='3' FillStyle='3' TextStyle='3'>"
                + cell('PinX', fmt(x0 + w / 2 - ox)) + cell('PinY', fmt(y0 + h / 2 - oy))
                + cell('Width', fmt(w)) + cell('Height', fmt(h))
                + cell('LocPinX', fmt(w / 2)) + cell('LocPinY', fmt(h / 2))
                + cell('LineWeight', fmt(self.weight * MM), 'MM') + cell('LineColor', '#000000')
                + cell('LineCap', 1) + cell('FillForegnd', '#000000') + cell('FillPattern', 1)
                + ''.join(secs) + '</Shape>'), (x0, y0, x1, y1)


def text_shape(sid, s, x, y, size, hj='center', vj='center', angle=0.0, ox=0.0, oy=0.0, bold=False):
    """Text anchored at page point (x, y) inch; size = cap height in mm."""
    s, over = clean(s)
    em = size / 0.716                                 # Arial em, mm
    w = (text_w(s, size) + 0.6 * em) * MM * 1.05 + 0.01
    h = em * 1.25 * MM
    lx = {'left': 0, 'center': w / 2, 'right': w}[hj]
    ly = {'bottom': 0, 'center': h / 2, 'top': h}[vj]
    ha = {'left': 0, 'center': 1, 'right': 2}[hj]
    va = {'top': 0, 'center': 1, 'bottom': 2}[vj]
    pad = 0.3 * em * MM
    if hj == 'left':
        x -= pad
    elif hj == 'right':
        x += pad
    a = math.radians(angle)
    bb = (x - w, y - w, x + w, y + w)
    return (f"<Shape ID='{sid}' Type='Shape' LineStyle='3' FillStyle='3' TextStyle='3'>"
            + cell('PinX', fmt(x - ox)) + cell('PinY', fmt(y - oy)) + cell('Width', fmt(w)) + cell('Height', fmt(h))
            + cell('LocPinX', fmt(lx)) + cell('LocPinY', fmt(ly)) + cell('Angle', fmt(a))
            + cell('LeftMargin', 0) + cell('RightMargin', 0) + cell('TopMargin', 0) + cell('BottomMargin', 0)
            + cell('VerticalAlign', va) + cell('LinePattern', 0) + cell('FillPattern', 0)
            + "<Section N='Character'><Row IX='0'>" + cell('Font', FONT) + cell('Color', '#000000')
            + cell('Style', 1 if bold else 0) + cell('Size', fmt(em / 25.4), 'PT') + cell('Overline', 1 if over else 0)
            + cell('LangID', 'ru-RU') + '</Row></Section>'
            + "<Section N='Paragraph'><Row IX='0'>" + cell('HorzAlign', ha) + cell('SpLine', -1) + '</Row></Section>'
            + f'<Text>{escape(s)}</Text></Shape>'), bb


def group_xml(sid, name, children, bbox):
    name = uname(name, sid)
    x0, y0, x1, y1 = bbox
    w, h = max(x1 - x0, 0.01), max(y1 - y0, 0.01)
    return (f"<Shape ID='{sid}' NameU='{escape(name)}' Name='{escape(name)}' Type='Group' LineStyle='3' FillStyle='3' TextStyle='3'>"
            + cell('PinX', fmt(x0 + w / 2)) + cell('PinY', fmt(y0 + h / 2)) + cell('Width', fmt(w)) + cell('Height', fmt(h))
            + cell('LocPinX', fmt(w / 2)) + cell('LocPinY', fmt(h / 2)) + cell('LinePattern', 0) + cell('FillPattern', 0)
            + '<Shapes>' + ''.join(children) + '</Shapes></Shape>')


# --------------------------------------------------------- KiCad -> shapes --
def tf(X, Y, ang, mirror, x, y):
    """Library point (Y up) -> schematic point (Y down), as KiCad does."""
    if mirror == 'y':
        x = -x
    elif mirror == 'x':
        y = -y
    a = math.radians(ang)
    rx, ry = x * math.cos(a) - y * math.sin(a), x * math.sin(a) + y * math.cos(a)
    return X + rx, Y - ry


def tdir(ang, mirror, dx, dy):
    """Library direction -> schematic direction (Y down)."""
    x, y = tf(0, 0, ang, mirror, dx, dy)
    return round(x, 6), round(y, 6)


def arc_pts(s, m, e, n=16):
    (x1, y1), (x2, y2), (x3, y3) = s, m, e
    d = 2 * (x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2))
    if abs(d) < 1e-9:
        return [s, e]
    ux = ((x1 ** 2 + y1 ** 2) * (y2 - y3) + (x2 ** 2 + y2 ** 2) * (y3 - y1) + (x3 ** 2 + y3 ** 2) * (y1 - y2)) / d
    uy = ((x1 ** 2 + y1 ** 2) * (x3 - x2) + (x2 ** 2 + y2 ** 2) * (x1 - x3) + (x3 ** 2 + y3 ** 2) * (x2 - x1)) / d
    r = math.hypot(x1 - ux, y1 - uy)
    a1, a2, a3 = (math.atan2(p[1] - uy, p[0] - ux) for p in (s, m, e))
    # sweep from a1 to a3 passing a2
    def norm(a):
        return a % (2 * math.pi)
    sweep = norm(a3 - a1)
    if norm(a2 - a1) > sweep:
        sweep -= 2 * math.pi
    return [(ux + r * math.cos(a1 + sweep * i / n), uy + r * math.sin(a1 + sweep * i / n)) for i in range(n + 1)]


class Sheet:
    def __init__(self, path):
        self.path = Path(path)
        self.root = parse(self.path.read_text(encoding='utf-8'))
        p = kid(self.root, 'paper')[1]
        self.paper = p
        self.w, self.h = PAPER.get(p, (420, 297))
        tb = kid(self.root, 'title_block')
        self.tb = {}
        if tb is not None:
            for c in tb[1:]:
                if isinstance(c, list):
                    if c[0] == 'comment':
                        self.tb[f'COMMENT{c[1]}'] = c[2]
                    else:
                        self.tb[c[0].upper()] = c[1] if len(c) > 1 else ''
        self.lib = {}
        ls = kid(self.root, 'lib_symbols')
        for s in kids(ls, 'symbol') if ls else []:
            self.lib[s[1]] = s


def lib_units(sym, unit):
    out = []
    for sub in kids(sym, 'symbol'):
        m = re.match(r'.*_(\d+)_(\d+)$', sub[1])
        if m and int(m.group(1)) in (0, unit) and int(m.group(2)) in (0, 1):
            out.append(sub)
    return out


def n_units(sym):
    us = set()
    for sub in kids(sym, 'symbol'):
        m = re.match(r'.*_(\d+)_(\d+)$', sub[1])
        if m and int(m.group(1)) > 0:
            us.add(int(m.group(1)))
    return len(us)


def draw_symbol(pg, sh, inst):
    lib_id = kid(inst, 'lib_id')[1]
    X, Y, ang = at(inst)
    mir = kid(inst, 'mirror')
    mirror = mir[1] if mir else None
    unit = int(kid(inst, 'unit')[1]) if kid(inst, 'unit') else 1
    sym = sh.lib.get(lib_id)
    if sym is None:
        return None
    pn = kid(sym, 'pin_names')
    names_off = float(kid(pn, 'offset')[1]) if pn is not None and kid(pn, 'offset') else 0.508
    names_hide = flag(pn, 'hide') if pn is not None else False
    nums_hide = flag(kid(sym, 'pin_numbers'), 'hide') if kid(sym, 'pin_numbers') else False
    P = lambda x, y: pg.P(*tf(X, Y, ang, mirror, x, y))
    g = Geo(W_SYM)
    texts = []                       # (s, x_in, y_in, size, hj, vj, angle)
    for sub in lib_units(sym, unit):
        for gr in sub[1:]:
            if not isinstance(gr, list):
                continue
            k = gr[0]
            fill = kid(gr, 'fill')
            filled = fill is not None and kid(fill, 'type') is not None and kid(fill, 'type')[1] in ('outline',)
            if k == 'polyline':
                g.poly([P(*xy(p)) for p in kids(kid(gr, 'pts'), 'xy')], filled)
            elif k == 'rectangle':
                (x1, y1), (x2, y2) = xy(kid(gr, 'start')), xy(kid(gr, 'end'))
                g.poly([P(x1, y1), P(x2, y1), P(x2, y2), P(x1, y2)], filled, closed=True)
            elif k == 'circle':
                c = P(*xy(kid(gr, 'center')))
                g.circle(c, float(kid(gr, 'radius')[1]) * MM, filled)
            elif k == 'arc':
                pts = arc_pts(xy(kid(gr, 'start')), xy(kid(gr, 'mid')), xy(kid(gr, 'end')))
                g.poly([P(*p) for p in pts], False)
            elif k == 'text':
                tx, ty, ta = at(gr)
                size, just, hide = effects(gr)
                if hide:
                    continue
                da = (ta / 10 if ta > 360 else ta) + ang
                da = da % 180
                texts.append((gr[1], *P(tx, ty), size, 'center', 'center', da))
            elif k == 'pin':
                if flag(gr, 'hide'):
                    continue
                shape = gr[2] if len(gr) > 2 else 'line'
                px, py, pa = at(gr)
                L = float(kid(gr, 'length')[1])
                d = (math.cos(math.radians(pa)), math.sin(math.radians(pa)))
                end = (px + d[0] * L, py + d[1] * L)
                if 'inverted' in shape:
                    r = 0.5
                    c = (px + d[0] * (L - r), py + d[1] * (L - r))
                    g.circle(P(*c), r * MM)
                    g.poly([P(px, py), P(px + d[0] * (L - 2 * r), py + d[1] * (L - 2 * r))])
                else:
                    g.poly([P(px, py), P(*end)])
                if 'clock' in shape:
                    cs = 0.635
                    nx, ny = -d[1], d[0]
                    g.poly([P(end[0] + nx * cs, end[1] + ny * cs), P(end[0] + d[0] * cs, end[1] + d[1] * cs),
                            P(end[0] - nx * cs, end[1] - ny * cs)])
                sd = tdir(ang, mirror, *d)                # schematic direction body-wards
                horiz = abs(sd[0]) > 0.5
                nm = kid(gr, 'name'); nu = kid(gr, 'number')
                if nu is not None and not nums_hide and nu[1] not in ('', '~'):
                    size = effects(nu)[0]
                    mx, my = tf(X, Y, ang, mirror, (px + end[0]) / 2, (py + end[1]) / 2)
                    off = 0.25
                    if horiz:
                        texts.append((nu[1], *pg.P(mx, my - off), size, 'center', 'bottom', 0))
                    else:
                        texts.append((nu[1], *pg.P(mx - off, my), size, 'center', 'bottom', 90))
                if nm is not None and not names_hide and nm[1] not in ('', '~') and names_off > 0:
                    size = effects(nm)[0]
                    qx, qy = tf(X, Y, ang, mirror, end[0] + d[0] * names_off, end[1] + d[1] * names_off)
                    if horiz:
                        texts.append((nm[1], *pg.P(qx, qy), size, 'left' if sd[0] > 0 else 'right', 'center', 0))
                    else:
                        # vertical pin: text reads bottom-to-top
                        texts.append((nm[1], *pg.P(qx, qy), size, 'left' if sd[1] < 0 else 'right', 'center', 90))
    # fields (positions are absolute; see tools/sheet.py for the convention)
    for pr in kids(inst, 'property'):
        name, val = pr[1], pr[2]
        size, just, hide = effects(pr)
        if hide or not val or name in ('Footprint', 'Datasheet', 'Description'):
            continue
        fx, fy, fa = at(pr)
        if name == 'Reference' and val.startswith('#'):
            continue
        if name == 'Reference' and n_units(sym) > 1:
            val = f'{val}.{unit}'                    # ГОСТ 2.710: D1.1, D1.2
        rot = ang % 180 == 90
        disp = (fa + (90 if rot else 0)) % 180
        flip = (ang + fa) % 360 == 180
        sw = {'left': 'right', 'right': 'left', 'top': 'bottom', 'bottom': 'top'}
        hj, vj = 'center', 'center'
        for wj in just:
            if flip:
                wj = sw.get(wj, wj)
            if mirror == 'y' and wj in ('left', 'right'):
                wj = sw[wj]
            if wj in ('left', 'right'):
                hj = wj
            elif wj in ('top', 'bottom'):
                vj = wj
        texts.append((val, *pg.P(fx, fy), size, hj, vj, disp))
    if not g.paths and not texts:
        return None
    ref = next((p[2] for p in kids(inst, 'property') if p[1] == 'Reference'), lib_id)
    if n_units(sym) > 1:
        ref = f'{ref}.{unit}'
    return g, texts, ref


def wks_items(wks_path, page_no, pages, sh, pg):
    """ГОСТ frame from the KiCad drawing sheet."""
    root = parse(Path(wks_path).read_text(encoding='utf-8'))
    W, H = sh.w, sh.h

    def corner(p):
        x, y = float(p[1]), float(p[2])
        c = p[3] if len(p) > 3 else 'rbcorner'
        if c == 'ltcorner':
            return x, y
        if c == 'lbcorner':
            return x, H - y
        if c == 'rtcorner':
            return W - x, y
        return W - x, H - y

    def visible(n):
        o = kid(n, 'option')
        if o is None:
            return True
        return not ((o[1] == 'page1only' and page_no != 1) or (o[1] == 'notonpage1' and page_no == 1))
    var = dict(sh.tb)
    var.update({'#': str(page_no), '##': str(pages), 'PAPER': sh.paper})
    lines, texts = [], []
    for n in root[1:]:
        if not isinstance(n, list) or not visible(n):
            continue
        if n[0] in ('line', 'rect'):
            a, b = corner(kid(n, 'start')), corner(kid(n, 'end'))
            lw = float(kid(n, 'linewidth')[1]) if kid(n, 'linewidth') else 0.25
            pts = [a, b] if n[0] == 'line' else [a, (b[0], a[1]), b, (a[0], b[1]), a]
            lines.append(([pg.P(*p) for p in pts], lw))
        elif n[0] == 'tbtext':
            s = re.sub(r'\$\{([^}]*)\}', lambda m: var.get(m.group(1), ''), n[1])
            if not s.strip():
                continue
            x, y = corner(kid(n, 'pos'))
            f = kid(n, 'font')
            size = float(kid(f, 'size')[1]) if f is not None and kid(f, 'size') else 2.5
            j = kid(n, 'justify')
            hj = 'center' if j is None else next((w for w in j[1:] if w in ('left', 'right', 'center')), 'left')
            vj = 'center' if j is None else next((w for w in j[1:] if w in ('top', 'bottom')), 'center')
            if j is None:
                hj = 'left'
            r = float(kid(n, 'rotate')[1]) if kid(n, 'rotate') else 0
            texts.append((s, *pg.P(x, y), size, hj, vj, r % 360 if r % 180 else 0))
    return lines, texts


def build_page(sh, page_no, pages, wks, title):
    pg = Page(title, sh.w, sh.h)
    NAMES.clear()
    out = []
    # frame
    lines, texts = wks_items(wks, page_no, pages, sh, pg)
    frame = []
    for lw in sorted({l[1] for l in lines}):
        g = Geo(lw)
        for pts, w in lines:
            if w == lw:
                g.poly(pts)
        x, _ = g.xml(pg.id())
        frame.append(x)
    for t in texts:
        x, _ = text_shape(pg.id(), *t)
        frame.append(x)
    out.append(group_xml(pg.id(), 'Рамка и основная надпись', frame, (0, 0, sh.w * MM, sh.h * MM)))
    # symbols
    for inst in kids(sh.root, 'symbol'):
        r = draw_symbol(pg, sh, inst)
        if r is None:
            continue
        g, texts, ref = r
        children, bbs = [], []

        if g.paths:
            x0, y0, x1, y1 = g.bbox()
            bbs.append((x0, y0, x1, y1))
        for t in texts:
            bbs.append((t[1] - 0.05, t[2] - 0.05, t[1] + 0.05, t[2] + 0.05))
        gx0 = min(b[0] for b in bbs); gy0 = min(b[1] for b in bbs)
        gx1 = max(b[2] for b in bbs); gy1 = max(b[3] for b in bbs)
        if g.paths:
            x, _ = g.xml(pg.id(), gx0, gy0)
            children.append(x)
        for t in texts:
            x, _ = text_shape(pg.id(), *t, ox=gx0, oy=gy0)
            children.append(x)
        out.append(group_xml(pg.id(), ref, children, (gx0, gy0, gx1, gy1)))
    # wires
    for wnode in kids(sh.root, 'wire'):
        pts = [pg.P(*xy(p)) for p in kids(kid(wnode, 'pts'), 'xy')]
        g = Geo(W_WIRE); g.poly(pts)
        x, _ = g.xml(pg.id(), name='Связь')
        out.append(x)
    for pl in kids(sh.root, 'polyline'):
        pts = [pg.P(*xy(p)) for p in kids(kid(pl, 'pts'), 'xy')]
        st = kid(pl, 'stroke')
        w = float(kid(st, 'width')[1]) if st is not None and kid(st, 'width') and float(kid(st, 'width')[1]) > 0 else W_THIN
        g = Geo(w); g.poly(pts)
        x, _ = g.xml(pg.id())
        out.append(x)
    # junctions, no-connects
    g = Geo(W_WIRE)
    for j in kids(sh.root, 'junction'):
        g.circle(pg.P(*xy(kid(j, 'at'))), 0.5 * MM, filled=True)
    if g.paths:
        x, _ = g.xml(pg.id(), name='Точки соединения')
        out.append(x)
    g = Geo(W_THIN)
    for nc in kids(sh.root, 'no_connect'):
        cx, cy = xy(kid(nc, 'at'))
        s = 0.6
        g.poly([pg.P(cx - s, cy - s), pg.P(cx + s, cy + s)])
        g.poly([pg.P(cx - s, cy + s), pg.P(cx + s, cy - s)])
    if g.paths:
        x, _ = g.xml(pg.id(), name='Неподключённые выводы')
        out.append(x)
    # labels and texts
    for kind in ('label', 'global_label', 'hierarchical_label', 'text'):
        for lb in kids(sh.root, kind):
            x, y, a = at(lb)
            size, just, hide = effects(lb)
            if hide:
                continue
            s = lb[1]
            hj = 'right' if 'right' in just else ('center' if kind == 'text' and 'left' not in just else 'left')
            if kind == 'text':
                hj = 'right' if 'right' in just else ('left' if 'left' in just else 'center')
                vj = 'top' if 'top' in just else ('bottom' if 'bottom' in just else 'center')
                px, py = pg.P(x, y)
                xs, _ = text_shape(pg.id(), s, px, py, size, hj, vj, a % 180)
                out.append(xs)
                continue
            # net label: text above the wire (local) or boxed (global/hierarchical)
            dx, dy = {0: (1, 0), 90: (0, -1), 180: (-1, 0), 270: (0, 1)}[int(a) % 360]
            tw = text_w(clean(s)[0], size)
            if kind == 'label':
                off = 0.35
                if dx:
                    px, py = pg.P(x, y - off)
                    xs, _ = text_shape(pg.id(), s, px, py, size, 'left' if dx > 0 else 'right', 'bottom', 0)
                else:
                    px, py = pg.P(x - off, y)
                    xs, _ = text_shape(pg.id(), s, px, py, size, 'left' if dy < 0 else 'right', 'bottom', 90)
                out.append(xs)
                continue
            hh = size * 0.8 + 0.4
            L = tw + 2 * hh
            ux, uy = dx, dy
            nx, ny = -uy, ux
            q = lambda t, n: pg.P(x + ux * t + nx * n, y + uy * t + ny * n)
            g = Geo(W_THIN)
            g.poly([q(0, 0), q(hh, hh), q(L, hh), q(L, -hh), q(hh, -hh), q(0, 0)])
            xg, _ = g.xml(pg.id())
            cx, cy = q(hh + tw / 2 + 0.2, 0)
            xs, _ = text_shape(pg.id(), s, cx, cy, size, 'center', 'center', 0 if dx else 90)
            out.append(xg)
            out.append(xs)
    # hierarchical sheet symbols (root page)
    for sn in kids(sh.root, 'sheet'):
        x, y = xy(kid(sn, 'at')); w, h = xy(kid(sn, 'size'))
        g = Geo(W_SYM)
        g.poly([pg.P(x, y), pg.P(x + w, y), pg.P(x + w, y + h), pg.P(x, y + h)], closed=True)
        xg, _ = g.xml(pg.id())
        out.append(xg)
        for pr in kids(sn, 'property'):
            size, just, hide = effects(pr)
            if hide:
                continue
            fx, fy, fa = at(pr)
            txt = pr[2] if pr[1] == 'Sheetname' else 'Файл: ' + pr[2]
            px, py = pg.P(fx, fy)
            xs, _ = text_shape(pg.id(), txt, px, py, size, 'left', 'bottom', fa)
            out.append(xs)
    return pg, out


# ------------------------------------------------------------ package write --
def page_xml(shapes):
    return ("<?xml version='1.0' encoding='utf-8' ?>\n<PageContents xmlns='http://schemas.microsoft.com/office/visio/2012/main' "
            "xmlns:r='http://schemas.openxmlformats.org/officeDocument/2006/relationships' xml:space='preserve'><Shapes>"
            + ''.join(shapes) + '</Shapes></PageContents>')


def write_vsdx(pages, out, template):
    zin = zipfile.ZipFile(template)
    files = {n: zin.read(n) for n in zin.namelist()}
    # drop the template's page, masters, thumbnail and custom props
    for n in list(files):
        if n.startswith('visio/pages/') or n.startswith('visio/masters/') or n in ('docProps/thumbnail.emf', 'docProps/custom.xml'):
            del files[n]
    ct = files['[Content_Types].xml'].decode()
    ct = re.sub(r'<Override PartName="/visio/(pages|masters)/[^"]*"[^>]*/>', '', ct)
    ct = re.sub(r'<Override PartName="/docProps/custom.xml"[^>]*/>', '', ct)
    ct = ct.replace('<Default Extension="emf" ContentType="image/x-emf"/>', '')
    over = '<Override PartName="/visio/pages/pages.xml" ContentType="application/vnd.ms-visio.pages+xml"/>'
    for i in range(len(pages)):
        over += f'<Override PartName="/visio/pages/page{i + 1}.xml" ContentType="application/vnd.ms-visio.page+xml"/>'
    ct = ct.replace('</Types>', over + '</Types>')
    files['[Content_Types].xml'] = ct.encode()
    rels = files['_rels/.rels'].decode()
    rels = re.sub(r'<Relationship Id="rId[25]"[^>]*/>', '', rels)
    files['_rels/.rels'] = rels.encode()
    dr = files['visio/_rels/document.xml.rels'].decode()
    dr = re.sub(r'<Relationship Id="rId1"[^>]*masters[^>]*/>', '', dr)
    files['visio/_rels/document.xml.rels'] = dr.encode()
    doc = files['visio/document.xml'].decode()
    doc = doc.replace("<FaceNames>", "<FaceNames><FaceName NameU='Arial' UnicodeRanges='-536859905 -1073711037 9 0' "
                      "CharSets='1073742335 -65536' Panose='2 11 6 4 2 2 2 2 2 4' Flags='325'/>", 1)
    files['visio/document.xml'] = doc.encode()
    win = files['visio/windows.xml'].decode()
    win = re.sub(r"ViewCenterX='[^']*' ViewCenterY='[^']*'", "ViewCenterX='11.7' ViewCenterY='8.27'", win)
    files['visio/windows.xml'] = win.encode()
    ps, rr = [], []
    for i, (pg, shapes) in enumerate(pages):
        ps.append(f"<Page ID='{i}' NameU='{escape(pg.name)}' Name='{escape(pg.name)}' ViewScale='-1' "
                  f"ViewCenterX='{fmt(pg.w * MM / 2)}' ViewCenterY='{fmt(pg.h * MM / 2)}'>"
                  "<PageSheet LineStyle='0' FillStyle='0' TextStyle='0'>"
                  + cell('PageWidth', fmt(pg.w * MM), 'MM') + cell('PageHeight', fmt(pg.h * MM), 'MM')
                  + cell('PageScale', fmt(MM), 'MM') + cell('DrawingScale', fmt(MM), 'MM')
                  + cell('DrawingSizeType', 3) + cell('DrawingScaleType', 0) + cell('PrintPageOrientation', 2)
                  + cell('PageLeftMargin', 0) + cell('PageRightMargin', 0) + cell('PageTopMargin', 0)
                  + cell('PageBottomMargin', 0)
                  + f"</PageSheet><Rel r:id='rId{i + 1}'/></Page>")
        rr.append(f'<Relationship Id="rId{i + 1}" Type="http://schemas.microsoft.com/visio/2010/relationships/page" '
                  f'Target="page{i + 1}.xml"/>')
        files[f'visio/pages/page{i + 1}.xml'] = page_xml(shapes).encode('utf-8')
    files['visio/pages/pages.xml'] = ("<?xml version='1.0' encoding='utf-8' ?>\n<Pages xmlns='http://schemas.microsoft.com/office/visio/2012/main' "
                                      "xmlns:r='http://schemas.openxmlformats.org/officeDocument/2006/relationships' xml:space='preserve'>"
                                      + ''.join(ps) + '</Pages>').encode('utf-8')
    files['visio/pages/_rels/pages.xml.rels'] = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
                                                '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                                                + ''.join(rr) + '</Relationships>').encode()
    app = files['docProps/app.xml'].decode()
    app = re.sub(r'<HeadingPairs>.*</TitlesOfParts>',
                 '<HeadingPairs><vt:vector size="2" baseType="variant"><vt:variant><vt:lpstr>Pages</vt:lpstr></vt:variant>'
                 f'<vt:variant><vt:i4>{len(pages)}</vt:i4></vt:variant></vt:vector></HeadingPairs>'
                 f'<TitlesOfParts><vt:vector size="{len(pages)}" baseType="lpstr">'
                 + ''.join(f'<vt:lpstr>{escape(pg.name)}</vt:lpstr>' for pg, _ in pages) + '</vt:vector></TitlesOfParts>', app, flags=re.S)
    files['docProps/app.xml'] = app.encode()
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as z:
        order = ['[Content_Types].xml'] + sorted(n for n in files if n != '[Content_Types].xml')
        for n in order:
            z.writestr(n, files[n])


def main(proj, out):
    proj = Path(proj)
    sheets = [('DDS_47_51.kicad_sch', 'Лист 1 — DDS'), ('02_controller.kicad_sch', 'Лист 2 — Контроллер'),
              ('03_power.kicad_sch', 'Лист 3 — Питание')]
    import vsdx
    template = Path(vsdx.__file__).parent / 'media' / 'media.vsdx'
    pages = []
    for i, (f, title) in enumerate(sheets):
        sh = Sheet(proj / f)
        pg, shapes = build_page(sh, i + 1, len(sheets), proj / 'lib' / 'GOST_E3.kicad_wks', title)
        pages.append((pg, shapes))
        print(f'{f}: {len(shapes)} shapes')
    write_vsdx(pages, out, template)
    print('wrote', out)


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
