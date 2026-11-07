//! Engineering model of the 1508ПЛ8Т on the DDS 48–52 МГц board (revision F).
//! Everything the UI shows is computed here; the UI never re-implements a formula.
//!
//! Sources
//! - ТО 1508ПЛ8Т, ред. 14.10.2020: разд. 3 (тракт канала, разрядности), 3.1 (Fout),
//!   3.3 (ЛЧМ), 6.4 (ЦАП, IFS = 72 В / RREF), табл. 8.1 (SFDR, фазовый шум), 9.1;
//! - board netlist `output/netlist/netlist.xml` (revision F): R11 = 7,15 кОм,
//!   OUTP1 → R44 49,9 Ом → C5 10 нФ → L1 51 нГн, C6 56 пФ, L5 200 нГн, C7 91 пФ,
//!   L6 200 нГн, C11 56 пФ, L7 51 нГн → π-аттенюатор R46 866 / R45 5,76 / R47 866 Ом → XW2;
//! - firmware `firmware/src/core/freq.c` (FTW rounding) and `config/dds_config.h`.
//!
//! Estimates that are not datasheet facts are marked «оценка» in the UI text.

use crate::regs::{self, Words};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

const TWO48: f64 = 281_474_976_710_656.0;

/// Board-level parameters that are not in the chip's registers.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    /// Clock frequency at XW1 (CSEL = 1) or G1 (CSEL = 0), Hz.
    pub fclk_hz: f64,
    /// S1: 1 — clock from XW1 (CLKDP/CLKDM, S1 closed), 0 — generator G1 (CLKP/CLKM).
    pub csel: u8,
    /// Clock input level, dBm into 50 Ом (table 8.1: −10…+5 dBm).
    pub clk_dbm: f64,
    /// R11 (IREF1), Ом.
    pub rref_ohm: f64,
    /// Unloaded Q of the filter inductors (design notes: Q ≥ 30 at 50 МГц).
    pub inductor_q: f64,
    /// Working band of the board and the DIP code scale f = base + step·N.
    pub band_lo_hz: f64,
    pub band_hi_hz: f64,
    pub step_hz: f64,
    /// Output level the board should deliver at XW2, dBm.
    pub target_dbm: f64,
    /// Simulator stimulus: clock present at the selected input.
    pub sim_clock_ok: bool,
    /// Simulator stimulus: +3,3 В present (DDS_PGOOD).
    pub sim_pgood: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            fclk_hz: 480e6,
            csel: 1,
            clk_dbm: 0.0,
            rref_ohm: 7150.0,
            inductor_q: 30.0,
            band_lo_hz: 48e6,
            band_hi_hz: 52e6,
            step_hz: 1000.0,
            target_dbm: -10.0,
            sim_clock_ok: true,
            sim_pgood: true,
        }
    }
}

// ───────────────────────────── frequency arithmetic ─────────────────────────────

/// FTW = round(f · 2^48 / fclk), half up — the same as `freq_hz_to_ftw()` in the
/// firmware, generalised to millihertz. Exact integer arithmetic (u128).
pub fn ftw_for(f_hz: f64, fclk_hz: f64) -> u64 {
    if !(f_hz > 0.0) || !(fclk_hz > 0.0) {
        return 0;
    }
    let f = (f_hz * 1000.0).round() as u128;
    let c = (fclk_hz * 1000.0).round() as u128;
    let q = ((f << 49) + c) / (2 * c);
    (q as u64) & ((1u64 << 48) - 1)
}

/// Fout = FTW · fclk / 2^48 (section 3.1).
pub fn f_of_ftw(ftw: u64, fclk_hz: f64) -> f64 {
    ftw as f64 * fclk_hz / TWO48
}

/// Clock that reaches the DAC: CSEL = 1 — no division; CSEL = 0 — CTR.cmx (table 5.3).
pub fn fclk_eff(w: &Words, s: &Settings) -> f64 {
    if s.csel == 1 {
        return s.fclk_hz;
    }
    match regs::get(w, "cmx") {
        0 => s.fclk_hz,
        1 => s.fclk_hz / 2.0,
        3 => s.fclk_hz / 4.0,
        _ => f64::NAN,
    }
}

/// Phase in degrees of a profile phase word (Φ = π·value/2^15, table 5.12).
pub fn phase_deg(v: i64) -> f64 {
    v as f64 * 180.0 / 32768.0
}

pub fn phase_word(deg: f64) -> i64 {
    let mut d = deg % 360.0;
    if d >= 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    ((d / 180.0 * 32768.0).round() as i64).clamp(-32768, 32767)
}

// ───────────────────────────── analog output chain ─────────────────────────────

#[derive(Clone, Copy, Debug, Default)]
pub struct Cx {
    pub re: f64,
    pub im: f64,
}

