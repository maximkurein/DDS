#!/usr/bin/env python3
"""Create / update the DDS board (KiCad 10, run with KiCad's python):

    python3 tools/dds_pcb.py place  <project_dir>     -> DDS_47_51.kicad_pcb (placed, unrouted)
    python3 tools/dds_pcb.py dsn    <project_dir> <out.dsn>
    python3 tools/dds_pcb.py ses    <project_dir> <in.ses>   (import routing, pour zones)

Inputs: the schematic netlist exported by kicad-cli (output/netlist/netlist.xml)
and verification/keymap.json (stable rev C keys -> rev D designators).
Board: 120 x 50 mm, 4 layers: F.Cu signal / In1.Cu GND plane / In2.Cu signal /
B.Cu signal.  All components on the top side.
"""
import json
import os
import math
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import pcbnew

MM = pcbnew.FromMM
O = (100.0, 100.0)                     # board origin (top-left corner) in KiCad coordinates
BW, BH = 120.0, 50.0
SYS_FP = Path('/usr/share/kicad/footprints')


def V(x, y):
    return pcbnew.VECTOR2I(MM(O[0] + x), MM(O[1] + y))


def local(p):
    return pcbnew.ToMM(p.x) - O[0], pcbnew.ToMM(p.y) - O[1]


class Proj:
    def __init__(self, d):
        self.d = Path(d)
        self.keymap = json.loads((self.d / 'verification' / 'keymap.json').read_text(encoding='utf-8'))
        self.inv = {v: k for k, v in self.keymap.items()}
        root = ET.parse(self.d / 'output' / 'netlist' / 'netlist.xml').getroot()
        self.comps = {}
        for c in root.iter('comp'):
            ref = c.get('ref')
            props = {p.get('name'): p.get('value') for p in c.iter('property')}
            fields = {f.get('name'): (f.text or '') for f in c.iter('field')}
            sp = c.find('sheetpath')
            ts = c.find('tstamps').text.split()[0] if c.find('tstamps') is not None else c.findtext('tstamp')
            self.comps[ref] = dict(value=c.findtext('value'), fp=c.findtext('footprint'),
                                   path=sp.get('tstamps') + ts, sheet=sp.get('names'),
                                   dnp='dnp' in props, props=props, fields=fields)
        self.nets = {}
        self.pin_net = {}
        for n in root.iter('net'):
            name = n.get('name')
            nodes = [(x.get('ref'), x.get('pin')) for x in n.iter('node')]
            self.nets[name] = nodes
            for r, p in nodes:
                self.pin_net[(r, p)] = name

    def ref(self, key):
        return self.keymap[key]


# ---------------------------------------------------------------- creation --

def load_fp(proj, fpid):
    lib, name = fpid.split(':', 1)
    path = proj.d / 'lib' / 'DDS.pretty' if lib == 'DDS' else SYS_FP / f'{lib}.pretty'
    fp = pcbnew.FootprintLoad(str(path), name)
    assert fp is not None, fpid
    fp.SetFPID(pcbnew.LIB_ID(lib, name))
    return fp


def new_board(proj):
    b = pcbnew.BOARD()
    b.SetCopperLayerCount(4)
    b.SetLayerName(pcbnew.In1_Cu, 'In1.Cu')
    b.SetLayerType(pcbnew.In1_Cu, pcbnew.LT_POWER)
    b.SetLayerType(pcbnew.In2_Cu, pcbnew.LT_SIGNAL)
    ds = b.GetDesignSettings()
    ds.SetBoardThickness(MM(1.6))
    # outline with 1 mm corner radius
    r = 1.0
    segs = [((r, 0), (BW - r, 0)), ((BW, r), (BW, BH - r)), ((BW - r, BH), (r, BH)), ((0, BH - r), (0, r))]
    for a, c in segs:
        s = pcbnew.PCB_SHAPE(b)
        s.SetShape(pcbnew.SHAPE_T_SEGMENT)
        s.SetStart(V(*a)); s.SetEnd(V(*c))
        s.SetLayer(pcbnew.Edge_Cuts); s.SetWidth(MM(0.1))
        b.Add(s)
    import math as _m
    for cx, cy, a0 in ((r, r, 180), (BW - r, r, 270), (BW - r, BH - r, 0), (r, BH - r, 90)):
        a = pcbnew.PCB_SHAPE(b)
        a.SetShape(pcbnew.SHAPE_T_ARC)
        pts = []
        for ang in (a0, a0 + 45, a0 + 90):
            t = _m.radians(ang)
            pts.append(V(cx + r * _m.cos(t), cy + r * _m.sin(t)))
        a.SetArcGeometry(pts[0], pts[1], pts[2])
        a.SetLayer(pcbnew.Edge_Cuts); a.SetWidth(MM(0.1))
        b.Add(a)
    # nets
    netobj = {}
    for name in proj.nets:
        n = pcbnew.NETINFO_ITEM(b, name)
        b.Add(n)
        netobj[name] = n
    fps = {}
    for ref, c in proj.comps.items():
        fp = load_fp(proj, c['fp'])
        fp.SetReference(ref)
        fp.SetValue(c['value'])
        fp.SetPath(pcbnew.KIID_PATH(c['path']))
        fp.SetSheetname(c['sheet'])
        fp.SetDNP(c['dnp'])
        for k, v in c['fields'].items():
            if k in ('Footprint', 'Datasheet', 'Description'):
                continue
            fp.SetField(k, v)
        for f_ in fp.GetFields():
            if f_.GetName() in c['fields'] and f_.GetName() not in ('Reference', 'Value'):
                f_.SetVisible(False)
                f_.SetLayer(pcbnew.F_Fab)
        if c['dnp']:
            fp.SetExcludedFromBOM(False)
        for pad in fp.Pads():
            num = pad.GetNumber()
            net = proj.pin_net.get((ref, num))
            if net:
                pad.SetNet(netobj[net])
        cy = fp.GetCourtyard(pcbnew.F_CrtYd).BBox()
        small = pcbnew.ToMM(cy.GetWidth()) * pcbnew.ToMM(cy.GetHeight()) < 12
        if small:
            fp.Reference().SetLayer(pcbnew.F_Fab)
            fp.Reference().SetTextSize(pcbnew.VECTOR2I(MM(0.5), MM(0.5)))
            fp.Reference().SetTextThickness(MM(0.08))
        else:
            fp.Reference().SetTextSize(pcbnew.VECTOR2I(MM(1.0), MM(1.0)))
            fp.Reference().SetTextThickness(MM(0.15))
        fp.Value().SetVisible(False)
        b.Add(fp)
        fps[ref] = fp
    return b, fps


