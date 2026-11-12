//! «Модуляция»: LFM generator and keying via profiles (`ModView.tsx`).

use crate::app::{key, Shared};
use crate::canvas::Canvas;
use crate::fmt::{axis_hz, eng, fixed, grouped};
use crate::plot::{Plot, Series};
use crate::widgets as w;
use crate::worker::{Data, Query};
use dds_core::model::{LfmReq, ModPlan, ModReq};
use eframe::egui::{self, Align2, Color32, RichText, Stroke, Ui};
use serde_json::json;

const KINDS: [(&str, &str); 8] = [
    ("fsk2", "2-ЧМн (2-FSK)"),
    ("fsk4", "4-ЧМн (4-FSK)"),
    ("bpsk", "ФМн-2 (BPSK)"),
    ("qpsk", "ФМн-4 (QPSK)"),
    ("psk8", "ФМн-8 (8-PSK)"),
    ("ask2", "2-АМн (OOK/ASK)"),
    ("ask4", "4-АМн (4-ASK)"),
    ("qam16", "КАМ-16 (16-QAM)"),
];

pub struct ModState {
    pub keying: bool,
    lfm: LfmReq,
    req: ModReq,
    ms: f64,
}

impl Default for ModState {
    fn default() -> Self {
        ModState {
            keying: false,
            lfm: LfmReq::default(),
            req: ModReq { kind: "qpsk".into(), f_center: 50e6, deviation: 100e3, mul: 4000 },
            ms: 500.0,
        }
    }
}

pub fn show(ui: &mut Ui, sh: &Shared, st: &mut ModState) {
    let p = sh.pal;
    w::page_header(
        ui,
        &p,
        "〰",
        "Модуляция",
        "ЛЧМ — встроенный генератор линейной частотной модуляции. Манипуляции (ЧМн, ФМн, АМн, КАМ) — набор профилей, между которыми переключается SEL_REG.",
        |ui| {
            if let Some(k) = w::segmented(ui, &p, st.keying, &[(false, "ЛЧМ"), (true, "Манипуляции")]) {
                st.keying = k;
            }
        },
    );
    if st.keying {
        keying(ui, sh, st);
    } else {
        lfm(ui, sh, st);
    }
}

fn lfm(ui: &mut Ui, sh: &Shared, st: &mut ModState) {
    let p = sh.pal;
    let plan = match sh.query("lfm", key((format!("{:?}", st.lfm), sh.cfg)), Query::Lfm(st.lfm.clone())) {
        Some(Data::Lfm(pl)) => Some(pl),
        _ => None,
    };
    let on = sh.field("CH1_LS_on") == 1;
    w::two_cols(
        ui,
        380.0,
        true,
        st,
        |ui, _| {
            w::card(
                ui,
                &p,
                "Частота во времени",
                "Два периода; серым — интервалы, где выход выключен",
                |ui| {
                    if let Some(pl) = plan {
                        let off: Vec<f64> = pl
                            .curve_f
                            .iter()
                            .zip(&pl.curve_on)
                            .map(|(f, on)| if *on { f64::NAN } else { *f })
                            .collect();
                        Plot::new(
                            "lfm",
                            vec![
                                Series::new("f(t)", pl.curve_t.clone(), pl.curve_f.clone(), p.sig).width(2.0),
                                Series::new("выход выкл.", pl.curve_t.clone(), off, p.muted).width(3.0),
                            ],
                        )
                        .height(320.0)
                        .x_fmt(|v| eng(v, "s", 3))
                        .y_fmt(axis_hz)
                        .labels("", "частота", "")
                        .tooltip_x(|v| eng(v, "s", 4))
                        .show(ui, &p);
                    }
                },
            );
            if let Some(pl) = plan {
                for n in &pl.notes {
                    w::hint_text(ui, &p, p.accent, n);
                }
            }
        },
        |ui, st| {
            w::card(
                ui,
                &p,
                "Параметры ЛЧМ",
                "Канал 1, генератор ЛЧМ (ТО, разд. 3.3)",
                |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let r = &mut st.lfm;
                    w::row(ui, &p, "Начало", "", |ui| {
                        if let Some(v) = w::eng_field(ui, &p, "lfm-a", r.f_start, "Hz", 0.0, 240e6, 140.0) {
                            r.f_start = v;
                        }
                    });
                    w::row(ui, &p, "Конец", "", |ui| {
                        if let Some(v) = w::eng_field(ui, &p, "lfm-b", r.f_stop, "Hz", 0.0, 240e6, 140.0) {
                            r.f_stop = v;
                        }
                    });
                    w::row(ui, &p, "Нарастание", "стадия 1", |ui| {
                        if let Some(v) = w::eng_field(ui, &p, "lfm-t", r.t_ramp, "s", 1e-8, 100.0, 140.0) {
                            r.t_ramp = v;
                        }
                    });
                    w::row(ui, &p, "Пауза", "стадии 2 и 4", |ui| {
                        if let Some(v) = w::eng_field(ui, &p, "lfm-h", r.t_hold, "s", 1e-8, 100.0, 140.0) {
                            r.t_hold = v;
                        }
                    });
                    w::row(ui, &p, "Форма", "", |ui| {
                        let cur = r.shape.as_str();
                        if let Some(v) = w::segmented_sized(
                            ui,
                            &p,
                            cur,
                            &[("saw", "Пила"), ("tri", "Треугольник"), ("single", "Однократно")],
                            true,
                        ) {
                            r.shape = v.to_string();
                        }
                    });
                    w::row(ui, &p, "Выключать в паузах", "импульсный ЛЧМ", |ui| {
                        w::toggle(ui, &p, &mut r.pause_off);
                    });
                    ui.horizontal_wrapped(|ui| {
                        if w::button(ui, &p, "💾 Записать", w::Btn::Primary).clicked() {
                            sh.rpc("lfm_apply", json!({ "req": r }));
                        }
                        if w::button_en(ui, &p, "▶ Старт", w::Btn::Subtle, on).clicked() {
                            sh.rpc("lfm_start", json!({}));
                        }
                        if w::button_en(ui, &p, "⏸ Стоп", w::Btn::Subtle, on).clicked() {
                            sh.rpc("lfm_stop", json!({}));
                        }
                        if w::button_en(ui, &p, "✕ Выключить", w::Btn::Ghost, on).clicked() {
                            sh.rpc("lfm_off", json!({}));
                        }
                    });
                    if on {
                        w::badge(ui, &p, "ЛЧМ включён (CH1_LS_on = 1)", p.ok);
                    } else {
                        w::badge_muted(ui, &p, "ЛЧМ выключен — работают профили");
                    }
                },
            );
            if let Some(pl) = plan {
                let cells: [(&str, String, Option<Color32>, &str); 6] = [
                    ("Скорость", eng(pl.slope_hz_s, "Hz", 4) + "/с", None, ""),
                    ("Шаг скорости", eng(pl.slope_step, "Hz", 3) + "/с", None, ""),
                    ("Фактический конец", eng(pl.f_end, "Hz", 7), Some(p.sig), ""),
                    ("Период", eng(pl.period, "s", 4), None, ""),
                    ("dF1", grouped(pl.df1 as i128), None, ""),
                    ("TPH1", grouped(pl.tph[0] as i128), None, "× 4 такта"),
                ];
                w::auto_grid(ui, 150.0, 6, |ui, i| {
                    let c = &cells[i];
                    w::stat(ui, &p, c.0, &c.1, c.2, c.3);
                });
            }
        },
    );
}

