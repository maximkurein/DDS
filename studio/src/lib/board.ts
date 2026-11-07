// Facts about the DDS 48–52 МГц board, revision F. Positions come from
// DDS_48_52.kicad_pcb (footprint centres, 1 мм = 8 px, origin at the board
// corner 100/100 мм); connector roles — from the schematic and USER_GUIDE.md.

export interface Hotspot {
  id: string;
  /** Designator as printed on the PCB. */
  ref: string;
  title: string;
  kind: "connector" | "jumper" | "power" | "testpoint" | "chip" | "led" | "switch" | "filter";
  desc: string;
  /** Position on the 1000×440 board drawing. */
  x: number;
  y: number;
  w: number;
  h: number;
  /** Optional block id to jump to on the overview. */
  block?: string;
}

export const BOARD = {
  name: "DDS 48–52 МГц, ревизия F",
  size: "120 × 50 мм",
  supply: "+15 В на X2 (0,25–0,30 А, расчёт)",
};

export const HOTSPOTS: Hotspot[] = [
  { id: "d1", ref: "D1", title: "1508ПЛ8Т — синтезатор DDS", kind: "chip", desc: "Двухканальный DDS, корпус LQFP-100. Такт 480 МГц с XW1, выход ЦАП 1 → ФНЧ → XW2. Питание +3,3 В (периферия) и +1,8 В (ядро).", x: 724, y: 140, w: 128, h: 128, block: "acc1" },
  { id: "d2", ref: "D2", title: "1887ВЕ7Т — контроллер", kind: "chip", desc: "AVR-совместимый МК (как ATmega128), RC 8 МГц, питание +5 В. Читает DIP-переключатели и пишет регистры DDS по SPI; с прошивкой 1.1 принимает команды DDS Studio по USART0 (X1).", x: 332, y: 108, w: 128, h: 128, block: "spi" },
  { id: "d3", ref: "D3", title: "1564АП3 — буфер SPI", kind: "chip", desc: "Буфер 5 В → 3,3 В для SCK, MOSI, CS и RSTn. Включается выходом BUF_OEn (PB5) только при наличии +3,3 В.", x: 508, y: 64, w: 80, h: 104, block: "spi" },
  { id: "d4", ref: "D4", title: "1594ТЛ2 — обратный канал", kind: "chip", desc: "Согласует SDO DDS (3,3 В) с входом MISO контроллера (5 В).", x: 584, y: 208, w: 56, h: 72, block: "spi" },
  { id: "d5", ref: "D5", title: "1393ЕУ014 — +5 В", kind: "power", desc: "Импульсный стабилизатор +15 → +5 В (дроссель L8).", x: 88, y: 180, w: 64, h: 64, block: "pwr" },
  { id: "d6", ref: "D6", title: "1158ЕН3.3 — +3,3 В", kind: "power", desc: "Линейный стабилизатор +3,3 В для DDS. Его наличие сообщает сигнал DDS_PGOOD (PC4 контроллера).", x: 173, y: 196, w: 56, h: 64, block: "pwr" },
  { id: "d7", ref: "D7", title: "1325ЕН1.8 — +1,8 В", kind: "power", desc: "Линейный стабилизатор ядра DDS. Рассеивает заметную мощность — проверьте нагрев (design_notes п. 4.6).", x: 234, y: 196, w: 56, h: 64, block: "pwr" },
  { id: "x2", ref: "X2", title: "Питание +15 В", kind: "power", desc: "Контакт 1 — +15 В, контакт 2 — GND. Предохранитель F1 0,5 А, защита VD1/VD2.", x: 30, y: 356, w: 28, h: 48, block: "pwr" },
  { id: "x1", ref: "X1", title: "ISP / UART", kind: "connector", desc: "Площадки ISP 2×3. X1.1 = TXD0 (PE1), X1.4 = RXD0 (PE0), X1.6 = GND. Сюда подключается USB-UART адаптер для DDS Studio. При ISP-программировании адаптер отсоединить.", x: 244, y: 30, w: 48, h: 36, block: "spi" },
  { id: "x3", ref: "X3", title: "JTAG", kind: "connector", desc: "Разъём 2×5, 1,27 мм для Atmel-ICE: прошивка и отладка D2. Плату не питает.", x: 348, y: 26, w: 96, h: 40 },
  { id: "xw1", ref: "XW1", title: "Вход такта", kind: "connector", desc: "SMA, 480 МГц, проектный уровень 0 дБм на 50 Ом (допустимо −10…+5 дБм, табл. 8.1). Работает при замкнутом S1.", x: 632, y: 392, w: 48, h: 36, block: "clk" },
  { id: "s1", ref: "S1", title: "Выбор такта", kind: "jumper", desc: "Замкнут — такт с XW1 (CSEL = 1). Разомкнут — генератор G1, который в ревизии F не установлен.", x: 688, y: 372, w: 52, h: 40, block: "clk" },
  { id: "g1", ref: "G1", title: "Генератор (не установлен)", kind: "chip", desc: "Место под генератор ГК361; вариант автономного тактирования не подтверждён (DNP).", x: 674, y: 326, w: 44, h: 28, block: "clk" },
  { id: "lpf", ref: "L1–L7", title: "ФНЧ и аттенюатор", kind: "filter", desc: "R44 49,9 Ом → C5 10 нФ → L1 51 нГн, C6 56 пФ, L5 200 нГн, C7 91 пФ, L6 200 нГн, C11 56 пФ, L7 51 нГн → П-аттенюатор R46/R45/R47 (866/5,76/866 Ом) → XW2.", x: 806, y: 318, w: 132, h: 64, block: "lpf" },
  { id: "r11", ref: "R11", title: "RREF ЦАП", kind: "testpoint", desc: "7,15 кОм: ток полной шкалы ЦАП IFS = 72 В / 7,15 кОм ≈ 10 мА.", x: 770, y: 312, w: 30, h: 22, block: "dac1" },
  { id: "xw2", ref: "XW2", title: "Выход", kind: "connector", desc: "SMA 50 Ом. Цель — около −10 дБм в полосе 48–52 МГц; нагружать на 50 Ом.", x: 944, y: 348, w: 36, h: 48, block: "out" },
  { id: "s2", ref: "S2", title: "DIP N[7:0]", kind: "switch", desc: "Младшие 8 бит кода N. ON = 1. Частота f = 48 МГц + 1 кГц · N.", x: 250, y: 368, w: 180, h: 50 },
  { id: "s3", ref: "S3", title: "DIP N[11:8]", kind: "switch", desc: "Старшие 4 бита кода N (N ≤ 4000, иначе код отклоняется).", x: 486, y: 368, w: 100, h: 50 },
  { id: "s4", ref: "S4", title: "Кнопка «Применить»", kind: "switch", desc: "Перечитать DIP и задать новую частоту. В режиме REMOTE не действует.", x: 224, y: 318, w: 48, h: 46 },
  { id: "tp", ref: "TP1/TP2", title: "Контрольные точки", kind: "testpoint", desc: "TP1 — восстановление BQ1, TP2 — GND.", x: 388, y: 248, w: 40, h: 20 },
];

