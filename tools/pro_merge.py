#!/usr/bin/env python3
"""Restore the schematic part of the .kicad_pro after pcbnew scripts.

    python3 tools/pro_merge.py <project_dir>

pcbnew's settings manager rewrites DDS_47_51.kicad_pro without its schematic
section when a Python process exits (even after a merged save).  Lost are the
sub-unit naming D1.1 (separator '.', first id '1'), the ГОСТ drawing sheet and
the sheet list; the netlist then names D1A instead of D1.1 and the DRC
schematic-parity check reports net conflicts.  The values are the ones
tools/dds_build.py writes; the sheet list is read from the root schematic.
"""
import json, re, sys
from pathlib import Path

d = Path(sys.argv[1])
pro_path = d / 'DDS_47_51.kicad_pro'
pro = json.loads(pro_path.read_text(encoding='utf-8'))
root = (d / 'DDS_47_51.kicad_sch').read_text(encoding='utf-8')
sheets = [[re.search(r'\(uuid "([^"]+)"\)', root).group(1), 'Root']]
for blk in re.findall(r'\n\t\(sheet\n(.*?)\n\t\)', root, re.S):
    u = re.search(r'\(uuid "([^"]+)"\)', blk).group(1)
    name = re.search(r'\(property "Sheetname" "([^"]+)"', blk).group(1)
    sheets.append([u, name])
sch = pro.setdefault('schematic', {})
sch.update({'subpart_id_separator': 46, 'subpart_first_id': 49,
            'page_layout_descr_file': '${KIPRJMOD}/lib/GOST_E3.kicad_wks'})
pro['sheets'] = sheets
pro_path.write_text(json.dumps(pro, indent=2, ensure_ascii=False), encoding='utf-8')
print('pro_merge: schematic settings restored,', len(sheets), 'sheets')
