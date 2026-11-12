//! Reusable widgets in the style of the web UI (`src/components/ui.tsx`):
//! cards, stats, hints, badges, segmented controls, engineering-unit fields,
//! and the metadata-driven register-field editor (`fields.tsx`).

use crate::app::{Act, Shared, View};
use crate::fmt::{eng, parse_eng};
use crate::theme::{mix, Pal};
use dds_core::regs::{Access, FieldDef, REGS};
use eframe::egui::{self, Color32, Response, RichText, Sense, Stroke, Ui};

pub fn card_frame(p: &Pal) -> egui::Frame {
    egui::Frame::new().fill(p.panel).stroke(Stroke::new(1.0, p.line)).corner_radius(14).inner_margin(14)
}

/// Card with an optional title and subtitle (icon glyphs go into the title).
pub fn card<R>(ui: &mut Ui, p: &Pal, title: &str, subtitle: &str, body: impl FnOnce(&mut Ui) -> R) -> R {
    card_frame(p)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            card_head(ui, p, title, subtitle);
            body(ui)
        })
        .inner
}

/// Card with a title row that has actions on the right.
pub fn card_actions<R>(
    ui: &mut Ui,
    p: &Pal,
    title: &str,
    subtitle: &str,
    actions: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    card_frame(p)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 120.0).max(120.0));
                    card_head(ui, p, title, subtitle);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), actions);
            });
            body(ui)
        })
        .inner
}

fn card_head(ui: &mut Ui, p: &Pal, title: &str, subtitle: &str) {
    if !title.is_empty() {
        ui.add(egui::Label::new(RichText::new(title).strong().size(13.0)).wrap());
    }
    if !subtitle.is_empty() {
        ui.add(egui::Label::new(RichText::new(subtitle).size(11.5).color(p.muted)).wrap());
    }
    if !title.is_empty() || !subtitle.is_empty() {
        ui.add_space(6.0);
    }
}

/// Page title with an icon, a subtitle and right-aligned actions. The actions'
/// width is remembered from the previous frame; when they would leave the
/// subtitle too little room they move to their own row under it.
pub fn page_header(ui: &mut Ui, p: &Pal, icon: &str, title: &str, subtitle: &str, actions: impl FnOnce(&mut Ui)) {
    let id = ui.id().with(("page-actions", title));
    let prev: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(0.0);
    let avail = ui.available_width();
    let inline = prev < (avail - 56.0) * 0.5;
    let mut used = prev;
    let mut actions = Some(actions);
    ui.horizontal_top(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::hover());
        ui.painter().rect(r, 12.0, p.panel, Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(18.0), p.accent);
        let text_w = if inline { (avail - 56.0 - prev - 24.0).max(320.0) } else { avail - 56.0 };
        ui.vertical(|ui| {
            ui.set_max_width(text_w);
            ui.label(RichText::new(title).size(20.0).strong());
            ui.add(egui::Label::new(RichText::new(subtitle).size(12.5).color(p.muted)).wrap());
        });
        if inline {
            let a = actions.take().unwrap();
            used = ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), a).response.rect.width();
        }
    });
    if let Some(a) = actions {
        ui.add_space(8.0);
        used =
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min).with_main_wrap(true), a).response.rect.width();
    }
    if (used - prev).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(id, used));
        ui.ctx().request_repaint();
    }
    ui.add_space(14.0);
}

/// Main column plus a fixed-width side column; stacks on narrow windows.
/// `side_left` puts the fixed column first (`380px_minmax(0,1fr)` in the web grid).
pub fn two_cols<S>(
    ui: &mut Ui,
    side_w: f32,
    side_left: bool,
    s: &mut S,
    main: impl FnOnce(&mut Ui, &mut S),
    side: impl FnOnce(&mut Ui, &mut S),
) {
    two_cols_min(ui, side_w, 520.0, side_left, s, main, side)
}

