#!/usr/bin/env python3
"""Create the project footprint library lib/DDS.pretty from KiCad standard footprints.

    python3 tools/make_footprints.py <kicad_footprints_dir> <project_dir>

Package data (customer, 2026-10-30; package drawings from the ТУ not yet checked):
  * 1508ПЛ8Т, АЕЯР.431320.596ТУ: LQFP-100, body 14x14x1.4 mm, 16x16 mm over
    leads, pitch 0.5 mm  -> standard IPC LQFP-100_14x14mm_P0.5mm land pattern.
  * 1887ВЕ7Т, АЕЯР.431280.910ТУ: metal-ceramic 4203.64-2 (CQFP-64), body
    14x14 mm, pitch 0.8 mm; formed flat leads need longer lands (1.8-2.0 mm):
    TQFP-64 14x14 P0.8 land pattern with pads lengthened outwards to 1.95 mm.
  * ГК361-ТК-02: NOT confirmed — 6-pad 5.0x3.2 mm P1.27 placeholder (DNP).
  * rev E: 1393ЕУ014 (4112.16-15.01), КТ-93-1 (1158ЕН3.3ВХ, 1325ЕН1.8У),
    1564АП3У2, 1594ТЛ2Т, 2Д2990А, ИД1-56 - placeholders (drawings not available).
"""
import re
import sys
from pathlib import Path


def blocks(t, head):
    """Yield (start, end) of every block that starts with '(head'."""
    i = 0
    while True:
        i = t.find('(' + head, i)
        if i < 0:
            return
        d = 0
        for j in range(i, len(t)):
            if t[j] == '(':
                d += 1
            elif t[j] == ')':
                d -= 1
                if d == 0:
                    break
        yield i, j + 1
        i = j + 1


def rename(t, name, descr, mark=None):
    old = re.match(r'\(footprint\s+"([^"]+)"', t).group(1)
    t = t.replace(f'(footprint "{old}"', f'(footprint "{name}"', 1)
    t = re.sub(r'\(descr "[^"]*"\)', '(descr "%s")' % descr.replace('"', "'"), t, count=1)
    t = re.sub(r'\(property "Value" "[^"]*"', f'(property "Value" "{name}"', t, count=1)
    if mark:
        t = t.rstrip()
        assert t.endswith(')')
        t = t[:-1] + (f'\t(fp_text user "{mark}" (at 0 1.5 0) (layer "F.Fab")\n'
                      f'\t\t(effects (font (size 0.8 0.8) (thickness 0.12))))\n)\n')
    return t


def lengthen_qfp(t, new_len):
    """Keep the heel (inner) edge of every pad, extend the toe outwards."""
    out, last, ext = [], 0, 0.0
    for a, b in blocks(t, 'pad '):
        blk = t[a:b]
        m_at = re.search(r'\(at (-?[\d.]+) (-?[\d.]+)\)', blk)
        m_sz = re.search(r'\(size ([\d.]+) ([\d.]+)\)', blk)
        x, y = float(m_at.group(1)), float(m_at.group(2))
        sx, sy = float(m_sz.group(1)), float(m_sz.group(2))
        if sx > sy:
            d = (new_len - sx) / 2
            x += d if x > 0 else -d
            sx = new_len
        else:
            d = (new_len - sy) / 2
            y += d if y > 0 else -d
            sy = new_len
        ext = max(ext, abs(x) + sx / 2, abs(y) + sy / 2)
        blk = blk.replace(m_at.group(0), f'(at {x:.4f} {y:.4f})', 1)
        blk = blk.replace(m_sz.group(0), f'(size {sx:.4f} {sy:.4f})', 1)
        out.append(t[last:a]); out.append(blk); last = b
    out.append(t[last:])
    t = ''.join(out)
    # drop the old courtyard, add a rectangle around the new land pattern
    for kind in ('fp_line', 'fp_poly', 'fp_rect'):
        while True:
            hit = next(((a, b) for a, b in blocks(t, kind) if '"F.CrtYd"' in t[a:b]), None)
            if not hit:
                break
            t = t[:hit[0]] + t[hit[1]:]
    e = round(ext + 0.25, 2)
    t = t.rstrip()[:-1] + (f'\t(fp_rect (start {-e} {-e}) (end {e} {e}) (stroke (width 0.05) (type solid))'
                           f' (fill no) (layer "F.CrtYd"))\n)\n')
    return t


