# DDS Studio на egui

`crates/dds-egui` — настольное приложение DDS Studio на [egui/eframe](https://github.com/emilk/egui): один исполняемый файл на Rust без WebView, Node и npm. Оно заменило прежнюю оболочку Tauri. Ядро то же — `dds-core`, команды те же, что у веб-интерфейса (`Engine::handle`).

## Запуск и сборка

```bash
cargo run -p dds-egui                 # отладочная сборка
cargo run -p dds-egui --release       # быстрее спектр и графики
make app                              # то же (или npm run app)
make egui-shots                       # тур скриншотов → release/egui-shots/
python3 build_macos.py                # .app + .dmg → release/vX.Y.Z/
python3 build_macos.py --universal    # Apple Silicon + Intel
.\build_windows.ps1                   # Windows: .zip с DDS Studio.exe
```

Результат `build_macos.py`: `release/vX.Y.Z/DDS-Studio-vX.Y.Z-macos-<arch>.dmg` и `.app.zip`; bundle id `com.dds-studio.egui`, подпись ad-hoc (без сертификата Apple). Перед сборкой скрипт проверяет `tools/update_version.py status` и прогоняет тесты `dds-core` и `dds-egui`; `--skip-version-check` и `--skip-tests` отключают проверки.

Linux: `cargo build --release -p dds-egui` → `target/release/dds-egui` (нужны `libgtk-3-dev`, `libxkbcommon-dev`, `libudev-dev`). Подробно о сборке — [BUILD.md](BUILD.md).

## Устройство

```
src/main.rs       окно eframe, шрифты, иконка (assets/icons/icon.png)
src/app.rs        App: верхняя строка, боковое меню, журнал шины, тосты, горячие клавиши, файлы
src/worker.rs     поток движка: Engine живёт в своём потоке, UI шлёт Act, читает Shared
src/theme.rs      палитра тёмной и светлой темы (токены из src/index.css)
src/widgets.rs    карточки, поля ввода с единицами, сегменты, kv, заголовок страницы
src/plot.rs       графики (порт components/Plot.tsx): лог-оси, вторая ось Y, подсказка
src/canvas.rs     рисование SVG-схем в координатах viewBox
src/fmt.rs        форматирование частот, дБ, инженерная запись
src/content.rs    блоки схемы, факты о плате, уроки Академии (из src/lib/*.ts)
src/eco.rs        «Ревизии платы» (из public/eco/index.html)
src/views/*.rs    12 экранов
src/shots.rs      тур скриншотов (DDS_SHOTS=<папка>)
```

**Поток движка.** `dds_core::Engine` принадлежит одному рабочему потоку. Экраны ничего не вычисляют и не блокируют кадр: читают снимок `Shared` (регистры, статус, журнал, ошибки) и ставят действия `Act` в очередь (`rpc`, `Batch`, `RefreshPorts` …). Дорогие расчёты (спектр, АЧХ тракта, план ЛЧМ) запрашиваются через `sh.query(slot, key, Query)`: запрос уходит один раз на ключ (хэш конфигурации и параметров), ответ кэшируется — это аналог `useRpcData` из веб-версии. Ошибки ядра превращаются в тосты.

**Экраны** — как в веб-интерфейсе: Обзор, Плата, Регистры, Частота, Спектр, Тракт и шум, Свип, Перебор, Модуляция, Подключение, Академия, Ревизии платы; журнал шины — ⌘/Ctrl + J, экраны — ⌘/Ctrl + 1…9.

**Файлы.** Сохранение экспорта регистров и загрузка конфигурации — системные диалоги (`rfd`). Настройки (параметры стенда, последняя конфигурация, порт) лежат в том же файле, что и у прежней версии на Tauri: `~/Library/Application Support/com.dds-studio.desktop/dds-studio.json` (Windows — `%APPDATA%\com.dds-studio.desktop\`, Linux — `~/.config/com.dds-studio.desktop/`).

## Синхронизация с веб-интерфейсом

Тексты и данные экранов взяты из React-версии. При их изменении правьте обе стороны:

| Веб | egui |
|---|---|
| `src/lib/blocks.ts`, `board.ts`, `lessons.ts` | `src/content.rs` |
| `public/eco/index.html` | `src/eco.rs` (тесты `eco_matches_web`, `wizard_logic_matches_the_page`) |
| `src/index.css` (цвета тем) | `src/theme.rs` |
| `src/views/*View.tsx` | `src/views/*.rs` |
| новая RPC-команда в `api.rs` | вызов через `sh.rpc(...)` или `Query` в `worker.rs` |

## Проверки

```bash
cargo test -p dds-core -p dds-egui
cargo clippy -p dds-egui --all-targets --no-deps -- -D warnings
rustfmt --edition 2021 --config-path crates/dds-egui/rustfmt.toml --check crates/dds-egui/src/main.rs
DDS_SHOTS=release/egui-shots cargo run -p dds-egui --release
```

Тур скриншотов проходит все экраны в тёмной и светлой теме и варианты (журнал шины, вкладка манипуляций, страницы «Ревизий платы») — 40 кадров `NN_View_theme.bmp`; файл настроек при этом не читается и не пишется.

## Шрифты и значки

Встроенные шрифты egui покрывают кириллицу, но не все эмодзи: `🎛 ⎇ ⏻ 🩺` не рисуются. Значки меню выбраны из того, что есть; тест `icons_have_glyphs` проверяет, что у каждого значка есть глиф.
