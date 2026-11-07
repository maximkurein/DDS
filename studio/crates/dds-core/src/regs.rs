//! Register map of the 1508ПЛ8Т (АО НПЦ «ЭЛВИС», «Техническое описание», ред. 14.10.2020).
//!
//! Source of every address, reset value and bit position: tables 5.1–5.31 of
//! the datasheet. Registers whose reset value is printed as «XXXX» (TC, LFM and
//! profile registers) are undefined after reset; the model uses 0 for them and
//! says so in the register title.
//!
//! What is modelled
//! - common registers DEVID, SEL_REG, CTR, SYNC, LINK, ROUTE, TC_L/H;
//! - for each channel: LFM (ЛЧМ) registers, TSW and the modulation profiles.
//!   The chip has 64 profiles per channel; the app shows profiles 0…15 of
//!   channel 1 and 0…3 of channel 2 (the rest exist in the chip, see `sim.rs`).
//! - write-only command registers (SWRST, CLR, CHx_*_all) are not configuration:
//!   they are sent by dedicated RPCs (`api.rs`), never by auto-apply.
//!
//! Conventions
//! - `REGS[i]` is one register; `FieldDef.reg` is an index into `REGS`.
//! - `defaults()` = datasheet power-on values; `starter()` = what the board's
//!   firmware sets up (firmware/src/drivers/pl8t.c, `pl8t_init`) at 50 МГц.
//! - 48-bit values (dPh, LS_F, LS_dF) and 46-bit LS_TPH are split into _L/_M/_H
//!   registers; use `get48`/`set48`.

use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// read/write configuration
    Rw,
    /// read only (status, ID)
    Ro,
    /// status flag that stays set until cleared (not used by this chip)
    Sticky,
    /// reserved bits that must be written with a fixed pattern (not used by this chip)
    Fixed,
}

#[derive(Serialize, Clone, Copy, Debug)]
pub struct RegDef {
    pub addr: u16,
    pub name: &'static str,
    /// Short Russian description shown next to the register.
    pub title: &'static str,
    /// Datasheet power-on value (0 where the datasheet prints XXXX).
    pub reset: u32,
    /// Group in the register list: "common" | "ch1" | "lfm1" | "prof1" | "ch2" | "lfm2" | "prof2".
    pub group: &'static str,
}

#[derive(Serialize, Clone, Copy, Debug)]
pub struct FieldDef {
    pub name: &'static str,
    /// Index into `REGS`.
    pub reg: usize,
    pub msb: u8,
    pub lsb: u8,
    pub access: Access,
    pub default: u32,
    pub title: &'static str,
    pub desc: &'static str,
    pub options: &'static [(u32, &'static str)],
    /// Block id on the overview diagram (`src/lib/blocks.ts`).
    pub block: &'static str,
    pub unit: &'static str,
    /// Value is two's complement (shown signed in the UI).
    pub signed: bool,
}

impl FieldDef {
    pub const fn new(name: &'static str, reg: usize, msb: u8, lsb: u8) -> Self {
        FieldDef { name, reg, msb, lsb, access: Access::Rw, default: 0, title: "", desc: "", options: &[], block: "", unit: "", signed: false }
    }
    pub const fn access(self, access: Access) -> Self {
        FieldDef { access, ..self }
    }
    pub const fn def(self, default: u32) -> Self {
        FieldDef { default, ..self }
    }
    pub const fn title(self, title: &'static str) -> Self {
        FieldDef { title, ..self }
    }
    pub const fn desc(self, desc: &'static str) -> Self {
        FieldDef { desc, ..self }
    }
    pub const fn opts(self, options: &'static [(u32, &'static str)]) -> Self {
        FieldDef { options, ..self }
    }
    pub const fn block(self, block: &'static str) -> Self {
        FieldDef { block, ..self }
    }
    pub const fn unit(self, unit: &'static str) -> Self {
        FieldDef { unit, ..self }
    }
    pub const fn signed(self) -> Self {
        FieldDef { signed: true, ..self }
    }

    pub fn width(&self) -> u32 {
        (self.msb - self.lsb + 1) as u32
    }
    pub fn mask(&self) -> u32 {
        let ones = if self.width() >= 32 { u32::MAX } else { (1u32 << self.width()) - 1 };
        ones << self.lsb
    }
    pub fn max(&self) -> u32 {
        self.mask() >> self.lsb
    }
    pub fn get(&self, w: &Words) -> u32 {
        (w[self.reg] & self.mask()) >> self.lsb
    }
    pub fn put(&self, w: &mut Words, v: u32) {
        w[self.reg] = (w[self.reg] & !self.mask()) | ((v << self.lsb) & self.mask());
    }
}

// ───────────────────────────── the map ─────────────────────────────

/// Register data width in bits (table 5.1: all registers are 16-bit).
pub const DATA_BITS: u32 = 16;

