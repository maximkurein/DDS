//! «Подключение»: current link, board state, USB-UART, wiring, USB devices (`HardwareView.tsx`).

use crate::app::{Act, Shared};
use crate::content::{SAFETY, WIRING};
use crate::widgets as w;
use dds_core::transport::PINS;
use eframe::egui::{self, RichText, Stroke, Ui};
use serde_json::json;

pub struct HwState {
    pub port: String,
    pub baud: u32,
    pub asked: bool,
}

impl Default for HwState {
    fn default() -> Self {
        HwState { port: String::new(), baud: 38400, asked: false }
    }
}

fn hex4(v: u16) -> String {
    format!("{:04X}", v)
}

fn board_card(ui: &mut Ui, sh: &Shared) {
    let p = sh.pal;
    let Some(b) = sh.st.board.as_ref() else { return };
    let sim = sh.is_sim();
    let (title, sub) = if sim {
        ("🖴 Виртуальная плата", "Модель платы вокруг кристалла: прошивка, питание, переключатели")
    } else {
        ("🖴 Состояние платы", "Ответ прошивки на команду ST (опрос 350 мс)")
    };
    w::card(ui, &p, title, sub, |ui| {
        egui::Grid::new("board-state").num_columns(2).spacing([16.0, 8.0]).min_col_width(150.0).show(ui, |ui| {
            let k = |ui: &mut Ui, s: &str| {
                ui.label(RichText::new(s).size(12.5).color(p.muted));
            };
            k(ui, "Прошивка");
            ui.label(RichText::new(if b.fw.is_empty() { "—" } else { &b.fw }).monospace().size(12.5));
            ui.end_row();
            k(ui, "Режим");
            if b.remote {
                w::badge(ui, &p, "REMOTE — управляет ПК", p.ok);
            } else {
                w::badge(ui, &p, "DIP — управляет плата", p.warn);
            }
            ui.end_row();
            k(ui, "+3,3 В (PGOOD)");
            if b.pgood {
                w::badge(ui, &p, "есть", p.ok);
            } else {
                w::badge(ui, &p, "нет", p.err);
            }
            ui.end_row();
            k(ui, "Кнопка S4");
            if b.apply_pressed {
                w::badge(ui, &p, "нажата", p.accent);
            } else {
                w::badge_muted(ui, &p, "отпущена");
            }
            ui.end_row();
            k(ui, "DIP-код N");
            let dip = match b.dip {
                Some(n) => format!("{} → {} МГц", n, crate::fmt::fixed(48.0 + n as f64 / 1000.0, 3)),
                None => "—".into(),
            };
            ui.label(RichText::new(dip).size(12.5));
            ui.end_row();
            k(ui, "Последняя ошибка");
            if b.err != 0 {
                w::badge(ui, &p, &format!("код {}", b.err), p.err);
            } else {
                w::badge(ui, &p, "нет", p.ok);
            }
            ui.end_row();
            k(ui, "Перебор (HOP)");
            let hop = if b.hop_n >= 2 {
                format!("{} профилей × {} мс", b.hop_n, b.hop_ms)
            } else {
                "выкл.".into()
            };
            ui.label(RichText::new(hop).size(12.5));
            ui.end_row();
        });
        if sim {
            ui.separator();
            w::section(ui, &p, "Воздействия на симулятор");
            let set = &sh.st.settings;
            w::row(ui, &p, "Такт на XW1", "без такта SPI молчит", |ui| {
                let mut on = set.sim_clock_ok;
                if w::toggle_tone(ui, &p, &mut on, p.ok).changed() {
                    sh.set_settings(json!({ "sim_clock_ok": on }));
                }
            });
            w::row(ui, &p, "+3,3 В (PGOOD)", "провал питания", |ui| {
                let mut on = set.sim_pgood;
                if w::toggle_tone(ui, &p, &mut on, p.ok).changed() {
                    sh.set_settings(json!({ "sim_pgood": on }));
                }
            });
            w::row(ui, &p, "DIP-код N", "S2 + S3, 0…4000", |ui| {
                if let Some(v) =
                    w::num_field(ui, &p, "sim-dip", b.dip.unwrap_or(0) as f64, 0.0, 4095.0, 1.0, true, "", 56.0)
                {
                    sh.rpc("set_sim_dip", json!({ "value": v as u32 }));
                }
            });
        }
    });
}

