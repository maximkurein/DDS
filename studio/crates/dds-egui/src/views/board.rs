//! «Плата»: the board drawing with clickable parts, first power-up checklist
//! and safety notes (`BoardView.tsx`).

use crate::app::{Shared, View};
use crate::canvas::Canvas;
use crate::content::{BOARD_NAME, BOARD_SIZE, BOARD_SUPPLY, CHECKLIST, HOTSPOTS, SAFETY};
use crate::theme::{mix, Pal};
use crate::widgets as w;
use eframe::egui::{self, Align2, Color32, RichText, Stroke, Ui};

pub struct BoardState {
    pub sel: &'static str,
    pub done: Vec<bool>,
}

impl Default for BoardState {
    fn default() -> Self {
        BoardState { sel: HOTSPOTS[0].id, done: vec![false; CHECKLIST.len()] }
    }
}

const KINDS: [&str; 8] = ["chip", "connector", "jumper", "power", "testpoint", "led", "switch", "filter"];

fn kind(p: &Pal, k: &str) -> (&'static str, Color32) {
    match k {
        "chip" => ("кристалл", p.accent),
        "connector" => ("разъём", p.clk),
        "jumper" => ("джампер", p.warn),
        "power" => ("питание", p.pwr),
        "testpoint" => ("контр. точка", p.refc),
        "led" => ("индикатор", p.ok),
        "switch" => ("переключатель", p.ctl),
        _ => ("фильтр", p.refc),
    }
}

pub fn show(ui: &mut Ui, sh: &Shared, st: &mut BoardState) {
    let p = sh.pal;
    w::page_header(
        ui,
        &p,
        "📟",
        &format!("Плата {}", BOARD_NAME),
        &format!("{} · питание: {}. Нажмите на элемент платы, чтобы узнать его назначение.", BOARD_SIZE, BOARD_SUPPLY),
        |_| {},
    );
    w::two_cols(
        ui,
        400.0,
        false,
        st,
        |ui, st| {
            w::card_frame(&p).inner_margin(12).show(ui, |ui| {
                ui.set_width(ui.available_width());
                let (c, _) = Canvas::new(ui, [0.0, 0.0, 1000.0, 440.0], 620.0);
                c.rect(20.0, 20.0, 960.0, 400.0, 8.0, mix(p.ok, 12.0, p.panel2), Stroke::new(2.0_f32, p.line2));
                c.dashed(
                    &[(656.0, 392.0), (656.0, 300.0), (788.0, 300.0), (788.0, 268.0)],
                    p.clk.gamma_multiply(0.5),
                    3.0,
                    6.0,
                    5.0,
                    0.0,
                );
                c.line(&[(852.0, 236.0), (900.0, 236.0), (900.0, 318.0)], p.sig.gamma_multiply(0.5), 3.0);
                c.line(&[(938.0, 350.0), (944.0, 350.0)], p.sig.gamma_multiply(0.5), 3.0);
                c.line(&[(460.0, 170.0), (508.0, 170.0)], p.dig.gamma_multiply(0.45), 2.5);
                c.line(&[(548.0, 168.0), (548.0, 196.0), (724.0, 196.0)], p.dig.gamma_multiply(0.45), 2.5);
                c.text(990.0, 436.0, Align2::RIGHT_BOTTOM, "вид сверху, слой F.Cu · 120 × 50 мм", 11.0, p.muted);
                for h in HOTSPOTS {
                    let (_, col) = kind(&p, h.kind);
                    let on = h.id == st.sel;
                    c.rect(
                        h.x,
                        h.y,
                        h.w,
                        h.h,
                        8.0,
                        mix(col, if on { 35.0 } else { 16.0 }, p.panel),
                        Stroke::new(if on { 3.0 } else { 1.5_f32 }, col),
                    );
                    let size = if h.w < 44.0 { 10.0 } else { 13.0 };
                    c.text_bold(h.x + h.w / 2.0, h.y + h.h / 2.0, Align2::CENTER_CENTER, h.r#ref, size, p.fg);
                    if c.hit(ui, h.id, h.x, h.y, h.w, h.h).on_hover_text(h.title).clicked() {
                        st.sel = h.id;
                    }
                }
            });
        },
        |ui, st| {
            if let Some(h) = HOTSPOTS.iter().find(|h| h.id == st.sel) {
                w::card(ui, &p, &format!("{} — {}", h.r#ref, h.title), kind(&p, h.kind).0, |ui| {
                    ui.add(egui::Label::new(RichText::new(h.desc).size(12.5).color(p.fg2)).wrap());
                    if let Some(b) = h.block {
                        ui.add_space(6.0);
                        if ui.add(w::small(&p, "Показать на блок-схеме", w::Btn::Subtle)).clicked() {
                            sh.go_block(View::Overview, Some(b));
                        }
                    }
                });
            }
            w::card(
                ui,
                &p,
                "Первое включение",
                "Отмечайте шаги по мере выполнения",
                |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    for (i, c) in CHECKLIST.iter().enumerate() {
                        ui.horizontal_top(|ui| {
                            ui.checkbox(&mut st.done[i], "");
                            let mut t = RichText::new(*c).size(12.5);
                            t = if st.done[i] { t.color(p.muted).strikethrough() } else { t.color(p.fg2) };
                            if ui.add(egui::Label::new(t).wrap().sense(egui::Sense::click())).clicked() {
                                st.done[i] = !st.done[i];
                            }
                        });
                    }
                },
            );
            w::card(ui, &p, "🛡 Безопасность", "", |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for s in SAFETY {
                    w::hint_text(ui, &p, p.warn, s);
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for k in KINDS {
                    let (label, col) = kind(&p, k);
                    egui::Frame::new()
                        .fill(p.panel3)
                        .stroke(Stroke::new(1.0_f32, p.line))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(6, 1))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                let (r, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                                ui.painter().circle_filled(r.center(), 4.0, col);
                                ui.label(RichText::new(label).size(11.0).color(p.muted));
                            });
                        });
                }
            });
        },
    );
}
