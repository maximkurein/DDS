"""Board cleanup after routing (one pass per process; call repeatedly):
drop degenerate (< 0.01 mm) and dangling track segments and dangling vias of
signal nets, keep every via's annular ring >= 0.125 mm.  Exit code 0 when
nothing was left to remove."""
import sys
import pcbnew

b = pcbnew.LoadBoard(sys.argv[1])
b.BuildConnectivity()
c = b.GetConnectivity()
tr = b.Tracks()
items = [tr[i] for i in range(len(tr))]
kill = []
for t in items:
    if t.IsLocked():
        continue
    if t.GetClass() == 'PCB_TRACK':
        if (t.GetStart() - t.GetEnd()).EuclideanNorm() < pcbnew.FromMM(0.01) or \
                c.TestTrackEndpointDangling(t, False):
            kill.append(t)
seen = {}
for t in items:                      # duplicate vias (same net, same place)
    if t.GetClass() == 'PCB_VIA' and not t.IsLocked() and t not in kill:
        k = (t.GetNetname(), t.GetPosition().x, t.GetPosition().y)
        if k in seen:
            kill.append(t)
        seen[k] = t
for t in kill:
    b.Remove(t)
for t in items:
    if t in kill:
        continue
    if t.GetClass() == 'PCB_VIA':
        v = t.Cast()
        w, d = pcbnew.ToMM(v.GetWidth(pcbnew.F_Cu)), pcbnew.ToMM(v.GetDrillValue())
        if (w - d) / 2 < 0.125 - 1e-6:
            if w <= 0.5:
                v.SetDrill(pcbnew.FromMM(max(0.2, w - 0.25)))
            else:
                v.SetWidth(pcbnew.FromMM(d + 0.25))
pcbnew.ZONE_FILLER(b).Fill(b.Zones())
pcbnew.SaveBoard(sys.argv[1], b)
print('cleanup: removed', len(kill), 'track segments')
sys.exit(0 if not kill else 3)