/** First power-up with DDS Studio (Board view checklist). */
export const CHECKLIST: string[] = [
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

/** USB-UART adapter wiring (Connection view, docs/PROTOCOL_ru.md). */
export const WIRING: { mcu: string; board: string; signal: string; dir: "→" | "←" | "—"; note?: string }[] = [
  { mcu: "RX", board: "X1.1", signal: "TXD0 (PE1)", dir: "←", note: "ответы и журнал платы" },
  { mcu: "TX", board: "X1.4", signal: "RXD0 (PE0)", dir: "→", note: "команды; логика 5 В" },
  { mcu: "GND", board: "X1.6", signal: "GND", dir: "—", note: "подключить первым" },
  { mcu: "VCC", board: "—", signal: "не подключать", dir: "—", note: "плата питается от X2" },
];

/** Hardware safety notes. Keep them in the UI and in docs. */
export const SAFETY: string[] = [
  "Контроллер D2 работает от +5 В: нужен адаптер с логикой 5 В. Адаптер 3,3 В допустим, только если его RX выдерживает 5 В; уровень 3,3 В на X1.4 — на границе VIH = 0,6·VCC = 3 В.",
  "Питание адаптера (VCC) к плате не подключать — плата питается только от +15 В через X2.",
  "X1 — это и площадки ISP: на время ISP-программирования адаптер отсоединить. Программирование через JTAG (X3) адаптеру не мешает.",
  "Соединения делайте при выключенном питании, общий провод (GND) — первым.",
  "Такт на XW1: от −10 до +5 дБм (ТО, табл. 8.1). Без такта SPI не работает, DEVID не читается.",
  "XW2 нагружайте на 50 Ом и не подавайте на него внешний сигнал; выходы ЦАП допускают не более ±0,6 В.",
];
