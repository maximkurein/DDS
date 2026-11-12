//! «Профили и перебор частот» (`HopView.tsx`).

use crate::app::{key, Act, Shared};
use crate::fmt::eng;
use crate::widgets as w;
use crate::worker::{Data, Query};
use dds_core::regs::N_PROF1;
use eframe::egui::{self, RichText, Stroke, Ui};
use serde_json::json;

pub struct HopState {
    pub n: usize,
    pub ms: f64,
    pub g_lo: f64,
    pub g_hi: f64,
}

impl Default for HopState {
    fn default() -> Self {
        HopState { n: 8, ms: 250.0, g_lo: 48.5e6, g_hi: 51.5e6 }
    }
}

pub fn show(ui: &mut Ui, sh: &Shared, st: &mut HopState) {
    let p = sh.pal;
    let rows = match sh.query("profiles", key(("prof", sh.cfg)), Query::Profiles(1)) {
        Some(Data::Profiles(r)) => Some(r),
        _ => None,
    };
    let pr = sh.field("Pr_1") as usize;
    let live = sh.st.live_profile.map(|v| v as usize).unwrap_or(pr);
    let board = sh.st.board.as_ref();
    let hopping = board.map(|b| b.hop_n).unwrap_or(0) >= 2;
    w::page_header(
        ui,
        &p,
        "🔀",
        "Профили и перебор частот",
        &format!(
            "Канал 1, профили 0…{} (в кристалле их 64). Выбор профиля — одна запись SEL_REG: частота, фаза и амплитуда меняются за такт. Перебор ведёт прошивка платы (команда HOP), ПК только задаёт профили.",
            N_PROF1 - 1
        ),
        |_| {},
    );
    w::two_cols(
        ui,
        380.0,
        false,
        st,
        |ui, _| {
            w::card(
                ui,
                &p,
                "Профили канала 1",
                "Щелчок по номеру — сделать профиль активным. Поля применяются по Enter.",
                |ui| {
                    let Some(rows) = rows else { return };
                    let fclk = sh.st.derived.calc.fclk;
                    egui::ScrollArea::horizontal().id_salt("hop-x").show(ui, |ui| {
                        egui::Grid::new("hop-grid").num_columns(6).spacing([10.0, 4.0]).min_row_height(28.0).show(
                            ui,
                            |ui| {
                                for h in ["#", "Частота", "Фаза, °", "Mul", "Offset", "dPh"] {
                                    ui.label(RichText::new(h).size(11.0).color(p.muted));
                                }
                                ui.end_row();
                                for r in rows {
                                    let (fill, fg, stroke) = if r.y == live {
                                        (p.accent, p.accent_fg, p.accent)
                                    } else if r.y == pr {
                                        (egui::Color32::TRANSPARENT, p.accent, p.accent)
                                    } else {
                                        (egui::Color32::TRANSPARENT, p.fg, p.line)
                                    };
                                    let b = egui::Button::new(RichText::new(r.y.to_string()).size(12.0).color(fg))
                                        .fill(fill)
                                        .stroke(Stroke::new(1.0_f32, stroke))
                                        .corner_radius(6)
                                        .min_size(egui::vec2(32.0, 24.0));
                                    if ui.add(b).on_hover_text(format!("Pr_1 = {}", r.y)).clicked() {
                                        sh.rpc("select_profile", json!({ "ch": 1, "y": r.y }));
                                    }
                                    if let Some(f) =
                                        w::eng_field(ui, &p, ("hop-f", r.y), r.f, "Hz", 0.0, fclk / 2.0, 150.0)
                                    {
                                        sh.rpc("set_profile", json!({ "ch": 1, "y": r.y, "f": f }));
                                    }
                                    let ph = (r.phase_deg * 100.0).round() / 100.0;
                                    if let Some(v) =
                                        w::num_field(ui, &p, ("hop-p", r.y), ph, -360.0, 360.0, 45.0, false, "", 60.0)
                                    {
                                        sh.rpc("set_profile", json!({ "ch": 1, "y": r.y, "phase_deg": v }));
                                    }
                                    if let Some(v) = w::num_field(
                                        ui,
                                        &p,
                                        ("hop-m", r.y),
                                        r.mul as f64,
                                        -4096.0,
                                        4095.0,
                                        64.0,
                                        true,
                                        "",
                                        52.0,
                                    ) {
                                        sh.rpc("set_profile", json!({ "ch": 1, "y": r.y, "mul": v as i64 }));
                                    }
                                    if let Some(v) = w::num_field(
                                        ui,
                                        &p,
                                        ("hop-o", r.y),
                                        r.offset as f64,
                                        -2048.0,
                                        2047.0,
                                        1.0,
                                        true,
                                        "",
                                        52.0,
                                    ) {
                                        sh.rpc("set_profile", json!({ "ch": 1, "y": r.y, "offset": v as i64 }));
                                    }
                                    ui.label(
                                        RichText::new(format!("{:012X}", r.ftw)).monospace().size(11.0).color(p.muted),
                                    );
                                    ui.end_row();
                                }
                            },
                        );
                    });
                },
            );
        },
        |ui, st| {
            w::card(
                ui,
                &p,
                "🔀 Перебор прошивкой",
                "Профили 0…n−1 по кругу, шаг задаёт таймер прошивки",
                |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    w::row(ui, &p, "Профилей n", "", |ui| {
                        if let Some(v) =
                            w::num_field(ui, &p, "hop-n", st.n as f64, 2.0, N_PROF1 as f64, 1.0, true, "", 56.0)
                        {
                            st.n = v as usize;
                        }
                    });
                    w::row(ui, &p, "На профиле", "", |ui| {
                        if let Some(v) = w::num_field(ui, &p, "hop-ms", st.ms, 1.0, 60000.0, 10.0, true, "мс", 56.0) {
                            st.ms = v;
                        }
                    });
                    ui.horizontal(|ui| {
                        if w::button(ui, &p, "▶ Запустить", w::Btn::Primary).clicked() {
                            sh.rpc("hop", json!({ "n": st.n, "ms": st.ms as u32 }));
                        }
                        if w::button_en(ui, &p, "■ Остановить", w::Btn::Subtle, hopping).clicked() {
                            sh.rpc("hop", json!({ "n": 0 }));
                        }
                    });
                    ui.add_space(4.0);
                    let sub = rows.and_then(|r| r.get(live)).map(|r| eng(r.f, "Hz", 7)).unwrap_or_default();
                    w::cols(ui, 2, 120.0, |ui, i| {
                        if i == 0 {
                            w::stat(ui, &p, "Сейчас играет", &format!("профиль {}", live), Some(p.sig), &sub);
                        } else {
                            let v = match board {
                                Some(b) if hopping => format!("{} × {} мс", b.hop_n, b.hop_ms),
                                _ => "выкл.".into(),
                            };
                            w::stat(ui, &p, "Перебор", &v, None, "");
                        }
                    });
                    if hopping {
                        w::badge(ui, &p, "прошивка переключает SEL_REG.Pr_1", p.ok);
                    }
                    w::hint_text(
                    ui,
                    &p,
                    p.accent,
                    &format!("Остановка возвращает профиль, выбранный в приложении (Pr_1 = {}). Перебор не мешает менять неактивные профили — так строится «прыгающая» частота с новой сеткой на лету.", pr),
                );
                },
            );
            w::card(
                ui,
                &p,
                "▦ Заполнить сеткой",
                &format!("Профили 0…{} равномерно от и до", st.n - 1),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    w::row(ui, &p, "От", "", |ui| {
                        if let Some(v) = w::eng_field(ui, &p, "g-lo", st.g_lo, "Hz", 0.0, 240e6, 140.0) {
                            st.g_lo = v;
                        }
                    });
                    w::row(ui, &p, "До", "", |ui| {
                        if let Some(v) = w::eng_field(ui, &p, "g-hi", st.g_hi, "Hz", 0.0, 240e6, 140.0) {
                            st.g_hi = v;
                        }
                    });
                    let label = if sh.busy_shown {
                        "Запись…".to_string()
                    } else {
                        format!("Записать {} частот", st.n)
                    };
                    if w::button_en(ui, &p, &label, w::Btn::Subtle, !sh.busy_shown).clicked() {
                        let n = st.n;
                        let calls = (0..n)
                            .map(|y| {
                                let f = if n > 1 {
                                    st.g_lo + (st.g_hi - st.g_lo) * y as f64 / (n - 1) as f64
                                } else {
                                    st.g_lo
                                };
                                ("set_profile", json!({ "ch": 1, "y": y, "f": f }))
                            })
                            .collect();
                        sh.act(Act::Batch(calls, None));
                    }
                },
            );
        },
    );
}
