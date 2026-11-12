//! «Карта регистров»: groups, bit grids, write/read, export/import (`RegistersView.tsx`).

use crate::app::{Act, Shared};
use crate::fmt::hex_w;
use crate::theme::mix;
use crate::widgets as w;
use dds_core::regs::{self, Access, FieldDef, FIELDS, REGS};
use eframe::egui::{self, Color32, RichText, Sense, Stroke, Ui};
use serde_json::json;

const GROUPS: [(&str, &str, &str); 7] = [
    ("common", "Общие", "SWRST, DEVID, SEL_REG, CTR, SYNC, CLR, LINK, ROUTE, TC"),
    ("ch1", "Канал 1", "Управление каналом 1"),
    ("prof1", "Профили К1", "dPh, P, Mul, Offset канала 1"),
    ("lfm1", "ЛЧМ К1", "Генератор ЛЧМ канала 1"),
    ("ch2", "Канал 2", "Управление каналом 2"),
    ("prof2", "Профили К2", "dPh, P, Mul, Offset канала 2"),
    ("lfm2", "ЛЧМ К2", "Генератор ЛЧМ канала 2"),
];

const PALETTE: [u32; 12] = [
    0x6366f1, 0x06b6d4, 0xf59e0b, 0x10b981, 0xec4899, 0x8b5cf6, 0xef4444, 0x14b8a6, 0xeab308, 0x3b82f6, 0xf97316,
    0x84cc16,
];

fn pal(i: usize) -> Color32 {
    let v = PALETTE[i % PALETTE.len()];
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Profile number of a profile register/field name (`CH1_dPh3_L` → 3).
pub fn prof_of(name: &str) -> Option<usize> {
    let rest = name.strip_prefix("CH1_").or_else(|| name.strip_prefix("CH2_"))?;
    for pre in ["dPh", "Offset", "Mul", "P"] {
        if let Some(d) = rest.strip_prefix(pre) {
            let digits: String = d.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return digits.parse().ok();
            }
        }
    }
    None
}

pub struct RegState {
    pub reg: usize,
    pub group: &'static str,
    pub prof: usize,
    seen_focus: Option<&'static str>,
    scroll_to: Option<&'static str>,
}

impl Default for RegState {
    fn default() -> Self {
        RegState { reg: regs::R_CTR, group: "common", prof: 0, seen_focus: None, scroll_to: None }
    }
}

fn fields_of(r: usize) -> Vec<&'static FieldDef> {
    FIELDS.iter().filter(|f| f.reg == r).collect()
}

/// Returns (bit clicked, field clicked).
fn bit_grid(
    ui: &mut Ui,
    sh: &Shared,
    word: u32,
    fields: &[&'static FieldDef],
) -> (Option<u32>, Option<&'static FieldDef>) {
    let p = &sh.pal;
    let bits = regs::DATA_BITS as usize;
    let mut owner: Vec<Option<usize>> = vec![None; bits];
    for (i, f) in fields.iter().enumerate() {
        for b in f.lsb..=f.msb {
            owner[b as usize] = Some(i);
        }
    }
    let gap = 2.0;
    let total = ui.available_width();
    let cw = (total - gap * (bits as f32 - 1.0)) / bits as f32;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(total, 28.0 + 14.0 + 22.0), Sense::hover());
    let mut out = (None, None);
    for k in 0..bits {
        let b = bits - 1 - k;
        let x = rect.left() + k as f32 * (cw + gap);
        let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(cw, 28.0));
        let v = (word >> b) & 1;
        let id = ui.id().with(("bit", word, b, fields.first().map(|f| f.reg)));
        let r = ui.interact(cell, id, Sense::click());
        let (fill, stroke) = match owner[b] {
            Some(i) => {
                let c = pal(i);
                let mut fill = mix(c, if v == 1 { 55.0 } else { 14.0 }, p.panel);
                if r.hovered() {
                    fill = mix(c, if v == 1 { 70.0 } else { 26.0 }, p.panel);
                }
                (fill, mix(c, 50.0, p.panel))
            }
            None => (p.panel2, p.line),
        };
        let focused = owner[b].is_some_and(|i| sh.focus == Some(fields[i].name));
        ui.painter().rect(
            cell,
            5.0,
            fill,
            Stroke::new(if focused { 2.0 } else { 1.0 }, if focused { p.accent } else { stroke }),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            cell.center(),
            egui::Align2::CENTER_CENTER,
            v.to_string(),
            egui::FontId::monospace(11.0),
            if v == 1 { p.fg } else { p.muted },
        );
        if b % 4 == 3 || b == 0 {
            ui.painter().text(
                egui::pos2(cell.center().x, cell.bottom() + 7.0),
                egui::Align2::CENTER_CENTER,
                b.to_string(),
                egui::FontId::monospace(9.0),
                p.muted,
            );
        }
        let r = match owner[b] {
            Some(i) => r
                .on_hover_text(format!("{}: {} — {}", b, fields[i].name, fields[i].title))
                .on_hover_cursor(egui::CursorIcon::PointingHand),
            None => r.on_hover_text(format!("{}: не используется", b)),
        };
        if r.clicked() {
            if let Some(i) = owner[b] {
                if fields[i].access != Access::Rw {
                    out.1 = Some(fields[i]);
                } else {
                    out.0 = Some(b as u32);
                }
            }
        }
    }
    // field name bars
    let y = rect.top() + 28.0 + 14.0;
    for (i, f) in fields.iter().enumerate() {
        let left = rect.left() + (bits - 1 - f.msb as usize) as f32 * (cw + gap);
        let width = (f.msb - f.lsb + 1) as f32 * (cw + gap) - gap;
        let bar = egui::Rect::from_min_size(egui::pos2(left, y), egui::vec2(width, 20.0));
        let c = pal(i);
        let r = ui.interact(bar, ui.id().with(("fbar", f.name)), Sense::click());
        ui.painter().rect_filled(bar, 4.0, mix(c, if r.hovered() { 30.0 } else { 18.0 }, p.panel));
        let pt = ui.painter_at(bar.shrink(1.0));
        pt.text(bar.center(), egui::Align2::CENTER_CENTER, f.name, egui::FontId::monospace(9.5), c);
        if r.on_hover_text(format!("{} {}", f.name, w::bits_label(f)))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            out.1 = Some(f);
        }
    }
    out
}

