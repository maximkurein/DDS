from pathlib import Path
import uuid,json,math,csv
O=Path('DDS_47_51_revB');O.mkdir(exist_ok=True)
u=lambda:str(uuid.uuid4())
q=lambda x:json.dumps(str(x),ensure_ascii=False)
root=u();lib={};geo={};sheets=[];bom=[];exp={};counts={}
def prop(k,v,x,y,hide=False):
 return f'(property {q(k)} {q(v)} (at {x} {y} 0) (effects (font (size 1 1))'+(' (hide yes)' if hide else '')+'))'
def block(n,names,types={}):
 h=max(8,math.ceil(len(names)/2)*1.27+2.54);geo[n]={}
 t=f'(symbol "Local:{n}" (pin_names (offset 1)) (in_bom yes) (on_board yes)'+prop('Reference','U',0,h+5)+prop('Value',n,0,h+2)
 t+=f'(symbol "{n}_0_1" (rectangle (start -18 {h}) (end 18 {-h}) (stroke (width 0.254) (type default)) (fill (type background))))(symbol "{n}_1_1" '
 for i,label in enumerate(names):
  side=i<math.ceil(len(names)/2);x=-23.08 if side else 23.08;y=((math.ceil(len(names)/2)-1)/2-i%math.ceil(len(names)/2))*2.54;geo[n][str(i+1)]=(x,-y)
  t+=f'(pin {types.get(i+1,"passive")} line (at {x} {y} {0 if side else 180}) (length 5.08) (name {q(label)} (effects (font (size 1 1)))) (number "{i+1}" (effects (font (size 1 1)))))'
 lib[n]=t+'))'
def native(n,file='Device',points=None):
 src=Path(r'C:\Program Files\KiCad\10.99\share\kicad\symbols',file+'.kicad_sym').read_text(encoding='utf-8');a=src.index('(symbol "'+n+'"');d=0;quoted=False;esc=False
 for b in range(a,len(src)):
  c=src[b]
  if esc:esc=False;continue
  if c=='\\' and quoted:esc=True;continue
  if c=='"':quoted=not quoted
  if not quoted:
   d+=(c=='(')-(c==')')
   if d==0:break
 lib[n]=src[a:b+1].replace('(symbol "'+n+'"','(symbol "Local:'+n+'"',1)
 geo[n]=points or {'1':(0,-3.81),'2':(0,3.81)}
for n in ['R','C','L']:native(n)
native('SW_SPST','Switch',{'1':(-5.08,0),'2':(5.08,0)})
native('D_Schottky','Device',{'1':(-3.81,0),'2':(3.81,0)})
native('Fuse','Device',{'1':(0,-3.81),'2':(0,3.81)})
block('J2',['1','2']);block('J6',['1','2','3','4','5','6'])
def new(n,title):
 s={'n':n,'title':title,'id':u(),'obj':[],'used':set()};sheets.append(s);return s
def note(s,t,x=20,y=255,size=1.3):
 s['obj'].append(f'(text {q(t)} (at {x} {y} 0) (effects (font (size {size} {size})) (justify left top)) (uuid {u()}))')
def wire(s,a,b):
 s['obj'].append(f'(wire (pts (xy {a[0]:.4f} {a[1]:.4f}) (xy {b[0]:.4f} {b[1]:.4f})) (stroke (width 0) (type default)) (uuid {u()}))')
def label(s,p,n):
 s['obj'].append(f'(global_label {q(n)} (shape passive) (at {p[0]:.4f} {p[1]:.4f} 0) (effects (font (size 0.9 0.9)) (justify left)) (uuid {u()})'+prop('Intersheetrefs','${INTERSHEET_REFS}',p[0],p[1],True)+')')
