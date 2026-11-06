"""Rebuild generated Visio primitives through Office COM to avoid XML import defects.

The result contains native editable Visio lines and text, not page screenshots.
Usage: python visio_native.py source.vsdx destination.vsdx destination.pdf
"""
import sys
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path
import win32com.client
import pythoncom

NS={'v':'http://schemas.microsoft.com/office/visio/2012/main'}


def cells(n):return {c.get('N'):c.get('V') for c in n.findall('v:Cell',NS)}


def main(source,dest,pdf):
    z=zipfile.ZipFile(source)
    pages=ET.fromstring(z.read('visio/pages/pages.xml')).findall('v:Page',NS)
    app=win32com.client.DispatchEx('Visio.Application');app.Visible=False
    app.ScreenUpdating=False
    doc=app.Documents.Add('')
    try:
        for index,p in enumerate(pages):
            page=doc.Pages.Item(1) if index==0 else doc.Pages.Add()
            page.Name=p.get('Name')
            cs=cells(p.find('v:PageSheet',NS))
            for k in ('PageWidth','PageHeight'):page.PageSheet.CellsU(k).ResultIU=float(cs[k])
            root=ET.fromstring(z.read(f'visio/pages/page{index+1}.xml'))
            count=0
            def draw(shapes,ox=0,oy=0):
                nonlocal count
                for n in shapes.findall('v:Shape',NS):
                    c=cells(n);px=float(c.get('PinX',0));py=float(c.get('PinY',0))
                    lx=float(c.get('LocPinX',0));ly=float(c.get('LocPinY',0))
                    x,y=ox+px-lx,oy+py-ly
                    if n.get('Type')=='Group':
                        draw(n.find('v:Shapes',NS),x,y);continue
                    for sec in n.findall('v:Section',NS):
                        if sec.get('N')!='Geometry':continue
                        pts=[]
                        for row in sec.findall('v:Row',NS):
                            r=cells(row)
                            if row.get('T') in ('MoveTo','LineTo'):pts += [x+float(r['X']),y+float(r['Y'])]
                            elif row.get('T')=='Ellipse':
                                cx,cy=x+float(r['X']),y+float(r['Y'])
                                rx=abs(float(r['A'])-float(r['X']));ry=abs(float(r['D'])-float(r['Y']))
                                s=page.DrawOval(cx-rx,cy-ry,cx+rx,cy+ry)
                                s.CellsU('FillPattern').FormulaU='0' if cells(sec).get('NoFill')=='1' else '1'
                                s.CellsU('FillForegnd').FormulaU='RGB(0,0,0)'
                                s.CellsU('LineWeight').ResultIU=float(c.get('LineWeight',.01))
                        if pts:
                            s=page.DrawPolyline(tuple(pts),0)
                            s.CellsU('LineWeight').ResultIU=float(c.get('LineWeight',.01))
                            s.CellsU('FillPattern').FormulaU='0' if cells(sec).get('NoFill')=='1' else '1'
                            s.CellsU('FillForegnd').FormulaU='RGB(0,0,0)'
                    tn=n.find('v:Text',NS)
                    if tn is not None:
                        w,h=float(c['Width']),float(c['Height'])
                        s=page.DrawRectangle(x,y,x+w,y+h);s.Text=''.join(tn.itertext())
                        for k in ('Width','Height','LocPinX','LocPinY','Angle'):
                            if k in c:s.CellsU(k).ResultIU=float(c[k])
                        s.CellsU('PinX').ResultIU=ox+px;s.CellsU('PinY').ResultIU=oy+py
                        for k in ('LeftMargin','RightMargin','TopMargin','BottomMargin','LinePattern','FillPattern'):s.CellsU(k).FormulaU='0'
                        s.CellsU('VerticalAlign').FormulaU=c.get('VerticalAlign','1')
                        s.CellsU('Char.Font').FormulaU='FONT("Arial")'
                        s.CellsU('Char.Color').FormulaU='RGB(0,0,0)'
                        for sec in n.findall('v:Section',NS):
                            rows=sec.findall('v:Row',NS)
                            if not rows:continue
                            r=cells(rows[0])
                            if sec.get('N')=='Character':
                                s.CellsU('Char.Size').ResultIU=float(r['Size'])
                                s.CellsU('Char.Style').FormulaU=r.get('Style','0')
                            elif sec.get('N')=='Paragraph':s.CellsU('Para.HorzAlign').FormulaU=r.get('HorzAlign','1')
                    count+=1
            draw(root.find('v:Shapes',NS))
            print('page',index+1,'native objects',count,flush=True)
        doc.SaveAs(str(Path(dest).resolve()))
        doc.ExportAsFixedFormat(1,str(Path(pdf).resolve()),1,0)
        doc.Close()
    finally:
        app.Quit()


if __name__=='__main__':main(*sys.argv[1:4])
