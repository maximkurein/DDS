#!/usr/bin/env python3
"""Generate the DDS 47-51 MHz KiCad project (rev D) from the design data.

    python3 tools/dds_build.py <project_dir> <revC_netlist.xml>

Writes into <project_dir>:
  lib/DDS.kicad_sym, lib/GOST_E3.kicad_wks, sym-lib-table,
  DDS_47_51.kicad_sch, 02_controller.kicad_sch, 03_power.kicad_sch,
  DDS_47_51.kicad_pro (schematic settings merged into an existing file),
  verification/refmap_revC_to_revD.json

Connectivity source of truth is the rev C netlist: every pin of the new
drawing takes its net from it (with the DIP-switch regrouping mapping), and
the result is checked by tools/netcompare.py against the same netlist.
"""
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from kisch import q, suid, effects, f  # noqa: E402
from gost_symbols import build_library, write_library  # noqa: E402
from sheet import Sheet, POWER_NETS  # noqa: E402
from dds_parts import part, old_to_new, gost_prefix, PARTS  # noqa: E402
from gost_wks import write_wks  # noqa: E402

PROJECT = 'DDS_47_51'
LIB = build_library()
INSTS = {}
TWO_PIN = {'R', 'C', 'L', 'L_Fe', 'F'}
NET = {}            # (key, pin) -> net name
SHEET_OF = {}       # key -> page

TB = dict(
    title='Синтезатор частоты 47–51 МГц',
    company='',
    rev='D',
    date='2026-10-30',
    c1='ХХХХ.ХХХХХХ.ХХХ Э3',
    c2='Схема электрическая принципиальная',
    c3='', c4='', c5='', c6='', c7='',
    c8='ЧЕРНОВИК — не для производства',
)


def load_netlist(path):
    root = ET.parse(path).getroot()
    for n in root.iter('net'):
        name = n.get('name')
        for x in n.iter('node'):
            key, unit, pin = old_to_new(x.get('ref'), x.get('pin'))
            NET[(key, pin)] = name


# ---------------------------------------------------------------- helpers --

def put(sh, key, x, y, ang=0, unit=1, mirror=None, fields='auto', hide_value=False):
    p = part(key)
    sym = LIB[p['sym']]
    i = sh.place(key, sym, x, y, ang, unit, mirror)
    i.value = p['value']
    i.footprint = p['fp']
    i.dnp = p.get('dnp', False)
    i.extra['_hide_value'] = hide_value or key in ('DIP8', 'DIP4')
    for k_src, k_dst in (('tol', 'Допуск'), ('volt', 'Напряжение'), ('diel', 'ТКЕ'),
                         ('pwr', 'Мощность'), ('note', 'Примечание')):
        if p.get(k_src):
            i.extra[k_dst] = p[k_src]
    INSTS[(key, unit)] = i
    SHEET_OF[key] = sh.page
    place_fields(i, fields)
    return i


def place_fields(i, mode='auto'):
    name = i.sym.name
    x, y, a = i.x, i.y, i.ang % 360
    if mode != 'auto':
        i.ref_pos, i.val_pos = mode
        return
    if name in ('R', 'C', 'L', 'L_Fe'):
        if a in (0, 180):
            i.ref_pos = (x + 2.54, y - 0.635, 'left bottom')
            i.val_pos = (x + 2.54, y + 0.635, 'left top')
        else:
            h = 2.032 if name == 'C' else 1.651
            i.ref_pos = (x, y - h, 'bottom')
            i.val_pos = (x, y + h, 'top')
    elif name in ('VD_Schottky', 'F', 'SB', 'XW_Coax') or name.startswith('S_DIP'):
        if a in (0, 180):
            up = 3.81 if name.startswith('S_DIP') or name == 'SB' else 2.286
            if name == 'SB':
                up = 4.064
            i.ref_pos = (x, y - up, 'bottom')
            i.val_pos = (x, y + 2.032, 'top')
            if name == 'XW_Coax':
                i.ref_pos = (x, y - 2.286, 'bottom')
                i.val_pos = (x + 2.54, y + 2.54, 'left top')
        else:
            i.ref_pos = (x + 3.048, y - 0.635, 'left bottom')
            i.val_pos = (x + 3.048, y + 0.635, 'left top')
    elif name.startswith('X_'):
        dx = -2.54 if i.mirror == 'y' else 2.54
        n = len(i.sym.units[1]['pins'])
        i.ref_pos = (x + dx, y - 3.81, 'bottom')
        i.val_pos = (x + dx, y + (n - 0.5) * 5.08 + 1.27, 'top')
    else:   # ICs: origin at top-left of the frame
        W = i.sym.geom[i.unit]['W']
        H = i.sym.geom[i.unit]['H']
        i.ref_pos = (x + W / 2, y - 3.048, 'bottom')
        i.val_pos = (x + W / 2, y - 0.762, 'bottom')
        if i.unit != 1:
            i.val_pos = (x + W / 2, y + H + 0.762, 'top')
            i.extra['_hide_value'] = True


def term(sh, inst, pin, length=2.54):
    """Terminate a pin by its net: stub + power port / label, or a no-connect.
    For symmetric two-pin parts the termination is recorded by location and
    resolved after the orientation fix (pin numbers may still swap)."""
    if inst.sym.name in TWO_PIN and not getattr(sh, 'final', False):
        sh.deferred.append((inst, inst.pin_xy(pin), length))
        return
    key = (inst.key, str(pin))
    net = NET[key]
    if net.startswith('unconnected'):
        sh.nc(inst, pin)
        return
    (ex, ey), d = sh.stub(inst, pin, length)
    if net in POWER_NETS:
        sh.power(net, ex, ey, d)
    else:
        sh.label(net, ex, ey, d, kind='global' if is_global(net) else 'label')
    sh.done_pins.add(key)
    sh.term_pts.add(inst.pin_xy(pin))


def is_global(net):
    pages = {SHEET_OF.get(k) for (k, p), n in NET.items() if n == net}
    return len(pages) > 1


def wl(sh, net, x, y, d):
    """Label on an existing wire point."""
    if net in POWER_NETS:
        sh.power(net, x, y, d)
    else:
        sh.label(net, x, y, d, kind='global' if is_global(net) else 'label')


def autoterm(sh):
    """Called by the layout functions; real work happens in finish()."""
    sh.want_autoterm = True


def finish(sh):
    """After the orientation fix: resolve deferred terminations, then give every
    still unconnected pin a stub with its net label / power port / NC flag."""
    sh.final = True
    for inst, xy, length in sh.deferred:
        pin = next(p for p in inst.pins() if inst.pin_xy(p) == xy)
        term(sh, inst, pin, length)
    sh.split_wires()
    ends = set()
    for x1, y1, x2, y2 in sh.wires:
        ends.add((x1, y1)); ends.add((x2, y2))
    for i in sh.insts:
        for p in i.pins():
            xy = i.pin_xy(p)
            if xy in ends or xy in sh.term_pts or (i.key, p) in sh.done_pins:
                continue
            term(sh, i, p)


