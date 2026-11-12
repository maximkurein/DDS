//! «Выходной тракт и фазовый шум» (`ChainView.tsx`).

use crate::app::{key, Shared};
use crate::fmt::{axis_hz, eng, fixed};
use crate::plot::{Plot, Series};
use crate::widgets as w;
use crate::worker::{Data, Query};
use eframe::egui::{self, RichText, Ui};

pub fn show(ui: &mut Ui, sh: &Shared) {
    let p = sh.pal;
    let c = &sh.st.derived.calc;
    let main = if c.ch1.on_dac1 { &c.ch1 } else { &c.ch2 };
    let r = match sh.query("response", key(("resp", sh.cfg)), Query::Response { lo: 1e6, hi: 600e6, n: 600 }) {
        Some(Data::Response(r)) => Some(r),
        _ => None,
    };
    let pn = match sh.query("phase_noise", key(("pn", sh.cfg)), Query::PhaseNoise) {
        Some(Data::PhaseNoise(pn)) => Some(pn),
        _ => None,
    };
    w::page_header(
        ui,
        &p,
        "📉",
        "Выходной тракт и фазовый шум",
        "АЧХ платы от выхода ЦАП до XW2 (ABCD-модель с потерями катушек), спад sin(x)/x ступенчатого ЦАП и итоговый уровень. Ниже — фазовый шум по таблице 8.1 и пересчёт шума такта.",
        |_| {},
    );
    ui.spacing_mut().item_spacing.y = 16.0;
    if let Some(r) = r {
        w::auto_grid(ui, 170.0, 6, |ui, i| {
            match i {
                0 => w::stat(ui, &p, "Срез ФНЧ (−3 дБ)", &eng(r.fc_3db, "Hz", 4), Some(p.refc), ""),
                1 => w::stat(
                    ui,
                    &p,
                    "Неравномерность 48–52 МГц",
                    &format!("{} дБ", fixed(r.band_ripple_db, 2)),
                    None,
                    "",
                ),
                2 => w::stat(
                    ui,
                    &p,
                    "Потери тракта на fout",
                    &format!("{} дБ", fixed(c.chain_db, 2)),
                    None,
                    &eng(main.fout, "Hz", 6),
                ),
                3 => w::stat(ui, &p, "sin(x)/x на fout", &format!("{} дБ", fixed(c.sinc_db, 2)), None, ""),
                4 => w::stat(
                    ui,
                    &p,
                    "Образ Fclk − f",
                    &format!("{} дБм", fixed(c.image_dbm, 1)),
                    Some(p.warn),
                    &eng(c.image_hz, "Hz", 4),
                ),
                _ => {
                    w::stat(ui, &p, "IFS", &eng(c.ifs, "A", 3), None, &format!("OUTM1 до {} В", fixed(c.v_outm_max, 2)))
                }
            };
        });
    }
    w::two_cols(
        ui,
        380.0,
        false,
        &mut (),
        |ui, _| {
            w::card(
                ui,
                &p,
                "АЧХ",
                "ФНЧ+аттенюатор и sin(x)/x — дБ (левая ось); уровень на XW2 при текущем Mul — дБм (правая ось)",
                |ui| {
                    if let Some(r) = r {
                        Plot::new(
                            "chain-resp",
                            vec![
                                Series::new("ФНЧ + аттенюатор", r.f.clone(), r.chain_db.clone(), p.refc).width(2.0),
                                Series::new("sin(x)/x", r.f.clone(), r.sinc_db.clone(), p.clk).width(1.5).dash(),
                                Series::new("Уровень на XW2", r.f.clone(), r.out_dbm.clone(), p.sig).width(1.5).y2(),
                            ],
                        )
                        .height(340.0)
                        .x_domain(1e6, 600e6)
                        .y_domain(-120.0, 5.0)
                        .y2_domain(-130.0, 0.0)
                        .x_fmt(axis_hz)
                        .labels("", "дБ", "дБм")
                        .shade(48e6, 52e6)
                        .vmark(main.fout, "f", Some(p.sig), false)
                        .vmark(c.image_hz, "Fclk − f", Some(p.warn), true)
                        .legend()
                        .tooltip_x(|v| eng(v, "Hz", 5))
                        .show(ui, &p);
                    }
                },
            );
        },
        |ui, _| {
            w::card(ui, &p, "Элементы тракта", "По схеме ревизии F", |ui| {
                if let Some(r) = r {
                    egui::Grid::new("chain-el").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
                        for (n, v) in &r.elements {
                            ui.label(RichText::new(n).monospace().size(12.0).color(p.accent));
                            ui.add(egui::Label::new(RichText::new(v).size(12.0).color(p.fg2)).wrap());
                            ui.end_row();
                        }
                    });
                }
                ui.add_space(6.0);
                w::hint_text(
                    ui,
                    &p,
                    p.accent,
                    "Добротность катушек и RREF меняются на вкладке «Частота» → «Параметры платы».",
                );
            });
        },
    );
    w::two_cols(
        ui,
        380.0,
        false,
        &mut (),
        |ui, _| {
            w::card(
                ui,
                &p,
                "Фазовый шум на выходе",
                "дБн/Гц от отстройки; пунктир — точки таблицы 8.1 (такт 1 ГГц)",
                |ui| {
                    if let Some(pn) = pn {
                        let mut series =
                            vec![Series::new(eng(pn.fout, "Hz", 4), pn.offsets.clone(), pn.dbc_hz.clone(), p.sig)
                                .width(2.0)];
                        let colors = [p.clk, p.refc, p.dig, p.ctl];
                        for (i, (f, pts)) in pn.ref_curves.iter().enumerate() {
                            series.push(
                                Series::new(
                                    format!("ТО: {}", eng(*f, "Hz", 3)),
                                    pts.iter().map(|q| q.0).collect(),
                                    pts.iter().map(|q| q.1).collect(),
                                    colors[i % 4],
                                )
                                .width(1.0)
                                .dash(),
                            );
                        }
                        Plot::new("chain-pn", series)
                            .height(300.0)
                            .x_log()
                            .x_fmt(axis_hz)
                            .labels("", "дБн/Гц", "")
                            .legend()
                            .tooltip_x(|v| eng(v, "Hz", 3))
                            .show(ui, &p);
                    }
                },
            );
        },
        |ui, _| {
            w::card(ui, &p, "Джиттер", "", |ui| {
                if let Some(pn) = pn {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    w::stat(
                        ui,
                        &p,
                        "Интегральный джиттер",
                        &eng(pn.jitter_s, "s", 3),
                        Some(p.sig),
                        "по показанной полосе отстроек",
                    );
                    let k = 20.0 * (c.fclk / main.fout.max(1.0)).log10();
                    w::hint_text(
                        ui,
                        &p,
                        p.accent,
                        &format!(
                            "DDS делит частоту такта: шум источника 480 МГц попадает на выход, уменьшенный на 20·lg(Fclk/Fout) ≈ {} дБ. Собственный шум кристалла — по ТО (таблица задана для такта 1 ГГц, это оценка).",
                            fixed(k, 1)
                        ),
                    );
                }
            });
        },
    );
}
