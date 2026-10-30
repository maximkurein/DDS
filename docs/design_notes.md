# Rev B design notes retained in Rev C

## 02_dds
ELVEES / 14.10.2020
Table 4.2 pin numbering
CSEL=0: G1 / CLKP,M
CSEL=1: external input
480 MHz / CTR.cmx=0

## 02_dds
Firmware initialization:
DAC2 and comparator OFF
SYNC outputs OFF
LINK OFF / profile 0
Unused IO pulled down

## 02_dds
Continuous common ground plane. RREF: nominal IFS=10.07 mA. OUT voltage limit +/-0.6 V.

## 03_bias
FB1..3 are ferrite beads, DCR <=0.1 ohm. Check LC resonance on prototype.
Unused bidirectional IO use resistive pull-downs, never direct ground shorts.

## 04_bypass
Place each capacitor at the indicated U1 supply pin with its own adjacent ground via.
Bulk capacitors supplement local bypass. Do not place all capacitors in a remote bank.

## 08_clock
Default: G1 and 0R links DNP; SW14 closed. J2: 480 MHz sine / 0 dBm / 50 ohm.
G1 option: verify manufacturer pinout, 480 MHz grade, LVDS, enable and current BEFORE fitting.
Then fit G1/links and open SW14. G1 pair: 100-ohm differential. No guessed footprint.
U1 clock input has internal common-mode bias; external input is AC coupled.

## 09_rf
OUTM1 has matching DC termination. IFS nominal 10.07 mA.
Unloaded pin swing <=0.503 V nominal (limit +/-0.6 V).
7-pole Butterworth fc~70 MHz; ideal values rounded to E24.
Pi attenuator ~1 dB. Set -10 dBm by calibrated DDS amplitude.
Use RF high-Q inductors; verify Q/SRF with vendor models.
No measured SFDR/noise or production compliance claimed.

## 05_mcu
Configure internal RC 8 MHz. AVCC tied directly to VCC. Disable unused peripherals.
ISP uses PE0/PE1, not PB2/PB3. J5.2=target voltage sense; programmer must not power target.
NC GPIO: configure as inputs with pull-ups; BQ1/BQ2 unused in internal RC mode.

## 06_ui
ON=bit 1: invert GPIO. N=sum(bit[i]*2^i). f=47,000,000+1,000*N Hz.
Accept N=0..4000; reject 4001..4095. Debounce APPLY and read stable DIP twice.
SW1..12 may be grouped into physical DIP packages when assigning footprints.

## 07_spi
U6 disabled at startup. PB5 open-drain emulation: output LOW or input, never output HIGH.
Set CS=1/SCK=0/MOSI=0/RESET=0, wait rails and clock; enable U6, then release DDS reset.
Start SPI at 100 kHz / MSB first / 24-bit commands. Keep U6 disabled during ISP.

## 01_power
Input +15 V +/-5%; VIN must stay <17 V.
D1 blocks reverse polarity.

## 01_power
Capacitance specified EFFECTIVE at DC bias; X7R. EP=GND. Keep SW loops short.
U4/U5 PG wired-AND. MCU checks rails after startup delay; PG is Hi-Z when disabled.
Budget: 1.8 V <=0.4 A DDS; 3.3 V DDS <=0.2 A plus oscillator. Verify G1 current.