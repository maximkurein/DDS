#!/usr/bin/env python3
"""Finish the few connections the autorouter left open (run with KiCad's python).

    python3 tools/minirouter.py <project_dir>

For every unconnected pair reported by DRC, try short candidate routes:
  1. a straight track on F.Cu,
  2. via near each end + straight track on B.Cu or In2.Cu (+ short F.Cu stubs),
and keep the first one that clears all other-net copper (shapely geometry).
GND pours are ignored (they are refilled afterwards), the In1 plane too.
"""
import math
import sys
from pathlib import Path

import pcbnew
from shapely.geometry import LineString, Point, Polygon, box
from shapely.ops import unary_union
from shapely.strtree import STRtree

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dds_pcb import (Proj, apply_rules, seq, tracks, track, via, add_zones, O, BW, BH, MM)  # noqa

CLR = 0.21          # routing clearance (largest net-class clearance + margin)
CLR_NET = {}        # per-net clearance (filled from the net classes)
FAILED = []
W = 0.2             # track width for completions
VIA_D = 0.45
LAYERS = {'F': pcbnew.F_Cu, 'I': pcbnew.In2_Cu, 'B': pcbnew.B_Cu}
ALLOWED = set(LAYERS)   # layers the current net may use (RF/CLK nets: F.Cu only,
                        # microstrip over the solid In1 GND plane)


def mm(v):
    return pcbnew.ToMM(v)


def obstacles(b):
    """{layer: [(netname, geometry)]} of copper that is not a GND pour."""
    _TREES.clear()
    obs = {k: [] for k in LAYERS}
    for fp in seq(b.Footprints()):
        for p in seq(fp.Pads()):
            poly = p.GetEffectivePolygon(pcbnew.F_Cu) if p.IsOnLayer(pcbnew.F_Cu) else None
            sh = None
            if poly is not None and poly.OutlineCount():
                ol = poly.Outline(0)
                pts = [(mm(ol.CPoint(i).x) - O[0], mm(ol.CPoint(i).y) - O[1]) for i in range(ol.PointCount())]
                if len(pts) >= 3:
                    sh = Polygon(pts)
            if sh is None:
                x, y = mm(p.GetPosition().x) - O[0], mm(p.GetPosition().y) - O[1]
                r = max(mm(p.GetSize().x), mm(p.GetSize().y)) / 2
                sh = Point(x, y).buffer(r)
            thru = p.GetDrillSize().x > 0
            for k, L in LAYERS.items():
                if thru or p.IsOnLayer(L):
                    obs[k].append((p.GetNetname(), sh))
    for t in tracks(b):
        n = t.GetNetname()
        if t.GetClass() == 'PCB_VIA':
            x, y = mm(t.GetPosition().x) - O[0], mm(t.GetPosition().y) - O[1]
            g = Point(x, y).buffer(mm(t.GetWidth()) / 2)
            for k in LAYERS:
                obs[k].append((n, g))
        else:
            for k, L in LAYERS.items():
                if t.GetLayer() == L:
                    s, e = t.GetStart(), t.GetEnd()
                    g = LineString([(mm(s.x) - O[0], mm(s.y) - O[1]), (mm(e.x) - O[0], mm(e.y) - O[1])]
                                   ).buffer(mm(t.GetWidth()) / 2)
                    obs[k].append((n, g))
    return obs


_TREES = {}


def free(obs, layer, geom, net):
    edge = box(0.3, 0.3, BW - 0.3, BH - 0.3)
    if not edge.contains(geom):
        return False
    key = (id(obs), layer)
    if key not in _TREES:
        _TREES[key] = STRtree([o for n, o in obs[layer]])
    tree = _TREES[key]
    g = geom.buffer(CLR_NET.get(net, 0.16))
    for idx in tree.query(g):
        n, o = obs[layer][idx]
        if n != net and g.intersects(o):
            return False
    return True


def via_d(net):
    return 0.6 if net.startswith('+') else VIA_D


def seg(a, c, w=None):
    return LineString([a, c]).buffer((W if w is None else w) / 2)


