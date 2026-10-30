#!/usr/bin/env python3
"""Перечень элементов (ПЭ3) in Word, generated from the KiCad netlist.

    python3 tools/pe3_docx.py <netlist.xml> <template.docx> <out.docx> [--pages N]
    python3 tools/pe3_docx.py --make-template <sample.docx> <form.docx>

The KiCad schematic is the single source of truth: every row comes from the
component fields exported by `kicad-cli sch export netlist`.  The Word
template (form 1 / 1а, ГОСТ 2.104) is the customer's sample "Плата
контроллера.docx"; its table rows are replaced, the stamp texts are set to
this project's data, the last page (лист регистрации изменений) is kept.
Rules: docs/Правила_оформления_Э3_ПЭ3.md.
"""
import copy
import re
import sys
import xml.etree.ElementTree as ET
from collections import OrderedDict

import docx
from docx.oxml.ns import qn

DESIGNATION = 'ХХХХ.ХХХХХХ.ХХХ ПЭ3'
TITLE = 'Синтезатор DDS 47–51 МГц'

# ГОСТ 2.710 letter codes -> group heading (plural) / single name
GROUPS = OrderedDict([
    ('C', ('Конденсаторы', 'Конденсатор')),
    ('D', ('Микросхемы', 'Микросхема')),
    ('F', ('Предохранители', 'Предохранитель')),
    ('G', ('Генераторы', 'Генератор')),
    ('L', ('Индуктивности', 'Индуктивность')),
    ('R', ('Резисторы', 'Резистор')),
    ('S', ('Переключатели', 'Переключатель')),
    ('VD', ('Диоды', 'Диод')),
    ('X', ('Соединители', 'Соединитель')),
    ('XW', ('Соединители коаксиальные', 'Соединитель коаксиальный')),
])

TU = {  # domestic ICs: ТУ from the customer's data / maker's specification
    '1508ПЛ8Т': '1508ПЛ8Т АЕЯР.431320.596ТУ',
    '1887ВЕ7Т': '1887ВЕ7Т АЕЯР.431280.910ТУ',
    '1393ЕУ014': '1393ЕУ014 АЕНВ.431420.153ТУ',
    '1158ЕН3.3ВХ': '1158ЕН3.3ВХ АЕЯР.431420.773ТУ',
    '1564АП3У2': '1564АП3У2 ЭП АЕЯР.431200.424-19ТУ',
}
NO_TU = {'1325ЕН1.8У', '1594ТЛ2Т'}          # ТУ not available yet


def add_note(note, text):
    return f'{note}. {text}' if note else text


def size_of(fp):
    m = re.search(r'_(0402|0603|0805|1206|1210)_', fp + '_')
    return m.group(1) if m else ''


def num(v):
    return ('%g' % v).replace('.', ',')


def value_ru(v, kind):
    m = re.match(r'^([\d.]+)([pnumkMR]?)(.*)$', v)
    if not m:
        return v
    x, p = float(m.group(1)), m.group(2)
    if kind == 'R':
        if p in ('R', ''):
            return f'{num(x)} Ом'
        return f'{num(x)} {"кОм" if p == "k" else "МОм"}'
    if kind == 'C':
        f = x * {'p': 1e-12, 'n': 1e-9, 'u': 1e-6}.get(p, 1)
        if f >= 1e-8:                                   # 0,01 мкФ and above in мкФ
            return f'{num(round(f * 1e6, 6))} мкФ'
        return f'{num(round(f * 1e12, 3))} пФ'
    if kind == 'L':
        return f'{num(x)} {"нГн" if p == "n" else "мкГн"}'
    return v


def tol(t):
    return f' ±{t}' if t else ''


