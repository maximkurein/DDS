#!/bin/bash
# Full regeneration of the DDS KiCad project (KiCad 10 + python3 + Java 25 + Freerouting).
# Usage: tools/build_all.sh <project_dir> [route]
#   KICAD_FP  - KiCad footprints dir (default /usr/share/kicad/footprints)
#   FREEROUTING_JAR - path to freerouting-2.5.0.jar (needed for "route")
set -e
P=$(cd "$1" && pwd); T=$(cd "$(dirname "$0")" && pwd)
FP=${KICAD_FP:-/usr/share/kicad/footprints}
python3 "$T/make_footprints.py" "$FP" "$P"
python3 "$T/dds_build.py" "$P" "$P/verification/netlist_revC.xml"
cd "$P"
for s in DDS_47_51 02_controller 03_power; do kicad-cli sch upgrade --force $s.kicad_sch >/dev/null; done
mkdir -p output/netlist output/pdf output/bom reports
kicad-cli sch export netlist --format kicadxml -o output/netlist/netlist.xml DDS_47_51.kicad_sch
kicad-cli sch erc --severity-all -o reports/ERC.rpt DDS_47_51.kicad_sch
python3 "$T/netcompare.py" --ignore-values --refmap verification/refmap_revC_to_revD.json \
    verification/netlist_revC.xml output/netlist/netlist.xml | tail -1
python3 "$T/dds_pcb.py" place "$P"
python3 "$T/pcb_meta.py" DDS_47_51.kicad_pcb
if [ "$2" = route ]; then
  sh "$T/route_all.sh" "$P"       # Freerouting (FREEROUTING_JAR) + finishing router + DRC
fi
