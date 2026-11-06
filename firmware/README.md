# Прошивка DDS 48–52 МГц: контроллер D2 1887ВЕ7Т → синтезатор D1 1508ПЛ8Т

Прошивка платы DDS ревизии F. Код N набирается на DIP-переключателях S2 и S3, частота
`f = 48 000 000 + 1000·N` Гц выставляется на 1508ПЛ8Т по SPI. Кнопка «Применить» (S4)
перечитывает переключатели.

Проект собирается в **VS Code** или **Cursor** (CMake + avr-gcc). Прошивается через
**Atmel-ICE** (avrdude, профиль 1887ВЕ7Т), отлаживается по JTAG на разъёме X3.

---

## 1. Как работает изделие

| Событие | Что делает прошивка |
|---|---|
| Включение | Ждёт +3,3 В (DDS_PGOOD), выдерживает 20 мс, сбрасывает DDS по RSTn и настраивает его. Затем выставляет частоту с DIP. |
| Кнопка «Применить» | Подавляет дребезг (20 мс) и читает DIP дважды через 10 мс. Если N ≤ 4000 — новая частота, иначе отказ: частота остаётся прежней. |
| N = 4001…4095 при включении | ЦАП выключен, сигнала на выходе нет, пока не набран правильный N и не нажата кнопка. |
| DDS сбросился (провал питания, помеха) | Раз в секунду читается CTR. Если значение не совпадает — повторный сброс, настройка и **последняя применённая** частота. |
| Пропало +3,3 В | Буфер D3 выключается, DDS остаётся в сбросе. После возврата питания — повторная настройка. |
| Нет такта 480 МГц на XW1 | DEVID не читается. Сброс и повтор каждые 0,5 с. |

**Смена частоты без промежуточных значений.** У 1508ПЛ8Т 48-битное приращение фазы
записывается тремя 16-битными регистрами. Поэтому используются два профиля:
новое значение пишется в неактивный, проверяется чтением и включается одной записью в SEL_REG.

**Расчёт частоты.** Приращение фазы `dPh = round(f · 2^48 / 480 МГц)` считается точно
(деление столбиком, без float и без 128-битной арифметики).
Погрешность частоты из-за округления — меньше 1 мкГц.

### Выводы D2 (по нетлисту ревизии F)

| Порт | Сигнал | Назначение |
|---|---|---|
| PB0 | MCU_CS | SCSn DDS через D3 |
| PB1 | MCU_SCK | SCK DDS через D3; это же ISP SCK (X1.3) |
| PB2 | MCU_MOSI | SDI DDS через D3 |
| PB3 | MCU_MISO | SDO DDS через D4 |
| PB4 | MCU_DDS_RESET | RSTn DDS через D3, 0 — сброс |
| PB5 | BUF_OEn | OE# буфера D3. Выход 0 — включён, третье состояние — выключен (R64) |
| PA0…PA7 | DIP0…DIP7 | S2. ON = вывод на земле = бит 1 |
| PC0…PC3 | DIP8…DIP11 | S3 |
| PC4 | DDS_PGOOD | признак +3,3 В. Внутренняя подтяжка выключена |
| PC5 | APPLYn | кнопка S4 |
| PE1 | TXD0 | отладочный лог, площадка X1.1 |
| PF4…PF7 | JTAG | разъём X3 |

---

## 2. Структура проекта

```
firmware/
├── CMakeLists.txt            сборка прошивки, тестов и симуляции
├── CMakePresets.json         пресеты: avr-debug, avr-release, host-tests, sim
├── cmake/
│   ├── avr-gcc.cmake         toolchain-файл avr-gcc
│   ├── avrdude.cmake         цели flash, fuses_*, osccal, backup…
│   └── fw_version.h.in       версия + git describe в прошивке
├── config/dds_config.h       ВСЕ параметры изделия: диапазон, шаг, Fclk, амплитуда, тайминги
├── src/
│   ├── main.c                старт: порты, калибровка RC, таймер, SPI, WDT
│   ├── app/app.c|h           конечный автомат изделия
│   ├── core/                 чистая логика без железа — тестируется на ПК
│   │   ├── freq.c|h          N → Гц → 48-битное приращение фазы
│   │   ├── dip_code.h        уровни PINA/PINC → код N
│   │   └── debounce.c|h      подавление дребезга
│   └── drivers/
│       ├── board.c|h         распиновка ревизии F, буфер D3, сброс DDS
│       ├── spi.c|h           аппаратный SPI, кадры 24 бита
│       ├── pl8t.c|h          драйвер 1508ПЛ8Т (SETA/WR/WRI/SETAFT, проверка записи)
│       ├── pl8t_regs.h       карта регистров по ТО 1508ПЛ8Т
│       ├── systick.c|h       Timer0, 1 мс
│       ├── osccal.c|h        калибровка RC 8 МГц из EEPROM
│       └── uart_log.c|h      лог в USART0 (только Debug)
├── tests/                    unit-тесты для ПК (CTest)
├── sim/sim_dds.c             прогон настоящего ELF в simavr с моделью 1508ПЛ8Т
├── tools/
│   ├── avrdude/1887ve7t.conf профиль avrdude для 1887ВЕ7Т (страница 64 байта)
│   ├── osccal.cmake          перенос калибровки 8 МГц в EEPROM
│   └── backup.cmake          резервная копия содержимого МК
└── .vscode/                  задачи, отладка, расширения — общие для VS Code и Cursor
```

