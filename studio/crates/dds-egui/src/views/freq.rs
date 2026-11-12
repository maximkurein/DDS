//! «Частота и уровень»: frequency planner, DIP code, amplitude and board
//! parameters (`FreqView.tsx`).

use crate::app::{key, Shared};
use crate::fmt::{eng, fixed, grouped, hex_w};
use crate::widgets as w;
use crate::worker::{Data, Query};
use dds_core::model::FreqPlan;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::json;

#[derive(Default)]
pub struct FreqState {
    pub f: Option<f64>,
    /// Slider draft and the Mul it was taken from.
    mul_draft: Option<(i64, i64)>,
}

/// DIP switch bank: ON = bit 1 (the lever is down).
fn dip_bank(ui: &mut Ui, p: &crate::theme::Pal, title: &str, bits: &[bool], offset: usize) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.label(RichText::new(title).size(11.0).color(p.muted));
        let n = bits.len() as f32;
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(12.0 + n * 20.0 + (n - 1.0) * 4.0, 66.0), egui::Sense::hover());
        let pt = ui.painter();
        pt.rect_filled(rect, 8.0, Color32::from_rgb(0xc0, 0x3a, 0x2b));
        for (i, on) in bits.iter().enumerate() {
            let x = rect.left() + 6.0 + i as f32 * 24.0;
            let slot = egui::Rect::from_min_size(egui::pos2(x, rect.top() + 6.0), egui::vec2(20.0, 40.0));
            pt.rect_filled(slot, 3.0, Color32::from_rgb(0xf4, 0xef, 0xe6));
            let lever = if *on {
                egui::Rect::from_min_size(egui::pos2(slot.left() + 2.0, slot.bottom() - 18.0), egui::vec2(16.0, 16.0))
            } else {
                egui::Rect::from_min_size(egui::pos2(slot.left() + 2.0, slot.top() + 2.0), egui::vec2(16.0, 16.0))
            };
            pt.rect_filled(lever, 2.0, Color32::from_rgb(0x3b, 0x3b, 0x3b));
            pt.text(
                egui::pos2(slot.center().x, slot.bottom() + 8.0),
                egui::Align2::CENTER_CENTER,
                (i + 1).to_string(),
                egui::FontId::proportional(9.0),
                Color32::from_white_alpha(230),
            );
        }
        ui.label(
            RichText::new(format!("биты {}…{} · ON = 1", offset, offset + bits.len() - 1)).size(10.0).color(p.muted),
        );
    });
}