/// Profiles shown in the app (the chip has 64 per channel).
pub const N_PROF1: usize = 16;
pub const N_PROF2: usize = 4;
/// Registers per channel outside the profiles: LS_CTR, LS_CRFMIN, TSW, 4×TPH(3),
/// F1(3), F2(3), Ph1, Ph2, dF1(3), dF2(3).
const CH_REGS: usize = 29;
/// Words per profile: dPh_L, dPh_M, dPh_H, P, Mul, Offset.
const PROF_REGS: usize = 6;
pub const NUM_REGS: usize = 8 + 2 * CH_REGS + PROF_REGS * (N_PROF1 + N_PROF2);
pub type Words = [u32; NUM_REGS];

pub const R_DEVID: usize = 0;
pub const R_SEL: usize = 1;
pub const R_CTR: usize = 2;
pub const R_SYNC: usize = 3;
pub const R_LINK: usize = 4;
pub const R_ROUTE: usize = 5;
pub const R_TC_L: usize = 6;
pub const R_TC_H: usize = 7;

/// Fixed addresses used by commands (table 5.1).
pub const A_SWRST: u16 = 0x0000;
pub const A_CLR: u16 = 0x0005;
pub const SWRST_KEY: u16 = 0x0078;
pub const DEVID_VALUE: u16 = 0x0201;

/// Base address of channel `ch` (1 or 2).
pub fn ch_base(ch: u8) -> u16 {
    if ch == 2 {
        0x2000
    } else {
        0x1000
    }
}

/// Address of profile `y` of channel `ch`, word `off` (0 dPh_L, 1 dPh_M, 2 dPh_H, 4 P, 5 Mul, 6 Offset).
pub fn prof_addr(ch: u8, y: usize, off: u16) -> u16 {
    ch_base(ch) + 0x400 + 0x10 * y as u16 + off
}

use Access::*;

const CMX_OPTS: &[(u32, &str)] = &[(0, "1:1"), (1, "1:2"), (2, "2 — не описан (сброс)"), (3, "1:4")];
const SUM_OPTS: &[(u32, &str)] = &[(0, "каждый канал на свой ЦАП"), (1, "канал 1 на оба ЦАП"), (2, "канал 2 на оба ЦАП"), (3, "сумма каналов на оба ЦАП")];
const ADITH_OPTS: &[(u32, &str)] = &[(0, "выкл"), (1, "½ LSB"), (2, "зарезервировано"), (3, "8 LSB")];
const LINK_CLK_OPTS: &[(u32, &str)] = &[(0, "Tсимв = 4·Tclk·TC"), (1, "Tсимв = 4·Tclk·2³²/TC"), (2, "зарезервировано"), (3, "внешний: фронт SEL2[0]")];
const STAGE_OPTS: &[(u32, &str)] = &[(0, "стадия 1"), (1, "стадия 2"), (2, "стадия 3"), (3, "стадия 4")];
const TSW_OPTS: &[(u32, &str)] = &[(0, "выкл"), (1, "22 такта"), (2, "24 такта"), (3, "28 тактов"), (4, "36 тактов"), (5, "52 такта"), (6, "84 такта"), (7, "148 тактов")];

pub struct Map {
    pub regs: Vec<RegDef>,
    pub fields: Vec<FieldDef>,
    index: HashMap<&'static str, usize>,
    by_addr: HashMap<u16, usize>,
    by_name: HashMap<&'static str, usize>,
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn build() -> Map {
    let mut regs: Vec<RegDef> = vec![];
    let mut fields: Vec<FieldDef> = vec![];
    let reg = |regs: &mut Vec<RegDef>, addr: u16, name: &'static str, title: &'static str, reset: u32, group: &'static str| -> usize {
        regs.push(RegDef { addr, name, title, reset, group });
        regs.len() - 1
    };

    // ── common registers (table 5.1) ──
    let r = reg(&mut regs, 0x0001, "DEVID", "Идентификатор устройства, только чтение (0x0201)", 0x0201, "common");
    fields.push(FieldDef::new("DEVID", r, 15, 0).access(Ro).def(0x0201).title("Идентификатор").desc("Постоянное значение 0x0201 (табл. 5.1). Его же выдаёт команда NOP — так прошивка проверяет связь по SPI.").block("spi"));

    let r = reg(&mut regs, 0x0002, "SEL_REG", "Активный профиль каналов, стадия ЛЧМ (табл. 5.2)", 0x0000, "common");
    fields.extend([
        FieldDef::new("LS2_stage", r, 15, 14).access(Ro).title("Стадия ЛЧМ, канал 2").desc("Чтение: текущая стадия ЛЧМ канала 2. Запись в режиме ЛЧМ запускает указанную стадию — поэтому в модели поле только читается (табл. 5.2).").opts(STAGE_OPTS).block("ch2"),
        FieldDef::new("Pr_2", r, 13, 8).title("Профиль канала 2").desc("Номер активного профиля 0…63 канала 2 в режиме без ЛЧМ (табл. 5.2).").block("ch2"),
        FieldDef::new("LS1_stage", r, 7, 6).access(Ro).title("Стадия ЛЧМ, канал 1").desc("Чтение: текущая стадия ЛЧМ канала 1. Запись SEL_REG в режиме ЛЧМ немедленно переводит канал в стадию из этого поля (разд. 3.3).").opts(STAGE_OPTS).block("lfm1"),
        FieldDef::new("Pr_1", r, 5, 0).title("Профиль канала 1").desc("Номер активного профиля 0…63 канала 1. Смена профиля — это смена частоты, фазы и амплитуды одной записью: так работают манипуляция и бесшовная перестройка (разд. 3.2).").block("prof1"),
    ]);