Правило: всё, что зависит от изделия, — в `config/dds_config.h`, распиновка — в
`src/drivers/board.h`, регистры DDS — в `src/drivers/pl8t_regs.h`.

---

## 3. Что проверено, а что нет

| Проверка | Результат |
|---|---|
| Сборка avr-gcc 7.3 (`-mmcu=atmega128`), Debug и Release | 0 предупреждений; flash 6,1 КБ (Debug) / 5,0 КБ (Release) |
| Unit-тесты (`host-tests`) | 5189 проверок, все проходят: dPh для всех N = 0…4000, DIP, дребезг |
| Симуляция `sim`: настоящий ELF в simavr и модель 1508ПЛ8Т по ТО | все 7 сценариев проходят (раздел 8.4) |
| Профиль avrdude 1887ve7t | разбирается avrdude 7.1, командная строка `flash` формируется |
| **Железо: плата, Atmel-ICE, 1887ВЕ7Т, 1508ПЛ8Т** | **не проверялось** — порядок первого запуска в разделе 6 |

На макете подтвердить:

1. запись и сверку через профиль `1887ve7t` (у ТО МК есть расхождение по размеру страницы);
2. чтение DEVID = 0x0201 через D4 (режим SPI 0);
3. уровень −10 дБм (параметр `DDS_AMPLITUDE_MUL`);
4. работу avarice с 1887ВЕ7Т.

---

## 4. Установка инструментов (Windows, один раз)

В PowerShell:

```powershell
winget install Git.Git Kitware.CMake Ninja-build.Ninja
winget install ZakKemble.avr-gcc        # avr-gcc + avr-libc + binutils + avr-gdb
winget install AVRDudes.AVRDUDE         # avrdude 8.x (нужен 7.1+)
winget install Microsoft.VisualStudioCode   # или: winget install Anysphere.Cursor
```

После установки перезапустите терминал и проверьте:

```powershell
avr-gcc --version; avrdude -?; cmake --version; ninja --version
```

Если `avr-gcc` не находится, задайте переменную окружения `AVR_TOOLCHAIN_DIR` —
каталог тулчейна, в котором лежит `bin\avr-gcc.exe`. Для avrdude то же делает `AVRDUDE_DIR`.
Подойдёт и Microchip AVR 8-bit Toolchain 3.7 (avr-gcc 7.3).

Для unit-тестов на ПК нужен обычный компилятор C. Подойдёт любой из трёх:

- `winget install BrechtSanders.WinLibs.POSIX.UCRT` (gcc);
- `winget install LLVM.LLVM` (clang);
- Visual Studio Build Tools.

Atmel-ICE на Windows работает как HID-устройство — отдельный драйвер не нужен.

---

## 5. Открыть проект в VS Code или Cursor

Действия одинаковы для обоих редакторов: Cursor — это VS Code с другим магазином
расширений (Open VSX), и все нужные расширения есть в обоих.

1. **File → Open Folder… → `C:\Project\DDS\firmware`.** Открывать именно `firmware`, а не корень DDS.
2. Внизу справа появится «Install recommended extensions» → **Install**. Ставятся:

   | Расширение | Зачем |
   |---|---|
   | CMake Tools | пресеты, сборка, цели flash и fuses |
   | clangd | автодополнение и переходы; берёт `compile_commands.json` из `build/avr-debug` |
   | Native Debug | отладка через avr-gdb |
   | WSL | режим отладки на железе (раздел 8.2) |

   Если редактор предложит отключить Microsoft C/C++ IntelliSense в пользу clangd — соглашайтесь.
