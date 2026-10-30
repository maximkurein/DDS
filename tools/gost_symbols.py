"""ESKD (GOST 2.7xx) style symbols for the DDS project library `DDS`.

Conventions
  * connection points on the 1.27 mm grid, IC pin pitch 5.08 mm (GOST 2.743
    asks for >= 5 mm between pin lines);
  * microcircuits: outer frame with left/right label fields and a central
    function field; active-low pins drawn with an inversion circle;
  * two-terminal parts are drawn vertical (pin 1 on top) and are rotated by
    90 deg on the sheet when used horizontally.
"""
from kisch import Symbol, Pin, poly, rect, circle, arc, text

P = 5.08          # IC pin pitch
PL = 2.54         # IC pin length
W2 = 0.254        # normal line width
WB = 0.4          # bold line width (capacitor plates etc.)


def _two_pin(name, prefix, desc, half=5.08, value=None):
    s = Symbol(name, prefix, value or name, hide_pin_names=True, hide_pin_numbers=True,
               ref_at=(2.54, 1.27, 'left'), val_at=(2.54, -1.27, 'left'), desc=desc)
    return s


def resistor():
    s = _two_pin('R', 'R', 'Резистор')
    s.gfx(1, rect(-1.27, -3.175, 1.27, 3.175, W2))
    s.pin(1, '1', '~', 0, 5.08, 270, 1.905)
    s.pin(1, '2', '~', 0, -5.08, 90, 1.905)
    return s


def capacitor():
    s = _two_pin('C', 'C', 'Конденсатор')
    s.ref_at = (2.54, 1.27, 'left'); s.val_at = (2.54, -1.27, 'left')
    s.gfx(1, poly([(-1.524, 0.508), (1.524, 0.508)], WB))
    s.gfx(1, poly([(-1.524, -0.508), (1.524, -0.508)], WB))
    s.pin(1, '1', '~', 0, 2.54, 270, 2.032)
    s.pin(1, '2', '~', 0, -2.54, 90, 2.032)
    return s


def capacitor_pol():
    """Polarized (tantalum/electrolytic) capacitor, ГОСТ 2.728: '+' at pin 1."""
    s = _two_pin('C_Pol', 'C', 'Конденсатор полярный')
    s.ref_at = (2.54, 1.27, 'left'); s.val_at = (2.54, -1.27, 'left')
    s.gfx(1, poly([(-1.524, 0.508), (1.524, 0.508)], WB))
    s.gfx(1, rect(-1.524, -0.381, 1.524, -0.635, W2, fill='outline'))
    s.gfx(1, text('+', -2.032, 2.032, 1.27))
    s.pin(1, '1', '~', 0, 2.54, 270, 2.032)
    s.pin(1, '2', '~', 0, -2.54, 90, 2.032)
    return s


def buffer_block(name, prefix, groups, power, desc, pitch=15.24):
    """ГОСТ 2.743 multi-channel 3-state buffer: one unit per group of channels
    with a common control section (EN) on top; a separate supply unit.
    groups: {unit: (en_pin, en_inverted, [(a_pin, y_pin), ...])}"""
    s = Symbol(name, prefix, name, pin_name_offset=0.762, ref_at=(0, 0, None), val_at=(0, 0, None),
               desc=desc)
    s.geom = {}
    wl, wc, wr = 5.08, 10.16, 0
    W = wl + wc + wr
    for u, (en, inv, chans) in groups.items():
        hc = 10.16
        H = hc + pitch * len(chans)
        s.gfx(u, rect(0, 0, W, -H, W2))
        s.gfx(u, poly([(wl, 0), (wl, -H)], W2))
        s.gfx(u, poly([(0, -hc), (W, -hc)], W2))
        s.gfx(u, text('1', wl + wc / 2, -2.54, 2.0))
        s.gfx(u, text('▽', wl + wc / 2, -6.35, 1.27))
        s.pin(u, en, 'EN', -PL, -5.08, 0, PL, etype='input', shape='inverted' if inv else 'line')
        for k, (a, y) in enumerate(chans):
            yy = -hc - pitch * k - pitch / 2
            if k:
                s.gfx(u, poly([(wl, -hc - pitch * k), (W, -hc - pitch * k)], W2))
            s.pin(u, a, '~', -PL, yy, 0, PL, etype='input')
            s.pin(u, y, '~', W + PL, yy, 180, PL, etype='tri_state')
        s.geom[u] = dict(W=W, H=H, rows=len(chans) + 1)
    pu = max(groups) + 1
    vcc, gnd = power
    s.gfx(pu, rect(0, 0, 17.78, -10.16, W2))
    s.gfx(pu, poly([(7.62, 0), (7.62, -10.16)], W2))
    s.gfx(pu, poly([(10.16, 0), (10.16, -10.16)], W2))
    s.pin(pu, vcc, 'VCC', -PL, -5.08, 0, PL, etype='power_in')
    s.pin(pu, gnd, 'GND', 17.78 + PL, -5.08, 180, PL, etype='power_in')
    s.geom[pu] = dict(W=17.78, H=10.16, rows=1)
    s.ref_at = (W / 2, 3.81, None)
    s.val_at = (W / 2, 1.524, None)
    return s


