//! Static texts and drawings shared with the web UI. Mirrors
//! `src/lib/blocks.ts`, `src/lib/board.ts` and `src/lib/lessons.ts` — keep them in
//! sync (the `content_matches_web` test checks that every string here is present
//! in the TS files).

use crate::app::View;

pub struct Block {
    pub id: &'static str,
    pub title: &'static str,
    /// Colour token: sig | clk | ref | dig | pwr | ctl.
    pub tone: &'static str,
    pub desc: &'static str,
    /// Position on the overview drawing (viewBox 60 118 1120 490).
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Block lives on the board, not in the chip.
    #[allow(dead_code)] // kept in sync with blocks.ts
    pub board: bool,
}

/// Blocks of the 1508ПЛ8Т functional diagram (ТО, рис. 3.1) plus the board's output chain.
pub const BLOCKS: &[Block] = &[
    Block { id: "clk", title: "Такт", tone: "clk", desc: "Вход такта CLKDP/CLKDM (XW1 при замкнутом S1) или CLKP/CLKM (G1), делитель CTR.cmx. Такт нужен и для SPI: без него регистры не пишутся.", x: 100.0, y: 230.0, w: 140.0, h: 90.0, board: false },
    Block { id: "sync", title: "SYNC / LINK", tone: "ctl", desc: "Синхронизация нескольких кристаллов, выбор профиля выводами SEL, LINK-порт. На плате не используются: SEL подтянуты к земле.", x: 100.0, y: 355.0, w: 140.0, h: 80.0, board: false },
    Block { id: "spi", title: "SPI", tone: "dig", desc: "Последовательный порт: кадры 24 бита (команда 8 + данные 16). SETA, WR, WRI, SETAFT, NOP. На плате ведущий — контроллер D2 через буферы D3/D4.", x: 100.0, y: 470.0, w: 140.0, h: 90.0, board: false },
    Block { id: "prof1", title: "Профили К1", tone: "dig", desc: "64 профиля канала 1: приращение фазы dPh (48 бит), фаза P, амплитуда Mul, смещение Offset. Активный выбирается SEL_REG.Pr_1 — так делается бесшовная смена частоты и манипуляции.", x: 280.0, y: 230.0, w: 160.0, h: 90.0, board: false },
    Block { id: "lfm1", title: "ЛЧМ К1", tone: "dig", desc: "Генератор линейной частотной модуляции канала 1: 4 стадии (нарастание, пауза, спад, пауза), скорость dF и длительности TPH.", x: 280.0, y: 355.0, w: 160.0, h: 80.0, board: false },
    Block { id: "acc1", title: "Аккумулятор К1", tone: "sig", desc: "48-битный аккумулятор фазы: каждый такт прибавляет dPh. Старшие 17 бит фазы (+ P, + дизеринг) идут в таблицу синуса. Fout = dPh·Fclk / 2⁴⁸.", x: 480.0, y: 230.0, w: 170.0, h: 90.0, board: false },
    Block { id: "ch2", title: "Канал 2", tone: "sig", desc: "Второй такой же канал (профили, ЛЧМ, аккумулятор). На плате выход ЦАП 2 не выведен, но канал 2 можно подать на ЦАП 1 через ROUTE.sum.", x: 480.0, y: 470.0, w: 170.0, h: 90.0, board: false },
    Block { id: "route", title: "ROUTE", tone: "dig", desc: "Маршрутизация каналов к ЦАП: канал 1, канал 2 или их сумма; дизеринг фазы и амплитуды.", x: 690.0, y: 300.0, w: 130.0, h: 120.0, board: false },
    Block { id: "dac1", title: "ЦАП 1", tone: "sig", desc: "10-битный токовый ЦАП, выходы OUTP1/OUTM1. Ток полной шкалы IFS = 72 В / RREF (R11 = 7,15 кОм → 10 мА). Ступенчатый выход: спад sin(x)/x и образы k·Fclk ± f.", x: 860.0, y: 230.0, w: 130.0, h: 90.0, board: false },
    Block { id: "dac2", title: "ЦАП 2", tone: "sig", desc: "Второй ЦАП. На плате его выход нагружен резисторами и не выведен на разъём.", x: 860.0, y: 355.0, w: 130.0, h: 70.0, board: false },
    Block { id: "cmp", title: "Компаратор", tone: "ctl", desc: "Встроенный компаратор для формирования меандра. На плате не используется.", x: 860.0, y: 470.0, w: 130.0, h: 90.0, board: false },
    Block { id: "lpf", title: "ФНЧ", tone: "ref", desc: "Плата: R44 49,9 Ом, C5, фильтр 7-го порядка L1–L7/C6–C11 (срез ≈ 70 МГц) и аттенюатор ≈ 1 дБ (R45–R47). Подавляет образы Fclk − f.", x: 1025.0, y: 230.0, w: 120.0, h: 90.0, board: true },
    Block { id: "out", title: "XW2", tone: "sig", desc: "Выход платы, SMA 50 Ом. Цель — около −10 дБм в полосе 48–52 МГц.", x: 1025.0, y: 355.0, w: 120.0, h: 70.0, board: true },
    Block { id: "pwr", title: "Питание", tone: "pwr", desc: "Плата: +15 В на X2 → D5 (+5 В) → D6 (+3,3 В) и D7 (+1,8 В ядра DDS). DDS_PGOOD сообщает контроллеру о +3,3 В.", x: 1025.0, y: 470.0, w: 120.0, h: 90.0, board: true },
];