/// `two_cols` that stacks only when the main column would be below `main_min`.
pub fn two_cols_min<S>(
    ui: &mut Ui,
    side_w: f32,
    main_min: f32,
    side_left: bool,
    s: &mut S,
    main: impl FnOnce(&mut Ui, &mut S),
    side: impl FnOnce(&mut Ui, &mut S),
) {
    let w = ui.available_width();
    if w < side_w + main_min {
        if side_left {
            side(ui, s);
            ui.add_space(12.0);
            main(ui, s);
        } else {
            main(ui, s);
            ui.add_space(12.0);
            side(ui, s);
        }
        return;
    }
    let gap = 16.0;
    let mw = w - side_w - gap;
    let col = |ui: &mut Ui, width: f32, f: &mut dyn FnMut(&mut Ui)| {
        ui.allocate_ui_with_layout(egui::vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 14.0);
            f(ui)
        });
    };
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let mut main = Some(main);
        let mut side = Some(side);
        if side_left {
            col(ui, side_w, &mut |ui| (side.take().unwrap())(ui, s));
            ui.add_space(gap);
            col(ui, mw, &mut |ui| (main.take().unwrap())(ui, s));
        } else {
            col(ui, mw, &mut |ui| (main.take().unwrap())(ui, s));
            ui.add_space(gap);
            col(ui, side_w, &mut |ui| (side.take().unwrap())(ui, s));
        }
    });
}

/// Equal columns (`grid-cols-2`); stacks below `min_w` per column.
pub fn cols(ui: &mut Ui, n: usize, min_w: f32, mut cell: impl FnMut(&mut Ui, usize)) {
    let gap = 16.0;
    let w = ui.available_width();
    if w < n as f32 * min_w {
        for i in 0..n {
            cell(ui, i);
            ui.add_space(4.0);
        }
        return;
    }
    let cw = (w - gap * (n as f32 - 1.0)) / n as f32;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for i in 0..n {
            ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(cw);
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 14.0);
                cell(ui, i)
            });
        }
    });
}

/// Auto-fill grid: as many columns of at least `min_w` as fit.
pub fn auto_grid(ui: &mut Ui, min_w: f32, n: usize, mut cell: impl FnMut(&mut Ui, usize)) {
    if n == 0 {
        return;
    }
    let gap = 8.0;
    let cols = (((ui.available_width() + gap) / (min_w + gap)).floor() as usize).max(1);
    let cw = (ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32;
    ui.spacing_mut().item_spacing.y = gap;
    for row in 0..n.div_ceil(cols) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for c in 0..cols {
                let i = row * cols + c;
                if i >= n {
                    break;
                }
                ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(cw);
                    cell(ui, i)
                });
            }
        });
    }
}

/// Number tile: label, big value, optional sub-line.
pub fn stat(ui: &mut Ui, p: &Pal, label: &str, value: &str, tone: Option<Color32>, sub: &str) -> Response {
    egui::Frame::new()
        .fill(p.panel2)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(12, 7))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.add(egui::Label::new(RichText::new(label).size(11.0).color(p.muted)).truncate());
            ui.add(egui::Label::new(RichText::new(value).size(16.0).strong().color(tone.unwrap_or(p.fg))).truncate());
            if !sub.is_empty() {
                ui.add(egui::Label::new(RichText::new(sub).size(11.0).color(p.muted)).truncate());
            }
        })
        .response
}

/// Tinted note box.
pub fn hint(ui: &mut Ui, p: &Pal, tone: Color32, body: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(mix(tone, 8.0, p.panel))
        .stroke(Stroke::new(1.0, mix(tone, 28.0, p.panel)))
        .corner_radius(9)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui)
        });
}

pub fn hint_text(ui: &mut Ui, p: &Pal, tone: Color32, text: &str) {
    hint(ui, p, tone, |ui| {
        ui.add(egui::Label::new(RichText::new(text).size(12.0).color(p.fg2)).wrap());
    });
}