def part(s,n,ref,val,x,y,nets,angle=0,dnp=False):
 s['used'].add(n);h=max(abs(p[1]) for p in geo[n].values());px=x+9 if n in ['R','C','L','Fuse'] and angle==0 else x;py=y-1 if n in ['R','C','L','Fuse'] and angle==0 else y-(4 if angle else h+6)
 t=f'(symbol (lib_id "Local:{n}") (at {x} {y} {angle}) (unit 1) (in_bom yes) (on_board yes) (dnp {"yes" if dnp else "no"}) (uuid {u()})'+prop('Reference',ref,px,py)+prop('Value',val,px,py+2)
 for pin in geo[n]:t+=f'(pin "{pin}" (uuid {u()}))'
 t+=f'(instances (project "DDS_47_51" (path "/{root}/{s["id"]}" (reference "{ref}") (unit 1)))))';s['obj'].append(t)
 points={}
 for i,(pin,(dx,dy)) in enumerate(geo[n].items()):
  if angle==90:dx,dy=dy,-dx
  p=(round(x+dx,4),round(y+dy,4));points[pin]=p;net=nets[i];exp[ref+'.'+pin]=net
  if net is None:s['obj'].append(f'(no_connect (at {p[0]} {p[1]}) (uuid {u()}))')
  elif net!='@':
   e=(round(p[0]+(-15.24 if dx<=0 else 15.24),4),p[1]);wire(s,p,e);label(s,e,net)
 bom.append({'Reference':ref,'Value':val,'DNP':dnp,'Sheet':s['n']})
 return points
def pas(s,k,v,x,y,a,b,ref=None,dnp=False):
 if ref is None:counts[k]=counts.get(k,0)+1;ref=k+str(counts[k])
 return part(s,k,ref,v,x,y,[a,b],dnp=dnp)
