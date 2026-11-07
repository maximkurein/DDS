//! Hardware links: the virtual chip and the board's own firmware over a USB-UART
//! adapter on X1 (USART0 of D2 1887ВЕ7Т: X1.1 = TXD0, X1.4 = RXD0, X1.6 = GND).
//!
//! Firmware text protocol (firmware/src/app/remote.c, docs/PROTOCOL_ru.md),
//! 38400 8N1, one command per line, replies end with \r\n. Only lines starting with
//! OK, ERR or PL8T are replies; everything else (the debug log, boot banner) is skipped:
//!   ?               → PL8T <fw> DDS48-52 uart1
//!   REMOTE 1|0      → OK            1 — приложение управляет DDS, DIP/S4 не опрашиваются
//!   W aaaa dddd     → OK            запись регистра (SETA + WR)
//!   R aaaa          → OK dddd       чтение регистра (SETAFT + NOP)
//!   S xxxxxx        → OK yyyyyy     один 24-битный кадр SPI
//!   RST             → OK            импульс RSTn: регистры DDS по умолчанию
//!   PIN RSTn 0|1    → OK            удерживать / отпустить сброс DDS
//!   HOP n ms        → OK            прошивка перебирает профили 0…n−1 канала 1 (n = 0 — стоп)
//!   ST              → OK pgood=1 apply=0 dip=2000 remote=1 err=0 hop=0 ms=0
//! Errors: ERR SPI | ERR POWER | ERR STATE | ERR ARG | ERR CMD | ERR LONG |
//!         ERR LOCAL (команда только в режиме REMOTE).

use crate::bus;
use crate::sim::{BoardState, VirtualChip};
use crate::{Error, Result};
use serde::Serialize;
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

/// Board signals the firmware exposes.
#[derive(Serialize, Clone, Copy, Debug)]
pub struct PinDef {
    pub name: &'static str,
    /// "out" (host drives) | "in" (board drives)
    pub dir: &'static str,
    /// Level after connect (outputs).
    pub idle: bool,
    pub title: &'static str,
}

pub const PINS: &[PinDef] = &[
    PinDef { name: "RSTn", dir: "out", idle: true, title: "Сброс DDS D1.10 (активный 0), через PB4 и буфер D3" },
    PinDef { name: "PGOOD", dir: "in", idle: true, title: "DDS_PGOOD (PC4): есть +3,3 В" },
];

pub const DEFAULT_BAUD: u32 = 38_400;

#[derive(Serialize, Clone, Debug)]
pub struct LinkInfo {
    /// "sim" | "serial"
    pub kind: &'static str,
    pub name: String,
    pub detail: String,
    pub readback: bool,
}

pub trait Link: Send {
    fn info(&self) -> LinkInfo;
    /// One 24-bit SPI frame. Returns the bytes seen on SDO.
    fn spi(&mut self, tx: &[u8]) -> Result<Vec<u8>>;
    /// Register write = SETA + WR.
    fn write_reg(&mut self, addr: u16, v: u16) -> Result<()> {
        for f in bus::write_frames(addr, v) {
            self.spi(&f)?;
        }
        Ok(())
    }
    /// Register read = SETAFT + NOP.
    fn read_reg(&mut self, addr: u16) -> Result<u16> {
        let mut last = vec![];
        for f in bus::read_frames(addr) {
            last = self.spi(&f)?;
        }
        Ok(bus::reply_data(&last))
    }
    fn set_pin(&mut self, pin: &str, high: bool) -> Result<()>;
    fn board(&mut self) -> Result<BoardState>;
    fn hop(&mut self, n: u8, ms: u32) -> Result<()>;
    /// Give the DDS back to the firmware (DIP switches + S4).
    fn release(&mut self) -> Result<()> {
        Ok(())
    }
    fn as_sim(&mut self) -> Option<&mut VirtualChip> {
        None
    }
}