/// Small rounded tag.
pub fn badge(ui: &mut Ui, p: &Pal, text: &str, tone: Color32) -> Response {
    egui::Frame::new()
        .fill(mix(tone, 12.0, p.panel))
        .stroke(Stroke::new(1.0, mix(tone, 30.0, p.panel)))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(tone));
        })
        .response
}

/// Muted tag (`<Badge>` without a tone).
pub fn badge_muted(ui: &mut Ui, p: &Pal, text: &str) -> Response {
    egui::Frame::new()
        .fill(p.panel3)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(p.muted));
        })
        .response
}

/// Clickable rounded pill with a dot.
pub fn pill(ui: &mut Ui, p: &Pal, text: &str, tone: Color32, neutral: bool) -> Response {
    let fill = if neutral { p.panel } else { mix(tone, 10.0, p.panel) };
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, if neutral { p.line } else { mix(tone, 35.0, p.panel) }))
        .corner_radius(14)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                led(ui, tone);
                ui.label(RichText::new(text).size(12.0).color(if neutral { p.fg } else { tone }));
            });
        })
        .response
        .interact(Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn led(ui: &mut Ui, c: Color32) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
    ui.painter().circle_filled(r.center(), 5.0, c);
    ui.painter().circle_stroke(r.center(), 6.5, Stroke::new(2.0, c.gamma_multiply(0.25)));
}

/// "label ……… value" line.
pub fn kv(ui: &mut Ui, p: &Pal, k: &str, v: &str) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 17.0), Sense::hover());
    let pt = ui.painter_at(r);
    pt.text(r.right_center(), egui::Align2::RIGHT_CENTER, v, egui::FontId::proportional(11.5), p.fg);
    pt.text(r.left_center(), egui::Align2::LEFT_CENTER, k, egui::FontId::proportional(11.5), p.muted);
}

/// Form row: fixed-width label (+hint) on the left, controls on the right (`<Row>`).
pub fn row(ui: &mut Ui, p: &Pal, label: &str, hint: &str, add: impl FnOnce(&mut Ui)) {
    // narrow side cards get a narrower label column
    let lw = (ui.available_width() * 0.32).clamp(104.0, 150.0);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(lw, 30.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(lw);
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(RichText::new(label).size(12.5).color(p.fg2));
            if !hint.is_empty() {
                ui.label(RichText::new(hint).size(10.5).color(p.muted));
            }
        });
        ui.horizontal_wrapped(add);
    });
}

/// Small upper-case section label.
pub fn section(ui: &mut Ui, p: &Pal, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text.to_uppercase()).size(10.5).color(p.muted).strong());
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    Primary,
    Subtle,
    Ghost,
    Danger,
}

pub fn btn(p: &Pal, text: &str, kind: Btn) -> egui::Button<'static> {
    let (fill, fg, stroke) = match kind {
        Btn::Primary => (p.accent, p.accent_fg, p.accent),
        Btn::Danger => (mix(p.err, 15.0, p.panel), p.err, mix(p.err, 30.0, p.panel)),
        Btn::Subtle => (p.panel3, p.fg, p.line),
        Btn::Ghost => (Color32::TRANSPARENT, p.fg2, Color32::TRANSPARENT),
    };
    egui::Button::new(RichText::new(text.to_string()).color(fg).size(12.5))
        .fill(fill)
        .stroke(Stroke::new(1.0, stroke))
        .corner_radius(8)
        .min_size(egui::vec2(0.0, 28.0))
}

pub fn button(ui: &mut Ui, p: &Pal, text: &str, kind: Btn) -> Response {
    ui.add(btn(p, text, kind))
}

/// Button that may be disabled.
pub fn button_en(ui: &mut Ui, p: &Pal, text: &str, kind: Btn, enabled: bool) -> Response {
    ui.add_enabled(enabled, btn(p, text, kind))
}

pub fn small(p: &Pal, text: &str, kind: Btn) -> egui::Button<'static> {
    btn(p, text, kind).min_size(egui::vec2(0.0, 24.0))
}

