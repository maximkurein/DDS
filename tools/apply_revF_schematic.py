"""One-time local revision E -> F ECO. Keeps untouched schematic objects intact.

Generate a candidate in pcb_route/revF_work/generated first. Only the additions,
X1/L8 symbols and title blocks are applied to the working schematic.
"""
import re
import shutil
import uuid
from pathlib import Path
from sch2vsdx import parse, kid, kids


def blocks(text):
    """Top-level children with offsets; quoted parentheses are not delimiters."""
    depth, quoted, escape, start = 0, False, False, None
    for i, c in enumerate(text):
        if quoted:
            if escape:
                escape = False
            elif c == '\\':
                escape = True
            elif c == '"':
                quoted = False
            continue
        if c == '"':
            quoted = True
        elif c == '(':
            if depth == 1:
                start = i
            depth += 1
        elif c == ')':
            depth -= 1
            if depth == 1 and start is not None:
                yield start, i + 1, parse(text[start:i + 1]), text[start:i + 1]
                start = None


def prop(n, key):
    return next((x[2] for x in kids(n, 'property') if x[1] == key), '')


def sig(n):
    if n[0] == 'wire':
        return ('wire', tuple(sorted(tuple(x[1:]) for x in kids(kid(n, 'pts'), 'xy'))))
    if n[0] == 'no_connect':
        return ('no_connect', tuple(kid(n, 'at')[1:]))
    if n[0] in ('label', 'global_label', 'text'):
        return (n[0], n[1], tuple(kid(n, 'at')[1:]))
    if n[0] == 'symbol':
        if prop(n, 'Reference').startswith('#'):
            return ('power', prop(n, 'Value'), tuple(kid(n, 'at')[1:]))
        return ('symbol', prop(n, 'Reference'))
    return (n[0],)


def fresh_ids(raw):
    return re.sub(r'\(uuid "([^"]+)"\)',
                  lambda m: '(uuid "' + str(uuid.uuid5(uuid.NAMESPACE_URL, 'DDS:revF:' + m[1])) + '")', raw)


def main():
    root = Path(__file__).resolve().parent.parent
    gen = root / 'pcb_route/revF_work/generated'
    for file in ('DDS_48_52.kicad_sch', '02_controller.kicad_sch', '03_power.kicad_sch'):
        p = root / file
        text = p.read_text(encoding='utf-8')
        assert '(rev "E")' in text, 'ECO already applied or wrong base'
        old, new = list(blocks(text)), list(blocks((gen / file).read_text(encoding='utf-8')))
        changes, additions = [], []
        replace_refs = {'X1'} if file.startswith('02') else {'L8'} if file.startswith('03') else set()
        for a, b, n, raw in old:
            if n[0] == 'title_block':
                raw = raw.replace('(rev "E")', '(rev "F")').replace('2026-10-30', '2026-11-05')
                changes.append((a, b, raw))
            elif n[0] == 'symbol' and prop(n, 'Reference') in replace_refs:
                replacement = next(t for _, _, nn, t in new if nn[0] == 'symbol' and prop(nn, 'Reference') == prop(n, 'Reference'))
                assert kid(parse(replacement), 'uuid') == kid(n, 'uuid')
                changes.append((a, b, replacement))
        if file.startswith('02'):
            # Existing geometry must be identical except five removed NC markers.
            kinds = {'wire', 'no_connect', 'label', 'global_label', 'text', 'symbol'}
            olds = {sig(n): (a, b, n, t) for a, b, n, t in old if n[0] in kinds}
            news = {sig(n): (a, b, n, t) for a, b, n, t in new if n[0] in kinds}
            removed = set(olds) - set(news)
            assert len(removed) == 5 and all(s[0] == 'no_connect' for s in removed), removed
            for s in removed:
                a, b, _, _ = olds[s]
                changes.append((a, b, ''))
            power_index = max(int(prop(n, 'Reference')[4:]) for _, _, n, _ in old
                              if n[0] == 'symbol' and prop(n, 'Reference').startswith('#PWR'))
            for s in sorted(news.keys() - olds.keys()):
                n, raw = news[s][2:]
                # New non-power component IDs must match the generator's stable IDs.
                if not (n[0] == 'symbol' and not prop(n, 'Reference').startswith('#')):
                    raw = fresh_ids(raw)
                if s[0] == 'power':
                    power_index += 1
                    raw = raw.replace(prop(n, 'Reference'), f'#PWR{power_index}')
                additions.append(raw)
            # Add only the two new library symbols to the existing embedded library.
            a, b, n, raw = next(v for v in old if v[2][0] == 'lib_symbols')
            nlib = next(v[3] for v in new if v[2][0] == 'lib_symbols')
            added = [t for _, _, nn, t in blocks(nlib) if nn[0] == 'symbol' and nn[1] in ('DDS:X_1', 'DDS:X_10')]
            assert len(added) == 2
            changes.append((a, b, raw.rstrip()[:-1] + '\n' + '\n'.join(added) + '\n)'))
        for a, b, t in sorted(changes, reverse=True):
            text = text[:a] + t + text[b:]
        if additions:
            text = text.rstrip()[:-1] + '\n' + '\n'.join(sorted(additions)) + '\n)\n'
        p.write_text(text, encoding='utf-8')
        print(file, 'replaced/removed', len(changes), 'added', len(additions))
    shutil.copyfile(gen / 'lib/DDS.kicad_sym', root / 'lib/DDS.kicad_sym')
    for p in (gen / 'verification').iterdir():
        shutil.copyfile(p, root / 'verification' / p.name)


if __name__ == '__main__':
    main()