def flatpack16(name, descr, pitch=1.25, xc=8.65, pl=2.2, pw=0.75, body=10.0):
    """Two-row 16-lead flat package land pattern (pins 1-8 left top-down,
    9-16 right bottom-up), courtyard and fab outline."""
    pads = []
    for i in range(8):
        y = (i - 3.5) * pitch
        pads.append((str(i + 1), -xc, y))
        pads.append((str(16 - i), xc, y))
    out = [f'(footprint "{name}"', '\t(version 20241229)', '\t(generator "dds_make_footprints")',
           '\t(layer "F.Cu")', f'\t(descr "{descr}")', '\t(attr smd)',
           f'\t(property "Reference" "REF**" (at 0 {-body / 2 - 1.5} 0) (layer "F.SilkS") '
           '(effects (font (size 1 1) (thickness 0.15))))',
           f'\t(property "Value" "{name}" (at 0 {body / 2 + 1.5} 0) (layer "F.Fab") '
           '(effects (font (size 1 1) (thickness 0.15))))',
           f'\t(fp_rect (start {-body / 2} {-body / 2}) (end {body / 2} {body / 2}) '
           '(stroke (width 0.1) (type solid)) (fill no) (layer "F.Fab"))',
           f'\t(fp_line (start {-body / 2 - 0.2} {-body / 2 + 1.0}) (end {-body / 2 - 0.2} {-body / 2 - 0.2}) '
           '(stroke (width 0.12) (type solid)) (layer "F.SilkS"))',
           f'\t(fp_circle (center {-xc - 0.4} {-3.5 * pitch - 1.2}) (end {-xc - 0.15} {-3.5 * pitch - 1.2}) '
           '(stroke (width 0.2) (type solid)) (fill yes) (layer "F.SilkS"))',
           f'\t(fp_rect (start {-xc - pl / 2 - 0.3} {-body / 2 - 0.3}) (end {xc + pl / 2 + 0.3} {body / 2 + 0.3}) '
           '(stroke (width 0.05) (type solid)) (fill no) (layer "F.CrtYd"))',
           '\t(fp_text user "PLACEHOLDER" (at 0 0 0) (layer "F.Fab") (effects (font (size 0.8 0.8) (thickness 0.12))))']
    for num, x, y in pads:
        out.append(f'\t(pad "{num}" smd roundrect (at {x:.3f} {y:.3f}) (size {pl} {pw}) '
                   '(layers "F.Cu" "F.Mask" "F.Paste") (roundrect_rratio 0.25))')
    out.append(')')
    return '\n'.join(out) + '\n'


