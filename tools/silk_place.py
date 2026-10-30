"""Place silkscreen reference designators clear of pads, other silk and the
board edge (KiCad python).  Only references that are on F.Silkscreen are
moved; small parts keep their designator on F.Fab (assembly drawing).

    python3 tools/silk_place.py board.kicad_pcb
"""
import sys
import pcbnew
from shapely.geometry import LineString, Polygon, box, Point
from shapely.ops import unary_union

MM = pcbnew.FromMM
mm = pcbnew.ToMM
CLR = 0.2          # silk-to-mask / silk-to-silk clearance, mm


def seq(c):
    return [c[i] for i in range(len(c))] if hasattr(c, '__len__') else list(c)


def shape_geom(s):
    w = max(mm(s.GetWidth()), 0.05) / 2
    t = s.GetShape()
    if t == pcbnew.SHAPE_T_SEGMENT:
        a, b = s.GetStart(), s.GetEnd()
        g = LineString([(mm(a.x), mm(a.y)), (mm(b.x), mm(b.y))])
    elif t == pcbnew.SHAPE_T_RECTANGLE:
        bb = s.GetBoundingBox()
        g = box(mm(bb.GetLeft()), mm(bb.GetTop()), mm(bb.GetRight()), mm(bb.GetBottom())).exterior
    elif t == pcbnew.SHAPE_T_CIRCLE:
        c = s.GetCenter()
        g = Point(mm(c.x), mm(c.y)).buffer(mm(s.GetRadius())).exterior
    else:
        bb = s.GetBoundingBox()
        g = box(mm(bb.GetLeft()), mm(bb.GetTop()), mm(bb.GetRight()), mm(bb.GetBottom())).exterior
    return g.buffer(w)


def bbox_geom(item, grow=0.0):
    bb = item.GetBoundingBox()
    return box(mm(bb.GetLeft()) - grow, mm(bb.GetTop()) - grow,
               mm(bb.GetRight()) + grow, mm(bb.GetBottom()) + grow)


def main(path):
    b = pcbnew.LoadBoard(path)
    ebb = b.GetBoardEdgesBoundingBox()
    inner = box(mm(ebb.GetLeft()) + 0.6, mm(ebb.GetTop()) + 0.6,
                mm(ebb.GetRight()) - 0.6, mm(ebb.GetBottom()) - 0.6)
    fps = seq(b.GetFootprints())
    pads = unary_union([bbox_geom(p, CLR) for fp in fps for p in seq(fp.Pads())])
    graphics = []
    for fp in fps:
        for g in seq(fp.GraphicalItems()):
            if g.GetLayer() != pcbnew.F_SilkS:
                continue
            if g.GetClass() in ('PCB_SHAPE', 'FP_SHAPE'):
                graphics.append(shape_geom(g.Cast()))
            else:
                graphics.append(bbox_geom(g))
    graphics = unary_union(graphics) if graphics else Polygon()
    texts = []
    order = sorted((fp for fp in fps if fp.Reference().GetLayer() == pcbnew.F_SilkS
                    and fp.Reference().IsVisible()),
                   key=lambda f: -mm(f.GetCourtyard(pcbnew.F_CrtYd).BBox().GetWidth()))
    moved = 0
    for fp in order:
        ref = fp.Reference()
        cy = fp.GetCourtyard(pcbnew.F_CrtYd).BBox()
        l, t, r, bo = mm(cy.GetLeft()), mm(cy.GetTop()), mm(cy.GetRight()), mm(cy.GetBottom())
        cx, cyy = (l + r) / 2, (t + bo) / 2
        cand = [(cx, cyy, 0)]
        for d in (0, 1, -1, 2, -2, 3, -3, 4, -4):
            cand += [(cx + d, t - 0.75, 0), (cx + d, bo + 0.75, 0)]
        for d in (0, 1, -1, 2, -2):
            cand += [(l - 0.75, cyy + d, 90), (r + 0.75, cyy + d, 90)]
        best = None
        for x, y, ang in cand:
            ref.SetTextAngleDegrees(ang)
            ref.SetPosition(pcbnew.VECTOR2I(MM(x), MM(y)))
            g = bbox_geom(ref, 0.0)
            if not inner.contains(g):
                continue
            if g.intersects(pads) or g.intersects(graphics):
                continue
            if any(g.intersects(o) for o in texts):
                continue
            best = (x, y, ang)
            break
        if best is None:
            ref.SetLayer(pcbnew.F_Fab)       # no room: keep it on the assembly drawing
            print('silk_place:', fp.GetReference(), '-> F.Fab (no free place)')
            continue
        ref.SetTextAngleDegrees(best[2])
        ref.SetPosition(pcbnew.VECTOR2I(MM(best[0]), MM(best[1])))
        texts.append(bbox_geom(ref, CLR))
        moved += 1
    pcbnew.SaveBoard(path, b)
    print('silk_place: placed', moved, 'references')


if __name__ == '__main__':
    main(sys.argv[1])
