"""Component database for DDS 47-51 MHz (rev D numbering).

Keys are the rev C designators (stable identities: the regenerated schematic
keeps UUIDs derived from them).  Each entry gives the KiCad symbol, value,
footprint and BOM fields.  Imported passives are allowed; the "Аналог" field
names a domestic candidate whose footprint compatibility still has to be
confirmed against its ТУ (marked "проверить").
"""

FP_R0603 = 'Resistor_SMD:R_0603_1608Metric'
FP_C0603 = 'Capacitor_SMD:C_0603_1608Metric'
FP_C0805 = 'Capacitor_SMD:C_0805_2012Metric'
FP_C1206 = 'Capacitor_SMD:C_1206_3216Metric'
FP_L0603 = 'Inductor_SMD:L_0603_1608Metric'
FP_L4020 = 'Inductor_SMD:L_Changjiang_FNR4020S'

PARTS = {}


def P(refs, **kw):
    for r in refs:
        PARTS[r] = dict(kw)


def rng(prefix, a, b):
    return [f'{prefix}{i}' for i in range(a, b + 1)]


# ---------------------------------------------------------------- resistors
R_DESC = 'Резистор'
P(['R1'], sym='R', value='47k', fp=FP_R0603, tol='5%', pwr='0,063 Вт')
P(['R2', 'R3'], sym='R', value='7.15k', fp=FP_R0603, tol='0,1%', pwr='0,063 Вт',
  note='задаёт IFS = 72 В / R = 10,07 мА')
P(['R4', 'R5', 'R6'], sym='R', value='10k', fp=FP_R0603, tol='5%', pwr='0,063 Вт')
P(rng('R', 7, 35), sym='R', value='100k', fp=FP_R0603, tol='5%', pwr='0,063 Вт')
P(rng('R', 36, 50), sym='R', value='10k', fp=FP_R0603, tol='5%', pwr='0,063 Вт')
P(rng('R', 51, 54), sym='R', value='33R', fp=FP_R0603, tol='5%', pwr='0,063 Вт')
P(['R55'], sym='R', value='1k', fp=FP_R0603, tol='5%', pwr='0,063 Вт')
P(['R56'], sym='R', value='100k', fp=FP_R0603, tol='5%', pwr='0,063 Вт')
P(rng('R', 57, 65), sym='R', value='10k', fp=FP_R0603, tol='5%', pwr='0,063 Вт')
P(['R66'], sym='R', value='10k', fp=FP_R0603, tol='5%', pwr='0,063 Вт', dnp=True)
P(['R67', 'R68'], sym='R', value='0R', fp=FP_R0603, tol='', pwr='0,063 Вт', dnp=True)
P(['R69'], sym='R', value='100R', fp=FP_R0603, tol='1%', pwr='0,063 Вт')
P(['R70'], sym='R', value='49.9R', fp=FP_R0603, tol='1%', pwr='0,063 Вт')
P(['R71'], sym='R', value='10k', fp=FP_R0603, tol='5%', pwr='0,063 Вт')
P(rng('R', 72, 75), sym='R', value='49.9R', fp=FP_R0603, tol='0,1%', pwr='0,063 Вт')
P(['R76'], sym='R', value='5.76R', fp=FP_R0603, tol='1%', pwr='0,063 Вт')
P(['R77', 'R78'], sym='R', value='866R', fp=FP_R0603, tol='1%', pwr='0,063 Вт')

# --------------------------------------------------------------- capacitors
P(['C1', 'C4', 'C7'], sym='C', value='10u', fp=FP_C1206, volt='25V', diel='X7R')
P(['C2', 'C5', 'C8'], sym='C', value='22u', fp=FP_C1206, volt='10V', diel='X7R')
P(['C3', 'C6', 'C9'], sym='C', value='100n', fp=FP_C0603, volt='50V', diel='X7R')
P(['C10', 'C11', 'C12'], sym='C', value='10u', fp=FP_C0805, volt='10V', diel='X7R')
P(rng('C', 13, 34), sym='C', value='100n', fp=FP_C0603, volt='50V', diel='X7R')
P(rng('C', 35, 38), sym='C', value='10u', fp=FP_C0805, volt='10V', diel='X7R')
P(rng('C', 39, 47), sym='C', value='100n', fp=FP_C0603, volt='50V', diel='X7R')
P(['C48'], sym='C', value='1u', fp=FP_C0603, volt='16V', diel='X7R')
P(['C49', 'C50', 'C51'], sym='C', value='10n', fp=FP_C0603, volt='50V', diel='C0G')
P(['C52', 'C54'], sym='C', value='56p', fp=FP_C0603, volt='50V', diel='C0G', tol='2%')
P(['C53'], sym='C', value='91p', fp=FP_C0603, volt='50V', diel='C0G', tol='2%')

# ---------------------------------------------------------------- inductors
P(['L1', 'L2', 'L3'], sym='L', value='2.2u', fp=FP_L4020, note='Isat >= 1,5 А')
P(['L4', 'L7'], sym='L', value='51n', fp=FP_L0603, note='ВЧ, Q >= 30 на 50 МГц', tol='2%')
P(['L5', 'L6'], sym='L', value='200n', fp=FP_L0603, note='ВЧ, Q >= 30 на 50 МГц', tol='2%')
P(['FB1', 'FB2', 'FB3'], sym='L_Fe', value='600R@100MHz', fp=FP_L0603,
  note='ферритовый фильтр, Rdc <= 0,1 Ом, 1 А')

