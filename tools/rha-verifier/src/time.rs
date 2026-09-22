/// Nanoseconds since the epoch for an RFC 3339 timestamp with `Z` or a
/// `±HH:MM` offset. Fractional seconds are kept to the nanosecond, so a
/// cooling-off or issue time 0.9 s in the future is not rounded into the past
/// (review round 1, finding 1). `None` if malformed, including impossible
/// dates such as 30 February.
#[must_use]
pub fn parse_rfc3339(text: &str) -> Option<i128> {
    let b = text.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let s = b.get(r)?;
        s.iter()
            .all(u8::is_ascii_digit)
            .then(|| s.iter().fold(0i64, |n, d| n * 10 + i64::from(d - b'0')))
    };
    let (y, mo, d, h, mi, s) = (
        num(0..4)?,
        num(5..7)?,
        num(8..10)?,
        num(11..13)?,
        num(14..16)?,
        num(17..19)?,
    );
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days_in_month = match mo {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if d < 1 || d > days_in_month || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let mut i = 19;
    let mut nanos: i128 = 0;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            if i - start < 9 {
                nanos = nanos * 10 + i128::from(b[i] - b'0');
            }
            i += 1;
        }
        if i == start {
            return None;
        }
        for _ in (i - start)..9 {
            nanos *= 10;
        }
    }
    let offset: i64 = match &b[i..] {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2]
            if [h1, h2, m1, m2].iter().all(|c| c.is_ascii_digit()) =>
        {
            let oh = i64::from((h1 - b'0') * 10 + (h2 - b'0'));
            let om = i64::from((m1 - b'0') * 10 + (m2 - b'0'));
            if oh > 23 || om > 59 {
                return None;
            }
            (if *sign == b'-' { -1 } else { 1 }) * (oh * 3600 + om * 60)
        }
        _ => return None,
    };
    let yy = if mo <= 2 { y - 1 } else { y };
    let era = yy.div_euclid(400);
    let yoe = yy - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + h * 3600 + mi * 60 + s - offset;
    Some(i128::from(secs) * 1_000_000_000 + nanos)
}

#[cfg(test)]
mod tests {
    use super::parse_rfc3339;

    const S: i128 = 1_000_000_000;

    #[test]
    fn timestamps() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339("1970-01-01T01:00:00+01:00"), Some(0));
        assert_eq!(
            parse_rfc3339("2000-03-01T00:00:00.5Z"),
            Some(951_868_800 * S + S / 2)
        );
        assert_eq!(
            parse_rfc3339("2026-09-21T12:00:00.900Z").map(|n| n % S),
            Some(900_000_000)
        );
        assert_eq!(parse_rfc3339("2026-09-22"), None);
        assert_eq!(parse_rfc3339("2026-13-01T00:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-02-30T00:00:00Z"), None);
        assert!(parse_rfc3339("2024-02-29T00:00:00Z").is_some());
        assert_eq!(parse_rfc3339("2026-09-22T12:00:00+25:00"), None);
        assert_eq!(parse_rfc3339("2026-09-22T12:00:00\u{e9}xxxx"), None);
    }
}