impl Link for VirtualChip {
    fn info(&self) -> LinkInfo {
        LinkInfo { kind: "sim", name: "Виртуальная плата".into(), detail: format!("Модель 1508ПЛ8Т и платы ревизии F · {}", bus::describe()), readback: true }
    }
    fn spi(&mut self, tx: &[u8]) -> Result<Vec<u8>> {
        Ok(VirtualChip::spi(self, tx))
    }
    fn set_pin(&mut self, pin: &str, high: bool) -> Result<()> {
        VirtualChip::set_pin(self, pin, high);
        Ok(())
    }
    fn board(&mut self) -> Result<BoardState> {
        Ok(VirtualChip::board(self))
    }
    fn hop(&mut self, n: u8, ms: u32) -> Result<()> {
        VirtualChip::hop(self, n, ms);
        Ok(())
    }
    fn as_sim(&mut self) -> Option<&mut VirtualChip> {
        Some(self)
    }
}

/// The board's firmware (D2) over a USB-UART adapter.
pub struct FirmwareLink {
    port: Box<dyn serialport::SerialPort>,
    reader: BufReader<Box<dyn serialport::SerialPort>>,
    path: String,
    ident: String,
}

impl FirmwareLink {
    pub fn open(path: &str, baud: u32) -> Result<Self> {
        let port = serialport::new(path, baud)
            .timeout(Duration::from_millis(400))
            .open()
            .map_err(|e| Error::Link(format!("Не удалось открыть {}: {}", path, e)))?;
        let reader = BufReader::new(port.try_clone().map_err(|e| Error::Link(e.to_string()))?);
        let mut b = FirmwareLink { port, reader, path: path.to_string(), ident: String::new() };
        for _ in 0..6 {
            let _ = b.port.clear(serialport::ClearBuffer::Input);
            if let Ok(r) = b.raw("?") {
                if r.starts_with("PL8T") {
                    b.ident = r;
                    b.cmd("REMOTE 1")?;
                    return Ok(b);
                }
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        Err(Error::Link(format!(
            "{}: прошивка не ответила на «?». Проверьте: прошивка с протоколом UART (v1.1+), {} бод, TX адаптера → X1.4, RX → X1.1, GND → X1.6",
            path, baud
        )))
    }

    fn raw(&mut self, line: &str) -> Result<String> {
        self.port.write_all(format!("{}\n", line).as_bytes()).map_err(|e| Error::Link(e.to_string()))?;
        self.port.flush().ok();
        let mut s = String::new();
        loop {
            s.clear();
            self.reader.read_line(&mut s).map_err(|e| Error::Link(format!("таймаут ответа платы: {}", e)))?;
            let t = s.trim();
            if is_reply(t) {
                return Ok(t.to_string());
            }
        }
    }

    fn cmd(&mut self, line: &str) -> Result<String> {
        let r = self.raw(line)?;
        match r.strip_prefix("OK") {
            Some(rest) => Ok(rest.trim().to_string()),
            None => Err(Error::Link(match r.as_str() {
                "ERR POWER" => "плата: нет +3,3 В (PGOOD) — DDS выключен".to_string(),
                "ERR SPI" => "плата: SPI не завершил обмен".to_string(),
                "ERR LOCAL" => "плата в режиме DIP: нужна команда REMOTE 1".to_string(),
                "ERR STATE" => "плата: DDS ещё запускается после сброса или провала питания — повторите".to_string(),
                "ERR LONG" => "плата: слишком длинная строка команды".to_string(),
                _ => format!("плата ответила: {}", r),
            })),
        }
    }
}

/// A firmware reply (the rest is log output).
pub fn is_reply(line: &str) -> bool {
    line == "OK" || line.starts_with("OK ") || line.starts_with("ERR ") || line.starts_with("PL8T ")
}

/// Parse `pgood=1 apply=0 dip=2000 remote=1 err=0 hop=0 ms=0`.
pub fn parse_status(s: &str, fw: &str) -> BoardState {
    let mut b = BoardState { fw: fw.to_string(), ..Default::default() };
    for kv in s.split_whitespace() {
        let Some((k, v)) = kv.split_once('=') else { continue };
        let n: u32 = v.parse().unwrap_or(0);
        match k {
            "pgood" => b.pgood = n == 1,
            "apply" => b.apply_pressed = n == 1,
            "dip" => b.dip = v.parse().ok(),
            "remote" => b.remote = n == 1,
            "err" => b.err = n as u8,
            "hop" => b.hop_n = n as u8,
            "ms" => b.hop_ms = n,
            _ => {}
        }
    }
    b
}

impl Link for FirmwareLink {
    fn info(&self) -> LinkInfo {
        LinkInfo { kind: "serial", name: format!("Плата {}", self.path), detail: self.ident.clone(), readback: true }
    }
    fn spi(&mut self, tx: &[u8]) -> Result<Vec<u8>> {
        let r = self.cmd(&format!("S {}", bus::hex(tx)))?;
        bus::unhex(&r).ok_or_else(|| Error::Link(format!("неверный ответ: {}", r)))
    }
    fn write_reg(&mut self, addr: u16, v: u16) -> Result<()> {
        self.cmd(&format!("W {:04X} {:04X}", addr, v)).map(|_| ())
    }
    fn read_reg(&mut self, addr: u16) -> Result<u16> {
        let r = self.cmd(&format!("R {:04X}", addr))?;
        u16::from_str_radix(r.trim(), 16).map_err(|_| Error::Link(format!("неверный ответ: {}", r)))
    }
    fn set_pin(&mut self, pin: &str, high: bool) -> Result<()> {
        self.cmd(&format!("PIN {} {}", pin, high as u8)).map(|_| ())
    }
    fn board(&mut self) -> Result<BoardState> {
        let r = self.cmd("ST")?;
        Ok(parse_status(&r, &self.ident))
    }
    fn hop(&mut self, n: u8, ms: u32) -> Result<()> {
        self.cmd(&format!("HOP {} {}", n, ms)).map(|_| ())
    }
    fn release(&mut self) -> Result<()> {
        self.cmd("REMOTE 0").map(|_| ())
    }
}

// ───────────── discovery ─────────────

#[derive(Serialize, Clone, Debug)]
pub struct PortInfo {
    pub path: String,
    pub kind: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
}

pub fn list_ports() -> Vec<PortInfo> {
    let mut v: Vec<PortInfo> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| {
            let mut info = PortInfo { path: p.port_name.clone(), kind: "other".into(), vid: None, pid: None, manufacturer: None, product: None, serial: None };
            match p.port_type {
                serialport::SerialPortType::UsbPort(u) => {
                    info.kind = "usb".into();
                    info.vid = Some(u.vid);
                    info.pid = Some(u.pid);
                    info.manufacturer = u.manufacturer;
                    info.product = u.product;
                    info.serial = u.serial_number;
                }
                serialport::SerialPortType::BluetoothPort => info.kind = "bluetooth".into(),
                serialport::SerialPortType::PciPort => info.kind = "pci".into(),
                serialport::SerialPortType::Unknown => {}
            }
            info
        })
        .collect();
    // macOS lists /dev/tty.* and /dev/cu.* for the same device: keep cu.*
    if cfg!(target_os = "macos") {
        v.retain(|p| !p.path.starts_with("/dev/tty."));
    }
    v
}

#[derive(Serialize, Clone, Debug)]
pub struct UsbDev {
    pub vid: u16,
    pub pid: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub bus: u8,
    pub address: u8,
    pub hint: Option<&'static str>,
    pub likely_bridge: bool,
}

/// Vendor hints for USB devices commonly found on or next to evaluation boards.
/// Add the board's own interface chip (vendor ID from its user guide) at the top.
fn vendor_hint(vid: u16, pid: u16) -> (Option<&'static str>, bool) {
    match (vid, pid) {
        // USB-UART adapters that fit X1 (5 V logic): the usual way to reach the board's firmware
        (0x0403, 0x6001) | (0x0403, 0x6015) => (Some("FTDI FT232R/FT231X — USB-UART для X1 (выберите 5 В)"), true),
        (0x1A86, 0x7523) | (0x1A86, 0x55D4) => (Some("WCH CH340/CH9102 — USB-UART для X1"), true),
        (0x10C4, 0xEA60) => (Some("Silicon Labs CP2102 — USB-UART для X1 (логика 3,3 В, см. «Подключение»)"), true),
        (0x067B, 0x2303) => (Some("Prolific PL2303 — USB-UART для X1"), true),
        (0x03EB, 0x2141) => (Some("Atmel-ICE — программатор/отладчик для X3 (JTAG)"), false),
        (0x0403, _) => (Some("FTDI — типичный USB-SPI/I²C мост (MPSSE: FT232H/FT2232H/FT4232H)"), true),
        (0x04B4, _) => (Some("Cypress/Infineon — USB-мост (FX2LP / CY7C652xx)"), true),
        (0x10C4, 0x87A0) => (Some("Silicon Labs CP2130 — USB-SPI мост"), true),
        (0x10C4, 0xEA90) => (Some("Silicon Labs CP2112 — USB-I²C мост"), true),
        (0x10C4, _) => (Some("Silicon Labs — USB-мост (CP21xx)"), true),
        (0x04D8, 0x00DE) => (Some("Microchip MCP2210 — USB-SPI мост"), true),
        (0x04D8, 0x00DD) => (Some("Microchip MCP2221 — USB-I²C/UART мост"), true),
        (0x04D8, _) => (Some("Microchip — USB-устройство / микроконтроллер"), true),
        (0x0456, _) => (Some("Analog Devices — интерфейс оценочной платы (SDP/ADALM)"), true),
        (0x045B, _) => (Some("Renesas — интерфейс оценочной платы"), true),
        (0x0451, _) => (Some("Texas Instruments — USB-контроллер / USB2ANY"), true),
        (0x1FC9, _) => (Some("NXP — USB-контроллер"), true),
        (0x0483, 0x3748) | (0x0483, 0x374B) | (0x0483, 0x374E) | (0x0483, 0x374F) | (0x0483, 0x3752) | (0x0483, 0x3753) => {
            (Some("ST-LINK — Nucleo можно использовать как мост (firmware/bridge)"), false)
        }
        (0x0483, _) => (Some("STMicroelectronics — USB-устройство"), false),
        (0x2E8A, _) => (Some("Raspberry Pi RP2040/RP2350 — подходит как мост"), false),
        (0x303A, _) => (Some("Espressif ESP32-S2/S3/C3 — подходит как мост"), false),
        (0x2341, _) | (0x2A03, _) => (Some("Arduino"), false),
        (0x239A, _) => (Some("Adafruit"), false),
        (0x067B, _) => (Some("Prolific PL2303 — USB-UART (без SPI)"), false),
        (0x1A86, _) => (Some("WCH CH34x — USB-UART (CH341A умеет SPI/I²C)"), false),
        (0x05AC, _) => (Some("Apple"), false),
        _ => (None, false),
    }
}

pub fn scan_usb() -> Result<Vec<UsbDev>> {
    let list = nusb::list_devices().map_err(|e| Error::Link(format!("USB: {}", e)))?;
    let mut v: Vec<UsbDev> = list
        .filter(|d| d.class() != 0x09) // skip hubs
        .map(|d| {
            let (hint, likely) = vendor_hint(d.vendor_id(), d.product_id());
            UsbDev {
                vid: d.vendor_id(),
                pid: d.product_id(),
                manufacturer: d.manufacturer_string().map(|s| s.to_string()),
                product: d.product_string().map(|s| s.to_string()),
                serial: d.serial_number().map(|s| s.to_string()),
                bus: d.bus_number(),
                address: d.device_address(),
                hint,
                likely_bridge: likely,
            }
        })
        .collect();
    v.sort_by_key(|d| (!d.likely_bridge, d.vid, d.pid));
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_vs_log() {
        assert!(is_reply("OK"));
        assert!(is_reply("OK 0201"));
        assert!(is_reply("ERR LOCAL"));
        assert!(is_reply("PL8T 1.1.0+abc DDS48-52 uart1"));
        assert!(!is_reply("[app] N=2000 f=50000000 Hz"));
        assert!(!is_reply("DDS 48-52 MHz, fw 1.1.0"));
        assert!(!is_reply(""));
        assert!(!is_reply("OKAY"));
    }

    #[test]
    fn status_line() {
        let b = parse_status("pgood=1 apply=0 dip=2000 remote=1 err=0 hop=4 ms=250", "PL8T 1.1.0");
        assert!(b.pgood && b.remote && !b.apply_pressed);
        assert_eq!(b.dip, Some(2000));
        assert_eq!((b.hop_n, b.hop_ms), (4, 250));
    }
}