3. CMake Tools спросит пресет — выберите **AVR Debug**. Или через команды: `Ctrl+Shift+P → CMake: Select Configure Preset`.
4. **`Ctrl+Shift+B`** — сборка. В панели вывода появятся размер и проценты занятой памяти.
   Результат — в `build/avr-debug/`:

   | Файл | Что это |
   |---|---|
   | `dds_fw.elf` | для отладчика |
   | `dds_fw.hex` | для прошивки |
   | `dds_fw.map` | карта памяти |
   | `dds_fw.lss` | листинг |

5. Остальные действия — через **`Ctrl+Shift+P → Tasks: Run Task`**:

| Задача | Что делает |
|---|---|
| FW: Build (Debug / Release) | сборка |
| FW: Flash (Debug / Release) | сборка и запись через Atmel-ICE по JTAG, со сверкой |
| FW: Probe | связь с МК: напряжение, сигнатура `1E 97 02` |
| FW: Backup | копия flash, EEPROM, фьюзов, lock и калибровки в `backup/<дата>` |
| FW: Read fuses / Write fuses (Debug / Release) | фьюзы, раздел 9 |
| FW: RC 8 MHz calibration → EEPROM | калибровка генератора, раздел 6, шаг 6 |
| FW: Erase chip | стирание кристалла |
| Test: unit tests (ПК) | unit-тесты |
| Test: simulation simavr (Linux/WSL) | прогон прошивки в симуляторе |
| Debug: avarice / simavr server | серверы GDB для F5, раздел 8 |

Из обычного терминала то же самое:

```powershell
cmake --preset avr-debug
cmake --build --preset avr-debug                    # сборка
cmake --build --preset avr-debug --target flash     # прошивка
cmake --build --preset avr-debug --target fuses_read
```

---

## 6. Первое подключение платы (новый 1887ВЕ7Т)

Новый кристалл приходит с заводскими фьюзами `L=0xE1 H=0x99 E=0xFD` (расшифровка ТО,
табл. 3.117–3.119). Это RC **1 МГц** и **M103C = 0** — режим совместимости с ATmega103,
в котором прошивка для ATmega128 работать не будет. Поэтому порядок такой:

1. **Подключение.** Atmel-ICE, порт **AVR**, 10-контактный шлейф 1,27 мм → разъём **X3**.
   Плата питается от своего источника +15 В: Atmel-ICE её не питает, а только измеряет VTG.
   Разъёмы соединять при выключенном питании.
2. **Связь** — задача `FW: Probe`. Ожидается `Device signature = 1E 97 02` и
   напряжение цели около 5 В. Скорость JTAG по умолчанию 125 кГц (`AVRDUDE_BITCLOCK=8`):
   при RC 1 МГц нужно TCK < F_CPU/4.
3. **Резервная копия** — `FW: Backup`. Сохраняются заводская программа, EEPROM, фьюзы и калибровка.
4. **Фьюзы** — `FW: Read fuses`, затем `FW: Write fuses (Debug)`: `L=0x24 H=0x11 E=0xFF`.
   После этого МК работает от RC 8 МГц, режим ATmega128, JTAG-отладка разрешена.
5. **Скорость JTAG** можно поднять до 1 МГц. В `CMakeUserPresets.json` или командой
   `cmake --preset avr-debug -DAVRDUDE_BITCLOCK=1`.
6. **Калибровка RC 8 МГц** — `FW: RC 8 MHz calibration → EEPROM`.
   При сбросе 1887ВЕ7Т загружает в OSCCAL калибровку для 1 МГц (ТО, с. 300),
   а программа не может прочитать байт для 8 МГц сама. Задача читает 4 калибровочных
   байта и пишет байт для 8 МГц и его дополнение в EEPROM[0..1].
   Прошивка применяет его при старте.
   Делается один раз на кристалл: EEPROM сохраняется при стирании (EESAVE = 0).
7. **Прошивка** — `FW: Flash (Debug)`. Пишется без предварительного стирания: память
   1887ВЕ7Т это допускает, а РЭ отладочной платы советует не стирать автоматически.
   После записи avrdude сверяет память — сообщение `... bytes of flash verified`.