pub fn icon_button(ui: &mut Ui, p: &Pal, icon: &str, active: bool) -> Response {
    let b = egui::Button::new(RichText::new(icon).size(14.0).color(if active { p.accent } else { p.fg2 }))
        .fill(if active { p.panel3 } else { Color32::TRANSPARENT })
        .stroke(if active { Stroke::new(1.0, p.accent) } else { Stroke::NONE })
        .corner_radius(8)
        .min_size(egui::vec2(28.0, 28.0));
    ui.add(b)
}

/// Sidebar entry.
pub fn nav_item(ui: &mut Ui, p: &Pal, icon: &str, label: &str, hint: &str, n: usize, on: bool) -> Response {
    let w = ui.available_width();
    let (rect, r) = ui.allocate_exact_size(egui::vec2(w, 38.0), Sense::click());
    let hovered = r.hovered();
    if on {
        ui.painter().rect(rect, 8.0, p.panel, Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
    } else if hovered {
        ui.painter().rect_filled(rect, 8.0, p.panel3);
    }
    let pt = ui.painter();
    pt.text(
        rect.left_center() + egui::vec2(16.0, 0.0),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(15.0),
        if on { p.accent } else { p.muted },
    );
    pt.text(
        rect.left_top() + egui::vec2(34.0, 5.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(12.5),
        if on { p.fg } else { p.fg2 },
    );
    pt.text(
        rect.left_top() + egui::vec2(34.0, 21.0),
        egui::Align2::LEFT_TOP,
        hint,
        egui::FontId::proportional(10.5),
        p.muted,
    );
    if hovered && n <= 9 {
        pt.text(
            rect.right_center() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            n.to_string(),
            egui::FontId::proportional(10.0),
            p.muted,
        );
    }
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// iOS-style switch (`<Toggle>`). Sets `changed()` when toggled.
pub fn toggle_tone(ui: &mut Ui, p: &Pal, on: &mut bool, tone: Color32) -> Response {
    let size = egui::vec2(32.0, 18.0);
    let (rect, mut r) = ui.allocate_exact_size(size, Sense::click());
    if r.clicked() {
        *on = !*on;
        r.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(r.id, *on);
    let bg = mix(tone, t * 100.0, p.line2);
    let rad = rect.height() / 2.0;
    ui.painter().rect_filled(rect, rad, bg);
    let cx = egui::lerp((rect.left() + rad)..=(rect.right() - rad), t);
    ui.painter().circle_filled(egui::pos2(cx, rect.center().y), rad - 2.0, Color32::WHITE);
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn toggle(ui: &mut Ui, p: &Pal, on: &mut bool) -> Response {
    toggle_tone(ui, p, on, p.accent)
}

/// Segmented control; returns the newly picked value.
pub fn segmented<T: PartialEq + Copy, S: AsRef<str>>(ui: &mut Ui, p: &Pal, value: T, options: &[(T, S)]) -> Option<T> {
    segmented_sized(ui, p, value, options, false)
}

pub fn segmented_sized<T: PartialEq + Copy, S: AsRef<str>>(
    ui: &mut Ui,
    p: &Pal,
    value: T,
    options: &[(T, S)],
    small: bool,
) -> Option<T> {
    let mut out = None;
    let size = if small { 11.5 } else { 12.0 };
    let h = if small { 20.0 } else { 24.0 };
    // Exact width up front: an LTR child inside a right-to-left parent (page
    // actions, field editors) would otherwise stretch the frame to the edge.
    let pad = ui.spacing().button_padding.x * 2.0;
    let text_w: f32 = options
        .iter()
        .map(|(_, l)| {
            ui.fonts_mut(|f| f.layout_no_wrap(l.as_ref().to_string(), egui::FontId::proportional(size), p.fg)).size().x
                + pad
        })
        .sum();
    let total = text_w + 2.0 * (options.len().max(1) - 1) as f32 + 4.0 + 2.0;
    ui.allocate_ui_with_layout(egui::vec2(total, h + 6.0), egui::Layout::left_to_right(egui::Align::Min), |ui| {
        egui::Frame::new().fill(p.panel3).stroke(Stroke::new(1.0, p.line)).corner_radius(8).inner_margin(2).show(
            ui,
            |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                for (v, label) in options {
                    let on = *v == value;
                    let mut text = RichText::new(label.as_ref()).size(size).color(if on { p.fg } else { p.muted });
                    if on {
                        text = text.strong();
                    }
                    let b = egui::Button::new(text)
                        .fill(if on { p.panel } else { Color32::TRANSPARENT })
                        .stroke(if on { Stroke::new(1.0, p.line2) } else { Stroke::NONE })
                        .corner_radius(6)
                        .min_size(egui::vec2(0.0, h));
                    if ui.add(b).clicked() && !on {
                        out = Some(*v);
                    }
                }
            },
        );
    });
    out
}

/// Text field with SI prefixes ("50 МГц", "250k", "1 мс"). Shows the formatted
/// value while not focused; commits on Enter / focus loss. Returns the new value.
#[allow(clippy::too_many_arguments)]
pub fn eng_field(
    ui: &mut Ui,
    p: &Pal,
    id: impl std::hash::Hash,
    value: f64,
    unit: &str,
    min: f64,
    max: f64,
    width: f32,
) -> Option<f64> {
    eng_field_ex(ui, p, id, value, unit, min, max, width, false)
}

#[allow(clippy::too_many_arguments)]
pub fn eng_field_ex(
    ui: &mut Ui,
    p: &Pal,
    id: impl std::hash::Hash,
    value: f64,
    unit: &str,
    min: f64,
    max: f64,
    width: f32,
    big: bool,
) -> Option<f64> {
    let id = ui.id().with(("eng", id));
    let shown = eng(value, unit, 6);
    let mut buf: String = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| shown.clone());
    let bad: bool = ui.data_mut(|d| d.get_temp::<bool>(id.with("bad"))).unwrap_or(false);
    let mut te = egui::TextEdit::singleline(&mut buf)
        .id(id.with("te"))
        .desired_width(width)
        .vertical_align(egui::Align::Center)
        .text_color(if bad { p.err } else { p.fg });
    if big {
        te = te.font(egui::FontId::proportional(20.0)).margin(egui::Margin::symmetric(10, 8));
    }
    // fixed size: inside a Grid the column width would otherwise shrink the field
    let h = if big { 40.0 } else { ui.spacing().interact_size.y };
    let r = ui.add_sized(egui::vec2(width, h), te);
    let mut out = None;
    if r.gained_focus() {
        ui.data_mut(|d| d.insert_temp(id, shown.clone()));
    }
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, buf.clone()));
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            ui.data_mut(|d| {
                d.remove::<String>(id);
                d.insert_temp(id.with("bad"), false)
            });
            r.surrender_focus();
        }
    }
    if r.lost_focus() {
        match parse_eng(&buf) {
            Some(v) if v >= min && v <= max => {
                if v != value {
                    out = Some(v);
                }
                ui.data_mut(|d| d.insert_temp(id.with("bad"), false));
            }
            _ => ui.data_mut(|d| d.insert_temp(id.with("bad"), true)),
        }
        ui.data_mut(|d| d.remove::<String>(id));
    } else if !r.has_focus() {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    if bad {
        r.on_hover_text(format!("Ожидается число от {} до {}", eng(min, unit, 4), eng(max, unit, 4)));
    }
    out
}