def row(origin_y, r):
    return origin_y + 5.08 * r


# ===================================================== sheet 3: power ====

def sheet_power(sh):
    Y = 60.96
    x4 = put(sh, 'J1', 35.56, Y, mirror='y')
    f1 = put(sh, 'F1', 50.8, Y)
    vd = put(sh, 'D1', 66.04, Y)
    sh.connect(x4, 1, f1, 1)
    sh.connect(f1, 2, vd, 2)
    g2 = x4.pin_xy(2)
    sh.pin_wire(x4, 2, (g2[0] + 7.62, g2[1]), (g2[0] + 7.62, g2[1] + 2.54))
    sh.power('GND', g2[0] + 7.62, g2[1] + 2.54, (0, 1))
    sh.flag(g2[0] + 5.08, g2[1], (0, -1))
    sh.text('+15 В ±5 %', 22.86, Y - 8.89, 1.524)

    def converter(key, x0, cin, cin2, lkey, cout, vin_net, out_net, pg_label, cin_x):
        d = put(sh, key, x0, Y - 5.08)
        vin = d.pin_xy(2)
        # input rail from the left
        sh.pin_wire(d, 2, (cin_x - 7.62, vin[1]))
        en = d.pin_xy(3)
        sh.pin_wire(d, 3, (vin[0] - 2.54, en[1]), (vin[0] - 2.54, vin[1]))
        cx = cin_x
        for ck in (cin, cin2):
            c = put(sh, ck, cx, Y + 5.08)
            sh.pin_wire(c, 1, (cx, Y))
            term(sh, c, 2, 0)
            cx += 10.16
        # grounds: FB, AGND, PGND, EP -> common wire
        gx = vin[0] - 2.54
        ys = []
        for p in ('5', '4', '1', '9'):
            px, py = d.pin_xy(p)
            sh.pin_wire(d, p, (gx, py))
            ys.append(py)
        sh.wire((gx, min(ys)), (gx, max(ys) + 2.54))
        sh.power('GND', gx, max(ys) + 2.54, (0, 1))
        # output: SW -> L -> rail; VOS -> rail
        sw = d.pin_xy(7)
        L = put(sh, lkey, sw[0] + 7.62, Y, ang=90)
        sh.connect(d, 7, L, 1)
        lo = L.pin_xy(2)
        vos = d.pin_xy(6)
        rx = lo[0] + 2.54
        sh.pin_wire(d, 6, (rx, vos[1]), (rx, Y))
        sh.pin_wire(L, 2, (lo[0] + 22.86, Y))
        co = put(sh, cout, lo[0] + 7.62, Y + 5.08)
        sh.pin_wire(co, 1, (co.x, Y))
        term(sh, co, 2, 0)
        sh.power(out_net, lo[0] + 22.86, Y, (0, -1))
        sh.flag(lo[0] + 15.24, Y, (0, -1))
        sh.wire((lo[0] + 15.24, Y), (lo[0] + 15.24, Y))
        # PG
        term(sh, d, 8)
        return d, (cin_x - 7.62, vin[1])

    # +5 V converter fed from VIN_PROT (after the diode)
    d5, inl = converter('U3', 101.6, 'C1', 'C3', 'L1', 'C2', 'VIN_PROT', '+5V', 'MCU_RESETn', 77.47)
    sh.flag(73.66, Y, (0, 1))
    sh.label('VIN_PROT', 72.39, Y, (1, 0))
    # +3,3 V and +1,8 V converters fed from +5 V
    d6, inl6 = converter('U4', 220.98, 'C4', 'C6', 'L2', 'C5', '+5V', '+3V3', 'DDS_PGOOD', 200.66)
    sh.power('+5V', *inl6, (0, -1))
    d7, inl7 = converter('U5', 345.44, 'C7', 'C9', 'L3', 'C8', '+5V', '+1V8', 'DDS_PGOOD', 325.12)
    sh.power('+5V', *inl7, (0, -1))
    # PGOOD pull-up
    r1 = put(sh, 'R1', 297.18, 99.06)
    term(sh, r1, 1)
    term(sh, r1, 2)
    sh.text('PG {U4}, {U5} — монтажное И: питание DDS в норме', 304.8, 93.98, 1.524)
    notes = [
        'Примечания',
        '1. Вход +15 В ±5 %; напряжение на входе {U3} должно оставаться ниже 17 В.',
        '2. {D1} — защита от обратной полярности, {F1} — от перегрузки.',
        '3. Ёмкости указаны эффективными при рабочем напряжении, диэлектрик X7R.',
        '4. Тепловые площадки {U3,U4,U5} соединить с GND; контуры SW — минимальной длины.',
        '5. PG {U3} формирует сброс МК; PG {U4}, {U5} — признак готовности питания DDS.',
        '6. Бюджет: 1,8 В <= 0,4 А (DDS); 3,3 В <= 0,2 А + генератор {G1} (уточнить).',
    ]
    for k, t in enumerate(notes):
        sh.text(t, 25.4, 140.0 + k * 5.08, 2.0 if k == 0 else 1.778)


# ================================================ sheet 2: controller ====

