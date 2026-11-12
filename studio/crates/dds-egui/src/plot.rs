//! Charts: a port of the web `components/Plot.tsx` (SVG) to the egui painter —
//! linear or log x, a second y axis, shaded band, vertical/horizontal markers,
//! labelled points, an "instrument" (spectrum analyser) look, hover tooltip and
//! legend. Drawing it directly keeps the look identical to the web version.

use crate::theme::Pal;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui};

#[derive(Clone)]
pub struct Series {
    pub name: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub color: Color32,
    pub width: f32,
    pub dash: bool,
    pub fill: bool,
    pub y2: bool,
}

impl Series {
    pub fn new(name: impl Into<String>, x: Vec<f64>, y: Vec<f64>, color: Color32) -> Self {
        Series { name: name.into(), x, y, color, width: 1.6, dash: false, fill: false, y2: false }
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    pub fn dash(mut self) -> Self {
        self.dash = true;
        self
    }
    #[allow(dead_code)] // part of the Plot.tsx API, unused by the current views
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }
    pub fn y2(mut self) -> Self {
        self.y2 = true;
        self
    }
}

pub struct Marker {
    pub v: f64,
    pub label: String,
    pub color: Option<Color32>,
    pub dash: bool,
}

pub struct Point {
    pub x: f64,
    pub y: f64,
    pub label: String,
    pub color: Option<Color32>,
}

type Fmt = Box<dyn Fn(f64) -> String>;

pub struct Plot {
    id: String,
    series: Vec<Series>,
    height: f32,
    x_log: bool,
    x_domain: Option<[f64; 2]>,
    y_domain: Option<[f64; 2]>,
    y2_domain: Option<[f64; 2]>,
    x_fmt: Option<Fmt>,
    y_fmt: Option<Fmt>,
    y2_fmt: Option<Fmt>,
    tip_x: Option<Fmt>,
    x_label: String,
    y_label: String,
    y2_label: String,
    v_markers: Vec<Marker>,
    h_markers: Vec<Marker>,
    points: Vec<Point>,
    shade: Option<[f64; 2]>,
    instrument: bool,
    legend: bool,
}

/// `shortNum` of Plot.tsx.
pub fn short_num(v: f64) -> String {
    let a = v.abs();
    let t = |x: f64| {
        if x.abs() >= 100.0 {
            format!("{:.0}", x)
        } else if x.abs() >= 10.0 {
            trim_fixed(x, 1)
        } else {
            trim_fixed(x, 2)
        }
    };
    if a >= 1e9 {
        t(v / 1e9) + "G"
    } else if a >= 1e6 {
        t(v / 1e6) + "M"
    } else if a >= 1e3 {
        t(v / 1e3) + "k"
    } else if a == 0.0 {
        "0".into()
    } else if a < 1.0 {
        crate::fmt::sig(v, 2)
    } else {
        t(v)
    }
}

fn trim_fixed(x: f64, dec: usize) -> String {
    let s = format!("{:.*}", dec, x);
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" {
        "0".into()
    } else {
        s
    }
}

fn nice_ticks(min: f64, max: f64, count: usize) -> Vec<f64> {
    let span = max - min;
    if span.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return vec![min];
    }
    let raw = span / count as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let n = raw / mag;
    let step = if n < 1.5 {
        1.0
    } else if n < 3.0 {
        2.0
    } else if n < 7.0 {
        5.0
    } else {
        10.0
    } * mag;
    let mut out = vec![];
    let mut v = (min / step).ceil() * step;
    while v <= max + step * 1e-9 && out.len() < 200 {
        out.push(if v.abs() < step * 1e-9 { 0.0 } else { v });
        v += step;
    }
    out
}