# ------------------------------------------------------------ semiconductors
P(['D1'], sym='VD_Schottky', value='SS34', fp='Diode_SMD:D_SMA')
P(['F1'], sym='F', value='0.5A', fp='Fuse:Fuse_1206_3216Metric')
P(['U1'], sym='1508PL8T', value='1508ПЛ8Т', fp='DDS:LQFP-100_14x14mm_P0.5mm_1508PL8T')
P(['U2'], sym='1887VE7T', value='1887ВЕ7Т', fp='DDS:CQFP-64_4203.64-2_14x14mm_P0.8mm_1887VE7T')
P(['U3'], sym='TPS6216x', value='TPS62163DSGR',
  fp='Package_SON:Texas_DSG0008A_WSON-8-1EP_2x2mm_P0.5mm_EP0.9x1.6mm', note='+5 В')
P(['U4'], sym='TPS6216x', value='TPS62162DSGR',
  fp='Package_SON:Texas_DSG0008A_WSON-8-1EP_2x2mm_P0.5mm_EP0.9x1.6mm', note='+3,3 В')
P(['U5'], sym='TPS6216x', value='TPS62161DSGR',
  fp='Package_SON:Texas_DSG0008A_WSON-8-1EP_2x2mm_P0.5mm_EP0.9x1.6mm', note='+1,8 В')
P(['U6'], sym='SN74LVC125A', value='SN74LVC125APWR', fp='Package_SO:TSSOP-14_4.4x5mm_P0.65mm')
P(['U7'], sym='SN74AHCT1G125', value='SN74AHCT1G125DBVR', fp='Package_TO_SOT_SMD:SOT-23-5')
P(['G1'], sym='GK361_UNVERIFIED', value='ГК361-ТК-02', dnp=True,
  fp='DDS:OSC_6pad_5.0x3.2mm_P1.27mm_GK361_UNVERIFIED',
  note='480 МГц LVDS; распиновка и корпус НЕ ПОДТВЕРЖДЕНЫ паспортом')

# ----------------------------------------------------- connectors / switches
P(['J1'], sym='X_2', value='PLS-2', fp='Connector_PinHeader_2.54mm:PinHeader_1x02_P2.54mm_Vertical',
  note='вход +15 В')
P(['J2'], sym='XW_Coax', value='SMA', fp='Connector_Coaxial:SMA_Samtec_SMA-J-P-H-ST-EM1_EdgeMount',
  note='вход 480 МГц, 0 дБм, 50 Ом')
P(['J3'], sym='XW_Coax', value='SMA', fp='Connector_Coaxial:SMA_Samtec_SMA-J-P-H-ST-EM1_EdgeMount',
  note='выход 47–51 МГц, −10 дБм, 50 Ом')
P(['J5'], sym='X_6', value='PLD-6', fp='Connector_PinHeader_2.54mm:PinHeader_2x03_P2.54mm_Vertical',
  note='ISP')

# DIP switches: 12 single sections of rev C become two physical DIP packages
DIP8 = dict(sym='S_DIP_x08', value='DIP-8',
            fp='Button_Switch_THT:SW_DIP_SPSTx08_Slide_9.78x22.5mm_W7.62mm_P2.54mm',
            note='биты 0–7 кода частоты')
DIP4 = dict(sym='S_DIP_x04', value='DIP-4',
            fp='Button_Switch_THT:SW_DIP_SPSTx04_Slide_9.78x12.34mm_W7.62mm_P2.54mm',
            note='биты 8–11 кода частоты')
DIP1 = dict(sym='S_DIP_x01', value='DIP-1',
            fp='Button_Switch_THT:SW_DIP_SPSTx01_Slide_9.78x4.72mm_W7.62mm_P2.54mm',
            note='CSEL: замкнут = внешний такт (XW1)')
BTN = dict(sym='SB', value='6x6', fp='Button_Switch_THT:SW_PUSH_6mm', note='«Применить»')


def old_to_new(ref, pin=None):
    """Map a rev C designator (and pin) to (key, unit, new pin)."""
    if ref.startswith('SW'):
        k = int(ref[2:])
        if 1 <= k <= 8:
            return 'DIP8', k, (None if pin is None else (str(k) if pin == '1' else str(17 - k)))
        if 9 <= k <= 12:
            j = k - 8
            return 'DIP4', j, (None if pin is None else (str(j) if pin == '1' else str(9 - j)))
        if k == 14:
            return 'DIP1', 1, pin
        if k == 13:
            return 'BTN', 1, pin
    return ref, None, pin


GROUPS = {'DIP8': DIP8, 'DIP4': DIP4, 'DIP1': DIP1, 'BTN': BTN}


def part(key):
    return GROUPS.get(key) or PARTS[key]


# ГОСТ 2.710 letter codes used in this project (as in the formatting samples)
PREFIX = {'R': 'R', 'C': 'C', 'L': 'L', 'FB': 'L', 'U': 'D', 'G': 'G', 'D': 'VD', 'F': 'F'}


def gost_prefix(key):
    if key in ('DIP8', 'DIP4', 'DIP1', 'BTN'):
        return 'S'
    if key in ('J2', 'J3'):
        return 'XW'
    if key.startswith('J'):
        return 'X'
    import re
    p = re.match(r'[A-Z]+', key).group(0)
    return PREFIX[p]