def sheet_controller(sh):
    X0, Y0 = 215.9, 30.48
    mcu = put(sh, 'U2', X0, Y0)
    # MCU left side: labels via auto-termination, PEN/RESET handled below
    # --- PEN pull-up and RESET network
    pen = mcu.pin_xy(1)
    r36 = put(sh, 'R36', pen[0] - 10.16, pen[1], ang=90)
    sh.connect(r36, 2, mcu, 1)
    term(sh, r36, 1)
    # --- right side: AREF cap, supply decoupling
    aref = mcu.pin_xy(62)
    c40 = put(sh, 'C40', aref[0] + 7.62, aref[1], ang=270)
    sh.connect(mcu, 62, c40, 2)
    term(sh, c40, 1, 0)
    # VCC pins with their capacitors
    vx = mcu.pin_xy(21)[0] + 5.08
    caps = {'21': 'C41', '52': 'C42', '64': 'C43'}
    ys = []
    for p, ck in caps.items():
        px, py = mcu.pin_xy(p)
        sh.pin_wire(mcu, p, (vx, py))
        c = put(sh, ck, vx + 7.62, py, ang=270, hide_value=True)
        sh.connect(c, 2, mcu, p, via=[(vx, py)]) if False else None
        sh.wire((vx, py), c.pin_xy(2))
        sh.wired_pins.add((ck, '2'))
        sh.wire(c.pin_xy(1), (vx + 15.24, py))
        sh.wired_pins.add((ck, '1'))
        ys.append(py)
    sh.wire((vx, min(ys)), (vx, max(ys)))
    sh.wire((vx, min(ys)), (vx, min(ys) - 2.54))
    sh.power('+5V', vx, min(ys) - 2.54, (0, -1))
    gys = []
    for p in ('22', '53', '63'):
        px, py = mcu.pin_xy(p)
        sh.pin_wire(mcu, p, (vx + 15.24, py))
        gys.append(py)
    sh.wire((vx + 15.24, min(ys)), (vx + 15.24, max(gys) + 5.08))
    sh.power('GND', vx + 15.24, max(gys) + 5.08, (0, 1))
    

    # --- DIP switch block (bits 0..11) -----------------------------------
    xa, xg = 35.56, 106.68
    y0 = 40.64
    rows = []
    for b in range(12):
        y = y0 + b * 10.16 + (5.08 if b >= 8 else 0)
        rows.append(y)
        rk = f'R{38 + b}'
        sk = f'SW{b + 1}'
        key, unit, _ = old_to_new(sk)
        r = put(sh, rk, xa + 7.62, y, ang=90)
        s = put(sh, key, xa + 50.8 - 15.24, y, unit=unit)
        nx = xa + 22.86
        sh.wire((xa, y), r.pin_xy(1)); sh.wired_pins.add((rk, '1'))
        sh.wire(r.pin_xy(2), (nx, y)); sh.wired_pins.add((rk, '2'))
        sh.wire((nx, y), s.pin_xy(old_to_new(sk, '1')[2])); sh.wired_pins.add((key, old_to_new(sk, '1')[2]))
        sh.wire(s.pin_xy(old_to_new(sk, '2')[2]), (xg, y)); sh.wired_pins.add((key, old_to_new(sk, '2')[2]))
        sh.wire((nx, y), (nx, y + 2.54))
        sh.label(f'DIP{b}', nx, y + 2.54, (1, 0))
    sh.wire((xa, rows[0] - 5.08), (xa, rows[-1]))
    sh.power('+5V', xa, rows[0] - 5.08, (0, -1))
    sh.wire((xg, rows[0]), (xg, rows[-1] + 5.08))
    sh.power('GND', xg, rows[-1] + 5.08, (0, 1))
    sh.text('{DIP8} — DIP-8 (биты 0–7), {DIP4} — DIP-4 (биты 8–11); подтяжки {' + ','.join(f'R{i}' for i in range(38, 50)) + '} 10 кОм', 30.48, rows[-1] + 10.16, 1.524)
    sh.text('Код частоты N = Σ бит·2^i; f = 47 000 000 + 1000·N Гц', 30.48, rows[-1] + 15.24, 1.524)
    sh.text('ВКЛ (замкнут) = 1 (программная инверсия); N = 0…4000, 4001…4095 отвергаются', 30.48,
            rows[-1] + 20.32, 1.524)

    # --- APPLY button, reset, ISP ---------------------------------------
    yb = rows[-1] + 40.64
    r50 = put(sh, 'R50', xa + 7.62, yb, ang=90)
    btn = put(sh, 'BTN', xa + 35.56, yb)
    nx = xa + 22.86
    sh.wire((xa, yb), r50.pin_xy(1)); sh.wired_pins.add(('R50', '1'))
    sh.power('+5V', xa, yb, (0, -1))
    sh.wire(r50.pin_xy(2), (nx, yb)); sh.wired_pins.add(('R50', '2'))
    sh.wire((nx, yb), btn.pin_xy(1)); sh.wired_pins.add(('BTN', '1'))
    sh.wire(btn.pin_xy(2), (xg, yb)); sh.wired_pins.add(('BTN', '2'))
    sh.power('GND', xg, yb, (0, 1))
    c44 = put(sh, 'C44', nx, yb + 7.62)
    sh.wire((nx, yb), c44.pin_xy(1)); sh.wired_pins.add(('C44', '1'))
    term(sh, c44, 2, 0)
    sh.wire((nx, yb), (nx, yb - 2.54))
    sh.label('APPLYn', nx, yb - 2.54, (1, 0))

    yr = yb + 30.48
    r37 = put(sh, 'R37', xa + 7.62, yr, ang=90)
    sh.wire((xa, yr), r37.pin_xy(1)); sh.wired_pins.add(('R37', '1'))
    sh.power('+5V', xa, yr, (0, -1))
    sh.wire(r37.pin_xy(2), (nx + 10.16, yr)); sh.wired_pins.add(('R37', '2'))
    c39 = put(sh, 'C39', nx, yr + 5.08)
    sh.wire((nx, yr), c39.pin_xy(1)); sh.wired_pins.add(('C39', '1'))
    term(sh, c39, 2, 0)
    sh.label('MCU_RESETn', nx + 10.16, yr, (1, 0), kind='global')

    isp = put(sh, 'J5', 314.96, 223.52)
    # SPI level shifter block --------------------------------------------
    spi_block(sh)
    autoterm(sh)


