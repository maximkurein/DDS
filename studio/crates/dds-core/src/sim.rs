//! Virtual 1508ПЛ8Т on the DDS board: answers the same 24-bit SPI frames as the
//! real chip (section 6.3), so the whole app and `cargo test` work without hardware.
//!
//! Modelled behaviour
//! - the 24-bit shift register: each frame returns what the PREVIOUS command left
//!   (NOP → DEVID, SETA/WR/WRI → address, SETAFT → data), table 6.1;
//! - all 64 profiles of both channels, *_all broadcast writes, SWRST (0x0078),
//!   CLR (LFM start/stop), reserved bits reading 0, write-only registers reading 0;
//! - LINK.on blocks writes ≥ 0x1000 (table 5.6);
//! - LFM stages advance in real time (stage times are scaled to be visible);
//! - failure modes of the board: no clock (SPI dead, DEVID reads 0), no +3,3 В
//!   (PGOOD low, the firmware keeps the DDS in reset), RSTn held low;
//! - the board around the chip: DIP switches S2/S3, button S4, hopping driven by
//!   the firmware's HOP command.

use crate::bus::{self, Cmd};
use crate::model::{self, Settings};
use crate::regs::{self, Words, NUM_REGS};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Serialize, Clone, Debug)]
pub struct SimSignal {
    pub name: &'static str,
    pub value: f64,
    pub unit: &'static str,
}

#[derive(Serialize, Clone, Debug)]
pub struct SimStatus {
    /// "ok" | "busy" | "warn" | "fault" | "off"
    pub state: &'static str,
    pub text: String,
    pub signals: Vec<SimSignal>,
    pub pins: Vec<(String, bool)>,
}

/// State of the board around the chip (firmware `ST` command or the simulator).
#[derive(Serialize, Clone, Debug, Default)]
pub struct BoardState {
    pub fw: String,
    pub remote: bool,
    pub pgood: bool,
    pub apply_pressed: bool,
    /// 12-bit DIP code (S2 = bits 0…7, S3 = bits 8…11), ON = 1.
    pub dip: Option<u16>,
    /// Last firmware error code (0 = ok).
    pub err: u8,
    /// Firmware hopping: number of profiles cycled (0 = off) and dwell.
    pub hop_n: u8,
    pub hop_ms: u32,
}

pub struct VirtualChip {
    mem: HashMap<u16, u16>,
    addr: u16,
    shift: u16,
    env: Settings,
    t0: Instant,
    rst_low: bool,
    lfm_t0: [Option<Instant>; 2],
    lfm_stopped: [bool; 2],
    hop: Option<(u8, u32, Instant)>,
    pub dip: u16,
    pub frames: u64,
}

/// Bits that exist at an address (reserved bits read 0).
fn addr_mask(a: u16) -> Option<u16> {
    if let Some(r) = regs::reg_by_addr(a) {
        return Some(regs::defined_mask(r) as u16);
    }
    // profiles 16…63 of channel 1 and 4…63 of channel 2 are not in REGS but exist in the chip
    for ch in [1u8, 2] {
        let base = regs::ch_base(ch) + 0x400;
        if (base..base + 0x400).contains(&a) {
            return match (a - base) & 0xF {
                0..=2 | 4 => Some(0xFFFF),
                5 => Some(0xFFF8),
                6 => Some(0xFFF0),
                _ => None,
            };
        }
    }
    None
}

impl VirtualChip {
    pub fn new(env: Settings) -> Self {
        let mut c = VirtualChip {
            mem: HashMap::new(),
            addr: 0,
            shift: regs::DEVID_VALUE,
            env,
            t0: Instant::now(),
            rst_low: false,
            lfm_t0: [None, None],
            lfm_stopped: [false, false],
            hop: None,
            dip: 2000, // 50,000 МГц on the DIP switches
            frames: 0,
        };
        c.reset();
        c
    }

    /// Hardware or software reset: every register to its table 5.1 value
    /// (XXXX registers modelled as 0).
    fn reset(&mut self) {
        self.mem.clear();
        let d = regs::defaults();
        for (r, def) in regs::REGS.iter().enumerate() {
            self.mem.insert(def.addr, d[r] as u16);
        }
        self.addr = 0;
        self.shift = regs::DEVID_VALUE;
        self.lfm_t0 = [None, None];
        self.lfm_stopped = [false, false];
    }

