"""Local E-to-F placement update. Run using KiCad Python; then finish open nets."""
import json
from pathlib import Path
import pcbnew
from dds_pcb import Proj, load_fp, seq, tracks, local, V, MM
from minirouter import obstacles, free, seg, via, track, LAYERS
from shapely.geometry import Point, LineString, box


def main():
    pr = Proj('.')
    b = pcbnew.LoadBoard('DDS_48_52.kicad_pcb')
    fps = {f.GetReference(): f for f in seq(b.Footprints())}
    assert 'X3' not in fps
    for n in pr.nets:
        if not b.FindNet(n):
            b.Add(pcbnew.NETINFO_ITEM(b, n))
    changed = {'X1', 'X3', 'TP1', 'TP2', 'L8'}
    for ref, xy in [('X1', (31, 3.5)), ('X3', (47, 3.35)), ('TP1', (47.2, 29.5)), ('TP2', (49.5, 29.5))]:
        c = pr.comps[ref]
        fp = load_fp(pr, c['fp'])
        if ref in fps:
            fp.SetUuid(fps[ref].m_Uuid)
            b.Remove(fps[ref])
        fp.SetReference(ref); fp.SetValue(c['value'])
        fp.SetPath(pcbnew.KIID_PATH(c['path'])); fp.SetSheetname(c['sheet'])
        fp.SetPosition(V(*xy))
        fp.SetExcludedFromBOM('exclude_from_bom' in c['props'])
        fp.SetExcludedFromPosFiles('exclude_from_bom' in c['props'])
        fp.Value().SetVisible(False)
        fp.Reference().SetPosition(V(xy[0], xy[1] + (4.0 if ref == 'X3' else -1.5)))
        fp.Reference().SetTextSize(pcbnew.VECTOR2I(MM(.8), MM(.8)))
        for k, v in c['fields'].items():
            if k not in ('Footprint', 'Datasheet', 'Description'):
                fp.SetField(k, v)
        for f in fp.GetFields():
            if f.GetName() not in ('Reference', 'Value'):
                f.SetVisible(False)
        for p in seq(fp.Pads()):
            p.SetNet(b.FindNet(pr.pin_net[(ref, p.GetNumber())]))
        b.Add(fp); fps[ref] = fp
    for p in seq(fps['D2'].Pads()):
        p.SetNet(b.FindNet(pr.pin_net[('D2', p.GetNumber())]))
    f = fps['L8']; c = pr.comps['L8']
    f.SetFPID(pcbnew.LIB_ID(*c['fp'].split(':')))
    f.SetField('Примечание', c['fields']['Примечание'])
    # Remove only segments physically crossed by new pads. Save the exact list.
    removed, nets = [], set()
    for t in tracks(b):
        if t.GetClass() != 'PCB_TRACK':
            continue
        g = LineString([local(t.GetStart()), local(t.GetEnd())]).buffer(pcbnew.ToMM(t.GetWidth()) / 2 + .21)
        for ref in ('X3', 'TP1', 'TP2'):
            for p in seq(fps[ref].Pads()):
                if not p.IsOnLayer(t.GetLayer()) or p.GetNetname() == t.GetNetname():
                    continue
                x,y=local(p.GetPosition())
                pg = box(x-.5,y-.5,x+.5,y+.5) if ref=='X3' and p.GetNumber()=='1' else Point(x,y).buffer(.5 if ref=='X3' else .75)
                if g.intersects(pg):
                    removed.append({'uuid': t.m_Uuid.AsString(), 'net': t.GetNetname(), 'a':local(t.GetStart()),'b':local(t.GetEnd())})
                    nets.add(t.GetNetname()); b.Remove(t); break
            else:
                continue
            break
    # X1 formerly carried inner-layer connections through header holes.
    # Replace each such hole by a separate via outside the exposed contact.
    for p in seq(fps['X1'].Pads()):
        n=p.GetNetname(); a=local(p.GetPosition()); obs=obstacles(b)
        candidates=[(a[0]+dx,a[1]+dy) for dx,dy in [(-1.2,0),(1.2,0),(0,-1.2),(0,1.2)]]
        for v in candidates:
            vg=Point(v).buffer(.3)
            if all(free(obs,L,vg,n) for L in LAYERS) and free(obs,'F',seg(a,v,.2),n) and free(obs,'I',seg(a,v,.2),n):
                via(b,n,*v);track(b,n,pcbnew.F_Cu,[a,v],.2)
                if n!='GND':track(b,n,pcbnew.In2_Cu,[a,v],.2)
                break
        else:
            print('X1 via deferred',p.GetNumber())
        nets.add(n)
    for ref in ('X3','TP1','TP2'):
        nets.update(p.GetNetname() for p in seq(fps[ref].Pads()) if not p.GetNetname().startswith('unconnected'))
    tb=b.GetTitleBlock();tb.SetRevision('F');tb.SetDate('2026-11-06');b.SetTitleBlock(tb)
    for d in seq(b.Drawings()):
        if d.GetClass()=='PCB_TEXT':
            d.SetText(d.GetText().replace('rev E','rev F').replace('Rev E','Rev F'))
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())
    pcbnew.SaveBoard('DDS_48_52.kicad_pcb',b)
    Path('pcb_route/revF_work/local_routing.json').write_text(json.dumps({'removed':removed,'nets':sorted(nets)},indent=2),encoding='utf-8')
    print('Removed local segments:',len(removed),'nets to finish:',sorted(nets))


if __name__=='__main__':
    main()
