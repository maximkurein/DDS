"""Rev E engineering change (ECO) on top of the rev C/D connectivity.

Imported regulators and logic are replaced by domestic parts (customer
decision 2026-10-30):

  power  : TPS62163/62/61 (3 x DC/DC)  ->  1393ЕУ014 (DC/DC +15 V -> +5 V)
                                           1158ЕН3.3ВХ (LDO +5 V -> +3,3 В)
                                           1325ЕН1.8У  (LDO +5 V -> +1,8 В)
           1393ЕУ014 PG keeps the MCU in reset until +5 V is up (as the PG
           of the old +5 V converter did).
  logic  : SN74LVC125A  -> 1564АП3У2 (74HC241) from +3,3 V, 1 kОм series
                            resistors on all five 5 V inputs incl. 1OE#
                            (clamp current about 1 mA)
           SN74AHCT1G125 -> 1594ТЛ2Т (74ACT14) from +5 V, two inverters in
                            series (TTL input accepts the 3,3 V SDO)

Keys are rev C designators (the drawing keeps UUIDs derived from them); keys
>= 100 are new parts.  The connectivity of every other part is still taken
from the rev C netlist and checked against it (tools/netcompare.py --exclude).
"""

# rev C parts that disappear, and rev C parts whose pins get new nets
REMOVED = {'L2', 'L3', 'C4', 'C5', 'C6', 'C7', 'C8', 'C9'}
REPLACED = {'U3', 'U4', 'U5', 'U6', 'U7', 'L1', 'C1', 'C2', 'C3', 'R1'}

G = 'GND'
NETS = {
    # ---- DC/DC 1393ЕУ014 (АЕНВ.431420.153ТУ, specification v2.1 of the maker):
    # 1-3 VIN, 4 BST, 5/11/12 GND, 6 CL, 7 ON_OFF, 8 EXT, 9 SS, 10 FB, 13 PG,
    # 14-15 SW, 16 NC.  ON_OFF left open = enabled.
    'U3': {'1': 'VIN_PROT', '2': 'VIN_PROT', '3': 'VIN_PROT', '4': 'BUCK_BST', '5': G,
           '6': 'BUCK_CL', '7': 'unconnected-(U3-ON_OFF)', '8': 'BUCK_EXT', '9': 'BUCK_SS',
           '10': 'BUCK_FB', '11': G, '12': G, '13': 'MCU_RESETn', '14': 'BUCK_SW',
           '15': 'BUCK_SW', '16': 'unconnected-(U3-NC)'},
    'C1': {'1': 'VIN_PROT', '2': G},           # 10 мкФ 25 В
    'C112': {'1': 'VIN_PROT', '2': G},         # 10 мкФ 25 В
    'C3': {'1': 'VIN_PROT', '2': G},           # 0,1 мкФ
    'R101': {'1': 'VIN_PROT', '2': 'BUCK_CL'},  # current limit, 909 Ом -> ~1,6 А
    'R109': {'1': 'VIN_PROT', '2': 'BUCK_SS'},  # soft start 100 кОм
    'C111': {'1': 'BUCK_SS', '2': G},          # soft start 0,22 мкФ
    'C101': {'1': 'BUCK_BST', '2': 'BUCK_SW'},  # bootstrap 0,1 мкФ
    'D101': {'1': 'BUCK_SW', '2': G},          # Schottky: 1 = cathode
    'L1': {'1': 'BUCK_SW', '2': '+5V'},        # 56 мкГн
    'R102': {'1': '+5V', '2': 'BUCK_FB'},      # 40,2 кОм
    'R103': {'1': 'BUCK_FB', '2': G},          # 10,0 кОм
    'R104': {'1': 'BUCK_EXT', '2': 'BUCK_CC'},  # 10 кОм  ("универсальная" коррекция)
    'C102': {'1': 'BUCK_CC', '2': G},          # 0,1 мкФ
    'C2': {'1': '+5V', '2': G},                # 22 мкФ ceramic
    'C103': {'1': '+5V', '2': G},              # 100 мкФ tantalum
    'C104': {'1': '+5V', '2': G},              # 0,1 мкФ
    # ---- LDO 1158ЕН3.3ВХ: 1 IN, 2 GND, 3 OUT
    'U4': {'1': '+5V', '2': G, '3': '+3V3'},
    'C105': {'1': '+5V', '2': G},              # 1 мкФ input
    'C106': {'1': '+3V3', '2': G},             # 10 мкФ tantalum output
    'C107': {'1': '+3V3', '2': G},             # 0,1 мкФ
    # ---- LDO 1325ЕН1.8У: 1 OUT, 2 IN, 3 GND
    'U5': {'1': '+1V8', '2': '+5V', '3': G},
    'C108': {'1': '+5V', '2': G},
    'C109': {'1': '+1V8', '2': G},             # 10 мкФ tantalum output
    'C110': {'1': '+1V8', '2': G},
    # ---- +3,3 V present -> MCU (was the PG wired-AND of the DC/DCs)
    'R1': {'1': '+3V3', '2': 'DDS_PGOOD'},
    # ---- 1564АП3 (74HC241) from +3,3 V. Group 1: 1OE# = 1, A 2/4/6/8 -> Y 18/16/14/12
    'U6': {'1': 'OE_IN3',
           '2': 'SCK_IN3', '18': 'SCK_BUF',
           '4': 'MOSI_IN3', '16': 'MOSI_BUF',
           '6': 'CS_IN3', '14': 'CS_BUF',
           '8': 'RST_IN3', '12': 'RST_BUF',
           # group 2 unused: 2OE = 19 low (outputs off), inputs to GND
           '19': G, '11': G, '13': G, '15': G, '17': G,
           '9': 'unconnected-(U6-2Y1)', '7': 'unconnected-(U6-2Y2)',
           '5': 'unconnected-(U6-2Y3)', '3': 'unconnected-(U6-2Y4)',
           '10': G, '20': '+3V3'},
    'R105': {'1': 'MCU_SCK', '2': 'SCK_IN3'},    # 1 кОм series (5 V -> 3,3 V input)
    'R106': {'1': 'MCU_MOSI', '2': 'MOSI_IN3'},
    'R107': {'1': 'MCU_CS', '2': 'CS_IN3'},
    'R108': {'1': 'MCU_DDS_RESET', '2': 'RST_IN3'},
    'R110': {'1': 'BUF_OEn', '2': 'OE_IN3'},     # 1OE#: same 1 кОм limiting (PB5 may drive 5 V)
    # ---- 1594ТЛ2Т (74ACT14) from +5 V: SDO_BUF -> 1 -> 2 -> 3 -> 4 -> MCU_MISO
    'U7': {'1': 'SDO_BUF', '2': 'SDO_INV', '3': 'SDO_INV', '4': 'MCU_MISO',
           '5': G, '9': G, '11': G, '13': G,
           '6': 'unconnected-(U7-3Y)', '8': 'unconnected-(U7-4Y)',
           '10': 'unconnected-(U7-5Y)', '12': 'unconnected-(U7-6Y)',
           '7': G, '14': '+5V'},
}

NEW_KEYS = sorted(k for k in NETS if k not in REPLACED)


def apply(net):
    """Edit the (key, pin) -> net map built from the rev C netlist in place."""
    for k in list(net):
        if k[0] in REMOVED or k[0] in REPLACED:
            del net[k]
    for key, pins in NETS.items():
        for pin, n in pins.items():
            net[(key, pin)] = n
    return net
