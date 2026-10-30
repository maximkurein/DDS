#!/bin/sh
# Full PCB build: placement + Freerouting + finishing router + cleanup + DRC.
#   tools/route_all.sh <project_dir>      (KiCad paths: tools/env.sh)
set -e
P=$(cd "$1" && pwd); T=$(cd "$(dirname "$0")" && pwd)
. "$T/env.sh"
cd "$P"
mkdir -p pcb_route reports
"$KPY" "$T/dds_pcb.py" route "$P" 3
cp DDS_47_51.kicad_pcb pcb_route/routed.kicad_pcb
cleanup() {   # separate processes: the KiCad SWIG iterators are not re-entrant
  for i in 1 2 3; do "$KPY" "$T/zero_len.py" DDS_47_51.kicad_pcb && break || true; done
}
for i in 1 2; do
  "$KPY" "$T/minirouter.py" "$P" | grep -v ': B$\|: via$\|: F$\|: F2$' || true
  cleanup
done
for i in 1 2; do
  "$KPY" "$T/pro_merge.py" . >/dev/null       # pcbnew drops the schematic settings
  "$KICAD_CLI" pcb drc --schematic-parity --severity-all -o reports/DRC.rpt DDS_47_51.kicad_pcb >/dev/null
  "$KPY" "$T/drc_fix.py" DDS_47_51.kicad_pcb reports/DRC.rpt
  cleanup
  "$KPY" "$T/minirouter.py" "$P" | grep -v ': B$\|: via$\|: F$\|: F2$' || true
  cleanup
done
"$KPY" "$T/silk_place.py" DDS_47_51.kicad_pcb
"$KPY" "$T/pcb_meta.py" DDS_47_51.kicad_pcb
"$KPY" "$T/pro_merge.py" .
"$KICAD_CLI" pcb drc --schematic-parity --severity-all -o reports/DRC.rpt DDS_47_51.kicad_pcb | tail -1
grep 'Found' reports/DRC.rpt
