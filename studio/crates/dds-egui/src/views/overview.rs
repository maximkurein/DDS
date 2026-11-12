//! «Обзор кристалла»: the live block diagram (`OverviewView.tsx`).

use crate::app::{Act, Shared};
use crate::canvas::Canvas;
use crate::content::{block, Block, BLOCKS, WIRES};
use crate::fmt::eng4;
use crate::theme::mix;
use crate::widgets as w;
use dds_core::api::{CHIP_NAME, CHIP_VENDOR};
use dds_core::regs::{Access, FIELDS};
use eframe::egui::{self, Align2, Color32, RichText, Stroke, Ui};

fn anchor(b: &Block, side: char) -> (f32, f32) {
    match side {
        'l' => (b.x, b.y + b.h / 2.0),
        'r' => (b.x + b.w, b.y + b.h / 2.0),
        't' => (b.x + b.w / 2.0, b.y),
        _ => (b.x + b.w / 2.0, b.y + b.h),
    }
}

/// Orthogonal wire between two blocks (`wirePath`).
fn wire_path(a: &Block, b: &Block) -> Vec<(f32, f32)> {
    let vertical = (a.x + a.w / 2.0 - (b.x + b.w / 2.0)).abs() < 40.0;
    if vertical {
        let (x1, y1) = anchor(a, if a.y < b.y { 'b' } else { 't' });
        let (_, y2) = anchor(b, if a.y < b.y { 't' } else { 'b' });
        return vec![(x1, y1), (x1, y2)];
    }
    let (x1, y1) = anchor(a, 'r');
    let (x2, y2) = anchor(b, 'l');
    if x2 < x1 {
        let (tx, ty) = anchor(a, if b.y < a.y { 't' } else { 'b' });
        let (ux, uy) = anchor(b, if b.y < a.y { 'b' } else { 't' });
        let my = (ty + uy) / 2.0;
        return vec![(tx, ty), (tx, my), (ux, my), (ux, uy)];
    }
    let mx = (x1 + x2) / 2.0;
    vec![(x1, y1), (mx, y1), (mx, y2), (x2, y2)]
}

fn diagram(ui: &mut Ui, sh: &Shared) {
    let p = sh.pal;
    let (c, _) = Canvas::new(ui, [60.0, 118.0, 1120.0, 490.0], 640.0);
    let active = sh.st.sim.as_ref().map(|s| s.state != "off").unwrap_or(true);
    c.rect(80.0, 170.0, 925.0, 420.0, 28.0, p.panel2, Stroke::new(2.0, p.line2));
    c.text_bold(104.0, 204.0, Align2::LEFT_BOTTOM, format!("{} {}", CHIP_VENDOR, CHIP_NAME), 15.0, p.muted);
    c.dashed(
        &[
            (1028.0, 214.0),
            (1142.0, 214.0),
            (1160.0, 232.0),
            (1160.0, 556.0),
            (1142.0, 574.0),
            (1028.0, 574.0),
            (1010.0, 556.0),
            (1010.0, 232.0),
            (1028.0, 214.0),
        ],
        p.line2,
        1.0,
        6.0,
        6.0,
        0.0,
    );
    c.text(1085.0, 590.0, Align2::CENTER_BOTTOM, "плата", 12.0, p.muted);
    let off = (sh.time as f32 * 16.0 / 1.2) % 16.0;
    for (from, to, tone) in WIRES {
        let (Some(a), Some(b)) = (block(from), block(to)) else { continue };
        let pts = wire_path(a, b);
        let col = p.tone(tone);
        c.line(&pts, col.gamma_multiply(0.35), 5.0);
        if active {
            c.dashed(&pts, col, 2.5, 4.0, 12.0, 16.0 - off);
        }
    }
    if active {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    }
    for b in BLOCKS {
        let sel = sh.block == Some(b.id);
        let iss = sh.st.derived.issues.iter().find(|i| i.block == b.id && i.level != "info");
        let metric = sh.st.derived.metrics.iter().find(|m| m.block == b.id);
        let tone = p.tone(b.tone);
        let stroke = match iss {
            Some(i) if i.level == "error" => p.err,
            Some(_) => p.warn,
            None => tone,
        };
        c.rect(
            b.x,
            b.y,
            b.w,
            b.h,
            16.0,
            mix(tone, if sel { 22.0 } else { 10.0 }, p.panel),
            Stroke::new(if sel { 3.0 } else { 1.6 }, stroke),
        );
        let ty = b.y + b.h / 2.0 - if metric.is_some() { 6.0 } else { -5.0 };
        c.text_bold(b.x + b.w / 2.0, ty, Align2::CENTER_BOTTOM, b.title, 16.0, p.fg);
        if let Some(m) = metric {
            c.text(b.x + b.w / 2.0, b.y + b.h / 2.0 + 18.0, Align2::CENTER_BOTTOM, eng4(m.value, m.unit), 13.0, tone);
        }
        if c.hit(ui, b.id, b.x, b.y, b.w, b.h).on_hover_text(b.desc).clicked() {
            sh.act(Act::SelectBlock(if sel { None } else { Some(b.id) }));
        }
    }
    c.text(80.0, 140.0, Align2::LEFT_BOTTOM, "Нажмите на блок, чтобы увидеть его параметры и регистры", 13.0, p.muted);
}