/// Number field (`<NumField>`): commits on Enter / focus loss, ↑/↓ steps (Shift ×10).
#[allow(clippy::too_many_arguments)]
pub fn num_field(
    ui: &mut Ui,
    p: &Pal,
    id: impl std::hash::Hash,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    integer: bool,
    unit: &str,
    width: f32,
) -> Option<f64> {
    num_field_ex(ui, p, id, value, min, max, step, integer, unit, width, false, true)
}

#[allow(clippy::too_many_arguments)]
pub fn num_field_ex(
    ui: &mut Ui,
    p: &Pal,
    id: impl std::hash::Hash,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    integer: bool,
    unit: &str,
    width: f32,
    hex: bool,
    enabled: bool,
) -> Option<f64> {
    let id = ui.id().with(("num", id));
    let show = |v: f64| {
        if hex {
            format!("0x{:X}", v as u32)
        } else if integer {
            format!("{:.0}", v)
        } else {
            crate::fmt::sig(v, 6)
        }
    };
    let shown = show(value);
    let mut buf: String = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| shown.clone());
    let bad: bool = ui.data_mut(|d| d.get_temp::<bool>(id.with("bad"))).unwrap_or(false);
    let mut out = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let te = egui::TextEdit::singleline(&mut buf)
            .id(id.with("te"))
            .desired_width(width)
            .vertical_align(egui::Align::Center)
            .font(egui::TextStyle::Monospace)
            .text_color(if bad { p.err } else { p.fg });
        let h = ui.spacing().interact_size.y;
        let r = ui.add_enabled_ui(enabled, |ui| ui.add_sized(egui::vec2(width, h), te)).inner;
        if r.has_focus() {
            let (up, down, shift) = ui
                .input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown), i.modifiers.shift));
            if up || down {
                let k = if up { step } else { -step } * if shift { 10.0 } else { 1.0 };
                let mut v = (value + k).clamp(min, max);
                if integer {
                    v = v.round();
                }
                out = Some(v);
                buf = show(v);
            }
            ui.data_mut(|d| d.insert_temp(id, buf.clone()));
        }
        if r.lost_focus() {
            let t = buf.trim().to_lowercase().replace(',', ".");
            let parsed = match t.strip_prefix("0x") {
                Some(h) => i64::from_str_radix(h, 16).ok().map(|v| v as f64),
                None => t.parse::<f64>().ok(),
            };
            match parsed {
                Some(mut v) if v.is_finite() => {
                    if integer {
                        v = v.round();
                    }
                    if v < min || v > max {
                        ui.data_mut(|d| d.insert_temp(id.with("bad"), true));
                    } else {
                        if v != value {
                            out = Some(v);
                        }
                        ui.data_mut(|d| d.insert_temp(id.with("bad"), false));
                    }
                }
                _ => ui.data_mut(|d| d.insert_temp(id.with("bad"), true)),
            }
            ui.data_mut(|d| d.remove::<String>(id));
        } else if !r.has_focus() {
            ui.data_mut(|d| d.remove::<String>(id));
        }
        if !unit.is_empty() {
            ui.label(RichText::new(unit).size(11.0).color(p.muted));
        }
    });
    out
}

