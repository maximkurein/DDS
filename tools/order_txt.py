#!/usr/bin/env python3
"""Order request texts (RU + EN) for a PCB-only fabrication order (run with KiCad's python).

    python3 tools/order_txt.py <board.kicad_pcb> <order_params.json> <out_dir> [--qty N] [--contact "name, phone"]

Writes <out_dir>/ORDER_RU.txt and ORDER_EN.txt to be sent together with the Gerber zip.
Taken from the board itself: project name, size, layer count, thickness.
Taken from <order_params.json> (edit it, or override on the command line): quantity,
finish, copper, mask/silk colour, stack-up, impedance wishes, stencil, contact.
"production_ready": false in the params puts a "not for production" note at the top.
"""
import argparse, json, os, sys
import pcbnew


def board_facts(path):
    b = pcbnew.LoadBoard(path)
    bb = b.GetBoardEdgesBoundingBox()
    def size(iu):                    # the bounding box includes the Edge.Cuts line width
        mm = pcbnew.ToMM(iu)
        return round(mm) if abs(mm - round(mm)) < 0.25 else round(mm, 1)
    return {
        'w': size(bb.GetWidth()),
        'h': size(bb.GetHeight()),
        'layers': b.GetCopperLayerCount(),
        'thick': round(pcbnew.ToMM(b.GetDesignSettings().GetBoardThickness()), 2),
    }


def num(x):
    s = f'{x:g}'
    return s


RU = {
    'title': 'ЗАКАЗ ПЕЧАТНЫХ ПЛАТ — {name} (текст для отправки посреднику)',
    'note': ('[ПЕРЕД ОТПРАВКОЙ: {note} Эту пометку в скобках перед отправкой удалить '
             '(или поставить "production_ready": true в {params}).]'),
    'hello': 'Здравствуйте! Прошу рассчитать и изготовить печатные платы по Gerber-файлам.',
    's1': '1. ФАЙЛЫ',
    'archive': 'Архив:  {zip}  (Gerber RS-274X, сверловка Excellon в мм)',
    'contents': ('В архиве: медь, паяльная маска, шелкография, паста обеих сторон; контур (Edge_Cuts);\n'
                 '          отверстия PTH и NPTH (.drl); файл задания (.gbrjob).'),
    'paste': 'Паста (F_Paste, B_Paste) нужна только для трафарета; если трафарет не заказывается — не учитывать.',
    's2': '2. ПАРАМЕТРЫ ПЛАТЫ',
    'rows': [
        ('Количество', '{qty} шт.'),
        ('Слоёв', '{layers}'),
        ('Размер', '{w} x {h} мм (одиночная плата, без панелизации)'),
        ('Толщина', '{thick} мм'),
        ('Стек (по возможности)', '{stackup}'),
        ('Медь', '{copper}'),
        ('Покрытие площадок', '{finish}'),
        ('Паяльная маска', '{mask}'),
        ('Шелкография', '{silk}'),
        ('Мин. дорожка/зазор', '{track}'),
        ('Мин. отверстие', '{via}'),
        ('Закрытие переходных', '{tenting}'),
        ('Край платы', 'прямоугольный контур, фрезеровка, без V-реза и скрайбирования'),
        ('Контроль импеданса', '{impedance}'),
        ('Номер заказа на плате', '{ordernum}'),
    ],
    's3': '3. ЧТО НЕ ТРЕБУЕТСЯ',
    'not_req': ('Монтаж, закупка компонентов, правка Gerber и трассировка — не нужны. Только изготовление плат.\n'
                '   Файлы BOM/CPL не прикладываются (монтаж выполняется самостоятельно).'),
    's4': '4. ВОПРОСЫ К ВАМ',
    'q': ['Стоимость и срок для указанных параметров и количества.',
          'Можно ли добавить трафарет ({stencil}) в этот заказ, и его цена?',
          'Возможен ли контроль импеданса, доплата?',
          'Нужен ли аудит файлов до расчёта и что именно проверяется?',
          'Фотоотчёт перед отправкой: прошу.',
          'Способ и срок доставки.'],
    'bye': 'С уважением,',
}
EN = {
    'title': 'PCB FABRICATION ORDER - {name} (text to send to the intermediary / fab house)',
    'note': ('[BEFORE SENDING: {note} Delete this bracketed note before sending '
             '(or set "production_ready": true in {params}).]'),
    'hello': 'Hello! Please quote and manufacture printed circuit boards from the attached Gerber files.',
    's1': '1. FILES',
    'archive': 'Archive:  {zip}  (Gerber RS-274X, Excellon drill in mm)',
    'contents': ('Contents: copper, solder mask, silkscreen and paste for both sides; board outline (Edge_Cuts);\n'
                 '             PTH and NPTH drill files (.drl); job file (.gbrjob).'),
    'paste': 'Paste layers (F_Paste, B_Paste) are only for a stencil; ignore them if no stencil is ordered.',
    's2': '2. BOARD SPECIFICATION',
    'rows': [
        ('Quantity', '{qty} pcs'),
        ('Layers', '{layers}'),
        ('Dimensions', '{w} x {h} mm (single board, no panelization)'),
        ('Thickness', '{thick} mm'),
        ('Stack-up (if possible)', '{stackup}'),
        ('Copper weight', '{copper}'),
        ('Surface finish', '{finish}'),
        ('Solder mask', '{mask}'),
        ('Silkscreen', '{silk}'),
        ('Min track / spacing', '{track}'),
        ('Min hole', '{via}'),
        ('Via covering', '{tenting}'),
        ('Board edge', 'rectangular routed outline, no V-cut, no scoring'),
        ('Impedance control', '{impedance}'),
        ('Order number on the PCB', '{ordernum}'),
    ],
    's3': '3. NOT REQUIRED',
    'not_req': ('Assembly, component sourcing, Gerber editing and routing are not needed. PCB fabrication only.\n'
                '   BOM/CPL files are not attached (assembly is done by the customer).'),
    's4': '4. QUESTIONS',
    'q': ['Price and lead time for the specification and quantity above.',
          'Can a stencil ({stencil}) be added to the same order, and at what price?',
          'Is impedance control available, and at what extra cost?',
          'Is a file audit done before the quotation, and what does it check?',
          'Please send a photo report before shipping.',
          'Shipping method and time.'],
    'bye': 'Best regards,',
}