def spi_block(sh):
    xb = 360.68
    ys = [45.72, 76.2, 106.68, 137.16]
    chans = [  # unit, A-net pull (ref, to), out R, out pull
        (1, 'R64', 'R51', 'R57'),
        (2, 'R65', 'R52', 'R58'),
        (3, 'R63', 'R53', 'R59'),
        (4, 'R62', 'R54', 'R60'),
    ]
    en_pts = []
    for (u, rin, rser, rout), y in zip(chans, ys):
        b = put(sh, 'U6', xb, y - 5.08, unit=u)
        a_pin, oe_pin, y_pin = [p.num for p in b.sym.units[u]['pins']]
        ax, ay = b.pin_xy(a_pin)
        # input: label + pull resistor (vertical, hanging from the input wire)
        sh.pin_wire(b, a_pin, (ax - 25.4, ay))
        wl(sh, NET[('U6', a_pin)], ax - 25.4, ay, (-1, 0))
        pu_in = NET[(rin, '1')] == '+5V'
        rp = put(sh, rin, ax - (20.32 if pu_in else 12.7), ay - 7.62 if pu_in else ay + 7.62)
        if NET[(rin, '1')] == '+5V':     # pull-up: pin 2 on the wire
            sh.wire(rp.pin_xy(2), (rp.x, ay)); sh.wired_pins.add((rin, '2'))
            term(sh, rp, 1, 0)
        else:                            # pull-down: pin 1 on the wire
            sh.wire(rp.pin_xy(1), (rp.x, ay)); sh.wired_pins.add((rin, '1'))
            term(sh, rp, 2, 0)
        # output: series resistor + pull + global label
        yx, yy = b.pin_xy(y_pin)
        rs = put(sh, rser, yx + 12.7, yy, ang=90)
        sh.connect(b, y_pin, rs, 1)
        sh.label(NET[('U6', y_pin)], yx + 7.62, yy, (-1, 0))
        ox = rs.pin_xy(2)[0]
        sh.pin_wire(rs, 2, (ox + 25.4, yy))
        wl(sh, NET[(rser, '2')], ox + 25.4, yy, (1, 0))
        up = NET[(rout, '1')] == '+3V3'
        ro = put(sh, rout, ox + (20.32 if up else 12.7), yy - 7.62 if up else yy + 7.62)
        if up:
            sh.wire(ro.pin_xy(2), (ro.x, yy)); sh.wired_pins.add((rout, '2'))
            term(sh, ro, 1, 0)
        else:
            sh.wire(ro.pin_xy(1), (ro.x, yy)); sh.wired_pins.add((rout, '1'))
            term(sh, ro, 2, 0)
        en_pts.append((b, oe_pin))
    # common OE line with pull-up to +3V3 and label BUF_OEn
    ex = en_pts[0][0].pin_xy(en_pts[0][1])[0] - 5.08
    eys = []
    for b, p in en_pts:
        px, py = b.pin_xy(p)
        sh.pin_wire(b, p, (ex, py))
        eys.append(py)
    sh.wire((ex, min(eys)), (ex, max(eys) + 10.16))
    r61 = put(sh, 'R61', ex - 7.62, max(eys) + 10.16, ang=90)
    sh.wire(r61.pin_xy(2), (ex, max(eys) + 10.16)); sh.wired_pins.add(('R61', '2'))
    term(sh, r61, 1, 0)
    sh.wire((ex, max(eys) + 10.16), (ex, max(eys) + 17.78))
    wl(sh, 'BUF_OEn', ex, max(eys) + 17.78, (0, 1))
    # supply unit of the buffer
    pw = put(sh, 'U6', xb - 2.54, 175.26, unit=5)
    c45 = put(sh, 'C45', xb - 15.24, 187.96)
    vx, vy = pw.pin_xy(14)
    sh.pin_wire(pw, 14, (c45.x, vy))
    sh.wire((c45.x, vy), c45.pin_xy(1)); sh.wired_pins.add(('C45', '1'))
    sh.wire((c45.x, vy), (c45.x - 7.62, vy))
    sh.power('+3V3', c45.x - 7.62, vy, (0, -1))
    term(sh, c45, 2, 0)
    # MISO path: DDS_SDO -> R55 -> SDO_BUF (R56) -> U7 -> MCU_MISO
    ym = 210.82
    g = put(sh, 'U7', 457.2, ym - 5.08, unit=1)
    a, en, yo = [p.num for p in g.sym.units[1]['pins']]
    ax, ay = g.pin_xy(a)
    r55 = put(sh, 'R55', ax - 22.86, ay, ang=90)
    sh.pin_wire(r55, 1, (r55.pin_xy(1)[0] - 10.16, ay))
    wl(sh, 'DDS_SDO', r55.pin_xy(1)[0] - 10.16, ay, (-1, 0))
    sh.connect(r55, 2, g, a)
    sh.label(NET[('U7', a)], ax - 5.08, ay, (-1, 0))
    r56 = put(sh, 'R56', ax - 12.7, ay + 7.62)
    sh.wire(r56.pin_xy(1), (r56.x, ay)); sh.wired_pins.add(('R56', '1'))
    term(sh, r56, 2, 0)
    gp = put(sh, 'U7', 457.2, 238.76, unit=2)
    c46 = put(sh, 'C46', 444.5, 251.46)
    vx, vy = gp.pin_xy(5)
    sh.pin_wire(gp, 5, (c46.x, vy))
    sh.wire((c46.x, vy), c46.pin_xy(1)); sh.wired_pins.add(('C46', '1'))
    sh.wire((c46.x, vy), (c46.x - 7.62, vy))
    sh.power('+5V', c46.x - 7.62, vy, (0, -1))
    term(sh, c46, 2, 0)
    sh.text('Согласование уровней SPI 5 В → 3,3 В', 320.04, 22.86, 2.0)
    sh.text('{U6} выключен при старте ({R61}); PB5 {U2} — только «0» или вход', 320.04, 27.94, 1.524)
    sh.text('SDO 3,3 В → 5 В через {U7} (ТТЛ-входы)', 381.0, 193.04, 1.524)


# ====================================================== sheet 1: DDS ======

