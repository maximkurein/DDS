#!/usr/bin/env python3
"""Compare two KiCad XML netlists by connectivity.

Nets are compared as sets of (reference, pin) pairs, so a pure redraw or
renaming of nets/sheets does not count as a change. An optional reference
map (JSON {"old": "new"}) allows comparing across a designator renumbering.
Exit code 0 = identical connectivity and component set.
"""
import json, sys, argparse
import xml.etree.ElementTree as ET

def _map(refmap, ref, pin=None):
    """refmap value: "NEWREF" or {"ref": "NEWREF", "pins": {"old": "new"}}."""
    m = refmap.get(ref, ref) if refmap else ref
    if isinstance(m, dict):
        return m['ref'], (m.get('pins', {}).get(pin, pin) if pin is not None else None)
    return m, pin


def load(path, refmap=None, exclude=()):
    """exclude: references (before mapping) whose pins are left out of the
    comparison - parts deliberately changed by an ECO."""
    root = ET.parse(path).getroot()
    comps = {}
    for c in root.iter('comp'):
        if c.get('ref') in exclude:
            continue
        ref, _ = _map(refmap, c.get('ref'))
        comps.setdefault(ref, (c.findtext('value') or '').strip())
    nets = {}
    for n in root.iter('net'):
        nodes = frozenset(_map(refmap, x.get('ref'), x.get('pin')) for x in n.iter('node')
                          if x.get('ref') not in exclude)
        if nodes:
            nets[nodes] = n.get('name')
    return comps, nets


def load_expected(path):
    """JSON {net: [[ref, pin], ...]} written by tools/dds_build.py."""
    d = json.load(open(path, encoding='utf-8'))
    nets = {frozenset((r, p) for r, p in nodes): name for name, nodes in d.items() if nodes}
    comps = {r: '' for nodes in d.values() for r, _ in nodes}
    return comps, nets

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('a'); ap.add_argument('b')
    ap.add_argument('--refmap', help='JSON old->new designator (and pin) map applied to A')
    ap.add_argument('--ignore-values', action='store_true')
    ap.add_argument('--exclude-a', default='', help='comma list of A references to leave out')
    ap.add_argument('--exclude-b', default='', help='comma list of B references to leave out')
    ap.add_argument('--expected', action='store_true',
                    help='A is a JSON {net: [[ref, pin]]} (intended netlist), values ignored')
    o = ap.parse_args()
    rm = json.load(open(o.refmap, encoding='utf-8')) if o.refmap else None
    xa = {r for r in o.exclude_a.split(',') if r}
    xb = {r for r in o.exclude_b.split(',') if r}
    if o.expected:
        ca, na = load_expected(o.a)
        o.ignore_values = True
    else:
        ca, na = load(o.a, rm, xa)
    cb, nb = load(o.b, None, xb)
    ok = True
    if set(ca) != set(cb):
        ok = False
        print('components only in A:', sorted(set(ca) - set(cb)))
        print('components only in B:', sorted(set(cb) - set(ca)))
    if not o.ignore_values:
        for r in sorted(set(ca) & set(cb)):
            if ca[r] != cb[r]:
                ok = False; print(f'value differs {r}: {ca[r]!r} -> {cb[r]!r}')
    sa, sb = set(na), set(nb)
    for s in sorted(sa - sb, key=lambda s: na[s]):
        ok = False; print('net only in A:', na[s], sorted(s)[:8])
    for s in sorted(sb - sa, key=lambda s: nb[s]):
        ok = False; print('net only in B:', nb[s], sorted(s)[:8])
    print(f'A: {len(ca)} comps, {len(na)} nets; B: {len(cb)} comps, {len(nb)} nets ->',
          'IDENTICAL' if ok else 'DIFFERENT')
    sys.exit(0 if ok else 1)

if __name__ == '__main__':
    main()