impl Cx {
    pub const fn new(re: f64, im: f64) -> Self {
        Cx { re, im }
    }
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    fn add(self, o: Cx) -> Cx {
        Cx::new(self.re + o.re, self.im + o.im)
    }
    fn mul(self, o: Cx) -> Cx {
        Cx::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
    fn div(self, o: Cx) -> Cx {
        let d = o.re * o.re + o.im * o.im;
        Cx::new((self.re * o.re + self.im * o.im) / d, (self.im * o.re - self.re * o.im) / d)
    }
    fn inv(self) -> Cx {
        Cx::new(1.0, 0.0).div(self)
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum El {
    SerC(f64),
    SerL(f64),
    ShC(f64),
    SerR(f64),
    ShR(f64),
}

/// Source load on OUTP1 (R44) and the load at XW2.
pub const R_SRC: f64 = 49.9;
pub const R_LOAD: f64 = 50.0;
/// OUTM1 load (R25): the complementary current flows here, DC-coupled.
pub const R_OUTM: f64 = 49.9;

/// OUTP1 → XW2, revision F netlist.
pub const OUT_CHAIN: &[(&str, El)] = &[
    ("C5", El::SerC(10e-9)),
    ("L1", El::SerL(51e-9)),
    ("C6", El::ShC(56e-12)),
    ("L5", El::SerL(200e-9)),
    ("C7", El::ShC(91e-12)),
    ("L6", El::SerL(200e-9)),
    ("C11", El::ShC(56e-12)),
    ("L7", El::SerL(51e-9)),
    ("R46", El::ShR(866.0)),
    ("R45", El::SerR(5.76)),
    ("R47", El::ShR(866.0)),
];

/// Transimpedance OUTP1 current → XW2 voltage, Ом (ABCD chain, R44 as source shunt).
pub fn transimpedance(f: f64, q: f64) -> Cx {
    let w = 2.0 * PI * f.max(1.0);
    // ABCD = [[a, b], [c, d]]
    let (mut a, mut b, mut c, mut d) = (Cx::new(1.0, 0.0), Cx::new(0.0, 0.0), Cx::new(0.0, 0.0), Cx::new(1.0, 0.0));
    let series = |z: Cx, a: &mut Cx, b: &mut Cx, c: &mut Cx, d: &mut Cx| {
        // [a b; c d] · [1 z; 0 1]
        *b = a.mul(z).add(*b);
        *d = c.mul(z).add(*d);
    };
    let shunt = |y: Cx, a: &mut Cx, b: &mut Cx, c: &mut Cx, d: &mut Cx| {
        // [a b; c d] · [1 0; y 1]
        *a = a.add(b.mul(y));
        *c = c.add(d.mul(y));
    };
    for (_, el) in OUT_CHAIN {
        match *el {
            El::SerC(cap) => series(Cx::new(0.0, -1.0 / (w * cap)), &mut a, &mut b, &mut c, &mut d),
            El::SerL(l) => series(Cx::new(w * l / q.max(1.0), w * l), &mut a, &mut b, &mut c, &mut d),
            El::SerR(r) => series(Cx::new(r, 0.0), &mut a, &mut b, &mut c, &mut d),
            El::ShC(cap) => shunt(Cx::new(0.0, w * cap), &mut a, &mut b, &mut c, &mut d),
            El::ShR(r) => shunt(Cx::new(1.0 / r, 0.0), &mut a, &mut b, &mut c, &mut d),
        }
    }
    let rl = Cx::new(R_LOAD, 0.0);
    let zin = a.mul(rl).add(b).div(c.mul(rl).add(d));
    // V1 = I · (R44 ∥ Zin);  VL = V1 · RL / (a·RL + b)
    let z1 = Cx::new(1.0 / R_SRC, 0.0).add(zin.inv()).inv();
    z1.mul(rl).div(a.mul(rl).add(b))
}

/// Loss of the filter + attenuator relative to an unfiltered path (I into R44 ∥ 50 Ом), dB.
pub fn chain_gain_db(f: f64, q: f64) -> f64 {
    let ideal = R_SRC * R_LOAD / (R_SRC + R_LOAD);
    20.0 * (transimpedance(f, q).abs() / ideal).log10()
}

/// Zero-order hold of the DAC: |sin(πf/fs) / (πf/fs)|.
pub fn sinc(f: f64, fs: f64) -> f64 {
    let x = PI * f / fs;
    if x.abs() < 1e-12 {
        1.0
    } else {
        (x.sin() / x).abs()
    }
}

/// Full-scale DAC current IFS = 72 В / RREF (section 6.4).
pub fn ifs(rref: f64) -> f64 {
    72.0 / rref
}

/// Power at XW2 for a tone of relative amplitude `a` (1 = full scale) at analog frequency `f`.
pub fn tone_dbm(a: f64, f: f64, fs: f64, s: &Settings) -> f64 {
    let ipk = a.abs() * ifs(s.rref_ohm) / 2.0 * sinc(f, fs);
    let v = ipk * transimpedance(f, s.inductor_q).abs();
    10.0 * (v * v / (2.0 * R_LOAD) / 1e-3).max(1e-30).log10()
}

/// Wideband SFDR of the DAC from table 8.1 (fc = 1 ГГц): 67 дБ at 26 МГц, 63 at 126, 56 at 284.
/// Interpolated in log-frequency — an estimate for the board's clock.
pub fn sfdr_spec_db(f: f64) -> f64 {
    let pts: [(f64, f64); 3] = [(26e6, 67.0), (126e6, 63.0), (284e6, 56.0)];
    let lf = f.max(1e6).log10();
    if lf <= pts[0].0.log10() {
        return pts[0].1;
    }
    for p in pts.windows(2) {
        let (x0, y0, x1, y1) = (p[0].0.log10(), p[0].1, p[1].0.log10(), p[1].1);
        if lf <= x1 {
            return y0 + (y1 - y0) * (lf - x0) / (x1 - x0);
        }
    }
    pts[2].1
}

// ───────────────────────────── typed results ─────────────────────────────

#[derive(Serialize, Clone, Debug, Default)]
pub struct ChanCalc {
    pub profile: u32,
    pub ftw: u64,
    pub fout: f64,
    pub phase_deg: f64,
    /// A = mul / 2^12 (signed).
    pub amp: f64,
    /// Offset as a fraction of full scale.
    pub offset: f64,
    /// Channel reaches DAC 1 (ROUTE.sum).
    pub on_dac1: bool,
    pub lfm: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Calc {
    pub fclk: f64,
    pub resolution: f64,
    pub ch1: ChanCalc,
    pub ch2: ChanCalc,
    pub dac1_on: bool,
    pub ifs: f64,
    pub sinc_db: f64,
    pub chain_db: f64,
    pub p_out_dbm: f64,
    pub p_fullscale_dbm: f64,
    /// mul that gives `target_dbm` at XW2 (None if not reachable).
    pub mul_for_target: Option<u32>,
    pub image_hz: f64,
    pub image_dbm: f64,
    pub hd2_hz: f64,
    pub hd2_dbm: f64,
    /// DIP code N for f = band_lo + step·N, if the carrier is on the grid.
    pub dip_n: Option<u32>,
    pub v_outm_max: f64,
    pub i_1v8: f64,
    pub p_d7: f64,
}

fn chan(w: &Words, ch: u8, fclk: f64) -> ChanCalc {
    let pr = regs::get(w, if ch == 1 { "Pr_1" } else { "Pr_2" }) as usize;
    let y = pr.min(regs::n_prof(ch) - 1);
    let ftw = regs::ftw(w, ch, y);
    let p = |n: &str| regs::get_signed(w, &format!("CH{}_{}{}", ch, n, y));
    ChanCalc {
        profile: pr as u32,
        ftw,
        fout: f_of_ftw(ftw, fclk),
        phase_deg: phase_deg(p("P")),
        amp: p("Mul") as f64 / 4096.0,
        offset: p("Offset") as f64 / 2048.0,
        on_dac1: false,
        lfm: regs::get(w, &format!("CH{}_LS_on", ch)) == 1,
    }
}

pub fn calc(w: &Words, s: &Settings) -> Calc {
    let fclk = fclk_eff(w, s);
    let mut ch1 = chan(w, 1, fclk);
    let mut ch2 = chan(w, 2, fclk);
    let sum = regs::get(w, "sum");
    ch1.on_dac1 = sum == 0 || sum == 1 || sum == 3;
    ch2.on_dac1 = sum == 2 || sum == 3;
    let dac1_on = regs::get(w, "DAC1_on") == 1;
    let main = if ch1.on_dac1 { &ch1 } else { &ch2 };
    let f = main.fout;
    let i_fs = ifs(s.rref_ohm);
    let p_fs = tone_dbm(1.0, f, fclk, s);
    let p_out = if dac1_on { tone_dbm(main.amp, f, fclk, s) } else { f64::NEG_INFINITY };
    // mul for the target level: P ∝ mul²
    let need = 4096.0 * 10f64.powf((s.target_dbm - p_fs) / 20.0);
    let mul_for_target = (need.is_finite() && need <= 4095.0 && need >= 1.0).then(|| need.round() as u32);
    let image_hz = fclk - f;
    let image_dbm = if dac1_on { tone_dbm(main.amp, image_hz, fclk, s) } else { f64::NEG_INFINITY };
    let hd2_base = fold(2.0 * f, fclk);
    let hd2_dbm = if dac1_on { tone_dbm(main.amp * 10f64.powf(-sfdr_spec_db(f) / 20.0), hd2_base, fclk, s) } else { f64::NEG_INFINITY };
    let n = (f - s.band_lo_hz) / s.step_hz;
    let dip_n = (n >= -1e-3 && (n - n.round()).abs() < 1e-3 && n.round() <= 4095.0).then(|| n.round() as u32);
    // 1.8 В: up to 400 мА dynamic at 1 ГГц (table 8.1) + 10 мА static; linear in fclk — оценка.
    let i_1v8 = 0.010 + 0.400 * (fclk / 1e9).clamp(0.0, 1.1);
    Calc {
        fclk,
        resolution: fclk / TWO48,
        ch1: ch1.clone(),
        ch2: ch2.clone(),
        dac1_on,
        ifs: i_fs,
        sinc_db: 20.0 * sinc(f, fclk).log10(),
        chain_db: chain_gain_db(f, s.inductor_q),
        p_out_dbm: p_out,
        p_fullscale_dbm: p_fs,
        mul_for_target,
        image_hz,
        image_dbm,
        hd2_hz: hd2_base,
        hd2_dbm,
        dip_n,
        v_outm_max: i_fs * R_OUTM,
        i_1v8,
        p_d7: (5.0 - 1.8) * i_1v8,
    }
}

/// Fold an analog frequency into the first Nyquist zone.
pub fn fold(f: f64, fs: f64) -> f64 {
    let r = f.rem_euclid(fs);
    if r > fs / 2.0 {
        fs - r
    } else {
        r
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Metric {
    pub key: &'static str,
    pub label: &'static str,
    pub value: f64,
    /// Base SI unit ("Hz", "A", "W", "dB", "dBm", "") — the UI adds prefixes.
    pub unit: &'static str,
    pub block: &'static str,
    /// Show in the top bar.
    pub top: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Issue {
    /// "error" | "warn" | "info"
    pub level: &'static str,
    pub block: &'static str,
    pub text: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Derived {
    pub metrics: Vec<Metric>,
    pub issues: Vec<Issue>,
    pub calc: Calc,
}

fn m(key: &'static str, label: &'static str, value: f64, unit: &'static str, block: &'static str, top: bool) -> Metric {
    Metric { key, label, value, unit, block, top }
}

fn mhz(f: f64) -> String {
    format!("{:.6} МГц", f / 1e6).replace('.', ",")
}

pub fn derive(w: &Words, s: &Settings) -> Derived {
    let c = calc(w, s);
    let main = if c.ch1.on_dac1 { &c.ch1 } else { &c.ch2 };
    let mut metrics = vec![
        m("fout", "Частота выхода", main.fout, "Hz", "acc1", true),
        m("pout", "Уровень на XW2", c.p_out_dbm, "dBm", "out", true),
        m("fclk", "Такт ЦАП", c.fclk, "Hz", "clk", true),
        m("res", "Шаг частоты", c.resolution, "Hz", "acc1", true),
        m("profile", "Профиль К1", c.ch1.profile as f64, "", "prof1", true),
        m("amp", "Амплитуда A", main.amp, "", "prof1", false),
        m("ifs", "Ток ЦАП IFS", c.ifs, "A", "dac1", false),
        m("sinc", "Спад sin(x)/x", c.sinc_db, "dB", "dac1", false),
        m("chain", "Затухание ФНЧ+атт.", c.chain_db, "dB", "lpf", false),
        m("image", "Образ fs − f", c.image_dbm, "dBm", "lpf", false),
        m("hd2", "2-я гармоника (оценка)", c.hd2_dbm, "dBm", "lpf", false),
        m("ftw", "FTW (dPh)", main.ftw as f64, "", "acc1", false),
        m("i18", "Ток +1,8 В (оценка)", c.i_1v8, "A", "pwr", false),
        m("pd7", "Рассеяние D7 (оценка)", c.p_d7, "W", "pwr", false),
    ];
    if let Some(mul) = c.mul_for_target {
        metrics.push(m("mul_target", "mul для цели", mul as f64, "", "out", false));
    }
    if c.ch2.on_dac1 && c.ch1.on_dac1 {
        metrics.push(m("fout2", "Частота К2", c.ch2.fout, "Hz", "ch2", false));
    }

    let mut issues = vec![];
    let mut issue = |level, block, text: String| issues.push(Issue { level, block, text });

    // clock
    if s.csel == 0 {
        issue("error", "clk", "Выбран генератор G1 (CSEL = 0), но в ревизии F он не устанавливается (DNP, design_notes п. 4.1). Замкните S1 и подайте такт на XW1.".into());
        if !matches!(regs::get(w, "cmx"), 0 | 1 | 3) {
            issue("warn", "clk", format!("CTR.cmx = {} не описан в ТО (табл. 5.3): допустимы 0 (1:1), 1 (1:2), 3 (1:4).", regs::get(w, "cmx")));
        }
    }
    if c.fclk > 1100e6 {
        issue("error", "clk", format!("Такт ЦАП {} выше предельных 1100 МГц (табл. 8.1).", mhz(c.fclk)));
    } else if c.fclk > 800e6 {
        issue("warn", "clk", format!("Такт ЦАП {} выше 800 МГц — максимума по ТУ АЕЯР.431320.596ТУ (сноска к табл. 8.1).", mhz(c.fclk)));
    }
    if s.clk_dbm < -10.0 || s.clk_dbm > 5.0 {
        issue("error", "clk", format!("Уровень такта {:.1} дБм вне диапазона −10…+5 дБм (табл. 8.1).", s.clk_dbm));
    }
    if !s.sim_clock_ok {
        issue("error", "clk", "Нет тактового сигнала: без такта не работает и SPI (tSCLK ≥ 10·tCLK, табл. 9.1) — DEVID не читается.".into());
    }
    if !s.sim_pgood {
        issue("error", "pwr", "Нет +3,3 В (DDS_PGOOD = 0): прошивка держит DDS в сбросе и выключает буфер D3.".into());
    }

    // output path
    if !c.dac1_on {
        issue("info", "dac1", "ЦАП 1 выключен (CTR.DAC1_on = 0): на XW2 сигнала нет.".into());
    }
    if main.fout > 0.4 * c.fclk {
        issue("warn", "acc1", format!("Частота {} выше 0,4·Fclk: образ fs − f близко, ФНЧ его не подавит.", mhz(main.fout)));
    }
    if main.fout < s.band_lo_hz - 1.0 || main.fout > s.band_hi_hz + 1.0 {
        issue("warn", "out", format!("Частота {} вне рабочей полосы платы {}…{}.", mhz(main.fout), mhz(s.band_lo_hz), mhz(s.band_hi_hz)));
    }
    if c.chain_db < -4.0 && c.dac1_on {
        issue("warn", "lpf", format!("На {} ФНЧ ослабляет сигнал на {:.1} дБ: частота за пределами полосы пропускания (fc ≈ 70 МГц).", mhz(main.fout), -c.chain_db));
    }
    if c.v_outm_max > 0.6 {
        issue("error", "dac1", format!("Напряжение на OUTM1 до {:.2} В — больше допустимых ±0,6 В (разд. 6.4). Увеличьте R11.", c.v_outm_max));
    }
    if !(5e-3..=20e-3).contains(&c.ifs) {
        issue("error", "dac1", format!("IFS = {:.2} мА вне диапазона 5…20 мА (табл. 8.1).", c.ifs * 1e3));
    }
    let peak = |ch: &ChanCalc| ch.amp.abs() + ch.offset.abs();
    let total = if c.ch1.on_dac1 && c.ch2.on_dac1 { peak(&c.ch1) + peak(&c.ch2) } else { peak(main) };
    if total > 1.0 {
        issue("warn", "route", format!("Пик сигнала {:.2} от полной шкалы: отсчёты ограничиваются, появятся гармоники. Уменьшите mul или смещение.", total));
    }
    if main.amp < 0.0 {
        issue("info", "prof1", "Отрицательный mul: сигнал инвертирован (сдвиг фазы на 180°).".into());
    }
    if main.offset != 0.0 {
        issue("info", "prof1", "Постоянное смещение не проходит через разделительный конденсатор C5, но съедает запас по амплитуде.".into());
    }
    if c.dac1_on && (c.p_out_dbm - s.target_dbm).abs() > 1.0 {
        if let Some(mul) = c.mul_for_target {
            issue("info", "out", format!("Уровень {:.1} дБм, цель {:.1} дБм: по модели нужен mul ≈ {} (уточните измерением, design_notes п. 5).", c.p_out_dbm, s.target_dbm, mul));
        }
    }
    match regs::get(w, "sum") {
        2 => issue("info", "route", "На ЦАП 1 подан канал 2 (ROUTE.sum = 2): частоту задают профили канала 2.".into()),
        3 => issue("info", "route", format!("Сумма каналов: на XW2 две несущие — {} и {}.", mhz(c.ch1.fout), mhz(c.ch2.fout))),
        _ => {}
    }
    if regs::get(w, "DAC2_on") == 1 {
        issue("info", "dac2", "ЦАП 2 включён, но его выход на плате нагружен резисторами и не выведен: только лишний ток.".into());
    }
    if regs::get(w, "SEL_IE") == 1 {
        issue("warn", "sync", "SEL_IE = 1: профиль выбирают выводы SEL, а на плате они подтянуты к земле — канал останется в профиле 0.".into());
    }
    if regs::get(w, "LINK_on") == 1 {
        issue("warn", "sync", "LINK включён: регистры ≥ 0x1000 (профили, ЛЧМ) не записываются (табл. 5.6).".into());
    }
    if regs::get(w, "CSYNC_OE") == 1 {
        issue("info", "sync", "Выход CSYNC переключается с частотой Fclk/4 — источник лишних помех; на плате не используется.".into());
    }
    let tsw = regs::get(w, "CH1_TSW");
    if tsw > 0 {
        let t = ((1u32 << tsw) + 20) as f64;
        issue("info", "prof1", format!("Плавное переключение: смена профиля длится {:.0} тактов ({:.0} нс); переключать не чаще.", t, t / c.fclk * 1e9));
    }
    if c.ch1.lfm {
        let l = lfm_read(w, 1, c.fclk);
        issue("info", "lfm1", format!("Режим ЛЧМ: частота меняется от {} до {} за {:.3} мс; запуск — командой CLR.LS1_start.", mhz(l.f_start), mhz(l.f_end), l.t_ramp * 1e3));
        if l.f_end < 0.0 || l.f_end > c.fclk / 2.0 {
            issue("warn", "lfm1", "Конечная частота ЛЧМ выходит за 0…Fclk/2.".into());
        }
        if l.t_ramp == 0.0 {
            issue("warn", "lfm1", "Длительность стадии 1 равна нулю (LS_TPH1).".into());
        }
    }
    if (regs::get(w, "Pr_1") as usize) >= regs::N_PROF1 {
        issue("warn", "prof1", format!("Профиль {} есть в кристалле, но в приложении показаны только 0…{}.", regs::get(w, "Pr_1"), regs::N_PROF1 - 1));
    }
    if c.p_d7 > 1.0 {
        issue("warn", "pwr", format!("D7 1325ЕН1.8У рассеивает ≈ {:.2} Вт (оценка по току ядра) — проверьте нагрев (design_notes п. 4.6).", c.p_d7));
    }
    Derived { metrics, issues, calc: c }
}

// ───────────────────────────── frequency plan ─────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct FreqPlan {
    pub target: f64,
    pub ftw: u64,
    pub ftw_hex: String,
    pub words: [u16; 3],
    pub actual: f64,
    pub error_hz: f64,
    pub resolution: f64,
    pub dip_n: Option<u32>,
    /// S2.1…S2.8, S3.1…S3.4 — true = ON (bit = 1).
    pub dip_on: Vec<bool>,
    pub in_band: bool,
    pub level_dbm: f64,
}

pub fn plan_frequency(f: f64, w: &Words, s: &Settings, amp: f64) -> FreqPlan {
    let fclk = fclk_eff(w, s);
    let ftw = ftw_for(f, fclk);
    let actual = f_of_ftw(ftw, fclk);
    let n = (f - s.band_lo_hz) / s.step_hz;
    let dip_n = (n >= 0.0 && (n - n.round()).abs() < 1e-6 && n.round() <= 4000.0).then(|| n.round() as u32);
    FreqPlan {
        target: f,
        ftw,
        ftw_hex: format!("{:012X}", ftw),
        words: [ftw as u16, (ftw >> 16) as u16, (ftw >> 32) as u16],
        actual,
        error_hz: actual - f,
        resolution: fclk / TWO48,
        dip_n,
        dip_on: (0..12).map(|i| dip_n.map(|n| (n >> i) & 1 == 1).unwrap_or(false)).collect(),
        in_band: f >= s.band_lo_hz && f <= s.band_hi_hz,
        level_dbm: tone_dbm(amp, actual, fclk, s),
    }
}

// ───────────────────────────── board response ─────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct Response {
    pub f: Vec<f64>,
    /// Filter + attenuator, dB (relative to an unfiltered 25 Ом path).
    pub chain_db: Vec<f64>,
    /// DAC zero-order hold, dB.
    pub sinc_db: Vec<f64>,
    /// Level at XW2 for the current amplitude, dBm.
    pub out_dbm: Vec<f64>,
    pub fc_3db: f64,
    pub band_ripple_db: f64,
    pub elements: Vec<(String, String)>,
}

pub fn response(w: &Words, s: &Settings, f_lo: f64, f_hi: f64, n: usize) -> Response {
    let c = calc(w, s);
    let amp = if c.ch1.on_dac1 { c.ch1.amp } else { c.ch2.amp };
    let n = n.clamp(16, 4000);
    let log = f_lo > 0.0 && f_hi / f_lo > 20.0;
    let f: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            if log {
                f_lo * (f_hi / f_lo).powf(t)
            } else {
                f_lo + (f_hi - f_lo) * t
            }
        })
        .collect();
    let chain_db: Vec<f64> = f.iter().map(|&x| chain_gain_db(x, s.inductor_q)).collect();
    let sinc_db: Vec<f64> = f.iter().map(|&x| 20.0 * sinc(x, c.fclk).log10()).collect();
    let out_dbm = f.iter().map(|&x| tone_dbm(amp, x, c.fclk, s)).collect();
    // −3 dB point relative to the band centre
    let ref_db = chain_gain_db((s.band_lo_hz + s.band_hi_hz) / 2.0, s.inductor_q);
    let mut fc = f64::NAN;
    let mut x = 20e6;
    while x < 400e6 {
        if chain_gain_db(x, s.inductor_q) < ref_db - 3.0 {
            fc = x;
            break;
        }
        x += 0.1e6;
    }
    let band: Vec<f64> = (0..=40).map(|i| s.band_lo_hz + (s.band_hi_hz - s.band_lo_hz) * i as f64 / 40.0).map(|x| chain_gain_db(x, s.inductor_q) + 20.0 * sinc(x, c.fclk).log10()).collect();
    let ripple = band.iter().cloned().fold(f64::MIN, f64::max) - band.iter().cloned().fold(f64::MAX, f64::min);
    let elements = OUT_CHAIN
        .iter()
        .map(|(n, e)| {
            let v = match *e {
                El::SerC(x) => format!("{} пФ, последовательно", fmt_num(x * 1e12)),
                El::ShC(x) => format!("{} пФ на землю", fmt_num(x * 1e12)),
                El::SerL(x) => format!("{} нГн, последовательно", fmt_num(x * 1e9)),
                El::SerR(x) => format!("{} Ом, последовательно", fmt_num(x)),
                El::ShR(x) => format!("{} Ом на землю", fmt_num(x)),
            };
            (n.to_string(), v)
        })
        .collect();
    Response { f, chain_db, sinc_db, out_dbm, fc_3db: fc, band_ripple_db: ripple, elements }
}

fn fmt_num(x: f64) -> String {
    let s = if x >= 100.0 { format!("{:.0}", x) } else { format!("{}", (x * 100.0).round() / 100.0) };
    s.replace('.', ",")
}

// ───────────────────────────── digital DDS simulation ─────────────────────────────

/// Bit-true model of one channel (section 3): 48-bit phase accumulator → 17 bits,
/// + phase offset P (16 bits) + 4-bit phase dither → 15 bits → sine 12 bits ×
/// mul (13 bits) → + offset (12 bits).
struct ChanSim {
    acc: u64,
    ftw: u64,
    p: u32,
    mul: i64,
    off: i64,
    dither: bool,
    bypass: bool,
    on: bool,
}

pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn uniform(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl ChanSim {
    fn new(w: &Words, ch: u8, on: bool) -> Self {
        let pr = (regs::get(w, if ch == 1 { "Pr_1" } else { "Pr_2" }) as usize).min(regs::n_prof(ch) - 1);
        let g = |n: &str| regs::get_signed(w, &format!("CH{}_{}{}", ch, n, pr));
        ChanSim {
            acc: 0,
            ftw: regs::ftw(w, ch, pr),
            p: (g("P") as u32) & 0xFFFF,
            mul: g("Mul"),
            off: g("Offset"),
            dither: regs::get(w, if ch == 1 { "pdith1" } else { "pdith2" }) == 1,
            bypass: regs::get(w, &format!("CH{}_PA_bypass", ch)) == 1,
            on,
        }
    }
    /// One clock: 12-bit signed value.
    fn step(&mut self, rng: &mut Rng, lut: &[i64]) -> i64 {
        if !self.on {
            return 0;
        }
        let ph17 = (self.acc >> 31) as u32;
        self.acc = (self.acc + self.ftw) & ((1u64 << 48) - 1);
        let d = if self.dither { (rng.next() & 0xF) as u32 } else { 0 };
        let sum = (ph17 + (self.p << 1) + d) & 0x1FFFF;
        let ph15 = (sum >> 2) as usize;
        let a = if self.bypass { (ph15 as i64 >> 3) - 2048 } else { lut[ph15] };
        ((a * self.mul) >> 12) + self.off
    }
}

/// DAC 1 codes (10-bit, signed −512…511) for `n` clocks of the current configuration.
pub fn dac_samples(w: &Words, n: usize, seed: u64) -> Vec<i32> {
    let lut: Vec<i64> = (0..(1 << 15)).map(|i| (2047.0 * (2.0 * PI * i as f64 / 32768.0).sin()).round() as i64).collect();
    let sum = regs::get(w, "sum");
    let mut c1 = ChanSim::new(w, 1, sum != 2);
    let mut c2 = ChanSim::new(w, 2, sum >= 2);
    let adith = regs::get(w, "adith1");
    let mut rng = Rng::new(seed);
    if regs::get(w, "DAC1_on") == 0 {
        return vec![0; n];
    }
    (0..n)
        .map(|_| {
            let mut v = c1.step(&mut rng, &lut) + c2.step(&mut rng, &lut);
            // amplitude dither before 12 → 10 bit truncation (table 5.7): ½ LSB or 8 LSB of the DAC
            v += match adith {
                1 => ((rng.uniform() - 0.5) * 2.0).round() as i64,
                3 => ((rng.uniform() - 0.5) * 32.0).round() as i64,
                _ => 0,
            };
            (v.clamp(-2048, 2047) >> 2) as i32
        })
        .collect()
}

fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for i in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (ar, ai) = (re[i + k], im[i + k]);
                let (br, bi) = (re[i + k + len / 2], im[i + k + len / 2]);
                let (tr, ti) = (br * cr - bi * ci, br * ci + bi * cr);
                re[i + k] = ar + tr;
                im[i + k] = ai + ti;
                re[i + k + len / 2] = ar - tr;
                im[i + k + len / 2] = ai - ti;
                let t = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = t;
            }
        }
        len <<= 1;
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Tone {
    pub name: String,
    pub f: f64,
    pub dbm: f64,
    pub dbc: f64,
    /// "carrier" | "image" | "harmonic" | "spur"
    pub kind: &'static str,
}

#[derive(Serialize, Clone, Debug)]
pub struct Spectrum {
    pub f: Vec<f64>,
    pub dbm: Vec<f64>,
    /// Digital spectrum of DAC codes in the first Nyquist zone, dBFS.
    pub dig_f: Vec<f64>,
    pub dig_dbfs: Vec<f64>,
    pub tones: Vec<Tone>,
    pub fs: f64,
    pub n: usize,
    pub rbw_hz: f64,
    pub carrier_dbm: f64,
    /// Worst spur in the displayed span relative to the carrier, dBc (positive number).
    pub sfdr_db: f64,
    /// Worst digital spur in the first Nyquist zone, dBc.
    pub sfdr_digital_db: f64,
    pub samples: Vec<i32>,
}

/// Spectrum at XW2: bit-true digital simulation + FFT, then every bin is mapped to
/// all Nyquist images k·fs ± f inside [f_lo, f_hi] through sinc × filter × attenuator.
/// DAC harmonics (analog nonlinearity) are added from the SFDR of table 8.1 — оценка.
pub fn spectrum(w: &Words, s: &Settings, f_lo: f64, f_hi: f64, log2n: u32, points: usize) -> Spectrum {
    let fs = fclk_eff(w, s);
    let n = 1usize << log2n.clamp(10, 16);
    let codes = dac_samples(w, n, 0x9E37_79B9_7F4A_7C15);
    // Blackman-Harris 4-term window
    let win: Vec<f64> = (0..n)
        .map(|i| {
            let x = 2.0 * PI * i as f64 / n as f64;
            0.35875 - 0.48829 * x.cos() + 0.14128 * (2.0 * x).cos() - 0.01168 * (3.0 * x).cos()
        })
        .collect();
    let wsum: f64 = win.iter().sum();
    let mut re: Vec<f64> = codes.iter().zip(&win).map(|(&c, &wv)| c as f64 / 511.5 * wv).collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im);
    let half = n / 2;
    let mut amp: Vec<f64> = (0..=half).map(|k| 2.0 * re[k].hypot(im[k]) / wsum).collect();
    amp[0] /= 2.0;
    let bin = fs / n as f64;

    let c = calc(w, s);
    let carriers: Vec<&ChanCalc> = [&c.ch1, &c.ch2].into_iter().filter(|ch| ch.on_dac1 && c.dac1_on && ch.amp != 0.0).collect();
    // DAC harmonics from the datasheet SFDR (оценка): HD2 = −SFDR, HD3 = −(SFDR + 3)
    let mut extra: Vec<(String, f64, f64)> = vec![];
    for ch in &carriers {
        let sf = sfdr_spec_db(ch.fout);
        for (h, d) in [(2.0, 0.0), (3.0, 3.0)] {
            let a = ch.amp.abs() * 10f64.powf(-(sf + d) / 20.0);
            extra.push((format!("HD{} = {}·{}", h as u32, h as u32, short_mhz(ch.fout)), fold(h * ch.fout, fs), a));
        }
    }
    let mut amp_h = amp.clone();
    for (_, fb, a) in &extra {
        let k = (fb / bin).round() as usize;
        if k <= half {
            amp_h[k] = (amp_h[k].powi(2) + a * a).sqrt();
        }
    }

    // display buckets
    let points = points.clamp(200, 6000);
    let span = (f_hi - f_lo).max(bin);
    let df = span / points as f64;
    let mut dbm = vec![-200.0f64; points];
    let fs_ref: Vec<f64> = (0..points).map(|i| tone_dbm(1.0, f_lo + (i as f64 + 0.5) * df, fs, s)).collect();
    let zmax = (f_hi / fs).ceil() as i64 + 1;
    for (k, &a) in amp_h.iter().enumerate() {
        let fb = k as f64 * bin;
        let lvl = 20.0 * a.max(1e-12).log10();
        for z in 0..=zmax {
            for sign in [-1.0, 1.0] {
                if z == 0 && sign < 0.0 {
                    continue;
                }
                let f = z as f64 * fs + sign * fb;
                if f < f_lo || f > f_hi {
                    continue;
                }
                let i = (((f - f_lo) / df) as usize).min(points - 1);
                dbm[i] = dbm[i].max(lvl + fs_ref[i]);
            }
        }
    }
    let f: Vec<f64> = (0..points).map(|i| f_lo + (i as f64 + 0.5) * df).collect();

    // digital spectrum (decimated with max-hold to ≤ 2048 points)
    let dstep = (half / 2048).max(1);
    let mut dig_f = vec![];
    let mut dig_dbfs = vec![];
    for chunk in (0..=half).collect::<Vec<_>>().chunks(dstep) {
        let mx = chunk.iter().map(|&k| amp[k]).fold(0.0, f64::max);
        dig_f.push(chunk[0] as f64 * bin);
        dig_dbfs.push(20.0 * mx.max(1e-12).log10());
    }

    // tones
    let mut tones: Vec<Tone> = vec![];
    let carrier_dbm = carriers.first().map(|ch| tone_dbm(ch.amp, ch.fout, fs, s)).unwrap_or(f64::NEG_INFINITY);
    let mut push = |name: String, f: f64, a: f64, kind: &'static str| {
        let d = tone_dbm(a, f, fs, s);
        tones.push(Tone { name, f, dbm: d, dbc: d - carrier_dbm, kind });
    };
    for (i, ch) in carriers.iter().enumerate() {
        let lab = if carriers.len() > 1 { format!(" К{}", i + 1) } else { String::new() };
        push(format!("Несущая{}", lab), ch.fout, ch.amp, "carrier");
        push(format!("Образ fs − f{}", lab), fs - ch.fout, ch.amp, "image");
        push(format!("Образ fs + f{}", lab), fs + ch.fout, ch.amp, "image");
        push(format!("Образ 2fs − f{}", lab), 2.0 * fs - ch.fout, ch.amp, "image");
    }
    for (name, fb, a) in &extra {
        push(name.clone(), *fb, *a, "harmonic");
    }
    // largest digital spur away from carriers and harmonics
    let near = |fb: f64| carriers.iter().any(|ch| (fb - fold(ch.fout, fs)).abs() < 8.0 * bin) || extra.iter().any(|(_, x, _)| (fb - x).abs() < 8.0 * bin) || fb < 8.0 * bin;
    let mut best = (0usize, 0.0f64);
    for k in 1..half {
        let fb = k as f64 * bin;
        if !near(fb) && amp[k] > best.1 && amp[k] >= amp[k - 1] && amp[k] >= amp[k + 1] {
            best = (k, amp[k]);
        }
    }
    let carrier_a = carriers.first().map(|ch| ch.amp.abs()).unwrap_or(1.0).max(1e-9);
    let sfdr_digital_db = if best.1 > 0.0 { 20.0 * (carrier_a / best.1).log10() } else { f64::INFINITY };
    if best.1 > 0.0 {
        push("Цифровой спур (усечение фазы/амплитуды)".into(), best.0 as f64 * bin, best.1, "spur");
    }
    // SFDR in the displayed span: worst non-carrier bucket
    let sfdr_db = {
        let mut worst = f64::NEG_INFINITY;
        for (i, &fv) in f.iter().enumerate() {
            if carriers.iter().any(|ch| (fv - ch.fout).abs() < 8.0 * bin.max(df)) {
                continue;
            }
            worst = worst.max(dbm[i]);
        }
        carrier_dbm - worst
    };
    tones.sort_by(|a, b| b.dbm.partial_cmp(&a.dbm).unwrap_or(std::cmp::Ordering::Equal));
    Spectrum {
        f,
        dbm,
        dig_f,
        dig_dbfs,
        tones,
        fs,
        n,
        rbw_hz: 2.0 * bin,
        carrier_dbm,
        sfdr_db,
        sfdr_digital_db,
        samples: codes.into_iter().take(256).collect(),
    }
}

fn short_mhz(f: f64) -> String {
    format!("{:.3} МГц", f / 1e6).replace('.', ",")
}

// ───────────────────────────── phase noise ─────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct PhaseNoise {
    pub offsets: Vec<f64>,
    pub dbc_hz: Vec<f64>,
    pub fout: f64,
    pub jitter_s: f64,
    pub ref_curves: Vec<(f64, Vec<(f64, f64)>)>,
}