def render(L, lang, v, p):
    out = [L['title'].format(**v)]
    out.append('=' * len(out[0]))
    out.append('')
    note = p.get('note_' + lang)
    if not p.get('production_ready', False) and note:
        out += [L['note'].format(note=note, params=p['_params_name']), '']
    out += [L['hello'], '', L['s1'], '   ' + L['archive'].format(**v), '   ' + L['contents'],
            '   ' + L['paste'], '', L['s2']]
    w = max(len(k) for k, _ in L['rows'])
    for k, t in L['rows']:
        out.append(f'   {k:<{w}} : ' + t.format(**v))
    out += ['', L['s3'], '   ' + L['not_req'], '', L['s4']]
    out += ['   - ' + q.format(**v) for q in L['q']]
    out += ['', L['bye'], v['contact'], '']
    return '\n'.join(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('pcb'); ap.add_argument('params'); ap.add_argument('out_dir')
    ap.add_argument('--qty', type=int); ap.add_argument('--contact')
    o = ap.parse_args()
    p = json.load(open(o.params, encoding='utf-8'))
    p['_params_name'] = os.path.basename(o.params)
    f = board_facts(o.pcb)
    name = os.path.splitext(os.path.basename(o.pcb))[0]
    v = {**f, 'zip': f'{name}_gerber.zip',
         'qty': o.qty or p.get('quantity') or '____',
         'contact': o.contact or p.get('contact') or '(имя, контакт / name, contact)'}
    v['layers'] = p.get('layers', f['layers'])
    for lang in ('ru', 'en'):
        v['name'] = p.get(f'title_{lang}', name)
        dec = (lambda x: num(x).replace('.', ',')) if lang == 'ru' else num
        v['w'], v['h'] = dec(f['w']), dec(f['h'])
        v['thick'] = dec(p.get('thickness', f['thick']))
        for k in ('stackup', 'copper', 'finish', 'mask', 'silk', 'track', 'via', 'tenting',
                  'impedance', 'ordernum', 'stencil'):
            v[k] = p[f'{k}_{lang}']
        text = render(RU if lang == 'ru' else EN, lang, v, p)
        path = os.path.join(o.out_dir, f'ORDER_{lang.upper()}.txt')
        with open(path, 'w', encoding='utf-8', newline='\n') as fh:
            fh.write(text)
        print('order_txt:', path)


if __name__ == '__main__':
    main()
