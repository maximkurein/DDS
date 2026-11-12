//! Engine thread. The UI never blocks on I/O (USB-UART to the board can take
//! tens of milliseconds per command): it sends `Job`s over a channel, the
//! worker runs them against the `Engine` it owns and publishes the new `State`,
//! the new bus-log entries and replies into an inbox the UI drains once per frame.
//!
//! Mutations go through `Engine::handle` — the same RPC as the Tauri app and
//! `dds-server`, so every rule of the core holds in one place. Read-only
//! analysis (spectrum, response, plans) uses the typed helpers of the core.

use crate::app::Tone;
use dds_core::api::{Profile, State, SweepPlan, Txn};
use dds_core::model::{FreqPlan, LfmPlan, LfmReq, ModPlan, ModReq, PhaseNoise, Response, Spectrum};
use dds_core::transport::{self, PortInfo, UsbDev};
use dds_core::Engine;
use eframe::egui;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// Republish while idle so live status keeps moving (the web UI polls `status` every 350 ms).
const IDLE_REFRESH: Duration = Duration::from_millis(350);

/// Read-only computation for a view (the web UI's `useRpcData`).
#[derive(Clone, Debug)]
pub enum Query {
    Plan(f64),
    Profiles(u8),
    Response { lo: f64, hi: f64, n: usize },
    Spectrum { lo: f64, hi: f64, log2n: u32, points: usize },
    PhaseNoise,
    Sweep { lo: f64, hi: f64, step: f64 },
    Lfm(LfmReq),
    Mod(ModReq),
}

/// Result of a `Query`.
pub enum Data {
    Plan(FreqPlan),
    Profiles(Vec<Profile>),
    Response(Response),
    Spectrum(Box<Spectrum>),
    PhaseNoise(PhaseNoise),
    Sweep(SweepPlan),
    Lfm(LfmPlan),
    Mod(ModPlan),
}