def schmitt_inverters(name, prefix, gates, power, desc):
    """Hex Schmitt-trigger inverter (74ACT14 type), one unit per gate, ГОСТ 2.743:
    function '1', hysteresis sign, inversion circle at the output."""
    s = Symbol(name, prefix, name, pin_name_offset=0.762, ref_at=(0, 0, None), val_at=(0, 0, None),
               desc=desc)
    s.geom = {}
    W, H = 10.16, 10.16
    for u, (a, y) in enumerate(gates, 1):
        s.gfx(u, rect(0, 0, W, -H, W2))
        s.gfx(u, text('1', 3.81, -2.54, 2.0))
        # hysteresis loop
        s.gfx(u, poly([(4.445, -7.62), (6.35, -7.62), (6.35, -5.715), (7.62, -5.715)], W2))
        s.gfx(u, poly([(5.08, -7.62), (5.08, -5.715), (6.985, -5.715)], W2))
        s.pin(u, a, '~', -PL, -5.08, 0, PL, etype='input')
        s.pin(u, y, '~', W + PL, -5.08, 180, PL, etype='output', shape='inverted')
        s.geom[u] = dict(W=W, H=H, rows=1)
    pu = len(gates) + 1
    vcc, gnd = power
    s.gfx(pu, rect(0, 0, 17.78, -10.16, W2))
    s.gfx(pu, poly([(7.62, 0), (7.62, -10.16)], W2))
    s.gfx(pu, poly([(10.16, 0), (10.16, -10.16)], W2))
    s.pin(pu, vcc, 'VCC', -PL, -5.08, 0, PL, etype='power_in')
    s.pin(pu, gnd, 'GND', 17.78 + PL, -5.08, 180, PL, etype='power_in')
    s.geom[pu] = dict(W=17.78, H=10.16, rows=1)
    s.ref_at = (W / 2, 3.81, None)
    s.val_at = (W / 2, 1.524, None)
    return s


def inductor(core=False):
    name = 'L_Fe' if core else 'L'
    s = _two_pin(name, 'L', 'Дроссель с ферритовым сердечником' if core else 'Катушка индуктивности')
    for k in range(4):
        y0 = 2.54 - 1.27 * k
        s.gfx(1, arc(0, y0, 0.635, y0 - 0.635, 0, y0 - 1.27, W2))
    if core:
        s.gfx(1, poly([(-0.762, 2.54), (-0.762, -2.54)], W2))
    s.pin(1, '1', '~', 0, 5.08, 270, 2.54)
    s.pin(1, '2', '~', 0, -5.08, 90, 2.54)
    return s


def schottky():
    # horizontal: anode (pin 2) left, cathode (pin 1) right
    s = Symbol('VD_Schottky', 'VD', 'VD_Schottky', hide_pin_names=True, hide_pin_numbers=True,
               ref_at=(0, 3.175, None), val_at=(0, -3.175, None), desc='Диод Шоттки')
    s.gfx(1, poly([(-1.27, 1.524), (-1.27, -1.524), (1.27, 0), (-1.27, 1.524)], W2))
    s.gfx(1, poly([(1.778, 1.016), (1.778, 1.524), (1.27, 1.524), (1.27, -1.524),
                   (0.762, -1.524), (0.762, -1.016)], W2))
    s.pin(1, '2', 'A', -3.81, 0, 0, 2.54)
    s.pin(1, '1', 'K', 3.81, 0, 180, 2.54)
    return s