pub fn show(ui: &mut Ui, sh: &Shared, st: &mut HwState) {
    let p = sh.pal;
    if !st.asked {
        st.asked = true;
        sh.act(Act::RefreshPorts);
    }
    let sim = sh.is_sim();
    w::page_header(
        ui,
        &p,
        "🔌",
        "Подключение",
        "Симулятор или настоящая плата: USB-UART адаптер на площадках X1 и прошивка D2 версии 1.1+ с командным протоколом (38400 8N1). SPI к 1508ПЛ8Т ведёт контроллер платы.",
        |_| {},
    );
    w::cols(ui, 2, 460.0, |ui, col| {
        if col == 0 {
            w::card(ui, &p, "⚡ Текущее подключение", "", |ui| {
                ui.horizontal(|ui| {
                    w::led(ui, if sh.st.link_error.is_some() { p.err } else { p.ok });
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.set_max_width((ui.available_width() - 130.0).max(120.0));
                        ui.label(RichText::new(&sh.st.link.name).strong().size(13.0));
                        let d = sh.st.link_error.clone().unwrap_or_else(|| sh.st.link.detail.clone());
                        ui.add(egui::Label::new(RichText::new(d).size(11.5).color(p.muted)).truncate());
                    });
                    if !sim {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(w::small(&p, "На симулятор", w::Btn::Subtle)).clicked() {
                                sh.rpc("connect", json!({ "kind": "sim" }));
                            }
                        });
                    }
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add(w::small(&p, "🔍 Проверить связь", w::Btn::Subtle))
                        .on_hover_text("Прочитать DEVID через кадры SETAFT + NOP")
                        .clicked()
                    {
                        sh.act(Act::CheckLink);
                    }
                    if ui
                        .add(w::small(&p, "⟲ Сброс RSTn", w::Btn::Subtle))
                        .on_hover_text("Импульс RSTn: регистры — по табл. 5.1")
                        .clicked()
                    {
                        sh.rpc("chip_reset", json!({}));
                    }
                    if ui.add(w::small(&p, "SWRST", w::Btn::Ghost)).on_hover_text("SWRST = 0x0078").clicked() {
                        sh.rpc("chip_reset", json!({ "soft": true }));
                    }
                    if !sim
                        && ui
                            .add(w::small(&p, "↶ Вернуть плате", w::Btn::Ghost))
                            .on_hover_text("REMOTE 0: частоту снова задают DIP-переключатели")
                            .clicked()
                    {
                        sh.rpc("release", json!({}));
                    }
                });
                ui.add_space(8.0);
                w::cols(ui, 2, 150.0, |ui, i| {
                    let Some(pin) = PINS.get(i) else { return };
                    egui::Frame::new()
                        .fill(p.panel2)
                        .stroke(Stroke::new(1.0_f32, p.line))
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [52.0, 18.0],
                                    egui::Label::new(RichText::new(pin.name).monospace().size(12.0)),
                                );
                                if pin.dir == "out" {
                                    let mut on = sh.st.pins.get(pin.name).copied().unwrap_or(pin.idle);
                                    if w::toggle(ui, &p, &mut on).on_hover_text(pin.title).changed() {
                                        sh.rpc("set_pin", json!({ "name": pin.name, "high": on }));
                                    }
                                } else {
                                    match sh.st.inputs.get(pin.name).copied().flatten() {
                                        None => w::badge_muted(ui, &p, "—"),
                                        Some(true) => w::badge(ui, &p, "1", p.ok),
                                        Some(false) => w::badge(ui, &p, "0", p.warn),
                                    };
                                }
                                ui.add(egui::Label::new(RichText::new(pin.title).size(11.0).color(p.muted)).truncate())
                                    .on_hover_text(pin.title);
                            });
                        });
                });
            });
            w::card_actions(
                ui,
                &p,
                "🔗 Плата через USB-UART",
                "",
                |ui| {
                    if ui.add(w::small(&p, "⟳", w::Btn::Ghost)).on_hover_text("Обновить список").clicked()
                    {
                        sh.act(Act::RefreshPorts);
                    }
                },
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let opts: Vec<(String, String)> = if sh.ports.is_empty() {
                            vec![(String::new(), "порты не найдены".into())]
                        } else {
                            sh.ports
                                .iter()
                                .map(|x| {
                                    let label = match &x.product {
                                        Some(pr) => format!("{} — {}", x.path, pr),
                                        None => x.path.clone(),
                                    };
                                    (x.path.clone(), label)
                                })
                                .collect()
                        };
                        if let Some(v) = w::select(ui, "hw-port", &st.port, &opts, 260.0) {
                            st.port = v;
                        }
                        let bauds: Vec<(u32, String)> = [9600u32, 19200, 38400, 57600]
                            .iter()
                            .map(|b| (*b, format!("{} бод{}", b, if *b == 38400 { " (прошивка)" } else { "" })))
                            .collect();
                        if let Some(v) = w::select(ui, "hw-baud", &st.baud, &bauds, 150.0) {
                            st.baud = v;
                        }
                        let connecting = sh.busy_shown;
                        let label = if connecting { "Подключение…" } else { "Подключить" };
                        if w::button_en(ui, &p, label, w::Btn::Primary, !st.port.is_empty() && !connecting).clicked() {
                            sh.rpc_ok(
                                "connect",
                                json!({ "kind": "serial", "path": st.port, "baud": st.baud }),
                                "Плата подключена, режим REMOTE",
                            );
                        }
                    });
                    ui.add_space(6.0);
                    w::hint_text(
                        ui,
                        &p,
                        p.accent,
                        "Приложение посылает «?» и ждёт «PL8T …», затем включает «REMOTE 1»: DIP-переключатели и кнопка S4 перестают действовать, регистры пишет ПК. Строки журнала отладочной прошивки пропускаются. Протокол — docs/PROTOCOL_ru.md.",
                    );
                },
            );
        } else {
            board_card(ui, sh);
            w::card(ui, &p, "🖧 Схема соединений", "", |ui| {
                egui::Grid::new("wiring").num_columns(5).spacing([14.0, 6.0]).show(ui, |ui| {
                    for h in ["Адаптер", "", "Плата", "Сигнал D2", "Примечание"] {
                        ui.label(RichText::new(h).size(11.0).color(p.muted));
                    }
                    ui.end_row();
                    for x in WIRING {
                        ui.label(RichText::new(x.mcu).monospace().size(12.5));
                        ui.label(RichText::new(x.dir).size(12.5).color(p.muted));
                        ui.label(RichText::new(x.board).monospace().size(12.5));
                        ui.label(RichText::new(x.signal).monospace().size(12.5).color(p.accent));
                        ui.label(RichText::new(x.note).size(12.5).color(p.muted));
                        ui.end_row();
                    }
                });
                ui.add_space(6.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                for s in &SAFETY[..3] {
                    w::hint(ui, &p, p.warn, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new("🛡").color(p.warn).size(13.0));
                            ui.add(egui::Label::new(RichText::new(*s).size(12.0).color(p.fg2)).wrap());
                        });
                    });
                }
            });
        }
    });
    w::card(
        ui,
        &p,
        "USB-устройства",
        "Подсказки помогают найти USB-UART адаптер (CH340, CP210x, FT232, PL2303)",
        |ui| match &sh.usb {
            None => {
                ui.label(RichText::new("Сканирование…").size(12.0).color(p.muted));
            }
            Some(list) if list.is_empty() => {
                ui.label(RichText::new("Устройства не найдены (или нет доступа к USB).").size(12.0).color(p.muted));
            }
            Some(list) => {
                egui::ScrollArea::vertical().max_height(300.0).min_scrolled_height(300.0).id_salt("usb").show(
                    ui,
                    |ui| {
                        w::auto_grid(ui, 360.0, list.len(), |ui, i| {
                            let d = &list[i];
                            let (fill, stroke) = if d.likely_bridge {
                                (crate::theme::mix(p.accent, 5.0, p.panel), crate::theme::mix(p.accent, 40.0, p.panel))
                            } else {
                                (p.panel2, p.line)
                            };
                            egui::Frame::new()
                                .fill(fill)
                                .stroke(Stroke::new(1.0_f32, stroke))
                                .corner_radius(8)
                                .inner_margin(egui::Margin::symmetric(12, 6))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("{}:{}", hex4(d.vid), hex4(d.pid)))
                                                .monospace()
                                                .size(12.0)
                                                .color(p.muted),
                                        );
                                        ui.label(
                                            RichText::new(d.product.as_deref().unwrap_or("—")).strong().size(12.0),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(d.manufacturer.as_deref().unwrap_or(""))
                                                    .size(12.0)
                                                    .color(p.muted),
                                            )
                                            .truncate(),
                                        );
                                    });
                                    if let Some(h) = d.hint {
                                        ui.label(RichText::new(h).size(11.0).color(p.muted));
                                    }
                                });
                        });
                    },
                );
            }
        },
    );
}
