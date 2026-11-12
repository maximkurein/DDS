//! Application shell: routes views, runs actions, talks to the engine thread.
//!
//! Views never touch the Engine. They read `Shared` (the latest typed `State`
//! published by the engine thread, see `worker.rs`) and queue `Act`s; the
//! shell turns them into `Job`s after the frame, so USB-UART I/O never blocks
//! drawing. Mutations go through `Engine::handle`, the same RPC the Tauri app
//! and `dds-server` use, so every rule of the core is enforced in one place.

use crate::fmt::{eng, eng4, hex_w};
use crate::theme::{self, Pal};
use crate::views;
use crate::widgets as w;
use crate::worker::{Data, Job, Query, Reply, Worker};
use dds_core::api::{State, Txn, BOARD_NAME, CHIP_NAME, CHIP_VENDOR};
use dds_core::regs::{self, REGS};
use dds_core::Engine;
use eframe::egui::{self, Align, Color32, Key, Layout, RichText};
use egui_extras::{Column, TableBuilder};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum View {
    Overview,
    Board,
    Registers,
    Freq,
    Spectrum,
    Chain,
    Sweep,
    Hop,
    Mod,
    Hardware,
    Learn,
    Eco,
}

/// Views in sidebar order (⌘/Ctrl + 1…9 for the first nine): (view, icon, label, hint, group).
/// Same order and texts as `src/components/Sidebar.tsx`.
pub const NAV: [(View, &str, &str, &str, &str); 12] = [
    (View::Overview, "🔲", "Обзор", "Блок-схема кристалла", "Устройство"),
    (View::Board, "📟", "Плата", "Разъёмы, переключатели, тракт", "Устройство"),
    (View::Registers, "🔢", "Регистры", "Биты, запись, чтение", "Устройство"),
    (View::Freq, "📻", "Частота", "FTW, DIP-код, уровень", "Синтез"),
    (View::Spectrum, "📊", "Спектр", "Образы, гармоники, SFDR", "Синтез"),
    (View::Chain, "📉", "Тракт и шум", "ФНЧ, sin(x)/x, фазовый шум", "Синтез"),
    (View::Sweep, "➡", "Свип", "Перестройка по сетке", "Опыты"),
    (View::Hop, "🔀", "Перебор", "Профили, прыжки частоты", "Опыты"),
    (View::Mod, "〰", "Модуляция", "ЛЧМ, ЧМн, ФМн, КАМ", "Опыты"),
    (View::Hardware, "🔌", "Подключение", "USB-UART, симулятор", "Практика"),
    (View::Learn, "🎓", "Академия", "Как работает DDS", "Практика"),
    (View::Eco, "🔧", "Ревизии платы", "Исправления и ворота KiCad", "Практика"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Ok,
    Err,
    Info,
}

/// Something a view asks the shell to do after the frame.
pub enum Act {
    Rpc(&'static str, Value, Option<String>),
    Batch(Vec<(&'static str, Value)>, Option<String>),
    Query(&'static str, u64, Query),
    Go(View, Option<Option<&'static str>>),
    SelectBlock(Option<&'static str>),
    Focus(Option<&'static str>),
    Toast(Tone, String),
    Export,
    Import,
    Raw(String),
    CheckLink,
    RefreshPorts,
    ClearLog,
    Console(bool),
    Theme(bool),
}

/// What every view may read, plus the action queue.
pub struct Shared {
    pub st: State,
    pub log: Vec<Txn>,
    pub pal: Pal,
    pub view: View,
    pub block: Option<&'static str>,
    pub focus: Option<&'static str>,
    pub console_open: bool,
    /// Wall-clock seconds since start (timers, animations).
    pub time: f64,
    /// The engine thread has queued or running jobs (bus I/O in progress).
    pub busy: bool,
    /// Busy for longer than a moment — show the indicator.
    pub busy_shown: bool,
    /// Errors reported by the engine so far (a running sweep watches it).
    pub errors: u64,
    /// Hash of the configuration (registers + board settings) for query keys.
    pub cfg: u64,
    /// Latest answer per query slot: (key it was computed for, result).
    data: HashMap<&'static str, (u64, Result<Data, String>)>,
    /// Key last requested per slot (requests are sent once per key).
    asked: RefCell<HashMap<&'static str, u64>>,
    pub ports: Vec<dds_core::transport::PortInfo>,
    pub usb: Option<Vec<dds_core::transport::UsbDev>>,
    acts: RefCell<Vec<Act>>,
}

/// Stable hash for query keys (floats by their bits).
pub fn key(parts: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

fn cfg_key(st: &State) -> u64 {
    key((&st.regs, format!("{:?}", st.settings)))
}

impl Shared {
    pub fn act(&self, a: Act) {
        self.acts.borrow_mut().push(a);
    }
    pub fn rpc(&self, cmd: &'static str, args: Value) {
        self.act(Act::Rpc(cmd, args, None));
    }
    pub fn rpc_ok(&self, cmd: &'static str, args: Value, ok: &str) {
        self.act(Act::Rpc(cmd, args, Some(ok.into())));
    }
    pub fn set_field(&self, name: &str, value: u32) {
        self.rpc("set_field", json!({ "name": name, "value": value }));
    }
    pub fn set_settings(&self, patch: Value) {
        self.rpc("set_settings", json!({ "patch": patch }));
    }
    pub fn go(&self, v: View) {
        self.act(Act::Go(v, None));
    }
    pub fn go_block(&self, v: View, b: Option<&'static str>) {
        self.act(Act::Go(v, Some(b)));
    }
    pub fn toast(&self, t: Tone, s: impl Into<String>) {
        self.act(Act::Toast(t, s.into()));
    }
    pub fn field(&self, name: &str) -> u32 {
        self.st.fields.get(name).copied().unwrap_or(0)
    }
    pub fn is_sim(&self) -> bool {
        self.st.link.kind == "sim"
    }

    /// Ask the engine for `q` under `slot` (once per `k`) and return the latest
    /// answer — possibly for an older key while the new one is computed, like
    /// the web UI's `useRpcData`.
    pub fn query(&self, slot: &'static str, k: u64, q: Query) -> Option<&Data> {
        let mut asked = self.asked.borrow_mut();
        if asked.get(slot) != Some(&k) {
            asked.insert(slot, k);
            self.act(Act::Query(slot, k, q));
        }
        self.data.get(slot).and_then(|(_, r)| r.as_ref().ok())
    }
    /// Error of the newest answer in `slot`, if it failed.
    pub fn query_err(&self, slot: &'static str) -> Option<&str> {
        self.data.get(slot).and_then(|(_, r)| r.as_ref().err()).map(String::as_str)
    }
    /// The answer for the last requested key has not arrived yet.
    pub fn query_busy(&self, slot: &'static str) -> bool {
        let asked = self.asked.borrow().get(slot).copied();
        asked.is_some() && self.data.get(slot).map(|d| d.0) != asked
    }
}

/// Per-view UI state that outlives a frame.
#[derive(Default)]
pub struct ViewState {
    pub board: views::board::BoardState,
    pub registers: views::registers::RegState,
    pub freq: views::freq::FreqState,
    pub spectrum: views::spectrum::SpecState,
    pub sweep: views::sweep::SweepState,
    pub hop: views::hop::HopState,
    pub modulation: views::modulation::ModState,
    pub hardware: views::hardware::HwState,
    pub learn: usize,
    pub eco: crate::eco::EcoState,
    pub console: ConsoleState,
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum LogFilter {
    #[default]
    All,
    Write,
    Read,
    Note,
}

#[derive(Default)]
pub struct ConsoleState {
    pub filter: LogFilter,
    pub q: String,
    pub raw: String,
}

struct Toast {
    tone: Tone,
    text: String,
    until: Instant,
}

pub struct App {
    worker: Worker,
    sh: Shared,
    vs: ViewState,
    toasts: Vec<Toast>,
    busy_since: Option<Instant>,
    t0: Instant,
    shots: Option<crate::shots::Shots>,
}

/// Same file as the Tauri app (`app_config_dir()/dds-studio.json`, identifier
/// `com.dds-studio.desktop` from tauri.conf.json), so both front-ends share the
/// board settings, the last configuration and the last serial port.
pub fn config_path() -> Option<PathBuf> {
    const ID: &str = "com.dds-studio.desktop";
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let dir = if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library/Application Support").join(ID))
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(ID))
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or(home.map(|h| h.join(".config"))).map(|c| c.join(ID))
    };
    dir.map(|d| d.join("dds-studio.json"))
}

/// Developer from `config/version.json` (single source of the version info).
pub fn developer() -> String {
    const VERSION_JSON: &str = include_str!("../../../config/version.json");
    serde_json::from_str::<Value>(VERSION_JSON)
        .ok()
        .and_then(|v| v["app_info"]["developer"].as_str().map(str::to_string))
        .unwrap_or_default()
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        let shots = crate::shots::Shots::from_env();
        let get = |k: &str| cc.storage.and_then(|s| s.get_string(k));
        let dark = get("theme").map(|t| t != "light").unwrap_or(true);
        let console_open = get("console").map(|c| c == "1").unwrap_or(false);
        let pal = Pal::get(dark);
        theme::apply(&cc.egui_ctx, &pal);
        // the screenshot tour must not overwrite the user's settings file
        let cfg = if shots.is_some() { None } else { config_path() };
        let (worker, st, log) = Worker::spawn(Engine::new(cfg), cc.egui_ctx.clone());
        let hardware =
            views::hardware::HwState { port: st.last_port.clone().unwrap_or_default(), ..Default::default() };
        App {
            worker,
            sh: Shared {
                cfg: cfg_key(&st),
                st,
                log,
                pal,
                view: View::Overview,
                block: None,
                focus: None,
                console_open,
                time: 0.0,
                busy: false,
                busy_shown: false,
                errors: 0,
                data: HashMap::new(),
                asked: RefCell::new(HashMap::new()),
                ports: vec![],
                usb: None,
                acts: RefCell::new(vec![]),
            },
            vs: ViewState { hardware, ..Default::default() },
            toasts: vec![],
            busy_since: None,
            t0: Instant::now(),
            shots,
        }
    }

    /// Take what the engine thread published since the last frame.
    fn refresh(&mut self, ctx: &egui::Context) {
        let u = self.worker.take();
        if let Some(st) = u.state {
            self.sh.cfg = cfg_key(&st);
            self.sh.st = st;
        }
        if u.log_reset {
            self.sh.log.clear();
        }
        self.sh.log.extend(u.log);
        let n = self.sh.log.len();
        if n > 2000 {
            self.sh.log.drain(..n - 2000);
        }
        self.sh.errors += u.errors;
        for r in u.replies {
            match r {
                Reply::Toast(t, s) => self.toast(t, s),
                Reply::ExportText(text) => self.save_export(text),
                Reply::Data { slot, key, data } => {
                    // a late answer for an older key must not replace the current one
                    let asked = self.sh.asked.borrow().get(slot).copied();
                    let have_current = self.sh.data.get(slot).is_some_and(|d| Some(d.0) == asked);
                    if !have_current || Some(key) == asked {
                        self.sh.data.insert(slot, (key, data));
                    }
                }
                Reply::Ports(ports, usb) => {
                    let hw = &mut self.vs.hardware;
                    if hw.port.is_empty() {
                        if let Some(p) = ports.iter().find(|p| p.kind == "usb").or(ports.first()) {
                            hw.port = p.path.clone();
                        }
                    }
                    self.sh.ports = ports;
                    self.sh.usb = Some(usb);
                }
            }
        }
        let busy = self.worker.busy();
        self.busy_since = if busy { self.busy_since.or(Some(Instant::now())) } else { None };
        self.sh.busy = busy;
        // show the indicator only for jobs that take noticeable time
        self.sh.busy_shown = self.busy_since.is_some_and(|t| t.elapsed() > Duration::from_millis(150));
        if busy && !self.sh.busy_shown {
            ctx.request_repaint_after(Duration::from_millis(160));
        }
    }

    fn send(&mut self, job: Job) {
        if !self.worker.send(job) {
            self.toast(Tone::Err, "Поток движка остановлен — перезапустите приложение");
        }
    }

    fn save_export(&mut self, text: String) {
        // dialogs stay on the UI thread (macOS requires it)
        let name = format!("{}_config.txt", CHIP_NAME);
        if let Some(path) = rfd::FileDialog::new().set_file_name(name).add_filter("Текст", &["txt"]).save_file() {
            match std::fs::write(&path, text) {
                Ok(()) => self.toast(Tone::Ok, format!("Конфигурация сохранена: {}", path.display())),
                Err(e) => self.toast(Tone::Err, format!("{}: {}", path.display(), e)),
            }
        }
    }

    fn toast(&mut self, tone: Tone, text: impl Into<String>) {
        let ms = if tone == Tone::Err { 6500 } else { 3200 };
        self.toasts.push(Toast { tone, text: text.into(), until: Instant::now() + Duration::from_millis(ms) });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    fn run(&mut self, ctx: &egui::Context, a: Act) {
        match a {
            Act::Rpc(cmd, args, ok) => self.send(Job::Rpc { cmd, args, ok }),
            Act::Batch(calls, ok) => self.send(Job::Batch { calls, ok }),
            Act::Query(slot, key, q) => self.send(Job::Query { slot, key, q }),
            Act::Go(v, b) => {
                self.sh.view = v;
                if let Some(b) = b {
                    self.sh.block = b;
                }
            }
            Act::SelectBlock(b) => self.sh.block = b,
            Act::Focus(f) => self.sh.focus = f,
            Act::Toast(t, s) => self.toast(t, s),
            Act::Export => self.send(Job::Export),
            Act::Import => {
                let Some(path) =
                    rfd::FileDialog::new().add_filter("Конфигурация", &["txt", "h", "c", "cfg"]).pick_file()
                else {
                    return;
                };
                match std::fs::read_to_string(&path) {
                    Ok(text) => self.send(Job::Rpc {
                        cmd: "import_config",
                        args: json!({ "text": text }),
                        ok: Some("Конфигурация загружена".into()),
                    }),
                    Err(e) => self.toast(Tone::Err, format!("{}: {}", path.display(), e)),
                }
            }
            Act::Raw(hex) => self.send(Job::Raw(hex)),
            Act::CheckLink => self.send(Job::CheckLink),
            Act::RefreshPorts => self.send(Job::Ports),
            Act::ClearLog => self.send(Job::ClearLog),
            Act::Console(open) => self.sh.console_open = open,
            Act::Theme(dark) => {
                self.sh.pal = Pal::get(dark);
                theme::apply(ctx, &self.sh.pal);
            }
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        const KEYS: [Key; 9] =
            [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];
        ctx.input_mut(|i| {
            for (n, k) in KEYS.iter().enumerate() {
                if i.consume_key(egui::Modifiers::COMMAND, *k) {
                    self.sh.view = NAV[n].0;
                }
            }
            if i.consume_key(egui::Modifiers::COMMAND, Key::J) || i.consume_key(egui::Modifiers::COMMAND, Key::Backtick)
            {
                self.sh.console_open = !self.sh.console_open;
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sh.time = self.t0.elapsed().as_secs_f64();
        self.refresh(ctx);
        self.shortcuts(ctx);
        if let Some(step) = self.shots.as_ref().and_then(|s| s.current()) {
            self.sh.view = step.view;
            self.sh.console_open = step.console;
            self.vs.modulation.keying = step.variant == 1 && step.view == View::Mod;
            if step.view == View::Eco {
                self.vs.eco.page = step.variant as usize;
            }
            if step.view == View::Overview {
                self.sh.block = (step.variant == 1).then_some("acc1");
            }
            if step.dark != self.sh.pal.dark {
                self.sh.pal = Pal::get(step.dark);
                theme::apply(ctx, &self.sh.pal);
            }
        }

        let sh = &self.sh;
        let vs = &mut self.vs;
        top_bar(ctx, sh);
        sidebar(ctx, sh);
        if sh.console_open {
            console(ctx, sh, &mut vs.console);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(sh.pal.bg).inner_margin(egui::Margin {
                left: 24,
                right: 24,
                top: 18,
                bottom: 18,
            }))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().id_salt(("view", sh.view)).auto_shrink(false).show(ui, |ui| {
                    ui.set_max_width(ui.available_width().min(1600.0));
                    match sh.view {
                        View::Overview => views::overview::show(ui, sh),
                        View::Board => views::board::show(ui, sh, &mut vs.board),
                        View::Registers => views::registers::show(ui, sh, &mut vs.registers),
                        View::Freq => views::freq::show(ui, sh, &mut vs.freq),
                        View::Spectrum => views::spectrum::show(ui, sh, &mut vs.spectrum),
                        View::Chain => views::chain::show(ui, sh),
                        View::Sweep => views::sweep::show(ui, sh, &mut vs.sweep),
                        View::Hop => views::hop::show(ui, sh, &mut vs.hop),
                        View::Mod => views::modulation::show(ui, sh, &mut vs.modulation),
                        View::Hardware => views::hardware::show(ui, sh, &mut vs.hardware),
                        View::Learn => views::learn::show(ui, sh, &mut vs.learn),
                        View::Eco => crate::eco::show(ui, sh, &mut vs.eco),
                    }
                });
            });
        self.show_toasts(ctx);

        // after the frame: run what the views queued
        let acts: Vec<Act> = self.sh.acts.borrow_mut().drain(..).collect();
        if !acts.is_empty() {
            for a in acts {
                self.run(ctx, a);
            }
            ctx.request_repaint();
        }
        // the web UI stops the sweep when its view unmounts
        if self.sh.view != View::Sweep {
            self.vs.sweep.stop();
        }
        if let Some(s) = self.shots.as_mut() {
            s.tick(ctx);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.shots.is_some() {
            return;
        }
        storage.set_string("theme", if self.sh.pal.dark { "dark".into() } else { "light".into() });
        storage.set_string("console", if self.sh.console_open { "1".into() } else { "0".into() });
    }
}

impl App {
    fn show_toasts(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.toasts.retain(|t| t.until > now);
        if self.toasts.is_empty() {
            return;
        }
        let p = self.sh.pal;
        let mut dismiss = None;
        egui::Area::new("toasts".into()).anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0)).show(ctx, |ui| {
            ui.set_max_width(380.0);
            for (i, t) in self.toasts.iter().enumerate() {
                let (c, icon) = match t.tone {
                    Tone::Ok => (p.ok, "✔"),
                    Tone::Err => (p.err, "⚠"),
                    Tone::Info => (p.accent, "ℹ"),
                };
                let r = egui::Frame::new()
                    .fill(p.panel)
                    .stroke(egui::Stroke::new(
                        1.0,
                        if t.tone == Tone::Info { p.line } else { theme::mix(c, 45.0, p.panel) },
                    ))
                    .corner_radius(12)
                    .inner_margin(egui::Margin::symmetric(12, 9))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 6],
                        blur: 18,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new(icon).color(c));
                            ui.add(egui::Label::new(RichText::new(&t.text).color(p.fg2).size(12.5)).wrap());
                        });
                    })
                    .response
                    .interact(egui::Sense::click())
                    .on_hover_text("Закрыть");
                if r.clicked() {
                    dismiss = Some(i);
                }
                ui.add_space(6.0);
            }
        });
        if let Some(i) = dismiss {
            self.toasts.remove(i);
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

/// Overall device state for the top-bar pill: (tone, text). Mirrors `stateInfo` in store.ts.
fn state_info(sh: &Shared) -> (Color32, String) {
    let p = &sh.pal;
    if sh.st.link_error.is_some() {
        return (p.err, "Ошибка связи".into());
    }
    match &sh.st.sim {
        None => (p.ok, "Плата подключена".into()),
        Some(s) => {
            let c = match s.state {
                "ok" => p.ok,
                "busy" | "warn" => p.warn,
                "fault" => p.err,
                _ => p.line2,
            };
            (c, s.text.clone())
        }
    }
}

fn top_bar(ctx: &egui::Context, sh: &Shared) {
    let p = sh.pal;
    egui::TopBottomPanel::top("top")
        .exact_height(52.0)
        .frame(
            egui::Frame::new()
                .fill(p.bg2)
                .inner_margin(egui::Margin::symmetric(14, 0))
                .stroke(egui::Stroke::new(1.0, p.line)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // logo: the web draws a gradient accent → clk; a mix of the two reads the same
                let (r, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                ui.painter().rect_filled(r, 8.0, theme::mix(p.accent, 60.0, p.clk));
                ui.painter().text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    "▣",
                    egui::FontId::proportional(15.0),
                    p.accent_fg,
                );
                ui.vertical(|ui| {
                    ui.add_space(9.0);
                    ui.label(RichText::new("DDS Studio").strong().size(13.0));
                    ui.label(
                        RichText::new(format!("{} {} · {}", CHIP_VENDOR, CHIP_NAME, BOARD_NAME))
                            .size(10.5)
                            .color(p.muted),
                    );
                });
                ui.add_space(4.0);
                ui.separator();

                // top metrics: as many as fit (the web hides them below 1280 px)
                let n = ((ui.available_width() - 760.0) / 130.0).floor().clamp(0.0, 5.0) as usize;
                for m in sh.st.derived.metrics.iter().filter(|m| m.top).take(n) {
                    let tone = crate::content::block(m.block).map(|b| p.tone(b.tone)).unwrap_or(p.fg);
                    let r = ui
                        .vertical(|ui| {
                            ui.add_space(9.0);
                            ui.label(RichText::new(m.label.to_uppercase()).size(9.5).color(p.muted));
                            ui.label(RichText::new(eng4(m.value, m.unit)).size(13.5).strong().color(tone));
                        })
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if r.clicked() {
                        sh.go_block(View::Overview, Some(m.block));
                    }
                    ui.add_space(4.0);
                }

                let (tone, text) = state_info(sh);
                if w::pill(ui, &p, &text, tone, true).on_hover_text("Состояние кристалла").clicked() {
                    sh.go(View::Overview);
                }
                let errors = sh.st.derived.issues.iter().filter(|i| i.level == "error").count();
                if errors > 0 {
                    let t = format!("{} {} конфигурации", errors, if errors == 1 { "ошибка" } else { "ошибки" });
                    if w::pill(ui, &p, &t, p.err, false).clicked() {
                        sh.go(View::Overview);
                    }
                }
                if sh.busy_shown {
                    ui.add(egui::Spinner::new().size(14.0).color(p.accent))
                        .on_hover_text("Идёт обмен с платой — интерфейс не ждёт его окончания");
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let theme_icon = if p.dark { "☀" } else { "🌙" };
                    if w::icon_button(ui, &p, theme_icon, false).on_hover_text("Тема оформления").clicked()
                    {
                        sh.act(Act::Theme(!p.dark));
                    }
                    if w::icon_button(ui, &p, "🖥", sh.console_open).on_hover_text("Журнал шины (⌘/Ctrl + J)").clicked()
                    {
                        sh.act(Act::Console(!sh.console_open));
                    }
                    if w::icon_button(ui, &p, "⟲", false)
                        .on_hover_text("Сброс кристалла (вывод RESET, если есть)")
                        .clicked()
                    {
                        sh.rpc("chip_reset", json!({}));
                    }
                    let dirty = sh.st.dirty.iter().filter(|d| **d).count();
                    let label = if dirty > 0 {
                        format!("➤ Записать ({})", dirty)
                    } else {
                        "➤ Записать".into()
                    };
                    let kind = if dirty > 0 { w::Btn::Primary } else { w::Btn::Subtle };
                    if ui
                        .add_enabled(dirty > 0, w::btn(&p, &label, kind))
                        .on_hover_text("Записать изменённые регистры")
                        .clicked()
                    {
                        sh.rpc("apply", json!({}));
                    }
                    let mut on = sh.st.auto_apply;
                    let r = w::toggle(ui, &p, &mut on)
                        .on_hover_text("Автоматически записывать изменённые регистры в кристалл");
                    if r.changed() {
                        sh.rpc("set_auto_apply", json!({ "on": on }));
                    }
                    if ui.available_width() > 260.0 {
                        ui.label(RichText::new("Авто-запись").size(12.0).color(p.fg2));
                    }
                    let (c, t) = if sh.st.link_error.is_some() {
                        (p.err, sh.st.link.name.clone())
                    } else if sh.is_sim() {
                        (p.refc, "Симулятор".to_string())
                    } else {
                        (p.ok, sh.st.link.name.clone())
                    };
                    let tip = sh.st.link_error.clone().unwrap_or_else(|| sh.st.link.detail.clone());
                    if w::pill(ui, &p, &t, c, false).on_hover_text(tip).clicked() {
                        sh.go(View::Hardware);
                    }
                });
            });
        });
}

fn sidebar(ctx: &egui::Context, sh: &Shared) {
    let p = sh.pal;
    egui::SidePanel::left("nav")
        .exact_width(212.0)
        .resizable(false)
        .frame(egui::Frame::new().fill(p.bg2).inner_margin(10).stroke(egui::Stroke::new(1.0, p.line)))
        .show(ctx, |ui| {
            // metrics and version pinned to the bottom, navigation scrolls above them
            egui::TopBottomPanel::bottom("nav-bottom")
                .frame(egui::Frame::new().inner_margin(egui::Margin { left: 0, right: 0, top: 8, bottom: 0 }))
                .show_separator_line(false)
                .show_inside(ui, |ui| {
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        w::card_frame(&p).inner_margin(10).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.spacing_mut().item_spacing.y = 3.0;
                            for m in sh.st.derived.metrics.iter().filter(|m| !m.top).take(4) {
                                w::kv(ui, &p, m.label, &eng(m.value, m.unit, 3));
                            }
                            if let Some(sim) = &sh.st.sim {
                                for s in &sim.signals {
                                    w::kv(ui, &p, s.name, &eng(s.value, s.unit, 3));
                                }
                            }
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!("DDS Studio v{} · {}", env!("CARGO_PKG_VERSION"), developer()))
                                .size(10.5)
                                .color(p.muted),
                        )
                        .on_hover_text("Версия приложения и разработчик");
                    });
                });
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let mut group = "";
                for (i, (v, icon, label, hint, g)) in NAV.iter().enumerate() {
                    if *g != group {
                        group = g;
                        ui.add_space(if i > 0 { 8.0 } else { 2.0 });
                        ui.label(RichText::new(g.to_uppercase()).size(10.0).color(p.muted).strong());
                    }
                    if w::nav_item(ui, &p, icon, label, hint, i + 1, sh.view == *v).clicked() {
                        sh.go(*v);
                    }
                }
            });
        });
}

