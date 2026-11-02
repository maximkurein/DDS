# Раскладка проекта и инструменты

## Каталоги (как в DDS)

| Путь | Что | Правится руками? |
|---|---|---|
| `<proj>.kicad_pro`, `*.kicad_sch`, `<proj>.kicad_pcb` | проект KiCad | генераторами; руками — только в ручном режиме |
| `lib/` | символы, посадочные места `*.pretty`, рамка `GOST_E3.kicad_wks` | генераторами (`gost_symbols.py`, `make_footprints.py`) |
| `tools/` | генераторы, проверки, шаблоны | да — это код проекта |
| `tools/legacy/` | скрипты прошлых ревизий (для истории) | нет |
| `verification/` | эталоны: нетлист прошлой ревизии, заданный нетлист, `refmap`, `keymap`, `eco_*.json` | нетлисты и JSON — нет; `eco.py` — да |
| `docs/` | `design_notes.md` (состояние, расчёты, открытые пункты), правила оформления, уроки | да |
| `output/` | PDF, BOM, нетлист, JLCPCB, Visio, Word | нет — только пересборка |
| `reports/` | ERC, DRC | нет |
| `obraz/` | образцы заказчика | не в git; формат/оформление брать оттуда |
| `pcb_route/` | рабочие файлы трассировки | не в git |

Служебные файлы KiCad (`*.kicad_prl`, `*-backups/`, `*.lck`) и `.vscode/`, `.history/` в git не идут (`.gitignore`).
Концы строк — LF на любой ОС (`.gitattributes`); CSV для JLCPCB хранятся как есть.

## Роли файлов `verification/`

| Файл | Роль |
|---|---|
| `netlist_revC.xml` | нетлист прошлой (опорной) ревизии — основа соединений неизменённых узлов |
| `expected_revE.json` | заданный нетлист новой ревизии: что должно получиться |
| `eco_revE.json` | списки позиций: `revC` — что изменено в старой ревизии, `revE` — что новое; для `--exclude-a/-b` |
| `refmap_revC_to_revE.json` | старое обозначение → новое (для сверки через переименование) |
| `keymap.json` | стабильный ключ → обозначение текущей ревизии (для расстановки платы) |

Имена привязаны к буквам ревизий; для новой ревизии создаются **новые** файлы, старые остаются — они часть истории и нужны для следующей сверки.

## Какие модули универсальны

| Модуль | Назначение | Универсален? |
|---|---|---|
| `env.sh` | пути к KiCad на Windows/macOS/Linux | да |
| `kisch.py`, `sheet.py` | писатель `.kicad_sch` / символов | да |
| `gost_symbols.py` | УГО ГОСТ | набор символов расширять под новые корпуса |
| `gost_wks.py` | рамка и штампы ГОСТ | да |
| `netcompare.py` | сверка нетлистов | да |
| `pro_merge.py` | восстановление секции схемы в `.kicad_pro` | да (имя файла проекта зашито) |
| `minirouter.py`, `zero_len.py`, `drc_fix.py`, `ripnet.py`, `ripbox.py`, `silk_place.py` | доводка и чистка платы | да (имя проекта зашито в части) |
| `pcb_meta.py` | стек и штамп платы | под изготовителя |
| `jlc_bom_cpl.py` | BOM/CPL для JLCPCB | да |
| `sch2vsdx.py` | схема → Visio | да (имя зашито) |
| `pe3_docx.py`, `templates/` | ПЭ3 → Word | да; шаблон — из образца заказчика |
| `dds_build.py`, `dds_parts.py`, `eco.py`, `dds_pcb.py`, `make_footprints.py` | содержимое изделия | **нет** — образец структуры |

## Что параметризовать при переносе в новый проект

Имя проекта `DDS_47_51` встречается в: `dds_build.py` (4), `dds_pcb.py` (4), `minirouter.py` (6), `pro_merge.py` (3), `sch2vsdx.py` (1),
`build_all.sh` (4), `make_outputs.sh` (7), `route_all.sh` (7). Это первое, что нужно вынести в общую переменную (`PROJECT` в `env.sh` и аргумент в Python-модулях).
Если начинаешь новый проект — сделай это сразу, одним коммитом, до остальной работы, и запиши в `lessons-learned.md`.

## Команды

```sh
sh tools/build_all.sh .            # схема + расстановка (+ route — полная разводка)   ЗАТИРАЕТ ручные правки
sh tools/route_all.sh .            # только разводка платы                              ЗАТИРАЕТ ручные трассы
sh tools/make_outputs.sh .         # документы и файлы производства                     ручные правки СОХРАНЯЕТ
```

Установка зависимостей (в Python KiCad):

- Windows: `"C:\Program Files\KiCad\10.0\bin\python.exe" -m pip install shapely vsdx python-docx pillow`
- macOS: `/Applications/KiCad/KiCad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3 -m pip install shapely vsdx python-docx pillow`
- Для разводки нужна Java 21+ и Freerouting 2.5 (`FREEROUTING_JAR`).

Из Git Bash на Windows задать `PYTHONUTF8=1` (делает `env.sh`), иначе кириллица в именах и выводе ломается.
