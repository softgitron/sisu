//! Parsing and printing of byte sizes and durations used on the command line.
//!
//! Sizes are binary: `4K` is 4096 bytes, `1M` is 1 MiB, `8MiB` and `8m` mean the
//! same. A plain number is bytes.

use std::time::Duration;

/// One kibibyte.
pub const KIB: usize = 1024;
/// One mebibyte.
pub const MIB: usize = 1024 * 1024;
/// One gibibyte.
pub const GIB: usize = 1024 * 1024 * 1024;

/// Parses a size such as `4096`, `16K`, `1M`, `8MiB` or `1G`.
pub fn parse_size(text: &str) -> Result<usize, String> {
    let t = text.trim();
    let split = t
        .find(|c: char| !c.is_ascii_digit() && c != '_')
        .unwrap_or(t.len());
    let (num, suffix) = t.split_at(split);
    let num: usize = num
        .replace('_', "")
        .parse()
        .map_err(|_| format!("not a size: {text:?}"))?;
    let mult = match suffix.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" | "kib" => KIB,
        "m" | "mb" | "mib" => MIB,
        "g" | "gb" | "gib" => GIB,
        other => return Err(format!("unknown size suffix {other:?} in {text:?}")),
    };
    num.checked_mul(mult)
        .ok_or_else(|| format!("size overflows: {text:?}"))
}

/// Parses a count such as `100000`, `100_000`, `100k` or `1M` (decimal multipliers).
pub fn parse_count(text: &str) -> Result<u64, String> {
    let t = text.trim().replace('_', "");
    let (num, mult) = match t.chars().last() {
        Some('k') | Some('K') => (&t[..t.len() - 1], 1_000),
        Some('m') | Some('M') => (&t[..t.len() - 1], 1_000_000),
        _ => (&t[..], 1),
    };
    let n: u64 = num.parse().map_err(|_| format!("not a count: {text:?}"))?;
    n.checked_mul(mult)
        .ok_or_else(|| format!("count overflows: {text:?}"))
}

/// Parses seconds, fractional allowed: `3`, `0.5`.
pub fn parse_secs(text: &str) -> Result<Duration, String> {
    let s: f64 = text
        .trim()
        .parse()
        .map_err(|_| format!("not a number of seconds: {text:?}"))?;
    if !(0.0..=86_400.0).contains(&s) {
        return Err(format!("seconds out of range: {text:?}"));
    }
    Ok(Duration::from_secs_f64(s))
}

/// Formats a byte count compactly: `4K`, `1M`, `1.5M`, `123B`.
pub fn fmt_size(bytes: usize) -> String {
    if bytes >= GIB && bytes % GIB == 0 {
        format!("{}G", bytes / GIB)
    } else if bytes >= MIB && bytes % MIB == 0 {
        format!("{}M", bytes / MIB)
    } else if bytes >= KIB && bytes % KIB == 0 {
        format!("{}K", bytes / KIB)
    } else if bytes >= MIB {
        format!("{:.1}M", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1}K", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes}B")
    }
}

/// Rounds `v` up to a multiple of `align`, which must be a power of two.
pub const fn round_up(v: usize, align: usize) -> usize {
    (v + align - 1) & !(align - 1)
}

/// Rounds `v` down to a multiple of `align`, which must be a power of two.
pub const fn round_down(v: usize, align: usize) -> usize {
    v & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_parse() {
        assert_eq!(parse_size("4096").unwrap(), 4096);
        assert_eq!(parse_size("16K").unwrap(), 16 * KIB);
        assert_eq!(parse_size("1M").unwrap(), MIB);
        assert_eq!(parse_size("8MiB").unwrap(), 8 * MIB);
        assert_eq!(parse_size("128k").unwrap(), 128 * KIB);
        assert_eq!(parse_size("1G").unwrap(), GIB);
        assert!(parse_size("1X").is_err());
        assert!(parse_size("").is_err());
    }

    #[test]
    fn counts_parse() {
        assert_eq!(parse_count("100_000").unwrap(), 100_000);
        assert_eq!(parse_count("10k").unwrap(), 10_000);
        assert_eq!(parse_count("1M").unwrap(), 1_000_000);
        assert!(parse_count("x").is_err());
    }

    #[test]
    fn sizes_format() {
        assert_eq!(fmt_size(4096), "4K");
        assert_eq!(fmt_size(MIB), "1M");
        assert_eq!(fmt_size(8 * MIB), "8M");
        assert_eq!(fmt_size(100), "100B");
        assert_eq!(round_up(4097, 4096), 8192);
        assert_eq!(round_down(4097, 4096), 4096);
    }
}