fn constellation(ui: &mut Ui, p: &crate::theme::Pal, plan: &ModPlan) {
    let r = 130.0;
    let max = plan.symbols.iter().map(|s| s.i.hypot(s.q)).fold(1e-9, f64::max);
    let fsk = plan.kind.starts_with("fsk");
    ui.vertical_centered(|ui| {
        ui.set_max_width(340.0);
        let (c, _) = Canvas::new(ui, [-160.0, -160.0, 320.0, 320.0], 340.0);
        c.circle(0.0, 0.0, r, Color32::TRANSPARENT, Stroke::NONE);
        let mut ring = vec![];
        for k in 0..=96 {
            let a = k as f32 / 96.0 * std::f32::consts::TAU;
            ring.push((r * a.cos(), r * a.sin()));
        }
        c.dashed(&ring, p.line, 1.0, 3.0, 4.0, 0.0);
        c.line(&[(-150.0, 0.0), (150.0, 0.0)], p.line2, 1.0);
        c.line(&[(0.0, -150.0), (0.0, 150.0)], p.line2, 1.0);
        c.text(146.0, -6.0, Align2::RIGHT_BOTTOM, "I", 11.0, p.muted);
        c.text(6.0, -146.0, Align2::LEFT_TOP, "Q", 11.0, p.muted);
        if fsk {
            c.text(0.0, 4.0, Align2::CENTER_CENTER, "ЧМн: символы различаются частотой", 12.0, p.muted);
        } else {
            for s in &plan.symbols {
                let x = (s.i / max) as f32 * r;
                let y = -(s.q / max) as f32 * r;
                c.circle(x, y, 6.0, p.sig, Stroke::NONE);
                c.text(x, y - 10.0, Align2::CENTER_BOTTOM, &s.bits, 10.0, p.fg2);
            }
        }
    });
}

