//! Small shared helpers: time, paths, names.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// `2026-11-21T18:17:21.653Z` -> unix milliseconds.
pub fn parse_iso_ms(s: &str) -> Option<i64> {
    let num = |a: usize, b: usize| -> Option<i64> { s.get(a..b)?.parse::<i64>().ok() };
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (h, mi, sec) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let mut ms = ((days_from_civil(y, mo, d) * 24 + h) * 60 + mi) * 60 + sec;
    ms *= 1000;
    if let Some(rest) = s.get(19..) {
        if let Some(frac) = rest.strip_prefix('.') {
            let digits: String = frac.chars().take_while(|c| c.is_ascii_digit()).take(3).collect();
            if !digits.is_empty() {
                let v: i64 = digits.parse().unwrap_or(0);
                ms += v * 10i64.pow(3 - digits.len() as u32);
            }
        }
    }
    Some(ms)
}

/// `5025000` -> `1h 23m`.
pub fn fmt_duration(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    let (d, h, m, sec) = (s / 86400, (s / 3600) % 24, (s / 60) % 60, s % 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {sec}s")
    } else {
        format!("{sec}s")
    }
}

/// `6750` -> `6,750`.
pub fn fmt_n(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Folder for settings, caches and snapshots.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("OrdisNexus")
}

/// Where AlecaFrame keeps the encrypted inventory snapshot.
pub fn alecaframe_data_file() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
    Some(base.join("AlecaFrame").join("lastData.dat"))
}

/// Where Warframe writes its log.
pub fn default_ee_log() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
    Some(base.join("Warframe").join("EE.log"))
}

/// A prime part exists as an unbuilt blueprint and as a built component. Given
/// either path, returns the blueprint form (or the path itself when there is no pair).
pub fn blueprint_form(unique: &str) -> String {
    match unique.strip_suffix("Component") {
        Some(base) => format!("{base}Blueprint"),
        None => unique.to_string(),
    }
}

pub fn component_form(unique: &str) -> String {
    match unique.strip_suffix("Blueprint") {
        Some(base) => format!("{base}Component"),
        None => unique.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso() {
        assert_eq!(parse_iso_ms("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(parse_iso_ms("1970-01-02T00:00:01.500Z"), Some(86_401_500));
        assert_eq!(parse_iso_ms("2026-11-21T18:17:21.653Z"), Some(1_795_285_041_653));
        assert_eq!(parse_iso_ms("nonsense"), None);
    }

    #[test]
    fn durations() {
        assert_eq!(fmt_duration(5_025_000), "1h 23m");
        assert_eq!(fmt_duration(45_000), "45s");
        assert_eq!(fmt_duration(-5), "0s");
    }

    #[test]
    fn thousands() {
        assert_eq!((fmt_n(0), fmt_n(999), fmt_n(6750), fmt_n(1234567)), ("0".into(), "999".into(), "6,750".into(), "1,234,567".into()));
    }

    #[test]
    fn forms() {
        assert_eq!(blueprint_form("/A/WispPrimeSystemsComponent"), "/A/WispPrimeSystemsBlueprint");
        assert_eq!(component_form("/A/WispPrimeSystemsBlueprint"), "/A/WispPrimeSystemsComponent");
        assert_eq!(blueprint_form("/A/AcceltraPrimeStock"), "/A/AcceltraPrimeStock");
    }
}