def describe(c):
    """(group letter, name text, note) for a component."""
    ref, val, fp, f = c['ref'], c['value'], c['footprint'], c['fields']
    letter = re.match(r'[A-Z]+', ref).group(0)
    note = ''
    if c['dnp']:
        note = 'Не устанавл.'
    if letter == 'R':
        name = f'{size_of(fp)} – {f.get("Мощность", "0,063 Вт")} – {value_ru(val, "R")}{tol(f.get("Допуск"))}'
    elif letter == 'C':
        v = f.get('Напряжение', '').replace('V', ' В')
        if 'Tantalum' in fp:
            name = f'К53-65 – {v} – {value_ru(val, "C")} ±20 %'
            note = add_note(note, 'Типоразм. уточн.')
        else:
            name = f'{size_of(fp)} – {f.get("ТКЕ", "")} – {v} – {value_ru(val, "C")}{tol(f.get("Допуск"))}'
    elif letter == 'L':
        if 'ID1' in fp:
            name = f'Дроссель ИД1 – {value_ru(val, "L")}, Iнас ≥ 1,6 А'
        elif '@' in val:
            name = f'Фильтр помехоподавляющий {size_of(fp)} – 600 Ом/100 МГц, Rdc ≤ 0,1 Ом, 1 А'
        elif 'FNR4020' in fp:
            name = f'Дроссель 4020 – {value_ru(val, "L")}, Iнас ≥ 1,5 А'
        else:
            name = f'{size_of(fp)} – {value_ru(val, "L")}{tol(f.get("Допуск"))}, Q ≥ 30 (50 МГц)'
    elif letter == 'D':
        name = TU.get(val, val)
        if val == '1887ВЕ7Т':
            note = note or 'Корпус 4203.64-2'
        if val in NO_TU:
            note = add_note(note, 'ТУ уточн.')
    elif letter == 'G':
        name = f'{val} (480 МГц, LVDS)'
        note = 'Не устанавл. Не подтвержд.'
    elif letter == 'F':
        name = f'Предохранитель {size_of(fp)} – 0,5 А'
    elif letter == 'VD':
        name = f'Диод Шоттки {val} (SMA)' if re.match(r'[A-Z]', val) else f'Диод {val}'
        if not re.match(r'[A-Z]', val):
            note = add_note(note, 'ТУ уточн.')
    elif letter == 'S':
        name = {'DIP-1': 'Переключатель DIP, 1 секция', 'DIP-4': 'Переключатель DIP, 4 секции',
                'DIP-8': 'Переключатель DIP, 8 секций', '6x6': 'Кнопка тактовая 6×6 мм'}.get(val, val)
    elif letter == 'XW':
        name = 'Розетка SMA, торцевая (Samtec SMA-J-P-H-ST-EM1)'
    elif letter == 'X':
        name = {'PLS-2': 'Вилка PLS-2 (штыревая 1×2, шаг 2,54)',
                'PLD-6': 'Вилка PLD-6 (штыревая 2×3, шаг 2,54)'}.get(val, val)
    else:
        name = val
    if 'PLACEHOLDER' in fp:                     # land pattern not yet from the package drawing
        note = 'ТУ, корпус уточн.' if note == 'ТУ уточн.' else add_note(note, 'Корпус уточн.')
    return letter, name, note


def natural(ref):
    m = re.match(r'([A-Z]+)(\d+)', ref)
    return m.group(1), int(m.group(2))


def ref_ranges(refs):
    """['C1','C2','C3','C5'] -> 'C1–C3, C5' (ГОСТ 2.701: ranges of 3+)."""
    refs = sorted(refs, key=natural)
    out, run = [], [refs[0]]
    for r in refs[1:]:
        if natural(r)[0] == natural(run[-1])[0] and natural(r)[1] == natural(run[-1])[1] + 1:
            run.append(r)
        else:
            out.append(run); run = [r]
    out.append(run)
    parts = []
    for run in out:
        if len(run) >= 3:
            parts.append(f'{run[0]}–{run[-1]}')
        else:
            parts.extend(run)
    return ', '.join(parts)


def rows_from_netlist(path):
    root = ET.parse(path).getroot()
    comps = []
    for c in root.iter('comp'):
        fields = {f.get('name'): (f.text or '') for f in c.iter('field')}
        props = {p.get('name') for p in c.iter('property')}
        comps.append(dict(ref=c.get('ref'), value=c.findtext('value'), footprint=c.findtext('footprint') or '',
                          fields=fields, dnp='dnp' in props))
    by_group = OrderedDict((g, []) for g in GROUPS)
    for c in comps:
        letter, name, note = describe(c)
        by_group.setdefault(letter, []).append((c['ref'], name, note))
    rows = []                                   # (pos, name, qty, note, kind)
    for letter, items in by_group.items():
        if not items:
            continue
        items.sort(key=lambda x: natural(x[0]))
        # merge consecutive refs with identical name and note
        merged = []
        for ref, name, note in items:
            if merged and merged[-1][1] == name and merged[-1][2] == note and \
                    natural(ref)[1] == natural(merged[-1][0][-1])[1] + 1:
                merged[-1][0].append(ref)
            else:
                merged.append([[ref], name, note])
        distinct = {m[1] for m in merged}
        if rows:
            rows.append(('', '', '', '', 'blank'))
        plural, single = GROUPS.get(letter, (letter, letter))
        if len(distinct) > 1 or letter in ('C', 'R', 'D', 'L'):
            rows.append(('', plural, '', '', 'head'))
            for refs, name, note in merged:
                rows.append((ref_ranges(refs), name, str(len(refs)), note, 'item'))
        else:
            for refs, name, note in merged:
                full = name if name.split()[0] in ('Предохранитель', 'Диод', 'Переключатель', 'Кнопка',
                                                   'Розетка', 'Вилка') else f'{single} {name}'
                rows.append((ref_ranges(refs), full, str(len(refs)), note, 'item'))
    return rows, len(comps)


# ----------------------------------------------------------------- Word ---
def set_cell(cell, text, align=None, size=24):
    p = cell.paragraphs[0]
    for r in list(p.runs):
        r._r.getparent().remove(r._r)
    for extra in cell.paragraphs[1:]:
        extra._p.getparent().remove(extra._p)
    if align:
        p.alignment = {'center': 1, 'left': 0}[align]
    if text:
        r = p.add_run(text)
        r.font.size = docx.shared.Pt(size / 2)


