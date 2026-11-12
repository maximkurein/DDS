//! «Спектр на выходе»: analyser view at XW2, digital spectrum and tones (`SpectrumView.tsx`).

use crate::app::{key, Shared};
use crate::fmt::{axis_hz, eng, fixed};
use crate::plot::{Plot, Series};
use crate::theme::Pal;
use crate::widgets as w;
use crate::worker::{Data, Query};
use eframe::egui::{self, Color32, RichText, Ui};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Span {
    Band,
    Nyq,
    G1,
    G2,
}

pub struct SpecState {
    pub span: Span,
    pub log2n: u32,
}

impl Default for SpecState {
    fn default() -> Self {
        SpecState { span: Span::G1, log2n: 14 }
    }
}

pub fn kind(p: &Pal, k: &str) -> (&'static str, Color32) {
    match k {
        "carrier" => ("несущая", p.sig),
        "image" => ("образ", p.warn),
        "harmonic" => ("гармоника", p.clk),
        _ => ("спур", p.muted),
    }
}

pub fn show(ui: &mut Ui, sh: &Shared, st: &mut SpecState) {
    let p = sh.pal;
    let fclk = sh.st.derived.calc.fclk;
    let (lo, hi) = match st.span {
        Span::Band => (40e6, 60e6),
        Span::Nyq => (0.0, fclk / 2.0),
        Span::G1 => (0.0, 1e9),
        Span::G2 => (0.0, 2e9),
    };
    let q = Query::Spectrum { lo, hi, log2n: st.log2n, points: 1600 };
    let data = sh.query("spectrum", key((lo.to_bits(), hi.to_bits(), st.log2n, sh.cfg)), q);
    let busy = sh.query_busy("spectrum");
    w::page_header(
        ui,
        &p,
        "📊",
        "Спектр на выходе",
        "Бит-в-бит модель канала (аккумулятор, усечение фазы, дизеринг, 10-битный ЦАП) + БПФ. Затем каждый бин переносится во все зоны Найквиста через sin(x)/x, ФНЧ и аттенюатор платы. Гармоники ЦАП — оценка по SFDR (ТО, табл. 8.1).",
        |ui| {
            let opts: Vec<(u32, String)> = [12, 14, 16].iter().map(|n| (*n, format!("2^{}", n))).collect();
            if let Some(n) = w::segmented_sized(ui, &p, st.log2n, &opts, true) {
                st.log2n = n;
            }
            if let Some(s) = w::segmented(
                ui,
                &p,
                st.span,
                &[(Span::Band, "40–60 МГц"), (Span::Nyq, "0…Fclk/2"), (Span::G1, "0…1 ГГц"), (Span::G2, "0…2 ГГц")],
            ) {
                st.span = s;
            }
            if busy {
                ui.add(egui::Spinner::new().size(15.0).color(p.muted));
            }
        },
    );
    if let Some(e) = sh.query_err("spectrum") {
        w::hint_text(ui, &p, p.err, e);
    }
    let Some(Data::Spectrum(sp)) = data else { return };
    ui.spacing_mut().item_spacing.y = 16.0;
    let floor = sp.dbm.iter().copied().filter(|v| v.is_finite()).fold(f64::INFINITY, f64::min);
    let floor = if floor.is_finite() { floor } else { -160.0 };
    let y_lo = ((floor / 10.0).floor() * 10.0).max(-170.0);
    let tones: Vec<_> = sp.tones.iter().filter(|t| t.f >= lo && t.f <= hi).collect();

    w::auto_grid(ui, 170.0, 5, |ui, i| {
        match i {
            0 => w::stat(ui, &p, "Несущая на XW2", &format!("{} дБм", fixed(sp.carrier_dbm, 2)), Some(p.sig), ""),
            1 => w::stat(
                ui,
                &p,
                "SFDR в окне",
                &format!("{} дБн", fixed(sp.sfdr_db, 1)),
                None,
                "худшая помеха относительно несущей",
            ),
            2 => w::stat(
                ui,
                &p,
                "SFDR цифровой",
                &format!("{} дБн", fixed(sp.sfdr_digital_db, 1)),
                None,
                "только отсчёты, 0…Fclk/2",
            ),
            3 => w::stat(ui, &p, "Полоса бина", &eng(sp.rbw_hz, "Hz", 3), None, &format!("БПФ {} точек", sp.n)),
            _ => w::stat(ui, &p, "Такт", &eng(sp.fs, "Hz", 4), None, ""),
        };
    });
    w::card(
        ui,
        &p,
        "Анализатор на XW2",
        "дБм в полосе бина; заштрихована рабочая полоса 48–52 МГц",
        |ui| {
            let mut pl =
                Plot::new("spec-xw2", vec![Series::new("XW2", sp.f.clone(), sp.dbm.clone(), p.sig).width(1.3)])
                    .instrument()
                    .height(340.0)
                    .x_domain(lo, hi)
                    .y_domain(y_lo, 10.0)
                    .x_fmt(axis_hz)
                    .y_fmt(|v| format!("{:.0}", v))
                    .labels("", "дБм", "")
                    .shade(48e6, 52e6)
                    .tooltip_x(|v| eng(v, "Hz", 6));
            if st.span != Span::Band {
                pl = pl.vmark(fclk / 2.0, "Fclk/2", Some(p.muted), true);
                if fclk <= hi {
                    pl = pl.vmark(fclk, "Fclk", Some(p.muted), true);
                }
            }
            for t in tones.iter().take(14) {
                pl = pl.point(t.f, t.dbm, t.name.clone(), Some(kind(&p, t.kind).1));
            }
            pl.show(ui, &p);
        },
    );
    w::cols(ui, 2, 420.0, |ui, i| {
        if i == 0 {
            w::card(
                ui,
                &p,
                "Цифровые отсчёты",
                "Спектр кодов ЦАП в первой зоне Найквиста, дБ от полной шкалы",
                |ui| {
                    Plot::new(
                        "spec-dig",
                        vec![Series::new("коды", sp.dig_f.clone(), sp.dig_dbfs.clone(), p.dig).width(1.1)],
                    )
                    .height(240.0)
                    .x_domain(0.0, sp.fs / 2.0)
                    .y_domain(-140.0, 5.0)
                    .x_fmt(axis_hz)
                    .labels("", "дБFS", "")
                    .tooltip_x(|v| eng(v, "Hz", 6))
                    .show(ui, &p);
                },
            );
        } else {
            w::card(
                ui,
                &p,
                "Составляющие",
                "Найденные моделью тона в окне",
                |ui| {
                    egui::ScrollArea::vertical().max_height(250.0).min_scrolled_height(250.0).id_salt("tones").show(
                        ui,
                        |ui| {
                            egui::Grid::new("tones-grid").num_columns(4).spacing([12.0, 6.0]).striped(false).show(
                                ui,
                                |ui| {
                                    for h in ["Что", "Частота", "дБм", "дБн"] {
                                        ui.label(RichText::new(h).size(11.0).color(p.muted));
                                    }
                                    ui.end_row();
                                    for t in &tones {
                                        let (label, c) = kind(&p, t.kind);
                                        ui.horizontal(|ui| {
                                            w::badge(ui, &p, label, c);
                                            ui.label(RichText::new(&t.name).size(12.0));
                                        });
                                        ui.label(RichText::new(eng(t.f, "Hz", 6)).size(12.0));
                                        ui.label(RichText::new(fixed(t.dbm, 1)).size(12.0));
                                        ui.label(
                                            RichText::new(if t.kind == "carrier" {
                                                "—".into()
                                            } else {
                                                fixed(t.dbc, 1)
                                            })
                                            .size(12.0),
                                        );
                                        ui.end_row();
                                    }
                                },
                            );
                        },
                    );
                },
            );
        }
    });
    w::hint_text(
        ui,
        &p,
        p.accent,
        "Попробуйте: поднимите частоту к 0,4·Fclk на вкладке «Частота» и посмотрите, как образ Fclk − f подходит к несущей; выключите дизеринг фазы — в цифровом спектре появятся дискретные спуры усечения фазы.",
    );
}