def fuse():
    s = Symbol('F', 'F', 'F', hide_pin_names=True, hide_pin_numbers=True,
               ref_at=(0, 2.54, None), val_at=(0, -2.54, None), desc='Вставка плавкая')
    s.gfx(1, rect(-2.54, -1.016, 2.54, 1.016, W2))
    s.gfx(1, poly([(-2.54, 0), (2.54, 0)], W2))
    s.pin(1, '1', '~', -5.08, 0, 0, 2.54)
    s.pin(1, '2', '~', 5.08, 0, 180, 2.54)
    return s


def _switch_gfx(s, u):
    s.gfx(u, poly([(-2.54, 0), (2.286, 1.905)], W2))
    s.gfx(u, poly([(2.54, 0), (2.54, 0.635)], W2))


def switch_dip(n):
    """DIP switch with n independent sections; section k: pins k and 2n+1-k."""
    name = f'S_DIP_x{n:02d}' if n > 1 else 'S_DIP_x01'
    s = Symbol(name, 'S', name, hide_pin_names=True, hide_pin_numbers=False,
               ref_at=(0, 3.81, None), val_at=(0, -2.54, None),
               desc=f'Переключатель DIP, {n} секц.')
    s.num_size = 1.0
    for k in range(1, n + 1):
        _switch_gfx(s, k)
        s.pin(k, str(k), '~', -5.08, 0, 0, 2.54)
        s.pin(k, str(2 * n + 1 - k), '~', 5.08, 0, 180, 2.54)
    return s


def button():
    s = Symbol('SB', 'S', 'SB', hide_pin_names=True, hide_pin_numbers=True,
               ref_at=(0, 5.08, None), val_at=(0, -2.54, None), desc='Кнопка')
    s.gfx(1, poly([(-2.54, 0), (-2.54, 0.762)], W2))
    s.gfx(1, poly([(2.54, 0), (2.54, 0.762)], W2))
    s.gfx(1, poly([(-3.048, 1.524), (3.048, 1.524)], W2))
    s.gfx(1, poly([(0, 1.524), (0, 3.302)], W2))
    s.gfx(1, poly([(-1.016, 3.302), (1.016, 3.302)], W2))
    s.pin(1, '1', '~', -5.08, 0, 0, 2.54)
    s.pin(1, '2', '~', 5.08, 0, 180, 2.54)
    return s


def coax():
    s = Symbol('XW_Coax', 'XW', 'XW_Coax', hide_pin_names=True, hide_pin_numbers=True,
               ref_at=(0, 3.175, None), val_at=(0, -5.08, None),
               desc='Соединитель коаксиальный')
    s.gfx(1, circle(0, 0, 1.524, W2))
    s.gfx(1, circle(0, 0, 0.3, W2, 'outline'))
    s.gfx(1, poly([(-2.54, 0), (-0.3, 0)], W2))
    s.pin(1, '1', '~', -5.08, 0, 0, 2.54)
    s.pin(1, '2', '~', 0, -3.81, 90, 2.286)
    return s


def connector(n, name=None):
    """Connector drawn as a contact table (pins on the left)."""
    name = name or f'X_{n}'
    s = Symbol(name, 'X', name, hide_pin_names=True, hide_pin_numbers=True,
               ref_at=(2.54, 2.54, None), val_at=(2.54, -(n * P) - 1.27, None),
               desc=f'Соединитель, {n} конт.')
    w = 5.08
    top = P / 2
    s.gfx(1, rect(0, top, w, top - n * P, W2))
    for i in range(n):
        y = -i * P
        if i:
            s.gfx(1, poly([(0, y + P / 2), (w, y + P / 2)], W2))
        s.gfx(1, text(str(i + 1), w / 2, y, 1.27))
        s.pin(1, str(i + 1), str(i + 1), -PL, y, 0, PL)
    return s


# ---------------------------------------------------------------- power ---