/// Wires between blocks: (from, to, tone).
pub const WIRES: &[(&str, &str, &str)] = &[
    ("clk", "prof1", "clk"),
    ("prof1", "acc1", "dig"),
    ("lfm1", "acc1", "dig"),
    ("acc1", "route", "sig"),
    ("ch2", "route", "sig"),
    ("route", "dac1", "sig"),
    ("route", "dac2", "sig"),
    ("dac1", "lpf", "sig"),
    ("lpf", "out", "sig"),
    ("sync", "spi", "ctl"),
];

pub fn block(id: &str) -> Option<&'static Block> {
    BLOCKS.iter().find(|b| b.id == id)
}

// ───────────── board (board.ts) ─────────────

pub struct Hotspot {
    pub id: &'static str,
    /// Designator as printed on the PCB.
    pub r#ref: &'static str,
    pub title: &'static str,
    /// connector | jumper | power | testpoint | chip | led | switch | filter
    pub kind: &'static str,
    pub desc: &'static str,
    /// Position on the 1000×440 board drawing.
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Block id to jump to on the overview.
    pub block: Option<&'static str>,
}

pub const BOARD_NAME: &str = "DDS 48–52 МГц, ревизия F";
pub const BOARD_SIZE: &str = "120 × 50 мм";
pub const BOARD_SUPPLY: &str = "+15 В на X2 (0,25–0,30 А, расчёт)";