def sheet_dds(sh):
    X0, Y0 = 172.72, 30.48
    d1 = put(sh, 'U1', X0, Y0, unit=1)
    xp = d1.pin_xy(24)[0]          # left pin connection x
    xr = d1.pin_xy(43)[0]          # right pin connection x

    # ---- clock inputs ---------------------------------------------------
    clkp, clkm = d1.pin_xy(24), d1.pin_xy(23)
    r69 = put(sh, 'R69', xp - 10.16, clkp[1])
    r69.ref_pos = (r69.x - 2.54, r69.y - 0.635, 'right bottom'); r69.val_pos = (r69.x - 2.54, r69.y + 0.635, 'right top')
    sh.pin_wire(d1, 24, (xp - 5.08, clkp[1]), (xp - 5.08, r69.pin_xy(1)[1]), r69.pin_xy(1))
    sh.wired_pins.add(('R69', '1'))
    sh.pin_wire(d1, 23, r69.pin_xy(2))
    sh.wired_pins.add(('R69', '2'))
    sh.wire(r69.pin_xy(1), (xp - 20.32, r69.pin_xy(1)[1]))
    sh.label('CLKP', xp - 20.32, r69.pin_xy(1)[1], (-1, 0))
    sh.wire(r69.pin_xy(2), (xp - 20.32, clkm[1]))
    sh.label('CLKM', xp - 20.32, clkm[1], (-1, 0))

    cdp = d1.pin_xy(18)
    xw1 = put(sh, 'J2', 81.28, cdp[1], mirror='y')
    node = xw1.pin_xy(1)[0] + 12.7
    sh.pin_wire(xw1, 1, (node, cdp[1]))
    r70 = put(sh, 'R70', node, cdp[1] + 5.08)
    sh.wire((node, cdp[1]), r70.pin_xy(1)); sh.wired_pins.add(('R70', '1'))
    term(sh, r70, 2, 0)
    term(sh, xw1, 2, 0)
    c49 = put(sh, 'C49', node + 17.78, cdp[1], ang=90)
    sh.wire((node, cdp[1]), c49.pin_xy(1)); sh.wired_pins.add(('C49', '1'))
    sh.connect(c49, 2, d1, 18)
    sh.label(NET[('U1', '18')], c49.pin_xy(2)[0] + 2.54, cdp[1], (1, 0))
    sh.text('Вход 480 МГц, 0 дБм / 50 Ом', 60.96, cdp[1] - 10.16, 1.524)
    cdm = d1.pin_xy(19)
    c50 = put(sh, 'C50', xp - 12.7, cdm[1], ang=90)
    sh.connect(c50, 2, d1, 19)
    sh.label(NET[('U1', '19')], xp - 10.16, cdm[1], (1, 0))
    term(sh, c50, 1, 0)
    cs = d1.pin_xy(13)
    nx = xp - 25.4
    sh.pin_wire(d1, 13, (nx, cs[1]))
    sh.label(NET[('U1', '13')], nx + 7.62, cs[1], (1, 0))
    r71 = put(sh, 'R71', nx, cs[1] + 5.08)
    sh.wire((nx, cs[1]), r71.pin_xy(1)); sh.wired_pins.add(('R71', '1'))
    term(sh, r71, 2, 0)
    s1 = put(sh, 'DIP1', nx - 15.24, cs[1])
    sh.wire(s1.pin_xy(2), (nx, cs[1])); sh.wired_pins.add(('DIP1', '2'))
    term(sh, s1, 1, 0)
    sh.text('{DIP1} замкнут: такт с {J2} (CLKD); разомкнут: {G1} (CLKP/M)', 66.04, cs[1] + 15.24, 1.27)

    # ---- control / analog pins on the left --------------------------------
    # pull-ups of CS, RD, WR to +3V3 (rail)
    rail = xp - 27.94
    pu = [('8', 'R4'), ('76', 'R5'), ('77', 'R6')]
    ys = []
    for p, rk in pu:
        px, py = d1.pin_xy(p)
        r = put(sh, rk, xp - 12.7, py, ang=90, hide_value=True)
        sh.connect(r, 2, d1, p)
        sh.wire(r.pin_xy(1), (rail, py)); sh.wired_pins.add((rk, '1'))
        sh.label(NET[('U1', p)], xp - 7.62, py, (1, 0))
        ys.append(py)
    sh.wire((rail, min(ys) - 2.54), (rail, max(ys)))
    sh.power('+3V3', rail, min(ys) - 2.54, (0, -1))
    # pull-downs to a common GND rail: ADR, IREF1/2, CMP inputs, SEL1, SEL2
    grail = xp - 22.86
    pd = [('11', 'R7'), ('37', 'R2'), ('28', 'R3'), ('50', None), ('49', None)]
    pd += list(zip(['67', '68', '70', '71', '73', '74', '56', '57', '58', '59', '62', '63'],
                   [f'R{i}' for i in range(14, 20)] + [f'R{i}' for i in range(8, 14)]))
    gys = []
    for p, rk in pd:
        px, py = d1.pin_xy(p)
        if rk:
            r = put(sh, rk, xp - 12.7, py, ang=90, hide_value=True)
            sh.connect(r, 2, d1, p)
            sh.wire(r.pin_xy(1), (grail, py)); sh.wired_pins.add((rk, '1'))
            sh.label(NET[('U1', p)], xp - 7.62, py, (1, 0))
        else:
            sh.pin_wire(d1, p, (grail, py))
        gys.append(py)
    sh.wire((grail, min(gys)), (grail, max(gys) + 5.08))
    sh.power('GND', grail, max(gys) + 5.08, (0, 1))
    sh.text('{' + ','.join(f'R{i}' for i in range(8, 20)) + '} 100 кОм ±5 %', grail - 2.54, d1.pin_xy('63')[1] + 10.16, 1.27)
    sh.text('{R4,R5,R6} 10 кОм; {R7} 100 кОм; {R2,R3} 7,15 кОм ±0,1 %', grail - 2.54, d1.pin_xy('63')[1] + 13.97, 1.27)

    # ---- right side: DAC terminations, RF chain, data pull-downs ----------
    for p, rk in (('42', 'R73'), ('34', 'R74'), ('33', 'R75')):
        r = put(sh, rk, xr + 12.7, d1.pin_xy(p)[1], ang=270, hide_value=True)
        sh.connect(d1, p, r, 2)
        term(sh, r, 1, 0)
        sh.label(NET[('U1', p)], xr + 7.62, d1.pin_xy(p)[1], (-1, 0))
    rf_chain(sh, d1)
    dgr = xr + 22.86
    dys = []
    for i, p in enumerate(['78', '79', '80', '81', '83', '84', '87', '88', '89', '90', '92', '93',
                           '95', '96', '97', '98']):
        py = d1.pin_xy(p)[1]
        r = put(sh, f'R{20 + i}', xr + 12.7, py, ang=270, hide_value=True)
        sh.connect(d1, p, r, 2)
        sh.label(NET[('U1', p)], xr + 7.62, py, (-1, 0))
        sh.wire(r.pin_xy(1), (dgr, py)); sh.wired_pins.add((f'R{20 + i}', '1'))
        dys.append(py)
    sh.wire((dgr, min(dys)), (dgr, max(dys) + 5.08))
    sh.power('GND', dgr, max(dys) + 5.08, (0, 1))
    sh.text('{' + ','.join(f'R{i}' for i in range(20, 36)) + '} 100 кОм ±5 %', xr + 2.54, max(dys) + 10.16, 1.27)
    sh.text('{R72,R73,R74,R75} 49,9 Ом ±0,1 %', xr + 2.54, max(dys) + 13.97, 1.27)

    # ---- G1 oscillator cluster (unverified, DNP) --------------------------
    g1 = put(sh, 'G1', 66.04, 99.06)
    op, om = g1.pin_xy(4), g1.pin_xy(5)
    r67 = put(sh, 'R67', op[0] + 10.16, op[1], ang=90)
    r68 = put(sh, 'R68', om[0] + 10.16, om[1], ang=90)
    sh.connect(g1, 4, r67, 1)
    sh.connect(g1, 5, r68, 1)
    term(sh, r67, 2)
    term(sh, r68, 2)
    en = g1.pin_xy(1)
    r66 = put(sh, 'R66', en[0] - 5.08, en[1] - 5.08)
    sh.pin_wire(g1, 1, (en[0] - 5.08, en[1]))
    term(sh, r66, 1, 0)
    sh.wired_pins.add(('R66', '2'))
    sh.nc(g1, 2)
    vcc = g1.pin_xy(6)
    xs = [vcc[0] - 10.16, vcc[0] - 20.32]
    sh.pin_wire(g1, 6, (vcc[0] - 27.94, vcc[1]))
    for x, ck in zip(xs, ('C47', 'C48')):
        c = put(sh, ck, x, vcc[1] + 2.54)
        sh.wired_pins.add((ck, '1'))
        term(sh, c, 2, 0)
    sh.wire((vcc[0] - 27.94, vcc[1]), (vcc[0] - 27.94, vcc[1] - 2.54))
    sh.power('+3V3_OSC', vcc[0] - 27.94, vcc[1] - 2.54, (0, -1))
    gnd = g1.pin_xy(3)
    sh.pin_wire(g1, 3, (gnd[0] - 2.54, gnd[1]))
    sh.power('GND', gnd[0] - 2.54, gnd[1], (-1, 0))
    sh.text('{G1}, {R66,R67,R68} не устанавливаются: параметры и распиновка', 35.56, 142.24, 1.27)
    sh.text('ГК361-ТК-02 не подтверждены паспортом. Перед установкой', 35.56, 146.05, 1.27)
    sh.text('проверить корпус, выходы LVDS, EN, ток; затем разомкнуть {DIP1}.', 35.56, 149.86, 1.27)

    # ---- supply unit with decoupling --------------------------------------
    power_unit(sh)
    # ---- ferrite filters ----------------------------------------------------
    fb_block(sh)
    # ---- notes ---------------------------------------------------------------
    notes = [
        'Примечания',
        '1. Сплошной общий полигон земли; AGND, PGND, CGND, CMP_GND соединены на нём.',
        '2. Ток полной шкалы ЦАП IFS = 72 В / R = 10,07 мА ({R2}); напряжение на OUTP/OUTM не более ±0,6 В.',
        '3. ФНЧ — Баттерворт 7-го порядка, fср ≈ 70 МГц; номиналы округлены до E24.',
        '4. Уровень −10 дБм на 50 Ом устанавливается кодом амплитуды DDS при калибровке.',
        '5. Индуктивности {L4,L5,L6,L7} — ВЧ с высокой добротностью; проверить Q и SRF по моделям.',
        '6. Неиспользуемые выводы DATA, SEL — через резисторы на GND, не напрямую.',
        '7. Инициализация: ЦАП2 и компаратор выключены, SYNC/LINK выключены, профиль 0.',
        '8. Измерения SFDR и шумов не проводились; схема не является выпуском в производство.',
    ]
    for k, t in enumerate(notes):
        sh.text(t, 25.4, 300.0 + k * 5.08, 2.0 if k == 0 else 1.778)
    autoterm(sh)


