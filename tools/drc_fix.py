"""Delete track segments named in DRC clearance/short violations (KiCad python),
so that tools/minirouter.py can route those connections again."""
import re
import sys
import pcbnew

pcb, rpt = sys.argv[1], sys.argv[2]
t = open(rpt, encoding='utf-8').read()
bad = []
thermal = []
for blk in re.split(r'\n(?=\[)', t):
    kind0 = blk.split(':')[0]
    if kind0 == '[starved_thermal]':
        for x, y, pad, ref in re.findall(r'@\(([\d.]+) mm, ([\d.]+) mm\): PTH pad (\S+) \[GND\] of (\S+)', blk):
            thermal.append((ref, pad))
    if kind0 in ('[clearance]', '[shorting_items]', '[hole_clearance]', '[hole_to_hole]',
                 '[via_dangling]', '[track_dangling]'):
        for x, y, kind, net in re.findall(r'@\(([\d.]+) mm, ([\d.]+) mm\): (Track|Via) \[([^\]]+)\]', blk):
            bad.append((float(x), float(y), kind, net))
b = pcbnew.LoadBoard(pcb)
tr = b.Tracks()
items = [tr[i] for i in range(len(tr))]
kill = []
for x, y, kind, net in bad:
    p = pcbnew.VECTOR2I(pcbnew.FromMM(x), pcbnew.FromMM(y))
    for it in items:
        if it.IsLocked() or it.GetNetname() != net or it in kill:
            continue
        if (kind == 'Via') != (it.GetClass() == 'PCB_VIA'):
            continue
        if it.HitTest(p, pcbnew.FromMM(0.02)):
            kill.append(it)
for it in kill:
    b.Remove(it)
# a THT GND pad whose thermal spokes on an inner layer reach only a pour fragment:
# connect it solidly (it is tied to GND on the other layers anyway)
for ref, pad in thermal:
    fp = b.FindFootprintByReference(ref)
    for pd in fp.Pads():
        if pd.GetNumber() == pad:
            pd.SetLocalZoneConnection(pcbnew.ZONE_CONNECTION_FULL)
if thermal:
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())
pcbnew.SaveBoard(pcb, b)
print('drc_fix: removed', len(kill), 'items; solid pads', thermal)
