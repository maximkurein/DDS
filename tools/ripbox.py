"""Remove unlocked tracks/vias of NET having an end inside the box (local mm).
    python3 tools/ripbox.py board.kicad_pcb NET x0 y0 x1 y1"""
import sys
import pcbnew
from dds_pcb import O
b = pcbnew.LoadBoard(sys.argv[1])
net = sys.argv[2]
x0, y0, x1, y1 = map(float, sys.argv[3:7])
L = lambda p: (pcbnew.ToMM(p.x) - O[0], pcbnew.ToMM(p.y) - O[1])
inside = lambda p: x0 <= p[0] <= x1 and y0 <= p[1] <= y1
tr = b.Tracks()
kill = [t for t in (tr[i] for i in range(len(tr)))
        if t.GetNetname() == net and not t.IsLocked() and (inside(L(t.GetStart())) or inside(L(t.GetEnd())))]
for t in kill:
    b.Remove(t)
pcbnew.SaveBoard(sys.argv[1], b)
print('ripbox:', net, 'removed', len(kill))