def try_pair(b, obs, net, A, B):
    # 1. straight on F.Cu
    if free(obs, 'F', seg(A, B), net):
        track(b, net, pcbnew.F_Cu, [A, B], W)
        return 'F'
    # 1b. two segments on F.Cu through an intermediate point
    xs = [min(A[0], B[0]) - 2 + 0.25 * i for i in range(int((abs(A[0] - B[0]) + 4) / 0.25) + 1)]
    ys = [min(A[1], B[1]) - 2 + 0.25 * i for i in range(int((abs(A[1] - B[1]) + 4) / 0.25) + 1)]
    cands = sorted(((x, y) for x in xs for y in ys), key=lambda m: math.dist(A, m) + math.dist(m, B))
    for m in cands[:2500]:
        if free(obs, 'F', seg(A, m), net) and free(obs, 'F', seg(m, B), net):
            track(b, net, pcbnew.F_Cu, [A, m, B], W)
            return 'F2'
    # 2. vias near both ends, inner/bottom layer between them
    if ALLOWED == {'F'}:
        return None
    dirs = [(math.cos(math.radians(a)), math.sin(math.radians(a))) for a in range(0, 360, 30)]
    for dist in (0.7, 1.0, 1.4, 1.9, 2.5, 3.2):
        va = [(A[0] + dx * dist, A[1] + dy * dist) for dx, dy in dirs]
        vb = [(B[0] + dx * dist, B[1] + dy * dist) for dx, dy in dirs]
        va.sort(key=lambda p: math.dist(p, B)); vb.sort(key=lambda p: math.dist(p, A))
        for pa in va[:12]:
            via_a = Point(pa).buffer(via_d(net) / 2)
            if not (free(obs, 'F', seg(A, pa), net) and all(free(obs, k, via_a, net) for k in LAYERS)):
                continue
            for pb in vb[:12]:
                via_b = Point(pb).buffer(via_d(net) / 2)
                if not (free(obs, 'F', seg(B, pb), net) and all(free(obs, k, via_b, net) for k in LAYERS)):
                    continue
                for k in ('B', 'I'):
                    if free(obs, k, seg(pa, pb), net):
                        track(b, net, pcbnew.F_Cu, [A, pa], W)
                        track(b, net, LAYERS[k], [pa, pb], W)
                        track(b, net, pcbnew.F_Cu, [pb, B], W)
                        via(b, net, *pa); via(b, net, *pb)
                        return k
    return None


def item_geoms(b, net):
    """[(kind, layers:set, {layer: geometry}, anchor points)] for copper of one net."""
    out = []
    for fp in seq(b.Footprints()):
        for p in seq(fp.Pads()):
            if p.GetNetname() != net:
                continue
            x, y = mm(p.GetPosition().x) - O[0], mm(p.GetPosition().y) - O[1]
            sx, sy = mm(p.GetSize().x), mm(p.GetSize().y)
            poly = p.GetEffectivePolygon(pcbnew.F_Cu)
            g = None
            if poly is not None and poly.OutlineCount():
                ol = poly.Outline(0)
                pts = [(mm(ol.CPoint(i).x) - O[0], mm(ol.CPoint(i).y) - O[1]) for i in range(ol.PointCount())]
                if len(pts) >= 3:
                    g = Polygon(pts).buffer(0)
            g = g or Point(x, y).buffer(max(sx, sy) / 2)
            thru = p.GetDrillSize().x > 0
            lay = set(LAYERS) | {'1'} if thru else {k for k, L in LAYERS.items() if p.IsOnLayer(L)}
            out.append(('pad', lay, {k: g for k in lay}, [(x, y)]))
    for t in tracks(b):
        if t.GetNetname() != net:
            continue
        if t.GetClass() == 'PCB_VIA':
            v = t.Cast()
            x, y = mm(v.GetPosition().x) - O[0], mm(v.GetPosition().y) - O[1]
            g = Point(x, y).buffer(mm(v.GetWidth(pcbnew.F_Cu)) / 2)
            lay = set(LAYERS) | {'1'}
            out.append(('via', lay, {k: g for k in lay}, [(x, y)]))
        else:
            k = next((k for k, L in LAYERS.items() if t.GetLayer() == L), None)
            if k is None:
                continue
            s_, e_ = t.GetStart(), t.GetEnd()
            a = (mm(s_.x) - O[0], mm(s_.y) - O[1]); c = (mm(e_.x) - O[0], mm(e_.y) - O[1])
            line = LineString([a, c])
            g = line.buffer(mm(t.GetWidth()) / 2)
            n = max(1, int(line.length / 0.25))
            pts = [line.interpolate(i / n, normalized=True).coords[0] for i in range(n + 1)]
            out.append(('track', {k}, {k: g}, pts))
    for z in seq(b.Zones()):
        if z.GetNetname() != net:
            continue
        L = z.GetLayer()
        k = '1' if L == pcbnew.In1_Cu else next((k for k, LL in LAYERS.items() if LL == L), None)
        if k is None:
            continue
        ps = z.GetFilledPolysList(L)
        for i in range(ps.OutlineCount()):
            ol = ps.Outline(i)
            pts = [(mm(ol.CPoint(j).x) - O[0], mm(ol.CPoint(j).y) - O[1]) for j in range(ol.PointCount())]
            if len(pts) >= 3:
                out.append(('zone', {k}, {k: Polygon(pts).buffer(0)}, []))
    return out