/// Table 8.1 points (fc = 1 ГГц): output frequency → (offset, dBc/Hz).
const PN_TABLE: &[(f64, &[(f64, f64)])] = &[
    (10e6, &[(1e3, -145.0), (1e4, -155.0), (1e5, -165.0), (1e6, -172.0)]),
    (20e6, &[(1e3, -140.0), (1e4, -150.0), (1e5, -162.0), (1e6, -167.0)]),
    (80e6, &[(1e3, -135.0), (1e4, -145.0), (1e5, -152.0), (1e6, -155.0)]),
    (160e6, &[(1e4, -140.0), (1e5, -145.0), (1e6, -149.0)]),
];

fn pn_at(curve: &[(f64, f64)], off: f64) -> f64 {
    let lo = off.log10();
    if lo <= curve[0].0.log10() {
        // extrapolate the first decade slope
        let s = (curve[1].1 - curve[0].1) / (curve[1].0.log10() - curve[0].0.log10());
        return curve[0].1 + s * (lo - curve[0].0.log10());
    }
    for p in curve.windows(2) {
        if lo <= p[1].0.log10() {
            let t = (lo - p[0].0.log10()) / (p[1].0.log10() - p[0].0.log10());
            return p[0].1 + t * (p[1].1 - p[0].1);
        }
    }
    curve[curve.len() - 1].1
}

