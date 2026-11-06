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
for s in DDS_48_52 02_controller 03_power; do "$KICAD_CLI" sch upgrade --force $s.kicad_sch >/dev/null; done
mkdir -p output/netlist output/pdf output/bom reports
"$KICAD_CLI" sch export netlist --format kicadxml -o output/netlist/netlist.xml DDS_48_52.kicad_sch
"$KICAD_CLI" sch erc --severity-all -o reports/ERC.rpt DDS_48_52.kicad_sch
# Revision F gates; preserve all historical reference netlists.
"$KPY" "$T/netcompare.py" --expected verification/expected_revF.json output/netlist/netlist.xml
"$KPY" "$T/netcompare.py" --ignore-values --refmap verification/refmap_revE_to_revF.json \
    --exclude-a D2 --exclude-b D2,X3,TP1,TP2 verification/netlist_revE.xml output/netlist/netlist.xml
"$KPY" "$T/dds_pcb.py" place "$P"
"$KPY" "$T/pcb_meta.py" DDS_48_52.kicad_pcb
"$KPY" "$T/pro_merge.py" .
if [ "$2" = route ]; then
  sh "$T/route_all.sh" "$P"       # Freerouting (FREEROUTING_JAR) + finishing router + DRC
fi
