// Blocks of the 1508ПЛ8Т functional diagram (ТО, рис. 3.1) plus the board's
// output chain. Ids match `FieldDef.block` (regs.rs) and `Metric.block` /
// `Issue.block` (model.rs).

export interface BlockDef {
  id: string;
  title: string;
  /** CSS colour token: sig | clk | ref | dig | pwr | ctl */
  tone: "sig" | "clk" | "ref" | "dig" | "pwr" | "ctl";
  desc: string;
  /** Position on the overview SVG (drawing area x 80…1160, y 170…590). */
  x: number;
  y: number;
  w: number;
  h: number;
  /** Block lives on the board, not in the chip. */
  board?: boolean;
}

export const BLOCKS: BlockDef[] = [
  { id: "clk", title: "Такт", tone: "clk", desc: "Вход такта CLKDP/CLKDM (XW1 при замкнутом S1) или CLKP/CLKM (G1), делитель CTR.cmx. Такт нужен и для SPI: без него регистры не пишутся.", x: 100, y: 230, w: 140, h: 90 },
  { id: "sync", title: "SYNC / LINK", tone: "ctl", desc: "Синхронизация нескольких кристаллов, выбор профиля выводами SEL, LINK-порт. На плате не используются: SEL подтянуты к земле.", x: 100, y: 355, w: 140, h: 80 },
  { id: "spi", title: "SPI", tone: "dig", desc: "Последовательный порт: кадры 24 бита (команда 8 + данные 16). SETA, WR, WRI, SETAFT, NOP. На плате ведущий — контроллер D2 через буферы D3/D4.", x: 100, y: 470, w: 140, h: 90 },
  { id: "prof1", title: "Профили К1", tone: "dig", desc: "64 профиля канала 1: приращение фазы dPh (48 бит), фаза P, амплитуда Mul, смещение Offset. Активный выбирается SEL_REG.Pr_1 — так делается бесшовная смена частоты и манипуляции.", x: 280, y: 230, w: 160, h: 90 },
  { id: "lfm1", title: "ЛЧМ К1", tone: "dig", desc: "Генератор линейной частотной модуляции канала 1: 4 стадии (нарастание, пауза, спад, пауза), скорость dF и длительности TPH.", x: 280, y: 355, w: 160, h: 80 },
  { id: "acc1", title: "Аккумулятор К1", tone: "sig", desc: "48-битный аккумулятор фазы: каждый такт прибавляет dPh. Старшие 17 бит фазы (+ P, + дизеринг) идут в таблицу синуса. Fout = dPh·Fclk / 2⁴⁸.", x: 480, y: 230, w: 170, h: 90 },
  { id: "ch2", title: "Канал 2", tone: "sig", desc: "Второй такой же канал (профили, ЛЧМ, аккумулятор). На плате выход ЦАП 2 не выведен, но канал 2 можно подать на ЦАП 1 через ROUTE.sum.", x: 480, y: 470, w: 170, h: 90 },
  { id: "route", title: "ROUTE", tone: "dig", desc: "Маршрутизация каналов к ЦАП: канал 1, канал 2 или их сумма; дизеринг фазы и амплитуды.", x: 690, y: 300, w: 130, h: 120 },
  { id: "dac1", title: "ЦАП 1", tone: "sig", desc: "10-битный токовый ЦАП, выходы OUTP1/OUTM1. Ток полной шкалы IFS = 72 В / RREF (R11 = 7,15 кОм → 10 мА). Ступенчатый выход: спад sin(x)/x и образы k·Fclk ± f.", x: 860, y: 230, w: 130, h: 90 },
  { id: "dac2", title: "ЦАП 2", tone: "sig", desc: "Второй ЦАП. На плате его выход нагружен резисторами и не выведен на разъём.", x: 860, y: 355, w: 130, h: 70 },
  { id: "cmp", title: "Компаратор", tone: "ctl", desc: "Встроенный компаратор для формирования меандра. На плате не используется.", x: 860, y: 470, w: 130, h: 90 },
  { id: "lpf", title: "ФНЧ", tone: "ref", desc: "Плата: R44 49,9 Ом, C5, фильтр 7-го порядка L1–L7/C6–C11 (срез ≈ 70 МГц) и аттенюатор ≈ 1 дБ (R45–R47). Подавляет образы Fclk − f.", x: 1025, y: 230, w: 120, h: 90, board: true },
  { id: "out", title: "XW2", tone: "sig", desc: "Выход платы, SMA 50 Ом. Цель — около −10 дБм в полосе 48–52 МГц.", x: 1025, y: 355, w: 120, h: 70, board: true },
  { id: "pwr", title: "Питание", tone: "pwr", desc: "Плата: +15 В на X2 → D5 (+5 В) → D6 (+3,3 В) и D7 (+1,8 В ядра DDS). DDS_PGOOD сообщает контроллеру о +3,3 В.", x: 1025, y: 470, w: 120, h: 90, board: true },
];

/** Wires between blocks: [from, to, tone]. */
export const WIRES: [string, string, BlockDef["tone"]][] = [
  ["clk", "prof1", "clk"],
  ["prof1", "acc1", "dig"],
  ["lfm1", "acc1", "dig"],
  ["acc1", "route", "sig"],
  ["ch2", "route", "sig"],
  ["route", "dac1", "sig"],
  ["route", "dac2", "sig"],
  ["dac1", "lpf", "sig"],
  ["lpf", "out", "sig"],
  ["sync", "spi", "ctl"],
];

export function blockById(id: string | null | undefined) {
  return BLOCKS.find((b) => b.id === id) ?? null;
}
