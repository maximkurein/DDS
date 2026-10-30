"""Board stack-up (JLCPCB JLC04161H-7628, 1.6 mm, ENIG) and title block,
written into the .kicad_pcb text (idempotent).

    python3 tools/pcb_meta.py board.kicad_pcb
Stack-up values are the manufacturer's published nominal ones; check them in
the JLCPCB impedance calculator before ordering impedance control."""
import sys

STACKUP = '''		(stackup
			(layer "F.SilkS" (type "Top Silk Screen") (color "White"))
			(layer "F.Paste" (type "Top Solder Paste"))
			(layer "F.Mask" (type "Top Solder Mask") (color "Green") (thickness 0.01))
			(layer "F.Cu" (type "copper") (thickness 0.035))
			(layer "dielectric 1" (type "prepreg") (color "FR4 natural") (thickness 0.2104) (material "7628") (epsilon_r 4.4) (loss_tangent 0.02))
			(layer "In1.Cu" (type "copper") (thickness 0.0152))
			(layer "dielectric 2" (type "core") (color "FR4 natural") (thickness 1.065) (material "FR4") (epsilon_r 4.6) (loss_tangent 0.02))
			(layer "In2.Cu" (type "copper") (thickness 0.0152))
			(layer "dielectric 3" (type "prepreg") (color "FR4 natural") (thickness 0.2104) (material "7628") (epsilon_r 4.4) (loss_tangent 0.02))
			(layer "B.Cu" (type "copper") (thickness 0.035))
			(layer "B.Mask" (type "Bottom Solder Mask") (color "Green") (thickness 0.01))
			(layer "B.Paste" (type "Bottom Solder Paste"))
			(layer "B.SilkS" (type "Bottom Silk Screen") (color "White"))
			(copper_finish "ENIG")
			(dielectric_constraints yes)
		)
'''

TITLE = '''	(title_block
		(title "Синтезатор частоты DDS 47–51 МГц")
		(date "2026-10-30")
		(rev "E")
		(comment 1 "ХХХХ.ХХХХХХ.ХХХ")
		(comment 2 "4 слоя, 1,6 мм, JLC04161H-7628, ENIG")
		(comment 3 "G1 (ГК361-ТК-02) не подтверждён - не устанавливать")
	)
'''


def drop_block(t, head):
    i = t.find(head)
    if i < 0:
        return t
    d = 0
    for j in range(i, len(t)):
        if t[j] == '(':
            d += 1
        elif t[j] == ')':
            d -= 1
            if d == 0:
                break
    k = j + 1
    while k < len(t) and t[k] in ' \t':
        k += 1
    if k < len(t) and t[k] == '\n':
        k += 1
    s = t.rfind('\n', 0, i) + 1
    return t[:s] + t[k:]


p = sys.argv[1]
t = open(p, encoding='utf-8').read()
t = drop_block(t, '(stackup')
t = drop_block(t, '(title_block')
t = t.replace('\t(setup\n', '\t(setup\n' + STACKUP, 1)
i = t.find('\t(paper ')
i = t.find('\n', i) + 1
t = t[:i] + TITLE + t[i:]
open(p, 'w', encoding='utf-8').write(t)
print('pcb_meta: stack-up JLC04161H-7628 / ENIG, title block rev E')
