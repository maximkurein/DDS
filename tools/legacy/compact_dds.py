from pathlib import Path
import re, json, uuid, shutil, subprocess, xml.etree.ElementTree as ET
SRC=Path(r'C:\Project\DDS')
OUT=Path('DDS_47_51_revC'); OUT.mkdir(exist_ok=True)
def parse(s):
 ts=iter(re.findall(r'"(?:\\.|[^"\\])*"|[^\s()]+|[()]',s))
 def read():
  a=[]
  for t in ts:
   if t==')':return a
   a.append(read() if t=='(' else t)
  return a
 return read()[0]
def dump(a):return '('+' '.join(dump(v) if isinstance(v,list) else str(v) for v in a)+')'
def ch(a,k):return next((v for v in a if isinstance(v,list) and v[0]==k),None)
def q(s):return json.dumps(str(s),ensure_ascii=False)
def uid():return str(uuid.uuid4())
def move(a,dx,dy):
 if a[0] in ('at','xy'):
  a[1]=str(round(float(a[1])+dx,4));a[2]=str(round(float(a[2])+dy,4))
 for v in a[1:]:
  if isinstance(v,list):move(v,dx,dy)
def ref(s):return next(json.loads(v[2]) for v in s if isinstance(v,list) and v[:2]==['property','"Reference"'])
root=uid(); ids=[root,uid(),uid()]
names=['DDS_47_51','02_controller','03_power']
titles=['DDS, clock and RF output','Controller, frequency selector and SPI','Power supply / +15 V input']
objs=[[],[],[]]; libs=[{}, {}, {}]; notes=[]; mapping={}
def load(n,page,position=None,offset=None):
 a=parse((SRC/(n+'.kicad_sch')).read_text(encoding='utf-8'))
 for v in ch(a,'lib_symbols')[1:]:libs[page][v[1]]=v
 groups=[];current=None
 for v in a[1:]:
  if not isinstance(v,list):continue
  if v[0]=='text':notes.append((n,json.loads(v[1])));continue
  if v[0]=='symbol':
   current=[v];groups.append(current)
  elif v[0] in ('wire','global_label','junction','no_connect','label'):
   if current is None:groups.append([v])
   else:current.append(v)
 for i,g in enumerate(groups):
  sym=g[0] if g[0][0]=='symbol' else None
  if offset:
   dx,dy=offset
   if n=='09_rf' and sym is not None:
    if i<4:
     x,y=grid(i,230,165,4,65,20); at=ch(sym,'at');dx=x-float(at[1]);dy=y-float(at[2])
    elif ref(sym)=='J3':
     at=ch(sym,'at');dx=480-float(at[1]);dy=135-float(at[2])
  else:
   at=ch(sym,'at');x,y=position(i,ref(sym));dx=x-float(at[1]);dy=y-float(at[2])
  for v in g:
   if n=='09_rf' and v[0]=='global_label' and v[1]=='"OUTP1"' and float(ch(v,'at')[1])==20:move(v,*offset)
   else:move(v,dx,dy)
  if sym:
   r=ref(sym);mapping[r]=names[page]
   sym[:]=[v for v in sym if not (isinstance(v,list) and v[0]=='instances')]
   path='/'+root+('' if page==0 else '/'+ids[page])
   sym.append(['instances',['project','"DDS_47_51"',['path',q(path),['reference',q(r)],['unit','1']]]])
  objs[page].extend(g)
def grid(i,x,y,cols=6,sx=52,sy=20):return x+i%cols*sx,y+i//cols*sy
load('02_dds',0,lambda i,r:(100,100) if i==0 else grid(i-1,40,185,3,55,22))
load('03_bias',0,lambda i,r:grid(i,35,265,10,55,20) if i<28 else grid(i-28,255,238,6,55,20))
load('04_bypass',0,lambda i,r:grid(i,35,340,10,55,20))
clock=[(260,42),(350,35),(410,35),(470,35),(530,35),(350,65),(410,65),(260,100),(350,100),(440,100),(530,100),(260,140),(350,140)]
load('08_clock',0,lambda i,r:clock[i])
load('09_rf',0,offset=(165,75))
# Move the four DAC loads and output connector separately, leaving the wired filter intact.
for v in objs[0]:
 pass
