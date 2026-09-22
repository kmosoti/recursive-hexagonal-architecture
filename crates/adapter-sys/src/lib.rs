//! The system clock, for the "built at" footer (DP-1.2). Display only.

use std::time::{SystemTime, UNIX_EPOCH};

/// UTC time from the system clock, formatted `YYYY-MM-DDTHH:MM:SSZ`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl site::Clock for SystemClock {
    fn now(&self) -> String {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        format_utc(secs)
    }
}

/// Seconds since the epoch as an RFC 3339 UTC timestamp.
#[must_use]
pub fn format_utc(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::format_utc;

    #[test]
    fn known_instants_format_correctly() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_utc(1_790_035_199), "2026-09-21T23:59:59Z");
    }
}