def rf_chain(sh, d1):
    op = d1.pin_xy(43)
    y = op[1]
    x = op[0] + 20.32
    sh.pin_wire(d1, 43, (x, y))
    sh.label(NET[('U1', '43')], op[0] + 7.62, y, (-1, 0))
    r72 = put(sh, 'R72', x, y + 5.08)
    sh.wire((x, y), r72.pin_xy(1)); sh.wired_pins.add(('R72', '1'))
    term(sh, r72, 2, 0)
    c51 = put(sh, 'C51', x + 12.7, y, ang=90)
    sh.wire((x, y), c51.pin_xy(1)); sh.wired_pins.add(('C51', '1'))
    prev = c51.pin_xy(2)
    xx = prev[0]
    for lk, ck in (('L4', 'C52'), ('L5', 'C53'), ('L6', 'C54'), ('L7', None)):
        L = put(sh, lk, xx + 12.7, y, ang=90)
        sh.wire(prev, L.pin_xy(1)); sh.wired_pins.add((lk, '1'))
        n = L.pin_xy(2)[0] + 5.08
        sh.wire(L.pin_xy(2), (n, y)); sh.wired_pins.add((lk, '2'))
        if ck:
            c = put(sh, ck, n, y + 5.08)
            sh.wire((n, y), c.pin_xy(1)); sh.wired_pins.add((ck, '1'))
            term(sh, c, 2, 0)
        prev = (n, y)
        xx = n
    # pi attenuator
    r77 = put(sh, 'R77', xx, y + 7.62)
    sh.wire((xx, y), r77.pin_xy(1)); sh.wired_pins.add(('R77', '1'))
    term(sh, r77, 2, 0)
    r76 = put(sh, 'R76', xx + 10.16, y, ang=90)
    sh.wire((xx, y), r76.pin_xy(1)); sh.wired_pins.add(('R76', '1'))
    n2 = r76.pin_xy(2)[0] + 5.08
    sh.wire(r76.pin_xy(2), (n2, y)); sh.wired_pins.add(('R76', '2'))
    r78 = put(sh, 'R78', n2, y + 7.62)
    sh.wire((n2, y), r78.pin_xy(1)); sh.wired_pins.add(('R78', '1'))
    term(sh, r78, 2, 0)
    xw2 = put(sh, 'J3', n2 + 15.24, y)
    sh.wire((n2, y), xw2.pin_xy(1)); sh.wired_pins.add(('J3', '1'))
    term(sh, xw2, 2, 0)
    sh.text('ФНЧ 7-го порядка, fср ≈ 70 МГц', op[0] + 40.64, y - 10.16, 1.524)
    sh.text('Аттенюатор ≈1 дБ', xx - 2.54, y - 10.16, 1.524)
    sh.text('Выход 47–51 МГц, −10 дБм / 50 Ом', n2 + 7.62, y + 15.24, 1.524)


def power_unit(sh):
    X0, Y0 = 381.0, 76.2
    d = put(sh, 'U1', X0, Y0, unit=2)
    caps = {}
    for k in range(13, 35):
        v = None
    # decoupling map from rev C values "100n / U1.<pin>"
    cmap = {'1': 'C13', '9': 'C14', '14': 'C15', '15': 'C16', '16': 'C17', '21': 'C18', '26': 'C19',
            '27': 'C20', '30': 'C21', '31': 'C22', '36': 'C23', '39': 'C24', '40': 'C25', '45': 'C26',
            '47': 'C27', '51': 'C28', '60': 'C29', '61': 'C30', '69': 'C31', '75': 'C32', '82': 'C33',
            '94': 'C34'}
    bulk = {'+1V8': 'C35', '+3V3': 'C36', '+3V3_A': 'C37', '+1V8_Q': 'C38'}
    xp = d.pin_xy('1')[0]
    xr = xp - 5.08
    xg = xr - 15.24
    groups = {}
    for p in d.pins():
        px, py = d.pin_xy(p)
        if px == xp:
            groups.setdefault(NET[('U1', p)], []).append((py, p))
    allys = []
    for net, items in groups.items():
        items.sort()
        for py, p in items:
            sh.pin_wire(d, p, (xr, py))
            c = put(sh, cmap[p], xr - 7.62, py, ang=90, hide_value=True)
            sh.wire((xr, py), c.pin_xy(2)); sh.wired_pins.add((cmap[p], '2'))
            sh.wire(c.pin_xy(1), (xg, py)); sh.wired_pins.add((cmap[p], '1'))
            allys.append(py)
        top, bot = items[0][0], items[-1][0]
        sh.wire((xr, top), (xr, bot))
        yb = bot + 5.08
        cb = put(sh, bulk[net], xr - 7.62, yb, ang=90)
        sh.wire((xr, bot), (xr, yb))
        sh.wire((xr, yb), cb.pin_xy(2)); sh.wired_pins.add((bulk[net], '2'))
        sh.wire(cb.pin_xy(1), (xg, yb)); sh.wired_pins.add((bulk[net], '1'))
        allys.append(yb)
        sh.wire((xr, top), (xr, top - 5.08))
        sh.power(net, xr, top - 5.08, (0, -1))
    sh.wire((xg, min(allys)), (xg, max(allys) + 5.08))
    sh.power('GND', xg, max(allys) + 5.08, (0, 1))
    # ground pins on the right
    xgr = d.pin_xy('17')[0] + 5.08
    rys = []
    for p in d.pins():
        px, py = d.pin_xy(p)
        if px != xp:
            sh.pin_wire(d, p, (xgr, py))
            rys.append(py)
    sh.wire((xgr, min(rys)), (xgr, max(rys) + 5.08))
    sh.power('GND', xgr, max(rys) + 5.08, (0, 1))
    sh.text('{' + ','.join(f'C{i}' for i in range(13, 35)) + '} 0,1 мкФ — у выводов {U1},', xg - 12.7, max(allys) + 12.7, 1.27)
    sh.text('указанных в поле «Размещение»; {C35,C36,C37,C38} 10 мкФ', xg - 12.7, max(allys) + 16.51, 1.27)
    for p, ck in cmap.items():
        INSTS[(ck, 1)].extra['Размещение'] = f'у D1 выв. {p}'