fn keying(ui: &mut Ui, sh: &Shared, st: &mut ModState) {
    let p = sh.pal;
    let plan = match sh.query("mod", key((format!("{:?}", st.req), sh.cfg)), Query::Mod(st.req.clone())) {
        Some(Data::Mod(pl)) => Some(pl),
        _ => None,
    };
    let fsk = st.req.kind.starts_with("fsk");
    w::two_cols(
        ui,
        380.0,
        true,
        st,
        |ui, _| {
            let Some(pl) = plan else { return };
            let side = 360.0;
            let wide = ui.available_width() >= side + 16.0 + 340.0;
            let symbols = |ui: &mut Ui| {
                w::card(ui, &p, "Символы → профили", "", |ui| {
                    egui::ScrollArea::vertical().max_height(360.0).min_scrolled_height(360.0).id_salt("sym").show(
                        ui,
                        |ui| {
                            egui::Grid::new("sym-grid").num_columns(5).spacing([14.0, 5.0]).show(ui, |ui| {
                                for h in ["Профиль", "Биты", "Частота", "Фаза", "Mul"] {
                                    ui.label(RichText::new(h).size(11.0).color(p.muted));
                                }
                                ui.end_row();
                                for s in &pl.symbols {
                                    ui.label(RichText::new(s.profile.to_string()).size(12.0));
                                    ui.label(RichText::new(&s.bits).monospace().size(12.0));
                                    ui.label(RichText::new(eng(s.f, "Hz", 7)).size(12.0));
                                    ui.label(RichText::new(format!("{}°", fixed(s.phase_deg, 1))).size(12.0));
                                    ui.label(RichText::new(s.mul.to_string()).size(12.0));
                                    ui.end_row();
                                }
                            });
                        },
                    );
                });
            };
            let cons = |ui: &mut Ui| {
                w::card(
                    ui,
                    &p,
                    &pl.title,
                    &format!("{} бит на символ, код Грея", pl.bits_per_symbol),
                    |ui| constellation(ui, &p, pl),
                );
            };
            if wide {
                w::two_cols_min(ui, side, 340.0, true, &mut (), |ui, _| symbols(ui), |ui, _| cons(ui));
            } else {
                cons(ui);
                symbols(ui);
            }
            w::card(
                ui,
                &p,
                "Пример последовательности",
                "Символы из псевдослучайной последовательности: так менялся бы Pr_1 при передаче",
                |ui| {
                    let x: Vec<f64> = (0..pl.sequence.len()).flat_map(|i| [i as f64, i as f64 + 1.0]).collect();
                    let y: Vec<f64> = pl.sequence.iter().flat_map(|v| [*v as f64, *v as f64]).collect();
                    Plot::new("mod-seq", vec![Series::new("профиль", x, y, p.dig).width(2.0)])
                        .height(180.0)
                        .x_fmt(|v| format!("{:.0}", v))
                        .y_fmt(|v| format!("{:.0}", v))
                        .labels("номер символа", "Pr_1", "")
                        .show(ui, &p);
                },
            );
        },
        |ui, st| {
            w::card(ui, &p, "Схема манипуляции", "Символ = профиль канала 1", |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let r = &mut st.req;
                w::row(ui, &p, "Вид", "", |ui| {
                    let opts: Vec<(String, String)> =
                        KINDS.iter().map(|k| (k.0.to_string(), k.1.to_string())).collect();
                    if let Some(k) = w::select(ui, "mod-kind", &r.kind, &opts, 190.0) {
                        r.kind = k;
                    }
                });
                w::row(ui, &p, "Несущая", "", |ui| {
                    if let Some(v) = w::eng_field(ui, &p, "mod-f", r.f_center, "Hz", 0.0, 240e6, 140.0) {
                        r.f_center = v;
                    }
                });
                if fsk {
                    w::row(ui, &p, "Разнос частот", "между соседними", |ui| {
                        if let Some(v) = w::eng_field(ui, &p, "mod-d", r.deviation, "Hz", 0.0, 20e6, 140.0) {
                            r.deviation = v;
                        }
                    });
                }
                w::row(ui, &p, "Пиковый Mul", "", |ui| {
                    if let Some(v) = w::num_field(ui, &p, "mod-mul", r.mul as f64, 1.0, 4095.0, 64.0, true, "", 56.0) {
                        r.mul = v as u32;
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    if w::button(ui, &p, "💾 Записать профили", w::Btn::Primary).clicked() {
                        sh.rpc("mod_apply", json!({ "req": r }));
                    }
                    if let Some(pl) = plan {
                        let n = pl.symbols.len();
                        if w::button(ui, &p, &format!("🔀 Перебор символов ({})", n), w::Btn::Subtle)
                            .on_hover_text("Прошивка перебирает символы по кругу")
                            .clicked()
                        {
                            sh.rpc("hop", json!({ "n": n, "ms": st.ms as u32 }));
                        }
                    }
                });
                w::row(ui, &p, "Длительность символа", "для перебора", |ui| {
                    if let Some(v) = w::num_field(ui, &p, "mod-ms", st.ms, 1.0, 60000.0, 50.0, true, "мс", 56.0) {
                        st.ms = v;
                    }
                });
            });
            if let Some(pl) = plan {
                for n in &pl.notes {
                    w::hint_text(ui, &p, p.accent, n);
                }
            }
        },
    );
}