def power_port(net, gnd=False):
    s = Symbol(net, '#PWR', net, power=True, hide_pin_names=True, hide_pin_numbers=True,
               pin_name_offset=0, ref_at=(0, -6.35, None), val_at=(0, 3.556, None),
               in_bom=False, desc=f'Цепь питания {net}')
    if gnd:
        s.val_at = (0, -5.08, None)
        s.gfx(1, poly([(0, 0), (0, -1.27)], W2))
        s.gfx(1, poly([(-1.905, -1.27), (1.905, -1.27)], W2))
        s.gfx(1, poly([(-1.143, -1.905), (1.143, -1.905)], W2))
        s.gfx(1, poly([(-0.381, -2.54), (0.381, -2.54)], W2))
        s.pin(1, '1', '', 0, 0, 270, 0, etype='power_in')
    else:
        s.gfx(1, poly([(0, 0), (0, 1.905)], W2))
        s.gfx(1, poly([(-1.524, 1.905), (1.524, 1.905)], WB))
        s.pin(1, '1', '', 0, 0, 90, 0, etype='power_in')
    return s


def pwr_flag():
    s = Symbol('PWR_FLAG', '#FLG', 'PWR_FLAG', power=True, hide_pin_names=True,
               hide_pin_numbers=True, pin_name_offset=0, ref_at=(0, 1.905, None),
               val_at=(0, 3.81, None), in_bom=False,
               desc='Флаг ERC: цепь питается от источника через пассивный элемент')
    s.gfx(1, poly([(0, 0), (0, 1.27), (-1.016, 1.905), (0, 2.54), (1.016, 1.905), (0, 1.27)], 0))
    s.pin(1, '1', '', 0, 0, 90, 0, etype='power_out')
    return s


# ------------------------------------------------------------------- ICs ---

class Grp(list):
    """A group of pins: list of (num, name, etype[, 'inv'])."""


def ic(name, prefix, units, value=None, desc='', unit_names=None):
    """units: {u: dict(title=..., left=[groups], right=[groups], wl=, wc=, wr=)}"""
    s = Symbol(name, prefix, value or name, pin_name_offset=0.762,
               ref_at=(0, 0, None), val_at=(0, 0, None), desc=desc, unit_names=unit_names)
    s.geom = {}
    for u, d in units.items():
        wl, wc, wr = d.get('wl', 12.7), d.get('wc', 10.16), d.get('wr', 12.7)
        W = wl + wc + wr

        def rows(groups):
            out, r = [], 0
            for gi, g in enumerate(groups):
                if gi:
                    out.append(('gap', r)); r += d.get('gap', 1)
                for p in g:
                    out.append(('pin', r, p)); r += 1
            return out, r
        lrows, nl = rows(d.get('left', []))
        rrows, nr = rows(d.get('right', []))
        n = max(nl, nr)
        H = (n + 1) * P
        top = 0
        s.gfx(u, rect(0, top, W, top - H, W2))
        if wl and d.get('left'):
            s.gfx(u, poly([(wl, top), (wl, top - H)], W2))
        if wr and d.get('right'):
            s.gfx(u, poly([(wl + wc, top), (wl + wc, top - H)], W2))
        title = d.get('title', '')
        if title:
            s.gfx(u, text(title, wl + wc / 2, top - 3.81, 2.0, bold=False))
        for sub in d.get('subtitle', []):
            s.gfx(u, text(sub[0], wl + wc / 2, top - sub[1], 1.27))

        gap = d.get('gap', 1)

        def place(rowlist, side):
            for item in rowlist:
                if item[0] == 'gap':
                    y = top - (item[1] + 1 + (gap - 1) / 2) * P
                    x0, x1 = (0, wl) if side == 'L' else (wl + wc, W)
                    s.gfx(u, poly([(x0, y), (x1, y)], W2))
                    continue
                _, r, p = item
                num, nm, et = p[0], p[1], p[2]
                shape = 'inverted' if len(p) > 3 and p[3] == 'inv' else 'line'
                y = top - (r + 1) * P
                if side == 'L':
                    s.pin(u, num, nm, -PL, y, 0, PL, etype=et, shape=shape)
                else:
                    s.pin(u, num, nm, W + PL, y, 180, PL, etype=et, shape=shape)
        place(lrows, 'L')
        place(rrows, 'R')
        s.geom[u] = dict(W=W, H=H, rows=n)
    # reference / value above the frame of unit 1
    W1 = s.geom[min(units)]['W']
    s.ref_at = (W1 / 2, 3.81, None)
    s.val_at = (W1 / 2, 1.524, None)
    return s


def g(*pins):
    return Grp(pins)


