#!/bin/bash
# Full regeneration of the DDS KiCad project (KiCad 10 + its Python + Java 25 + Freerouting).
# Usage: tools/build_all.sh <project_dir> [route]
# Paths to kicad-cli, KiCad Python, footprints, Freerouting: see tools/env.sh
set -e
P=$(cd "$1" && pwd); T=$(cd "$(dirname "$0")" && pwd)
. "$T/env.sh"
FP=$KICAD_FP
"$KPY" "$T/make_footprints.py" "$FP" "$P"
"$KPY" "$T/dds_build.py" "$P" "$P/verification/netlist_revC.xml"
cd "$P"
for s in DDS_47_51 02_controller 03_power; do "$KICAD_CLI" sch upgrade --force $s.kicad_sch >/dev/null; done
mkdir -p output/netlist output/pdf output/bom reports
"$KICAD_CLI" sch export netlist --format kicadxml -o output/netlist/netlist.xml DDS_47_51.kicad_sch
"$KICAD_CLI" sch erc --severity-all -o reports/ERC.rpt DDS_47_51.kicad_sch
# rev E: intended netlist (rev C + ECO), then rev C for every part the ECO did not touch
"$KPY" "$T/netcompare.py" --expected verification/expected_revE.json output/netlist/netlist.xml | tail -1
EXA=$("$KPY" -c "import json; print(','.join(json.load(open('verification/eco_revE.json'))['revC']))")
EXB=$("$KPY" -c "import json; print(','.join(json.load(open('verification/eco_revE.json'))['revE']))")
"$KPY" "$T/netcompare.py" --ignore-values --refmap verification/refmap_revC_to_revE.json \
    --exclude-a "$EXA" --exclude-b "$EXB" verification/netlist_revC.xml output/netlist/netlist.xml | tail -1
"$KPY" "$T/dds_pcb.py" place "$P"
"$KPY" "$T/pcb_meta.py" DDS_47_51.kicad_pcb
"$KPY" "$T/pro_merge.py" .
if [ "$2" = route ]; then
  sh "$T/route_all.sh" "$P"       # Freerouting (FREEROUTING_JAR) + finishing router + DRC
fi