pub fn show(ui: &mut Ui, sh: &Shared, st: &mut FreqState) {
    let p = sh.pal;
    let c = &sh.st.derived.calc;
    let set = &sh.st.settings;
    let main = if c.ch1.on_dac1 { &c.ch1 } else { &c.ch2 };
    let f = *st.f.get_or_insert(if main.fout > 0.0 { main.fout.round() } else { 50e6 });
    let plan: Option<FreqPlan> = match sh.query("plan", key((f.to_bits(), sh.cfg)), Query::Plan(f)) {
        Some(Data::Plan(pl)) => Some(pl.clone()),
        _ => None,
    };
    let pr = sh.field("Pr_1");
    let mul_name = format!("CH1_Mul{}", pr.min(dds_core::regs::N_PROF1 as u32 - 1));
    let raw = sh.field(&mul_name) as i64;
    let mul = if raw >= 4096 { raw - 8192 } else { raw };
    if st.mul_draft.is_none_or(|d| d.1 != mul) {
        st.mul_draft = Some((mul, mul));
    }
    let apply = |x: f64| sh.rpc("set_frequency", json!({ "f": x, "seamless": true }));

    w::page_header(
        ui,
        &p,
        "📻",
        "Частота и уровень",
        "Планировщик частоты: приращение фазы dPh, ошибка сетки, код DIP-переключателей и ожидаемый уровень на XW2. «Установить» меняет частоту бесшовно — через неактивный профиль 0/1.",
        |_| {},
    );
    w::two_cols(
        ui,
        420.0,
        false,
        st,
        |ui, st| {
            w::card(ui, &p, "🎯 Желаемая частота", "", |ui| {
                ui.horizontal_wrapped(|ui| {
                    if let Some(x) = w::eng_field_ex(ui, &p, "freq-f", f, "Hz", 0.0, c.fclk / 2.0, 240.0, true) {
                        st.f = Some(x);
                    }
                    let nudge = |k: f64| (set.band_lo_hz - 2e6).max(f + k * set.step_hz);
                    if w::button(ui, &p, "−", w::Btn::Subtle)
                        .on_hover_text(format!("−{} и установить", eng(set.step_hz, "Hz", 4)))
                        .clicked()
                    {
                        let x = nudge(-1.0);
                        st.f = Some(x);
                        apply(x);
                    }
                    if w::button(ui, &p, "+", w::Btn::Subtle)
                        .on_hover_text(format!("+{} и установить", eng(set.step_hz, "Hz", 4)))
                        .clicked()
                    {
                        let x = nudge(1.0);
                        st.f = Some(x);
                        apply(x);
                    }
                    if w::button(ui, &p, "➜ Установить", w::Btn::Primary).clicked() {
                        apply(f);
                    }
                    ui.add_space(16.0);
                    let opts: Vec<(u64, String)> =
                        [48u64, 49, 50, 51, 52].iter().map(|m| (m * 1_000_000, m.to_string())).collect();
                    let cur = if f.fract() == 0.0 { f as u64 } else { 0 };
                    if let Some(x) = w::segmented(ui, &p, cur, &opts) {
                        st.f = Some(x as f64);
                        apply(x as f64);
                    }
                    ui.label(RichText::new("МГц").size(11.0).color(p.muted));
                });
                if let Some(pl) = &plan {
                    ui.add_space(8.0);
                    w::auto_grid(ui, 170.0, 4, |ui, i| match i {
                        0 => {
                            w::stat(ui, &p, "Будет на выходе", &eng(pl.actual, "Hz", 12), Some(p.sig), "");
                        }
                        1 => {
                            w::stat(
                                ui,
                                &p,
                                "Ошибка сетки",
                                &eng(pl.error_hz, "Hz", 3),
                                None,
                                &format!("шаг {}", eng(pl.resolution, "Hz", 4)),
                            );
                        }
                        2 => {
                            w::stat(
                                ui,
                                &p,
                                "dPh (48 бит)",
                                &format!("0x{}", pl.ftw_hex),
                                None,
                                &grouped(pl.ftw as i128),
                            );
                        }
                        _ => {
                            let ok = (pl.level_dbm - set.target_dbm).abs() < 1.0;
                            w::stat(
                                ui,
                                &p,
                                "Уровень на XW2",
                                &format!("{} дБм", fixed(pl.level_dbm, 2)),
                                Some(if ok { p.ok } else { p.warn }),
                                &format!("цель {} дБм", crate::fmt::sig(set.target_dbm, 6)),
                            );
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Регистры профиля:").size(12.0).color(p.muted));
                        for (i, n) in ["L", "M", "H"].iter().enumerate() {
                            w::badge(ui, &p, &format!("dPh_{} = {}", n, hex_w(pl.words[i] as u32, 16)), p.dig);
                        }
                        if !pl.in_band {
                            w::badge(
                                ui,
                                &p,
                                &format!(
                                    "вне полосы платы {}…{}",
                                    eng(set.band_lo_hz, "Hz", 4),
                                    eng(set.band_hi_hz, "Hz", 4)
                                ),
                                p.warn,
                            );
                        }
                    });
                }
            });
            w::card(
                ui,
                &p,
                "DIP-переключатели для этой частоты",
                "f = 48 МГц + 1 кГц · N. Так плата задаёт частоту без ПК (режим DIP).",
                |ui| {
                    match plan.as_ref().and_then(|pl| pl.dip_n.map(|n| (pl, n))) {
                    Some((pl, n)) => {
                        let mut bits = pl.dip_on.clone();
                        bits.resize(12, false);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 24.0;
                            dip_bank(ui, &p, "S2 — N[7:0]", &bits[..8], 0);
                            dip_bank(ui, &p, "S3 — N[11:8]", &bits[8..12], 8);
                            ui.vertical(|ui| {
                                ui.set_width(170.0);
                                w::stat(ui, &p, "Код N", &n.to_string(), None, &format!("0x{:03X}", n));
                                if sh.is_sim() && ui.add(w::small(&p, "⌖ Выставить на виртуальной плате", w::Btn::Subtle)).clicked() {
                                    sh.rpc("set_sim_dip", json!({ "value": n }));
                                }
                            });
                        });
                    }
                    None => w::hint_text(
                        ui,
                        &p,
                        p.warn,
                        "Частота не лежит на сетке 1 кГц в диапазоне 48–52 МГц (N 0…4000): переключателями её не задать, только с ПК.",
                    ),
                }
                },
            );
        },
        |ui, st| {
            w::card(
                ui,
                &p,
                &format!("Амплитуда профиля {}", pr),
                "A = Mul / 4096. Отрицательный Mul — инверсия (180°).",
                |ui| {
                    let draft = &mut st.mul_draft.as_mut().unwrap().0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().slider_width = (ui.available_width() - 100.0).max(120.0);
                        ui.add(egui::Slider::new(draft, 0..=4095).show_value(false));
                        if let Some(v) = w::num_field(ui, &p, "mul", mul as f64, -4096.0, 4095.0, 1.0, true, "", 56.0) {
                            sh.rpc("set_profile", json!({ "ch": 1, "y": pr, "mul": v as i64 }));
                        }
                    });
                    let d = *draft;
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(d != mul, w::small(&p, &format!("Применить {}", d), w::Btn::Primary))
                            .clicked()
                        {
                            sh.rpc("set_profile", json!({ "ch": 1, "y": pr, "mul": d }));
                        }
                        if let Some(t) = c.mul_for_target {
                            let label = format!("Mul для {} дБм ≈ {}", crate::fmt::sig(set.target_dbm, 6), t);
                            if ui.add(w::small(&p, &label, w::Btn::Subtle)).on_hover_text("По модели тракта").clicked()
                            {
                                sh.rpc("set_profile", json!({ "ch": 1, "y": pr, "mul": t }));
                            }
                        }
                    });
                    ui.add_space(4.0);
                    w::cols(ui, 2, 120.0, |ui, i| {
                        if i == 0 {
                            w::stat(
                                ui,
                                &p,
                                "Сейчас на XW2",
                                &format!("{} дБм", fixed(c.p_out_dbm, 2)),
                                Some(p.sig),
                                "",
                            );
                        } else {
                            w::stat(
                                ui,
                                &p,
                                "При полной шкале",
                                &format!("{} дБм", fixed(c.p_fullscale_dbm, 2)),
                                None,
                                "",
                            );
                        }
                    });
                    w::hint_text(
                    ui,
                    &p,
                    p.accent,
                    "Модель тракта — оценка. Точный Mul подбирайте по анализатору спектра (design_notes п. 5) и запишите его в DDS_AMPLITUDE_MUL прошивки.",
                );
                },
            );
            w::card(
                ui,
                &p,
                "Параметры платы",
                "Не регистры: то, что модель знает о плате и стенде",
                |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let s = |k: &str, v: f64| sh.set_settings(json!({ k: v }));
                    w::row(ui, &p, "Такт Fclk", "XW1, после S1", |ui| {
                        if let Some(v) = w::eng_field(ui, &p, "fclk", set.fclk_hz, "Hz", 1e6, 1.2e9, 140.0) {
                            s("fclk_hz", v);
                        }
                    });
                    w::row(ui, &p, "Уровень такта", "−10…+5 дБм", |ui| {
                        if let Some(v) =
                            w::num_field(ui, &p, "clk_dbm", set.clk_dbm, f64::MIN, f64::MAX, 0.5, false, "дБм", 56.0)
                        {
                            s("clk_dbm", v);
                        }
                    });
                    w::row(ui, &p, "Источник такта", "перемычка S1", |ui| {
                        if let Some(v) =
                            w::segmented_sized(ui, &p, set.csel, &[(1u8, "XW1 (S1 замкн.)"), (0u8, "G1")], true)
                        {
                            sh.set_settings(json!({ "csel": v }));
                        }
                    });
                    w::row(ui, &p, "Цель на XW2", "", |ui| {
                        if let Some(v) =
                            w::num_field(ui, &p, "target", set.target_dbm, f64::MIN, f64::MAX, 0.5, false, "дБм", 56.0)
                        {
                            s("target_dbm", v);
                        }
                    });
                    w::row(ui, &p, "RREF (R11)", "IFS = 72 В / RREF", |ui| {
                        if let Some(v) =
                            w::num_field(ui, &p, "rref", set.rref_ohm, 1000.0, 20000.0, 1.0, true, "Ом", 72.0)
                        {
                            s("rref_ohm", v);
                        }
                    });
                    w::row(ui, &p, "Добротность L", "катушки фильтра", |ui| {
                        if let Some(v) = w::num_field(ui, &p, "q", set.inductor_q, 2.0, 500.0, 1.0, true, "", 56.0) {
                            s("inductor_q", v);
                        }
                    });
                    w::row(ui, &p, "Шаг ±", "", |ui| {
                        if let Some(v) = w::eng_field(ui, &p, "step", set.step_hz, "Hz", 1e-6, 10e6, 140.0) {
                            s("step_hz", v);
                        }
                    });
                    w::row(ui, &p, "Дизеринг фазы", "ROUTE.pdith1", |ui| {
                        let mut on = sh.field("pdith1") == 1;
                        if w::toggle(ui, &p, &mut on).changed() {
                            sh.set_field("pdith1", on as u32);
                        }
                    });
                },
            );
        },
    );
}