8. **Проверка лога** (раздел 8.1). Ожидается:

   ```
   DDS 48-52 MHz, fw 1.0.0+<git>
   [boot] MCUCSR=0x.. OSCCAL=0x.. (EEPROM)
   [dds] 1508PL8T ready (DEVID ok)
   [app] N=... f=... Hz FTW=0x... profile=1
   ```

   Если вместо `DEVID ok` идёт `init: error DEVID` — нет такта 480 МГц на XW1, нет +1,8/+3,3 В
   у D1 или обрыв на линиях SPI/D3/D4.
9. Анализатором спектра проверить частоту и уровень и подобрать `DDS_AMPLITUDE_MUL` под −10 дБм.

**Если запись не сверяется.** Профиль `1887ve7t` использует страницу 64 байта из XML
производителя. ТО МК местами говорит о 128 словах. Порядок действий:

1. повторить `FW: Erase chip` и `FW: Flash (Debug)`;
2. если ошибка повторяется — записать HEX из Microchip Studio 7 или Atmel Studio 6.2 с профилем
   НИИЭТ (`info/1887BE1T/xml files`) и сообщить результат: профиль avrdude нужно будет уточнить.

---

## 7. Повседневная работа

```
правка кода → Ctrl+Shift+B → Tasks: FW: Flash (Debug) → смотреть лог / отлаживать
```

- `Debug` — для разработки: лог в UART, `-Og`, сторожевой таймер выключен (не мешает
  остановкам в отладчике), фьюз OCDEN = 0.
- `Release` — для изделия: без лога, `-Os`, WDT 1 с, OCDEN = 1. Перед выпуском:
  `FW: Write fuses (Release)` → `FW: Flash (Release)`.
- Версия прошивки (`project(VERSION …)` + `git describe`) печатается в первой строке лога.

---

## 8. Отладка

### 8.1. UART-лог — самый быстрый способ

USB-UART адаптер: **RX адаптера → X1.1**, **GND → X1.6**. Настройки 38400, 8N1.
Вход адаптера должен выдерживать 5 В.
Терминал — любой: PuTTY, расширение Serial Monitor в VS Code, `screen` в WSL.

Площадки X1 — это и ISP. При программировании через ISP адаптер отключить.
При работе через JTAG (X3) X1 свободен.

### 8.2. Пошаговая отладка в VS Code / Cursor: WSL2 + avarice + avr-gdb

Брейкпоинты, шаги и переменные — прямо в редакторе. Для Atmel-ICE и megaAVR по JTAG
GDB-сервер есть только под Linux (avarice), поэтому он работает в WSL2, а USB пробрасывается
через usbipd-win.

**Подготовка (один раз):**

```powershell
wsl --install -d Ubuntu-24.04
winget install usbipd
```

В Ubuntu (WSL):

```bash
sudo apt update
sudo apt install cmake ninja-build pkg-config gcc-avr avr-libc binutils-avr gdb-avr \
                 avrdude avarice simavr libsimavr-dev libelf-dev gcc
# доступ к Atmel-ICE без sudo
echo 'SUBSYSTEM=="usb", ATTRS{idVendor}=="03eb", ATTRS{idProduct}=="2141", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="03eb", ATTRS{idProduct}=="2141", MODE="0666"' \
  | sudo tee /etc/udev/rules.d/99-atmel-ice.rules
sudo udevadm control --reload-rules
```

Проброс Atmel-ICE в WSL. В PowerShell **от администратора** один раз:

```powershell
usbipd list                       # найти BUSID устройства 03eb:2141 (Atmel-ICE)
usbipd bind --busid <BUSID>
```

**Сеанс отладки:**

1. Открыть проект в режиме WSL: `Ctrl+Shift+P → WSL: Open Folder in WSL…` →
   `/mnt/c/Project/DDS/firmware`. В Cursor — `Remote-WSL: Open Folder…`.
   Теперь сборка, avrdude, avarice и avr-gdb работают внутри Linux.
2. Задача `USB: attach Atmel-ICE to WSL` (или `usbipd attach --wsl --hardware-id 03eb:2141`
   в PowerShell). Пока ICE подключён к WSL, Windows его не видит.
3. `FW: Flash (Debug)` — прошивка изнутри WSL. В Ubuntu 24.04 avrdude 7.1 — профиль 1887ВЕ7Т работает.
4. Задача `Debug: avarice server :4242`. avarice сам определяет ATmega128 по JTAG ID и ждёт GDB.
5. **F5 → «AVR JTAG: Atmel-ICE + avarice»**. GDB подключится, МК остановлен после сброса.
   Поставьте брейкпоинт (например, в `apply_from_dip` в `app.c`) и нажмите Continue.