def islands(items):
    parent = list(range(len(items)))

    def f(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]; i = parent[i]
        return i
    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            common = items[i][1] & items[j][1]
            if any(items[i][2][k].intersects(items[j][2][k]) for k in common):
                parent[f(i)] = f(j)
    groups = {}
    for i in range(len(items)):
        groups.setdefault(f(i), []).append(items[i])
    return list(groups.values())


def open_pairs(b, net):
    """Nearest anchor pairs that join the islands of a net (Prim-like)."""
    its = islands(item_geoms(b, net))
    if len(its) < 2:
        return []
    main = max(its, key=lambda g: sum(1 for it in g if it[0] == 'zone') * 1000 + len(g))
    pairs = []
    for g in its:
        if g is main:
            continue
        has_plane = any(it[0] == 'zone' for it in main)
        best = None
        for it in g:
            for a in it[3]:
                for jt in main:
                    for c in jt[3]:
                        d = math.dist(a, c)
                        if d > 0.05 and (best is None or d < best[0]):
                            best = (d, a, c, it[1], jt[1])
        if best is None:
            continue
        pairs.append((net, best[1], best[2], has_plane, best[3], best[4]))
    return pairs


def plane_via(b, obs, net, A):
    """Connect an isolated pad/track to the In1 GND plane with a short stub + via."""
    for dist in (0.6, 0.8, 1.0, 1.3, 1.7, 2.2, 2.8, 3.5, 4.3):
        for ang in range(0, 360, 10):
            pa = (A[0] + dist * math.cos(math.radians(ang)), A[1] + dist * math.sin(math.radians(ang)))
            vg = Point(pa).buffer(via_d(net) / 2)
            if free(obs, 'F', seg(A, pa, 0.15), net) and all(free(obs, k, vg, net) for k in LAYERS):
                track(b, net, pcbnew.F_Cu, [A, pa], 0.15)
                via(b, net, *pa)
                return 'via'
    return None