    pub fn set_env(&mut self, env: Settings) {
        self.env = env;
    }

    fn alive(&self) -> bool {
        self.env.sim_clock_ok && self.env.sim_pgood && !self.rst_low
    }

    /// Register words of the modelled map as the chip holds them.
    pub fn active(&self) -> Words {
        let mut w = [0u32; NUM_REGS];
        for (r, def) in regs::REGS.iter().enumerate() {
            w[r] = *self.mem.get(&def.addr).unwrap_or(&0) as u32;
        }
        w[regs::R_SEL] = self.read(0x0002) as u32;
        w
    }

    fn fclk(&self) -> f64 {
        model::fclk_eff(&self.active_raw(), &self.env)
    }

    fn active_raw(&self) -> Words {
        let mut w = [0u32; NUM_REGS];
        for (r, def) in regs::REGS.iter().enumerate() {
            w[r] = *self.mem.get(&def.addr).unwrap_or(&0) as u32;
        }
        w
    }

    /// LFM stage durations of channel `ch`, seconds. Real cycles last µs…ms; the
    /// simulator stretches them ×1000 so the stage indicator is visible at a 350 ms poll.
    fn lfm_durations(&self, ch: u8) -> [f64; 4] {
        let w = self.active_raw();
        let f = self.fclk();
        std::array::from_fn(|k| regs::get48(&w, &format!("CH{}_LS_TPH{}", ch, k + 1)) as f64 * 4.0 / f * 1000.0)
    }

    /// Current LFM stage (0…3) of channel `ch`.
    fn lfm_stage(&self, ch: u8) -> u16 {
        let i = (ch - 1) as usize;
        if self.lfm_stopped[i] {
            return 3;
        }
        let Some(t0) = self.lfm_t0[i] else { return 0 };
        let d = self.lfm_durations(ch);
        let period: f64 = d.iter().sum();
        let mut t = t0.elapsed().as_secs_f64();
        let auto = self.mem.get(&regs::ch_base(ch)).map(|v| v & (1 << 4) != 0).unwrap_or(false);
        if auto && period > 0.0 {
            t %= period;
        }
        let mut acc = 0.0;
        for (k, x) in d.iter().enumerate() {
            acc += x;
            if t < acc {
                return k as u16;
            }
        }
        3
    }

    fn read(&self, a: u16) -> u16 {
        match a {
            0x0001 => regs::DEVID_VALUE,
            0x0000 | 0x0005 => 0,
            0x0002 => {
                let mut v = *self.mem.get(&a).unwrap_or(&0) & 0x3F3F;
                if let Some((n, _, _)) = self.hop {
                    v = (v & !0x3F) | self.hop_profile(n);
                }
                v | self.lfm_stage(1) << 6 | self.lfm_stage(2) << 14
            }
            _ => match addr_mask(a) {
                Some(m) => self.mem.get(&a).copied().unwrap_or(0) & m,
                None => 0,
            },
        }
    }

    fn hop_profile(&self, n: u8) -> u16 {
        let Some((_, ms, t0)) = self.hop else { return 0 };
        ((t0.elapsed().as_millis() / ms.max(1) as u128) % n.max(1) as u128) as u16
    }