load('05_mcu',1,lambda i,r:(100,100) if i==0 else ((70,205) if i==1 else grid(i-2,35,175,4,58,65)))
load('06_ui',1,lambda i,r:grid(i//2,300,40,4,70,52) if i<24 and i%2==0 else (grid(i//2,300,60,4,70,52) if i<24 else [(310,215),(415,215),(520,215)][i-24]))
load('07_spi',1,lambda i,r:((90,295) if i==0 else (230,295)) if i<2 else grid(i-2,330,275,5,52,26))
load('01_power',2,offset=(0,0))
def text(page,s,x,y,size=1.5):objs[page].append(['text',q(s),['at',str(x),str(y),'0'],['effects',['font',['size',str(size),str(size)]],['justify','left','top']],['uuid',uid()]])
text(0,'01  DDS / 47-51 MHz / 1 kHz / -10 dBm',20,12,2)
text(0,'480 MHz clock / G1 unverified, DNP',230,16)
text(0,'Unused input pull-downs',20,246)
text(0,'Local bypass: place at the indicated U1 pins',20,322)
text(0,'RF: DAC loads / 7-pole LPF / pi attenuator',210,153)
text(1,'02  CONTROLLER / 1887VE7T / +5 V',20,12,2)
text(1,'12-bit selector / ON = 1 / N = 0..4000',280,15)
text(1,'SPI level translation / start at 100 kHz',20,255)
text(1,'f = 47,000,000 + 1,000*N Hz. Reject N > 4000. Debounce APPLY.\nInternal RC 8 MHz; ISP uses PE0/PE1. See design_notes.md for startup sequence.',20,375)
text(2,'03  POWER / preliminary circuit',20,12,2)
text(2,'Input +15 V +/-5%; VIN <17 V. Effective capacitance at DC bias.\nPG U4/U5 wired-AND. Verify oscillator current, thermal and noise performance.',20,265)
for page in (1,2):
 x=20+(page-1)*95;y=395
 objs[0].append(['sheet',['at',str(x),str(y)],['size','85','10'],['stroke',['width','0.254'],['type','default']],['fill',['color','0','0','0','0']],['uuid',ids[page]],['property','"Sheetname"',q(titles[page].split(',')[0]),['at',str(x),str(y-2),'0'],['effects',['font',['size','1','1']]]],['property','"Sheetfile"',q(names[page]+'.kicad_sch'),['at',str(x),str(y+11),'0'],['effects',['font',['size','1','1']],['hide','yes']]],['instances',['project','"DDS_47_51"',['path',q('/'+root),['page',q(page+1)]]]]])
for p in range(3):
 a=['kicad_sch',['version','20250114'],['generator','"eeschema"'],['uuid',ids[p]],['paper',q('A3' if p==2 else 'A2')],['title_block',['title',q(titles[p])],['date','"2026-10-30"'],['rev','"C / layout only / DRAFT"']],['lib_symbols',*libs[p].values()],*objs[p]]
 if p==0:a.append(['sheet_instances',['path','"/"',['page','"1"']]])
 (OUT/(names[p]+'.kicad_sch')).write_text(dump(a),encoding='utf-8')
shutil.copy2(SRC/'DDS_47_51.kicad_pro',OUT/'DDS_47_51.kicad_pro')
shutil.copy2(SRC/'expected_connections.json',OUT/'expected_connections.json')
import csv
with (SRC/'BOM.csv').open(encoding='utf-8-sig',newline='') as f:rows=list(csv.DictReader(f))
for row in rows:row['Sheet']=mapping[row['Reference']]
with (OUT/'BOM.csv').open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=rows[0]);w.writeheader();w.writerows(rows)
(OUT/'design_notes.md').write_text('# Rev B design notes retained in Rev C\n\n'+'\n\n'.join('## '+n+'\n'+t for n,t in notes),encoding='utf-8')
print('Generated',len(mapping),'components in exactly 3 sheets')