    let r = reg(&mut regs, 0x0003, "CTR", "Управление: ЦАП, компаратор, делитель такта (табл. 5.3)", 0x0002, "common");
    fields.extend([
        FieldDef::new("CMP_on", r, 14, 14).title("Компаратор").desc("1 — включить компаратор (синус → меандр). На плате выходы компаратора не используются.").opts(&[(0, "выкл"), (1, "вкл")]).block("cmp"),
        FieldDef::new("DAC2_on", r, 13, 13).title("ЦАП 2").desc("1 — включить ЦАП 2. На плате его выходы OUTP2/OUTM2 нагружены резисторами 49,9 Ом и на разъём не выведены.").opts(&[(0, "выкл"), (1, "вкл")]).block("dac2"),
        FieldDef::new("DAC1_on", r, 12, 12).title("ЦАП 1").desc("1 — включить ЦАП 1: его выход OUTP1 идёт через ФНЧ на XW2. Прошивка включает ЦАП только после записи первой частоты.").opts(&[(0, "выкл"), (1, "вкл")]).block("dac1"),
        FieldDef::new("cmx", r, 4, 0).def(2).title("Делитель такта").desc("Коэффициент деления при CSEL = 0 (вход CLKP/CLKM, генератор G1): 0 → 1:1, 1 → 1:2, 3 → 1:4. При CSEL = 1 (вход XW1, CLKDP/CLKDM) деление всегда 1 (табл. 5.3). Значение 2 после сброса в ТО не описано.").opts(CMX_OPTS).block("clk"),
    ]);

    let r = reg(&mut regs, 0x0004, "SYNC", "Синхронизация, SEL, SPI-мастер (табл. 5.4)", 0x4800, "common");
    fields.extend([
        FieldDef::new("CSYNC_OE", r, 14, 14).def(1).title("Выход CSYNC").desc("Выдать на CSYNC частоту дискретизации ÷4 (÷8 с CSYNC_DIV). По сбросу включено; прошивка выключает, на плате CSYNC не используется.").opts(&[(0, "выкл"), (1, "вкл")]).block("sync"),
        FieldDef::new("CSYNC_IE", r, 13, 13).title("Вход CSYNC").desc("Синхронизация от внешнего сигнала CSYNC (несколько микросхем).").opts(&[(0, "выкл"), (1, "вкл")]).block("sync"),
        FieldDef::new("CSYNC_DIV", r, 11, 11).def(1).title("CSYNC ÷2").desc("Дополнительное деление CSYNC на 2 (итого Fclk/8).").opts(&[(0, "нет"), (1, "÷2")]).block("sync"),
        FieldDef::new("SPI_master", r, 10, 10).title("Выход SSCSn").desc("1 — разрешить выход SSCSn (на плате вывод 7 не подключён).").opts(&[(0, "выкл"), (1, "вкл")]).block("sync"),
        FieldDef::new("SYNC_del", r, 9, 9).title("Задержка CSYNC 0,5 нс").desc("Дополнительная задержка входа CSYNC, если не выполняется tsucsc.").opts(&[(0, "нет"), (1, "+0,5 нс")]).block("sync"),
        FieldDef::new("SEL_IE", r, 8, 8).title("Профиль от выводов SEL").desc("0 — профиль выбирает SEL_REG; 1 — выводы SEL (аппаратная манипуляция). На плате SEL подтянуты к земле: при 1 канал уйдёт в профиль 0.").opts(&[(0, "SEL_REG"), (1, "выводы SEL")]).block("sync"),
        FieldDef::new("SEL_OE", r, 7, 7).title("SEL как выходы").desc("1 — выводы SEL показывают номер активного профиля (стадию ЛЧМ).").opts(&[(0, "входы"), (1, "выходы")]).block("sync"),
        FieldDef::new("SYNC_Phase", r, 6, 5).title("Фаза синхронизации").desc("Задержка такта ядра относительно CSYNC, тактов ЦАП.").block("sync"),
        FieldDef::new("SYNC_Out_Phase", r, 4, 3).title("Задержка выхода CSYNC").desc("Задержка выходного CSYNC, тактов ЦАП.").block("sync"),
    ]);