    fn write(&mut self, a: u16, v: u16) {
        let link_on = self.mem.get(&0x0006).map(|x| x & 1 == 1).unwrap_or(false);
        if link_on && a >= 0x1000 {
            return; // table 5.6
        }
        match a {
            0x0000 => {
                if v == regs::SWRST_KEY {
                    self.reset();
                }
            }
            0x0001 => {}
            0x0005 => {
                // CLR (table 5.5)
                for ch in [1u8, 2] {
                    let i = (ch - 1) as usize;
                    if v & (1 << (3 + ch)) != 0 {
                        self.lfm_t0[i] = Some(Instant::now());
                        self.lfm_stopped[i] = false;
                    }
                    if v & (1 << (5 + ch)) != 0 {
                        self.lfm_stopped[i] = true;
                    }
                }
            }
            0x0002 => {
                // in LFM mode a SEL_REG write jumps to the written stage (section 3.3)
                for ch in [1u8, 2] {
                    let i = (ch - 1) as usize;
                    let ls_on = self.mem.get(&regs::ch_base(ch)).map(|x| x & 0x8000 != 0).unwrap_or(false);
                    if ls_on {
                        let stage = if ch == 1 { (v >> 6) & 3 } else { (v >> 14) & 3 } as usize;
                        let d = self.lfm_durations(ch);
                        let before: f64 = d[..stage].iter().sum();
                        self.lfm_t0[i] = Instant::now().checked_sub(std::time::Duration::from_secs_f64(before));
                        self.lfm_stopped[i] = false;
                    }
                }
                self.mem.insert(a, v & 0x3F3F);
            }
            _ => {
                for ch in [1u8, 2] {
                    let all = regs::ch_base(ch) + 0x300;
                    if (all..=all + 6).contains(&a) {
                        let off = a - all;
                        if off == 3 {
                            return;
                        }
                        let m = [0xFFFF, 0xFFFF, 0xFFFF, 0, 0xFFFF, 0xFFF8, 0xFFF0][off as usize];
                        for y in 0..64 {
                            self.mem.insert(regs::prof_addr(ch, y, off), v & m);
                        }
                        return;
                    }
                }
                if let Some(m) = addr_mask(a) {
                    self.mem.insert(a, v & m);
                }
            }
        }
    }

    // ───────────── bus side ─────────────

    /// One 24-bit frame; returns the bytes shifted out on SDO during it.
    pub fn spi(&mut self, tx: &[u8]) -> Vec<u8> {
        if !self.alive() {
            return vec![0; tx.len()]; // R82 pulls the buffered SDO low
        }
        self.frames += 1;
        let out = vec![0, (self.shift >> 8) as u8, self.shift as u8];
        match bus::decode(tx) {
            Some((Cmd::Nop, _)) => self.shift = regs::DEVID_VALUE,
            Some((Cmd::Seta, p)) => {
                self.addr = p;
                self.shift = p;
            }
            Some((Cmd::Wr, p)) => {
                self.write(self.addr, p);
                self.shift = self.addr;
            }
            Some((Cmd::Wri, p)) => {
                self.write(self.addr, p);
                self.addr = self.addr.wrapping_add(1);
                self.shift = self.addr;
            }
            Some((Cmd::Setaft, p)) => {
                self.addr = p;
                self.shift = self.read(p);
            }
            None => {}
        }
        out
    }

    pub fn pin(&mut self, name: &str) -> Option<bool> {
        match name {
            "PGOOD" => Some(self.env.sim_pgood),
            "RSTn" => Some(!self.rst_low),
            _ => None,
        }
    }

    pub fn set_pin(&mut self, name: &str, high: bool) {
        if name == "RSTn" {
            if !high {
                self.reset();
            }
            self.rst_low = !high;
        }
    }

    pub fn hop(&mut self, n: u8, ms: u32) {
        self.hop = if n >= 2 { Some((n, ms.max(1), Instant::now())) } else { None };
    }

    pub fn board(&self) -> BoardState {
        BoardState {
            fw: "симулятор".into(),
            remote: true,
            pgood: self.env.sim_pgood,
            apply_pressed: false,
            dip: Some(self.dip),
            err: if !self.env.sim_pgood { 4 } else if !self.env.sim_clock_ok { 2 } else { 0 },
            hop_n: self.hop.map(|h| h.0).unwrap_or(0),
            hop_ms: self.hop.map(|h| h.1).unwrap_or(0),
        }
    }