pub enum Job {
    /// `Engine::handle(cmd, args)`; `ok` is toasted on success.
    Rpc {
        cmd: &'static str,
        args: Value,
        ok: Option<String>,
    },
    /// Several RPCs in order; stops at the first error (e.g. "fill the profile grid").
    Batch {
        calls: Vec<(&'static str, Value)>,
        ok: Option<String>,
    },
    /// Read-only analysis; the answer comes back as `Reply::Data` for `slot`.
    Query {
        slot: &'static str,
        key: u64,
        q: Query,
    },
    /// `export_config`; the UI shows the save dialog (dialogs stay on the UI thread).
    Export,
    /// One raw 24-bit SPI frame from the bus console.
    Raw(String),
    /// DEVID check with a verdict toast.
    CheckLink,
    /// Serial port list + USB scan for the Connection view.
    Ports,
    ClearLog,
}

pub enum Reply {
    Toast(Tone, String),
    ExportText(String),
    Data { slot: &'static str, key: u64, data: Result<Data, String> },
    Ports(Vec<PortInfo>, Vec<UsbDev>),
}

/// What the worker published since the UI last looked.
pub struct Update {
    pub state: Option<State>,
    pub log: Vec<Txn>,
    pub log_reset: bool,
    pub replies: Vec<Reply>,
    /// Errors since the last take (lets a running sweep stop on a failed write).
    pub errors: u64,
}

#[derive(Default)]
struct Inbox {
    state: Option<State>,
    log: Vec<Txn>,
    log_reset: bool,
    replies: Vec<Reply>,
    errors: u64,
}

pub struct Worker {
    tx: Sender<Job>,
    inbox: Arc<Mutex<Inbox>>,
    pending: Arc<AtomicUsize>,
}

fn lock(m: &Mutex<Inbox>) -> MutexGuard<'_, Inbox> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Worker {
    /// Move the engine to its own thread. Returns the worker, the initial state and log.
    pub fn spawn(mut engine: Engine, ctx: egui::Context) -> (Worker, State, Vec<Txn>) {
        let state = engine.state();
        let log: Vec<Txn> = engine.log_since(0).cloned().collect();
        let mut head = engine.log_head();
        let (tx, rx) = mpsc::channel::<Job>();
        let inbox = Arc::new(Mutex::new(Inbox::default()));
        let pending = Arc::new(AtomicUsize::new(0));
        let (inbox2, pending2) = (inbox.clone(), pending.clone());
        std::thread::Builder::new()
            .name("engine".into())
            .spawn(move || loop {
                let first = match rx.recv_timeout(IDLE_REFRESH) {
                    Ok(j) => Some(j),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                let mut jobs: Vec<Job> = first.into_iter().collect();
                jobs.extend(rx.try_iter());
                let taken = jobs.len();
                let mut out = Out::default();
                for job in coalesce(jobs) {
                    run(&mut engine, job, &mut out);
                }
                let state = engine.state();
                if out.log_reset {
                    head = 0;
                }
                let new_log: Vec<Txn> = engine.log_since(head).cloned().collect();
                head = engine.log_head();
                {
                    let mut ib = lock(&inbox2);
                    ib.state = Some(state);
                    if out.log_reset {
                        ib.log.clear();
                        ib.log_reset = true;
                    }
                    ib.log.extend(new_log);
                    ib.replies.extend(out.replies);
                    ib.errors += out.errors;
                }
                pending2.fetch_sub(taken, Ordering::SeqCst);
                ctx.request_repaint();
            })
            .expect("failed to start the engine thread");
        (Worker { tx, inbox, pending }, state, log)
    }

    /// Queue a job; `false` if the engine thread is gone.
    pub fn send(&self, job: Job) -> bool {
        self.pending.fetch_add(1, Ordering::SeqCst);
        if self.tx.send(job).is_err() {
            self.pending.fetch_sub(1, Ordering::SeqCst);
            return false;
        }
        true
    }

    /// Jobs queued or running.
    pub fn busy(&self) -> bool {
        self.pending.load(Ordering::SeqCst) > 0
    }

    /// Never waits: takes whatever was published.
    pub fn take(&self) -> Update {
        let mut ib = lock(&self.inbox);
        Update {
            state: ib.state.take(),
            log: std::mem::take(&mut ib.log),
            log_reset: std::mem::take(&mut ib.log_reset),
            replies: std::mem::take(&mut ib.replies),
            errors: std::mem::take(&mut ib.errors),
        }
    }
}

/// Key under which a job may be replaced by a later one of the same kind.
/// `set_field` of the same field and queries of the same slot are coalesced.
fn coalesce_key(j: &Job) -> Option<String> {
    match j {
        Job::Rpc { cmd: "set_field", args, ok: None } => args["name"].as_str().map(|n| format!("set_field:{}", n)),
        Job::Query { slot, .. } => Some(format!("query:{}", slot)),
        _ => None,
    }
}

/// Keep only the last of each coalescible run, preserving order otherwise.
/// A job of another kind in between is a barrier.
fn coalesce(jobs: Vec<Job>) -> Vec<Job> {
    let keys: Vec<Option<String>> = jobs.iter().map(coalesce_key).collect();
    let mut out = Vec::with_capacity(jobs.len());
    for (i, job) in jobs.into_iter().enumerate() {
        let superseded = keys[i].as_ref().is_some_and(|k| {
            keys[i + 1..].iter().take_while(|n| n.is_some()).any(|n| n.as_deref() == Some(k.as_str()))
        });
        if !superseded {
            out.push(job);
        }
    }
    out
}

#[derive(Default)]
struct Out {
    replies: Vec<Reply>,
    errors: u64,
    log_reset: bool,
}

impl Out {
    fn err(&mut self, msg: String) {
        self.errors += 1;
        self.replies.push(Reply::Toast(Tone::Err, msg));
    }
}

fn call(engine: &mut Engine, cmd: &str, args: Value, out: &mut Out) -> Option<Value> {
    match engine.handle(cmd, args) {
        Ok(v) => Some(v),
        Err(e) => {
            out.err(e.to_string());
            None
        }
    }
}

fn query(engine: &Engine, q: Query) -> Result<Data, String> {
    Ok(match q {
        Query::Plan(f) => Data::Plan(engine.plan_frequency(f)),
        Query::Profiles(ch) => Data::Profiles(engine.profiles(ch)),
        Query::Response { lo, hi, n } => Data::Response(engine.response(lo, hi, n)),
        Query::Spectrum { lo, hi, log2n, points } => {
            Data::Spectrum(Box::new(engine.spectrum(lo, hi, log2n, points).map_err(|e| e.to_string())?))
        }
        Query::PhaseNoise => Data::PhaseNoise(engine.phase_noise()),
        Query::Sweep { lo, hi, step } => Data::Sweep(engine.sweep_plan(lo, hi, step).map_err(|e| e.to_string())?),
        Query::Lfm(req) => Data::Lfm(engine.lfm_design(&req)),
        Query::Mod(req) => Data::Mod(engine.mod_design(&req)),
    })
}

fn run(engine: &mut Engine, job: Job, out: &mut Out) {
    match job {
        Job::Rpc { cmd, args, ok } => {
            if call(engine, cmd, args, out).is_some() {
                if let Some(msg) = ok {
                    out.replies.push(Reply::Toast(Tone::Ok, msg));
                }
            }
        }
        Job::Batch { calls, ok } => {
            let n = calls.len();
            let done =
                calls.into_iter().take_while(|(cmd, args)| call(engine, cmd, args.clone(), out).is_some()).count();
            if done == n {
                if let Some(msg) = ok {
                    out.replies.push(Reply::Toast(Tone::Ok, msg));
                }
            }
        }
        Job::Query { slot, key, q } => out.replies.push(Reply::Data { slot, key, data: query(engine, q) }),
        Job::Export => {
            if let Some(r) = call(engine, "export_config", json!({}), out) {
                out.replies.push(Reply::ExportText(r["text"].as_str().unwrap_or_default().to_string()));
            }
        }
        Job::Raw(hex) => {
            if let Some(r) = call(engine, "raw", json!({ "hex": hex }), out) {
                let rx = r["rx"].as_str().filter(|s| !s.is_empty()).unwrap_or("—");
                out.replies.push(Reply::Toast(Tone::Info, format!("Ответ: {}", rx)));
            }
        }
        Job::CheckLink => {
            if let Some(r) = call(engine, "check_link", json!({}), out) {
                if r["ok"].as_bool() == Some(true) {
                    out.replies.push(Reply::Toast(Tone::Ok, "DEVID = 0x0201: связь с DDS есть".into()));
                } else {
                    let v = r["value"].as_u64().unwrap_or(0);
                    out.err(format!("DEVID = 0x{:X}: нет такта, питания DDS или связи SPI", v));
                }
            }
        }
        Job::Ports => {
            let usb = transport::scan_usb().unwrap_or_default();
            out.replies.push(Reply::Ports(transport::list_ports(), usb));
        }
        Job::ClearLog => {
            engine.clear_log();
            out.log_reset = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn set(name: &str, v: u32) -> Job {
        Job::Rpc { cmd: "set_field", args: json!({ "name": name, "value": v }), ok: None }
    }

    fn describe(jobs: &[Job]) -> Vec<String> {
        jobs.iter()
            .map(|j| match j {
                Job::Rpc { cmd, args, .. } => format!("{}:{}", cmd, args.get("value").cloned().unwrap_or(Value::Null)),
                Job::Query { slot, key, .. } => format!("q:{}:{}", slot, key),
                _ => "other".into(),
            })
            .collect()
    }

    #[test]
    fn coalesces_runs_but_keeps_barriers() {
        let q = |key| Job::Query { slot: "spectrum", key, q: Query::PhaseNoise };
        let jobs = vec![
            set("pdith1", 1),
            set("pdith1", 0),
            set("sum", 0),
            set("pdith1", 1),
            q(1),
            q(2),
            Job::Rpc { cmd: "apply", args: json!({}), ok: None },
            set("pdith1", 0),
            set("pdith1", 1),
        ];
        assert_eq!(
            describe(&coalesce(jobs)),
            ["set_field:0", "set_field:1", "q:spectrum:2", "apply:null", "set_field:1"]
        );
    }

    /// Poll the inbox until the worker is idle and something was published.
    fn wait(w: &Worker) -> Update {
        let t0 = Instant::now();
        let mut acc = Update { state: None, log: vec![], log_reset: false, replies: vec![], errors: 0 };
        loop {
            let u = w.take();
            if u.log_reset {
                acc.log.clear();
                acc.log_reset = true;
            }
            acc.log.extend(u.log);
            acc.replies.extend(u.replies);
            acc.errors += u.errors;
            if u.state.is_some() {
                acc.state = u.state;
            }
            if !w.busy() && acc.state.is_some() {
                return acc;
            }
            assert!(t0.elapsed() < Duration::from_secs(10), "worker did not answer");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn runs_jobs_off_thread_and_publishes() {
        let (w, st, log) = Worker::spawn(Engine::new(None), egui::Context::default());
        assert!(!log.is_empty(), "startup log is handed over");
        assert_eq!(st.fields["pdith1"], 0);
        assert!(w.send(set("pdith1", 1)));
        let u = wait(&w);
        assert_eq!(u.state.unwrap().fields["pdith1"], 1);
        assert!(u.log.iter().any(|t| t.op == "write"), "the write is streamed to the bus log");
        assert!(u.log.iter().all(|t| t.id > log.last().unwrap().id), "no entry is sent twice");
    }

    #[test]
    fn errors_become_toasts_and_are_counted() {
        let (w, _, _) = Worker::spawn(Engine::new(None), egui::Context::default());
        // read-only field: the core refuses it
        w.send(Job::Rpc { cmd: "set_field", args: json!({ "name": "DEVID", "value": 1 }), ok: Some("нет".into()) });
        let u = wait(&w);
        assert_eq!(u.errors, 1);
        assert!(matches!(u.replies.as_slice(), [Reply::Toast(Tone::Err, _)]));
    }

    #[test]
    fn batch_stops_at_first_error() {
        let (w, _, _) = Worker::spawn(Engine::new(None), egui::Context::default());
        w.send(Job::Batch {
            calls: vec![
                ("set_profile", json!({ "ch": 1, "y": 2, "f": 49e6 })),
                ("set_profile", json!({ "ch": 1, "y": 99, "f": 49e6 })),
                ("set_profile", json!({ "ch": 1, "y": 3, "f": 51e6 })),
            ],
            ok: Some("готово".into()),
        });
        let u = wait(&w);
        assert_eq!(u.errors, 1, "profile 99 does not exist");
        assert!(!u.replies.iter().any(|r| matches!(r, Reply::Toast(Tone::Ok, _))));
        w.send(Job::Query { slot: "profiles", key: 7, q: Query::Profiles(1) });
        let u = wait(&w);
        let rows = u.replies.into_iter().find_map(|r| match r {
            Reply::Data { slot: "profiles", key: 7, data: Ok(Data::Profiles(p)) } => Some(p),
            _ => None,
        });
        let rows = rows.expect("profiles answered");
        assert!((rows[2].f - 49e6).abs() < 1.0);
        assert!((rows[3].f - 51e6).abs() > 1e3, "the batch stopped before profile 3");
    }

    #[test]
    fn queries_report_core_errors() {
        let (w, _, _) = Worker::spawn(Engine::new(None), egui::Context::default());
        w.send(Job::Query { slot: "sweep", key: 1, q: Query::Sweep { lo: 48e6, hi: 52e6, step: 0.0 } });
        let u = wait(&w);
        assert!(u.replies.iter().any(|r| matches!(r, Reply::Data { slot: "sweep", data: Err(_), .. })));
    }

    #[test]
    fn clear_log_resets_the_stream() {
        let (w, _, _) = Worker::spawn(Engine::new(None), egui::Context::default());
        w.send(Job::ClearLog);
        let u = wait(&w);
        assert!(u.log_reset);
        assert!(u.log.is_empty());
        w.send(set("pdith1", 1));
        let u = wait(&w);
        assert!(!u.log.is_empty(), "new entries after the reset arrive");
    }
}