def maze(b, obs, net, A, B, margin=8.0, step=0.15, la=None, lb=None):
    """A* on a 3-layer grid (F, In2, B) with vias; returns True if routed."""
    import heapq
    x0 = max(0.6, min(A[0], B[0]) - margin); x1 = min(BW - 0.6, max(A[0], B[0]) + margin)
    y0 = max(0.6, min(A[1], B[1]) - margin); y1 = min(BH - 0.6, max(A[1], B[1]) + margin)
    nx, ny = int((x1 - x0) / step) + 1, int((y1 - y0) / step) + 1
    keys = [k for k in LAYERS if k in ALLOWED]
    rad = W / 2
    cache = {}

    def ok(k, i, j):
        c = (k, i, j)
        if c not in cache:
            cache[c] = free(obs, k, Point(x0 + i * step, y0 + j * step).buffer(rad), net)
        return cache[c]
    vcache = {}

    def via_ok(i, j):
        if (i, j) not in vcache:
            g = Point(x0 + i * step, y0 + j * step).buffer(via_d(net) / 2)
            vcache[(i, j)] = all(free(obs, k, g, net) for k in keys)
        return vcache[(i, j)]
    si, sj = round((A[0] - x0) / step), round((A[1] - y0) / step)
    ti, tj = round((B[0] - x0) / step), round((B[1] - y0) / step)
    la = [k for k in keys if k in (la or {'F'})] or ['F']
    lb = [k for k in keys if k in (lb or {'F'})] or ['F']
    goals = {(k, ti, tj) for k in lb}
    h = lambda n: math.hypot(n[1] - ti, n[2] - tj)
    openq, came, gbest = [], {}, {}
    for k in la:
        st = (k, si, sj)
        openq.append((h(st), 0.0, st)); came[st] = None; gbest[st] = 0.0
    start = openq[0][2]
    moves = [(1, 0, 1), (-1, 0, 1), (0, 1, 1), (0, -1, 1), (1, 1, 1.42), (1, -1, 1.42), (-1, 1, 1.42), (-1, -1, 1.42)]
    found = None
    n_exp = 0
    while openq and n_exp < 1500000:
        _, g, cur = heapq.heappop(openq)
        n_exp += 1
        if cur in goals:
            found = cur
            break
        k, i, j = cur
        nbrs = []
        for di, dj, c in moves:
            ii, jj = i + di, j + dj
            if 0 <= ii < nx and 0 <= jj < ny and (ok(k, ii, jj) or (k, ii, jj) in goals):
                nbrs.append(((k, ii, jj), c))
        if via_ok(i, j):
            for kk in keys:
                if kk != k:
                    nbrs.append(((kk, i, j), 12))
        for nb, c in nbrs:
            ng = g + c
            if ng < gbest.get(nb, 1e18):
                gbest[nb] = ng
                came[nb] = cur
                heapq.heappush(openq, (ng + h(nb), ng, nb))
    if not found:
        return False
    path = []
    n = found
    while n:
        path.append(n); n = came[n]
    path.reverse()
    # emit: merge collinear runs on the same layer, vias at layer changes
    pt = lambda n: (round(x0 + n[1] * step, 4), round(y0 + n[2] * step, 4))
    run = [path[0]]
    segs = []
    for a, c in zip(path, path[1:]):
        if a[0] != c[0]:
            segs.append((a[0], [pt(m) for m in run]))
            via(b, net, *pt(a))
            run = [c]
        else:
            run.append(c)
    segs.append((run[0][0], [pt(m) for m in run]))
    for k, pts in segs:
        simp = [pts[0]]
        for q in range(1, len(pts) - 1):
            ax, ay = simp[-1]; bx, by = pts[q]; cx, cy = pts[q + 1]
            if abs((bx - ax) * (cy - by) - (by - ay) * (cx - bx)) > 1e-6:
                simp.append(pts[q])
        simp.append(pts[-1])
        if pts[0] == pt(path[0]) and path[0][0] == k:
            simp[0] = A
        if pts[-1] == pt(found) and found[0] == k:
            simp[-1] = B
        if len(simp) >= 2:
            track(b, net, LAYERS[k], simp, W)
    return True


def remove_dangling(b):
    """Iteratively delete track segments with a free end and vias touching < 2 items."""
    removed = 0
    for _ in range(10):
        geo = {}
        for net in {t.GetNetname() for t in tracks(b)}:
            geo[net] = item_geoms(b, net)
        kill = []
        for t in tracks(b):
            if t.IsLocked():
                continue
            net = t.GetNetname()
            if t.GetClass() == 'PCB_VIA':
                v = t.Cast()
                p = Point(mm(v.GetPosition().x) - O[0], mm(v.GetPosition().y) - O[1])
                touches = sum(1 for it in geo[net] if it[0] != 'via' and any(
                    g.distance(p) < 0.01 for k, g in it[2].items()))
                if touches < 2 and net != 'GND':
                    kill.append(t)
                continue
            k = next((k for k, L in LAYERS.items() if t.GetLayer() == L), None)
            if k is None:
                continue
            for e in (t.GetStart(), t.GetEnd()):
                p = Point(mm(e.x) - O[0], mm(e.y) - O[1])
                others = [it for it in geo[net] if k in it[2] and not (
                    it[0] == 'track' and it[3] and Point(it[3][0]).distance(p) < 1e-3 and
                    Point(it[3][-1]).distance(Point(mm(t.GetEnd().x) - O[0], mm(t.GetEnd().y) - O[1])) < 1e-3)]
                hit = sum(1 for it in others if it[2][k].distance(p) < 0.01)
                if hit <= 1:      # only itself
                    kill.append(t)
                    break
        if not kill:
            break
        for t in kill:
            b.Remove(t)
        removed += len(kill)
    return removed



