# Управление версиями DDS

Единственный источник — **`config/version.json`**. Руками версии в файлах не правятся: только через `tools/update_version.py`, затем запись в `CHANGELOG.md`.
Разработчик: Kurein M. N. (`app_info.developer`).

| Часть | Формат | Где хранится | Куда расставляется |
|---|---|---|---|
| Проект (репозиторий) | SemVer `1.0.0` | `app_info.version` | значок «версия» в `README.md` |
| Плата | буква ревизии `F` | `components.board.revision` | штампы `(rev …)` во всех `*.kicad_sch` и `*.kicad_pcb`, значок «ревизия» в `README.md`, имена `_revF_` в `tools/make_outputs.sh` |
| Прошивка | SemVer `1.1.0` | `components.firmware.version` | `firmware/CMakeLists.txt` (`project(dds_fw VERSION …)`), значок «прошивка» в `README.md` |
| DDS Studio | SemVer `0.2.0` | `studio/config/version.json` (свой источник) | файлы студии — своим `studio/tools/update_version.py`; корень хранит копию |

## Команды (из корня `C:\Project\DDS` или `~/Project/DDS`)

> Это справочник: **не запускайте блок целиком** — каждая строка меняет версию. Выполняйте только нужную команду; сначала можно с `--dry-run`.

```bash
python tools/update_version.py status                     # всё ли совпадает; код 1 при расхождении
python tools/update_version.py sync                       # расставить версии из config/version.json
python tools/update_version.py bump patch|minor|major     # версия проекта, дата выпуска = сегодня
python tools/update_version.py firmware bump patch        # 1.1.0 -> 1.1.1
python tools/update_version.py firmware set 1.2.0
python tools/update_version.py board next --yes           # F -> G, только в начале новой ревизии; без --yes не выполнится
python tools/update_version.py studio bump minor          # передаётся в studio/tools/update_version.py
python tools/update_version.py --dry-run board next       # показать, что изменится, без записи
```

## Когда что поднимать

- **Плата `board next`** — в начале новой ревизии по навыку `pcb-revision-eco`, до правок: ветка `rev<G>`, ECO в `docs/design_notes.md`. После проверенной сборки — метка `rev<G>-baseline`. Генераторы под конкретную ревизию (`tools/apply_revF_*.py`, `eco_revF`) скрипт не трогает.
- **Прошивка** — patch: исправление; minor: новая команда протокола; major: несовместимый протокол. Строка идентификации `?` берёт версию из CMake.
- **DDS Studio** — по её [VERSION_MANAGEMENT.md](studio/VERSION_MANAGEMENT.md); запись в `studio/CHANGELOG.md`.
- **Проект** — patch: документы, навыки, CI; minor: новая версия любой части (ревизия платы, прошивка, студия); major: смена изделия или несовместимая перестройка репозитория.

Порядок выпуска: поднять версию → `status` = OK → запись в `CHANGELOG.md` → коммит → метка (`v1.1.0`, `rev<G>-baseline`, `studio-v0.3.0`) → `git push origin main --tags` и `git push nas main --tags`.