    let r = reg(&mut regs, 0x0006, "LINK", "LINK-порт (табл. 5.6)", 0x0000, "common");
    fields.extend([
        FieldDef::new("clk_mode", r, 4, 3).title("Тактирование LINK").desc("Скорость следования символов LINK-порта (табл. 5.6).").opts(LINK_CLK_OPTS).block("sync"),
        FieldDef::new("LINK_on", r, 0, 0).title("LINK-порт").desc("1 — включить LINK-порт: линии SEL1 заняты под LINK, а регистры с адресами ≥ 0x1000 недоступны для записи. На плате LINK не используется.").opts(&[(0, "выкл"), (1, "вкл")]).block("sync"),
    ]);

    let r = reg(&mut regs, 0x0007, "ROUTE", "Маршрутизация к ЦАП и рандомизация (табл. 5.7)", 0x0000, "common");
    fields.extend([
        FieldDef::new("pdith2", r, 7, 7).title("Рандомизация фазы К2").desc("Выбеляющий шум фазы канала 2.").opts(&[(0, "выкл"), (1, "вкл")]).block("ch2"),
        FieldDef::new("pdith1", r, 6, 6).title("Рандомизация фазы К1").desc("Добавляет 4-битный шум к фазе перед преобразователем фаза-амплитуда: дискретные спуры усечения фазы превращаются в шумовой пьедестал (разд. 3).").opts(&[(0, "выкл"), (1, "вкл")]).block("acc1"),
        FieldDef::new("adith1", r, 5, 4).title("Шум амплитуды К1").desc("Шум перед ограничением 12 → 10 бит в канале 1 (табл. 5.7).").opts(ADITH_OPTS).block("route"),
        FieldDef::new("adith2", r, 3, 2).title("Шум амплитуды К2").desc("То же для канала 2. При adith1 = adith2 = 3 шумы противофазны и в сумме дают ноль.").opts(ADITH_OPTS).block("route"),
        FieldDef::new("sum", r, 1, 0).title("Маршрут к ЦАП").desc("Какой канал попадает на какой ЦАП. На плате выведен только ЦАП 1: чтобы услышать канал 2, выберите «канал 2 на оба ЦАП» или «сумма».").opts(SUM_OPTS).block("route"),
    ]);

    let r = reg(&mut regs, 0x0008, "TC_L", "Делитель чиповой скорости LINK [15:0], после сброса не определён", 0, "common");
    fields.push(FieldDef::new("TC_L", r, 15, 0).title("TC [15:0]").desc("Младшие разряды делителя скорости символов LINK-порта.").block("sync"));
    let r = reg(&mut regs, 0x0009, "TC_H", "Делитель чиповой скорости LINK [31:16], после сброса не определён", 0, "common");
    fields.push(FieldDef::new("TC_H", r, 15, 0).title("TC [31:16]").desc("Старшие разряды делителя скорости символов LINK-порта.").block("sync"));

