//! «Свип частоты»: stepped sweep driven from the PC (`SweepView.tsx`). Each
//! point is a seamless frequency change; the timer runs on the UI side and
//! never waits for the bus — the next point is sent only when the engine
//! thread has finished the previous one, so a slow USB-UART link stretches
//! the dwell instead of queueing writes.

use crate::app::{key, Act, Shared, Tone};
use crate::fmt::{axis_hz, eng};
use crate::plot::{Plot, Series};
use crate::widgets as w;
use crate::worker::{Data, Query};
use eframe::egui::Ui;
use serde_json::json;

pub struct SweepState {
    pub lo: f64,
    pub hi: f64,
    pub step: f64,
    pub dwell: f64,
    pub updown: bool,
    running: bool,
    idx: Option<usize>,
    passes: u32,
    i: usize,
    dir: i64,
    next_at: f64,
    errors0: u64,
}

impl Default for SweepState {
    fn default() -> Self {
        SweepState {
            lo: 48e6,
            hi: 52e6,
            step: 250e3,
            dwell: 200.0,
            updown: false,
            running: false,
            idx: None,
            passes: 0,
            i: 0,
            dir: 1,
            next_at: 0.0,
            errors0: 0,
        }
    }
}

impl SweepState {
    pub fn stop(&mut self) {
        self.running = false;
    }
}