fn console(ctx: &egui::Context, sh: &Shared, cs: &mut ConsoleState) {
    let p = sh.pal;
    let bits = regs::DATA_BITS;
    let reg_name = |r: Option<usize>| r.map(|r| REGS.get(r).map(|d| d.name).unwrap_or("?")).unwrap_or("");
    egui::TopBottomPanel::bottom("console")
        .resizable(true)
        .default_height(260.0)
        .height_range(140.0..=600.0)
        .frame(
            egui::Frame::new()
                .fill(p.panel)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .stroke(egui::Stroke::new(1.0, p.line)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Журнал шины").strong().size(12.5));
                ui.label(RichText::new(format!("{} записей", sh.log.len())).size(11.0).color(p.muted));
                if let Some(f) = w::segmented(
                    ui,
                    &p,
                    cs.filter,
                    &[
                        (LogFilter::All, "Все"),
                        (LogFilter::Write, "Запись"),
                        (LogFilter::Read, "Чтение"),
                        (LogFilter::Note, "События"),
                    ],
                ) {
                    cs.filter = f;
                }
                ui.add(egui::TextEdit::singleline(&mut cs.q).hint_text("🔍 поле, регистр, hex…").desired_width(150.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if w::icon_button(ui, &p, "✕", false).on_hover_text("Закрыть").clicked() {
                        sh.act(Act::Console(false));
                    }
                    if w::icon_button(ui, &p, "🗑", false).on_hover_text("Очистить журнал").clicked() {
                        sh.act(Act::ClearLog);
                    }
                    let send = w::icon_button(ui, &p, "➤", false).on_hover_text("Отправить кадр").clicked();
                    let r = ui
                        .add(
                            egui::TextEdit::singleline(&mut cs.raw)
                                .hint_text("81 00 00")
                                .font(egui::TextStyle::Monospace)
                                .desired_width(120.0),
                        )
                        .on_hover_text("Произвольный кадр: SPI — hex-байты; I²C — «i2c <адрес> <байты> [rN]»");
                    ui.label(RichText::new("raw").size(11.0).color(p.muted));
                    if send || (r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
                        send_raw(sh, &cs.raw);
                    }
                });
            });
            ui.separator();

            let q = cs.q.trim().to_lowercase();
            let rows: Vec<&Txn> = sh
                .log
                .iter()
                .filter(|t| match cs.filter {
                    LogFilter::Write => matches!(t.op, "write" | "raw" | "pin" | "cmd"),
                    LogFilter::Read => t.op == "read",
                    LogFilter::Note => t.op == "note",
                    LogFilter::All => true,
                })
                .filter(|t| {
                    q.is_empty()
                        || t.note.to_lowercase().contains(&q)
                        || t.changes.iter().any(|c| c.0.to_lowercase().contains(&q))
                        || t.tx.as_deref().unwrap_or("").to_lowercase().contains(q.trim_start_matches("0x"))
                        || reg_name(t.reg).to_lowercase() == q
                })
                .collect();
            if rows.is_empty() {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Журнал пуст — измените любой параметр, и кадры шины появятся здесь.")
                            .color(p.muted),
                    );
                });
                return;
            }
            let t0 = sh.log.first().map(|t| t.t_ms).unwrap_or(0.0);
            let mono = |s: String, c: Color32| RichText::new(s).monospace().size(11.5).color(c);
            TableBuilder::new(ui)
                .striped(false)
                .stick_to_bottom(true)
                .resizable(false)
                .column(Column::exact(44.0))
                .column(Column::exact(76.0))
                .column(Column::exact(44.0))
                .column(Column::exact(96.0))
                .column(Column::exact(64.0))
                .column(Column::exact(130.0))
                .column(Column::exact(70.0))
                .column(Column::remainder())
                .header(18.0, |mut h| {
                    for title in ["#", "T, МС", "ОП.", "РЕГИСТР", "ЗНАЧЕНИЕ", "TX", "RX", "ИЗМЕНЕНИЯ / СОБЫТИЕ"]
                    {
                        h.col(|ui| {
                            ui.label(RichText::new(title).size(10.0).color(p.muted));
                        });
                    }
                })
                .body(|body| {
                    body.rows(18.0, rows.len(), |mut row| {
                        let t = rows[row.index()];
                        let base = if t.ok { p.fg2 } else { p.err };
                        row.col(|ui| {
                            ui.label(mono(t.id.to_string(), p.muted));
                        });
                        row.col(|ui| {
                            ui.label(mono(format!("{:.1}", t.t_ms - t0), p.muted));
                        });
                        row.col(|ui| {
                            let (s, c) = match t.op {
                                "write" => ("↑ W", p.sig),
                                "read" => ("↓ R", p.clk),
                                "raw" => ("RAW", p.refc),
                                "pin" => ("PIN", p.ctl),
                                "cmd" => ("CMD", p.dig),
                                _ => ("•", p.muted),
                            };
                            ui.label(mono(s.into(), c));
                        });
                        row.col(|ui| {
                            let r = match (t.reg, t.addr) {
                                (Some(_), _) => reg_name(t.reg).to_string(),
                                (None, Some(a)) => hex_w(a as u32, 16),
                                _ => String::new(),
                            };
                            ui.add(egui::Label::new(mono(r, base)).truncate());
                        });
                        row.col(|ui| {
                            ui.label(mono(t.value.map(|v| hex_w(v, bits)).unwrap_or_default(), base));
                        });
                        row.col(|ui| {
                            ui.add(egui::Label::new(mono(t.tx.clone().unwrap_or_default(), base)).truncate());
                        });
                        row.col(|ui| {
                            ui.label(mono(t.rx.clone().unwrap_or_default(), base));
                        });
                        row.col(|ui| {
                            let mut job = egui::text::LayoutJob::default();
                            let mf = egui::FontId::monospace(11.5);
                            let pf = egui::FontId::proportional(12.0);
                            let fmtc = |c: Color32, f: &egui::FontId| egui::TextFormat::simple(f.clone(), c);
                            for (n, a, b) in &t.changes {
                                job.append(n, 0.0, fmtc(p.accent, &mf));
                                job.append(&format!(" {}→", a), 0.0, fmtc(p.muted, &mf));
                                job.append(&format!("{}  ", b), 0.0, fmtc(p.fg, &mf));
                            }
                            job.append(&t.note, 0.0, fmtc(if t.ok { p.fg2 } else { p.err }, &pf));
                            ui.add(egui::Label::new(job).truncate());
                        });
                    });
                });
        });
}

