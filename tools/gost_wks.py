"""ESKD drawing sheet (GOST 2.104): frame, main title block form 1 on the
first sheet, form 2a on the following sheets, additional graphs 19-26.

Title block fields are taken from the KiCad title block of each sheet:
  ${COMMENT1} designation (e.g. "ХХХХ.ХХХХХХ.ХХХ Э3"), ${TITLE} product name,
  ${COMMENT2} document name, ${COMMENT3..7} Разраб./Пров./Т.контр./Н.контр./Утв.,
  ${COMPANY} organisation, ${COMMENT8} free field (material box),
  ${COMMENT9} sheet name (shown above the title block of follow-up sheets).
"""

THICK, THIN = 0.5, 0.25


class W:
    def __init__(self):
        self.items = []

    def line(self, x1, y1, x2, y2, corner, w=THIN, opt=None):
        o = f' (option {opt})' if opt else ''
        self.items.append(f'(line (name "") (start {x1:g} {y1:g} {corner}) (end {x2:g} {y2:g} {corner})'
                          f' (linewidth {w}){o})')

    def rect(self, x1, y1, x2, y2, c1, c2, w=THICK, opt=None):
        o = f' (option {opt})' if opt else ''
        self.items.append(f'(rect (name "") (start {x1:g} {y1:g} {c1}) (end {x2:g} {y2:g} {c2})'
                          f' (linewidth {w}){o})')

    def text(self, s, x, y, corner, size=2.5, justify='center', rot=0, opt=None, maxlen=None):
        o = f' (option {opt})' if opt else ''
        r = f' (rotate {rot})' if rot else ''
        ml = f' (maxlen {maxlen:g})' if maxlen else ''
        j = f' (justify {justify})' if justify else ''
        s = s.replace('"', '\\"')
        self.items.append(f'(tbtext "{s}" (name "") (pos {x:g} {y:g} {corner}){o}{r}'
                          f' (font (face "Arial") (size {size:g} {size:g})){j}{ml})')


def stamp_form1(w):
    # stamp-local (u from left edge, v from bottom) -> rbcorner coordinates
    X = lambda u: 5 + (185 - u)
    Y = lambda v: 5 + v
    rb, o = 'rbcorner', 'page1only'
    L = lambda u1, v1, u2, v2, wd=THIN: w.line(X(u1), Y(v1), X(u2), Y(v2), rb, wd, o)
    # outer border of the stamp (top and left lines; right/bottom are the frame)
    L(0, 55, 185, 55, THICK); L(0, 0, 0, 55, THICK)
    # left part
    for u in (7, 17, 40, 55):
        L(u, 30, u, 55, THICK if u in (7, 17, 40, 55) else THIN)
    for u in (17, 40, 55):
        L(u, 0, u, 30, THICK)
    L(65, 0, 65, 55, THICK)
    for v in range(5, 55, 5):
        L(0, v, 65, v, THICK if v in (30, 35) else THIN)
    # designation, name, material, right column
    L(65, 40, 185, 40, THICK)
    L(65, 15, 185, 15, THICK)
    L(135, 0, 135, 40, THICK)
    L(135, 35, 185, 35, THICK)
    L(135, 20, 185, 20, THICK)
    L(150, 20, 150, 40, THICK)
    L(167, 20, 167, 40, THICK)
    L(140, 20, 140, 35, THIN); L(145, 20, 145, 35, THIN)
    L(155, 15, 155, 20, THICK)
    T = lambda s, u, v, size=2.5, j='center', ml=None: w.text(s, X(u), Y(v), rb, size, j, opt=o, maxlen=ml)
    for s, u in (('Изм.', 3.5), ('Лист', 12), ('№ докум.', 28.5), ('Подп.', 47.5), ('Дата', 60)):
        T(s, u, 32.5, 2.0)
    for s, v, var in (('Разраб.', 27.5, '${COMMENT3}'), ('Пров.', 22.5, '${COMMENT4}'),
                      ('Т.контр.', 17.5, '${COMMENT5}'), ('Н.контр.', 7.5, '${COMMENT6}'),
                      ('Утв.', 2.5, '${COMMENT7}')):
        T(s, 1, v, 2.0, 'left')
        T(var, 18, v, 2.0, 'left', 21)
    T('${COMMENT1}', 125, 47.5, 5.0, ml=115)
    T('${TITLE}', 100, 31, 3.5, ml=66)
    T('${COMMENT2}', 100, 20, 2.5, ml=66)
    T('${COMMENT8}', 100, 7.5, 2.0, ml=66)
    T('Лит.', 142.5, 37.5, 2.0); T('Масса', 158.5, 37.5, 2.0); T('Масштаб', 176, 37.5, 2.0)
    T('Лист ${#}', 136, 17.5, 2.0, 'left'); T('Листов ${##}', 156, 17.5, 2.0, 'left')
    T('${COMPANY}', 160, 7.5, 2.5, ml=48)