def build_library():
    lib = {}

    def add(s):
        lib[s.name] = s
        return s
    add(resistor()); add(capacitor()); add(inductor()); add(inductor(core=True))
    add(schottky()); add(fuse()); add(button()); add(coax()); add(capacitor_pol())
    add(switch_dip(1)); add(switch_dip(4)); add(switch_dip(8))
    add(connector(2, 'X_2')); add(connector(6, 'X_6'))
    for net in ['+15V', '+5V', '+3V3', '+3V3_A', '+3V3_OSC', '+1V8', '+1V8_Q']:
        add(power_port(net))
    add(power_port('GND', gnd=True))
    add(pwr_flag())

    I, O, B, PW, PS, T, OC, NC = ('input', 'output', 'bidirectional', 'power_in', 'passive',
                                  'tri_state', 'open_collector', 'no_connect')
    # ---- 1508ПЛ8Т (DDS), two units: signals / supply
    sel1 = g(*[(n, f'SEL1[{i}]', B) for i, n in enumerate(['67', '68', '70', '71', '73', '74'])])
    sel2 = g(*[(n, f'SEL2[{i}]', B) for i, n in enumerate(['56', '57', '58', '59', '62', '63'])])
    data = g(*[(n, f'D{i}', B) for i, n in enumerate(
        ['78', '79', '80', '81', '83', '84', '87', '88', '89', '90', '92', '93', '95', '96', '97', '98'])])
    add(ic('1508PL8T', 'D', {
        1: dict(title='DDS', wl=15.24, wc=10.16, wr=12.7,
                left=[g(('24', 'CLKP', I), ('23', 'CLKM', I), ('18', 'CLKDP', I),
                        ('19', 'CLKDM', I), ('13', 'CSEL', I)),
                      g(('2', 'SCK', I), ('3', 'SDI', I), ('6', 'SCS', I, 'inv')),
                      g(('10', 'RST', I, 'inv'), ('8', 'CS', I, 'inv'), ('76', 'RD', I, 'inv'),
                        ('77', 'WR', I, 'inv'), ('11', 'ADR', I)),
                      g(('37', 'IREF1', PS), ('28', 'IREF2', PS)),
                      g(('50', 'CMP_INP', I), ('49', 'CMP_INM', I)),
                      sel1, sel2],
                right=[g(('43', 'OUTP1', O), ('42', 'OUTM1', O)),
                       g(('34', 'OUTP2', O), ('33', 'OUTM2', O)),
                       g(('52', 'CMP_OP', O), ('53', 'CMP_OM', O)),
                       g(('5', 'SDO', T), ('7', 'SSCS', O, 'inv'), ('100', 'CSYNC', B)),
                       data]),
        2: dict(title='', wl=12.7, wc=5.08, wr=12.7,
                left=[g(*[(n, 'CVDD', PW) for n in ['1', '14', '15', '60', '61', '75', '82']]),
                      g(('16', 'DVDD', PW), ('47', 'DVDD', PW), ('45', 'DVDD1', PW), ('36', 'DVDD2', PW)),
                      g(*[(n, 'PVDD', PW) for n in ['9', '69', '94']]),
                      g(('21', 'AVDD', PW), ('26', 'AVDD', PW), ('27', 'AVDD', PW), ('39', 'AVDD1', PW),
                        ('40', 'AVDD1', PW), ('30', 'AVDD2', PW), ('31', 'AVDD2', PW), ('51', 'CMP_VDD', PW))],
                right=[g(*[(n, 'AGND', PW) for n in ['17', '20', '22', '25', '29', '32', '35', '38', '41', '44', '46']]),
                       g(*[(n, 'PGND', PW) for n in ['12', '66', '91']]),
                       g(*[(n, 'CGND', PW) for n in ['4', '55', '64', '65', '72', '85', '86', '99']]),
                       g(('48', 'CMP_GND', PW), ('54', 'CMP_GND', PW))],
                gap=3),
    }, desc='Синтезатор DDS 1508ПЛ8Т'))
    # ---- 1887ВЕ7Т (MCU)
    port = lambda p, nums: g(*[(n, f'P{p}{i}', B) for i, n in enumerate(nums)])
    add(ic('1887VE7T', 'D', {
        1: dict(title='MCU', wl=12.7, wc=10.16, wr=12.7,
                left=[port('A', ['51', '50', '49', '48', '47', '46', '45', '44']),
                      port('B', ['10', '11', '12', '13', '14', '15', '16', '17']),
                      port('C', ['35', '36', '37', '38', '39', '40', '41', '42']),
                      port('D', ['25', '26', '27', '28', '29', '30', '31', '32']),
                      g(('1', 'PEN', I, 'inv'), ('20', 'RESET', I, 'inv'), ('24', 'BQ1', I), ('23', 'BQ2', O))],
                right=[port('E', ['2', '3', '4', '5', '6', '7', '8', '9']),
                       port('F', ['61', '60', '59', '58', '57', '56', '55', '54']),
                       g(*[(n, f'PG{i}', B) for i, n in enumerate(['33', '34', '43', '18', '19'])]),
                       g(('62', 'AREF', PS)),
                       g(('21', 'VCC', PW), ('52', 'VCC', PW), ('64', 'AVCC', PW)),
                       g(('22', 'GND', PW), ('53', 'GND', PW), ('63', 'AGND', PW))]),
    }, desc='Микроконтроллер 1887ВЕ7Т'))
    # ---- rev E: domestic logic and power
    add(buffer_block('1564AP3', 'D', {
        1: ('1', True, [('2', '18'), ('4', '16'), ('6', '14'), ('8', '12')]),
        2: ('19', False, [('11', '9'), ('13', '7'), ('15', '5'), ('17', '3')]),
    }, ('20', '10'), 'Буфер 2×4 с Z-состоянием 1564АП3 (аналог 74HC241)', pitch=20.32))
    add(schmitt_inverters('1594TL2T', 'D', [('1', '2'), ('3', '4'), ('5', '6'), ('9', '8'),
                                             ('11', '10'), ('13', '12')], ('14', '7'),
                          '6 инверторов с триггером Шмитта 1594ТЛ2Т (аналог 74ACT14)'))
    add(ic('1393EU014', 'D', {
        1: dict(title='DC/DC', wl=7.62, wc=12.7, wr=7.62,
                left=[g(('1', 'VIN1', PW), ('2', 'VIN2', PW), ('3', 'VIN3', PW)),
                      g(('6', 'CL', PS), ('7', 'ON', I)),
                      g(('9', 'SS', PS), ('8', 'EXT', PS)),
                      g(('5', 'GND1', PW), ('11', 'GND2', PW), ('12', 'GND3', PW))],
                right=[g(('14', 'SW1', PS), ('15', 'SW2', PS), ('4', 'BST', PS)),
                       g(('10', 'FB', I)), g(('13', 'PG', OC)), g(('16', 'NC', NC))]),
    }, desc='Понижающий DC/DC 1393ЕУ014, 9–20 В / 2,4 А, 260 кГц'))
    add(ic('LDO_1158', 'D', {
        1: dict(title='LDO', wl=7.62, wc=7.62, wr=7.62,
                left=[g(('1', 'IN', PW)), g(('2', 'GND', PW))], right=[g(('3', 'OUT', 'power_out'))]),
    }, desc='Стабилизатор 1158ЕН3.3ВХ, 3,3 В / 0,5 А'))
    add(ic('LDO_1325', 'D', {
        1: dict(title='LDO', wl=7.62, wc=7.62, wr=7.62,
                left=[g(('2', 'IN', PW)), g(('3', 'GND', PW))], right=[g(('1', 'OUT', 'power_out'))]),
    }, desc='Стабилизатор 1325ЕН1.8У, 1,8 В / 0,8 А'))
    # ---- GK361-TK-02 (UNVERIFIED pinout)
    add(ic('GK361_UNVERIFIED', 'G', {
        1: dict(title='G', wl=7.62, wc=7.62, wr=10.16, subtitle=[('LVDS', 6.35)],
                left=[g(('1', 'EN?', I), ('2', 'NC?', NC)), g(('6', 'VCC?', PW), ('3', 'GND?', PW))],
                right=[g(('4', 'OUT+?', O), ('5', 'OUT-?', O))]),
    }, desc='Генератор 480 МГц ГК361-ТК-02 — распиновка НЕ ПОДТВЕРЖДЕНА'))
    return lib


def write_library(lib, path):
    body = '\n  '.join(s.sexpr() for s in lib.values())
    txt = ('(kicad_symbol_lib (version 20250114) (generator "dds_build") (generator_version "1")\n  '
           + body + '\n)\n')
    open(path, 'w', encoding='utf-8').write(txt)


if __name__ == '__main__':
    import sys
    write_library(build_library(), sys.argv[1] if len(sys.argv) > 1 else 'DDS.kicad_sym')
