//! Serial port of the 1508ПЛ8Т (datasheet section 6.3, table 6.1).
//!
//! - one command = 24 bits, MSB first: [23:16] opcode, [15:0] parameter;
//! - SDI is sampled on the rising SCK edge, SDO changes on the falling edge (SPI mode 0);
//! - the command executes on the rising edge of SCSn, so SCSn rises after every word;
//! - after a command the shift register is loaded with a new value (table 6.1) that is
//!   shifted out on SDO during the NEXT frame. Reading a register is therefore
//!   SETAFT(addr) followed by any command (NOP): the data are the low 16 bits of the
//!   second reply.
//!
//! A register write is SETA(addr) + WR(data). The simulator decodes these very
//! frames, so a framing bug fails `cargo test` before it reaches the board.

/// Opcodes, table 6.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// No operation; shift register ← DEVID (0x0201).
    Nop = 0x00,
    /// Address register ← addr.
    Seta = 0x10,
    /// [address] ← data.
    Wr = 0x20,
    /// [address] ← data, then address + 1.
    Wri = 0x30,
    /// Address register ← addr, shift register ← [addr].
    Setaft = 0xB0,
}

impl Cmd {
    pub fn from_u8(v: u8) -> Option<Cmd> {
        Some(match v {
            0x00 => Cmd::Nop,
            0x10 => Cmd::Seta,
            0x20 => Cmd::Wr,
            0x30 => Cmd::Wri,
            0xB0 => Cmd::Setaft,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Cmd::Nop => "NOP",
            Cmd::Seta => "SETA",
            Cmd::Wr => "WR",
            Cmd::Wri => "WRI",
            Cmd::Setaft => "SETAFT",
        }
    }
}

/// SPI mode and clock of the board: ATmega128 SPI, prescaler 128 at 8 МГц
/// (firmware/config/dds_config.h, DDS_SPI_PRESCALER). The chip itself allows
/// tSCLK ≥ max(20 нс, 10·tCLK) (table 9.1); the level shifters D3/D4 with 1 кОм
/// series resistors limit the board to ≈ 100 кГц (docs/design_notes.md).
pub const SPI_MODE: u8 = 0;
pub const SPI_BOARD_HZ: u32 = 62_500;

/// One 24-bit frame.
pub fn frame(cmd: Cmd, param: u16) -> [u8; 3] {
    [cmd as u8, (param >> 8) as u8, param as u8]
}

/// Frames of a register write: SETA addr, WR data.
pub fn write_frames(addr: u16, value: u16) -> Vec<[u8; 3]> {
    vec![frame(Cmd::Seta, addr), frame(Cmd::Wr, value)]
}

/// Frames of a register read: SETAFT addr, NOP. Data = low 16 bits of the last reply.
pub fn read_frames(addr: u16) -> Vec<[u8; 3]> {
    vec![frame(Cmd::Setaft, addr), frame(Cmd::Nop, 0)]
}

/// Data word carried by a 24-bit reply.
pub fn reply_data(rx: &[u8]) -> u16 {
    if rx.len() < 3 {
        return 0;
    }
    u16::from_be_bytes([rx[1], rx[2]])
}

/// Decode a frame: (opcode, parameter). `None` for wrong length or unknown opcode.
pub fn decode(tx: &[u8]) -> Option<(Cmd, u16)> {
    if tx.len() != 3 {
        return None;
    }
    Some((Cmd::from_u8(tx[0])?, u16::from_be_bytes([tx[1], tx[2]])))
}

/// Human-readable frame, e.g. "SETA 1400".
pub fn describe_frame(tx: &[u8]) -> String {
    match decode(tx) {
        Some((c, p)) => format!("{} {:04X}", c.name(), p),
        None => format!("? {}", hex(tx)),
    }
}

pub fn describe() -> String {
    format!("SPI режим {}, 24-битные команды (SETA/WR/SETAFT), {} кГц на плате", SPI_MODE, SPI_BOARD_HZ / 1000)
}

pub fn can_read() -> bool {
    true
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02X}", x)).collect()
}

pub fn unhex(s: &str) -> Option<Vec<u8>> {
    let s: String = s.chars().filter(|c| !c.is_whitespace() && *c != '_').collect();
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(&s);
    if s.is_empty() || !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_match_firmware() {
        // firmware/src/drivers/pl8t.c: frame(cmd, param) = cmd << 16 | param
        assert_eq!(write_frames(0x1400, 0xABCD), vec![[0x10, 0x14, 0x00], [0x20, 0xAB, 0xCD]]);
        assert_eq!(read_frames(0x0001), vec![[0xB0, 0x00, 0x01], [0x00, 0x00, 0x00]]);
        assert_eq!(decode(&[0x30, 0x12, 0x34]), Some((Cmd::Wri, 0x1234)));
        assert_eq!(decode(&[0x40, 0, 0]), None);
        assert_eq!(reply_data(&[0x00, 0x02, 0x01]), 0x0201);
        assert_eq!(describe_frame(&[0xB0, 0x00, 0x03]), "SETAFT 0003");
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(unhex("0x01ab FF"), Some(vec![0x01, 0xAB, 0xFF]));
        assert_eq!(hex(&[0x01, 0xAB]), "01AB");
        assert_eq!(unhex("123"), None);
    }
}