def fb_block(sh):
    x0 = 266.7
    for k, (fb, c, src, dst) in enumerate((('FB1', 'C10', '+3V3', '+3V3_A'),
                                            ('FB2', 'C11', '+3V3', '+3V3_OSC'),
                                            ('FB3', 'C12', '+1V8', '+1V8_Q'))):
        y = 132.08 + k * 30.48
        L = put(sh, fb, x0 + 10.16, y, ang=90)
        sh.pin_wire(L, 1, (x0, y))
        sh.power(src, x0, y, (0, -1))
        n = L.pin_xy(2)[0] + 7.62
        sh.pin_wire(L, 2, (n + 10.16, y))
        cc = put(sh, c, n, y + 5.08)
        sh.wire((n, y), cc.pin_xy(1)); sh.wired_pins.add((c, '1'))
        term(sh, cc, 2, 0)
        sh.power(dst, n + 10.16, y, (0, -1))
        sh.flag(n + 5.08, y, (0, -1))
    sh.text('Фильтры питания DDS', x0 - 2.54, 116.84, 1.524)


# ======================================================= orientation fix ===



class UF:
    def __init__(self):
        self.p = {}

    def find(self, a):
        self.p.setdefault(a, a)
        while self.p[a] != a:
            self.p[a] = self.p[self.p[a]]
            a = self.p[a]
        return a

    def union(self, a, b):
        ra, rb = self.find(a), self.find(b)
        if ra != rb:
            self.p[ra] = rb


def drawn_components(sheets):
    uf = UF()
    pin_node = {}
    for sh in sheets:
        sh.split_wires()
        pg = sh.page
        for x1, y1, x2, y2 in sh.wires:
            uf.union((pg, x1, y1), (pg, x2, y2))
        for kind, net, x, y, d in sh.labels:
            uf.union((pg, x, y), ('G', net) if kind == 'global' else ('L', pg, net))
        for net, x, y, d in sh.powers:
            uf.union((pg, x, y), ('G', net))
        for i in sh.insts:
            for p in i.pins():
                xy = i.pin_xy(p)
                pin_node[(i.key, p, i.unit)] = (pg, xy[0], xy[1])
                uf.find((pg, xy[0], xy[1]))
    return uf, pin_node


def fix_orientation(sheets):
    """Rotate symmetric two-pin parts by 180 deg where the drawing connected
    them the other way round, so pin numbers match the reference netlist.
    Evidence comes from IC pins, labels and power ports, then from two-pin
    parts already settled (for chains such as the RF filter)."""
    settled = set()
    for _ in range(12):
        uf, pin_node = drawn_components(sheets)
        prev = set(settled)
        two = {i.key for sh in sheets for i in sh.insts if i.sym.name in TWO_PIN}
        comp = {}
        for (k, p, u), node in pin_node.items():
            comp.setdefault(uf.find(node), []).append((k, p))
        named = {}
        for node in list(uf.p):
            if node[0] in ('G', 'L'):
                named.setdefault(uf.find(node), set()).add(node[-1])

        def expected(root, exclude):
            nets = {NET[m] for m in comp.get(root, [])
                    if m[0] != exclude and m in NET and (m[0] not in two or m[0] in prev)}
            return nets | named.get(root, set())
        changed = 0
        for sh in sheets:
            for i in sh.insts:
                if i.sym.name not in TWO_PIN or i.key in settled:
                    continue
                r1 = uf.find(pin_node[(i.key, '1', 1)])
                r2 = uf.find(pin_node[(i.key, '2', 1)])
                e1, e2 = expected(r1, i.key), expected(r2, i.key)
                if not e1 and not e2:
                    continue
                n1, n2 = NET[(i.key, '1')], NET[(i.key, '2')]
                ok = (not e1 or n1 in e1) and (not e2 or n2 in e2)
                swapped = (not e1 or n2 in e1) and (not e2 or n1 in e2)
                if not ok and swapped:
                    i.ang = (i.ang + 180) % 360
                settled.add(i.key)
                changed += 1
        if not changed:
            return


# ================================================================ output ===