    // ── channels ──
    for ch in [1u8, 2] {
        let b = ch_base(ch);
        let (g_ch, g_lfm, g_prof) = if ch == 1 { ("ch1", "lfm1", "prof1") } else { ("ch2", "lfm2", "prof2") };
        let (blk_lfm, blk_prof) = if ch == 1 { ("lfm1", "prof1") } else { ("ch2", "ch2") };
        let p = |s: &str| leak(format!("CH{}_{}", ch, s));

        let r = reg(&mut regs, b, p("LS_CTR"), if ch == 1 { "Управление ЛЧМ канала 1 (табл. 5.21)" } else { "Управление ЛЧМ канала 2 (табл. 5.21)" }, 0, g_lfm);
        fields.extend([
            FieldDef::new(p("LS_on"), r, 15, 15).title("Режим ЛЧМ").desc("1 — канал формирует ЛЧМ (4 стадии), профили используются для коррекции.").opts(&[(0, "выкл"), (1, "вкл")]).block(blk_lfm),
            FieldDef::new(p("PA_bypass"), r, 14, 14).title("Без фаза→амплитуда").desc("1 — отключить преобразование фаза-амплитуда: на ЦАП идёт пила фазы.").opts(&[(0, "синус"), (1, "пила")]).block(blk_lfm),
            FieldDef::new(p("frq_reset_3"), r, 13, 13).title("F2 в начале стадии 3").desc("Установить частоту CHx_LS_F2 в начале стадии 3.").opts(&[(0, "нет"), (1, "да")]).block(blk_lfm),
            FieldDef::new(p("frq_reset_1"), r, 12, 12).title("F1 в начале стадии 1").desc("Установить частоту CHx_LS_F1 в начале стадии 1.").opts(&[(0, "нет"), (1, "да")]).block(blk_lfm),
            FieldDef::new(p("ph_reset_3"), r, 11, 11).title("Фаза Ph2 в стадии 3").desc("Сброс фазы в CHx_LS_Ph2 в начале стадии 3.").opts(&[(0, "нет"), (1, "да")]).block(blk_lfm),
            FieldDef::new(p("ph_reset_1"), r, 10, 10).title("Фаза Ph1 в стадии 1").desc("Сброс фазы в CHx_LS_Ph1 в начале стадии 1.").opts(&[(0, "нет"), (1, "да")]).block(blk_lfm),
            FieldDef::new(p("s2_on"), r, 9, 9).title("Сигнал в стадии 2").desc("0 — выключить выход во 2-й стадии (пауза).").opts(&[(0, "выкл"), (1, "вкл")]).block(blk_lfm),
            FieldDef::new(p("s4_on"), r, 8, 8).title("Сигнал в стадии 4").desc("0 — выключить выход в 4-й стадии (пауза).").opts(&[(0, "выкл"), (1, "вкл")]).block(blk_lfm),
            FieldDef::new(p("corr_enable"), r, 7, 7).title("Коррекция АЧХ").desc("Кусочно-линейная коррекция фазы, амплитуды и смещения по 64 профилям (разд. 3.3.1).").opts(&[(0, "выкл"), (1, "вкл")]).block(blk_lfm),
            FieldDef::new(p("s2_f0"), r, 6, 6).title("Стадия 2: f = 0").desc("1 — нулевое приращение фазы во 2-й стадии (постоянная фаза).").opts(&[(0, "держать f"), (1, "f = 0")]).block(blk_lfm),
            FieldDef::new(p("s4_f0"), r, 5, 5).title("Стадия 4: f = 0").desc("1 — нулевое приращение фазы в 4-й стадии.").opts(&[(0, "держать f"), (1, "f = 0")]).block(blk_lfm),
            FieldDef::new(p("auto"), r, 4, 4).title("Автоповтор").desc("После стадии 4 снова начинается стадия 1.").opts(&[(0, "однократно"), (1, "повтор")]).block(blk_lfm),
            FieldDef::new(p("corr_fscale"), r, 3, 0).title("Масштаб коррекции").desc("Диапазон коррекции ΔF = 63/64 · Fclk / 2^corr_fscale (разд. 3.3.1).").block(blk_lfm),
        ]);
        let r = reg(&mut regs, b + 1, p("LS_CRFMIN"), "Нижняя граница коррекции ЛЧМ (старшие 16 бит частоты)", 0, g_lfm);
        fields.push(FieldDef::new(p("LS_CRFMIN"), r, 15, 0).title("F0 коррекции").desc("F0 = value · Fclk / 2^16 (разд. 3.3.1).").block(blk_lfm));
        let r = reg(&mut regs, b + 2, p("TSW"), "Время переключения параметров (гауссов фильтр, табл. 5.8)", 0, g_ch);
        fields.push(FieldDef::new(p("TSW"), r, 2, 0).title("Плавное переключение").desc("tsw > 0: параметры профиля меняются плавно за 2^tsw + 4 + 16 тактов ЦАП; переключать профили не чаще этого интервала (разд. 3.2). 0 — мгновенно.").opts(TSW_OPTS).block(blk_prof));

        for k in 1..=4u16 {
            for (i, part) in ["L", "M", "H"].iter().enumerate() {
                let r = reg(&mut regs, b + 0x10 + 4 * (k - 1) + i as u16, p(&format!("LS_TPH{}_{}", k, part)), leak(format!("Длительность стадии {} ЛЧМ, {} (×4 такта)", k, ["[15:0]", "[31:16]", "[45:32]"][i])), 0, g_lfm);
                let msb = if i == 2 { 13 } else { 15 };
                fields.push(FieldDef::new(regs[r].name, r, msb, 0).title(leak(format!("TPH{} {}", k, ["[15:0]", "[31:16]", "[45:32]"][i]))).desc("Длительность стадии в единицах 4·Tclk (табл. 5.28–5.31).").block(blk_lfm));
            }
        }
        for (base, nm, what) in [(0x20u16, "LS_F1", "Начальная частота ЛЧМ 1"), (0x24, "LS_F2", "Начальная частота ЛЧМ 2")] {
            for (i, part) in ["L", "M", "H"].iter().enumerate() {
                let r = reg(&mut regs, b + base + i as u16, p(&format!("{}_{}", nm, part)), leak(format!("{}, {}", what, ["[15:0]", "[31:16]", "[47:32]"][i])), 0, g_lfm);
                fields.push(FieldDef::new(regs[r].name, r, 15, 0).title(leak(format!("{} {}", nm, ["[15:0]", "[31:16]", "[47:32]"][i]))).desc("48-битная частота в единицах Fclk/2^48 (табл. 5.23).").block(blk_lfm));
            }
        }
        for (k, off) in [(1, 0x30u16), (2, 0x31)] {
            let r = reg(&mut regs, b + off, p(&format!("LS_Ph{}", k)), leak(format!("Начальная фаза ЛЧМ {} (табл. 5.24)", k)), 0, g_lfm);
            fields.push(FieldDef::new(regs[r].name, r, 15, 0).title(leak(format!("Фаза Ph{}", k))).desc("Φ = π · value / 2^15, дополнительный код.").block(blk_lfm).signed());
        }
        for (base, nm, what) in [(0x40u16, "LS_dF1", "Приращение частоты стадии 1"), (0x44, "LS_dF2", "Приращение частоты стадии 3")] {
            for (i, part) in ["L", "M", "H"].iter().enumerate() {
                let r = reg(&mut regs, b + base + i as u16, p(&format!("{}_{}", nm, part)), leak(format!("{}, {}", what, ["[15:0]", "[31:16]", "[47:32]"][i])), 0, g_lfm);
                fields.push(FieldDef::new(regs[r].name, r, 15, 0).title(leak(format!("{} {}", nm, ["[15:0]", "[31:16]", "[47:32]"][i]))).desc("Добавляется к частоте каждый такт; 48 бит, дополнительный код (табл. 5.25–5.27).").block(blk_lfm));
            }
        }

        let n = if ch == 1 { N_PROF1 } else { N_PROF2 };
        for y in 0..n {
            for (i, part) in ["L", "M", "H"].iter().enumerate() {
                let r = reg(&mut regs, prof_addr(ch, y, i as u16), p(&format!("dPh{}_{}", y, part)), leak(format!("Профиль {}: приращение фазы {}", y, ["[15:0]", "[31:16]", "[47:32]"][i])), 0, g_prof);
                fields.push(FieldDef::new(regs[r].name, r, 15, 0).title(leak(format!("dPh{} {}", y, ["[15:0]", "[31:16]", "[47:32]"][i]))).desc("Частота Fout = dPh · Fclk / 2^48 (разд. 3.1). Удобнее задавать на вкладке «Частота».").block(blk_prof));
            }
            let r = reg(&mut regs, prof_addr(ch, y, 4), p(&format!("P{}", y)), leak(format!("Профиль {}: смещение фазы (табл. 5.12)", y)), 0, g_prof);
            fields.push(FieldDef::new(regs[r].name, r, 15, 0).title(leak(format!("Фаза, профиль {}", y))).desc("Φ = π · value / 2^15, дополнительный код: 16384 = 90°.").block(blk_prof).signed());
            let r = reg(&mut regs, prof_addr(ch, y, 5), p(&format!("Mul{}", y)), leak(format!("Профиль {}: амплитуда (табл. 5.13)", y)), 0, g_prof);
            fields.push(FieldDef::new(regs[r].name, r, 15, 3).title(leak(format!("Амплитуда, профиль {}", y))).desc("A = mul / 2^12, 13 бит, дополнительный код: 4095 ≈ полная шкала, отрицательное значение инвертирует сигнал (табл. 5.13).").block(blk_prof).signed());
            let r = reg(&mut regs, prof_addr(ch, y, 6), p(&format!("Offset{}", y)), leak(format!("Профиль {}: постоянное смещение (табл. 5.14)", y)), 0, g_prof);
            fields.push(FieldDef::new(regs[r].name, r, 15, 4).title(leak(format!("Смещение, профиль {}", y))).desc("Постоянная составляющая, 12 бит, дополнительный код. Через C5 на выход не проходит.").block(blk_prof).signed());
        }
    }
    assert_eq!(regs.len(), NUM_REGS, "NUM_REGS disagrees with the register list");

