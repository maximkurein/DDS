"""Revision F additions; stable keys do not renumber the revision E drawing."""
NEW_REFS = {'J101': 'X3', 'TP101': 'TP1', 'TP102': 'TP2'}
MCU_PINS = {'54': 'JTAG_TDI', '55': 'JTAG_TDO', '56': 'JTAG_TMS',
            '57': 'JTAG_TCK', '24': 'MCU_BQ1_RECOVERY'}
NETS = {
    'J101': {'1': 'JTAG_TCK', '2': 'GND', '3': 'JTAG_TDO', '4': '+5V',
             '5': 'JTAG_TMS', '6': 'MCU_RESETn', '7': 'unconnected-X3-7',
             '8': 'unconnected-X3-8', '9': 'JTAG_TDI', '10': 'GND'},
    'TP101': {'1': 'MCU_BQ1_RECOVERY'},
    'TP102': {'1': 'GND'},
}


def apply(net):
    for p, n in MCU_PINS.items():
        net[('U2', p)] = n
    for k, pins in NETS.items():
        for p, n in pins.items():
            net[(k, p)] = n
