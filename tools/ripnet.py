"""Remove the (unlocked) tracks and vias of the given nets (KiCad python).
    python3 tools/ripnet.py board.kicad_pcb NET [NET ...]"""
import sys
import pcbnew
b = pcbnew.LoadBoard(sys.argv[1])
nets = set(sys.argv[2:])
tr = b.Tracks()
kill = [t for t in (tr[i] for i in range(len(tr))) if t.GetNetname() in nets and not t.IsLocked()]
for t in kill:
    b.Remove(t)
pcbnew.SaveBoard(sys.argv[1], b)
print('ripnet: removed', len(kill))
