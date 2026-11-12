//! «Академия»: lessons with a «try it» button (`LearnView.tsx`).

use crate::app::Shared;
use crate::content::LESSONS;
use crate::widgets as w;
use eframe::egui::{self, RichText, Stroke, Ui};

pub fn show(ui: &mut Ui, sh: &Shared, cur: &mut usize) {
    let p = sh.pal;
    w::page_header(
        ui,
        &p,
        "🎓",
        "Академия",
        "Как устроен кристалл — от принципа работы до настройки регистров. Каждый урок можно сразу попробовать в приложении.",
        |_| {},
    );
    ui.set_max_width(ui.available_width().min(1200.0));
    *cur = (*cur).min(LESSONS.len() - 1);
    w::two_cols(
        ui,
        260.0,
        true,
        cur,
        |ui, cur| {
            let l = &LESSONS[*cur];
            w::card(ui, &p, "", "", |ui| {
                ui.label(RichText::new(l.title).strong().size(15.0));
                ui.add_space(6.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                for para in l.body {
                    if let Some(h) = para.strip_prefix("§ ") {
                        ui.add_space(4.0);
                        ui.label(RichText::new(h).strong().size(14.0));
                    } else if let Some(f) = para.strip_prefix("$ ") {
                        egui::Frame::new()
                            .fill(p.panel2)
                            .stroke(Stroke::new(1.0, p.line))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.add(
                                    egui::Label::new(RichText::new(f).monospace().size(13.0).color(p.accent))
                                        .wrap()
                                        .selectable(true),
                                );
                            });
                    } else if let Some(b) = para.strip_prefix("- ") {
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new("•").color(p.muted));
                            ui.add(egui::Label::new(RichText::new(b).size(13.5).color(p.fg2)).wrap().selectable(true));
                        });
                    } else {
                        ui.add(egui::Label::new(RichText::new(*para).size(13.5).color(p.fg2)).wrap().selectable(true));
                    }
                }
                if let Some((label, view, block)) = l.try_it {
                    ui.add_space(4.0);
                    if w::button(ui, &p, &format!("➜ {}", label), w::Btn::Primary).clicked() {
                        sh.go_block(view, block);
                    }
                }
            });
        },
        |ui, cur| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for (i, l) in LESSONS.iter().enumerate() {
                let on = i == *cur;
                let r = egui::Frame::new()
                    .fill(if on { p.panel } else { egui::Color32::TRANSPARENT })
                    .stroke(if on { Stroke::new(1.0, p.line) } else { Stroke::NONE })
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new(format!("{}.", i + 1)).size(12.5).color(p.muted));
                            let t = RichText::new(l.title).size(12.5).color(if on { p.fg } else { p.fg2 });
                            ui.add(egui::Label::new(if on { t.strong() } else { t }).wrap());
                        });
                    })
                    .response
                    .interact(egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if r.clicked() {
                    *cur = i;
                }
            }
        },
    );
}