6. По окончании — `USB: detach Atmel-ICE from WSL`, чтобы ICE снова был виден в Windows.

Ограничения JTAG megaAVR: **3 аппаратных брейкпоинта**, программных нет.
Каждый watchpoint занимает один брейкпоинт (avarice(1)).
Нужны фьюзы OCDEN = 0 и JTAGEN = 0 (Debug) и отсутствие lock-битов.

Если в Windows-тулчейне есть `avr-gdb.exe` (он входит в сборку ZakKemble), F5 работает и из
обычного (не WSL) окна. avarice при этом всё равно запускается в WSL, а порт 4242 виден из
Windows как `localhost:4242`.

### 8.3. Запасной путь: Microchip Studio 7

Если WSL не подходит, писать код и прошивать — в VS Code/Cursor, а шагать — в Microchip Studio:

1. Записать прошивку задачей `FW: Flash (Debug)`.
2. Microchip Studio: **File → Open → Open Object File For Debugging** →
   `build\avr-debug\dds_fw.elf` → устройство **ATmega128** (сигнатура и JTAG ID совпадают) →
   инструмент **Atmel-ICE**, интерфейс **JTAG**.
3. В свойствах инструмента: **Programming settings → Skip programming**. Studio не будет
   перезаписывать память алгоритмом ATmega128 со страницей 256 байт.
4. **Debug → Start Debugging and Break**.

После каждой пересборки снова прошить из VS Code и перезапустить сеанс в Studio.

### 8.4. Без платы: симулятор и unit-тесты

- **Unit-тесты** — `Test: unit tests (ПК)`. Проверяют расчёт приращения фазы для всех
  N = 0…4000 двумя независимыми способами, а также декодирование DIP и дребезг.
- **Симуляция** — `Test: simulation simavr (Linux/WSL)`. Запускает настоящий `dds_fw.elf`
  в simavr. На шине SPI работает модель 1508ПЛ8Т по ТО: команды из табл. 6.1, регистры
  из табл. 5.1. Сценарий:

  | № | Что проверяется |
  |---|---|
  | 0 | калибровка RC из EEPROM |
  | 1 | без +3,3 В нет кадров SPI, D3 выключен |
  | 2 | старт, N = 1234 → dPh = 0x1A421506B0AD |
  | 3 | кнопка, N = 4000 → второй профиль |
  | 4 | N = 4001 → отказ |
  | 5 | сбой DDS → восстановление прежней частоты |
  | 6 | пропадание +3,3 В |
  | 7 | дребезг кнопки → ровно одна смена |

- **Шаги в симуляторе** — задача `Debug: simavr server :1234`, затем **F5 → «AVR sim: simavr»**.

---

## 9. Фьюзы

0 — бит запрограммирован. Значения заданы в `CMakeLists.txt` и пресетах.

| Байт | Debug | Release | Биты |
|---|---|---|---|
| Low | 0x24 | 0x24 | BODLEVEL = 0 (4,0 В), BODEN = 0 (BOD вкл.), SUT = 10, CKSEL = 0100 (RC 8 МГц) |
| High | **0x11** | **0x91** | OCDEN (0 — отладка), JTAGEN = 0, SPIEN = 0, CKOPT = 1, EESAVE = 0 (EEPROM сохраняется при стирании), BOOTSZ = 00, BOOTRST = 1 |
| Ext | 0xFF | 0xFF | **M103C = 1** (режим ATmega128), WDTON = 1 (WDT включает программа) |
| Lock | 0xFF | 0xFF | не программировать: защита блокирует JTAG-отладку |

Не менять одновременно CKSEL и интерфейс программирования. После смены CKSEL
скорость JTAG/ISP должна оставаться меньше F_CPU/4.

---

## 10. Если что-то пошло не так

