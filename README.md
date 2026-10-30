# DDS 47–51 MHz — revision D (engineering version)

Frequency synthesizer 47–51 MHz, 1 kHz step, −10 dBm into 50 Ω; DDS 1508PL8T,
controller 1887VE7T, +15 V supply; board 120×50 mm, 4 layers (JLCPCB
JLC04161H-7628, ENIG).

Open `DDS_47_51.kicad_pro` in KiCad 10.0. Three schematic sheets (DDS,
controller, power), ГОСТ-style symbols/designators and title blocks; the PCB
is fully routed. ERC 0, DRC with schematic parity 0, connectivity identical to
rev C. Fabrication files: `output/jlcpcb/` (see `README_JLC.md`).

Not a production release yet: the 480 MHz oscillator GK361-TK-02 (G1) is not
confirmed by a datasheet and is not fitted (DNP); the D1/D2 package drawings
from the ТУ have not been checked. Details, open items and rebuild
instructions: [README_RU.md](README_RU.md), [docs/design_notes.md](docs/design_notes.md).