    let index = fields.iter().enumerate().map(|(i, f)| (f.name, i)).collect();
    let by_addr = regs.iter().enumerate().map(|(i, r)| (r.addr, i)).collect();
    let by_name = regs.iter().enumerate().map(|(i, r)| (r.name, i)).collect();
    Map { regs, fields, index, by_addr, by_name }
}

pub static MAP: LazyLock<Map> = LazyLock::new(build);

/// All registers (`REGS[i]`).
pub struct Regs;
impl std::ops::Deref for Regs {
    type Target = [RegDef];
    fn deref(&self) -> &[RegDef] {
        &MAP.regs
    }
}
pub static REGS: Regs = Regs;

pub struct Fields;
impl std::ops::Deref for Fields {
    type Target = [FieldDef];
    fn deref(&self) -> &[FieldDef] {
        &MAP.fields
    }
}
pub static FIELDS: Fields = Fields;

/// No double buffering in this chip: every write takes effect at once.
pub const COMMIT_REG: Option<usize> = None;

/// Order of configuration writes (follows `pl8t_init`): synchronisation and
/// routing first, then channel registers and profiles, LINK after the channel
/// registers (LINK.on blocks writes ≥ 0x1000), SEL_REG, and CTR last so the DAC
/// is enabled only when the profile is ready.
pub fn write_order() -> Vec<usize> {
    let mut v = vec![R_SYNC, R_ROUTE, R_TC_L, R_TC_H];
    v.extend(8..NUM_REGS);
    v.extend([R_LINK, R_SEL, R_CTR]);
    v.retain(|&r| writable(r));
    v
}

// ───────────────────────────── helpers ─────────────────────────────

pub fn field(name: &str) -> Option<&'static FieldDef> {
    MAP.index.get(name).map(|&i| &MAP.fields[i])
}

pub fn fields_of(r: usize) -> impl Iterator<Item = &'static FieldDef> {
    MAP.fields.iter().filter(move |f| f.reg == r)
}