    pub fn poll(&mut self) -> SimStatus {
        let w = self.active();
        let d = model::derive(&w, &self.env);
        let c = &d.calc;
        let main = if c.ch1.on_dac1 { &c.ch1 } else { &c.ch2 };
        let lfm = main.lfm;
        let stage = self.lfm_stage(if c.ch1.on_dac1 { 1 } else { 2 });
        let (state, text) = if !self.env.sim_pgood {
            ("fault", "Нет +3,3 В (PGOOD)".to_string())
        } else if !self.env.sim_clock_ok {
            ("fault", "Нет такта на XW1".to_string())
        } else if self.rst_low {
            ("off", "DDS в сбросе (RSTn = 0)".to_string())
        } else if !c.dac1_on {
            ("off", "ЦАП 1 выключен".to_string())
        } else if lfm {
            let run = self.lfm_t0[0].is_some() && !self.lfm_stopped[0];
            ("busy", if run { format!("ЛЧМ: стадия {}", stage + 1) } else { "ЛЧМ ждёт запуска".to_string() })
        } else if let Some((n, ms, _)) = self.hop {
            ("busy", format!("Перебор {} профилей по {} мс", n, ms))
        } else {
            ("ok", format!("{:.6} МГц", main.fout / 1e6).replace('.', ","))
        };
        let _ = self.t0;
        SimStatus {
            state,
            text,
            signals: vec![
                SimSignal { name: "Профиль К1", value: regs::get(&w, "Pr_1") as f64, unit: "" },
                SimSignal { name: "Ток +1,8 В", value: if self.alive() { c.i_1v8 } else { 0.010 }, unit: "A" },
            ],
            pins: vec![("PGOOD".into(), self.env.sim_pgood), ("RSTn".into(), !self.rst_low)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(chip: &mut VirtualChip, a: u16, v: u16) {
        for f in bus::write_frames(a, v) {
            chip.spi(&f);
        }
    }

    fn read(chip: &mut VirtualChip, a: u16) -> u16 {
        let mut last = vec![];
        for f in bus::read_frames(a) {
            last = chip.spi(&f);
        }
        bus::reply_data(&last)
    }

    #[test]
    fn devid_and_pipeline() {
        let mut c = VirtualChip::new(Settings::default());
        assert_eq!(read(&mut c, 0x0001), 0x0201);
        // NOP returns DEVID in the next frame
        c.spi(&bus::frame(Cmd::Nop, 0));
        assert_eq!(bus::reply_data(&c.spi(&bus::frame(Cmd::Nop, 0))), 0x0201);
        assert_eq!(read(&mut c, 0x0003), 0x0002, "CTR reset");
        assert_eq!(read(&mut c, 0x0004), 0x4800, "SYNC reset");
    }

    #[test]
    fn write_read_and_reserved_bits() {
        let mut c = VirtualChip::new(Settings::default());
        write(&mut c, 0x1405, 0xFFFF); // Mul0: bits [2:0] reserved
        assert_eq!(read(&mut c, 0x1405), 0xFFF8);
        write(&mut c, 0x0001, 0x1234);
        assert_eq!(read(&mut c, 0x0001), 0x0201, "DEVID is read-only");
        write(&mut c, 0x17F5, 0x1238); // profile 63 exists in the chip
        assert_eq!(read(&mut c, 0x17F5), 0x1238);
    }

    #[test]
    fn wri_increments_and_all_broadcasts() {
        let mut c = VirtualChip::new(Settings::default());
        c.spi(&bus::frame(Cmd::Seta, 0x1400));
        for v in [0x1111, 0x2222, 0x3333] {
            c.spi(&bus::frame(Cmd::Wri, v));
        }
        assert_eq!(read(&mut c, 0x1402), 0x3333);
        write(&mut c, 0x1305, 1000 << 3); // CH1_Mul_all
        assert_eq!(read(&mut c, 0x1435), 1000 << 3);
        assert_eq!(read(&mut c, 0x17F5), 1000 << 3);
    }

    #[test]
    fn swrst_and_link_lock() {
        let mut c = VirtualChip::new(Settings::default());
        write(&mut c, 0x0003, 0x1000);
        write(&mut c, 0x0000, 0x0078);
        assert_eq!(read(&mut c, 0x0003), 0x0002);
        write(&mut c, 0x0006, 0x0001); // LINK on
        write(&mut c, 0x1400, 0xBEEF);
        assert_eq!(read(&mut c, 0x1400), 0, "LINK.on blocks ≥ 0x1000");
    }

    #[test]
    fn no_clock_kills_spi() {
        let mut c = VirtualChip::new(Settings { sim_clock_ok: false, ..Settings::default() });
        assert_eq!(read(&mut c, 0x0001), 0);
        assert_eq!(c.poll().state, "fault");
    }

    #[test]
    fn hop_cycles_profiles() {
        let mut c = VirtualChip::new(Settings::default());
        c.hop(4, 1);
        std::thread::sleep(std::time::Duration::from_millis(3));
        let p = read(&mut c, 0x0002) & 0x3F;
        assert!(p < 4);
        c.hop(0, 0);
        assert_eq!(read(&mut c, 0x0002) & 0x3F, 0);
    }
}