def ripup_fix(d, idx):
    """Rip up the idx-th neighbouring two-pin signal net around the first open
    connection recorded in pcb_route/failed.json, connect it, re-route the victim.
    One attempt per process (KiCad's SWIG layer dislikes several boards per process)."""
    import json
    d = Path(d)
    pcb = d / 'DDS_47_51.kicad_pcb'
    fails = json.loads((d / 'pcb_route' / 'failed.json').read_text())
    if not fails:
        return None
    net, A, B, plane, la, lb = fails[0]
    A, B, la, lb = tuple(A), tuple(B), set(la), set(lb)
    sm = pcbnew.GetSettingsManager()
    sm.LoadProject(str(d / 'DDS_47_51.kicad_pro'))
    b = pcbnew.LoadBoard(str(pcb))
    proj = Proj(d)
    apply_rules(proj, b)
    from dds_pcb import class_nets
    rf, clk, pwr = class_nets(proj)
    for n in rf | clk:
        CLR_NET[n] = 0.205
    pa = Point(A)
    cand = {}
    all_t = tracks(b)
    for t in all_t:
        n = t.GetNetname()
        if n == net or n == 'GND' or n in CLR_NET or t.IsLocked():
            continue
        if t.GetClass() == 'PCB_VIA':
            dist = Point(mm(t.GetPosition().x) - O[0], mm(t.GetPosition().y) - O[1]).distance(pa)
        else:
            s_, e_ = t.GetStart(), t.GetEnd()
            dist = LineString([(mm(s_.x) - O[0], mm(s_.y) - O[1]),
                               (mm(e_.x) - O[0], mm(e_.y) - O[1])]).distance(pa)
        if dist < 2.0:
            cand[n] = min(cand.get(n, 9), dist)
    order = sorted(cand, key=cand.get)
    print('candidates', order)
    # idx < len(order): one victim; idx == len(order): all of them at once
    if idx > len(order):
        return None
    victims = order if idx == len(order) else [order[idx]]
    region = pa.buffer(2.5)
    for t in all_t:
        if t.GetNetname() not in victims or t.IsLocked():
            continue
        if t.GetClass() == 'PCB_VIA':
            g = Point(mm(t.GetPosition().x) - O[0], mm(t.GetPosition().y) - O[1])
        else:
            s_, e_ = t.GetStart(), t.GetEnd()
            g = LineString([(mm(s_.x) - O[0], mm(s_.y) - O[1]), (mm(e_.x) - O[0], mm(e_.y) - O[1])])
        if g.intersects(region):
            b.Remove(t)
    obs = obstacles(b)
    r = plane_via(b, obs, net, A) if (net == 'GND' and plane) else None
    r = r or maze(b, obs, net, A, B, la=la, lb=lb)
    if not r:
        return False
    for victim in victims:
        for _ in range(4):
            prs = open_pairs(b, victim)
            if not prs:
                break
            for _n, A2, B2, pl2, la2, lb2 in prs:
                obs = obstacles(b)
                rr = try_pair(b, obs, victim, A2, B2) if ('F' in la2 and 'F' in lb2) else None
                rr = rr or maze(b, obs, victim, A2, B2, la=la2, lb=lb2) or \
                    maze(b, obs, victim, A2, B2, margin=12.0, step=0.1, la=la2, lb=lb2)
                if not rr:
                    print('  could not re-route', victim)
                    return False
        if open_pairs(b, victim):
            return False
    if open_pairs(b, net):
        return False
    victim = '+'.join(victims)
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())
    pcbnew.SaveBoard(str(pcb), b)
    (d / 'pcb_route' / 'failed.json').write_text(json.dumps(fails[1:]))
    print(f'rip-up of {victim} let {net} connect')
    return True