# --------------------------------------------------------------- placement --

def put(fps, ref, x, y, rot=0):
    fp = fps[ref]
    fp.SetOrientationDegrees(rot)
    fp.SetPosition(V(x, y))
    fp.Reference().SetPosition(V(x, y))
    fp.Reference().SetTextAngleDegrees(rot if rot in (0, 90) else (rot - 180))
    return fp


def pad_xy(fp, num):
    for p in fp.Pads():
        if p.GetNumber() == str(num):
            return local(p.GetPosition())
    raise KeyError((fp.GetReference(), num))


def rot_for(vx, vy):
    """Orientation that turns the footprint's +x axis (pad1 -> pad2) into (vx, vy)."""
    return round(math.degrees(math.atan2(-vy, vx))) % 360


def ring(proj, fps, ic_ref, items, dist, pitch=1.7, limit=8.2, row_step=3.4, keepout=None):
    """Place two-pin parts radially around an IC.
    items: list of (ref, ic_pin, inner_pin) — inner_pin is the part pad that faces the IC.
    A side that does not fit into one row continues in further rows outwards."""
    ic = fps[ic_ref]
    cx, cy = local(ic.GetPosition())
    sides = {}
    for ref, pin, inner in items:
        px, py = pad_xy(ic, pin)
        dx, dy = px - cx, py - cy
        if abs(dx) >= abs(dy):
            side = 'R' if dx > 0 else 'L'
            t = py - cy
        else:
            side = 'B' if dy > 0 else 'T'
            t = px - cx
        sides.setdefault(side, []).append([t, ref, inner])
    cap = int(2 * limit / pitch) + 1
    for side, lst in sides.items():
        lst.sort()
        nrows = (len(lst) + cap - 1) // cap
        rows = [lst[k::nrows] for k in range(nrows)]
        for ri, rl in enumerate(rows):
            pos = []
            for t, ref, inner in rl:
                p = t if not pos else max(t, pos[-1] + pitch)
                pos.append(p)
            shift = sum(a[0] for a in rl) / len(rl) - sum(pos) / len(pos)
            pos = [p + shift for p in pos]
            if pos[-1] > limit:
                pos = [p - (pos[-1] - limit) for p in pos]
            if pos[0] < -limit:
                pos = [p + (-limit - pos[0]) for p in pos]
            ko = (keepout or {}).get(side)
            if ko:                      # free channel [k0, k1] along this side
                k0, k1 = ko
                left = [i for i, p in enumerate(pos) if p < (k0 + k1) / 2]
                right = [i for i, p in enumerate(pos) if p >= (k0 + k1) / 2]
                nxt = k0 - pitch / 2
                for i in reversed(left):
                    pos[i] = min(pos[i], nxt); nxt = pos[i] - pitch
                nxt = k1 + pitch / 2
                for i in right:
                    pos[i] = max(pos[i], nxt); nxt = pos[i] + pitch
            out = {'R': (1, 0), 'L': (-1, 0), 'B': (0, 1), 'T': (0, -1)}[side]
            d = dist + ri * row_step
            for (t, ref, inner), p in zip(rl, pos):
                if side in 'LR':
                    x, y = cx + out[0] * d, cy + p
                else:
                    x, y = cx + p, cy + out[1] * d
                vin = (-out[0], -out[1])
                v = vin if inner == '2' else (out[0], out[1])
                put(fps, ref, round(x, 3), round(y, 3), rot_for(*v))
    return {k: (len(v) + cap - 1) // cap for k, v in sides.items()}


def inner_pin(proj, ref, ic_net_of):
    """Which pad of a two-pin part is on the IC side net."""
    return '1' if proj.pin_net.get((ref, '1')) == ic_net_of else '2'


def place_all(proj, fps):
    R = proj.ref
    # ---------------- power block (x 0..30)
    put(fps, R('J1'), 3.0, 45.0, 0)               # +15 V header, pin 1 top
    put(fps, R('F1'), 8.0, 45.0, 0)
    put(fps, R('D1'), 14.0, 45.0, 180)            # anode left, cathode right
    rows = [('U3', 'C1', 'C3', 'L1', 'C2', 36.0, 18.0),
            ('U4', 'C4', 'C6', 'L2', 'C5', 22.0, 18.0),
            ('U5', 'C7', 'C9', 'L3', 'C8', 8.0, 18.0)]
    for u, cin, c100, L, cout, y, x in rows:
        put(fps, R(u), x, y, 0)
        put(fps, R(cin), x - 4.6, y, 90)
        put(fps, R(c100), x - 2.2, y + 3.4, 0)
        put(fps, R(L), x + 5.0, y, 0)
        put(fps, R(cout), x + 9.6, y, 90)
    put(fps, R('R1'), 27.0, 15.0, 90)
    # ---------------- controller (x 30..70)
    put(fps, R('U2'), 47.0, 19.0, 0)
    put(fps, R('J5'), 31.0, 3.5, 0)               # ISP header near the top edge
    put(fps, R('DIP8'), 40.0, 47.0, 90)           # DIP-8 along the bottom edge
    put(fps, R('DIP4'), 64.5, 47.0, 90)
    put(fps, R('BTN'), 28.5, 41.5, 0)
    put(fps, R('R50'), 33.0, 36.5, 0)
    put(fps, R('C44'), 33.0, 38.5, 0)
    # pull-ups of the DIP bits in a row above the switches
    for b in range(12):
        x = 40.0 + b * 2.2 + (3.0 if b >= 8 else 0)
        put(fps, R(f'R{38 + b}'), x, 33.0, 90)
    U2 = R('U2')
    ring(proj, fps, U2, [(R('C41'), '21', inner_pin(proj, R('C41'), '+5V')),
                         (R('C42'), '52', inner_pin(proj, R('C42'), '+5V')),
                         (R('C43'), '64', inner_pin(proj, R('C43'), '+5V')),
                         (R('C40'), '62', inner_pin(proj, R('C40'), proj.pin_net[(U2, '62')])),
                         (R('R36'), '1', inner_pin(proj, R('R36'), proj.pin_net[(U2, '1')])),
                         (R('R37'), '20', inner_pin(proj, R('R37'), proj.pin_net[(U2, '20')]))],
         dist=11.0)
    put(fps, R('C39'), 35.5, 30.0, 0)
    # level shifters between MCU and DDS
    put(fps, R('U6'), 65.0, 12.0, 0)
    put(fps, R('C45'), 65.0, 7.5, 0)
    put(fps, R('R61'), 60.5, 7.5, 0)
    for k, ref in enumerate(['R64', 'R65', 'R63', 'R62']):
        put(fps, R(ref), 58.4, 10.0 + k * 1.7, 0)
    for k, ref in enumerate(['R51', 'R52', 'R53', 'R54']):
        put(fps, R(ref), 70.8, 10.0 + k * 1.7, 0)
    for k, ref in enumerate(['R57', 'R58', 'R59', 'R60']):
        put(fps, R(ref), 74.0, 10.0 + k * 1.7, 0)
    put(fps, R('U7'), 64.0, 24.0, 0)
    put(fps, R('C46'), 64.0, 20.5, 0)
    put(fps, R('R55'), 68.5, 24.0, 0)
    put(fps, R('R56'), 68.5, 26.0, 0)
    # ---------------- DDS (x 70..120)
    DX, DY = 96.0, 23.0
    put(fps, R('U1'), DX, DY, 0)
    U1 = R('U1')
    pn = lambda p: proj.pin_net[(U1, p)]
    # decoupling: ring 1
    cmap = {'1': 'C13', '9': 'C14', '14': 'C15', '15': 'C16', '16': 'C17', '21': 'C18', '26': 'C19',
            '27': 'C20', '30': 'C21', '31': 'C22', '36': 'C23', '39': 'C24', '40': 'C25', '45': 'C26',
            '47': 'C27', '51': 'C28', '60': 'C29', '61': 'C30', '69': 'C31', '75': 'C32', '82': 'C33',
            '94': 'C34'}
    ko = {'B': (0.6, 3.6)}           # RF channel under pins 41..44 (OUTM1/OUTP1)
    clk_caps = [(R('C49'), '18', inner_pin(proj, R('C49'), pn('18'))),
                (R('C50'), '19', inner_pin(proj, R('C50'), pn('19')))]
    ring(proj, fps, U1, [(R(c), p, inner_pin(proj, R(c), pn(p))) for p, c in cmap.items()] + clk_caps,
         dist=11.5, limit=9.0, keepout=ko)
    # pull resistors: ring 2
    pulls = [('R4', '8'), ('R5', '76'), ('R6', '77'), ('R7', '11'), ('R2', '37'), ('R3', '28'), ('R71', '13'),
             ('R74', '34'), ('R75', '33')]
    sel = ['67', '68', '70', '71', '73', '74', '56', '57', '58', '59', '62', '63']
    pulls += list(zip([f'R{i}' for i in range(14, 20)] + [f'R{i}' for i in range(8, 14)], sel))
    data = ['78', '79', '80', '81', '83', '84', '87', '88', '89', '90', '92', '93', '95', '96', '97', '98']
    pulls += list(zip([f'R{20 + i}' for i in range(16)], data))
    ring(proj, fps, U1, [(R(r), p, inner_pin(proj, R(r), pn(p))) for r, p in pulls],
         dist=15.0, limit=9.8, keepout=ko)
    # DAC1 loads right in the channel: OUTM1 termination and OUTP1 shunt
    # (signal pad on top, next to the D1 pins; GND pad away from them)
    put(fps, R('R73'), DX + 1.4, DY + 11.0, 270 if proj.pin_net[(R('R73'), '1')] != 'GND' else 90)
    put(fps, R('R72'), DX + 3.4, DY + 14.5, 270 if proj.pin_net[(R('R72'), '1')] != 'GND' else 90)
    # RF output chain along y = 44 to XW2 at the right edge
    y = 44.0
    for ref, x in (('C51', 98.3), ('L4', 101.3), ('L5', 104.3), ('L6', 107.3), ('L7', 110.3), ('R76', 113.3)):
        put(fps, R(ref), x, y, 0)
    for ref, x, yy in (('C52', 102.8, y + 2.6), ('C53', 105.8, y + 2.6), ('C54', 108.8, y + 2.6),
                       ('R77', 111.8, y + 2.6), ('R78', 114.4, y - 2.6)):
        down = yy > y
        gnd_pin1 = proj.pin_net[(R(ref), '1')] == 'GND'
        put(fps, R(ref), x, yy, (90 if gnd_pin1 else 270) if down else (270 if gnd_pin1 else 90))
    put(fps, R('J3'), BW - 2.1, y, 0)            # SMA output, edge launch to +x
    # 480 MHz clock inputs.  D1 left side, top to bottom: CLKDP/CLKDM (pins 18/19,
    # external clock via XW1) then CLKM/CLKP (pins 23/24, LVDS from G1).  To keep
    # both paths on F.Cu over the solid In1 GND plane without crossing, the
    # external line runs north at x = 81 and the oscillator group sits below the
    # D1 corner, its pair entering pins 23/24 from below (see clock_routes()).
    put(fps, R('J2'), 80.0, BH - 2.1, 270)
    put(fps, R('R70'), 82.2, 27.4, 270)          # 49.9 R termination at the receiver
    put(fps, R('DIP1'), 86.5, 46.5, 0)
    put(fps, R('G1'), 84.5, 40.0, 90)             # outputs (pads 4/5) face D1
    put(fps, R('R68'), 84.0, 36.2, 90)            # CLKM series 0R
    put(fps, R('R67'), 85.6, 36.2, 90)            # CLKP series 0R
    put(fps, R('R69'), 84.8, 32.75, 180)          # 100 R LVDS termination
    put(fps, R('C47'), 82.4, 36.2, 90)            # 100n at G1 VCC
    put(fps, R('R66'), 83.0, 43.2, 0)             # EN pull-up
    put(fps, R('C48'), 78.8, 34.0, 0)
    # ferrite filters and bulk capacitors (left of the DDS)
    put(fps, R('FB1'), 78.0, 4.0, 0)
    put(fps, R('C10'), 78.0, 6.0, 0)
    put(fps, R('FB2'), 75.5, 36.5, 0)
    put(fps, R('C11'), 78.8, 36.5, 0)
    put(fps, R('FB3'), 78.0, 8.5, 0)
    put(fps, R('C12'), 78.0, 10.5, 0)
    put(fps, R('C35'), 77.5, 14.0, 90)
    put(fps, R('C36'), 83.0, 3.5, 0)
    put(fps, R('C37'), 86.5, 3.5, 0)
    put(fps, R('C38'), 77.5, 17.5, 90)


# ------------------------------------------------------------------ rules --

def netclass(name, clearance, width, via_d=0.6, via_drill=0.3, dp_w=0.2, dp_gap=0.25, prio=10):
    return {"name": name, "clearance": clearance, "track_width": width, "via_diameter": via_d,
            "via_drill": via_drill, "microvia_diameter": 0.3, "microvia_drill": 0.1,
            "diff_pair_width": dp_w, "diff_pair_gap": dp_gap, "diff_pair_via_gap": 0.25,
            "wire_width": 6, "bus_width": 12, "line_style": 0, "priority": prio,
            "pcb_color": "rgba(0, 0, 0, 0.000)", "schematic_color": "rgba(0, 0, 0, 0.000)"}


def class_nets(proj):
    R = proj.ref

    def nets_of(keys_pins):
        out = set()
        for key, pin in keys_pins:
            n = proj.pin_net.get((R(key), pin))
            if n and n != 'GND':
                out.add(n)
        return out
    rf = nets_of([('U1', '43'), ('C51', '2'), ('L4', '2'), ('L5', '2'), ('L6', '2'), ('L7', '2'),
                  ('R76', '2'), ('J3', '1'), ('U1', '42')])
    clk = nets_of([('J2', '1'), ('U1', '18'), ('U1', '19'), ('U1', '23'), ('U1', '24'),
                   ('G1', '4'), ('G1', '5')])
    pwr = {n for n in proj.nets if n.startswith('+')}
    pwr |= nets_of([('J1', '1'), ('D1', '2'), ('D1', '1'), ('U3', '7'), ('U4', '7'), ('U5', '7')])
    return rf, clk, pwr


def apply_rules(proj, b):
    """Net classes (exact net names from the netlist) and board rules.
    Stack-up assumption (JLC04161H-7628 like): F.Cu-In1 prepreg 0.21 mm, er 4.4
    -> 50 ohm microstrip ~0.36 mm; confirm with the manufacturer's calculator."""
    rf, clk, pwr = class_nets(proj)
    ds = b.GetDesignSettings()
    ns = ds.m_NetSettings
    ns.ClearNetclasses() if hasattr(ns, 'ClearNetclasses') else None
    ns.ClearNetclassPatternAssignments()
    dflt = ns.GetDefaultNetclass()
    # JLCPCB 4-layer capability: 0.09/0.09 mm min (we use 0.10/0.10 as the floor),
    # vias 0.3/0.6 standard, 0.2/0.45 minimum
    dflt.SetClearance(MM(0.127)); dflt.SetTrackWidth(MM(0.15))
    dflt.SetViaDiameter(MM(0.45)); dflt.SetViaDrill(MM(0.2))

    def nc(name, clr, w, vd=0.45, vdr=0.2, prio=10):
        c = pcbnew.NETCLASS(name)
        c.SetClearance(MM(clr)); c.SetTrackWidth(MM(w))
        c.SetViaDiameter(MM(vd)); c.SetViaDrill(MM(vdr))
        c.SetDiffPairWidth(MM(0.2)); c.SetDiffPairGap(MM(0.25))
        c.SetPriority(prio)
        ns.SetNetclass(name, c)
    nc('RF50', 0.2, 0.3, prio=0)
    nc('PWR', 0.15, 0.3, 0.6, 0.3, prio=1)
    nc('CLK', 0.2, 0.25, prio=2)
    nc('CLK50', 0.2, 0.36, prio=3)           # 480 MHz input line: ~50 ohm microstrip
    ext = {proj.pin_net[(proj.ref('J2'), '1')]}
    for cls, nets in (('RF50', rf), ('CLK', clk - ext), ('CLK50', ext), ('PWR', pwr)):
        for n in sorted(nets):
            ns.SetNetclassPatternAssignment(n, cls)
    ds.m_MinClearance = MM(0.1)
    ds.m_TrackMinWidth = MM(0.1)
    ds.m_ViasMinSize = MM(0.45)
    ds.m_ViasMinAnnularWidth = MM(0.125)
    ds.m_MinThroughDrill = MM(0.2)
    ds.m_CopperEdgeClearance = MM(0.3)
    ds.m_HoleToHoleMin = MM(0.25)
    ds.m_HoleClearance = MM(0.2)
    b.SynchronizeNetsAndNetClasses(True)
    print('rules: rf', len(rf), 'clk', len(clk), 'pwr', len(pwr))

def fiducials_and_holes(b):
    for x, y in ((3.5, 3.5), (BW - 3.5, 3.5), (3.5, BH - 3.5) if False else (24.0, 47.0)):
        pass


def save_project(sm, pro):
    """pcbnew's SaveProject() drops the schematic section of the .kicad_pro
    (sub-unit naming D1.1, ГОСТ drawing sheet, sheet list): merge it back."""
    old = json.loads(Path(pro).read_text(encoding='utf-8'))
    sm.SaveProject(pro)
    new = json.loads(Path(pro).read_text(encoding='utf-8'))
    sch = new.setdefault('schematic', {})
    for k, v in old.get('schematic', {}).items():
        if k not in sch or k in ('subpart_id_separator', 'subpart_first_id', 'page_layout_descr_file', 'drawing'):
            sch[k] = v
    for k in ('sheets', 'text_variables', 'erc', 'libraries'):
        if old.get(k) and not new.get(k):
            new[k] = old[k]
    Path(pro).write_text(json.dumps(new, indent=2, ensure_ascii=False), encoding='utf-8')


def main():
    cmd, d = sys.argv[1], sys.argv[2]
    proj = Proj(d)
    pcb = Path(d) / 'DDS_47_51.kicad_pcb'
    pro = str(Path(d) / 'DDS_47_51.kicad_pro')
    sm = pcbnew.GetSettingsManager()
    sm.LoadProject(pro)
    if cmd == 'place':
        b, fps = new_board(proj)
        place_all(proj, fps)
        gnd_zone(b, pcbnew.In1_Cu)              # solid GND plane on layer 2
        manual_fixes(proj, b)                   # locked VOS sense tracks (pre-route)
        clock_routes(proj, b)                   # locked 480 MHz microstrips
        dac_routes(proj, b)                     # locked DAC1 output escape
        print('GND pre-fanout vias', pre_fanout(proj, b))
        b.SetFileName(str(pcb))
        apply_rules(proj, b)
        pcbnew.SaveBoard(str(pcb), b)
        save_project(sm, pro)
        print('placed', len(fps))
    elif cmd == 'dsn':
        b = pcbnew.LoadBoard(str(pcb))
        apply_rules(proj, b)
        print('dsn', pcbnew.ExportSpecctraDSN(b, sys.argv[3]))
    elif cmd == 'route':
        route(proj, d, int(sys.argv[3]) if len(sys.argv) > 3 else 3)
        b = pcbnew.LoadBoard(str(pcb))
        apply_rules(proj, b)
        add_zones(b)
        pcbnew.SaveBoard(str(pcb), b)
        save_project(sm, pro)
    elif cmd == 'import':                      # import routing only (iterative routing)
        b = pcbnew.LoadBoard(str(pcb))
        apply_rules(proj, b)
        print('ses', pcbnew.ImportSpecctraSES(b, sys.argv[3]))
        pcbnew.SaveBoard(str(pcb), b)
    elif cmd == 'zones':
        b = pcbnew.LoadBoard(str(pcb))
        apply_rules(proj, b)
        add_zones(b)
        pcbnew.SaveBoard(str(pcb), b)
        save_project(sm, pro)
    elif cmd == 'ses':
        b = pcbnew.LoadBoard(str(pcb))
        apply_rules(proj, b)
        ok = pcbnew.ImportSpecctraSES(b, sys.argv[3])
        print('ses', ok)
        add_zones(b)
        pcbnew.SaveBoard(str(pcb), b)
        save_project(sm, pro)


# ------------------------------------------------------------- routing ------

FR_JAR = os.environ.get('FREEROUTING_JAR', '/opt/fr/freerouting-2.5.0.jar')


def seq(container):
    """Robust list() for SWIG containers (their Python iterator is flaky in KiCad 10)."""
    try:
        return list(container)
    except Exception:
        return [container[i] for i in range(len(container))]


def tracks(b):
    return seq(b.Tracks())


def unrouted(b):
    b.BuildConnectivity()
    return b.GetConnectivity().GetUnconnectedCount(False)


def freeroute(proj, b, work, tag, passes=1):
    import subprocess
    dsn, ses = work / f'{tag}.dsn', work / f'{tag}.ses'
    apply_rules(proj, b)
    pcbnew.ExportSpecctraDSN(b, str(dsn))
    restrict_layers(dsn)
    with open(work / f'{tag}.log', 'w') as lg:
        subprocess.run(['java', '-Djava.awt.headless=true', '-Xmx3g', '-jar', FR_JAR, '-de', str(dsn),
                        '-do', str(ses), '-mp', str(passes), '-mt', '2', '--gui.enabled=false'],
                       stdout=lg, stderr=subprocess.STDOUT, check=False)
    ok = pcbnew.ImportSpecctraSES(b, str(ses))
    return ok


def track(b, net, layer, pts, w=0.2, locked=False):
    for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
        t = pcbnew.PCB_TRACK(b)
        t.SetStart(V(x1, y1)); t.SetEnd(V(x2, y2))
        t.SetWidth(MM(w)); t.SetLayer(layer); t.SetNet(b.FindNet(net))
        t.SetLocked(locked)
        b.Add(t)


def via(b, net, x, y, d=0.45, drill=0.2):
    if net.startswith('+'):
        d, drill = 0.6, 0.3
    v = pcbnew.PCB_VIA(b)
    v.SetPosition(V(x, y)); v.SetWidth(MM(d)); v.SetDrill(MM(drill))
    v.SetNet(b.FindNet(net)); v.SetViaType(pcbnew.VIATYPE_THROUGH)
    b.Add(v)
    return v


def stitch(b, pitch=2.5, margin=0.42):
    """GND stitching vias wherever F.Cu, In2.Cu and B.Cu GND fills overlap with margin."""
    fills = {}
    for z in seq(b.Zones()):
        if z.GetNetname() == 'GND' and z.GetLayer() in (pcbnew.F_Cu, pcbnew.In2_Cu, pcbnew.B_Cu):
            ps = pcbnew.SHAPE_POLY_SET(z.GetFilledPolysList(z.GetLayer()))
            ps.Deflate(MM(margin), pcbnew.CORNER_STRATEGY_ROUND_ALL_CORNERS, MM(0.01))
            fills[z.GetLayer()] = ps
    holes = []
    for fp in seq(b.Footprints()):
        for p in seq(fp.Pads()):
            if p.GetDrillSize().x > 0:
                holes.append((local(p.GetPosition()), pcbnew.ToMM(max(p.GetSize().x, p.GetSize().y)) / 2))
    for t in tracks(b):
        if t.GetClass() == 'PCB_VIA':
            holes.append((local(t.GetPosition()), pcbnew.ToMM(t.GetWidth()) / 2))
    n = 0
    y = 1.5
    while y < BH - 1.2:
        x = 1.5
        while x < BW - 1.2:
            p = V(x, y)
            if all(f.Contains(p) for f in fills.values()) and \
                    all(math.hypot(x - hx, y - hy) > hr + 0.6 for (hx, hy), hr in holes):
                via(b, 'GND', x, y)
                holes.append(((x, y), 0.225))
                n += 1
            x += pitch
        y += pitch
    return n


def restrict_layers(dsn, classes=('CLK', 'CLK50')):
    """Freerouting: keep the 480 MHz clock classes on F.Cu (over the In1 plane)."""
    s = Path(dsn).read_text(encoding='utf-8')
    out, i = [], 0
    while True:
        j = s.find('(class ', i)
        if j < 0:
            break
        name = s[j + 7:].split()[0]
        k = s.find('(circuit', j)
        nxt = s.find('(class ', j + 1)
        if name in classes and k > 0 and (nxt < 0 or k < nxt):
            out.append(s[i:k + 8]); out.append('\n        (use_layer F.Cu)')
            i = k + 8
        else:
            out.append(s[i:j + 7]); i = j + 7
    out.append(s[i:])
    Path(dsn).write_text(''.join(out), encoding='utf-8')


def clock_routes(proj, b):
    """Locked F.Cu routes of the 480 MHz paths (microstrip over the In1 plane)."""
    R = proj.ref
    fp = lambda k: b.FindFootprintByReference(R(k))
    pn = lambda k, p: proj.pin_net[(R(k), p)]
    # LVDS pair: G1 -> R67/R68 (0R) -> R69 (100R) -> D1 pins 24/23, w 0.25, s 0.25
    p24, p23 = pad_xy(fp('U1'), '24'), pad_xy(fp('U1'), '23')
    t1, t2 = pad_xy(fp('R69'), '1'), pad_xy(fp('R69'), '2')          # CLKP, CLKM
    s1, s2 = pad_xy(fp('R67'), '2'), pad_xy(fp('R68'), '2')
    g4, g5 = pad_xy(fp('G1'), '4'), pad_xy(fp('G1'), '5')
    r67a, r68a = pad_xy(fp('R67'), '1'), pad_xy(fp('R68'), '1')
    xp, xm = p24[0] - 1.65, p24[0] - 2.15
    dp = t1[0] - xp                                   # 45 deg from R69 to the vertical
    track(b, pn('U1', '24'), pcbnew.F_Cu, [s1, t1, (xp, t1[1] + dp), (xp, p24[1]), p24], 0.25, locked=True)
    dm = xm - t2[0]
    track(b, pn('U1', '23'), pcbnew.F_Cu, [s2, t2, (xm, t2[1] - dm), (xm, p23[1]), p23], 0.25, locked=True)
    track(b, pn('G1', '4'), pcbnew.F_Cu, [g4, r67a], 0.25, locked=True)
    track(b, pn('G1', '5'), pcbnew.F_Cu, [g5, r68a], 0.25, locked=True)
    # external 480 MHz input: XW1 -> north along x = 81 -> R70 (termination) -> C49
    j = pad_xy(fp('J2'), '1')
    r70, c49 = pad_xy(fp('R70'), '1'), pad_xy(fp('C49'), '1')
    x = 81.0
    net = pn('J2', '1')
    track(b, net, pcbnew.F_Cu, [j, (j[0], 44.0), (x, 44.0 - (x - j[0])), (x, r70[1] + 0.6),
                                (x + 0.6, r70[1]), r70, c49], 0.36, locked=True)


def dac_routes(proj, b):
    """Locked F.Cu escape of the DAC1 output pair: OUTM1 to its termination R73,
    OUTP1 straight down past its shunt R72 to the output filter (C51)."""
    R = proj.ref
    fp = lambda k: b.FindFootprintByReference(R(k))
    pn = lambda k, p: proj.pin_net[(R(k), p)]
    m = next(p for p in ('41', '42', '43', '44') if pn('U1', p) == '/OUTM1')
    pp = next(p for p in ('41', '42', '43', '44') if pn('U1', p) == '/OUTP1')
    pm, ppp = pad_xy(fp('U1'), m), pad_xy(fp('U1'), pp)
    r73 = pad_xy(fp('R73'), '1' if pn('R73', '1') == '/OUTM1' else '2')
    r72 = pad_xy(fp('R72'), '1' if pn('R72', '1') == '/OUTP1' else '2')
    c51 = pad_xy(fp('C51'), '1' if pn('C51', '1') == '/OUTP1' else '2')
    y1 = pm[1] + 1.8
    track(b, '/OUTM1', pcbnew.F_Cu, [pm, (pm[0], y1), (r73[0], y1 + (pm[0] - r73[0])), r73], 0.25, locked=True)
    x = ppp[0]
    track(b, '/OUTP1', pcbnew.F_Cu, [ppp, (x, y1)], 0.25, locked=True)
    track(b, '/OUTP1', pcbnew.F_Cu, [(x, y1), (x, r72[1]), (x, c51[1] - abs(x - c51[0])), c51], 0.3, locked=True)
    track(b, '/OUTP1', pcbnew.F_Cu, [(x, r72[1]), r72], 0.3, locked=True)


def manual_fixes(proj, b):
    """Short connections the autorouter cannot enter: VOS sense of the DC-DC converters."""
    R = proj.ref
    for u, L in (('U3', 'L1'), ('U4', 'L2'), ('U5', 'L3')):
        fp = b.FindFootprintByReference(R(u))
        lf = b.FindFootprintByReference(R(L))
        vx, vy = pad_xy(fp, '6')
        out = '2' if proj.pin_net[(R(L), '2')] == proj.pin_net[(R(u), '6')] else '1'
        lx, ly = pad_xy(lf, out)
        net = proj.pin_net[(R(u), '6')]
        if not any(t.GetNetname() == net and t.GetClass() == 'PCB_TRACK' and
                   math.hypot(*(a - c for a, c in zip(local(t.GetStart()), (vx, vy)))) < 0.05
                   for t in tracks(b)):
            track(b, net, pcbnew.F_Cu, [(vx, vy), (vx + 0.75, vy), (vx + 0.75, vy + 2.4),
                                         (lx, vy + 2.4), (lx, ly + 1.85)], 0.2, locked=True)


def route(proj, d, iterations=3):
    work = Path(d) / 'pcb_route'
    work.mkdir(exist_ok=True)
    pcb = Path(d) / 'DDS_47_51.kicad_pcb'
    b = pcbnew.LoadBoard(str(pcb))
    apply_rules(proj, b)
    print('start: unrouted', unrouted(b))
    freeroute(proj, b, work, 'fr0', 1)
    print('pass 0: unrouted', unrouted(b))
    for k in range(1, iterations + 1):
        before = unrouted(b)
        if before == 0:
            break
        freeroute(proj, b, work, f'fr{k}', 1)
        print(f'pass {k}: unrouted {before} -> {unrouted(b)}')
    pcbnew.SaveBoard(str(pcb), b)
    print('saved; unrouted', unrouted(b))


def pre_fanout(proj, b):
    """Locked GND fanout before autorouting: every GND pin of the two QFPs gets a
    short stub inwards to a via into the In1 plane; every decoupling capacitor
    around them gets a via next to its GND pad."""
    R = proj.ref
    n = 0
    boxes = []
    for fp_ in seq(b.Footprints()):
        for p_ in seq(fp_.Pads()):
            bb = p_.GetBoundingBox()
            boxes.append((p_.GetNetname(), pcbnew.ToMM(bb.GetLeft()) - O[0], pcbnew.ToMM(bb.GetTop()) - O[1],
                          pcbnew.ToMM(bb.GetRight()) - O[0], pcbnew.ToMM(bb.GetBottom()) - O[1]))

    segs = [(t_.GetNetname(), local(t_.GetStart()), local(t_.GetEnd()), pcbnew.ToMM(t_.GetWidth()))
            for t_ in tracks(b) if t_.GetClass() == 'PCB_TRACK' and t_.GetNetname() != 'GND']

    def seg_dist(p, a, c):
        ax, ay = a; cx_, cy_ = c; px_, py_ = p
        L2 = (cx_ - ax) ** 2 + (cy_ - ay) ** 2
        u = 0 if L2 == 0 else max(0, min(1, ((px_ - ax) * (cx_ - ax) + (py_ - ay) * (cy_ - ay)) / L2))
        return math.hypot(px_ - ax - u * (cx_ - ax), py_ - ay - u * (cy_ - ay))

    def clear(x, y, r):
        return all(nm == 'GND' or x < l - r or x > rr + r or y < t - r or y > bt + r
                   for nm, l, t, rr, bt in boxes) and \
            all(seg_dist((x, y), a, c) >= r + w / 2 for nm, a, c, w in segs)
    for key in ('U1', 'U2'):
        fp = b.FindFootprintByReference(R(key))
        cx, cy = local(fp.GetPosition())
        gpads = [p for p in seq(fp.Pads()) if p.GetNetname() == 'GND']
        gpads.sort(key=lambda p: int(p.GetNumber()))
        for k, p in enumerate(gpads):
            px, py = local(p.GetPosition())
            sx, sy = pcbnew.ToMM(p.GetSize().x), pcbnew.ToMM(p.GetSize().y)
            if abs(px - cx) > abs(py - cy):          # left/right side, pad long in x
                ux, uy = (-1 if px > cx else 1), 0
                inner = (px + ux * sx / 2, py)
            else:
                ux, uy = 0, (-1 if py > cy else 1)
                inner = (px, py + uy * sy / 2)
            depth = 0.75 if k % 2 == 0 else 1.45
            vx, vy = inner[0] + ux * depth, inner[1] + uy * depth
            track(b, 'GND', pcbnew.F_Cu, [(px, py), (vx, vy)], 0.25, locked=True)
            v = via(b, 'GND', vx, vy)
            v.SetLocked(True)
            n += 1
    # decoupling capacitors of D1/D2 and the bulk capacitors: via beside the GND pad
    caps = [f'C{i}' for i in range(13, 39)] + ['C40', 'C41', 'C42', 'C43', 'C10', 'C11', 'C12',
                                                 'C45', 'C46', 'C47', 'C48', 'C50', 'R70']
    for key in caps:
        fp = b.FindFootprintByReference(R(key))
        pads = {p.GetNumber(): p for p in seq(fp.Pads())}
        g = next((p for p in pads.values() if p.GetNetname() == 'GND'), None)
        o = next((p for p in pads.values() if p.GetNetname() != 'GND'), None)
        if g is None or o is None:
            continue
        gx, gy = local(g.GetPosition()); ox, oy = local(o.GetPosition())
        dx, dy = gx - ox, gy - oy
        L = math.hypot(dx, dy) or 1
        vx, vy = gx + dx / L * 0.95, gy + dy / L * 0.95
        if not clear(vx, vy, 0.45):
            continue
        track(b, 'GND', pcbnew.F_Cu, [(gx, gy), (vx, vy)], 0.3, locked=True)
        v = via(b, 'GND', vx, vy)
        v.SetLocked(True)
        n += 1
    return n


def gnd_zone(b, layer, prio=0):
    z = pcbnew.ZONE(b)
    z.SetLayer(layer)
    z.SetNet(b.FindNet('GND'))
    z.SetLocalClearance(MM(0.25))
    z.SetMinThickness(MM(0.2))
    z.SetPadConnection(pcbnew.ZONE_CONNECTION_FULL if layer == pcbnew.In1_Cu
                       else pcbnew.ZONE_CONNECTION_THERMAL)
    z.SetThermalReliefGap(MM(0.25))
    z.SetThermalReliefSpokeWidth(MM(0.3))
    z.SetAssignedPriority(prio)
    z.SetIslandRemovalMode(pcbnew.ISLAND_REMOVAL_MODE_ALWAYS)
    ol = z.Outline()
    ol.NewOutline()
    for x, y in ((0.3, 0.3), (BW - 0.3, 0.3), (BW - 0.3, BH - 0.3), (0.3, BH - 0.3)):
        ol.Append(MM(O[0] + x), MM(O[1] + y))
    b.Add(z)
    return z


def add_zones(b):
    """GND pours on the outer and In2 layers (In1 plane exists from placement),
    stitching vias, refill."""
    have = {z.GetLayer() for z in b.Zones()}
    for layer in (pcbnew.F_Cu, pcbnew.In2_Cu, pcbnew.B_Cu):
        if layer not in have:
            gnd_zone(b, layer)
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())
    n = stitch(b)
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())
    print('stitching vias', n)


if __name__ == '__main__':
    main()
