#!/bin/sh
# Full PCB build: placement + Freerouting + finishing router + cleanup + DRC.
#   tools/route_all.sh <project_dir>      (run inside the KiCad environment)
set -e
P=$(cd "$1" && pwd); T=$(cd "$(dirname "$0")" && pwd)
cd "$P"
mkdir -p pcb_route reports
python3 "$T/dds_pcb.py" route "$P" 3
cp DDS_47_51.kicad_pcb pcb_route/routed.kicad_pcb
cleanup() {   # separate processes: the KiCad SWIG iterators are not re-entrant
  for i in 1 2 3; do python3 "$T/zero_len.py" DDS_47_51.kicad_pcb && break || true; done
}
for i in 1 2; do
  python3 "$T/minirouter.py" "$P" | grep -v ': B$\|: via$\|: F$\|: F2$' || true
  cleanup
done
for i in 1 2; do
  python3 "$T/pro_merge.py" . >/dev/null       # pcbnew drops the schematic settings
  kicad-cli pcb drc --schematic-parity --severity-all -o reports/DRC.rpt DDS_47_51.kicad_pcb >/dev/null
  python3 "$T/drc_fix.py" DDS_47_51.kicad_pcb reports/DRC.rpt
  cleanup
  python3 "$T/minirouter.py" "$P" | grep -v ': B$\|: via$\|: F$\|: F2$' || true
  cleanup
done
python3 "$T/silk_place.py" DDS_47_51.kicad_pcb
python3 "$T/pcb_meta.py" DDS_47_51.kicad_pcb
python3 "$T/pro_merge.py" .
kicad-cli pcb drc --schematic-parity --severity-all -o reports/DRC.rpt DDS_47_51.kicad_pcb | tail -1
grep 'Found' reports/DRC.rpt
