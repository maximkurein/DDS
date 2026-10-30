# DDS 47–51 MHz — revision C (working draft)

Frequency synthesizer 47–51 MHz, 1 kHz step, −10 dBm into 50 Ω; DDS 1508PL8T,
controller 1887VE7T, +15 V supply, board ≤ 80×50 mm.

Open `DDS_47_51.kicad_pro` in KiCad 10.0. The schematic has exactly three
sheets (DDS, controller, power) and no index sheet. See [README_RU.md](README_RU.md)
for the folder layout and status.

This is an engineering draft, not a manufacturing release: ERC is not clean,
footprints are not assigned, there is no PCB yet, and the 480 MHz oscillator
GK361-TK-02 is not confirmed by a datasheet. Rev B (10 sheets) is preserved in
git under the tag `revB-baseline`.
