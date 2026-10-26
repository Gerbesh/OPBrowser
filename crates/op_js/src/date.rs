//! First-party bounded ECMAScript Date UTC calendar primitives.
//! TimeClip follows the JS millisecond range; local-time-zone APIs remain future work.
const DAY_MS: i64 = 86_400_000;

pub(crate) fn clip(milliseconds: f64) -> f64 {
    if milliseconds.is_finite() && milliseconds.abs() <= 8.64e15 {
        milliseconds.trunc() + 0.0
    } else {
        f64::NAN
    }
}

fn leap(year: i64) -> bool {
    year.rem_euclid(4) == 0 && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
}

fn month_days(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

// Gregorian calendar algorithms, all arithmetic signed to support pre-1970.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

/// UTC date components: year, 0-based month, day, hour, minute, second, millisecond.
pub(crate) fn fields(milliseconds: f64) -> Option<[i64; 7]> {
    let clipped = clip(milliseconds);
    if clipped.is_nan() {
        return None;
    }
    let millis = clipped as i64;
    let days = millis.div_euclid(DAY_MS);
    let day_time = millis.rem_euclid(DAY_MS);
    let (year, month, day) = civil_from_days(days);
    Some([
        year,
        month - 1,
        day,
        day_time / 3_600_000,
        day_time / 60_000 % 60,
        day_time / 1_000 % 60,
        day_time % 1_000,
    ])
}

/// UTC field normalization follows MakeDay/MakeTime for supported bounded values.
pub(crate) fn from_fields(input: [f64; 7], adjust_year_0_99: bool) -> f64 {
    if input
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 10_000_000.0)
    {
        return f64::NAN;
    }
    let [year, month, day, hour, minute, second, millis] = input.map(|v| v.trunc() as i64);
    let year = if adjust_year_0_99 && (0..=99).contains(&year) {
        1900 + year
    } else {
        year
    };
    let year = year + month.div_euclid(12);
    let month = month.rem_euclid(12) + 1;
    let days = days_from_civil(year, month, 1) + day - 1;
    clip((days * DAY_MS + hour * 3_600_000 + minute * 60_000 + second * 1_000 + millis) as f64)
}

fn parse_digits(s: &str, count: usize) -> Option<i64> {
    (s.len() == count && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse::<i64>().ok())
        .flatten()
}

/// Strict ISO YYYY-MM-DD and YYYY-MM-DDTHH:mm[:ss[.sss]][Z|+HH:mm|-HH:mm].
/// An offset-free date-time is deliberately UTC pending browser timezone support.
pub(crate) fn parse_iso(text: &str) -> f64 {
    let (date, time) = match text.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (text, None),
    };
    if date.len() != 10
        || date.as_bytes().get(4) != Some(&b'-')
        || date.as_bytes().get(7) != Some(&b'-')
    {
        return f64::NAN;
    }
    let Some(year) = parse_digits(&date[0..4], 4) else {
        return f64::NAN;
    };
    let Some(month) = parse_digits(&date[5..7], 2) else {
        return f64::NAN;
    };
    let Some(day) = parse_digits(&date[8..10], 2) else {
        return f64::NAN;
    };
    if !(1..=12).contains(&month) || !(1..=month_days(year, month)).contains(&day) {
        return f64::NAN;
    }
    let mut parts = [0_i64; 4];
    let mut offset_minutes = 0_i64;
    if let Some(raw) = time {
        let (raw, offset) = if let Some(part) = raw.strip_suffix('Z') {
            (part, None)
        } else if let Some(i) = raw.bytes().position(|b| b == b'+' || b == b'-') {
            (&raw[..i], Some(&raw[i..]))
        } else {
            (raw, None)
        };
        if let Some(off) = offset {
            if off.len() != 6 || off.as_bytes()[3] != b':' {
                return f64::NAN;
            }
            let Some(h) = parse_digits(&off[1..3], 2) else {
                return f64::NAN;
            };
            let Some(m) = parse_digits(&off[4..6], 2) else {
                return f64::NAN;
            };
            if h > 23 || m > 59 {
                return f64::NAN;
            }
            offset_minutes = (h * 60 + m) * if off.starts_with('+') { 1 } else { -1 };
        }
        let (clock, frac) = raw
            .split_once('.')
            .map_or((raw, None), |(a, b)| (a, Some(b)));
        let components: Vec<_> = clock.split(':').collect();
        if !(2..=3).contains(&components.len()) {
            return f64::NAN;
        }
        for (i, component) in components.iter().enumerate() {
            let Some(number) = parse_digits(component, 2) else {
                return f64::NAN;
            };
            parts[i] = number;
        }
        if let Some(frac) = frac {
            if components.len() != 3
                || frac.is_empty()
                || frac.len() > 3
                || !frac.bytes().all(|b| b.is_ascii_digit())
            {
                return f64::NAN;
            }
            parts[3] =
                frac.parse::<i64>().unwrap_or_default() * 10_i64.pow((3 - frac.len()) as u32);
        }
        if parts[0] > 23 || parts[1] > 59 || parts[2] > 59 {
            return f64::NAN;
        }
    }
    let epoch = from_fields(
        [
            year as f64,
            (month - 1) as f64,
            day as f64,
            parts[0] as f64,
            parts[1] as f64,
            parts[2] as f64,
            parts[3] as f64,
        ],
        false,
    );
    clip(epoch - (offset_minutes * 60_000) as f64)
}

