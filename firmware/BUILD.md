# Сборка приложения DDS Studio (Rust + egui) для Windows и macOS

Как собрать **программу для ПК**, которой управляют платой DDS 48–52 МГц: `DDS Studio` —
один исполняемый файл на Rust с интерфейсом на [egui](https://github.com/emilk/egui),
без WebView, Node и установщика.

> **Важно: исходники приложения лежат не здесь.** Папка `firmware` — это прошивка
> контроллера 1887ВЕ7Т на C (avr-gcc, см. [README.md](README.md)).
> Приложение находится в соседней папке **`../studio`**, крейт `crates/dds-egui`.
> Все команды ниже выполняются из папки `studio`.

| Что собираем | Где исходники | Чем собирается | Инструкция |
|---|---|---|---|
| **Прошивка платы** (C, AVR) | `firmware/` | CMake + avr-gcc, прошивка Atmel-ICE | [README.md](README.md), разделы 4–7 |
| **Приложение для ПК** (Rust, egui) | `../studio/crates/dds-egui` | `cargo` + скрипт сборки | этот файл |

Связанное: [studio/BUILD.md](../studio/BUILD.md) (полная версия) · [studio/EGUI.md](../studio/EGUI.md) (устройство приложения) · [README.md, раздел 8.5](README.md) (протокол UART)

---

## 1. Что понадобится

Node.js и npm настольному приложению **не нужны** — они нужны только для веб-режима.

| | Windows | macOS |
|---|---|---|
| Rust | [rustup](https://rustup.rs) stable, toolchain **MSVC** (по умолчанию) | `curl https://sh.rustup.rs -sSf \| sh` |
| Компилятор C/C++ | Visual Studio Build Tools 2022, компонент **«Desktop development with C++»** | Xcode Command Line Tools: `xcode-select --install` |
| Python 3 | желателен (без него проверка версии пропускается) | уже есть в системе |
| Git | `winget install Git.Git` | идёт с Command Line Tools |

Установка Rust на Windows одной командой:

```powershell
winget install Rustlang.Rustup
```

После установки **перезапустите терминал** и проверьте:

```powershell
cargo --version
rustc --version
```

---

## 2. Самый быстрый путь: просто запустить

Если нужно не раздать дистрибутив, а поработать с платой здесь и сейчас:

```powershell
cd C:\Project\DDS\studio
cargo run --release -p dds-egui
```

```bash
cd ~/Project/DDS/studio
cargo run --release -p dds-egui
```

Первая сборка скачивает зависимости и занимает 5–15 минут, последующие — секунды.
Ключ `--release` важен: в отладочной сборке расчёт спектра и графиков заметно медленнее.

Дальше — экран **«Подключение»**: выберите порт, скорость **38400 бод (прошивка)** и нажмите
«Подключить». Приложение само переведёт плату в режим `REMOTE`. Распайка и команды —
[README.md, раздел 8.5](README.md); прошивка D2 должна быть версии **1.1.0** или новее.

---

## 3. Windows: собрать готовый `.zip`

Из папки `studio` в PowerShell:

```powershell
cd C:\Project\DDS\studio
.\build_windows.ps1
```

Из обычной командной строки (CMD) — то же самое: `build_windows.bat`.

Ключи:

| Ключ | Что делает |
|---|---|
| `-WithServer` | дополнительно собрать `dds-server` (веб-режим, «удалённая лаборатория») |
| `-Open` | открыть папку с результатом после сборки |
| `-SkipVersionCheck` | не проверять, что версия совпадает во всех файлах |

Результат:

```text
studio\release\v0.3.0\
  DDS-Studio-v0.3.0-windows-x64.zip     (DDS Studio\DDS Studio.exe + LICENSE)
  dds-server-v0.3.0-x64.exe             (с -WithServer)
  checksum.txt                          (SHA-256)
```

Внутри архива — **один `DDS Studio.exe`**, установка не требуется: распаковать и запустить.
Программа не подписана, поэтому SmartScreen может показать предупреждение —
«Подробнее» → «Выполнить в любом случае».

---

## 4. macOS: собрать `.dmg`

Из папки `studio` в терминале:

```bash
cd ~/Project/DDS/studio
python3 build_macos.py                              # под архитектуру этого Mac
python3 build_macos.py --universal                  # Apple Silicon + Intel в одном файле
python3 build_macos.py --universal --with-server --open
```

Короткие варианты: `make build-mac` и `make build-mac-universal`.

Ключи:

| Ключ | Что делает |
|---|---|
| `--universal` | собрать для arm64 и x86_64 сразу (нужны оба target, см. раздел 8) |
| `--with-server` | дополнительно собрать `dds-server` |
| `--open` | открыть папку с результатом |
| `--skip-tests` | пропустить тесты `dds-core` и `dds-egui` |
| `--skip-version-check` | не проверять синхронность версий |

Результат:

```text
studio/release/v0.3.0/
  DDS-Studio-v0.3.0-macos-arm64.dmg
  DDS-Studio-v0.3.0-macos-arm64.app.zip
  dds-server-v0.3.0-macos-arm64         (с --with-server)
  checksum.txt
```

Приложение подписано **ad-hoc**, без сертификата Apple и нотаризации. При первом запуске
скачанного `.dmg` macOS предупредит о неизвестном разработчике: правый клик по
`DDS Studio.app` → **«Открыть»**, либо снять карантин:

```bash
xattr -dr com.apple.quarantine "/Applications/DDS Studio.app"
```

---

## 5. Версия: только через инструмент

Версию **нельзя править руками** — она продублирована в нескольких файлах, и скрипты
сборки остановятся, если значения разойдутся. Единственный источник — `studio/config/version.json`.

```bash
python3 tools/update_version.py status        # проверить, что везде одинаково
python3 tools/update_version.py bump patch    # 0.3.0 → 0.3.1 во всех файлах
python3 tools/update_version.py sync          # починить расхождение
```

Затем допишите раздел `## [X.Y.Z] - YYYY-MM-DD` в `studio/CHANGELOG.md`.
Подробнее — [studio/VERSION_MANAGEMENT.md](../studio/VERSION_MANAGEMENT.md).

---

## 6. Проверка перед сборкой (необязательно)

```bash
cargo test -p dds-core -p dds-egui        # тесты модели кристалла и интерфейса
make check                                # полная проверка: версии, TS, тесты, clippy
```

На Windows `make` обычно нет — запускайте `cargo test` напрямую, `build_windows.ps1`
сам прогоняет нужные проверки.

---

## 7. Если нет своего Mac (или своей Windows)

Сборку под обе системы можно получить в GitHub Actions, не имея второго компьютера.

1. GitHub → репозиторий → **Actions** → **CI** → **Run workflow**, ветка `main`.
   Из терминала: `gh workflow run ci.yml --ref main`, затем `gh run watch`.
2. Через 15–25 минут откройте запуск → **Artifacts**:
   - `dds-studio-macos-universal` — `.dmg` и `.app.zip`;
   - `dds-studio-windows-x64` — `.zip` с `DDS Studio.exe`.
   Скачать из терминала: `gh run download <id>`. Артефакты хранятся 14 дней.

Почему нельзя собрать всё на одной машине: Windows-сборке нужен компоновщик MSVC,
а `.app`/`.dmg` создаются только средствами macOS (`codesign`, `hdiutil`).

---

## 8. Если что-то пошло не так

**`cargo: command not found` / `cargo не распознан`** — Rust не установлен или терминал
не перезапущен после установки. Проверьте `rustup --version`.

**Windows: `link.exe not found`** — нет Visual Studio Build Tools с компонентом
«Desktop development with C++». Установите и перезапустите терминал.

**Windows: `running scripts is disabled on this system`** — политика PowerShell.
Запускайте через `build_windows.bat` или:

```powershell
powershell -ExecutionPolicy Bypass -File build_windows.ps1
```

**macOS: `xcrun: error: invalid active developer path`** — выполните `xcode-select --install`.

**macOS universal: `target may not be installed`** — добавьте обе архитектуры:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

**`версии в файлах расходятся`** — кто-то поправил версию вручную:
`python3 tools/update_version.py sync`.

**Приложение показывает старую версию** — пересоберите после `sync`: версия в интерфейсе
(внизу боковой панели) зашивается из `Cargo.toml` при компиляции.

**Платы нет в списке портов** — проверьте USB-UART адаптер и распайку
(RX адаптера → X1.1, TX → X1.4, GND → X1.6), на время ISP-программирования адаптер
отключайте. Кнопка `⟳` («Обновить список») на экране «Подключение» перечитывает порты.

**Linux** (не основная платформа): `cargo build --release -p dds-egui`, нужны пакеты
`libgtk-3-dev`, `libxkbcommon-dev`, `libudev-dev`.

---

## 9. Где что лежит после сборки

| Что | Путь (от папки `studio`) |
|---|---|
| Готовые дистрибутивы | `release/vX.Y.Z/` |
| Бинарники Rust | `target/release/` (или `target/<triple>/release/`) |
| Промежуточный `.app` (macOS) | `target/egui-bundle/dmg/DDS Studio.app` |
| Настройки приложения (Windows) | `%APPDATA%\com.dds-studio.desktop\dds-studio.json` |
| Настройки приложения (macOS) | `~/Library/Application Support/com.dds-studio.desktop/dds-studio.json` |

Удалить всё собранное: `make clean` (или `cargo clean` и удаление `release/`).