pub fn show(ui: &mut Ui, sh: &Shared, st: &mut RegState) {
    let p = sh.pal;
    // a field link elsewhere (or the bit grid) asked to show a field
    if sh.focus != st.seen_focus {
        st.seen_focus = sh.focus;
        if let Some(f) = sh.focus.and_then(regs::field) {
            st.reg = f.reg;
            st.group = REGS[f.reg].group;
            if let Some(y) = prof_of(REGS[f.reg].name) {
                st.prof = y;
            }
            st.scroll_to = Some(f.name);
        }
    }
    let bits = regs::DATA_BITS;
    let can_read = sh.st.link.readback && dds_core::bus::can_read();
    let has_sticky = FIELDS.iter().any(|f| f.access == Access::Sticky);
    w::page_header(
        ui,
        &p,
        "🔢",
        "Карта регистров",
        &format!(
            "{} регистров × {} бит (ТО, табл. 5.1) · {}. Щелчок по биту инвертирует его, по имени поля — открывает редактор.",
            REGS.len(),
            bits,
            dds_core::bus::describe()
        ),
        |ui| {
            if w::button(ui, &p, "⬆ Импорт", w::Btn::Ghost).clicked() {
                sh.act(Act::Import);
            }
            if w::button(ui, &p, "⬇ Экспорт", w::Btn::Ghost).clicked() {
                sh.act(Act::Export);
            }
            if w::button(ui, &p, "📖 По даташиту", w::Btn::Ghost).on_hover_text("Значения после сброса по даташиту").clicked() {
                sh.rpc("reset_defaults", json!({ "starter": false }));
            }
            if w::button(ui, &p, "⟲ Стартовая", w::Btn::Subtle).on_hover_text("Рабочая стартовая конфигурация").clicked() {
                sh.rpc("reset_defaults", json!({ "starter": true }));
            }
            if has_sticky && w::button(ui, &p, "Сброс флагов", w::Btn::Subtle).on_hover_text("Сбросить sticky-флаги").clicked() {
                sh.rpc("clear_sticky", json!({}));
            }
            if w::button_en(ui, &p, "⬇ Прочитать всё", w::Btn::Subtle, can_read).on_hover_text("Прочитать все регистры").clicked() {
                sh.rpc("read_all", json!({}));
            }
            if w::button(ui, &p, "➤ Записать всё", w::Btn::Subtle).on_hover_text("Записать всю конфигурацию").clicked() {
                sh.rpc("write_all", json!({}));
            }
        },
    );
    let side = if ui.available_width() > 1500.0 { 440.0 } else { 400.0 };
    w::two_cols(
        ui,
        side,
        false,
        st,
        |ui, st| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.horizontal_wrapped(|ui| {
                let opts: Vec<(&str, &str)> = GROUPS.iter().map(|g| (g.0, g.1)).collect();
                if let Some(g) = w::segmented(ui, &p, st.group, &opts) {
                    st.group = g;
                }
                let is_prof = st.group.starts_with("prof");
                if is_prof {
                    let n = if st.group == "prof2" { regs::N_PROF2 } else { regs::N_PROF1 };
                    let opts: Vec<(usize, String)> = (0..n).map(|y| (y, y.to_string())).collect();
                    if let Some(y) = w::segmented_sized(ui, &p, st.prof.min(n - 1), &opts, true) {
                        st.prof = y;
                    }
                }
                if ui
                    .add_enabled(can_read, w::small(&p, "⬇ Прочитать группу", w::Btn::Ghost))
                    .on_hover_text("Прочитать регистры этой группы")
                    .clicked()
                {
                    sh.rpc("read_all", json!({ "group": st.group }));
                }
            });
            let is_prof = st.group.starts_with("prof");
            let visible: Vec<usize> = (0..REGS.len())
                .filter(|&r| REGS[r].group == st.group && (!is_prof || prof_of(REGS[r].name) == Some(st.prof)))
                .collect();
            for r in visible {
                reg_card(ui, sh, st, r, can_read);
            }
        },
        |ui, st| {
            let rd = &REGS[st.reg];
            w::card(ui, &p, &format!("Поля {}", rd.name), rd.title, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for f in fields_of(st.reg) {
                    let focused = sh.focus == Some(f.name);
                    let r = ui.scope(|ui| w::field_control(ui, sh, f, false, focused)).response;
                    if st.scroll_to == Some(f.name) {
                        r.scroll_to_me(Some(egui::Align::Center));
                        st.scroll_to = None;
                    }
                }
            });
        },
    );
}