pub fn data_mask() -> u32 {
    (1u32 << DATA_BITS) - 1
}

/// Register holds configuration that the host writes.
pub fn writable(r: usize) -> bool {
    fields_of(r).any(|f| f.access == Rw)
}

pub fn defined_mask(r: usize) -> u32 {
    fields_of(r).fold(0, |m, f| m | f.mask())
}

/// Bits that change on their own (excluded from readback comparison).
pub fn status_mask(r: usize) -> u32 {
    fields_of(r).filter(|f| matches!(f.access, Ro | Sticky)).fold(0, |m, f| m | f.mask())
}

pub fn defaults() -> Words {
    std::array::from_fn(|r| REGS[r].reset)
}

pub fn reg_by_addr(addr: u16) -> Option<usize> {
    MAP.by_addr.get(&addr).copied()
}

pub fn reg_by_name(name: &str) -> Option<usize> {
    MAP.by_name.get(name).copied()
}

/// 48-bit value split over `<base>_L`, `<base>_M`, `<base>_H` (H may be narrower).
pub fn get48(w: &Words, base: &str) -> u64 {
    let part = |s: &str| field(&format!("{}_{}", base, s)).map(|f| f.get(w) as u64).unwrap_or(0);
    part("L") | part("M") << 16 | part("H") << 32
}

pub fn set48(w: &mut Words, base: &str, v: u64) -> bool {
    let mut ok = true;
    for (i, s) in ["L", "M", "H"].iter().enumerate() {
        match field(&format!("{}_{}", base, s)) {
            Some(f) => f.put(w, ((v >> (16 * i)) as u32) & f.max()),
            None => ok = false,
        }
    }
    ok
}

/// Phase increment (FTW) of profile `y` of channel `ch`.
pub fn ftw(w: &Words, ch: u8, y: usize) -> u64 {
    get48(w, &format!("CH{}_dPh{}", ch, y))
}

pub fn set_ftw(w: &mut Words, ch: u8, y: usize, v: u64) -> bool {
    set48(w, &format!("CH{}_dPh{}", ch, y), v & ((1u64 << 48) - 1))
}

/// Number of modelled profiles of a channel.
pub fn n_prof(ch: u8) -> usize {
    if ch == 2 {
        N_PROF2
    } else {
        N_PROF1
    }
}

/// Working initial configuration for the board — the same state the firmware's
/// `pl8t_init` + `pl8t_set_ftw(50 МГц)` leave (firmware/config/dds_config.h):
/// - CTR: DAC1 on, DAC2 and comparator off, cmx = 0 (the reset value 2 is not a documented ratio);
/// - SYNC = 0: CSYNC output off (reset 0x4800 drives the CSYNC pin for nothing);
/// - channel 1 profiles 0/1 at 50 МГц with mul = 4095 (DDS_AMPLITUDE_MUL);
///   profiles 2…9 hold 48,5…52 МГц for the hopping lesson, the rest 50 МГц;
/// - channel 2 profile 0 at 51 МГц, half amplitude — heard only when ROUTE.sum routes it.
pub fn starter() -> Words {
    let mut w = defaults();
    set(&mut w, "cmx", 0);
    set(&mut w, "DAC1_on", 1);
    set(&mut w, "CSYNC_OE", 0);
    set(&mut w, "CSYNC_DIV", 0);
    let ftw_of = |f: f64| crate::model::ftw_for(f, 480e6);
    for y in 0..N_PROF1 {
        let f = if (2..10).contains(&y) { 48.5e6 + 0.5e6 * (y - 2) as f64 } else { 50e6 };
        set_ftw(&mut w, 1, y, ftw_of(f));
        set(&mut w, &format!("CH1_Mul{}", y), 4095);
    }
    for y in 0..N_PROF2 {
        set_ftw(&mut w, 2, y, ftw_of(51e6));
        set(&mut w, &format!("CH2_Mul{}", y), 2048);
    }
    w
}

/// Keep only defined data bits (reserved bits read as 0, table 5.1 note).
pub fn sanitize(r: usize, w: u32) -> u32 {
    w & data_mask() & defined_mask(r)
}

pub fn get(w: &Words, name: &str) -> u32 {
    field(name).map(|f| f.get(w)).unwrap_or(0)
}

/// Field value as the UI sees it (sign-extended for two's complement fields).
pub fn get_signed(w: &Words, name: &str) -> i64 {
    match field(name) {
        Some(f) => to_signed(f, f.get(w)),
        None => 0,
    }
}

pub fn to_signed(f: &FieldDef, v: u32) -> i64 {
    let v = v as i64;
    let bits = f.width();
    if f.signed && (v >> (bits - 1)) & 1 == 1 {
        v - (1i64 << bits)
    } else {
        v
    }
}