/// Combo box over (value, label) options.
pub fn select<T: PartialEq + Clone>(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    value: &T,
    options: &[(T, String)],
    width: f32,
) -> Option<T> {
    let mut out = None;
    let cur = options.iter().find(|o| o.0 == *value).map(|o| o.1.clone()).unwrap_or_default();
    egui::ComboBox::from_id_salt(id).selected_text(cur).width(width).show_ui(ui, |ui| {
        for (v, label) in options {
            if ui.selectable_label(*v == *value, label).clicked() && *v != *value {
                out = Some(v.clone());
            }
        }
    });
    out
}

/// `SEL_REG[3:0]`.
pub fn bits_label(f: &FieldDef) -> String {
    let reg = REGS.get(f.reg).map(|r| r.name).unwrap_or("R?");
    if f.msb == f.lsb {
        format!("{}[{}]", reg, f.msb)
    } else {
        format!("{}[{}:{}]", reg, f.msb, f.lsb)
    }
}

/// Two's complement value of a `bits`-wide field.
pub fn signed(v: u32, bits: u32) -> i64 {
    if bits < 64 && v as i64 >= 1i64 << (bits - 1) {
        v as i64 - (1i64 << bits)
    } else {
        v as i64
    }
}

/// Auto-generated editor for any register field (from the Rust metadata).
pub fn field_control(ui: &mut Ui, sh: &Shared, f: &'static FieldDef, compact: bool, highlight: bool) {
    let p = &sh.pal;
    let width = f.width();
    let raw = sh.field(f.name);
    let ro = f.access != Access::Rw;
    let v = if f.signed { signed(raw, width) } else { raw as i64 };
    let opt_label = f.options.iter().find(|o| o.0 == raw).map(|o| o.1);

    let frame = egui::Frame::new()
        .fill(mix(p.panel2, 60.0, p.panel))
        .stroke(Stroke::new(if highlight { 2.0 } else { 1.0 }, if highlight { p.accent } else { p.line }))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(12, if compact { 6 } else { 8 }));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let ctrl_w = if !ro && !f.options.is_empty() && f.options.len() > 4 { 230.0 } else { 160.0 };
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.set_max_width((ui.available_width() - ctrl_w).max(150.0));
                ui.add(egui::Label::new(RichText::new(f.title).size(12.5).strong()).wrap());
                let link = ui
                    .add(
                        egui::Label::new(
                            RichText::new(format!("{} · {}", f.name, bits_label(f)))
                                .monospace()
                                .size(10.5)
                                .color(p.muted),
                        )
                        .sense(Sense::click()),
                    )
                    .on_hover_text("Открыть в карте регистров")
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if link.clicked() {
                    sh.act(Act::Focus(Some(f.name)));
                    sh.go(View::Registers);
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let unit = if f.unit.is_empty() { String::new() } else { format!(" {}", f.unit) };
                if ro {
                    let what = match f.access {
                        Access::Ro => "только чтение",
                        Access::Sticky => "sticky",
                        _ => "фиксировано",
                    };
                    let shown = opt_label.map(str::to_string).unwrap_or_else(|| v.to_string());
                    let text = format!("{}{} · {}", shown, unit, what);
                    if f.access == Access::Sticky {
                        badge(ui, p, &text, p.warn);
                    } else {
                        badge_muted(ui, p, &text);
                    }
                } else if !f.options.is_empty()
                    && f.options.len() <= 4
                    && f.options.iter().all(|o| o.1.chars().count() < 26)
                {
                    if let Some(x) = segmented_sized(ui, p, raw, f.options, true) {
                        sh.set_field(f.name, x);
                    }
                } else if !f.options.is_empty() {
                    let opts: Vec<(u32, String)> =
                        f.options.iter().map(|(v, l)| (*v, format!("{}  ({})", l, v))).collect();
                    if let Some(x) = select(ui, f.name, &raw, &opts, 220.0) {
                        sh.set_field(f.name, x);
                    }
                } else if width == 1 {
                    let mut on = raw == 1;
                    if toggle(ui, p, &mut on).changed() {
                        sh.set_field(f.name, on as u32);
                    }
                } else {
                    let (lo, hi) = if f.signed {
                        (-(1i64 << (width - 1)) as f64, ((1i64 << (width - 1)) - 1) as f64)
                    } else {
                        (0.0, f.max() as f64)
                    };
                    let fw = if width > 10 { 80.0 } else { 56.0 };
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        if let Some(x) = num_field(ui, p, f.name, v as f64, lo, hi, 1.0, true, f.unit, fw) {
                            let x = x as i64;
                            let w = if x < 0 { x + (1i64 << width) } else { x };
                            sh.set_field(f.name, w as u32);
                        }
                    });
                }
            });
        });
        if !compact && !f.desc.is_empty() {
            ui.add(egui::Label::new(RichText::new(f.desc).size(11.5).color(p.muted)).wrap());
        }
    });
}
