# Сборка DDS Studio

Как собрать **настольное приложение** (нативный интерфейс на egui, `crates/dds-egui`) для macOS (`.app` + `.dmg`) и Windows (`.zip` с `DDS Studio.exe`), а также веб-сервер `dds-server`.

Связанное: [EGUI.md](EGUI.md) · [VERSION_MANAGEMENT.md](VERSION_MANAGEMENT.md) · [docs/release_guide_ru.md](docs/release_guide_ru.md) · [CHANGELOG.md](CHANGELOG.md)

## Модели сборки

| Платформа | Результат | Нужен ли пользователю Rust/Node? |
|-----------|-----------|----------------------------------|
| **macOS** | `DDS-Studio-vX.Y.Z-macos-<arch>.dmg` и `.app.zip` (ad-hoc подпись) | Нет |
| **Windows** | `DDS-Studio-vX.Y.Z-windows-x64-setup.exe` (ярлык на рабочем столе) и `.zip` | Нет |
| **Веб / сервер** | `dds-server` + папка `dist/` | Нет |

Все артефакты складываются в `release/vX.Y.Z/` вместе с `checksum.txt` (SHA-256).

## Требования

| | macOS | Windows |
|---|---|---|
| Rust | `rustup` stable (`curl https://sh.rustup.rs -sSf \| sh`) | `rustup` stable, toolchain **MSVC** |
| Системное | Xcode Command Line Tools: `xcode-select --install` | Visual Studio Build Tools 2022, компонент «Desktop development with C++» |
| Python | `python3` (есть в macOS) — для проверки версии | Python 3 (необязательно — без него проверка версии пропускается) |
| Установщик | — | [Inno Setup 6](https://jrsoftware.org/isdl.php): `C:\Program Files (x86)\Inno Setup 6\ISCC.exe` |
| Node.js | 20+ — только для веб-интерфейса (`npm ci`, `npm run build`) | то же |

Настольному приложению Node.js не нужен: это один бинарник на Rust.

---

## Шаг 1. Версия (только через инструмент)

Никогда не правьте версию вручную. Единственный источник — `config/version.json`.

```bash
python3 tools/update_version.py status        # всё ли синхронно
python3 tools/update_version.py bump patch    # 0.1.0 → 0.1.1 во всех файлах
```

Затем добавьте раздел `## [X.Y.Z] - YYYY-MM-DD` в `CHANGELOG.md`. Подробности — [VERSION_MANAGEMENT.md](VERSION_MANAGEMENT.md).

## Шаг 2. Проверка

```bash
make check     # версии + tsc + vite build + cargo test + clippy egui + cargo check
```

## Шаг 3. Сборка

### macOS

```bash
python3 build_macos.py                         # архитектура этого Mac (arm64 или x64)
python3 build_macos.py --universal             # один бинарник для Apple Silicon и Intel
python3 build_macos.py --universal --with-server --open
```

или `make build-mac` / `make build-mac-universal`.

Скрипт проверяет инструменты и версии, запускает тесты `dds-core` и `dds-egui`, собирает `cargo build --release -p dds-egui` (для `--universal` — обе архитектуры и `lipo`), складывает `DDS Studio.app` (Info.plist, иконка `assets/icons/icon.icns`), подписывает ad-hoc и делает `.dmg` через `hdiutil`:

```text
release/v0.2.0/
  DDS-Studio-v0.2.0-macos-arm64.dmg
  DDS-Studio-v0.2.0-macos-arm64.app.zip
  dds-server-v0.2.0-macos-arm64        (с --with-server)
  checksum.txt
```

Bundle id — `com.dds-studio.egui`. Настройки приложения лежат в `~/Library/Application Support/com.dds-studio.desktop/dds-studio.json` (тот же файл, что у прежней версии на Tauri).

Приложение подписано **ad-hoc** (`codesign --sign -`), без нотаризации. При первом запуске скачанного `.dmg` macOS может предупредить о неизвестном разработчике: правый клик → «Открыть», или

```bash
xattr -dr com.apple.quarantine "/Applications/DDS Studio.app"
```

Для распространения с нотаризацией нужен сертификат Developer ID: подпишите `.app` своим идентификатором (`codesign --options runtime --sign "Developer ID Application: …"`) и отправьте `.dmg` в `xcrun notarytool submit`.

### Windows

```powershell
.\build_windows.ps1                    # zip и DDS-Studio-vX.Y.Z-windows-x64-setup.exe
.\build_windows.ps1 -WithServer -Open
```

или из CMD: `build_windows.bat` (те же параметры).

```text
release\v0.3.1\
  DDS-Studio-v0.3.1-windows-x64-setup.exe
  DDS-Studio-v0.3.1-windows-x64.zip    (DDS Studio\DDS Studio.exe + LICENSE)
  dds-server-v0.3.1-x64.exe            (с -WithServer)
  checksum.txt
```

`setup.exe` ставит программу в профиль пользователя: `%LOCALAPPDATA%\Programs\DDS Studio`. Права администратора не нужны. Ярлык появляется в меню «Пуск», галочка «ярлык на рабочем столе» в мастере уже включена. Иконка вшита в `DDS Studio.exe`, поэтому ярлык показывает значок приложения, а не пустой лист.

Программа не подписана — SmartScreen может показать предупреждение («Подробнее» → «Выполнить в любом случае»). Для себя распаковывать zip не нужно: тот же файл лежит в `target\win-bundle\DDS Studio\DDS Studio.exe` (и он же — `target\release\dds-egui.exe`).

### Веб-режим / удалённая лаборатория

```bash
make build-web
./target/release/dds-server --host 0.0.0.0 --port 7878 --dist dist
```

Откройте `http://<адрес>:7878`. Параметры: `--host` (по умолчанию `127.0.0.1`), `--port` (7878), `--dist` (папка интерфейса), `--config` (файл состояния, по умолчанию `~/.dds-studio/server.json`). Открывая сервер в сеть (`0.0.0.0`), помните: аутентификации нет — см. [SECURITY.md](SECURITY.md). Экспорт регистров браузер скачивает файлом, загрузка конфигурации — через выбор файла.

---

## Сборка в GitHub Actions

GitHub выполняет workflow **только из корневой папки репозитория** — `DDS/.github/workflows/`.

| Workflow (корень репозитория) | Когда | Что делает |
|---|---|---|
| `.github/workflows/ci.yml` → `firmware`, `studio` | push/PR в `main`, затронуты `studio/**` или `firmware/**` | host-тесты прошивки; проверка версий, `npm run build`, `cargo test -p dds-core -p dds-egui`, clippy, `cargo check --workspace` |
| `.github/workflows/ci.yml` → `desktop` | только ручной запуск (`workflow_dispatch`) | `build_macos.py --universal` (`.dmg`, Apple Silicon и Intel) и `build_windows.ps1` (`.zip` и установщик, Windows x64) |

### Как получить сборки macOS и Windows без своего Mac

1. GitHub → репозиторий → **Actions** → **CI** → **Run workflow** (ветка `main`).
   Или из терминала: `gh workflow run ci.yml --ref main` и `gh run watch`.
2. Через 15–25 минут откройте запуск → раздел **Artifacts**:
   - `dds-studio-macos-universal` — `DDS-Studio-vX.Y.Z-macos-universal.dmg` и `.app.zip`;
   - `dds-studio-windows-x64` — `DDS-Studio-vX.Y.Z-windows-x64.zip` и `DDS-Studio-vX.Y.Z-windows-x64-setup.exe`.
   Скачать из терминала: `gh run download <id>`. Артефакты хранятся 14 дней.
3. Сборки не подписаны: macOS — правый клик → «Открыть» (или `xattr -dr com.apple.quarantine`), Windows — SmartScreen «Подробнее» → «Выполнить в любом случае».

### Релиз на GitHub

`DDS/.github/workflows/release.yml` собирает `.dmg` (macOS universal), `.zip` и установщик (Windows x64) теми же скриптами и создаёт **черновик** GitHub Release с ними и `checksum.txt`.

- Автоматически: `python tools/update_version.py studio bump minor` → запись в `studio/CHANGELOG.md` → коммит → `git tag -a studio-vX.Y.Z -m "..."` → `git push origin main --tags`.
- Вручную для уже отправленной метки: Actions → **Release** → Run workflow → `studio-v0.2.0`.
- Метка обязана совпадать с `studio/config/version.json`, иначе сборка остановится на проверке.
- Через 15–25 минут: Releases → черновик «DDS Studio studio-vX.Y.Z» → проверить файлы → **Publish release**.

### Почему не собрать Windows/macOS-сборку где угодно

- **Windows** собирается на Windows (нужен MSVC): локально `build_windows.bat` или в CI.
- **macOS** `.app`/`.dmg` собирается только на macOS: локально `build_macos.py` или в CI. С Windows-ПК — только через `desktop` в GitHub Actions.

## Где что лежит после сборки

| Что | Путь |
|---|---|
| Веб-интерфейс | `dist/` |
| Бинарники Rust | `target/release/` (или `target/<triple>/release/`) |
| Промежуточный `.app` (macOS) | `target/egui-bundle/dmg/DDS Studio.app` |
| Готовые артефакты | `release/vX.Y.Z/` |

## Проблемы и решения

**`версии в файлах расходятся`** — кто-то изменил версию вручную. `python3 tools/update_version.py sync`.

**macOS: `xcrun: error: invalid active developer path`** — `xcode-select --install`.

**macOS universal: `target may not be installed`** — `rustup target add aarch64-apple-darwin x86_64-apple-darwin`.

**Windows: `link.exe not found`** — установите Visual Studio Build Tools с компонентом C++ и перезапустите терминал.

**Windows: `running scripts is disabled on this system`** — запускайте через `build_windows.bat` или `powershell -ExecutionPolicy Bypass -File build_windows.ps1`.

**Linux: не собирается `dds-egui`** — нужны `libgtk-3-dev` (диалоги файлов), `libxkbcommon-dev` и `libudev-dev` (последовательный порт).

**Приложение показывает старую версию** — пересоберите после `sync`; версия в интерфейсе (внизу боковой панели) берётся из `Cargo.toml` при компиляции.

**`npm ci` ругается на lock-файл** — `npm install` обновит `package-lock.json`; затем снова `python3 tools/update_version.py status`.