/// Phase noise at the current output frequency: interpolated between the table 8.1
/// curves in log(fout) — оценка (the table is for fc = 1 ГГц and an ideal clock).
pub fn phase_noise(w: &Words, s: &Settings) -> PhaseNoise {
    let c = calc(w, s);
    let fout = if c.ch1.on_dac1 { c.ch1.fout } else { c.ch2.fout }.max(1e6);
    let offsets: Vec<f64> = (0..=60).map(|i| 1e3 * 10f64.powf(i as f64 / 20.0)).collect();
    let lf = fout.log10();
    let dbc_hz: Vec<f64> = offsets
        .iter()
        .map(|&o| {
            let (mut a, mut b) = (&PN_TABLE[0], &PN_TABLE[1]);
            for p in PN_TABLE.windows(2) {
                a = &p[0];
                b = &p[1];
                if lf <= p[1].0.log10() {
                    break;
                }
            }
            let t = ((lf - a.0.log10()) / (b.0.log10() - a.0.log10())).clamp(-1.0, 2.0);
            pn_at(a.1, o) + t * (pn_at(b.1, o) - pn_at(a.1, o))
        })
        .collect();
    // integrate L(f) over 1 кГц…1 МГц: σ = sqrt(2·∫10^(L/10) df) / (2π·fout)
    let mut integral = 0.0;
    for i in 1..offsets.len() {
        let (f0, f1) = (offsets[i - 1], offsets[i]);
        let (l0, l1) = (10f64.powf(dbc_hz[i - 1] / 10.0), 10f64.powf(dbc_hz[i] / 10.0));
        integral += 0.5 * (l0 + l1) * (f1 - f0);
    }
    let jitter_s = (2.0 * integral).sqrt() / (2.0 * PI * fout);
    PhaseNoise { offsets, dbc_hz, fout, jitter_s, ref_curves: PN_TABLE.iter().map(|(f, c)| (*f, c.to_vec())).collect() }
}

