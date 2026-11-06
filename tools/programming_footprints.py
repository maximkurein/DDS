"""Revision F service footprints. Samtec drawing AB, PCB layout drawing A.

Coordinates are TOP view: SHF pin 1 bottom left, odd row by the key notch.
No locking clips (-LC) are specified, so there are no extra mounting holes.
"""
from pathlib import Path


def header(name, description, attr):
    return [f'(footprint "{name}" (version 20241229) (generator "dds") (layer "F.Cu")',
            f'(descr "{description}") (attr {attr})',
            '(property "Reference" "REF**" (at 0 -4 0) (layer "F.SilkS") '
            '(effects (font (size 0.8 0.8) (thickness 0.12))))',
            f'(property "Value" "{name}" (at 0 4 0) (layer "F.Fab") '
            '(effects (font (size 0.8 0.8) (thickness 0.12))))']


def rect(x1, y1, x2, y2, layer, width):
    return f'(fp_rect (start {x1} {y1}) (end {x2} {y2}) (stroke (width {width}) (type solid)) (fill no) (layer "{layer}"))'


def line(a, b, layer='F.SilkS', width=0.12):
    return f'(fp_line (start {a[0]} {a[1]}) (end {b[0]} {b[1]}) (stroke (width {width}) (type solid)) (layer "{layer}"))'


def service_pads():
    name = 'ISP_Pads_2x03_P2.54mm'
    s = header(name, 'ISP pogo pads, 2x3 P2.54, diameter 1.5 mm, NO paste / NO drill',
               'smd exclude_from_pos_files exclude_from_bom')
    for n in range(1, 7):
        x, y = ((n - 1) % 2) * 2.54, ((n - 1) // 2) * 2.54
        s.append(f'(pad "{n}" smd circle (at {x} {y}) (size 1.5 1.5) (layers "F.Cu" "F.Mask"))')
    s += [rect(-1, -1, 3.54, 6.08, 'F.CrtYd', 0.05),
          rect(-0.9, -0.9, 3.44, 5.98, 'F.Fab', 0.1),
          line((-1.15, 0.6), (-1.15, -1.15)), line((-1.15, -1.15), (0.6, -1.15))]
    return name, '\n'.join(s + [')']) + '\n'


def test_pad():
    name = 'TestPad_D1.5mm'
    s = header(name, 'Exposed service pad diameter 1.5 mm, NO paste / NO drill',
               'smd exclude_from_pos_files exclude_from_bom')
    s += ['(pad "1" smd circle (at 0 0) (size 1.5 1.5) (layers "F.Cu" "F.Mask"))',
          rect(-1, -1, 1, 1, 'F.CrtYd', 0.05)]
    return name, '\n'.join(s + [')']) + '\n'


def jtag():
    name = 'Samtec_SHF-105-01-L-D-TH'
    s = header(name, 'Samtec SHF-105-01-L-D-TH, keyed shrouded 2x5 P1.27, drawing AB; drill 0.74 mm', 'through_hole')
    for n in range(1, 11):
        x, y = (((n - 1) // 2) - 2) * 1.27, (0.635 if n % 2 else -0.635)
        shape = 'rect' if n == 1 else 'circle'
        s.append(f'(pad "{n}" thru_hole {shape} (at {x:.3f} {y}) (size 1.0 1.0) (drill 0.74) (layers "*.Cu" "*.Mask"))')
    # Body L = 5*1.27+6.35=12.70 mm; conservative outer width 5.59 mm.
    s += [rect(-6.35, -2.795, 6.35, 2.795, 'F.Fab', 0.1),
          rect(-6.6, -3.05, 6.6, 3.05, 'F.CrtYd', 0.05)]
    outline = [(-6.48, 2.925), (-6.48, -2.925), (6.48, -2.925), (6.48, 2.925),
               (1.27, 2.925), (1.27, 1.78), (-1.27, 1.78), (-1.27, 2.925), (-6.48, 2.925)]
    s += [line(a, b) for a, b in zip(outline, outline[1:])]
    s += [line((-3.0, 2.3), (-2.54, 1.6)), line((-2.54, 1.6), (-2.08, 2.3)),
          line((-2.08, 2.3), (-3.0, 2.3))]
    return name, '\n'.join(s + [')']) + '\n'


def write(project):
    dst = Path(project) / 'lib' / 'DDS.pretty'
    dst.mkdir(parents=True, exist_ok=True)
    for name, content in (service_pads(), test_pad(), jtag()):
        (dst / (name + '.kicad_mod')).write_text(content, encoding='utf-8')


if __name__ == '__main__':
    import sys
    write(sys.argv[1])