def annotate(sheets):
    """GOST 2.702: number within each letter code in sheet order, top to bottom
    within columns from left to right (column width 25.4 mm)."""
    first = {}
    for sh in sheets:
        for i in sh.insts:
            k = i.key
            pos = (int(sh.page), round(i.x // 25.4), i.y, i.x)
            if k not in first or pos < first[k]:
                first[k] = pos
    by = {}
    for k, pos in first.items():
        by.setdefault(gost_prefix(k), []).append((pos, k))
    refs = {}
    for pfx, items in by.items():
        for n, (_, k) in enumerate(sorted(items), 1):
            refs[k] = f'{pfx}{n}'
    for sh in sheets:
        for i in sh.insts:
            i.ref = refs[i.key]
    return refs


def compact(refs):
    """['R13','R14','R15','R20'] -> 'R13–R15, R20'"""
    def k(r):
        m = re.match(r'([A-Z]+)(\d+)', r)
        return m.group(1), int(m.group(2))
    items = sorted(set(refs), key=k)
    out, i = [], 0
    while i < len(items):
        p, n = k(items[i]); j = i
        while j + 1 < len(items) and k(items[j + 1]) == (p, n + (j + 1 - i)):
            j += 1
        if j - i >= 2:
            out.append(f'{items[i]}–{items[j]}')
        else:
            out.extend(items[i:j + 1])
        i = j + 1
    return ', '.join(out)


def make_resolver(refs):
    def res(s):
        def rep(m):
            keys = m.group(1).split(',')
            return compact([refs[k.strip()] for k in keys])
        return re.sub(r'\{([A-Z0-9_,\s]+)\}', rep, s)
    return res


def title_block(sh):
    t = TB
    return (f'(title_block (title {q(t["title"])}) (date {q(t["date"])}) (rev {q(t["rev"])}) '
            f'(company {q(t["company"])}) (comment 1 {q(t["c1"])}) (comment 2 {q(t["c2"])}) '
            f'(comment 3 {q(t["c3"])}) (comment 4 {q(t["c4"])}) (comment 5 {q(t["c5"])}) '
            f'(comment 6 {q(t["c6"])}) (comment 7 {q(t["c7"])}) (comment 8 {q(t["c8"])}) '
            f'(comment 9 {q(sh.title)}))')


def sheet_symbol(root, child, x, y, w, h):
    return (f'(sheet (at {f(x)} {f(y)}) (size {f(w)} {f(h)}) (exclude_from_sim no) (in_bom yes) '
            f'(on_board yes) (dnp no) (stroke (width 0.1524) (type solid)) (fill (color 0 0 0 0)) '
            f'(uuid "{child.sheet_path_uuid}") '
            f'(property "Sheetname" {q(child.title)} (at {f(x)} {f(y - 0.762)} 0) {effects(1.524, "left bottom")}) '
            f'(property "Sheetfile" {q(child.file)} (at {f(x)} {f(y + h + 0.762)} 0) {effects(1.27, "left top")}) '
            f'(instances (project {q(PROJECT)} (path "/{root.uuid}" (page {q(child.page)})))))')


def main(out_dir, base_netlist):
    out = Path(out_dir)
    load_netlist(base_netlist)
    s1 = Sheet('dds', 'DDS_47_51.kicad_sch', 'A2', 'DDS, тактирование, выход', '1', 'dds')
    s2 = Sheet('ctl', '02_controller.kicad_sch', 'A2', 'Контроллер', '2', 'ctl')
    s3 = Sheet('pwr', '03_power.kicad_sch', 'A3', 'Питание', '3', 'pwr')
    s1.uuid = suid('root:dds')
    for sh in (s1, s2, s3):
        sh.deferred, sh.term_pts = [], set()
    for sh in (s2, s3):
        sh.sheet_path_uuid = suid('sheetsym:' + sh.name)
    # pages must be known before layout (global/local label decision)
    for k in list(PARTS) + ['DIP8', 'DIP4', 'DIP1', 'BTN']:
        pass
    pre_pages(s1, s2, s3)
    sheet_dds(s1)
    sheet_controller(s2)
    sheet_power(s3)
    s1.sheet_syms.append(sheet_symbol(s1, s2, 25.4, 375.92, 50.8, 10.16))
    s1.sheet_syms.append(sheet_symbol(s1, s3, 86.36, 375.92, 50.8, 10.16))
    fix_orientation([s1, s2, s3])
    for sh in (s1, s2, s3):
        finish(sh)
    refs = annotate([s1, s2, s3])
    res = make_resolver(refs)
    for sh in (s1, s2, s3):
        sh.resolve = res
    # placement notes now that D1 is known
    (out / 'lib').mkdir(exist_ok=True)
    write_library(LIB, out / 'lib' / 'DDS.kicad_sym')
    write_wks(out / 'lib' / 'GOST_E3.kicad_wks')
    for sh in (s1, s2, s3):
        (out / sh.file).write_text(sh.sexpr(PROJECT, s1.uuid, LIB, title_block(sh)), encoding='utf-8')
    # symbol library table
    (out / 'sym-lib-table').write_text(
        '(sym_lib_table\n  (version 7)\n'
        '  (lib (name "DDS")(type "KiCad")(uri "${KIPRJMOD}/lib/DDS.kicad_sym")(options "")'
        '(descr "DDS project symbols, ESKD style"))\n)\n', encoding='utf-8')
    # project settings
    pro_path = out / f'{PROJECT}.kicad_pro'
    try:
        pro = json.loads(pro_path.read_text(encoding='utf-8'))
    except Exception:
        pro = {}
    pro.setdefault('meta', {'filename': f'{PROJECT}.kicad_pro', 'version': 3})
    sch = pro.setdefault('schematic', {})
    sch['subpart_id_separator'] = 46      # '.'  -> D3.1
    sch['subpart_first_id'] = 49          # '1'
    sch['page_layout_descr_file'] = '${KIPRJMOD}/lib/GOST_E3.kicad_wks'
    pro['sheets'] = [[s1.uuid, 'Root'], [s2.sheet_path_uuid, s2.title], [s3.sheet_path_uuid, s3.title]]
    pro.setdefault('text_variables', {})
    pro_path.write_text(json.dumps(pro, indent=2, ensure_ascii=False), encoding='utf-8')
    # rev C -> rev D designator map for verification
    refmap = {}
    import xml.etree.ElementTree as ET2
    for c in ET2.parse(base_netlist).getroot().iter('comp'):
        old = c.get('ref')
        key, unit, _ = old_to_new(old)
        if key == old:
            refmap[old] = refs[key]
        else:
            pins = {p: old_to_new(old, p)[2] for p in ('1', '2')}
            refmap[old] = {'ref': refs[key], 'pins': pins}
    (out / 'verification').mkdir(exist_ok=True)
    (out / 'verification' / 'keymap.json').write_text(
        json.dumps(refs, indent=1, ensure_ascii=False, sort_keys=True), encoding='utf-8')
    (out / 'verification' / 'refmap_revC_to_revD.json').write_text(
        json.dumps(refmap, indent=1, ensure_ascii=False, sort_keys=True), encoding='utf-8')
    print('written; parts:', len(refs))


def pre_pages(s1, s2, s3):
    """Assign pages to keys before drawing (needed to decide local/global labels)."""
    ctl = {'U2', 'U6', 'U7', 'J5', 'DIP8', 'DIP4', 'BTN'} | {f'R{i}' for i in range(36, 66)} \
        | {f'C{i}' for i in range(39, 47)}
    pwr = {'J1', 'F1', 'D1', 'U3', 'U4', 'U5', 'L1', 'L2', 'L3', 'R1'} | {f'C{i}' for i in range(1, 10)}
    for (k, p) in NET:
        SHEET_OF[k] = '2' if k in ctl else '3' if k in pwr else '1'


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
