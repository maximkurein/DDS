# DDS 47–51 MHz — revision E (engineering version)

Frequency synthesizer 47–51 MHz, 1 kHz step, −10 dBm into 50 Ω; DDS 1508PL8T,
controller 1887VE7T, +15 V supply; board 120×50 mm, 4 layers (JLCPCB
JLC04161H-7628, ENIG).

Rev E replaces the imported regulators and logic with domestic parts:
DC/DC 1393EU014 (+5 V) followed by LDOs 1158EN3.3VKh (+3.3 V) and 1325EN1.8U
(+1.8 V) instead of TPS62161/2/3; 1564AP3U2 (74HC241, from +3.3 V with 1 kΩ
input resistors) and 1594TL2T (74ACT14, from +5 V) instead of SN74LVC125A and
SN74AHCT1G125.

Open `DDS_47_51.kicad_pro` in KiCad 10.0. Three schematic sheets (DDS,
controller, power), ГОСТ-style symbols/designators and title blocks. ERC 0;
connectivity matches the intended rev E netlist and, for every part the change
did not touch, rev C. Board routed, DRC with schematic parity 0 (on placeholder
land patterns). Fabrication files:
`output/jlcpcb/` (see `README_JLC.md`).

Not a production release: the land patterns of the new domestic parts are
placeholders until their package drawings are available; the 480 MHz
oscillator GK361-TK-02 (G1) is not confirmed by a datasheet and is not fitted
(DNP); the D1/D2 package drawings have not been checked; the 1325EN1.8U
dissipation (up to 1.3 W) and the 1393EU014 loop need checking. Details, open
items and rebuild instructions: [README_RU.md](README_RU.md),
[docs/design_notes.md](docs/design_notes.md).
