#!/bin/sh
# Release outputs from the KiCad project (run inside the KiCad environment).
#   tools/make_outputs.sh <project_dir> [<word_template.docx>]
# Schematic PDF, BOM, netlist, ERC/DRC reports, board PDF, JLCPCB fabrication
# set (Gerber + Excellon + BOM/CPL), Visio Э3 and Word ПЭ3.
set -e
P=$(cd "$1" && pwd); T=$(cd "$(dirname "$0")" && pwd)
TPL=${2:-"$T/templates/ПЭ3_форма_А4.docx"}
cd "$P"
mkdir -p output/pdf output/bom output/netlist output/visio output/docs reports
SCH=DDS_47_51.kicad_sch; PCB=DDS_47_51.kicad_pcb

# --- schematic
kicad-cli sch export netlist --format kicadxml -o output/netlist/netlist.xml $SCH
kicad-cli sch export pdf -o output/pdf/DDS_47_51_sch.pdf $SCH >/dev/null
kicad-cli sch export bom -o output/bom/BOM.csv \
  --fields 'Reference,Value,Footprint,Допуск,Мощность,Напряжение,ТКЕ,Примечание,${QUANTITY},${DNP}' \
  --labels 'Поз. обозначение,Номинал,Посадочное место,Допуск,Мощность,Напряжение,ТКЕ,Примечание,Кол.,Не устанавливать' \
  --group-by 'Value,Footprint,Допуск,Мощность,Напряжение,ТКЕ,Примечание,${DNP}' --ref-range-delimiter '-' $SCH >/dev/null
kicad-cli sch erc --severity-all -o reports/ERC.rpt $SCH | tail -1

# --- board
kicad-cli pcb drc --schematic-parity --severity-all -o reports/DRC.rpt $PCB | tail -1
kicad-cli pcb export pdf --mode-multipage --include-border-title --drill-shape-opt 1 --scale 2 \
  -l F.Cu,In1.Cu,In2.Cu,B.Cu,F.Silkscreen,F.Fab --cl Edge.Cuts -o output/pdf/DDS_47_51_pcb.pdf $PCB >/dev/null

# --- JLCPCB fabrication set
J=output/jlcpcb; rm -rf $J/gerber; mkdir -p $J/gerber
kicad-cli pcb export gerbers --no-x2 --no-netlist --subtract-soldermask \
  -l F.Cu,In1.Cu,In2.Cu,B.Cu,F.Paste,B.Paste,F.Silkscreen,B.Silkscreen,F.Mask,B.Mask,Edge.Cuts \
  -o $J/gerber/ $PCB >/dev/null
kicad-cli pcb export drill --format excellon --excellon-units mm --excellon-zeros-format decimal \
  --excellon-oval-format alternate --excellon-separate-th --drill-origin absolute \
  --generate-map --map-format gerberx2 -o $J/gerber/ $PCB >/dev/null
(cd $J/gerber && rm -f ../DDS_47_51_gerber.zip && python3 -c "
import zipfile, os
with zipfile.ZipFile('../DDS_47_51_gerber.zip', 'w', zipfile.ZIP_DEFLATED) as z:
    for f in sorted(os.listdir('.')):
        if 'drl_map' not in f:            # drill maps are for review only
            z.write(f)
    print('gerber zip:', len(z.namelist()), 'files')")
kicad-cli pcb export pos --format csv --units mm --side both --exclude-dnp -o $J/pos_kicad.csv $PCB >/dev/null
python3 "$T/jlc_bom_cpl.py" output/netlist/netlist.xml $J/pos_kicad.csv $J

# --- demonstration documents (Visio Э3, Word ПЭ3): need pip packages vsdx, python-docx, pillow
if ! python3 -c "import vsdx, docx, PIL" 2>/dev/null; then
  echo "Visio/Word: python packages vsdx, python-docx, pillow missing - skipped"; exit 0
fi
python3 "$T/sch2vsdx.py" "$P" "output/visio/DDS_47_51_Э3.vsdx" | tail -1
if [ -f "$TPL" ]; then
  python3 "$T/pe3_docx.py" output/netlist/netlist.xml "$TPL" "output/docs/DDS_47_51_ПЭ3.docx"
else
  echo "ПЭ3: template not found ($TPL) - skipped"
fi