pub const HOTSPOTS: &[Hotspot] = &[
    Hotspot { id: "d1", r#ref: "D1", title: "1508ПЛ8Т — синтезатор DDS", kind: "chip", desc: "Двухканальный DDS, корпус LQFP-100. Такт 480 МГц с XW1, выход ЦАП 1 → ФНЧ → XW2. Питание +3,3 В (периферия) и +1,8 В (ядро).", x: 724.0, y: 140.0, w: 128.0, h: 128.0, block: Some("acc1") },
    Hotspot { id: "d2", r#ref: "D2", title: "1887ВЕ7Т — контроллер", kind: "chip", desc: "AVR-совместимый МК (как ATmega128), RC 8 МГц, питание +5 В. Читает DIP-переключатели и пишет регистры DDS по SPI; с прошивкой 1.1 принимает команды DDS Studio по USART0 (X1).", x: 332.0, y: 108.0, w: 128.0, h: 128.0, block: Some("spi") },
    Hotspot { id: "d3", r#ref: "D3", title: "1564АП3 — буфер SPI", kind: "chip", desc: "Буфер 5 В → 3,3 В для SCK, MOSI, CS и RSTn. Включается выходом BUF_OEn (PB5) только при наличии +3,3 В.", x: 508.0, y: 64.0, w: 80.0, h: 104.0, block: Some("spi") },
    Hotspot { id: "d4", r#ref: "D4", title: "1594ТЛ2 — обратный канал", kind: "chip", desc: "Согласует SDO DDS (3,3 В) с входом MISO контроллера (5 В).", x: 584.0, y: 208.0, w: 56.0, h: 72.0, block: Some("spi") },
    Hotspot { id: "d5", r#ref: "D5", title: "1393ЕУ014 — +5 В", kind: "power", desc: "Импульсный стабилизатор +15 → +5 В (дроссель L8).", x: 88.0, y: 180.0, w: 64.0, h: 64.0, block: Some("pwr") },
    Hotspot { id: "d6", r#ref: "D6", title: "1158ЕН3.3 — +3,3 В", kind: "power", desc: "Линейный стабилизатор +3,3 В для DDS. Его наличие сообщает сигнал DDS_PGOOD (PC4 контроллера).", x: 173.0, y: 196.0, w: 56.0, h: 64.0, block: Some("pwr") },
    Hotspot { id: "d7", r#ref: "D7", title: "1325ЕН1.8 — +1,8 В", kind: "power", desc: "Линейный стабилизатор ядра DDS. Рассеивает заметную мощность — проверьте нагрев (design_notes п. 4.6).", x: 234.0, y: 196.0, w: 56.0, h: 64.0, block: Some("pwr") },
    Hotspot { id: "x2", r#ref: "X2", title: "Питание +15 В", kind: "power", desc: "Контакт 1 — +15 В, контакт 2 — GND. Предохранитель F1 0,5 А, защита VD1/VD2.", x: 30.0, y: 356.0, w: 28.0, h: 48.0, block: Some("pwr") },
    Hotspot { id: "x1", r#ref: "X1", title: "ISP / UART", kind: "connector", desc: "Площадки ISP 2×3. X1.1 = TXD0 (PE1), X1.4 = RXD0 (PE0), X1.6 = GND. Сюда подключается USB-UART адаптер для DDS Studio. При ISP-программировании адаптер отсоединить.", x: 244.0, y: 30.0, w: 48.0, h: 36.0, block: Some("spi") },
    Hotspot { id: "x3", r#ref: "X3", title: "JTAG", kind: "connector", desc: "Разъём 2×5, 1,27 мм для Atmel-ICE: прошивка и отладка D2. Плату не питает.", x: 348.0, y: 26.0, w: 96.0, h: 40.0, block: None },
    Hotspot { id: "xw1", r#ref: "XW1", title: "Вход такта", kind: "connector", desc: "SMA, 480 МГц, проектный уровень 0 дБм на 50 Ом (допустимо −10…+5 дБм, табл. 8.1). Работает при замкнутом S1.", x: 632.0, y: 392.0, w: 48.0, h: 36.0, block: Some("clk") },
    Hotspot { id: "s1", r#ref: "S1", title: "Выбор такта", kind: "jumper", desc: "Замкнут — такт с XW1 (CSEL = 1). Разомкнут — генератор G1, который в ревизии F не установлен.", x: 688.0, y: 372.0, w: 52.0, h: 40.0, block: Some("clk") },
    Hotspot { id: "g1", r#ref: "G1", title: "Генератор (не установлен)", kind: "chip", desc: "Место под генератор ГК361; вариант автономного тактирования не подтверждён (DNP).", x: 674.0, y: 326.0, w: 44.0, h: 28.0, block: Some("clk") },
    Hotspot { id: "lpf", r#ref: "L1–L7", title: "ФНЧ и аттенюатор", kind: "filter", desc: "R44 49,9 Ом → C5 10 нФ → L1 51 нГн, C6 56 пФ, L5 200 нГн, C7 91 пФ, L6 200 нГн, C11 56 пФ, L7 51 нГн → П-аттенюатор R46/R45/R47 (866/5,76/866 Ом) → XW2.", x: 806.0, y: 318.0, w: 132.0, h: 64.0, block: Some("lpf") },
    Hotspot { id: "r11", r#ref: "R11", title: "RREF ЦАП", kind: "testpoint", desc: "7,15 кОм: ток полной шкалы ЦАП IFS = 72 В / 7,15 кОм ≈ 10 мА.", x: 770.0, y: 312.0, w: 30.0, h: 22.0, block: Some("dac1") },
    Hotspot { id: "xw2", r#ref: "XW2", title: "Выход", kind: "connector", desc: "SMA 50 Ом. Цель — около −10 дБм в полосе 48–52 МГц; нагружать на 50 Ом.", x: 944.0, y: 348.0, w: 36.0, h: 48.0, block: Some("out") },
    Hotspot { id: "s2", r#ref: "S2", title: "DIP N[7:0]", kind: "switch", desc: "Младшие 8 бит кода N. ON = 1. Частота f = 48 МГц + 1 кГц · N.", x: 250.0, y: 368.0, w: 180.0, h: 50.0, block: None },
    Hotspot { id: "s3", r#ref: "S3", title: "DIP N[11:8]", kind: "switch", desc: "Старшие 4 бита кода N (N ≤ 4000, иначе код отклоняется).", x: 486.0, y: 368.0, w: 100.0, h: 50.0, block: None },
    Hotspot { id: "s4", r#ref: "S4", title: "Кнопка «Применить»", kind: "switch", desc: "Перечитать DIP и задать новую частоту. В режиме REMOTE не действует.", x: 224.0, y: 318.0, w: 48.0, h: 46.0, block: None },
    Hotspot { id: "tp", r#ref: "TP1/TP2", title: "Контрольные точки", kind: "testpoint", desc: "TP1 — восстановление BQ1, TP2 — GND.", x: 388.0, y: 248.0, w: 40.0, h: 20.0, block: None },
];

/// First power-up with DDS Studio (Board view checklist).
pub const CHECKLIST: &[&str] = &[
    "Питание выключено. Замкните S1, подключите источник 480 МГц 0 дБм к XW1 и анализатор/измеритель 50 Ом к XW2.",
    "Подайте +15 В на X2 (контакт 1 — плюс). Ток — около 0,25–0,30 А.",
    "Прошивка D2 — версия 1.1.0 или новее (с UART-протоколом), записана через X3.",
    "USB-UART адаптер 5 В: RX → X1.1, TX → X1.4, GND → X1.6. Питание адаптера не подключать.",
    "«Подключение» → порт, 38400 бод → «Подключить». Прошивка отвечает PL8T и переходит в REMOTE.",
    "«Проверить связь»: DEVID должен быть 0x0201 (иначе нет такта, питания DDS или SPI).",
    "«Регистры» → «Стартовая» → «Записать всё»: на XW2 50 МГц, уровень около −10 дБм.",
    "Сверьте уровень на анализаторе и подберите mul на вкладке «Частота».",
    "По окончании — «Вернуть плате» (REMOTE 0): частоту снова задают DIP-переключатели.",
];

pub struct Wire {
    pub mcu: &'static str,
    pub board: &'static str,
    pub signal: &'static str,
    pub dir: &'static str,
    pub note: &'static str,
}

/// USB-UART adapter wiring (Connection view, docs/PROTOCOL_ru.md).
pub const WIRING: &[Wire] = &[
    Wire {
        mcu: "RX", board: "X1.1", signal: "TXD0 (PE1)", dir: "←", note: "ответы и журнал платы"
    },
    Wire { mcu: "TX", board: "X1.4", signal: "RXD0 (PE0)", dir: "→", note: "команды; логика 5 В" },
    Wire { mcu: "GND", board: "X1.6", signal: "GND", dir: "—", note: "подключить первым" },
    Wire {
        mcu: "VCC", board: "—", signal: "не подключать", dir: "—", note: "плата питается от X2"
    },
];

/// Hardware safety notes. Keep them in the UI and in docs.
pub const SAFETY: &[&str] = &[
    "Контроллер D2 работает от +5 В: нужен адаптер с логикой 5 В. Адаптер 3,3 В допустим, только если его RX выдерживает 5 В; уровень 3,3 В на X1.4 — на границе VIH = 0,6·VCC = 3 В.",
    "Питание адаптера (VCC) к плате не подключать — плата питается только от +15 В через X2.",
    "X1 — это и площадки ISP: на время ISP-программирования адаптер отсоединить. Программирование через JTAG (X3) адаптеру не мешает.",
    "Соединения делайте при выключенном питании, общий провод (GND) — первым.",
    "Такт на XW1: от −10 до +5 дБм (ТО, табл. 8.1). Без такта SPI не работает, DEVID не читается.",
    "XW2 нагружайте на 50 Ом и не подавайте на него внешний сигнал; выходы ЦАП допускают не более ±0,6 В.",
];

// ───────────── academy (lessons.ts) ─────────────

pub struct Lesson {
    #[allow(dead_code)] // kept in sync with lessons.ts
    pub id: &'static str,
    pub title: &'static str,
    /// Paragraphs; "§ text" — sub-heading, "$ text" — formula block, "- text" — bullet.
    pub body: &'static [&'static str],
    /// (button label, view, block)
    pub try_it: Option<(&'static str, View, Option<&'static str>)>,
}

pub const LESSONS: &[Lesson] = &[
    Lesson {
        id: "dds",
        title: "Что такое DDS",
        body: &[
            "Прямой цифровой синтез (DDS) строит синусоиду из чисел. Ни ГУН, ни петли ФАПЧ нет: частота задаётся числом в регистре и меняется за несколько тактов, без переходного процесса.",
            "§ Четыре шага на каждом такте",
            "- Аккумулятор фазы прибавляет к себе приращение dPh (48 бит). Его значение — это фаза, «намотанная» на окружность 0…2⁴⁸.",
            "- Старшие биты фазы (17 бит, плюс фазовый сдвиг P и дизеринг) превращаются в отсчёт синуса.",
            "- Отсчёт умножается на амплитуду Mul и складывается со смещением Offset.",
            "- 10-битный ЦАП превращает код в ток. Ступеньки ЦАП сглаживает фильтр нижних частот платы.",
            "В 1508ПЛ8Т два таких канала. На плате используется канал 1 и ЦАП 1, выход — разъём XW2.",
        ],
        try_it: Some(("Открыть блок-схему", View::Overview, Some("acc1"))),
    },
    Lesson {
        id: "ftw",
        title: "Частота и шаг сетки",
        body: &[
            "Аккумулятор переполняется Fclk·dPh/2⁴⁸ раз в секунду — это и есть выходная частота:",
            "$ Fout = dPh · Fclk / 2⁴⁸        dPh = round(Fout · 2⁴⁸ / Fclk)",
            "При такте 480 МГц шаг сетки — 480 МГц / 2⁴⁸ ≈ 1,705 мкГц. Любую частоту из полосы 48–52 МГц можно задать с ошибкой меньше 1 мкГц.",
            "§ Пример: 50 МГц",
            "$ dPh = 50/480 · 2⁴⁸ = 29 320 310 074 026,67 → 0x1AAA AAAA AAAB",
            "Число 48-битное, поэтому пишется тремя регистрами: dPh_L [15:0], dPh_M [31:16], dPh_H [47:32]. Прошивка платы округляет так же — половина вверх.",
            "Частота на выходе зависит от точности такта: ошибка источника 480 МГц в 1 ppm даёт 50 Гц на 50 МГц — гораздо больше шага сетки.",
        ],
        try_it: Some(("Рассчитать частоту", View::Freq, None)),
    },
    Lesson {
        id: "profiles",
        title: "Профили и бесшовная смена частоты",
        body: &[
            "У каждого канала 64 профиля (ТО, разд. 3.2). Профиль — это набор dPh, P, Mul, Offset. Активный профиль выбирается полем Pr_1 регистра SEL_REG; остальные можно переписывать «на лету».",
            "§ Почему не писать частоту прямо в активный профиль",
            "48-битное число пишется тремя кадрами SPI. Между кадрами в аккумулятор попадёт смесь старых и новых битов — на выходе мелькнёт случайная частота.",
            "§ Как делает прошивка и приложение",
            "- Новая частота пишется в неактивный профиль (0 или 1).",
            "- Запись проверяется чтением.",
            "- Одна запись SEL_REG переключает профиль — частота меняется за такт, фаза не рвётся.",
            "Поле TSW включает гауссово сглаживание: переход занимает несколько тактов и даёт меньше всплесков в спектре.",
        ],
        try_it: Some(("Профили канала 1", View::Hop, None)),
    },
    Lesson {
        id: "amp",
        title: "Амплитуда, фаза и смещение",
        body: &[
            "Отсчёт синуса умножается на Mul (13 бит со знаком) и складывается со смещением Offset:",
            "$ A = Mul / 4096        φ = P · 360° / 2¹⁶",
            "Отрицательный Mul инвертирует сигнал — это сдвиг фазы на 180°. Offset сдвигает сигнал по постоянному току: через разделительный конденсатор C5 он не пройдёт, но займёт запас ЦАП.",
            "§ Уровень на XW2",
            "Ток полной шкалы ЦАП задаёт резистор R11:",
            "$ IFS = 72 В / RREF = 72 / 7150 ≈ 10,1 мА",
            "Ток течёт в R44 49,9 Ом параллельно с фильтром на 50 Ом, дальше — фильтр и аттенюатор ≈ 1 дБ. Модель даёт около −9,8 дБм при Mul = 4095; для −10 дБм нужен Mul ≈ 4010. Точное значение подбирается по анализатору.",
        ],
        try_it: Some(("Подобрать уровень", View::Freq, None)),
    },
    Lesson {
        id: "images",
        title: "Образы и sin(x)/x",
        body: &[
            "ЦАП держит каждый отсчёт до следующего такта — получаются ступеньки. В спектре ступенчатого сигнала, кроме несущей f, есть образы на k·Fclk ± f:",
            "$ 480 − 50 = 430 МГц,   480 + 50 = 530 МГц,   960 − 50 = 910 МГц …",
            "Их огибающая — функция sin(x)/x:",
            "$ H(f) = sin(π f / Fclk) / (π f / Fclk)",
            "На 50 МГц спад всего 0,15 дБ, а образ на 430 МГц ослаблен ЦАПом лишь на ≈ 19 дБ. Его давит ФНЧ платы 7-го порядка (срез ≈ 70 МГц) — на 430 МГц ослабление больше 100 дБ.",
            "Поэтому частоту держат ниже 0,4·Fclk: ближе к Fclk/2 образ подходит к несущей и фильтр уже не успевает его подавить.",
        ],
        try_it: Some(("Посмотреть спектр", View::Spectrum, None)),
    },
    Lesson {
        id: "spurs",
        title: "Откуда берутся помехи",
        body: &[
            "Кроме образов, в спектре DDS есть паразитные составляющие (spurs):",
            "- Усечение фазы: в таблицу синуса идут только 17 старших бит из 48. Отброшенные биты дают периодическую ошибку и дискретные спуры. Дизеринг фазы (ROUTE.pdith1) размывает их в шум.",
            "- Квантование амплитуды: 10-битный ЦАП, теоретический предел ≈ 62 дБ отношения сигнал/шум.",
            "- Нелинейность ЦАП: гармоники 2f, 3f… Их в цифровой модели нет — приложение добавляет их по таблице SFDR (ТО, табл. 8.1: 67 дБ на 26 МГц, 63 дБ на 126 МГц).",
            "- Гармоники, отразившиеся от Fclk: 3·50 = 150 МГц, 480 − 2·50 = 380 МГц и т. д.",
            "На вкладке «Спектр» верхний график — цифровые отсчёты (бит-в-бит модель кристалла), нижний — что увидит анализатор на XW2.",
        ],
        try_it: Some(("Спектр и SFDR", View::Spectrum, None)),
    },
    Lesson {
        id: "chain",
        title: "Выходной тракт платы",
        body: &[
            "OUTP1 → R44 49,9 Ом на землю → C5 10 нФ → фильтр L1 51 нГн, C6 56 пФ, L5 200 нГн, C7 91 пФ, L6 200 нГн, C11 56 пФ, L7 51 нГн → П-аттенюатор 866/5,76/866 Ом → XW2.",
            "Приложение считает тракт через ABCD-матрицы с конечной добротностью катушек (Q = 30 на 50 МГц, задаётся в настройках) и показывает АЧХ, sin(x)/x и итоговый уровень.",
            "Напряжение на выходах ЦАП не должно выходить за ±0,6 В (ТО, разд. 6.4). На OUTP1 ток делится между R44 и фильтром (≈ 25 Ом) — около 0,25 В. Второй выход OUTM1 нагружен только R25 49,9 Ом: при полной шкале 10 мА там ≈ 0,5 В — запас до предела небольшой, поэтому RREF меньше 6 кОм ставить нельзя.",
        ],
        try_it: Some(("АЧХ тракта", View::Chain, None)),
    },
    Lesson {
        id: "noise",
        title: "Фазовый шум и джиттер",
        body: &[
            "DDS делит частоту такта: фазовый шум источника 480 МГц переносится на выход с уменьшением на 20·lg(Fclk/Fout) — для 50 МГц это ≈ 19,6 дБ.",
            "Собственный шум кристалла (ТО, табл. 8.1) задан для такта 1 ГГц: около −145 дБн/Гц при отстройке 1 кГц на 10 МГц.",
            "Интеграл фазового шума по полосе отстроек — это джиттер. Приложение показывает кривую и джиттер для текущей частоты.",
        ],
        try_it: Some(("Шум и джиттер", View::Chain, None)),
    },
    Lesson {
        id: "lfm",
        title: "ЛЧМ: линейная частотная модуляция",
        body: &[
            "Генератор ЛЧМ меняет приращение фазы на dF каждый такт. Цикл из четырёх стадий:",
            "- 1: частота растёт со скоростью dF1, длительность TPH1;",
            "- 2: пауза TPH2;",
            "- 3: частота меняется со скоростью dF2 (для «треугольника» — обратно);",
            "- 4: пауза TPH4, затем всё сначала.",
            "$ скорость = dF · Fclk² / 2⁴⁸ Гц/с        T = TPH · 4 / Fclk",
            "Запуск — записью CLR.LS1_start, остановка — CLR.LS1_stop. Пока ЛЧМ включён, профили частоты не используются.",
        ],
        try_it: Some(("Настроить ЛЧМ", View::Mod, None)),
    },
    Lesson {
        id: "mod",
        title: "Манипуляции через профили",
        body: &[
            "Цифровая модуляция в 1508ПЛ8Т — это переключение заранее записанных профилей:",
            "- ЧМн (FSK): профили отличаются частотой;",
            "- ФМн (BPSK, QPSK, 8-PSK): профили отличаются фазой P;",
            "- АМн (ASK): профили отличаются амплитудой Mul;",
            "- КАМ-16: и фазой, и амплитудой.",
            "Символ выбирается записью Pr_1 в SEL_REG. Прошивка платы умеет перебирать профили сама (команда HOP) — так можно увидеть манипуляцию на анализаторе без потока команд с ПК. На кристалле профиль можно выбирать и выводами SEL, но на плате они подтянуты к земле.",
        ],
        try_it: Some(("Манипуляции", View::Mod, None)),
    },
    Lesson {
        id: "link",
        title: "Как приложение говорит с платой",
        body: &[
            "Приложение не трогает SPI напрямую: им владеет контроллер D2. Начиная с прошивки 1.1 он принимает текстовые команды по USART0 (X1.1/X1.4, 38400 8N1).",
            "- «?» — кто ты: PL8T <версия> DDS48-52;",
            "- «REMOTE 1» — DIP и кнопка отключаются, регистры пишет ПК;",
            "- «W 1400 AAAB» — записать регистр (кадры SETA + WR);",
            "- «R 0001» — прочитать (SETAFT + NOP), ответ «OK 0201»;",
            "- «HOP 8 100» — перебирать профили 0…7 по 100 мс;",
            "- «REMOTE 0» — вернуть плату переключателям.",
            "Каждый кадр SPI — 24 бита: команда (8) и данные (16). Чтение конвейерное: данные приходят в следующем кадре. Все кадры видны в журнале шины (⌘/Ctrl + J).",
        ],
        try_it: Some(("Подключение", View::Hardware, None)),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn web(file: &str) -> String {
        std::fs::read_to_string(format!("{}/../../src/lib/{}", env!("CARGO_MANIFEST_DIR"), file)).unwrap()
    }

    /// TS source spells strings with \" escapes; compare against the raw text.
    fn has(src: &str, s: &str) -> bool {
        src.contains(s) || src.contains(&s.replace('"', "\\\""))
    }

    #[test]
    fn content_matches_web() {
        let blocks = web("blocks.ts");
        for b in BLOCKS {
            for s in [b.id, b.title, b.desc] {
                assert!(has(&blocks, s), "blocks.ts lost: {}", s);
            }
        }
        let board = web("board.ts");
        for s in HOTSPOTS
            .iter()
            .flat_map(|h| [h.r#ref, h.title, h.desc])
            .chain(CHECKLIST.iter().copied())
            .chain(SAFETY.iter().copied())
        {
            assert!(has(&board, s), "board.ts lost: {}", s);
        }
        let lessons = web("lessons.ts");
        for l in LESSONS {
            for s in std::iter::once(l.title).chain(l.body.iter().copied()) {
                assert!(has(&lessons, s), "lessons.ts lost: {}", s);
            }
        }
        assert_eq!(blocks.matches("{ id: \"").count(), BLOCKS.len(), "blocks.ts has a block that is not here");
        assert_eq!(lessons.matches("    id: \"").count(), LESSONS.len(), "lessons.ts has a lesson that is not here");
    }
}
