//! Screenshot tour for visual checks: `DDS_SHOTS=<dir> dds-egui` (or `APP_SHOTS`)
//! walks every view in the dark theme, then in the light one, plus a few
//! variants (bus log open, keying tab, every ECO page, a selected block), saves
//! `NN_View_theme.bmp` into `<dir>` and closes the window. The configuration
//! file is not read or written during the tour (see `App::new`).

use crate::app::{View, NAV};
use eframe::egui::{self, ColorImage};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub view: View,
    pub dark: bool,
    pub console: bool,
    /// Mod: 1 = keying tab; Eco: page index; Overview: 1 = block selected.
    pub variant: u8,
}

fn steps() -> Vec<Step> {
    let mut v = vec![];
    for dark in [true, false] {
        for n in NAV.iter() {
            v.push(Step { view: n.0, dark, console: false, variant: 0 });
        }
        v.push(Step { view: View::Overview, dark, console: true, variant: 1 });
        v.push(Step { view: View::Mod, dark, console: false, variant: 1 });
        for page in 1..crate::eco::PAGES.len() as u8 {
            v.push(Step { view: View::Eco, dark, console: false, variant: page });
        }
    }
    v
}

pub struct Shots {
    dir: PathBuf,
    idx: usize,
    frames: u32,
    pending: bool,
}

impl Shots {
    pub fn from_env() -> Option<Shots> {
        let dir = PathBuf::from(std::env::var_os("DDS_SHOTS").or_else(|| std::env::var_os("APP_SHOTS"))?);
        std::fs::create_dir_all(&dir).ok()?;
        Some(Shots { dir, idx: 0, frames: 0, pending: false })
    }

    pub fn current(&self) -> Option<Step> {
        steps().get(self.idx).copied()
    }

    /// Call once per frame after drawing.
    pub fn tick(&mut self, ctx: &egui::Context) {
        let Some(s) = self.current() else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        let image = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = image {
            let mut name = format!("{:02}_{:?}_{}", self.idx + 1, s.view, if s.dark { "dark" } else { "light" });
            if s.variant > 0 {
                name += &format!("_v{}", s.variant);
            }
            if s.console {
                name += "_console";
            }
            let _ = write_bmp(&self.dir.join(name + ".bmp"), &img);
            self.idx += 1;
            self.frames = 0;
            self.pending = false;
        } else if !self.pending {
            if self.idx == 0 && self.frames == 0 {
                // same frame for every run, whatever size the window was left at
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1440.0, 900.0)));
            }
            self.frames += 1;
            // let layout, animations and the engine's answers settle
            if self.frames > 25 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.pending = true;
            }
        }
        ctx.request_repaint();
    }
}

fn write_bmp(path: &std::path::Path, img: &ColorImage) -> std::io::Result<()> {
    let [w, h] = img.size;
    let row = (w * 3).div_ceil(4) * 4;
    let size = 54 + row * h;
    let mut b = Vec::with_capacity(size);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&(size as u32).to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(w as i32).to_le_bytes());
    b.extend_from_slice(&(h as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0; 24]);
    for y in (0..h).rev() {
        let start = b.len();
        for x in 0..w {
            let c = img.pixels[y * w + x];
            b.extend_from_slice(&[c.b(), c.g(), c.r()]);
        }
        b.resize(start + row, 0);
    }
    std::fs::write(path, b)
}