def grid(s,items,y=30):
 for i,(k,v,a,b) in enumerate(items):pas(s,k,v,45+i%6*63.5,y+i//6*25.4,a,b,dnp='DNP' in v)
block('TPS6216x',['PGND','VIN','EN','AGND','FB','VOS','SW','PG','EP'],{1:'power_in',2:'power_in',3:'input',4:'power_in',5:'input',6:'input',7:'output',8:'open_collector',9:'power_in'})
s=new('01_power','Power / regulated 15 V input')
part(s,'J2','J1','15V INPUT',60,35,['VIN15','GND']);part(s,'Fuse','F1','0.5A',140,35,['VIN15','FUSED15']);part(s,'D_Schottky','D1','SS34',225,35,['VIN_PROT','FUSED15'])
note(s,'Input +15 V +/-5%; VIN must stay <17 V.\nD1 blocks reverse polarity.',270,25,1.2)
for i,(ref,val,vin,vout,pg) in enumerate([('U3','TPS62163DSGR','VIN_PROT','+5V','MCU_RESETn'),('U4','TPS62162DSGR','+5V','+3V3','DDS_PGOOD'),('U5','TPS62161DSGR','+5V','+1V8','DDS_PGOOD')]):
 y=90+i*60;part(s,'TPS6216x',ref,val,70,y,['GND',vin,vin,'GND','GND',vout,ref+'_SW',pg,'GND'])
 pas(s,'C','10u / 25V',145,y,vin,'GND');pas(s,'L','2.2u / Isat>=1.5A',215,y,ref+'_SW',vout);pas(s,'C','22u / 10V',290,y,vout,'GND');pas(s,'C','100n',355,y,vin,'GND')
pas(s,'R','47k',335,245,'+5V','DDS_PGOOD')
note(s,'Capacitance specified EFFECTIVE at DC bias; X7R. EP=GND. Keep SW loops short.\nU4/U5 PG wired-AND. MCU checks rails after startup delay; PG is Hi-Z when disabled.\nBudget: 1.8 V <=0.4 A DDS; 3.3 V DDS <=0.2 A plus oscillator. Verify G1 current.',20,255,1.2)
names='CVDD SCK SDI CGND SDO SCSn SSCSn CSn PVDD RSTn ADR PGND CSEL CVDD CVDD DVDD AGND CLKDP CLKDM AGND AVDD AGND CLKM CLKP AGND AVDD AVDD IREF2 AGND AVDD2 AVDD2 AGND OUTM2 OUTP2 AGND DVDD2 IREF1 AGND AVDD1 AVDD1 AGND OUTM1 OUTP1 AGND DVDD1 AGND DVDD CMP_GND CMP_INM CMP_INP CMP_VDD CMP_OP CMP_OM CMP_GND CGND SEL2_0 SEL2_1 SEL2_2 SEL2_3 CVDD CVDD SEL2_4 SEL2_5 CGND CGND PGND SEL1_0 SEL1_1 PVDD SEL1_2 SEL1_3 CGND SEL1_4 SEL1_5 CVDD RDn WRn DATA_0 DATA_1 DATA_2 DATA_3 CVDD DATA_4 DATA_5 CGND CGND DATA_6 DATA_7 DATA_8 DATA_9 PGND DATA_10 DATA_11 PVDD DATA_12 DATA_13 DATA_14 DATA_15 CGND CSYNC'.split()
types={i+1:('power_in' if 'VDD' in n or 'GND' in n else 'output' if n in ['SDO','SSCSn','OUTM1','OUTP1','OUTM2','OUTP2','CMP_OP','CMP_OM'] else 'bidirectional' if n.startswith(('DATA','SEL')) or n=='CSYNC' else 'input') for i,n in enumerate(names)}
block('1508PL8T',names,types)
rails={'CVDD':'+1V8','DVDD':'+1V8_Q','DVDD1':'+1V8_Q','DVDD2':'+1V8_Q','PVDD':'+3V3','AVDD':'+3V3_A','AVDD1':'+3V3_A','AVDD2':'+3V3_A','CMP_VDD':'+3V3_A'}
nets=[]
for n in names:
 nn='GND' if 'GND' in n else rails.get(n,n)
 if n in ['SSCSn','CMP_OP','CMP_OM','CSYNC']:nn=None
 if n in ['CMP_INM','CMP_INP']:nn='GND'
 if n in ['SCK','SDI','SCSn','RSTn','SDO']:nn='DDS_'+n
 nets.append(nn)
s=new('02_dds','DDS / 1508PL8T / all 100 physical pins')
part(s,'1508PL8T','U1','1508PL8T',195,115,nets)
note(s,'ELVEES / 14.10.2020\nTable 4.2 pin numbering\nCSEL=0: G1 / CLKP,M\nCSEL=1: external input\n480 MHz / CTR.cmx=0',20,25,1.3)
note(s,'Firmware initialization:\nDAC2 and comparator OFF\nSYNC outputs OFF\nLINK OFF / profile 0\nUnused IO pulled down',285,25,1.3)
grid(s,[('R','7.15k / 0.1%','IREF1','GND'),('R','7.15k / 0.1%','IREF2','GND'),('R','10k','+3V3','CSn'),('R','10k','+3V3','RDn'),('R','10k','+3V3','WRn'),('R','100k','ADR','GND')],220)
note(s,'Continuous common ground plane. RREF: nominal IFS=10.07 mA. OUT voltage limit +/-0.6 V.')
s=new('03_bias','DDS / unused inputs and supply filtering')
grid(s,[('R','100k',n,'GND') for n in names if n.startswith(('SEL','DATA'))])
for j,(a,b) in enumerate([('+3V3','+3V3_A'),('+3V3','+3V3_OSC'),('+1V8','+1V8_Q')]):
 pas(s,'L','600R@100MHz / 1A',55+j*125,185,a,b,ref='FB'+str(j+1));pas(s,'C','10u / 10V',55+j*125,215,b,'GND')
note(s,'FB1..3 are ferrite beads, DCR <=0.1 ohm. Check LC resonance on prototype.\nUnused bidirectional IO use resistive pull-downs, never direct ground shorts.')
s=new('04_bypass','DDS / capacitor at every supply pin')
grid(s,[('C','100n / U1.'+str(i+1),rails[n],'GND') for i,n in enumerate(names) if n in rails]+[('C','10u / 10V',r,'GND') for r in ['+1V8','+3V3','+3V3_A','+1V8_Q']],35)
note(s,'Place each capacitor at the indicated U1 supply pin with its own adjacent ground via.\nBulk capacitors supplement local bypass. Do not place all capacitors in a remote bank.',20,230)
mn=['PENn']+[f'PE{i}' for i in range(8)]+[f'PB{i}' for i in range(8)]+['PG3','PG4','RESETn','VCC','GND','BQ2','BQ1']+[f'PD{i}' for i in range(8)]+['PG0','PG1']+[f'PC{i}' for i in range(8)]+['PG2']+[f'PA{i}' for i in range(7,-1,-1)]+['VCC','GND']+[f'PF{i}' for i in range(7,-1,-1)]+['AREF','AGND','AVCC']
assert len(mn)==64
block('1887VE7T',mn,{i+1:('power_in' if n in ['VCC','AVCC','GND','AGND'] else 'input' if n in ['PENn','RESETn','BQ1'] else 'output' if n=='BQ2' else 'bidirectional') for i,n in enumerate(mn)})
mapping={'PENn':'PENn','RESETn':'MCU_RESETn','VCC':'+5V','AVCC':'+5V','GND':'GND','AGND':'GND','BQ1':None,'BQ2':None,'AREF':'AREF','PE0':'ISP_PDI','PE1':'ISP_PDO','PB0':'MCU_CS','PB1':'MCU_SCK','PB2':'MCU_MOSI','PB3':'MCU_MISO','PB4':'MCU_DDS_RESET','PB5':'BUF_OEn','PC4':'DDS_PGOOD','PC5':'APPLYn'}
mapping.update({f'PA{i}':f'DIP{i}' for i in range(8)});mapping.update({f'PC{i}':f'DIP{i+8}' for i in range(4)})
s=new('05_mcu','Controller / 1887VE7T / internal RC / 5 V')
part(s,'1887VE7T','U2','1887VE7T / 4203.64-2',195,100,[mapping.get(n) for n in mn])
part(s,'J6','J5','ISP 2x3',70,225,['ISP_PDO','+5V','MCU_SCK','ISP_PDI','MCU_RESETn','GND'])
grid(s,[('R','10k','+5V','PENn'),('R','10k','+5V','MCU_RESETn'),('C','100n','MCU_RESETn','GND'),('C','100n','AREF','GND')],185)
for i in range(3):pas(s,'C','100n / U2.'+str([21,52,64][i]),235+i*65,230,'+5V','GND')
note(s,'Configure internal RC 8 MHz. AVCC tied directly to VCC. Disable unused peripherals.\nISP uses PE0/PE1, not PB2/PB3. J5.2=target voltage sense; programmer must not power target.\nNC GPIO: configure as inputs with pull-ups; BQ1/BQ2 unused in internal RC mode.')
s=new('06_ui','Frequency selector / 12 contacts and APPLY')
for i in range(12):
 x=65+i%4*95;y=40+i//4*60
 part(s,'SW_SPST',f'SW{i+1}',f'DIP bit {i}',x,y,[f'DIP{i}','GND']);pas(s,'R','10k',x,y+22,'+5V',f'DIP{i}')
part(s,'SW_SPST','SW13','APPLY momentary NO',90,235,['APPLYn','GND']);pas(s,'R','10k',215,235,'+5V','APPLYn');pas(s,'C','100n',315,235,'APPLYn','GND')
note(s,'ON=bit 1: invert GPIO. N=sum(bit[i]*2^i). f=47,000,000+1,000*N Hz.\nAccept N=0..4000; reject 4001..4095. Debounce APPLY and read stable DIP twice.\nSW1..12 may be grouped into physical DIP packages when assigning footprints.')
block('LVC125',['1OEn','1A','1Y','2OEn','2A','2Y','GND','3Y','3A','3OEn','4Y','4A','4OEn','VCC'],{1:'input',2:'input',3:'tri_state',4:'input',5:'input',6:'tri_state',7:'power_in',8:'tri_state',9:'input',10:'input',11:'tri_state',12:'input',13:'input',14:'power_in'})
block('AHCT1G125',['OEn','A','GND','Y','VCC'],{1:'input',2:'input',3:'power_in',4:'tri_state',5:'power_in'})
s=new('07_spi','SPI / 5 V to 3.3 V translation')
part(s,'LVC125','U6','SN74LVC125APWR',85,65,['BUF_OEn','MCU_SCK','SCK_BUF','BUF_OEn','MCU_MOSI','MOSI_BUF','GND','CS_BUF','MCU_CS','BUF_OEn','RST_BUF','MCU_DDS_RESET','BUF_OEn','+3V3'])
part(s,'AHCT1G125','U7','SN74AHCT1G125DBVR',280,65,['GND','SDO_BUF','GND','MCU_MISO','+5V'])
grid(s,[('R','33R','SCK_BUF','DDS_SCK'),('R','33R','MOSI_BUF','DDS_SDI'),('R','33R','CS_BUF','DDS_SCSn'),('R','33R','RST_BUF','DDS_RSTn'),('R','1k','DDS_SDO','SDO_BUF'),('R','100k','SDO_BUF','GND'),('R','10k','DDS_SCK','GND'),('R','10k','DDS_SDI','GND'),('R','10k','+3V3','DDS_SCSn'),('R','10k','DDS_RSTn','GND'),('R','10k','+3V3','BUF_OEn'),('R','10k','MCU_DDS_RESET','GND'),('R','10k','+5V','MCU_CS'),('R','10k','MCU_SCK','GND'),('R','10k','MCU_MOSI','GND'),('C','100n','+3V3','GND'),('C','100n','+5V','GND')],120)
note(s,'U6 disabled at startup. PB5 open-drain emulation: output LOW or input, never output HIGH.\nSet CS=1/SCK=0/MOSI=0/RESET=0, wait rails and clock; enable U6, then release DDS reset.\nStart SPI at 100 kHz / MSB first / 24-bit commands. Keep U6 disabled during ISP.',20,235,1.2)
block('GK361_UNVERIFIED',['EN?','NC?','GND?','OUT_P?','OUT_N?','VCC?'],{1:'input',2:'no_connect',3:'power_in',4:'output',5:'output',6:'power_in'})
s=new('08_clock','480 MHz clock / oscillator option and external input')
part(s,'GK361_UNVERIFIED','G1','GK361-TK-02 / DNP',70,65,['OSC_EN',None,'GND','OSC_P','OSC_M','+3V3_OSC'],dnp=True)
grid(s,[('R','10k / DNP','+3V3_OSC','OSC_EN'),('R','0R / DNP','OSC_P','CLKP'),('R','0R / DNP','OSC_M','CLKM'),('R','100R / 1%','CLKP','CLKM'),('C','100n','+3V3_OSC','GND'),('C','1u','+3V3_OSC','GND')],115)
part(s,'J2','J2','SMA EXT / 480MHz',80,180,['EXT_CLK','GND']);pas(s,'R','49.9R / 1%',175,180,'EXT_CLK','GND');pas(s,'C','10n C0G',260,180,'EXT_CLK','CLKDP');pas(s,'C','10n C0G',350,180,'CLKDM','GND')
part(s,'SW_SPST','SW14','CSEL / closed=EXT',85,230,['+3V3','CSEL']);pas(s,'R','10k',215,230,'CSEL','GND')
note(s,'Default: G1 and 0R links DNP; SW14 closed. J2: 480 MHz sine / 0 dBm / 50 ohm.\nG1 option: verify manufacturer pinout, 480 MHz grade, LVDS, enable and current BEFORE fitting.\nThen fit G1/links and open SW14. G1 pair: 100-ohm differential. No guessed footprint.\nU1 clock input has internal common-mode bias; external input is AC coupled.',20,255,1.2)
s=new('09_rf','RF / DAC loads / 7-pole LPF / pi pad / output')
grid(s,[('R','49.9R / 0.1%','OUTP1','GND'),('R','49.9R / 0.1%','OUTM1','GND'),('R','49.9R / 0.1%','OUTP2','GND'),('R','49.9R / 0.1%','OUTM2','GND')],35)
def rf(k,val,x,y,a,b,angle):
 counts[k]=counts.get(k,0)+1;ref=k+str(counts[k]);p=part(s,k,ref,val,x,y,['@','@'],angle)
 exp[ref+'.1']=a;exp[ref+'.2']=b;return p
y=110;last=(20,y);label(s,last,'OUTP1')
for k,val,a,b,x in [('C','10n C0G','OUTP1','RF0',45),('L','51nH','RF0','RF1',85),('L','200nH','RF1','RF2',145),('L','200nH','RF2','RF3',205),('L','51nH','RF3','RF4',265),('R','5.76R 1%','RF4','RF_OUT',330)]:
 p=rf(k,val,x,y,a,b,90);wire(s,last,p['1']);last=p['2']
wire(s,last,(390,y));label(s,(390,y),'RF_OUT')
for net,x,val,k in [('RF1',110,'56p C0G','C'),('RF2',170,'91p C0G','C'),('RF3',230,'56p C0G','C'),('RF4',295,'866R 1%','R'),('RF_OUT',365,'866R 1%','R')]:
 p=rf(k,val,x,135,net,'GND',0);wire(s,(x,y),p['1']);wire(s,p['2'],(x,150));label(s,(x,150),'GND')
 s['obj'].append(f'(junction (at {x} {y}) (diameter 0) (color 0 0 0 0) (uuid {u()}))')
part(s,'J2','J3','SMA OUT / 50 ohm',80,205,['RF_OUT','GND'])
note(s,'OUTM1 has matching DC termination. IFS nominal 10.07 mA.\nUnloaded pin swing <=0.503 V nominal (limit +/-0.6 V).\n7-pole Butterworth fc~70 MHz; ideal values rounded to E24.\nPi attenuator ~1 dB. Set -10 dBm by calibrated DDS amplitude.\nUse RF high-Q inductors; verify Q/SRF with vendor models.\nNo measured SFDR/noise or production compliance claimed.',150,185,1.3)
main={'n':'DDS_47_51','title':'DDS 47-51 MHz / revision B / circuit draft','id':root,'obj':[],'used':set()}
note(main,'DDS 47-51 MHz | 1 kHz step | -10 dBm into 50 ohm\n1508PL8T + 1887VE7T | +15 V | target PCB <=80 x 50 mm',20,20,2)
note(main,'REVISION B - ENGINEERING DRAFT\nAll circuit sections connected. G1 blocked pending manufacturer passport.\nPCB placement, thermal/noise validation and output calibration remain open.\nMatching global labels connect electrically across all sheets.',20,45,1.5)
for i,sh in enumerate(sheets):
 x=25+i%3*130;y=90+i//3*50
 main['obj'].append(f'(sheet (at {x} {y}) (size 110 25) (stroke (width 0.254) (type default)) (fill (color 0 0 0 0)) (uuid {sh["id"]})'+prop('Sheetname',sh['n'],x,y-2)+prop('Sheetfile',sh['n']+'.kicad_sch',x,y+27)+f'(instances (project "DDS_47_51" (path "/{root}" (page "{i+2}")))))')
note(main,'Read README_RU.md for assembly choices, firmware contract, sources and validation limits.\nOpen DDS_47_51.kicad_pro. PDF and netlist come from this same schematic.',20,260)
for i,sh in enumerate([main]+sheets):
 t=f'(kicad_sch (version 20250114) (generator "eeschema") (uuid {sh["id"]}) (paper "A3") (title_block (title {q(sh["title"])}) (date "2026-10-29") (rev "B / engineering draft")))'
 t=t[:-1]+'(lib_symbols '+''.join(lib[n] for n in sorted(sh['used']))+')'+''.join(sh['obj'])
 if i==0:t+='(sheet_instances (path "/" (page "1")))'
 (O/(sh['n']+'.kicad_sch')).write_text(t+')',encoding='utf-8')
(O/'DDS_47_51.kicad_pro').write_text(json.dumps({'meta':{'filename':'DDS_47_51.kicad_pro','version':1}}),encoding='utf-8')
with (O/'BOM.csv').open('w',newline='',encoding='utf-8-sig') as f:
 w=csv.DictWriter(f,fieldnames=list(bom[0]));w.writeheader();w.writerows(bom)
(O/'expected_connections.json').write_text(json.dumps(exp,indent=2),encoding='utf-8')
print(len(sheets)+1,'sheets;',len(bom),'parts')