fn send_raw(sh: &Shared, raw: &str) {
    let s = raw.trim();
    let body = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if body.is_empty() || !body.chars().all(|c| c.is_ascii_hexdigit() || c.is_whitespace()) {
        sh.toast(Tone::Err, "SPI: hex-байты, например 81 00 00. I²C: i2c 48 00 r2");
        return;
    }
    sh.act(Act::Raw(body.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every icon glyph must exist in the bundled fonts, or egui draws a box.
    #[test]
    fn icons_have_glyphs() {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        let _ = ctx.run(Default::default(), |_| {});
        let font = egui::FontId::proportional(14.0);
        let mut icons: Vec<&str> = NAV.iter().map(|n| n.1).collect();
        icons.extend([
            "☀", "🌙", "🖥", "⟲", "➤", "✔", "⚠", "ℹ", "✕", "🗑", "🔍", "⊗", "🛡", "🎯", "➜", "⌖", "▶", "■", "⏸", "💾",
            "🔀", "▦", "⚡", "↶", "🔗", "⟳", "🖴", "🖧", "⬆", "⬇", "📖", "•", "🔧", "📻",
        ]);
        let missing: Vec<&str> = icons.into_iter().filter(|s| !ctx.fonts_mut(|f| f.has_glyphs(&font, s))).collect();
        assert!(missing.is_empty(), "no glyph for {:?}", missing);
    }
}
