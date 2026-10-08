//! Conversions between whole-coin amounts typed by people ("12.5") and base units.
//! SUI, IOTA and IKA all use 9 decimals.

use anyhow::{Result, bail};

pub const DECIMALS: u32 = 9;

pub fn parse(text: &str) -> Result<u64> {
    let text = text.trim().replace('_', "");
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty()) || !digits(whole) || !digits(fraction) {
        bail!("invalid amount {text:?}: expected a number like 12.5");
    }
    if fraction.len() > DECIMALS as usize {
        bail!("invalid amount {text:?}: at most {DECIMALS} decimal places");
    }
    let scale = 10u128.pow(DECIMALS);
    let whole: u128 = if whole.is_empty() { 0 } else { whole.parse()? };
    let fraction: u128 = format!("{fraction:0<width$}", width = DECIMALS as usize).parse()?;
    let value = whole
        .checked_mul(scale)
        .and_then(|w| w.checked_add(fraction))
        .filter(|v| *v <= u64::MAX as u128);
    match value {
        Some(0) => bail!("amount must be greater than zero"),
        Some(v) => Ok(v as u64),
        None => bail!("amount {text} is too large"),
    }
}

/// Formats base units as whole coins with thousands separators: 1234500000000 -> "1,234.5".
pub fn format(base_units: i128) -> String {
    let scale = 10i128.pow(DECIMALS);
    let sign = if base_units < 0 { "-" } else { "" };
    let abs = base_units.unsigned_abs();
    let whole = (abs / scale as u128).to_string();
    let fraction = abs % scale as u128;

    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    if fraction == 0 {
        return format!("{sign}{grouped}");
    }
    let fraction = format!("{fraction:0width$}", width = DECIMALS as usize);
    format!("{sign}{grouped}.{}", fraction.trim_end_matches('0'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_whole_coins() {
        assert_eq!(parse("1").unwrap(), 1_000_000_000);
        assert_eq!(parse("12.5").unwrap(), 12_500_000_000);
        assert_eq!(parse(".000000001").unwrap(), 1);
        assert_eq!(parse("1_000").unwrap(), 1_000_000_000_000);
        assert!(parse("0").is_err());
        assert!(parse("1.0000000001").is_err());
        assert!(parse("-1").is_err());
        assert!(parse("1e9").is_err());
        assert!(parse("").is_err());
        assert!(parse("99999999999").is_err());
    }

    #[test]
    fn formats_base_units() {
        assert_eq!(format(0), "0");
        assert_eq!(format(1), "0.000000001");
        assert_eq!(format(736_780_171_792_602), "736,780.171792602");
        assert_eq!(format(-2_500_000_000), "-2.5");
        assert_eq!(format(1_000_000_000_000), "1,000");
    }
}
