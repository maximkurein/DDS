# Управление версией

Где хранится версия DDS Studio, как её безопасно менять и какие файлы держатся синхронно.

Связанное: [BUILD.md](BUILD.md) · [docs/release_guide_ru.md](docs/release_guide_ru.md) · [CHANGELOG.md](CHANGELOG.md)

## Источник истины

Единственный источник версии релиза — `config/version.json`:

```json
{
  "app_info": {
    "name": "DDS Studio",
    "version": "X.Y.Z",
    "release_date": "DD.MM.YYYY",
    "last_updated": "YYYY-MM-DD"
  }
}
```

Версия в интерфейсе (внизу боковой панели) и в RPC `get_meta` берётся из `Cargo.toml` при компиляции (`CARGO_PKG_VERSION`), а тот синхронизируется из этого файла.

## Команды

`tools/update_version.py` использует только стандартную библиотеку Python — виртуальное окружение не нужно.

| Команда | Версия | Даты релиза |
|---|---|---|
| `status` | показывает все отслеживаемые файлы | — |
| `bump patch\|minor\|major` | следующая семантическая версия | сегодня |
| `sync X.Y.Z` | устанавливает `X.Y.Z` | сегодня |
| `sync` | повторно применяет текущую версию источника | сохраняются из источника |

Флаги `bump` и `sync`: `--release-date DD.MM.YYYY`, `--last-updated YYYY-MM-DD`, `--dry-run` (показать, что изменится, ничего не записывая).

```text
$ python3 tools/update_version.py bump patch
Target version:      0.1.1
Target release date: 08.11.2026
Target last updated: 2026-11-08

Updated config/version.json
Updated Cargo.toml
Updated Cargo.lock
Updated package.json
Updated package-lock.json
Updated README.md
Updated .github/ISSUE_TEMPLATE/bug.yml
```

Короткие варианты: `make version`, `make version-status`, `make bump-patch` / `bump-minor` / `bump-major`.

## Что синхронизируется

| Метка в `status` | Файл | Что меняется |
|---|---|---|
| `source` | `config/version.json` | `app_info.version` и даты |
| `cargo` | `Cargo.toml` | `[workspace.package] version` (все крейты наследуют `version.workspace = true`) |
| `cargo_lock` | `Cargo.lock` | версии `dds-core`, `dds-server`, `dds-egui` |
| `npm` | `package.json` | `"version"` |
| `npm_lock` | `package-lock.json` | `version` и `packages[""].version` |
| `readme` | `README.md` | заголовок `# DDS Studio vX.Y.Z` |
| `bug_form` | `.github/ISSUE_TEMPLATE/bug.yml` | подсказка версии |

Намеренно **не** синхронизируются: `CHANGELOG.md` (пишется вручную), упоминания версий в `docs/` (история), версия прошивки платы D2 (`firmware/CMakeLists.txt`, своя нумерация; протокол — `docs/PROTOCOL_ru.md`).

## Чек-лист релиза

1. Поднять версию:
   ```bash
   python3 tools/update_version.py bump patch
   ```
   Windows: `python tools\update_version.py bump patch`.
2. В `CHANGELOG.md` переименовать `## [Unreleased]` в `## [X.Y.Z] - YYYY-MM-DD` (та же дата, что `last_updated`).
3. Проверить, что ничего не разошлось:
   ```bash
   python3 tools/update_version.py status
   ```
   Все строки должны быть `OK`.
4. `make check`, коммит, push. Сообщение коммита: описание для пользователя, затем `; bump to X.Y.Z`.
5. Сборка: `build_macos.py` / `build_windows.ps1` берут версию из `config/version.json` и отказываются собирать при расхождении. Либо в GitHub Actions: Actions → CI → Run workflow — задание `desktop` соберёт установщики macOS и Windows как артефакты (релиз по тегу пока не подключён, см. [BUILD.md](BUILD.md#сборка-в-github-actions)).

## Семантика версий

- **patch** (0.1.0 → 0.1.1) — исправления, уточнения моделей, тексты.
- **minor** (0.1.x → 0.2.0) — новые экраны, эксперименты, драйверы подключения, новые поля конфигурации.
- **major** (0.x → 1.0.0) — несовместимые изменения формата экспорта/настроек или UART-протокола прошивки.

## Проблемы

**`status` показывает `DIFF`** — файл правили вручную. `sync` (сохраняет версию и даты источника).

**`RuntimeError: Не удалось обновить …`** — строку, которую ищет инструмент, переформатировали. Верните ожидаемый вид (см. таблицу) и повторите `sync`.

**`cargo_lock` = `mixed`** — Cargo.lock частично обновлён. `sync`, затем `cargo check` (Cargo сам допишет остальное).

## Правило

Меняйте версию только через `tools/update_version.py` — никогда поиском-заменой по репозиторию. Источник истины — `config/version.json`. Перед релизом всегда `status`.