| Симптом | Причина и действие |
|---|---|
| `probe`: нет связи по JTAG | Проверить питание платы (VTG ≈ 5 В), шлейф в порт AVR, ключ X3. Попробовать `-B 16`. Если JTAGEN был отключён — ISP: `cmake --preset avr-debug -DAVRDUDE_PROGRAMMER=atmelice_isp`, Atmel-ICE через ISP-переходник к площадкам X1, затем записать фьюзы. |
| Прошивка «не стартует», в логе мусор | Нет калибровки (лог покажет `factory 1 MHz`) — выполнить шаг 6. Или E-фьюз 0xFD (M103C) — записать 0xFF. |
| `init: error DEVID` | Нет такта 480 МГц на XW1 (S1 замкнут, CSEL = 1), нет +1,8/+3,3 В у D1, не включается D3 (PB5), обрыв MISO через D4. |
| `set_ftw: error VERIFY` | Запись в DDS не подтверждается чтением. Проверить SPI осциллографом: SCK 62,5 кГц, SCSn поднимается после каждых 24 бит. |
| Частота верная, уровень не тот | `DDS_AMPLITUDE_MUL` в `config/dds_config.h`. |
| avrdude: `verification error` | Раздел 6, «Если запись не сверяется». |

---

## 11. Настройка изделия

Всё в `config/dds_config.h`:

| Параметр | По умолчанию | Смысл |
|---|---|---|
| `DDS_F_BASE_HZ`, `DDS_F_STEP_HZ`, `DDS_N_MAX` | 48 МГц, 1 кГц, 4000 | шкала частот |
| `DDS_FCLK_HZ` | 480 МГц | такт ЦАП (XW1 или G1) |
| `DDS_CLK_CMX` | 0 | делитель такта при CSEL = 0 (G1) |
| `DDS_AMPLITUDE_MUL` | 4095 | амплитуда, A = mul/4096, калибруется на −10 дБм |
| `DDS_TSW` | 0 | плавное переключение частоты (гауссов фильтр) |
| `DDS_SPI_PRESCALER` | 128 | SPI 62,5 кГц, по design_notes не быстрее 100 кГц |
| `DDS_HEALTH_PERIOD_MS` | 1000 | период проверки DDS, 0 — выкл. |

Проверки на этапе компиляции (`_Static_assert`) не дадут собрать прошивку с частотой
выше 0,4·Fclk или с кодом шире 12 разрядов.

---

## 12. Как создать такой проект с нуля (для другой платы)

1. Каталог проекта: `src/` (код), `config/` (параметры изделия), `cmake/`, `tests/`, `.vscode/`.
2. `cmake/avr-gcc.cmake` — toolchain-файл:
   - `CMAKE_SYSTEM_NAME Generic`;
   - компилятор `avr-gcc`;
   - `CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY`.
3. `CMakeLists.txt`:
   - `add_executable` и флаги `-mmcu=…`, `F_CPU`, `-ffunction-sections -Wl,--gc-sections`;
   - post-build: `avr-objcopy -O ihex`, `avr-size`.
4. `CMakePresets.json`:
   - пресеты Debug/Release с `toolchainFile`;
   - `CMAKE_EXPORT_COMPILE_COMMANDS=ON` — для clangd;
   - отдельный пресет для тестов на ПК.
5. Цели программирования — `add_custom_target(flash COMMAND avrdude …)`.
   Если МК нет в avrdude — профиль `part parent "<аналог>"` и подключение через `-C +файл`.
6. `.vscode/tasks.json` — задачи вызывают `cmake --build --preset … --target …`.
   Одни и те же задачи работают в VS Code, Cursor, Windows и WSL.
7. `.vscode/launch.json` — Native Debug, `target: localhost:<порт GDB-сервера>`.
8. Логику без обращений к регистрам — в отдельные модули (`core/`) и покрыть unit-тестами.
   Драйверы проверять в simavr с моделью внешней микросхемы.

---

## 13. Источники

- «Микросхема интегральная 1508ПЛ8Т. Техническое описание», АО НПЦ «ЭЛВИС», 14.10.2020:
  - табл. 5.1–5.21 — регистры;
  - табл. 6.1, разд. 6.3 — SPI;
  - разд. 3.1 — формула частоты.
- «Управление микросхемой 1508ПЛ8Т через порт SPI», Appnote ЭЛВИС, 14.08.2019.
- ТО 1887ВЕ7Т КФДЛ.431295.044ТО и сводка `info/1887BE1T/1887ve7t-summary.md`.
  Профиль Atmel Studio 6 `info/1887BE1T/xml files/Atmel Studio 6/devices/1887BE7T.xml`.
- `docs/design_notes.md` (разд. 2.3, 4, 5), `docs/REVISION_F.md`, нетлист `output/netlist/netlist.xml`.
- avrdude: https://avrdudes.github.io/avrdude/
- avarice(1): https://manpages.ubuntu.com/manpages/noble/man1/avarice.1.html