fn extent<'a>(series: impl Iterator<Item = &'a [f64]>) -> [f64; 2] {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for s in series {
        for &v in s {
            if v.is_finite() {
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
    }
    if !lo.is_finite() {
        return [0.0, 1.0];
    }
    if lo == hi {
        return [lo - 1.0, hi + 1.0];
    }
    [lo, hi]
}

fn nearest(xs: &[f64], x: f64) -> usize {
    if xs.len() < 2 {
        return 0;
    }
    let (mut lo, mut hi) = (0, xs.len() - 1);
    while hi - lo > 1 {
        let m = (lo + hi) / 2;
        if xs[m] < x {
            lo = m
        } else {
            hi = m
        }
    }
    if (xs[lo] - x).abs() < (xs[hi] - x).abs() {
        lo
    } else {
        hi
    }
}

impl Plot {
    pub fn new(id: impl Into<String>, series: Vec<Series>) -> Self {
        Plot {
            id: id.into(),
            series,
            height: 260.0,
            x_log: false,
            x_domain: None,
            y_domain: None,
            y2_domain: None,
            x_fmt: None,
            y_fmt: None,
            y2_fmt: None,
            tip_x: None,
            x_label: String::new(),
            y_label: String::new(),
            y2_label: String::new(),
            v_markers: vec![],
            h_markers: vec![],
            points: vec![],
            shade: None,
            instrument: false,
            legend: false,
        }
    }
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    pub fn x_log(mut self) -> Self {
        self.x_log = true;
        self
    }
    pub fn x_domain(mut self, a: f64, b: f64) -> Self {
        self.x_domain = Some([a, b]);
        self
    }
    pub fn y_domain(mut self, a: f64, b: f64) -> Self {
        self.y_domain = Some([a, b]);
        self
    }
    pub fn y2_domain(mut self, a: f64, b: f64) -> Self {
        self.y2_domain = Some([a, b]);
        self
    }
    pub fn x_fmt(mut self, f: impl Fn(f64) -> String + 'static) -> Self {
        self.x_fmt = Some(Box::new(f));
        self
    }
    pub fn y_fmt(mut self, f: impl Fn(f64) -> String + 'static) -> Self {
        self.y_fmt = Some(Box::new(f));
        self
    }
    #[allow(dead_code)] // part of the Plot.tsx API, unused by the current views
    pub fn y2_fmt(mut self, f: impl Fn(f64) -> String + 'static) -> Self {
        self.y2_fmt = Some(Box::new(f));
        self
    }
    pub fn tooltip_x(mut self, f: impl Fn(f64) -> String + 'static) -> Self {
        self.tip_x = Some(Box::new(f));
        self
    }
    pub fn labels(mut self, x: &str, y: &str, y2: &str) -> Self {
        self.x_label = x.into();
        self.y_label = y.into();
        self.y2_label = y2.into();
        self
    }
    pub fn vmark(mut self, v: f64, label: impl Into<String>, color: Option<Color32>, dash: bool) -> Self {
        self.v_markers.push(Marker { v, label: label.into(), color, dash });
        self
    }
    #[allow(dead_code)] // part of the Plot.tsx API, unused by the current views
    pub fn hmark(mut self, v: f64, label: impl Into<String>, color: Option<Color32>) -> Self {
        self.h_markers.push(Marker { v, label: label.into(), color, dash: true });
        self
    }
    pub fn point(mut self, x: f64, y: f64, label: impl Into<String>, color: Option<Color32>) -> Self {
        self.points.push(Point { x, y, label: label.into(), color });
        self
    }
    pub fn shade(mut self, a: f64, b: f64) -> Self {
        self.shade = Some([a, b]);
        self
    }
    pub fn instrument(mut self) -> Self {
        self.instrument = true;
        self
    }
    pub fn legend(mut self) -> Self {
        self.legend = true;
        self
    }

    pub fn show(self, ui: &mut Ui, p: &Pal) {
        let w = ui.available_width().max(200.0);
        let h = self.height;
        let has_y2 = self.series.iter().any(|s| s.y2);
        let m_l = 52.0;
        let m_r = if has_y2 { 52.0 } else { 14.0 };
        let m_t = 10.0;
        let m_b = if self.x_label.is_empty() { 24.0 } else { 36.0 };
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::hover());
        let pw = w - m_l - m_r;
        let ph = h - m_t - m_b;
        let o = rect.min;
        let plot = Rect::from_min_size(o + egui::vec2(m_l, m_t), egui::vec2(pw, ph));
        let painter = ui.painter_at(rect);

        let xd = self.x_domain.unwrap_or_else(|| extent(self.series.iter().map(|s| s.x.as_slice())));
        let yd = match self.y_domain {
            Some(d) => d,
            None => {
                let e = extent(self.series.iter().filter(|s| !s.y2).map(|s| s.y.as_slice()));
                let pad = (e[1] - e[0]) * 0.05;
                [e[0] - pad, e[1] + pad]
            }
        };
        let y2d = self.y2_domain.unwrap_or_else(|| extent(self.series.iter().filter(|s| s.y2).map(|s| s.y.as_slice())));
        let x_log = self.x_log;
        let (la, lb) = (xd[0].max(1e-12).log10(), xd[1].max(1e-12).log10());
        let sx = |v: f64| -> f32 {
            let t = if x_log { (v.max(1e-12).log10() - la) / (lb - la) } else { (v - xd[0]) / (xd[1] - xd[0]) };
            plot.left() + t as f32 * pw
        };
        let inv_x = |px: f32| -> f64 {
            let t = ((px - plot.left()) / pw) as f64;
            if x_log {
                10f64.powf(la + t * (lb - la))
            } else {
                xd[0] + t * (xd[1] - xd[0])
            }
        };
        let sy = |v: f64| plot.top() + (1.0 - ((v - yd[0]) / (yd[1] - yd[0])) as f32) * ph;
        let sy2 = |v: f64| plot.top() + (1.0 - ((v - y2d[0]) / (y2d[1] - y2d[0])) as f32) * ph;

        let inst = self.instrument;
        let grid_c = if inst { Color32::from_rgb(0x13, 0x36, 0x1f) } else { p.grid };
        let axis_c = if inst { Color32::from_rgb(0x5f, 0xbf, 0x7a) } else { p.muted };
        let font = FontId::proportional(10.5);

        if inst {
            painter.rect_filled(rect, 8.0, Color32::from_rgb(0x02, 0x08, 0x05));
        }
        let clip = painter.with_clip_rect(plot);
        if let Some([a, b]) = self.shade {
            let r = Rect::from_x_y_ranges(sx(a)..=sx(b).max(sx(a)), plot.y_range());
            clip.rect_filled(r, 0.0, p.accent.gamma_multiply(0.07));
        }

        // x ticks
        let (major, minor) = if x_log {
            let (mut major, mut minor) = (vec![], vec![]);
            let mut k = xd[0].log10().floor() as i32;
            while k <= xd[1].log10().ceil() as i32 {
                let d = 10f64.powi(k);
                if d >= xd[0] * 0.999 && d <= xd[1] * 1.001 {
                    major.push(d);
                }
                for j in 2..10 {
                    let v = j as f64 * d;
                    if v > xd[0] && v < xd[1] {
                        minor.push(v);
                    }
                }
                k += 1;
            }
            (major, minor)
        } else {
            (nice_ticks(xd[0], xd[1], ((pw / 90.0).floor() as usize).max(3)), vec![])
        };
        for v in minor {
            let x = sx(v);
            painter.line_segment(
                [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
                Stroke::new(0.6, grid_c.gamma_multiply(0.6)),
            );
        }
        let xf = |v: f64| self.x_fmt.as_ref().map(|f| f(v)).unwrap_or_else(|| short_num(v));
        let yf = |v: f64| self.y_fmt.as_ref().map(|f| f(v)).unwrap_or_else(|| short_num(v));
        let y2f = |v: f64| self.y2_fmt.as_ref().map(|f| f(v)).unwrap_or_else(|| short_num(v));
        for v in major {
            let x = sx(v);
            painter.line_segment([Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())], Stroke::new(1.0, grid_c));
            painter.text(Pos2::new(x, plot.bottom() + 9.0), Align2::CENTER_CENTER, xf(v), font.clone(), axis_c);
        }
        let ny = ((ph / 42.0).floor() as usize).max(3);
        for v in nice_ticks(yd[0], yd[1], ny) {
            let y = sy(v);
            painter.line_segment([Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)], Stroke::new(1.0, grid_c));
            painter.text(Pos2::new(plot.left() - 6.0, y), Align2::RIGHT_CENTER, yf(v), font.clone(), axis_c);
        }
        if has_y2 {
            for v in nice_ticks(y2d[0], y2d[1], ny) {
                painter.text(Pos2::new(plot.right() + 6.0, sy2(v)), Align2::LEFT_CENTER, y2f(v), font.clone(), axis_c);
            }
        }
        let frame_c = if inst { Color32::from_rgb(0x1f, 0x5a, 0x33) } else { p.line2 };
        painter.rect_stroke(plot, 0.0, Stroke::new(1.0, frame_c), egui::StrokeKind::Inside);
        if !self.x_label.is_empty() {
            painter.text(
                Pos2::new(plot.center().x, rect.bottom() - 4.0),
                Align2::CENTER_BOTTOM,
                &self.x_label,
                FontId::proportional(11.0),
                axis_c,
            );
        }
        let vertical = |text: &str, x: f32, angle: f32| {
            let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), FontId::proportional(11.0), axis_c));
            let sz = g.size();
            let c = Pos2::new(x, plot.center().y);
            // rotate around the galley's top-left: place so that its centre lands on c
            let (s, co) = angle.sin_cos();
            let half = egui::vec2(sz.x / 2.0, sz.y / 2.0);
            let rot = egui::vec2(half.x * co - half.y * s, half.x * s + half.y * co);
            let mut ts = egui::epaint::TextShape::new(c - rot, g, axis_c);
            ts.angle = angle;
            painter.add(ts);
        };
        if !self.y_label.is_empty() {
            vertical(&self.y_label, rect.left() + 8.0, -std::f32::consts::FRAC_PI_2);
        }
        if has_y2 && !self.y2_label.is_empty() {
            vertical(&self.y2_label, rect.right() - 8.0, std::f32::consts::FRAC_PI_2);
        }

        // series
        for s in &self.series {
            let yy = |v: f64| if s.y2 { sy2(v) } else { sy(v) };
            let mut runs: Vec<Vec<Pos2>> = vec![vec![]];
            for (x, y) in s.x.iter().zip(&s.y) {
                if !x.is_finite() || !y.is_finite() || (x_log && *x <= 0.0) {
                    if !runs.last().unwrap().is_empty() {
                        runs.push(vec![]);
                    }
                    continue;
                }
                runs.last_mut().unwrap().push(Pos2::new(sx(*x), yy(*y)));
            }
            if s.fill {
                for r in runs.iter().filter(|r| r.len() > 1) {
                    // fill as vertical strips (the area may be non-convex)
                    let c = s.color.gamma_multiply(0.12);
                    let mut mesh = egui::Mesh::default();
                    for wnd in r.windows(2) {
                        let i = mesh.vertices.len() as u32;
                        mesh.colored_vertex(wnd[0], c);
                        mesh.colored_vertex(Pos2::new(wnd[0].x, plot.bottom()), c);
                        mesh.colored_vertex(wnd[1], c);
                        mesh.colored_vertex(Pos2::new(wnd[1].x, plot.bottom()), c);
                        mesh.add_triangle(i, i + 1, i + 2);
                        mesh.add_triangle(i + 1, i + 2, i + 3);
                    }
                    clip.add(mesh);
                }
            }
            for r in runs.into_iter().filter(|r| r.len() > 1) {
                let st = Stroke::new(s.width, s.color);
                if inst {
                    clip.add(Shape::line(r.clone(), Stroke::new(s.width + 3.0, s.color.gamma_multiply(0.18))));
                }
                if s.dash {
                    clip.extend(Shape::dashed_line(&r, st, 5.0, 4.0));
                } else {
                    clip.add(Shape::line(r, st));
                }
            }
        }
        for hm in &self.h_markers {
            let c = hm.color.unwrap_or(p.muted);
            let y = sy(hm.v);
            clip.extend(Shape::dashed_line(
                &[Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                Stroke::new(1.0, c),
                4.0,
                4.0,
            ));
            if !hm.label.is_empty() {
                clip.text(Pos2::new(plot.right() - 4.0, y - 4.0), Align2::RIGHT_BOTTOM, &hm.label, font.clone(), c);
            }
        }
        for (i, vm) in self.v_markers.iter().enumerate() {
            let c = vm.color.unwrap_or(p.accent);
            let x = sx(vm.v);
            let seg = [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())];
            if vm.dash {
                clip.extend(Shape::dashed_line(&seg, Stroke::new(1.0, c), 4.0, 4.0));
            } else {
                clip.line_segment(seg, Stroke::new(1.0, c));
            }
            if !vm.label.is_empty() {
                let y = plot.top() + 12.0 + (i % 3) as f32 * 13.0;
                clip.text(Pos2::new(x + 4.0, y), Align2::LEFT_BOTTOM, &vm.label, font.clone(), c);
            }
        }
        for pt in &self.points {
            let c = pt.color.unwrap_or(p.sig);
            let pos = Pos2::new(sx(pt.x), sy(pt.y));
            clip.circle(pos, 3.5, c, Stroke::new(1.0, p.panel));
            if !pt.label.is_empty() {
                clip.text(pos + egui::vec2(6.0, -6.0), Align2::LEFT_BOTTOM, &pt.label, font.clone(), c);
            }
        }

        // hover
        if let Some(hp) = resp.hover_pos().filter(|h| h.x >= plot.left() && h.x <= plot.right()) {
            let hx = inv_x(hp.x);
            painter
                .line_segment([Pos2::new(hp.x, plot.top()), Pos2::new(hp.x, plot.bottom())], Stroke::new(0.8, axis_c));
            let tips: Vec<(&Series, f64, f64)> = self
                .series
                .iter()
                .filter(|s| !s.x.is_empty())
                .map(|s| {
                    let i = nearest(&s.x, hx);
                    (s, s.x[i], s.y[i])
                })
                .collect();
            for (s, x, y) in &tips {
                if y.is_finite() {
                    painter.circle_filled(Pos2::new(sx(*x), if s.y2 { sy2(*y) } else { sy(*y) }), 3.0, s.color);
                }
            }
            if let Some((_, x0, _)) = tips.first() {
                let head = self.tip_x.as_ref().map(|f| f(*x0)).unwrap_or_else(|| xf(*x0));
                let left = (hp.x + 12.0).min(rect.right() - 200.0);
                egui::Area::new(egui::Id::new(("plot-tip", &self.id)))
                    .order(egui::Order::Tooltip)
                    .fixed_pos(Pos2::new(left, plot.top() + 6.0))
                    .interactable(false)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::new()
                            .fill(p.panel.gamma_multiply(0.96))
                            .stroke(Stroke::new(1.0, p.line))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.set_min_width(150.0);
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(RichText::new(head).size(11.5).color(p.muted));
                                for (s, _, y) in &tips {
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), Sense::hover());
                                        ui.painter().circle_filled(r.center(), 4.0, s.color);
                                        ui.label(RichText::new(&s.name).size(11.5).color(p.fg2));
                                        let v = if !y.is_finite() {
                                            "—".to_string()
                                        } else if s.y2 && self.y2_fmt.is_some() {
                                            y2f(*y)
                                        } else if self.y_fmt.is_some() {
                                            yf(*y)
                                        } else {
                                            format!("{:.1}", y)
                                        };
                                        ui.label(RichText::new(v).size(11.5).strong());
                                    });
                                }
                            });
                    });
            }
        }

        if self.legend {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 2.0);
                for s in &self.series {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 10.0), Sense::hover());
                        ui.painter().rect_filled(
                            Rect::from_center_size(r.center(), egui::vec2(16.0, 3.0)),
                            1.5,
                            s.color,
                        );
                        ui.label(RichText::new(&s.name).size(11.5).color(p.fg2));
                    });
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_and_short_numbers() {
        assert_eq!(nice_ticks(0.0, 10.0, 5), vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(short_num(50e6), "50M");
        assert_eq!(short_num(1.5e9), "1.5G");
        assert_eq!(short_num(12.34), "12.3");
        assert_eq!(short_num(0.0), "0");
        assert_eq!(nearest(&[0.0, 1.0, 2.0, 3.0], 2.2), 2);
    }
}