def fill(template, out, rows, total_pages=None):
    d = docx.Document(template)
    t = d.tables[0]
    proto = copy.deepcopy(t.rows[1]._tr)              # an empty, formatted row
    for r in list(t.rows)[1:]:
        t._tbl.remove(r._tr)
    for pos, name, qty, note, kind in rows:
        tr = copy.deepcopy(proto)
        t._tbl.append(tr)
        row = t.rows[-1]
        c = row.cells
        set_cell(c[1], pos, 'center')
        if kind == 'head':
            set_cell(c[2], name, 'center')
        else:
            set_cell(c[2], name)
        set_cell(c[3], qty, 'center')
        set_cell(c[4], note, 'center', size=20)
    # a few empty rows to close the page
    for _ in range(2):
        t._tbl.append(copy.deepcopy(proto))
    # the change-registration sheet starts on a new page
    for p in d.paragraphs:
        if 'Лист регистрации изменений' in p.text:
            p.paragraph_format.page_break_before = True
            break
    # stamp texts
    repl = {'ГВАТ.687281.492 ПЭ3': DESIGNATION, 'Плата контроллера ТЛ-7': TITLE,
            'ХХХХ.ХХХХХХ.ХХХ ПЭ3': DESIGNATION, 'Наименование изделия': TITLE,
            'Чугунов': '', 'Трашков': ''}
    for s in d.sections:
        for part in (s.footer, s.first_page_footer, s.header, s.first_page_header):
            for el in part._element.iter(qn('w:t')):
                if el.text in repl:
                    el.text = repl[el.text]
                    if el.text == TITLE:                  # keep the title on one line
                        rpr = el.getparent().find(qn('w:rPr'))
                        if rpr is None:
                            rpr = el.getparent().makeelement(qn('w:rPr'), {})
                            el.getparent().insert(0, rpr)
                        for tag in ('w:sz', 'w:szCs'):
                            s = rpr.find(qn(tag))
                            if s is None:
                                s = rpr.makeelement(qn(tag), {}); rpr.append(s)
                            s.set(qn('w:val'), '24')
                elif total_pages and (el.text or '').strip() == '4' and (el.text or '').endswith('  '):
                    el.text = f'{total_pages}  '
    d.core_properties.title = f'{TITLE}. Перечень элементов'
    d.core_properties.author = ''
    d.core_properties.last_modified_by = ''
    d.save(out)
    scrub(out)


def scrub(path):
    """Remove the sample's company/manager from docProps/app.xml."""
    import zipfile, shutil, tempfile
    tmp = tempfile.mktemp(suffix='.docx')
    with zipfile.ZipFile(path) as zi, zipfile.ZipFile(tmp, 'w', zipfile.ZIP_DEFLATED) as zo:
        for it in zi.infolist():
            data = zi.read(it.filename)
            if it.filename == 'docProps/app.xml':
                s = data.decode('utf-8')
                s = re.sub(r'<Company>[^<]*</Company>', '<Company></Company>', s)
                s = re.sub(r'<Manager>[^<]*</Manager>', '<Manager></Manager>', s)
                data = s.encode('utf-8')
            zo.writestr(it, data)
    shutil.move(tmp, path)


def make_template(sample, out):
    """Blank form from the customer's sample: header row + one empty row,
    stamp texts of the other project removed."""
    d = docx.Document(sample)
    t = d.tables[0]
    for r in list(t.rows)[2:]:
        t._tbl.remove(r._tr)
    for c in t.rows[1].cells:
        set_cell(c, '')
    for s in d.sections:
        for part in (s.footer, s.first_page_footer):
            for el in part._element.iter(qn('w:t')):
                if el.text in ('ГВАТ.687281.492 ПЭ3', 'Плата контроллера ТЛ-7', 'Чугунов', 'Трашков'):
                    el.text = {'ГВАТ.687281.492 ПЭ3': 'ХХХХ.ХХХХХХ.ХХХ ПЭ3',
                               'Плата контроллера ТЛ-7': 'Наименование изделия'}.get(el.text, '')
    d.core_properties.author = ''
    d.core_properties.last_modified_by = ''
    d.core_properties.title = 'Перечень элементов (форма)'
    d.save(out)
    scrub(out)
    print('template ->', out)


def main(argv):
    if argv[0] == '--make-template':
        return make_template(argv[1], argv[2])
    net, template, out = argv[:3]
    pages = int(argv[argv.index('--pages') + 1]) if '--pages' in argv else None
    rows, n = rows_from_netlist(net)
    fill(template, out, rows, pages)
    items = sum(int(r[2]) for r in rows if r[4] == 'item')
    print(f'ПЭ3: {len(rows)} rows, {items} elements (netlist: {n} components) -> {out}')
    assert items == n, 'ПЭ3 quantity does not match the netlist'


if __name__ == '__main__':
    main(sys.argv[1:])
