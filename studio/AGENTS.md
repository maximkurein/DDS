# AGENTS.md

Инструкции для ИИ-агентов (Codex, Cursor, Claude Code и др.), работающих с этим репозиторием. Подробная техническая справка — в [CLAUDE.md](CLAUDE.md).

## Проект

DDS Studio — приложение для управления и изучения АО НПЦ «ЭЛВИС» 1508ПЛ8Т (двухканальный DDS 1 ГГц с 48-битным аккумулятором) и платы DDS 48–52 МГц, ревизия F. Rust-ядро (`crates/dds-core`), desktop-оболочка Tauri 2 (`src-tauri`), веб-сервер (`crates/dds-server`), интерфейс React 19 + Tailwind 4 (`src`). С платой приложение говорит через USB-UART с прошивкой контроллера D2 (`../firmware`, версия 1.1.0+).

## Обязательные правила

1. **Версия** меняется только через `python3 tools/update_version.py` (`status`, `bump patch|minor|major`, `sync X.Y.Z`). Источник — `config/version.json`. Никаких ручных правок версий в `Cargo.toml`, `package.json`, `tauri.conf.json`.
2. **CHANGELOG.md** обновляется вручную в разделе `## [Unreleased]` при каждом заметном изменении; при релизе раздел переименовывается в `## [X.Y.Z] - YYYY-MM-DD`.
3. **Перед завершением работы**: `make check` (или по отдельности: `python3 tools/update_version.py status`, `npm run build`, `cargo test -p dds-core -p dds-egui`, `cargo clippy -p dds-egui --all-targets --no-deps -- -D warnings`, `cargo check -p dds-server -p dds-studio`).
4. **Позиции битов регистров** — только из даташита 1508ПЛ8Т. Не угадывать. При изменении `regs.rs` обновить тесты.
5. **Физика и расчёты** — в Rust (`dds-core`). Фронтенд только отображает.
6. **Новая функция бэкенда** = команда в `Engine::handle` (`api.rs`) + типы в `src/lib/types.ts` + вызов через `useStore().call()`.
7. **Язык интерфейса** — русский; имена регистров и полей — как в даташите (английские).
8. **Безопасность железа**: предупреждения (логика D2 — 5 В, питание адаптера не подключать, отключать адаптер на время ISP, уровень такта на XW1) не удалять из документации и интерфейса.
9. **Протокол прошивки** меняется согласованно: `../firmware/src/app/remote.c`, `crates/dds-core/src/transport.rs`, `docs/PROTOCOL_ru.md`.

10. **Два интерфейса.** Тексты и данные экранов есть и в React (`src/`), и в egui (`crates/dds-egui`). Меняя уроки, блоки, факты о плате или «Ревизии платы», править обе стороны: `src/lib/*.ts` ↔ `content.rs`, `public/eco/index.html` ↔ `eco.rs` (тесты сверяют), `src/index.css` ↔ `theme.rs`. Подробно — [EGUI.md](EGUI.md).

## Стиль коммитов

`<описание для пользователя>` и, если менялась версия, `; bump to X.Y.Z`. Пример: `Экран «Спектр» показывает спуры усечения фазы; bump to 0.1.1`.

## Структура документации

| Файл | Назначение |
|---|---|
| `README.md` | обзор, быстрый старт, возможности |
| `BUILD.md` | сборка macOS / Windows / веб |
| `EGUI.md` | нативная версия на egui: запуск, устройство, синхронизация с вебом |
| `VERSION_MANAGEMENT.md` | версия и релизный чек-лист |
| `CHANGELOG.md` | история изменений |
| `SECURITY.md` | безопасность ПО и железа |
| `docs/INDEX_ru.md` | индекс руководств |
| `docs/FIRST_RUN_ru.md` | первый запуск с платой |
| `docs/PROTOCOL_ru.md` | подключение USB-UART и команды прошивки D2 |
| `docs/ARCHITECTURE_ru.md` | архитектура кода |
| `docs/release_guide_ru.md` | выпуск релиза |

## Что не коммитить

`target/`, `node_modules/`, `dist/`, `release/`, `src-tauri/gen/`, `.env`, `.claude/settings.local.json`, даташиты производителя (PDF) — ссылки на них давать в документации.
