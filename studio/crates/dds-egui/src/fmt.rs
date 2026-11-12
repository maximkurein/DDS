//! Display formatting only — no engineering math here (that lives in dds-core).
//! Mirrors `src/lib/format.ts`.

const PREFIX: [(f64, &str); 8] =
    [(1e9, "Г"), (1e6, "М"), (1e3, "к"), (1.0, ""), (1e-3, "м"), (1e-6, "мк"), (1e-9, "н"), (1e-12, "п")];

fn unit_ru(unit: &str) -> Option<&'static str> {
    Some(match unit {
        "V" => "В",
        "A" => "А",
        "W" => "Вт",
        "Hz" => "Гц",
        "s" => "с",
        "F" => "Ф",
        "Ohm" | "Ω" => "Ом",
        _ => return None,
    })
}

/// Logarithmic units: no SI prefixes, Russian names.
fn log_ru(unit: &str) -> Option<&'static str> {
    Some(match unit {
        "dBm" => "дБм",
        "dB" => "дБ",
        "dBc" => "дБн",
        "dBFS" => "дБFS",
        _ => return None,
    })
}

fn trim(s: String) -> String {
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" {
        "0".into()
    } else {
        s
    }
}

/// `digits` significant digits, trailing zeros trimmed (JS `toPrecision` + trim).
pub fn sig(v: f64, digits: usize) -> String {
    if v == 0.0 || !v.is_finite() {
        return if v == 0.0 { "0".into() } else { "—".into() };
    }
    let exp = v.abs().log10().floor() as i32;
    let dec = (digits as i32 - 1 - exp).max(0) as usize;
    trim(format!("{:.*}", dec, v))
}

/// Engineering format with SI prefix: `eng(0.0025, "V", 4)` → "2.5 мВ".
/// Non-SI units are printed as-is.
pub fn eng(v: f64, unit: &str, digits: usize) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    if let Some(lu) = log_ru(unit) {
        let dec = (digits as i32 - 2).clamp(0, 3) as usize;
        return format!("{} {}", trim(format!("{:.*}", dec, v)), lu);
    }
    let Some(u) = unit_ru(unit) else {
        let s = if v.abs() >= 1000.0 { format!("{:.0}", v) } else { sig(v, digits) };
        return if unit.is_empty() { s } else { format!("{} {}", s, unit) };
    };
    if v == 0.0 {
        return format!("0 {}", u);
    }
    let a = v.abs();
    let (k, p) = PREFIX.iter().copied().find(|(k, _)| a >= k * 0.9995).unwrap_or(PREFIX[PREFIX.len() - 1]);
    format!("{} {}{}", sig(v / k, digits), p, u)
}

/// `eng` with the default 4 digits.
pub fn eng4(v: f64, unit: &str) -> String {
    eng(v, unit, 4)
}

/// Parse "2.5m", "4.7 мк", "10k", "1e-3", "250 мВ", "50 МГц" → base-unit number.
pub fn parse_eng(input: &str) -> Option<f64> {
    let s: String = input.trim().replace(',', ".").chars().filter(|c| !c.is_whitespace()).collect();
    let num_end = s.char_indices().take_while(|(_, c)| c.is_ascii_digit() || "+-.eE".contains(*c)).count();
    let chars: Vec<char> = s.chars().collect();
    let (v, rest) = (1..=num_end).rev().find_map(|n| {
        let head: String = chars[..n].iter().collect();
        head.parse::<f64>().ok().map(|v| (v, chars[n..].iter().collect::<String>()))
    })?;
    let lower = rest.to_lowercase();
    let mut pre = rest.as_str();
    for u in ["ohm", "ом", "гц", "hz", "вт", "в", "v", "а", "a", "w", "ф", "f", "с", "s"] {
        if lower.ends_with(u) {
            let cut = rest.chars().count() - u.chars().count();
            let idx = rest.char_indices().nth(cut).map(|(i, _)| i).unwrap_or(rest.len());
            pre = &rest[..idx];
            break;
        }
    }
    let k = match pre {
        "" => 1.0,
        "p" | "п" => 1e-12,
        "n" | "н" => 1e-9,
        "u" | "µ" | "мк" => 1e-6,
        "m" | "м" => 1e-3,
        "k" | "к" | "K" | "К" => 1e3,
        "M" | "М" | "meg" => 1e6,
        "G" | "Г" => 1e9,
        _ => return None,
    };
    Some(v * k)
}

/// "0x1FF" with `bits / 4` hex digits.
pub fn hex_w(v: u32, bits: u32) -> String {
    format!("0x{:0w$X}", v, w = bits.div_ceil(4) as usize)
}

/// Compact frequency for plot axes: 100 МГц, 1.5 ГГц, 250 кГц.
pub fn axis_hz(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let a = v.abs();
    let (k, p) = if a >= 1e9 {
        (1e9, "ГГц")
    } else if a >= 1e6 {
        (1e6, "МГц")
    } else if a >= 1e3 {
        (1e3, "кГц")
    } else {
        (1.0, "Гц")
    };
    format!("{} {}", sig(v / k, 3), p)
}

/// Integer with thin groups: 29 320 310 074 027 (like `toLocaleString("ru-RU")`).
pub fn grouped(v: i128) -> String {
    let s = v.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{a0}');
        }
        out.push(c);
    }
    if v < 0 {
        format!("-{}", out)
    } else {
        out
    }
}

/// Fixed decimals (`toFixed`).
pub fn fixed(v: f64, dec: usize) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    let s = format!("{:.*}", dec, v);
    if s.starts_with("-0") && s.trim_start_matches(['-', '0', '.']).is_empty() {
        s[1..].to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(eng(0.0025, "V", 4), "2.5 мВ");
        assert_eq!(eng(50e6, "Hz", 4), "50 МГц");
        assert_eq!(eng(49.5e6, "Hz", 6), "49.5 МГц");
        assert_eq!(eng(1.705e-6, "Hz", 4), "1.705 мкГц");
        assert_eq!(eng(-9.84, "dBm", 4), "-9.84 дБм");
        assert_eq!(eng(f64::NAN, "V", 3), "—");
        assert_eq!(eng(4010.0, "", 4), "4010");
        assert_eq!(hex_w(0x201, 16), "0x0201");
        assert_eq!(axis_hz(430e6), "430 МГц");
        assert_eq!(axis_hz(1.5e9), "1.5 ГГц");
        assert_eq!(grouped(29_320_310_074_027), "29\u{a0}320\u{a0}310\u{a0}074\u{a0}027");
        assert_eq!(fixed(-0.001, 1), "0.0");
    }

    #[test]
    fn parses() {
        assert_eq!(parse_eng("50 МГц"), Some(50e6));
        assert_eq!(parse_eng("2.5m"), Some(2.5e-3));
        assert_eq!(parse_eng("250k"), Some(250e3));
        assert_eq!(parse_eng("1 мс"), Some(1e-3));
        assert_eq!(parse_eng("3,3"), Some(3.3));
        assert_eq!(parse_eng("1e-3"), Some(1e-3));
        assert_eq!(parse_eng("abc"), None);
    }
}