/// Only the active profile's fields of the profile blocks (16 × 7 otherwise).
fn prof_ok(sh: &Shared, name: &str) -> bool {
    let Some(rest) = name.strip_prefix("CH") else { return true };
    let mut ch = rest.chars();
    let Some(n) = ch.next().filter(|c| *c == '1' || *c == '2') else { return true };
    let tail = ch.as_str();
    let Some(tail) = tail.strip_prefix('_') else { return true };
    for pre in ["dPh", "P", "Mul", "Offset"] {
        if let Some(d) = tail.strip_prefix(pre) {
            let digits: String = d.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() {
                continue;
            }
            let y: u32 = digits.parse().unwrap_or(0);
            let active = sh.field(if n == '1' { "Pr_1" } else { "Pr_2" });
            return y == active;
        }
    }
    true
}

pub fn show(ui: &mut Ui, sh: &Shared) {
    let p = sh.pal;
    w::page_header(
        ui,
        &p,
        "🔲",
        "Обзор кристалла",
        &format!(
            "{} {} на плате {}. Блок-схема живая: значения пересчитываются моделью при каждом изменении регистров.",
            CHIP_VENDOR,
            CHIP_NAME,
            dds_core::api::BOARD_NAME
        ),
        |_| {},
    );
    let b = sh.block.and_then(block);
    let d = &sh.st.derived;
    w::two_cols(
        ui,
        420.0,
        false,
        &mut (),
        |ui, _| {
            w::card_frame(&p).inner_margin(8).show(ui, |ui| {
                ui.set_width(ui.available_width());
                diagram(ui, sh);
            });
            w::auto_grid(ui, 160.0, d.metrics.len(), |ui, i| {
                let m = &d.metrics[i];
                let tone = block(m.block).map(|b| p.tone(b.tone));
                w::stat(ui, &p, m.label, &eng4(m.value, m.unit), tone, "");
            });
        },
        |ui, _| {
            let (title, sub) = match b {
                Some(b) => (b.title, b.desc),
                None => ("Все блоки", "Выберите блок на схеме"),
            };
            w::card(ui, &p, title, sub, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for i in d.issues.iter().filter(|i| b.is_none_or(|b| i.block == b.id)) {
                    let (c, icon) = match i.level {
                        "error" => (p.err, "⊗"),
                        "warn" => (p.warn, "⚠"),
                        _ => (p.accent, "ℹ"),
                    };
                    w::hint(ui, &p, c, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new(icon).color(c).size(13.0));
                            ui.add(egui::Label::new(RichText::new(&i.text).size(12.0).color(p.fg2)).wrap());
                        });
                    });
                }
                match b {
                    Some(b) => {
                        let ms: Vec<_> = d.metrics.iter().filter(|m| m.block == b.id).collect();
                        if !ms.is_empty() {
                            w::auto_grid(ui, 150.0, ms.len(), |ui, i| {
                                w::stat(ui, &p, ms[i].label, &eng4(ms[i].value, ms[i].unit), Some(p.tone(b.tone)), "");
                            });
                        }
                        let fields: Vec<_> = FIELDS.iter().filter(|f| f.block == b.id && prof_ok(sh, f.name)).collect();
                        for f in fields.iter().filter(|f| f.access == Access::Rw) {
                            w::field_control(ui, sh, f, false, sh.focus == Some(f.name));
                        }
                        for f in fields.iter().filter(|f| f.access != Access::Rw) {
                            w::field_control(ui, sh, f, true, false);
                        }
                    }
                    None => w::hint_text(
                        ui,
                        &p,
                        p.accent,
                        "Выберите блок на схеме: здесь появятся его регистры. Полная карта — на вкладке «Регистры».",
                    ),
                }
            });
        },
    );
    let _ = Color32::TRANSPARENT;
}