pub fn show(ui: &mut Ui, sh: &Shared, st: &mut SweepState) {
    let p = sh.pal;
    let k = key((st.lo.to_bits(), st.hi.to_bits(), st.step.to_bits(), sh.cfg));
    let plan = match sh.query("sweep", k, Query::Sweep { lo: st.lo, hi: st.hi, step: st.step }) {
        Some(Data::Sweep(s)) => Some(s),
        _ => None,
    };
    let n = plan.map(|p| p.f.len()).unwrap_or(0);
    let hw = !sh.is_sim();
    let min_dwell = if hw { 50.0 } else { 20.0 };

    // the timer
    if st.running {
        match plan {
            Some(pl) if !pl.f.is_empty() => {
                if sh.errors > st.errors0 {
                    let f = st.idx.and_then(|i| pl.f.get(i)).copied().unwrap_or(0.0);
                    sh.toast(Tone::Err, format!("Свип остановлен на {}: ошибка записи", eng(f, "Hz", 6)));
                    st.running = false;
                } else if sh.time >= st.next_at && !sh.busy {
                    let i = st.i.min(pl.f.len() - 1);
                    st.idx = Some(i);
                    sh.rpc("set_frequency", json!({ "f": pl.f[i], "seamless": true }));
                    let mut j = i as i64 + st.dir;
                    if j >= pl.f.len() as i64 || j < 0 {
                        st.passes += 1;
                        if st.updown && pl.f.len() > 1 {
                            st.dir = -st.dir;
                            j = i as i64 + st.dir;
                        } else {
                            j = 0;
                        }
                    }
                    st.i = j as usize;
                    st.next_at = sh.time + st.dwell / 1000.0;
                }
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(10));
            }
            _ => st.running = false,
        }
    }

    w::page_header(
        ui,
        &p,
        "➡",
        "Свип частоты",
        "Ступенчатая перестройка по сетке. Каждая точка — бесшовная смена частоты через профили 0/1. Для непрерывной перестройки без ступенек используйте ЛЧМ на вкладке «Модуляция».",
        |ui| {
            if st.running {
                if w::button(ui, &p, "■ Стоп", w::Btn::Danger).clicked() {
                    st.running = false;
                }
            } else if w::button_en(ui, &p, "▶ Запустить", w::Btn::Primary, n > 0).clicked() {
                st.i = 0;
                st.dir = 1;
                st.passes = 0;
                st.errors0 = sh.errors;
                st.next_at = 0.0;
                st.running = true;
            }
        },
    );
    let cur = st.idx.and_then(|i| plan.and_then(|p| p.f.get(i).copied()));
    w::two_cols(
        ui,
        380.0,
        true,
        st,
        |ui, st| {
            w::card(
                ui,
                &p,
                "Ожидаемый уровень по сетке",
                "Модель тракта: sin(x)/x × ФНЧ × аттенюатор при текущем Mul",
                |ui| {
                    if let Some(pl) = plan {
                        let mut plot = Plot::new(
                            "sweep",
                            vec![Series::new("XW2", pl.f.clone(), pl.dbm.clone(), p.sig).width(2.0)],
                        )
                        .height(380.0)
                        .x_fmt(axis_hz)
                        .labels("", "дБм", "")
                        .shade(48e6, 52e6)
                        .tooltip_x(|v| eng(v, "Hz", 6));
                        if let (Some(f), Some(i)) = (cur, st.idx) {
                            plot = plot.vmark(f, eng(f, "Hz", 6), Some(p.accent), false).point(
                                f,
                                pl.dbm[i],
                                "",
                                Some(p.accent),
                            );
                        }
                        plot.show(ui, &p);
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if st.running {
                            w::badge(ui, &p, "идёт свип", p.ok);
                        }
                        if ui.add(w::small(&p, "Встать на начало", w::Btn::Ghost)).clicked() {
                            sh.rpc("set_frequency", json!({ "f": st.lo, "seamless": true }));
                        }
                    });
                },
            );
        },
        |ui, st| {
            w::card(ui, &p, "Сетка", "", |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                w::row(ui, &p, "От", "", |ui| {
                    if let Some(v) = w::eng_field(ui, &p, "sw-lo", st.lo, "Hz", 0.0, 240e6, 140.0) {
                        st.lo = v;
                    }
                });
                w::row(ui, &p, "До", "", |ui| {
                    if let Some(v) = w::eng_field(ui, &p, "sw-hi", st.hi, "Hz", 0.0, 240e6, 140.0) {
                        st.hi = v;
                    }
                });
                w::row(ui, &p, "Шаг", "", |ui| {
                    if let Some(v) = w::eng_field(ui, &p, "sw-step", st.step, "Hz", 1e-3, 100e6, 140.0) {
                        st.step = v;
                    }
                });
                w::row(ui, &p, "Время на точке", &format!("не меньше {} мс", min_dwell), |ui| {
                    if let Some(v) =
                        w::num_field(ui, &p, "sw-dwell", st.dwell, min_dwell, 60000.0, 10.0, true, "мс", 56.0)
                    {
                        st.dwell = v;
                    }
                });
                w::row(ui, &p, "Проход", "", |ui| {
                    if let Some(v) =
                        w::segmented_sized(ui, &p, st.updown, &[(false, "Пила ↗"), (true, "Туда-обратно ↗↘")], true)
                    {
                        st.updown = v;
                    }
                });
                if let Some(e) = sh.query_err("sweep") {
                    w::hint_text(ui, &p, p.err, e);
                }
            });
            w::cols(ui, 2, 120.0, |ui, i| {
                if i == 0 {
                    w::stat(ui, &p, "Точек", &n.to_string(), None, "");
                    w::stat(ui, &p, "Сейчас", &cur.map(|f| eng(f, "Hz", 7)).unwrap_or("—".into()), Some(p.sig), "");
                } else {
                    w::stat(ui, &p, "Период прохода", &eng(n as f64 * st.dwell / 1000.0, "s", 3), None, "");
                    w::stat(ui, &p, "Проходов", &st.passes.to_string(), None, "");
                }
            });
            if hw {
                w::hint_text(ui, &p, p.warn, "Плата: одна точка — около десяти команд по UART (запись трёх слов dPh, проверка, SEL_REG). При шаге меньше 50 мс точки будут растягиваться.");
            } else {
                w::hint_text(ui, &p, p.accent, "Симулятор: смотрите, как меняются «Частота выхода» в верхней строке и номер профиля 0/1 в журнале шины (⌘/Ctrl + J).");
            }
        },
    );
    let _ = Act::ClearLog;
}