// ───────────────────────────── LFM (ЛЧМ) ─────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LfmReq {
    pub f_start: f64,
    pub f_stop: f64,
    pub t_ramp: f64,
    pub t_hold: f64,
    /// "saw" — пила, "tri" — треугольник, "single" — однократно
    pub shape: String,
    /// Switch the output off in stages 2 and 4 (pulsed LFM).
    pub pause_off: bool,
}

impl Default for LfmReq {
    fn default() -> Self {
        LfmReq { f_start: 48e6, f_stop: 52e6, t_ramp: 1e-3, t_hold: 0.2e-3, shape: "saw".into(), pause_off: false }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct LfmPlan {
    pub f_start: f64,
    pub f_end: f64,
    pub t_ramp: f64,
    pub slope_hz_s: f64,
    pub slope_step: f64,
    pub df1: i64,
    pub df2: i64,
    pub tph: [u64; 4],
    /// Field values to write (name → value).
    pub fields: Vec<(String, u32)>,
    /// f(t) over two cycles: (t, f, on).
    pub curve_t: Vec<f64>,
    pub curve_f: Vec<f64>,
    pub curve_on: Vec<bool>,
    pub period: f64,
    pub notes: Vec<String>,
}

fn tph_of(t: f64, fclk: f64) -> u64 {
    ((t * fclk / 4.0).round() as u64).clamp(1, (1u64 << 46) - 1)
}

/// Design the 4-stage LFM cycle of channel 1 (section 3.3).
/// dF per clock = slope · 2^48 / fclk², stage time = 4·TPH / fclk.
pub fn lfm_design(req: &LfmReq, w: &Words, s: &Settings) -> LfmPlan {
    let fclk = fclk_eff(w, s);
    let tph1 = tph_of(req.t_ramp, fclk);
    let t_ramp = tph1 as f64 * 4.0 / fclk;
    let slope = (req.f_stop - req.f_start) / t_ramp;
    let df1 = (slope * TWO48 / (fclk * fclk)).round() as i64;
    let f1 = ftw_for(req.f_start, fclk);
    let hold = tph_of(req.t_hold.max(0.0), fclk);
    let (df2, tph) = match req.shape.as_str() {
        "tri" => (-df1, [tph1, hold, tph1, hold]),
        "single" => (0, [tph1, hold, 1, 1]),
        _ => (0, [tph1, hold, 1, 1]),
    };
    let f_start = f_of_ftw(f1, fclk);
    let step = df1 as f64 * fclk / TWO48; // Hz per clock
    let f_end = f_start + step * 4.0 * tph1 as f64;
    let mut fields: Vec<(String, u32)> = vec![
        ("CH1_LS_on".into(), 1),
        ("CH1_frq_reset_1".into(), 1),
        ("CH1_frq_reset_3".into(), 0),
        ("CH1_ph_reset_1".into(), 0),
        ("CH1_ph_reset_3".into(), 0),
        ("CH1_s2_on".into(), (!req.pause_off) as u32),
        ("CH1_s4_on".into(), (!req.pause_off) as u32),
        ("CH1_s2_f0".into(), 0),
        ("CH1_s4_f0".into(), 0),
        ("CH1_auto".into(), (req.shape != "single") as u32),
        ("CH1_corr_enable".into(), 0),
    ];
    let mut put48 = |base: &str, v: u64| {
        for (i, p) in ["L", "M", "H"].iter().enumerate() {
            let mask = if base.contains("TPH") && i == 2 { 0x3FFF } else { 0xFFFF };
            fields.push((format!("{}_{}", base, p), ((v >> (16 * i)) as u32) & mask));
        }
    };
    put48("CH1_LS_F1", f1);
    put48("CH1_LS_F2", f1);
    put48("CH1_LS_dF1", (df1 as u64) & ((1u64 << 48) - 1));
    put48("CH1_LS_dF2", (df2 as u64) & ((1u64 << 48) - 1));
    for (k, t) in tph.iter().enumerate() {
        put48(&format!("CH1_LS_TPH{}", k + 1), *t);
    }
    // curve over two cycles
    let dur: Vec<f64> = tph.iter().map(|&t| t as f64 * 4.0 / fclk).collect();
    let period: f64 = dur.iter().sum();
    let mut curve_t = vec![];
    let mut curve_f = vec![];
    let mut curve_on = vec![];
    let mut t = 0.0;
    let mut f = f_start;
    let cycles = if req.shape == "single" { 1 } else { 2 };
    for _ in 0..cycles {
        for (k, d) in dur.iter().enumerate() {
            if k == 0 {
                f = f_start; // frq_reset_1
            }
            let slope_k = match k {
                0 => slope,
                2 => df2 as f64 * fclk * fclk / TWO48,
                _ => 0.0,
            };
            let on = !(req.pause_off && (k == 1 || k == 3));
            for i in 0..=20 {
                let tt = d * i as f64 / 20.0;
                curve_t.push(t + tt);
                curve_f.push(f + slope_k * tt);
                curve_on.push(on);
            }
            f += slope_k * d;
            t += d;
        }
    }
    let mut notes = vec![];
    if f_end < 0.0 || f_end > fclk / 2.0 || f_start > fclk / 2.0 {
        notes.push("Частота выходит за 0…Fclk/2 — получится зеркальная ЛЧМ.".into());
    }
    if req.f_start < s.band_lo_hz || req.f_stop > s.band_hi_hz || req.f_stop < s.band_lo_hz {
        notes.push("Часть ЛЧМ вне рабочей полосы платы 48–52 МГц: уровень на XW2 будет меняться из-за ФНЧ.".into());
    }
    if df1 == 0 && req.f_stop != req.f_start {
        notes.push("Наклон меньше одного шага dF — увеличьте девиацию или уменьшите время.".into());
    }
    LfmPlan { f_start, f_end, t_ramp, slope_hz_s: df1 as f64 * fclk * fclk / TWO48, slope_step: fclk * fclk / TWO48, df1, df2, tph, fields, curve_t, curve_f, curve_on, period, notes }
}

#[derive(Serialize, Clone, Debug)]
pub struct LfmState {
    pub f_start: f64,
    pub f_end: f64,
    pub t_ramp: f64,
}

/// LFM parameters as currently written in channel `ch`.
pub fn lfm_read(w: &Words, ch: u8, fclk: f64) -> LfmState {
    let p = format!("CH{}_", ch);
    let f1 = regs::get48(w, &format!("{}LS_F1", p));
    let mut df = regs::get48(w, &format!("{}LS_dF1", p)) as i64;
    if df & (1 << 47) != 0 {
        df -= 1 << 48;
    }
    let tph = regs::get48(w, &format!("{}LS_TPH1", p));
    let f_start = f_of_ftw(f1, fclk);
    LfmState { f_start, f_end: f_start + df as f64 * fclk / TWO48 * 4.0 * tph as f64, t_ramp: tph as f64 * 4.0 / fclk }
}

// ───────────────────────────── manipulation (profiles) ─────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ModReq {
    /// fsk2 | fsk4 | bpsk | qpsk | psk8 | ask2 | ask4 | qam16
    pub kind: String,
    pub f_center: f64,
    /// FSK: distance between adjacent tones, Hz.
    pub deviation: f64,
    /// Peak amplitude, mul (≤ 4095).
    pub mul: u32,
}

impl Default for ModReq {
    fn default() -> Self {
        ModReq { kind: "qpsk".into(), f_center: 50e6, deviation: 100e3, mul: 4095 }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ModSymbol {
    pub profile: usize,
    pub bits: String,
    pub f: f64,
    pub phase_deg: f64,
    pub mul: i64,
    pub i: f64,
    pub q: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct ModPlan {
    pub kind: String,
    pub title: String,
    pub bits_per_symbol: u32,
    pub symbols: Vec<ModSymbol>,
    pub fields: Vec<(String, u32)>,
    /// A demo symbol sequence (indices) from a PRBS.
    pub sequence: Vec<usize>,
    pub notes: Vec<String>,
}

fn gray(i: usize) -> usize {
    i ^ (i >> 1)
}

/// Fill channel-1 profiles 0…M−1 for a manipulation scheme (section 3.2: modulation
/// = switching between pre-programmed profiles).
pub fn modulation(req: &ModReq, w: &Words, s: &Settings) -> ModPlan {
    let fclk = fclk_eff(w, s);
    let a = req.mul.min(4095) as f64;
    let (m, title): (usize, &str) = match req.kind.as_str() {
        "fsk2" => (2, "2-FSK: две частоты"),
        "fsk4" => (4, "4-FSK: четыре частоты"),
        "bpsk" => (2, "BPSK: фаза 0° / 180°"),
        "psk8" => (8, "8-PSK: восемь фаз"),
        "ask2" => (2, "ASK (OOK): есть / нет сигнала"),
        "ask4" => (4, "4-ASK: четыре уровня"),
        "qam16" => (16, "16-QAM: фаза и амплитуда"),
        _ => (4, "QPSK: фазы 45°, 135°, 225°, 315°"),
    };
    let k = (m as f64).log2() as u32;
    let mut symbols = vec![];
    for y in 0..m {
        let g = gray(y);
        let (f, ph, amp) = match req.kind.as_str() {
            "fsk2" | "fsk4" => (req.f_center + req.deviation * (y as f64 - (m as f64 - 1.0) / 2.0), 0.0, a),
            "bpsk" => (req.f_center, 180.0 * y as f64, a),
            "psk8" => (req.f_center, 45.0 * y as f64, a),
            "ask2" => (req.f_center, 0.0, a * y as f64),
            "ask4" => (req.f_center, 0.0, a * (y as f64) / 3.0),
            "qam16" => {
                let lv = [-3.0, -1.0, 1.0, 3.0];
                let (i, q) = (lv[y % 4], lv[y / 4]);
                let r: f64 = (i * i + q * q as f64).sqrt() / (18f64).sqrt();
                (req.f_center, q.atan2(i).to_degrees(), a * r)
            }
            _ => (req.f_center, 45.0 + 90.0 * y as f64, a),
        };
        let mul = amp.round() as i64;
        let pw = phase_word(ph);
        let ph_q = phase_deg(pw);
        let ftw = ftw_for(f, fclk);
        symbols.push(ModSymbol {
            profile: y,
            bits: format!("{:0width$b}", g, width = k as usize),
            f: f_of_ftw(ftw, fclk),
            phase_deg: ph_q,
            mul,
            i: mul as f64 / 4096.0 * ph_q.to_radians().cos(),
            q: mul as f64 / 4096.0 * ph_q.to_radians().sin(),
        });
    }
    let mut fields = vec![];
    for sym in &symbols {
        let ftw = ftw_for(sym.f, fclk);
        for (i, p) in ["L", "M", "H"].iter().enumerate() {
            fields.push((format!("CH1_dPh{}_{}", sym.profile, p), ((ftw >> (16 * i)) & 0xFFFF) as u32));
        }
        fields.push((format!("CH1_P{}", sym.profile), (phase_word(sym.phase_deg) as u32) & 0xFFFF));
        fields.push((format!("CH1_Mul{}", sym.profile), (sym.mul as u32) & 0x1FFF));
        fields.push((format!("CH1_Offset{}", sym.profile), 0));
    }
    fields.push(("CH1_LS_on".into(), 0));
    let mut rng = Rng::new(0xACE1);
    let sequence = (0..24).map(|_| (rng.next() >> 33) as usize % m).collect();
    let mut notes = vec![];
    if req.kind.starts_with("fsk") && (req.f_center - req.deviation * m as f64 / 2.0 < s.band_lo_hz || req.f_center + req.deviation * m as f64 / 2.0 > s.band_hi_hz) {
        notes.push("Часть частот вне полосы 48–52 МГц.".into());
    }
    notes.push("Символы переключаются записью Pr_1 в SEL_REG. С ПК через UART — не быстрее ≈ 10 мс на символ; для быстрой манипуляции нужны выводы SEL или прошивка.".into());
    ModPlan { kind: req.kind.clone(), title: title.into(), bits_per_symbol: k, symbols, fields, sequence, notes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ftw_matches_firmware_rounding() {
        // round(50e6 · 2^48 / 480e6) — the firmware's long division gives the same
        let ftw = ftw_for(50e6, 480e6);
        let exact = 50e6 * TWO48 / 480e6;
        assert!((ftw as f64 - exact).abs() <= 0.5);
        assert_eq!(ftw, 29_320_310_074_027); // 2^48 · 5/48, rounded half up
        assert!((f_of_ftw(ftw, 480e6) - 50e6).abs() < 1e-6);
        assert!((480e6 / TWO48 - 1.705e-6).abs() < 1e-9);
    }

    #[test]
    fn starter_has_no_errors_and_sane_level() {
        let d = derive(&regs::starter(), &Settings::default());
        assert!(d.issues.iter().all(|i| i.level != "error"), "{:?}", d.issues);
        let c = &d.calc;
        assert!((c.ch1.fout - 50e6).abs() < 1e-3);
        assert!((c.ifs - 10.07e-3).abs() < 0.01e-3);
        // design: about −10 dBm at full scale through a 1 dB pad
        assert!(c.p_out_dbm > -12.0 && c.p_out_dbm < -7.0, "{}", c.p_out_dbm);
        assert!((c.v_outm_max - 0.503).abs() < 0.002, "design_notes: ≤ 0,503 В");
        assert!(c.image_dbm < c.p_out_dbm - 60.0, "image at 430 MHz must be killed by the LPF");
    }

    #[test]
    fn filter_shape() {
        let q = 30.0;
        let pass = chain_gain_db(50e6, q);
        assert!(pass > -2.5 && pass < 0.0, "passband incl. 1 dB pad: {}", pass);
        assert!(chain_gain_db(100e6, q) < pass - 15.0, "2nd harmonic suppression ≥ 15 dB (notes: 18–23)");
        assert!(chain_gain_db(430e6, q) < -60.0);
    }

    #[test]
    fn issues_fire() {
        let s = Settings::default();
        let mut w = regs::starter();
        regs::set(&mut w, "DAC1_on", 0);
        assert!(derive(&w, &s).issues.iter().any(|i| i.block == "dac1"));
        let s2 = Settings { csel: 0, ..Settings::default() };
        assert!(derive(&regs::starter(), &s2).issues.iter().any(|i| i.level == "error" && i.block == "clk"));
        let s3 = Settings { rref_ohm: 3000.0, ..Settings::default() };
        assert!(derive(&regs::starter(), &s3).issues.iter().any(|i| i.level == "error" && i.block == "dac1"));
    }

    #[test]
    fn spectrum_finds_carrier_and_kills_image() {
        let s = Settings::default();
        let sp = spectrum(&regs::starter(), &s, 0.0, 1000e6, 13, 2000);
        let carrier = sp.tones.iter().find(|t| t.kind == "carrier").unwrap();
        assert!((carrier.f - 50e6).abs() < 1.0);
        let img = sp.tones.iter().find(|t| t.name.starts_with("Образ fs − f")).unwrap();
        assert!((img.f - 430e6).abs() < 1.0);
        assert!(img.dbc < -60.0);
        // the trace has the carrier peak near 50 MHz
        let (i, _) = sp.dbm.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap();
        assert!((sp.f[i] - 50e6).abs() < 2e6, "{}", sp.f[i]);
        assert!(sp.sfdr_digital_db > 55.0, "10-bit DAC: {}", sp.sfdr_digital_db);
    }

    #[test]
    fn lfm_design_round_trip() {
        let s = Settings::default();
        let w = regs::starter();
        let p = lfm_design(&LfmReq::default(), &w, &s);
        assert_eq!(p.tph[0], 120_000); // 1 ms · 480 MHz / 4
        assert!((p.f_end - 52e6).abs() < 1e3, "{}", p.f_end);
        let mut w2 = w;
        for (n, v) in &p.fields {
            assert!(regs::set(&mut w2, n, *v), "{} = {}", n, v);
        }
        let st = lfm_read(&w2, 1, 480e6);
        assert!((st.f_start - 48e6).abs() < 1.0 && (st.f_end - 52e6).abs() < 1e3);
    }

    #[test]
    fn qam16_fills_16_profiles() {
        let p = modulation(&ModReq { kind: "qam16".into(), ..ModReq::default() }, &regs::starter(), &Settings::default());
        assert_eq!(p.symbols.len(), 16);
        let mut w = regs::starter();
        for (n, v) in &p.fields {
            assert!(regs::set(&mut w, n, *v), "{}", n);
        }
        let max = p.symbols.iter().map(|s| s.mul).max().unwrap();
        assert_eq!(max, 4095);
    }

    #[test]
    fn phase_noise_between_table_rows() {
        let pn = phase_noise(&regs::starter(), &Settings::default());
        let at10k = pn.dbc_hz[20]; // 1e3 · 10^(20/20) = 10 kHz
        assert!(at10k < -145.0 && at10k > -150.0, "{}", at10k);
        assert!(pn.jitter_s > 0.0 && pn.jitter_s < 1e-12);
    }
}
