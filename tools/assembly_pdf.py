"""Demonstration assembly view from measured PCB courtyard/pad geometry."""
import json
import sys
from pathlib import Path
from reportlab.pdfgen import canvas
from reportlab.lib.units import mm
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont


def main(data, output):
    parts=json.loads(Path(data).read_text(encoding='utf-8'))
    pdfmetrics.registerFont(TTFont('Arial','C:/Windows/Fonts/arial.ttf'))
    c=canvas.Canvas(output,pagesize=(420*mm,297*mm))
    c.setTitle('DDS 48–52 МГц. Ревизия F. Предварительный общий вид платы')
    def text(x,y,t,size=10):
        c.setFont('Arial',size);c.drawString(x*mm,y*mm,t)
    def line(x1,y1,x2,y2):c.line(x1*mm,y1*mm,x2*mm,y2*mm)
    def point(x,y):return 48+2*x,90+2*(50-y)
    c.setLineWidth(.3*mm);c.rect(12*mm,12*mm,396*mm,273*mm)
    text(22,272,'DDS 48–52 МГц · Ревизия F',19)
    text(22,261,'Предварительный общий вид платы со стороны установки компонентов',12)
    text(22,251,'Масштаб 2:1 · размеры в мм · 120 × 50 × 1,6 · 4 слоя',10)
    c.setFillColorRGB(.96,.97,.98);c.roundRect(48*mm,90*mm,240*mm,100*mm,2*mm,fill=1)
    main={'D1','D2','D3','D5','D6','D7','G1','X1','X3','X2','XW1','XW2'}
    refs={p['ref']:p for p in parts}
    for p in parts:
        x,y,w,h=p['box'];px,py=point(x,y+h)
        c.setLineWidth(.12*mm)
        c.setStrokeColorRGB(.35,.4,.45)
        c.setFillColorRGB(*((.77,.87,.94) if p['ref'] in main else (.92,.93,.94)))
        c.rect(px*mm,py*mm,2*w*mm,2*h*mm,stroke=1,fill=1)
        c.setFillColorRGB(.55,.55,.55)
        for ax,ay,aw,ah in p['pads']:
            px,py=point(ax-aw/2,ay+ah/2);c.rect(px*mm,py*mm,aw*2*mm,ah*2*mm,stroke=0,fill=1)
        if p['ref'] in main:
            px,py=point(*p['pos']);c.setFillColorRGB(0,0,0);text(px-2,py-.8,p['ref'],7)
    c.setStrokeColorRGB(0,0,0);c.setFillColorRGB(0,0,0);c.setLineWidth(.2*mm)
    # Dimensions and extension lines are independent of component labels.
    for x in (48,288):line(x,190,x,204)
    line(48,201,288,201);text(162,203,'120',11)
    for x in (48,288):line(x-1.5,199.5,x+1.5,202.5)
    for y in (90,190):line(288,y,302,y)
    line(299,90,299,190);text(301,138,'50',11)
    for y in (90,190):line(297.5,y-1.5,300.5,y+1.5)
    labels=[('D5',25,225,'D5 · 1393ЕУ014 · +15 → +5 В'),
            ('D2',112,235,'D2 · 1887ВЕ7Т · контроллер'),
            ('X3',112,218,'X3 · AVR JTAG · 2×5 / 1,27 мм'),
            ('D1',216,235,'D1 · 1508ПЛ8Т · DDS'),
            ('G1',308,218,'G1 · опорный генератор'),
            ('X1',25,66,'X1 · площадки ISP'),
            ('D6',25,49,'D6 / D7 · стабилизаторы +3,3 / +1,8 В'),
            ('XW1',210,66,'XW1 · внешний такт 480 МГц'),
            ('XW2',308,76,'XW2 · выход 48–52 МГц')]
    for ref,x,y,label in labels:
        px,py=point(*refs[ref]['pos']);text(x,y,label,9)
        ex=x+5;ey=y-2 if y>200 else y+4
        line(ex,ey,ex,ey-5 if y>200 else ey+5);line(ex,ey-5 if y>200 else ey+5,px,py)
        c.circle(px*mm,py*mm,.6*mm,fill=1)
    text(22,31,'Контуры компонентов показаны условно по посадочным местам. Это демонстрационный рисунок, не выпуск СБ.',9)
    text(22,23,'G1 не устанавливается до подтверждения паспорта. Тип L8 по перечню Минпромторга ещё не выбран.',9)
    text(326,16,'06.11.2026 · Лист 1 / 1',9)
    c.save()


if __name__=='__main__':main(*sys.argv[1:3])
