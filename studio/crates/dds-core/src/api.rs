//! Engine: single owner of the configuration, the hardware link and the bus
//! transaction log. The HTTP server exposes it as one JSON RPC
//! (`handle(cmd, args) -> json`); the egui app calls `handle` for mutations
//! and the typed methods (`state()`, `spectrum()` …) for reads.
//!
//! Adding a feature = a new `cmd` arm here + its types in `src/lib/types.ts`.

use crate::bus;
use crate::model::{self, Derived, FreqPlan, LfmPlan, LfmReq, ModPlan, ModReq, PhaseNoise, Response, Settings, Spectrum};
use crate::regs::{self, Words, FIELDS, NUM_REGS, REGS};
use crate::sim::{BoardState, SimStatus, VirtualChip};
use crate::transport::{self, FirmwareLink, Link, LinkInfo, PINS};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

const LOG_CAP: usize = 4000;

/// Chip and board names (`get_meta.chip`, the UIs' headers).
pub const CHIP_NAME: &str = "1508ПЛ8Т";
pub const CHIP_VENDOR: &str = "АО НПЦ «ЭЛВИС»";
pub const BOARD_NAME: &str = "DDS 48–52 МГц, ревизия F";

#[derive(Serialize, Clone, Debug)]
pub struct Txn {
    pub id: u64,
    pub t_ms: f64,
    /// "write" | "read" | "raw" | "pin" | "note" | "cmd"
    pub op: &'static str,
    pub reg: Option<usize>,
    /// Register address (also for command registers outside the modelled map).
    pub addr: Option<u16>,
    /// SPI frames, hex, space separated ("101400 20ABCD").
    pub tx: Option<String>,
    pub rx: Option<String>,
    pub value: Option<u32>,
    /// (field, old, new)
    pub changes: Vec<(String, u32, u32)>,
    pub note: String,
    pub ok: bool,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Persist {
    settings: Settings,
    regs: Vec<u32>,
    auto_apply: bool,
    port: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Readback {
    pub words: Vec<Option<u32>>,
    pub matches: Vec<Option<bool>>,
}

/// Typed snapshot of the engine. `get_state` serializes exactly this (the web
/// UI reads the JSON, the native egui UI reads the fields directly).
#[derive(Serialize, Clone, Debug)]
pub struct State {
    pub regs: Vec<u32>,
    pub desired: Vec<u32>,
    pub dev_regs: Option<Vec<u32>>,
    pub dirty: Vec<bool>,
    pub fields: BTreeMap<&'static str, u32>,
    pub derived: Derived,
    pub settings: Settings,
    pub link: LinkInfo,
    pub link_error: Option<String>,
    pub auto_apply: bool,
    pub pins: BTreeMap<String, bool>,
    pub inputs: BTreeMap<String, Option<bool>>,
    pub board: Option<BoardState>,
    pub log_head: u64,
    pub readback: Readback,
    pub sim: Option<SimStatus>,
    pub last_port: Option<String>,
    /// Profile the simulator plays right now (firmware hopping); `status` RPC only.
    #[serde(skip)]
    pub live_profile: Option<u32>,
}

/// One row of the channel profile table (`profiles` RPC).
#[derive(Serialize, Clone, Debug)]
pub struct Profile {
    pub y: usize,
    pub ftw: u64,
    pub f: f64,
    pub phase_deg: f64,
    pub mul: i64,
    pub offset: i64,
}

/// Stepped sweep grid and the level the model expects at each point (`sweep_plan` RPC).
#[derive(Serialize, Clone, Debug)]
pub struct SweepPlan {
    pub f: Vec<f64>,
    pub dbm: Vec<f64>,
}

impl Readback {
    fn empty() -> Self {
        Readback { words: vec![None; NUM_REGS], matches: vec![None; NUM_REGS] }
    }
}

pub struct Engine {
    /// Desired configuration (what the UI edits).
    regs: Words,
    /// What we believe the chip holds; `None` = unknown (fresh hardware link).
    dev: Option<Words>,
    settings: Settings,
    link: Box<dyn Link>,
    auto_apply: bool,
    pins: BTreeMap<String, bool>,
    log: VecDeque<Txn>,
    next_id: u64,
    t0: Instant,
    cfg_path: Option<PathBuf>,
    readback: Readback,
    link_error: Option<String>,
    board: Option<BoardState>,
    last_port: Option<String>,
}

fn arg<T: for<'de> Deserialize<'de>>(v: &Value, key: &str) -> Result<T> {
    serde_json::from_value(v.get(key).cloned().unwrap_or(Value::Null)).map_err(|e| Error::Invalid(format!("параметр '{}': {}", key, e)))
}

fn opt<T: for<'de> Deserialize<'de>>(v: &Value, key: &str) -> Option<T> {
    v.get(key).and_then(|x| serde_json::from_value(x.clone()).ok())
}

fn sanitized(w: &Words) -> Words {
    std::array::from_fn(|r| regs::sanitize(r, w[r]))
}

fn frames_hex(frames: &[[u8; 3]]) -> String {
    frames.iter().map(|f| bus::hex(f)).collect::<Vec<_>>().join(" ")
}

impl Engine {
    pub fn new(cfg_path: Option<PathBuf>) -> Self {
        let mut p = Persist { auto_apply: true, ..Default::default() };
        if let Some(path) = &cfg_path {
            if let Ok(txt) = std::fs::read_to_string(path) {
                if let Ok(x) = serde_json::from_str::<Persist>(&txt) {
                    p = x;
                }
            }
        }
        let mut regs = regs::starter();
        if p.regs.len() == NUM_REGS {
            for (i, w) in p.regs.iter().enumerate() {
                regs[i] = regs::sanitize(i, *w);
            }
        }
        // a persisted fault stimulus would greet the user with a dead board
        p.settings.sim_clock_ok = true;
        p.settings.sim_pgood = true;
        let sim = VirtualChip::new(p.settings.clone());
        let mut e = Engine {
            regs,
            dev: Some(sanitized(&regs::defaults())),
            settings: p.settings,
            link: Box::new(sim),
            auto_apply: p.auto_apply,
            pins: PINS.iter().filter(|p| p.dir == "out").map(|p| (p.name.to_string(), p.idle)).collect(),
            log: VecDeque::new(),
            next_id: 1,
            t0: Instant::now(),
            cfg_path,
            readback: Readback::empty(),
            link_error: None,
            board: None,
            last_port: p.port,
        };
        e.note("Запуск: подключена виртуальная плата (симулятор 1508ПЛ8Т)");
        if e.auto_apply {
            let _ = e.apply();
        }
        e
    }

    fn persist(&self) {
        if let Some(path) = &self.cfg_path {
            let p = Persist { settings: self.settings.clone(), regs: self.regs.to_vec(), auto_apply: self.auto_apply, port: self.last_port.clone() };
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(path, serde_json::to_string_pretty(&p).unwrap_or_default());
        }
    }

    // ───────────── log ─────────────

    fn push_log(&mut self, mut t: Txn) {
        t.id = self.next_id;
        self.next_id += 1;
        t.t_ms = self.t0.elapsed().as_secs_f64() * 1e3;
        self.log.push_back(t);
        while self.log.len() > LOG_CAP {
            self.log.pop_front();
        }
    }

    fn txn(op: &'static str, note: impl Into<String>) -> Txn {
        Txn { id: 0, t_ms: 0.0, op, reg: None, addr: None, tx: None, rx: None, value: None, changes: vec![], note: note.into(), ok: true }
    }

    fn note(&mut self, msg: impl Into<String>) {
        self.push_log(Self::txn("note", msg));
    }

    fn track<T>(&mut self, r: Result<T>) -> Result<T> {
        match &r {
            Ok(_) => self.link_error = None,
            Err(e) => self.link_error = Some(e.to_string()),
        }
        r
    }

    // ───────────── bus ─────────────

    /// Write any address (also command registers outside the map) and log it.
    fn write_addr(&mut self, addr: u16, value: u16, reg: Option<usize>, changes: Vec<(String, u32, u32)>, note: &str) -> Result<()> {
        let res = self.link.write_reg(addr, value);
        let res = self.track(res);
        let mut t = Self::txn(if reg.is_some() { "write" } else { "cmd" }, note);
        t.reg = reg;
        t.addr = Some(addr);
        t.tx = Some(frames_hex(&bus::write_frames(addr, value)));
        t.value = Some(value as u32);
        t.changes = changes;
        if let Err(e) = &res {
            t.ok = false;
            t.note = format!("{} — ОШИБКА: {}", note, e);
        }
        self.push_log(t);
        res
    }

    fn write_reg(&mut self, r: usize, note: &str) -> Result<()> {
        let value = regs::sanitize(r, self.regs[r]);
        let prev = self.dev.map(|d| d[r]);
        let mut changes = vec![];
        let (mut a, mut b) = ([0u32; NUM_REGS], [0u32; NUM_REGS]);
        a[r] = prev.unwrap_or(REGS[r].reset);
        b[r] = value;
        for f in regs::fields_of(r).filter(|f| f.access == regs::Access::Rw) {
            let (o, n) = (f.get(&a), f.get(&b));
            if o != n || prev.is_none() {
                changes.push((f.name.to_string(), o, n));
            }
        }
        self.write_addr(REGS[r].addr, value as u16, Some(r), changes, note)?;
        let mut d = self.dev.unwrap_or_else(|| sanitized(&regs::defaults()));
        d[r] = value;
        self.dev = Some(d);
        Ok(())
    }

    pub fn write_all(&mut self) -> Result<Vec<usize>> {
        let mut v = vec![];
        for r in regs::write_order() {
            self.write_reg(r, "полная запись конфигурации")?;
            v.push(r);
        }
        Ok(v)
    }

    /// Write only registers whose desired value differs from the chip.
    pub fn apply(&mut self) -> Result<Vec<usize>> {
        let Some(dev) = self.dev else {
            return self.write_all();
        };
        let desired = sanitized(&self.regs);
        let changed: Vec<usize> = regs::write_order().into_iter().filter(|&r| desired[r] != dev[r]).collect();
        for &r in &changed {
            self.write_reg(r, "изменение конфигурации")?;
        }
        Ok(changed)
    }

    fn after_change(&mut self) -> Result<()> {
        self.persist();
        if self.auto_apply {
            self.apply()?;
        }
        Ok(())
    }

    fn dirty(&self) -> Vec<bool> {
        let desired = sanitized(&self.regs);
        (0..NUM_REGS).map(|r| regs::writable(r) && self.dev.map(|d| d[r] != desired[r]).unwrap_or(true)).collect()
    }

    fn set_many(&mut self, values: &[(String, u32)]) -> Result<()> {
        let mut w = self.regs;
        for (k, v) in values {
            if !regs::set(&mut w, k, *v) {
                return Err(Error::Invalid(format!("поле {} не принимает значение {}", k, v)));
            }
        }
        self.regs = w;
        self.after_change()
    }

    // ───────────── readback ─────────────

    pub fn read_register(&mut self, r: usize) -> Result<u32> {
        let addr = REGS[r].addr;
        let res = self.link.read_reg(addr);
        let res = self.track(res);
        let mut t = Self::txn("read", format!("чтение {}", REGS[r].name));
        t.reg = Some(r);
        t.addr = Some(addr);
        t.tx = Some(frames_hex(&bus::read_frames(addr)));
        match &res {
            Ok(v) => {
                t.rx = Some(format!("00{:04X}", v));
                t.value = Some(*v as u32);
            }
            Err(e) => {
                t.ok = false;
                t.note = format!("{} — ОШИБКА: {}", t.note, e);
            }
        }
        self.push_log(t);
        let v = res? as u32;
        let expect = self.dev.map(|d| d[r]).unwrap_or(regs::sanitize(r, self.regs[r]));
        let mask = regs::defined_mask(r) & !regs::status_mask(r);
        self.readback.words[r] = Some(v);
        self.readback.matches[r] = Some(mask == 0 || (v & mask) == (expect & mask));
        Ok(v)
    }

    // ───────────── derived data ─────────────

    pub fn derived(&self) -> Derived {
        model::derive(&self.regs, &self.settings)
    }

    fn sim_status(&mut self) -> Option<crate::sim::SimStatus> {
        self.link.as_sim().map(|s| s.poll())
    }

    fn refresh_board(&mut self) {
        let b = self.link.board();
        match b {
            Ok(b) => {
                self.board = Some(b);
                self.link_error = None;
            }
            Err(e) => self.link_error = Some(e.to_string()),
        }
    }

    fn inputs(&self) -> BTreeMap<String, Option<bool>> {
        let mut m = BTreeMap::new();
        m.insert("PGOOD".to_string(), self.board.as_ref().map(|b| b.pgood));
        m
    }

    /// Typed snapshot (refreshes the simulator and board status first).
    pub fn state(&mut self) -> State {
        let derived = self.derived();
        let sim = self.sim_status();
        self.refresh_board();
        let live_profile = self.link.as_sim().map(|s| regs::get(&s.active(), "Pr_1"));
        State {
            regs: self.regs.to_vec(),
            desired: sanitized(&self.regs).to_vec(),
            dev_regs: self.dev.map(|d| d.to_vec()),
            dirty: self.dirty(),
            fields: regs::decode(&self.regs),
            derived,
            settings: self.settings.clone(),
            link: self.link.info(),
            link_error: self.link_error.clone(),
            auto_apply: self.auto_apply,
            pins: self.pins.clone(),
            inputs: self.inputs(),
            board: self.board.clone(),
            log_head: self.log_head(),
            readback: self.readback.clone(),
            sim,
            last_port: self.last_port.clone(),
            live_profile,
        }
    }

    fn snapshot(&mut self) -> Value {
        serde_json::to_value(self.state()).unwrap_or(Value::Null)
    }

    /// Id of the newest bus-log entry (0 = none yet).
    pub fn log_head(&self) -> u64 {
        self.next_id - 1
    }

    /// Bus-log entries newer than `id`, oldest first.
    pub fn log_since(&self, id: u64) -> impl Iterator<Item = &Txn> {
        self.log.iter().filter(move |t| t.id > id)
    }

    /// Drop the bus log (ids keep increasing).
    pub fn clear_log(&mut self) {
        self.log.clear();
    }

    fn status(&mut self) -> Value {
        let sim = self.sim_status();
        self.refresh_board();
        // the simulator may change SEL_REG by itself (firmware hopping)
        let live = self.link.as_sim().map(|s| regs::get(&s.active(), "Pr_1"));
        json!({ "sim": sim, "inputs": self.inputs(), "board": self.board, "log_head": self.log_head(), "link_error": self.link_error, "live_profile": live })
    }

    // ───────────── config files ─────────────

    fn export_text(&self) -> String {
        let d = self.derived();
        let mut s = format!("# DDS Studio — конфигурация 1508ПЛ8Т\n# {}\n", bus::describe());
        for m in d.metrics.iter().filter(|m| m.top) {
            s.push_str(&format!("# {} = {} {}\n", m.label, m.value, m.unit));
        }
        s.push_str("# адрес\tрегистр\tзначение (порядок записи)\n");
        for r in regs::write_order() {
            s.push_str(&format!("0x{:04X}\t{}\t0x{:04X}\n", REGS[r].addr, REGS[r].name, regs::sanitize(r, self.regs[r])));
        }
        s
    }

    fn export_c(&self) -> String {
        let mut s = String::from("/* DDS Studio: конфигурация 1508ПЛ8Т, порядок записи. Для pl8t_write(addr, value). */\n#include <stdint.h>\n\nstatic const uint16_t dds_config[][2] = {\n");
        for r in regs::write_order() {
            s.push_str(&format!("    {{ 0x{:04X}u, 0x{:04X}u }}, /* {} */\n", REGS[r].addr, regs::sanitize(r, self.regs[r]), REGS[r].name));
        }
        s.push_str("};\n");
        s
    }

    fn import_text(&mut self, text: &str) -> Result<usize> {
        let num = |t: &str| -> Option<u32> {
            let t = t.trim().trim_end_matches(',').trim_end_matches('u');
            match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                Some(h) => u32::from_str_radix(h, 16).ok(),
                None => t.parse().ok(),
            }
        };
        let mut n = 0;
        let mut w = self.regs;
        for line in text.lines().map(|l| l.split("/*").next().unwrap_or("").split("//").next().unwrap_or("").trim()).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let toks: Vec<&str> = line.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '=' || c == ':' || c == '{' || c == '}').filter(|t| !t.is_empty()).collect();
            if toks.len() < 2 {
                continue;
            }
            let reg = toks.iter().find_map(|t| regs::reg_by_name(t)).or_else(|| num(toks[0]).and_then(|a| regs::reg_by_addr(a as u16)));
            let val = num(toks[toks.len() - 1]);
            if let (Some(r), Some(v)) = (reg, val) {
                w[r] = regs::sanitize(r, v);
                n += 1;
            }
        }
        if n == 0 {
            return Err(Error::Invalid("не найдено ни одной строки «адрес/имя значение»".into()));
        }
        self.regs = w;
        Ok(n)
    }

    // ───────────── domain operations ─────────────

    /// Set the output frequency of channel `ch`. `seamless`: write the inactive
    /// profile of the pair 0/1 and switch SEL_REG (as the firmware does), so the
    /// output never shows a half-written FTW.
    fn set_frequency(&mut self, ch: u8, f: f64, seamless: bool) -> Result<Value> {
        let fclk = model::fclk_eff(&self.regs, &self.settings);
        if !(f > 0.0 && f < fclk / 2.0) {
            return Err(Error::Invalid(format!("частота должна быть в пределах 0…{:.3} МГц (Fclk/2)", fclk / 2e6)));
        }
        let ftw = model::ftw_for(f, fclk);
        let sel = if ch == 1 { "Pr_1" } else { "Pr_2" };
        let cur = regs::get(&self.regs, sel) as usize;
        let mut w = self.regs;
        let target = if seamless {
            let next = if cur == 0 { 1 } else { 0 };
            for n in ["P", "Mul", "Offset"] {
                let v = regs::get(&w, &format!("CH{}_{}{}", ch, n, cur.min(regs::n_prof(ch) - 1)));
                regs::set(&mut w, &format!("CH{}_{}{}", ch, n, next), v);
            }
            regs::set(&mut w, sel, next as u32);
            next
        } else {
            cur.min(regs::n_prof(ch) - 1)
        };
        regs::set_ftw(&mut w, ch, target, ftw);
        regs::set(&mut w, &format!("CH{}_LS_on", ch), 0);
        self.regs = w;
        self.note(format!("Частота К{} → {:.6} МГц (профиль {}{})", ch, model::f_of_ftw(ftw, fclk) / 1e6, target, if seamless { ", бесшовно" } else { "" }));
        self.after_change()?;
        Ok(self.snapshot())
    }

    // ───────────── typed analysis (read-only) ─────────────

    fn main_amp(&self) -> f64 {
        let c = model::calc(&self.regs, &self.settings);
        if c.ch1.on_dac1 {
            c.ch1.amp
        } else {
            c.ch2.amp
        }
    }

    /// FTW, grid error, DIP code and expected level for `f`.
    pub fn plan_frequency(&self, f: f64) -> FreqPlan {
        model::plan_frequency(f, &self.regs, &self.settings, self.main_amp())
    }

    pub fn profiles(&self, ch: u8) -> Vec<Profile> {
        let fclk = model::fclk_eff(&self.regs, &self.settings);
        (0..regs::n_prof(ch))
            .map(|y| {
                let g = |n: &str| regs::get_signed(&self.regs, &format!("CH{}_{}{}", ch, n, y));
                let ftw = regs::ftw(&self.regs, ch, y);
                Profile { y, ftw, f: model::f_of_ftw(ftw, fclk), phase_deg: model::phase_deg(g("P")), mul: g("Mul"), offset: g("Offset") }
            })
            .collect()
    }

    pub fn response(&self, lo: f64, hi: f64, n: usize) -> Response {
        model::response(&self.regs, &self.settings, lo, hi, n)
    }

    pub fn spectrum(&self, lo: f64, hi: f64, log2n: u32, points: usize) -> Result<Spectrum> {
        if !(hi > lo) {
            return Err(Error::Invalid("верхняя граница должна быть больше нижней".into()));
        }
        Ok(model::spectrum(&self.regs, &self.settings, lo, hi, log2n, points))
    }

    pub fn phase_noise(&self) -> PhaseNoise {
        model::phase_noise(&self.regs, &self.settings)
    }

    /// Stepped sweep: frequencies and the level the model expects at each.
    pub fn sweep_plan(&self, lo: f64, hi: f64, step: f64) -> Result<SweepPlan> {
        if !(step > 0.0) || !(hi >= lo) || (hi - lo) / step > 5000.0 {
            return Err(Error::Invalid("шаг > 0 и не больше 5000 точек".into()));
        }
        let c = model::calc(&self.regs, &self.settings);
        let amp = self.main_amp();
        let n = ((hi - lo) / step).floor() as usize + 1;
        let f: Vec<f64> = (0..n).map(|i| lo + step * i as f64).collect();
        let dbm = f.iter().map(|&x| model::tone_dbm(amp, model::f_of_ftw(model::ftw_for(x, c.fclk), c.fclk), c.fclk, &self.settings)).collect();
        Ok(SweepPlan { f, dbm })
    }

    pub fn lfm_design(&self, req: &LfmReq) -> LfmPlan {
        model::lfm_design(req, &self.regs, &self.settings)
    }

    pub fn mod_design(&self, req: &ModReq) -> ModPlan {
        model::modulation(req, &self.regs, &self.settings)
    }

    fn set_profile(&mut self, a: &Value) -> Result<Value> {
        let ch: u8 = opt(a, "ch").unwrap_or(1);
        let y: usize = arg(a, "y")?;
        if y >= regs::n_prof(ch) {
            return Err(Error::Invalid(format!("профиль 0…{}", regs::n_prof(ch) - 1)));
        }
        let fclk = model::fclk_eff(&self.regs, &self.settings);
        let mut w = self.regs;
        if let Some(f) = opt::<f64>(a, "f") {
            if !(f >= 0.0 && f < fclk / 2.0) {
                return Err(Error::Invalid("частота вне 0…Fclk/2".into()));
            }
            regs::set_ftw(&mut w, ch, y, model::ftw_for(f, fclk));
        }
        if let Some(p) = opt::<f64>(a, "phase_deg") {
            regs::set_signed(&mut w, &format!("CH{}_P{}", ch, y), model::phase_word(p));
        }
        if let Some(m) = opt::<i64>(a, "mul") {
            regs::set_signed(&mut w, &format!("CH{}_Mul{}", ch, y), m.clamp(-4096, 4095));
        }
        if let Some(o) = opt::<i64>(a, "offset") {
            regs::set_signed(&mut w, &format!("CH{}_Offset{}", ch, y), o);
        }
        self.regs = w;
        self.after_change()?;
        Ok(self.snapshot())
    }

    fn clr(&mut self, bits: u16, note: &str) -> Result<()> {
        self.write_addr(regs::A_CLR, bits, None, vec![], note)
    }

    fn connect(&mut self, a: &Value) -> Result<Value> {
        let kind: String = arg(a, "kind")?;
        let _ = self.link.hop(0, 0);
        match kind.as_str() {
            "sim" => {
                let _ = self.link.release();
                self.link = Box::new(VirtualChip::new(self.settings.clone()));
                self.dev = Some(sanitized(&regs::defaults()));
                self.note("Подключена виртуальная плата");
            }
            "serial" => {
                let path: String = arg(a, "path")?;
                let baud: u32 = opt(a, "baud").unwrap_or(transport::DEFAULT_BAUD);
                let b = FirmwareLink::open(&path, baud)?;
                let detail = b.info().detail;
                let _ = self.link.release();
                self.link = Box::new(b);
                self.dev = None; // unknown until the first full write
                self.last_port = Some(path.clone());
                self.persist();
                self.note(format!("Подключена плата {} ({}), режим REMOTE", path, detail));
            }
            k => return Err(Error::Invalid(format!("неизвестный тип подключения {}", k))),
        }
        self.link_error = None;
        self.readback = Readback::empty();
        for (name, high) in self.pins.clone() {
            let _ = self.link.set_pin(&name, high);
        }
        if self.auto_apply {
            self.apply()?;
        }
        Ok(self.snapshot())
    }

    // ───────────── RPC ─────────────

    pub fn handle(&mut self, cmd: &str, args: Value) -> Result<Value> {
        let a = &args;
        match cmd {
            "get_state" => Ok(self.snapshot()),
            "get_meta" => Ok(json!({
                "regs": &*REGS, "fields": &*FIELDS, "defaults": regs::defaults().to_vec(), "pins": PINS,
                "version": env!("CARGO_PKG_VERSION"),
                "chip": { "name": CHIP_NAME, "vendor": CHIP_VENDOR, "board": BOARD_NAME,
                          "bus": bus::describe(), "data_bits": regs::DATA_BITS, "can_read": bus::can_read(),
                          "commit_reg": regs::COMMIT_REG, "n_prof": [regs::N_PROF1, regs::N_PROF2] },
                "chain": model::OUT_CHAIN.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            })),
            "status" => Ok(self.status()),

            "set_field" => {
                let name: String = arg(a, "name")?;
                let value: u32 = arg(a, "value")?;
                self.set_many(&[(name, value)])?;
                Ok(self.snapshot())
            }
            "set_fields" => {
                let values: BTreeMap<String, u32> = arg(a, "values")?;
                let v: Vec<(String, u32)> = values.into_iter().collect();
                self.set_many(&v)?;
                Ok(self.snapshot())
            }
            "set_register" => {
                let i: usize = arg(a, "index")?;
                let v: u32 = arg(a, "value")?;
                if i >= NUM_REGS {
                    return Err(Error::Invalid(format!("индекс регистра 0…{}", NUM_REGS - 1)));
                }
                self.regs[i] = regs::sanitize(i, v);
                self.after_change()?;
                Ok(self.snapshot())
            }
            "reset_defaults" => {
                let starter = opt::<bool>(a, "starter").unwrap_or(false);
                self.regs = if starter { regs::starter() } else { regs::defaults() };
                self.after_change()?;
                Ok(self.snapshot())
            }
            "set_auto_apply" => {
                self.auto_apply = arg(a, "on")?;
                self.persist();
                if self.auto_apply {
                    self.apply()?;
                }
                Ok(self.snapshot())
            }
            "apply" => {
                if self.apply()?.is_empty() {
                    self.note("Нет изменений для записи");
                }
                Ok(self.snapshot())
            }
            "write_all" => {
                self.write_all()?;
                Ok(self.snapshot())
            }
            "write_register" => {
                let i: usize = arg(a, "index")?;
                if i >= NUM_REGS || !regs::writable(i) {
                    return Err(Error::Invalid("регистр не записывается".into()));
                }
                self.write_reg(i, "запись по запросу")?;
                Ok(self.snapshot())
            }
            "read_register" => {
                let i: usize = arg(a, "index")?;
                if i >= NUM_REGS {
                    return Err(Error::Invalid("нет такого регистра".into()));
                }
                let v = self.read_register(i)?;
                Ok(json!({ "value": v, "state": self.snapshot() }))
            }
            "read_all" => {
                let only: Option<String> = opt(a, "group");
                for r in 0..NUM_REGS {
                    if only.as_deref().map(|g| REGS[r].group == g).unwrap_or(true) {
                        self.read_register(r)?;
                    }
                }
                Ok(self.snapshot())
            }
            "raw" => {
                // {"hex": "B00001"} → one 24-bit SPI frame
                let hex: String = opt(a, "hex").unwrap_or_default();
                let bytes = bus::unhex(&hex).ok_or_else(|| Error::Invalid("ожидаются 3 hex-байта, например B0 00 01".into()))?;
                if bytes.len() != 3 {
                    return Err(Error::Invalid("кадр 1508ПЛ8Т — ровно 24 бита (3 байта)".into()));
                }
                let res = self.link.spi(&bytes);
                let res = self.track(res);
                let mut t = Self::txn("raw", format!("кадр {}", bus::describe_frame(&bytes)));
                t.tx = Some(bus::hex(&bytes));
                t.rx = res.as_ref().ok().map(|x| bus::hex(x));
                t.ok = res.is_ok();
                self.push_log(t);
                let rx = res?;
                if matches!(bus::decode(&bytes), Some((bus::Cmd::Wr | bus::Cmd::Wri, _))) {
                    self.dev = None; // the chip state is unknown after a raw write
                }
                Ok(json!({ "rx": bus::hex(&rx), "data": bus::reply_data(&rx), "state": self.snapshot() }))
            }
            "set_settings" => {
                let patch: serde_json::Map<String, Value> = arg(a, "patch")?;
                let mut v = serde_json::to_value(&self.settings)?;
                for (k, x) in patch {
                    if v.get(&k).is_none() {
                        return Err(Error::Invalid(format!("неизвестная настройка {}", k)));
                    }
                    v[k] = x;
                }
                self.settings = serde_json::from_value(v)?;
                let env = self.settings.clone();
                if let Some(sim) = self.link.as_sim() {
                    sim.set_env(env);
                }
                self.persist();
                Ok(self.snapshot())
            }
            "set_sim_dip" => {
                let n: u16 = arg(a, "value")?;
                match self.link.as_sim() {
                    Some(sim) => sim.dip = n & 0xFFF,
                    None => return Err(Error::Invalid("DIP-переключатели реальной платы переключаются руками".into())),
                }
                Ok(self.snapshot())
            }
            "set_pin" => {
                let name: String = arg(a, "name")?;
                let high: bool = arg(a, "high")?;
                if !PINS.iter().any(|p| p.name == name && p.dir == "out") {
                    return Err(Error::Invalid(format!("нет управляемого вывода {}", name)));
                }
                let res = self.link.set_pin(&name, high);
                let res = self.track(res);
                let mut t = Self::txn("pin", format!("{} = {}", name, high as u8));
                t.ok = res.is_ok();
                self.push_log(t);
                res?;
                self.pins.insert(name.clone(), high);
                if name == "RSTn" && !high {
                    self.dev = Some(sanitized(&regs::defaults()));
                }
                Ok(self.snapshot())
            }
            "chip_reset" => {
                // hardware: RSTn pulse through the firmware; software: SWRST = 0x0078
                let soft = opt::<bool>(a, "soft").unwrap_or(false);
                if soft {
                    self.write_addr(regs::A_SWRST, regs::SWRST_KEY, None, vec![], "SWRST = 0x0078: программный сброс")?;
                } else {
                    let r = self.link.set_pin("RSTn", false).and_then(|_| {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        self.link.set_pin("RSTn", true)
                    });
                    self.track(r)?;
                    self.note("Импульс RSTn");
                }
                self.dev = Some(sanitized(&regs::defaults()));
                self.readback = Readback::empty();
                self.note("Сброс: регистры вернулись к значениям табл. 5.1");
                if self.auto_apply {
                    self.apply()?;
                }
                Ok(self.snapshot())
            }
            "check_link" => {
                // DEVID through the real frames: the first thing to try on a new board
                let r = regs::R_DEVID;
                let v = self.read_register(r)?;
                let ok = v == regs::DEVID_VALUE as u32;
                self.note(if ok { "DEVID = 0x0201: связь с DDS есть".to_string() } else { format!("DEVID = 0x{:04X} вместо 0x0201: нет такта, питания или связи по SPI", v) });
                Ok(json!({ "ok": ok, "value": v, "state": self.snapshot() }))
            }

            // ── frequency, profiles, hopping ──
            "plan_frequency" => {
                let f: f64 = arg(a, "f")?;
                Ok(serde_json::to_value(self.plan_frequency(f))?)
            }
            "set_frequency" => {
                let f: f64 = arg(a, "f")?;
                let ch: u8 = opt(a, "ch").unwrap_or(1);
                let seamless: bool = opt(a, "seamless").unwrap_or(true);
                self.set_frequency(ch, f, seamless)
            }
            "profiles" => Ok(serde_json::to_value(self.profiles(opt(a, "ch").unwrap_or(1)))?),
            "set_profile" => self.set_profile(a),
            "select_profile" => {
                let ch: u8 = opt(a, "ch").unwrap_or(1);
                let y: u32 = arg(a, "y")?;
                self.set_many(&[(if ch == 1 { "Pr_1" } else { "Pr_2" }.to_string(), y)])?;
                Ok(self.snapshot())
            }
            "hop" => {
                let n: u8 = arg(a, "n")?;
                let ms: u32 = opt(a, "ms").unwrap_or(100);
                if n > regs::N_PROF1 as u8 {
                    return Err(Error::Invalid(format!("не больше {} профилей", regs::N_PROF1)));
                }
                if n >= 2 {
                    self.apply()?;
                }
                let r = self.link.hop(n, ms);
                self.track(r)?;
                self.note(if n >= 2 { format!("Прошивка перебирает профили 0…{} по {} мс", n - 1, ms) } else { "Перебор профилей остановлен".to_string() });
                if n < 2 {
                    // the firmware leaves SEL_REG wherever it stopped: write ours again
                    self.write_reg(regs::R_SEL, "профиль после перебора")?;
                }
                Ok(self.snapshot())
            }
            "release" => {
                let r = self.link.release();
                self.track(r)?;
                self.dev = None;
                self.note("Плата возвращена прошивке: частота задаётся DIP-переключателями и кнопкой S4");
                Ok(self.snapshot())
            }

            // ── analysis ──
            "response" => {
                let lo: f64 = opt(a, "lo").unwrap_or(1e6);
                let hi: f64 = opt(a, "hi").unwrap_or(500e6);
                let n: usize = opt(a, "n").unwrap_or(600);
                Ok(serde_json::to_value(self.response(lo, hi, n))?)
            }
            "spectrum" => {
                let lo: f64 = opt(a, "lo").unwrap_or(0.0);
                let hi: f64 = opt(a, "hi").unwrap_or(1000e6);
                let log2n: u32 = opt(a, "log2n").unwrap_or(14);
                let points: usize = opt(a, "points").unwrap_or(1600);
                Ok(serde_json::to_value(self.spectrum(lo, hi, log2n, points)?)?)
            }
            "phase_noise" => Ok(serde_json::to_value(self.phase_noise())?),
            "sweep_plan" => {
                // stepped sweep: frequencies and the level the model expects at each
                let lo: f64 = arg(a, "lo")?;
                let hi: f64 = arg(a, "hi")?;
                let step: f64 = arg(a, "step")?;
                Ok(serde_json::to_value(self.sweep_plan(lo, hi, step)?)?)
            }
            "lfm_design" => {
                let req: LfmReq = opt(a, "req").unwrap_or_default();
                Ok(serde_json::to_value(self.lfm_design(&req))?)
            }
            "lfm_apply" => {
                let req: LfmReq = opt(a, "req").unwrap_or_default();
                let plan = model::lfm_design(&req, &self.regs, &self.settings);
                self.set_many(&plan.fields)?;
                self.note(format!("ЛЧМ записан: {:.3} → {:.3} МГц за {:.3} мс", plan.f_start / 1e6, plan.f_end / 1e6, plan.t_ramp * 1e3));
                Ok(self.snapshot())
            }
            "lfm_start" => {
                self.clr(1 << 4, "CLR.LS1_start: запуск ЛЧМ канала 1")?;
                Ok(self.snapshot())
            }
            "lfm_stop" => {
                self.clr(1 << 6, "CLR.LS1_stop: переход к стадии 4")?;
                Ok(self.snapshot())
            }
            "lfm_off" => {
                self.set_many(&[("CH1_LS_on".into(), 0)])?;
                Ok(self.snapshot())
            }
            "clear_phase" => {
                self.clr(0b11, "CLR.Clr_ph1/2: обнуление аккумуляторов фазы")?;
                Ok(self.snapshot())
            }
            "mod_design" => {
                let req: ModReq = opt(a, "req").unwrap_or_default();
                Ok(serde_json::to_value(self.mod_design(&req))?)
            }
            "mod_apply" => {
                let req: ModReq = opt(a, "req").unwrap_or_default();
                let plan = model::modulation(&req, &self.regs, &self.settings);
                self.set_many(&plan.fields)?;
                self.note(format!("{}: профили 0…{} записаны", plan.title, plan.symbols.len() - 1));
                Ok(json!({ "plan": plan, "state": self.snapshot() }))
            }

            // ── log, ports, files ──
            "log" => {
                let since: u64 = opt(a, "since").unwrap_or(0);
                let entries: Vec<&Txn> = self.log_since(since).collect();
                Ok(json!({ "entries": entries, "head": self.log_head() }))
            }
            "clear_log" => {
                self.clear_log();
                Ok(json!({ "head": self.log_head() }))
            }
            "list_ports" => Ok(serde_json::to_value(transport::list_ports())?),
            "scan_usb" => Ok(serde_json::to_value(transport::scan_usb()?)?),
            "connect" => self.connect(a),
            "export_config" => {
                let fmt: String = opt(a, "format").unwrap_or_else(|| "text".into());
                Ok(json!({ "text": if fmt == "c" { self.export_c() } else { self.export_text() } }))
            }
            "import_config" => {
                let text: String = arg(a, "text")?;
                let n = self.import_text(&text)?;
                self.note(format!("Импорт: {} регистров", n));
                self.after_change()?;
                Ok(self.snapshot())
            }
            _ => Err(Error::UnknownCommand(cmd.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_round_trip() {
        let mut e = Engine::new(None);
        let meta = e.handle("get_meta", json!({})).unwrap();
        assert_eq!(meta["fields"].as_array().unwrap().len(), FIELDS.len());
        let s = e.handle("set_field", json!({"name": "sum", "value": 3})).unwrap();
        assert_eq!(s["fields"]["sum"], 3);
        assert!(s["dirty"].as_array().unwrap().iter().all(|d| d == false), "auto-apply wrote everything");
        assert!(e.handle("set_field", json!({"name": "DEVID", "value": 1})).is_err(), "read-only field");
        assert!(e.handle("nope", json!({})).is_err());
    }

    #[test]
    fn readback_matches_after_write() {
        let mut e = Engine::new(None);
        e.handle("write_all", json!({})).unwrap();
        let s = e.handle("read_all", json!({})).unwrap();
        for r in regs::write_order() {
            assert_eq!(s["readback"]["matches"][r], true, "{}", REGS[r].name);
        }
        let c = e.handle("check_link", json!({})).unwrap();
        assert_eq!(c["ok"], true);
    }

    #[test]
    fn seamless_frequency_uses_profile_pair() {
        let mut e = Engine::new(None);
        let s = e.handle("set_frequency", json!({"f": 49.5e6})).unwrap();
        assert_eq!(s["fields"]["Pr_1"], 1);
        assert!((s["derived"]["calc"]["ch1"]["fout"].as_f64().unwrap() - 49.5e6).abs() < 1e-3);
        let s = e.handle("set_frequency", json!({"f": 51e6})).unwrap();
        assert_eq!(s["fields"]["Pr_1"], 0);
        // the log shows SEL_REG written after the profile words
        let log = e.handle("log", json!({})).unwrap();
        let names: Vec<String> = log["entries"].as_array().unwrap().iter().filter_map(|t| t["reg"].as_u64()).map(|r| REGS[r as usize].name.to_string()).collect();
        let sel = names.iter().rposition(|n| n == "SEL_REG").unwrap();
        let dph = names.iter().rposition(|n| n.starts_with("CH1_dPh0")).unwrap();
        assert!(dph < sel);
    }

    #[test]
    fn export_import_round_trip() {
        let mut e = Engine::new(None);
        e.handle("set_field", json!({"name": "pdith1", "value": 1})).unwrap();
        let text = e.handle("export_config", json!({})).unwrap()["text"].as_str().unwrap().to_string();
        e.handle("reset_defaults", json!({})).unwrap();
        let s = e.handle("import_config", json!({ "text": text })).unwrap();
        assert_eq!(s["fields"]["pdith1"], 1);
        let c = e.handle("export_config", json!({"format": "c"})).unwrap();
        assert!(c["text"].as_str().unwrap().contains("0x0003u"));
    }

    #[test]
    fn lfm_and_modulation_rpcs() {
        let mut e = Engine::new(None);
        let s = e.handle("lfm_apply", json!({"req": {"f_start": 48e6, "f_stop": 52e6, "t_ramp": 1e-3, "t_hold": 1e-4, "shape": "tri"}})).unwrap();
        assert_eq!(s["fields"]["CH1_LS_on"], 1);
        e.handle("lfm_start", json!({})).unwrap();
        let r = e.handle("mod_apply", json!({"req": {"kind": "bpsk", "f_center": 50e6}})).unwrap();
        assert_eq!(r["state"]["fields"]["CH1_LS_on"], 0);
        assert_eq!(r["plan"]["symbols"].as_array().unwrap().len(), 2);
        e.handle("hop", json!({"n": 2, "ms": 50})).unwrap();
        e.handle("hop", json!({"n": 0})).unwrap();
    }

    #[test]
    fn no_clock_breaks_devid() {
        let mut e = Engine::new(None);
        e.handle("set_settings", json!({"patch": {"sim_clock_ok": false}})).unwrap();
        let c = e.handle("check_link", json!({})).unwrap();
        assert_eq!(c["ok"], false);
        assert!(e.handle("set_settings", json!({"patch": {"bogus": 1}})).is_err());
    }

    #[test]
    fn spectrum_rpc() {
        let mut e = Engine::new(None);
        let sp = e.handle("spectrum", json!({"lo": 0, "hi": 600e6, "log2n": 12, "points": 400})).unwrap();
        assert_eq!(sp["f"].as_array().unwrap().len(), 400);
        let r = e.handle("response", json!({})).unwrap();
        assert!(r["fc_3db"].as_f64().unwrap() > 55e6 && r["fc_3db"].as_f64().unwrap() < 90e6);
    }

    #[test]
    fn typed_state_matches_get_state_json() {
        let mut e = Engine::new(None);
        let typed = e.state();
        let v = e.handle("get_state", json!({})).unwrap();
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(|k| k.as_str()).collect();
        assert_eq!(
            keys,
            [
                "auto_apply", "board", "derived", "desired", "dev_regs", "dirty", "fields", "inputs", "last_port",
                "link", "link_error", "log_head", "pins", "readback", "regs", "settings", "sim"
            ],
            "the web UI reads exactly these keys"
        );
        assert_eq!(v["regs"].as_array().unwrap().len(), typed.regs.len());
        assert_eq!(v["fields"]["Pr_1"], typed.fields["Pr_1"]);
        assert_eq!(typed.live_profile, Some(typed.fields["Pr_1"]));
    }

    #[test]
    fn log_stream_and_typed_helpers() {
        let mut e = Engine::new(None);
        let head = e.log_head();
        assert!(head > 0, "startup writes are logged");
        e.handle("set_field", json!({"name": "pdith1", "value": 1})).unwrap();
        let new: Vec<&Txn> = e.log_since(head).collect();
        assert!(!new.is_empty() && new.iter().all(|t| t.id > head));
        assert_eq!(e.log_head(), new.last().unwrap().id);
        e.clear_log();
        assert_eq!(e.log_since(0).count(), 0);
        assert_eq!(e.profiles(1).len(), regs::N_PROF1);
        assert_eq!(e.profiles(2).len(), regs::N_PROF2);
        assert!(e.spectrum(10.0, 5.0, 12, 100).is_err());
        assert!(e.sweep_plan(48e6, 52e6, 0.0).is_err());
        let sw = e.sweep_plan(48e6, 52e6, 1e6).unwrap();
        assert_eq!(sw.f.len(), 5);
        assert_eq!(e.plan_frequency(50e6).dip_n, Some(2000));
        // JSON of the typed helpers is what the RPC returns
        let rows = e.handle("profiles", json!({"ch": 1})).unwrap();
        assert_eq!(rows[0]["f"].as_f64().unwrap(), e.profiles(1)[0].f);
    }
}
