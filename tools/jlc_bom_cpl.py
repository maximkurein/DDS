#!/usr/bin/env python3
"""JLCPCB assembly files from the KiCad outputs.

    python3 tools/jlc_bom_cpl.py <netlist.xml> <kicad_pos.csv> <out_dir>

BOM_JLC.csv : Comment, Designator, Footprint, JLCPCB Part #  (DNP excluded)
CPL_JLC.csv : Designator, Mid X, Mid Y, Layer, Rotation      (DNP excluded)
The LCSC part numbers are left empty: the domestic ICs (1508ПЛ8Т, 1887ВЕ7Т)
are not stocked by JLCPCB, so the intended order is bare PCB + stencil and
assembly by the customer (see output/jlcpcb/README_JLC.md).  Rotations are
KiCad's; JLC's viewer must be checked for polarized parts before ordering.
"""
import csv
import re
import sys
import xml.etree.ElementTree as ET
from collections import OrderedDict
from pathlib import Path


def natural(r):
    m = re.match(r'([A-Z]+)(\d+)', r)
    return (m.group(1), int(m.group(2))) if m else (r, 0)


def main(net, pos, out):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    comps = {}
    for c in ET.parse(net).getroot().iter('comp'):
        props = {p.get('name') for p in c.iter('property')}
        fields = {f.get('name'): (f.text or '') for f in c.iter('field')}
        comps[c.get('ref')] = dict(value=c.findtext('value'), fp=(c.findtext('footprint') or '').split(':')[-1],
                                   dnp='dnp' in props, fields=fields)
    groups = OrderedDict()
    for ref in sorted(comps, key=natural):
        c = comps[ref]
        if c['dnp']:
            continue
        f = c['fields']
        extra = ' '.join(x for x in (f.get('Допуск', ''), f.get('Напряжение', ''), f.get('ТКЕ', ''),
                                     f.get('Мощность', '').replace(',', '.').replace(' Вт', 'W')) if x)
        comment = (c['value'] + (' ' + extra if extra else '')).strip()
        groups.setdefault((comment, c['fp']), []).append(ref)
    with open(out / 'BOM_JLC.csv', 'w', newline='', encoding='utf-8') as fh:
        w = csv.writer(fh)
        w.writerow(['Comment', 'Designator', 'Footprint', 'JLCPCB Part #'])
        for (comment, fp), refs in groups.items():
            w.writerow([comment, ','.join(refs), fp, ''])
    n = 0
    with open(pos, newline='', encoding='utf-8') as fh, open(out / 'CPL_JLC.csv', 'w', newline='', encoding='utf-8') as fo:
        r = csv.DictReader(fh)
        w = csv.writer(fo)
        w.writerow(['Designator', 'Mid X', 'Mid Y', 'Layer', 'Rotation'])
        for row in sorted(r, key=lambda x: natural(x['Ref'])):
            ref = row['Ref']
            if ref not in comps or comps[ref]['dnp']:
                continue
            w.writerow([ref, f"{float(row['PosX']):.4f}mm", f"{float(row['PosY']):.4f}mm",
                        'Top' if row['Side'].lower().startswith('top') else 'Bottom', f"{float(row['Rot']):.0f}"])
            n += 1
    placed = sum(len(v) for v in groups.values())
    print(f'BOM_JLC: {len(groups)} lines, {placed} parts; CPL_JLC: {n} placements')
    assert n == placed, 'CPL and BOM disagree'


if __name__ == '__main__':
    main(*sys.argv[1:4])