def main(src, proj):
    src, dst = Path(src), Path(proj) / 'lib' / 'DDS.pretty'
    dst.mkdir(parents=True, exist_ok=True)
    for old in dst.glob('*.kicad_mod'):
        old.unlink()
    rd = lambda p: (src / p).read_text(encoding='utf-8')
    items = [
        ('LQFP-100_14x14mm_P0.5mm_1508PL8T',
         rename(rd('Package_QFP.pretty/LQFP-100_14x14mm_P0.5mm.kicad_mod'), 'LQFP-100_14x14mm_P0.5mm_1508PL8T',
                '1508ПЛ8Т АЕЯР.431320.596ТУ: LQFP-100 14x14x1,4, шаг 0,5 (данные заказчика; сверить с чертежом ТУ)')),
        ('CQFP-64_4203.64-2_14x14mm_P0.8mm_1887VE7T',
         rename(lengthen_qfp(rd('Package_QFP.pretty/TQFP-64_14x14mm_P0.8mm.kicad_mod'), 1.95),
                'CQFP-64_4203.64-2_14x14mm_P0.8mm_1887VE7T',
                '1887ВЕ7Т АЕЯР.431280.910ТУ: корпус 4203.64-2, тело 14x14, шаг 0,8, площадки 1,95 мм '
                '(данные заказчика; размах формованных выводов сверить с чертежом ТУ)', 'PRELIM')),
        ('OSC_6pad_5.0x3.2mm_P1.27mm_GK361_UNVERIFIED',
         rename(rd('Oscillator.pretty/Oscillator_SMD_IDT_JS6-6_5.0x3.2mm_P1.27mm.kicad_mod'),
                'OSC_6pad_5.0x3.2mm_P1.27mm_GK361_UNVERIFIED',
                'ГК361-ТК-02: корпус и распиновка НЕ подтверждены паспортом (DNP)', 'UNVERIFIED')),
    ]
    # ---- rev E: domestic regulators and logic.  Package drawings were not
    # available: these are placeholders on standard land patterns, marked on
    # F.Fab, to be replaced from the ТУ drawings before ordering boards.
    ph = 'посадочное место-ЗАПОЛНИТЕЛЬ: заменить по чертежу ТУ'
    items += [
        ('4112.16-15.01_1393EU014_PLACEHOLDER',
         flatpack16('4112.16-15.01_1393EU014_PLACEHOLDER',
                    '1393ЕУ014, корпус 4112.16-15.01: ' + ph + '; принято 2x8 выводов, шаг 1,25, '
                    'тело 10x10, размах 19,5 (данные заказчика)')),
        ('KT-93-1_PLACEHOLDER',
         rename(rd('Package_TO_SOT_SMD.pretty/TO-252-2.kicad_mod'), 'KT-93-1_PLACEHOLDER',
                'КТ-93-1 (1158ЕН3.3ВХ, 1325ЕН1.8У): ' + ph + '; принят TO-252, вывод 2 = теплоотвод',
                'PLACEHOLDER')),
        ('SOIC-20W_1564AP3U2_PLACEHOLDER',
         rename(rd('Package_SO.pretty/SOIC-20W_7.5x12.8mm_P1.27mm.kicad_mod'),
                'SOIC-20W_1564AP3U2_PLACEHOLDER', '1564АП3У2: ' + ph + '; принят SOIC-20W', 'PLACEHOLDER')),
        ('SOIC-14_1594TL2T_PLACEHOLDER',
         rename(rd('Package_SO.pretty/SOIC-14_3.9x8.7mm_P1.27mm.kicad_mod'),
                'SOIC-14_1594TL2T_PLACEHOLDER', '1594ТЛ2Т: ' + ph + '; принят SOIC-14', 'PLACEHOLDER')),
        ('D_SMA_2D2990A_PLACEHOLDER',
         rename(rd('Diode_SMD.pretty/D_SMA.kicad_mod'), 'D_SMA_2D2990A_PLACEHOLDER',
                '2Д2990А: ' + ph + '; принят SMA (DO-214AC), 1 = катод', 'PLACEHOLDER')),
        ('L_10.4x10.4_ID1_PLACEHOLDER',
         rename(rd('Inductor_SMD.pretty/L_10.4x10.4_H4.8.kicad_mod'), 'L_10.4x10.4_ID1_PLACEHOLDER',
                'Дроссель ИД1-56 (56 мкГн, Iнас >= 1,6 А): ' + ph + '; принят 10,4x10,4 мм', 'PLACEHOLDER')),
    ]
    for name, t in items:
        (dst / f'{name}.kicad_mod').write_text(t, encoding='utf-8')
        print('wrote', name)


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