def main(d, nets=None):
    d = Path(d)
    pcb = d / 'DDS_47_51.kicad_pcb'
    sm = pcbnew.GetSettingsManager()
    sm.LoadProject(str(d / 'DDS_47_51.kicad_pro'))
    b = pcbnew.LoadBoard(str(pcb))
    proj = Proj(d)
    apply_rules(proj, b)
    from dds_pcb import class_nets
    rf, clk, pwr = class_nets(proj)
    for n in rf | clk:
        CLR_NET[n] = 0.205
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())
    names = nets or sorted({n for n in proj.nets if not n.startswith('unconnected')})
    done = fail = 0
    global W, ALLOWED
    for net in names:
        if net in clk:                   # 480 MHz: F.Cu microstrip only
            W, ALLOWED = 0.25, {'F'}
        elif net in rf:                  # 47-51 MHz output: any layer
            W, ALLOWED = 0.3, set(LAYERS)
        else:
            W, ALLOWED = 0.2, set(LAYERS)
        for _ in range(4):
            pairs = open_pairs(b, net)
            if not pairs:
                break
            progress = False
            for net_, A, B, plane, la, lb in pairs:
                obs = obstacles(b)
                both_f = 'F' in la and 'F' in lb
                r = plane_via(b, obs, net, A) if (net == 'GND' and plane) else None
                if not r and both_f:
                    r = try_pair(b, obs, net, A, B)
                if not r:
                    r = 'maze' if maze(b, obs, net, A, B, la=la, lb=lb) else None
                if not r:
                    r = 'maze-fine' if maze(b, obs, net, A, B, margin=12.0, step=0.1, la=la, lb=lb) else None
                print(f'{net:28s} ({A[0]:.2f},{A[1]:.2f}) -> ({B[0]:.2f},{B[1]:.2f}): {r or "FAILED"}')
                if not r:
                    FAILED.append((net, A, B, plane, la, lb))
                done += bool(r); fail += not r
                progress |= bool(r)
            if not progress:
                break
            if net == 'GND':
                pcbnew.ZONE_FILLER(b).Fill(b.Zones())
    fix_vias(b)
    for t in tracks(b):
        if t.GetClass() == 'PCB_TRACK' and mm(t.GetWidth()) < 0.1:
            t.SetWidth(MM(0.1))
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())
    pcbnew.SaveBoard(str(pcb), b)
    import json
    (d / 'pcb_route' / 'failed.json').write_text(json.dumps(
        [(n, A, B, pl, sorted(la), sorted(lb)) for n, A, B, pl, la, lb in FAILED]))
    print(f'completed {done}, failed {fail}')


def fix_vias(b):
    """Keep the annular ring >= 0.125 mm on every via."""
    for t in tracks(b):
        if t.GetClass() == 'PCB_VIA':
            v = t.Cast()
            dr = mm(v.GetDrillValue())
            w = mm(v.GetWidth(pcbnew.F_Cu))
            if w - dr < 0.25 - 1e-6:
                if w <= 0.5:
                    v.SetDrill(MM(max(0.2, w - 0.25)))
                else:
                    v.SetWidth(MM(dr + 0.25))


def cleanup(d):
    d = Path(d)
    pcb = d / 'DDS_47_51.kicad_pcb'
    sm = pcbnew.GetSettingsManager()
    sm.LoadProject(str(d / 'DDS_47_51.kicad_pro'))
    b = pcbnew.LoadBoard(str(pcb))
    apply_rules(Proj(d), b)
    print('dangling removed', remove_dangling(b))
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())
    pcbnew.SaveBoard(str(pcb), b)


if __name__ == '__main__':
    if sys.argv[1] == '--cleanup':
        cleanup(sys.argv[2])
    elif sys.argv[1] == '--ripup':
        r = ripup_fix(sys.argv[2], int(sys.argv[3]))
        sys.exit(0 if r else (2 if r is None else 1))
    else:
        main(sys.argv[1], sys.argv[2:] or None)