fn reg_card(ui: &mut Ui, sh: &Shared, st: &mut RegState, r: usize, can_read: bool) {
    let p = sh.pal;
    let rd = &REGS[r];
    let bits = regs::DATA_BITS;
    let fields = fields_of(r);
    let rw = fields.iter().any(|f| f.access == Access::Rw);
    let word = sh.st.regs[r];
    let sel = st.reg == r;
    let resp = egui::Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, if sel { p.accent } else { p.line }))
        .corner_radius(14)
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add_sized([64.0, 18.0], egui::Label::new(RichText::new(rd.name).strong().size(13.0)));
                ui.label(RichText::new(hex_w(rd.addr as u32, 16)).monospace().size(11.0).color(p.muted));
                if let Some(v) =
                    w::num_field_ex(ui, &p, ("reg", r), word as f64, 0.0, 65535.0, 1.0, true, "", 64.0, true, rw)
                {
                    sh.rpc("set_register", json!({ "index": r, "value": v as u32 }));
                }
                if rw {
                    if sh.st.dirty[r] {
                        w::badge(ui, &p, "не записан", p.warn);
                    } else {
                        w::badge(ui, &p, "записан", p.ok);
                    }
                } else {
                    w::badge_muted(ui, &p, "статус");
                }
                let rb = sh.st.readback.words.get(r).copied().flatten();
                match (sh.st.readback.matches.get(r).copied().flatten(), rb) {
                    (Some(true), Some(v)) => {
                        w::badge(ui, &p, &format!("чтение ✓ {}", hex_w(v, bits)), p.ok);
                    }
                    (Some(false), Some(v)) => {
                        w::badge(ui, &p, &format!("чтение ≠ {}", hex_w(v, bits)), p.err);
                    }
                    _ => {}
                }
                if let Some(dev) = sh.st.dev_regs.as_ref().and_then(|d| d.get(r)) {
                    if rw && *dev != sh.st.desired[r] {
                        ui.label(
                            RichText::new(format!("в кристалле {}", hex_w(*dev, bits)))
                                .monospace()
                                .size(11.0)
                                .color(p.muted),
                        );
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(can_read, w::small(&p, "R", w::Btn::Ghost))
                        .on_hover_text(format!("Прочитать {}", rd.name))
                        .clicked()
                    {
                        sh.rpc("read_register", json!({ "index": r }));
                    }
                    if ui
                        .add_enabled(rw, w::small(&p, "W", w::Btn::Ghost))
                        .on_hover_text(format!("Записать {}", rd.name))
                        .clicked()
                    {
                        sh.rpc("write_register", json!({ "index": r }));
                    }
                    ui.add(egui::Label::new(RichText::new(rd.title).size(11.5).color(p.muted)).truncate());
                });
            });
            ui.add_space(6.0);
            let (bit, field) = bit_grid(ui, sh, word, &fields);
            if let Some(b) = bit {
                st.reg = r;
                sh.rpc("set_register", json!({ "index": r, "value": word ^ (1 << b) }));
            }
            if let Some(f) = field {
                st.reg = r;
                sh.act(Act::Focus(Some(f.name)));
            }
        })
        .response;
    if resp.interact(Sense::click()).clicked() {
        st.reg = r;
    }
}