pub(crate) fn iso(milliseconds: f64) -> Option<String> {
    let [year, month, day, hour, minute, second, millis] = fields(milliseconds)?;
    let year = if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else if year < 0 {
        format!("-{:06}", -year)
    } else {
        format!("+{year:06}")
    };
    Some(format!(
        "{year}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z",
        month = month + 1
    ))
}

pub(crate) fn utc_string(milliseconds: f64) -> String {
    let Some([year, month, day, hour, minute, second, _]) = fields(milliseconds) else {
        return "Invalid Date".into();
    };
    let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let weekday = (milliseconds as i64)
        .div_euclid(DAY_MS)
        .saturating_add(4)
        .rem_euclid(7) as usize;
    format!(
        "{}, {day:02} {} {year:04} {hour:02}:{minute:02}:{second:02} GMT",
        names[weekday], months[month as usize]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leap_year_epoch_and_negative_time() {
        assert_eq!(iso(0.0).as_deref(), Some("1970-01-01T00:00:00.000Z"));
        assert_eq!(iso(-1.0).as_deref(), Some("1969-12-31T23:59:59.999Z"));
        assert_eq!(
            parse_iso("2024-02-29T12:13:14.125Z"),
            from_fields([2024., 1., 29., 12., 13., 14., 125.], false)
        );
        assert!(parse_iso("2023-02-29").is_nan());
        assert!(parse_iso("2024-13-29").is_nan());
        assert_eq!(parse_iso("1970-01-01T01:00+01:00"), 0.0);
        assert_eq!(parse_iso("1970-01-01T01:00-01:00"), 7_200_000.0);
        assert!(clip(8.64e15 + 1.0).is_nan());
        assert_eq!(clip(-1.9), -1.0);
    }
    #[test]
    fn utc_component_roundtrip() {
        for [year, month, day] in [
            [1970, 0, 1],
            [1900, 1, 28],
            [2000, 1, 29],
            [2038, 0, 19],
            [1969, 11, 31],
            [-1, 0, 1],
        ] {
            let stamp = from_fields(
                [year as f64, month as f64, day as f64, 0., 0., 0., 0.],
                false,
            );
            assert_eq!(fields(stamp).unwrap()[..3], [year, month, day]);
        }
        assert_eq!(
            from_fields([99., 0., 1., 0., 0., 0., 0.], true),
            from_fields([1999., 0., 1., 0., 0., 0., 0.], false)
        );
        assert!(utc_string(0.0).starts_with("Thu, 01 Jan 1970"));
    }
}