/// Store a signed value into a two's-complement field (clamped to its range).
pub fn set_signed(w: &mut Words, name: &str, v: i64) -> bool {
    match field(name) {
        Some(f) if f.access == Rw => {
            let bits = f.width();
            let (lo, hi) = if f.signed { (-(1i64 << (bits - 1)), (1i64 << (bits - 1)) - 1) } else { (0, f.max() as i64) };
            let v = v.clamp(lo, hi);
            f.put(w, (v as u32) & f.max());
            true
        }
        _ => false,
    }
}

/// Returns false for unknown fields, read-only fields and out-of-range values.
pub fn set(w: &mut Words, name: &str, v: u32) -> bool {
    match field(name) {
        Some(f) if f.access == Rw && v <= f.max() => {
            f.put(w, v);
            true
        }
        _ => false,
    }
}

pub fn decode(w: &Words) -> BTreeMap<&'static str, u32> {
    FIELDS.iter().map(|f| (f.name, f.get(w))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_fit_and_do_not_overlap() {
        for r in 0..NUM_REGS {
            let mut seen = 0u32;
            for f in fields_of(r) {
                assert!(f.msb >= f.lsb, "{}: msb < lsb", f.name);
                assert!((f.msb as u32) < DATA_BITS, "{}: outside the register", f.name);
                assert_eq!(seen & f.mask(), 0, "{} overlaps another field", f.name);
                seen |= f.mask();
                assert!(f.default <= f.max(), "{}: default does not fit", f.name);
                for (v, _) in f.options {
                    assert!(*v <= f.max(), "{}: option {} does not fit", f.name, v);
                }
            }
        }
    }

    #[test]
    fn unique_names_and_addresses() {
        assert_eq!(MAP.index.len(), FIELDS.len(), "duplicate field name");
        assert_eq!(MAP.by_addr.len(), REGS.len(), "duplicate address");
        assert_eq!(MAP.by_name.len(), REGS.len(), "duplicate register name");
    }

    #[test]
    fn reset_values_match_field_defaults() {
        let d = defaults();
        for f in FIELDS.iter() {
            assert_eq!(f.get(&d), f.default, "{}: reset value of {} disagrees with field default", f.name, REGS[f.reg].name);
        }
    }

    #[test]
    fn datasheet_addresses() {
        // table 5.1 and the profile layout (profile y at 0x1400 + 0x10·y)
        let a = |n: &str| REGS[reg_by_name(n).unwrap()].addr;
        assert_eq!(a("CTR"), 0x0003);
        assert_eq!(a("SYNC"), 0x0004);
        assert_eq!(a("ROUTE"), 0x0007);
        assert_eq!(a("CH1_LS_CTR"), 0x1000);
        assert_eq!(a("CH1_TSW"), 0x1002);
        assert_eq!(a("CH1_LS_TPH2_L"), 0x1014);
        assert_eq!(a("CH1_LS_F2_H"), 0x1026);
        assert_eq!(a("CH1_LS_Ph2"), 0x1031);
        assert_eq!(a("CH1_LS_dF2_L"), 0x1044);
        assert_eq!(a("CH1_dPh0_L"), 0x1400);
        assert_eq!(a("CH1_Offset0"), 0x1406);
        assert_eq!(a("CH1_dPh1_L"), 0x1410);
        assert_eq!(a("CH2_Mul3"), 0x2435);
        // firmware/src/drivers/pl8t_regs.h agrees
        assert_eq!(prof_addr(1, 1, 0), 0x1410);
    }

    #[test]
    fn round_trip_every_rw_field() {
        for f in FIELDS.iter().filter(|f| f.access == Rw) {
            let mut w = starter();
            let before = w;
            assert!(set(&mut w, f.name, f.max()));
            assert_eq!(get(&w, f.name), f.max());
            for g in FIELDS.iter().filter(|g| g.name != f.name) {
                assert_eq!(g.get(&w), g.get(&before), "setting {} changed {}", f.name, g.name);
            }
        }
    }

    #[test]
    fn mul_and_ftw_packing() {
        let mut w = defaults();
        // CHx_Muly[15:3] = mul — the same packing as PL8T_MUL() in the firmware
        assert!(set(&mut w, "CH1_Mul0", 4095));
        assert_eq!(w[reg_by_name("CH1_Mul0").unwrap()], 4095 << 3);
        assert!(set_signed(&mut w, "CH1_Mul0", -1));
        assert_eq!(get_signed(&w, "CH1_Mul0"), -1);
        assert!(set_signed(&mut w, "CH1_P0", -16384));
        assert_eq!(get_signed(&w, "CH1_P0"), -16384);
        let v = 0x1234_5678_9ABC;
        assert!(set_ftw(&mut w, 1, 3, v));
        assert_eq!(ftw(&w, 1, 3), v);
        assert_eq!(w[reg_by_name("CH1_dPh3_M").unwrap()], 0x5678);
    }

    #[test]
    fn write_order_ends_with_ctr() {
        let o = write_order();
        assert_eq!(*o.last().unwrap(), R_CTR);
        assert!(!o.contains(&R_DEVID));
        assert_eq!(o.len(), NUM_REGS - 1);
    }
}
