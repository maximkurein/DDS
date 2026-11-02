# START HERE — с чего начать на любой машине

Короткая инструкция: как за 5 минут продолжить работу над проектом (Windows или macOS) вместе с Claude Code и скиллами.
Подробности: [README_RU.md](README_RU.md) (состояние и сборка), [docs/design_notes.md](docs/design_notes.md) (решения и открытые пункты).

## 1. Получить проект

```sh
git clone https://github.com/maximkurein/DDS.git     # первый раз
cd DDS
git pull                                              # каждый раз перед работой
```

Копия на NAS: `ssh://git@192.168.1.222:2222/mxm/DDS.git` (remote `nas`). Публиковать в оба: `git push origin main && git push nas main`.

## 2. Подготовить среду (один раз на машине)

| Что | Windows | macOS |
|---|---|---|
| KiCad 10 | `C:\Program Files\KiCad\10.0` | `/Applications/KiCad/KiCad.app` |
| Запуск скриптов | Git Bash: `sh tools/make_outputs.sh .` | Terminal: `sh tools/make_outputs.sh .` |
| Пакеты Python (в Python KiCad) | `"C:\Program Files\KiCad\10.0\bin\python.exe" -m pip install shapely vsdx python-docx pillow` | `/Applications/KiCad/KiCad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3 -m pip install shapely vsdx python-docx pillow` |
| Для разводки | Java 21+ и Freerouting 2.5 (`FREEROUTING_JAR`) | то же |

Пути к KiCad скрипты находят сами (`tools/env.sh`). Проверка: `sh tools/make_outputs.sh .` проходит без строк `skipped`.

## 3. Запустить Claude Code из корня репозитория

Проектные скиллы лежат в `.claude/skills/` и подхватываются автоматически (после `git pull` перезапустите сессию).

| Скилл | Когда срабатывает |
|---|---|
| `pcb-design-pipeline` | новое изделие, выбор ЭРИ, схема, плата, разводка, документы, заказ в производство |
| `pcb-revision-eco` | исправление ошибок, замена ЭРИ, правка размеров/шелкографии/надписей, новая ревизия готовой платы |

Примеры запросов:

- «Начинаю новое изделие: ТЗ вот такое, помоги выбрать ЭРИ и пройти по технологии» → `pcb-design-pipeline`
- «Нашли ошибку на макете: … Сделай ревизию F, не трогая остальное» → `pcb-revision-eco`
- «Замени D5 на … и пересоберите документы» → `pcb-revision-eco`

## 4. Три правила, которые нельзя нарушать

1. **Источник истины — проект KiCad и база в `tools/`.** Visio, Word, PDF, BOM, Gerber руками не правятся — только пересборка.
2. **`build_all.sh` и `route_all.sh` затирают ручные правки.** Начали править руками — запускайте только `make_outputs.sh`.
3. **Одну и ту же схему или плату не править на двух машинах одновременно.** Порядок: `git pull` → работа → ERC/DRC → `git commit` → `git push`.

## 5. Что где

| Нужно | Смотреть |
|---|---|
| Текущее состояние, открытые пункты | `docs/design_notes.md` |
| Как пересобрать, история ревизий | `README_RU.md` |
| Правила оформления Э3 и ПЭ3 | `docs/Правила_оформления_Э3_ПЭ3.md` |
| KiCad для пользователя P-CAD | `docs/KiCad_уроки.md` |
| Заказ плат | `output/jlcpcb/README_JLC.md`; текст для посредника — `output/jlcpcb/ORDER_RU.txt` / `ORDER_EN.txt` (параметры: `docs/order_params.json`, количество: `ORDER_QTY=10 sh tools/make_outputs.sh .`) |
| Технология и её грабли | `.claude/skills/pcb-design-pipeline/` |
| Порядок выпуска ревизий | `.claude/skills/pcb-revision-eco/` |

Метки: `revB-baseline`, `revE-baseline` — опорные точки ревизий; новая ревизия начинается от метки предыдущей.

## 6. Как улучшать скиллы

Нашли ошибку или грабли — добавьте запись в `.claude/skills/pcb-design-pipeline/references/lessons-learned.md`
(или попросите Claude: «обнови скиллы по итогам этой работы»), затем:

```sh
git add .claude && git commit -m "Skills: <что уточнено>" && git push origin main && git push nas main
```

Для нового изделия в другом репозитории скопируйте `.claude/skills/` в его корень
(или в `~/.claude/skills/`, чтобы скиллы работали во всех проектах на этой машине).