def stamp_form2a(w):
    X = lambda u: 5 + (185 - u)
    Y = lambda v: 5 + v
    rb, o = 'rbcorner', 'notonpage1'
    L = lambda u1, v1, u2, v2, wd=THICK: w.line(X(u1), Y(v1), X(u2), Y(v2), rb, wd, o)
    L(0, 15, 185, 15); L(0, 0, 0, 15)
    for u in (7, 17, 40, 55, 65):
        L(u, 0, u, 15)
    L(0, 5, 65, 5); L(0, 10, 65, 10, THIN)
    L(175, 0, 175, 15); L(175, 8, 185, 8, THIN)
    T = lambda s, u, v, size=2.0, j='center': w.text(s, X(u), Y(v), rb, size, j, opt=o)
    for s, u in (('Изм.', 3.5), ('Лист', 12), ('№ докум.', 28.5), ('Подп.', 47.5), ('Дата', 60)):
        T(s, u, 2.5)
    T('${COMMENT1}', 120, 7.5, 5.0)
    T('Лист', 180, 11.5)
    T('${#}', 180, 4, 2.5)
    T('${COMMENT9}', 185, 17.5, 2.5, 'right')


def write_wks(path):
    w = W()
    # outer frame: 20 mm left margin, 5 mm elsewhere
    w.rect(20, 5, 5, 5, 'ltcorner', 'rbcorner', THICK)
    stamp_form1(w)
    stamp_form2a(w)
    # graph 26: designation rotated 180 deg at the top-left corner
    w.rect(20, 5, 90, 19, 'ltcorner', 'ltcorner', THICK)
    w.text('${COMMENT1}', 55, 12, 'ltcorner', 3.5, 'center', rot=180, maxlen=66)
    # graphs 19-23 in the left margin (lbcorner: x right, y up)
    lb = 'lbcorner'
    bounds = [5, 30, 65, 90, 115, 150]
    names = ['Инв. № подл.', 'Подп. и дата', 'Взам. инв. №', 'Инв. № дубл.', 'Подп. и дата']
    w.rect(8, 5, 20, 150, lb, lb, THICK)
    w.line(13, 5, 13, 150, lb, THICK)
    for y in bounds[1:-1]:
        w.line(8, y, 20, y, lb, THICK)
    for (y1, y2), s in zip(zip(bounds, bounds[1:]), names):
        w.text(s, 10.5, (y1 + y2) / 2, lb, 2.0, 'center', rot=90)
    # graphs 24/25 on the first sheet
    w.rect(8, 160, 20, 280, lb, lb, THICK, 'page1only')
    w.line(13, 160, 13, 280, lb, THICK, 'page1only')
    w.line(8, 220, 20, 220, lb, THICK, 'page1only')
    w.text('Справ. №', 10.5, 190, lb, 2.0, 'center', rot=90, opt='page1only')
    w.text('Перв. примен.', 10.5, 250, lb, 2.0, 'center', rot=90, opt='page1only')
    w.text('${COMMENT1}', 16.5, 250, lb, 2.0, 'center', rot=90, opt='page1only', maxlen=55)
    # below the frame
    w.text('Копировал', 120, 2.5, 'rbcorner', 2.0)
    w.text('Формат ${PAPER}', 40, 2.5, 'rbcorner', 2.0)
    txt = ('(kicad_wks (version 20231118) (generator "dds_build") (generator_version "9.0")\n'
           '  (setup (textsize 2.5 2.5) (linewidth 0.25) (textlinewidth 0.25)\n'
           '    (left_margin 0) (right_margin 0) (top_margin 0) (bottom_margin 0))\n  '
           + '\n  '.join(w.items) + '\n)\n')
    open(path, 'w', encoding='utf-8').write(txt)


if __name__ == '__main__':
    import sys
    write_wks(sys.argv[1])
