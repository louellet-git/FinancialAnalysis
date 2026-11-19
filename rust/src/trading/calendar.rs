//! Calendar helpers. A date is a number of days since 1970-01-01.
//! Business days are Monday to Friday. No holiday calendar exists (same rule as numpy's
//! default `busday` functions, which the Python backtester used).

pub type Date = i32;

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> Date {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i32 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// (year, month, day) for a number of days since 1970-01-01.
pub fn civil_from_days(z: Date) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Monday = 0 ... Sunday = 6. (1970-01-01 was a Thursday.)
pub fn weekday(d: Date) -> u32 {
    (d + 3).rem_euclid(7) as u32
}

pub fn iso(d: Date) -> String {
    let (y, m, day) = civil_from_days(d);
    format!("{y:04}-{m:02}-{day:02}")
}

/// Parse `YYYY-MM-DD`.
pub fn parse_iso(s: &str) -> Option<Date> {
    let b = s.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y: i32 = s.get(0..4)?.parse().ok()?;
    let m: u32 = s.get(5..7)?.parse().ok()?;
    let d: u32 = s.get(8..10)?.parse().ok()?;
    Some(days_from_civil(y, m, d))
}

fn weekdays_before(x: i32) -> i32 {
    let w = x.div_euclid(7);
    let r = x.rem_euclid(7);
    w * 5 + (0..r).filter(|k| (3 + k) % 7 < 5).count() as i32
}

/// Number of business days in `[a, b)`; negative when `a > b` (numpy `busday_count`).
pub fn busday_count(a: Date, b: Date) -> i32 {
    weekdays_before(b) - weekdays_before(a)
}

/// numpy `busday_offset`: roll `d` to a business day (`forward` or backward), then move `n`
/// business days.
pub fn busday_offset(d: Date, n: i32, roll_forward: bool) -> Date {
    let mut d = d;
    while weekday(d) >= 5 {
        d += if roll_forward { 1 } else { -1 };
    }
    d += n.div_euclid(5) * 7;
    for _ in 0..n.rem_euclid(5) {
        d += 1;
        while weekday(d) >= 5 {
            d += 1;
        }
    }
    d
}

/// ISO week number (1 to 53).
pub fn iso_week(d: Date) -> u32 {
    let (y, _, _) = civil_from_days(d);
    let wd = weekday(d) as i32; // Monday = 0
    let thursday = d - wd + 3; // the Thursday of this ISO week decides the ISO year
    let (ty, _, _) = civil_from_days(thursday);
    let jan1 = days_from_civil(ty, 1, 1);
    let _ = y;
    ((thursday - jan1) / 7 + 1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trip_and_weekday() {
        let d = days_from_civil(2026, 11, 19);
        assert_eq!(civil_from_days(d), (2026, 11, 19));
        assert_eq!(weekday(d), 3); // Thursday
        assert_eq!(weekday(0), 3);
        assert_eq!(iso(days_from_civil(2005, 1, 3)), "2005-01-03");
        assert_eq!(parse_iso("2005-01-03"), Some(days_from_civil(2005, 1, 3)));
    }

    #[test]
    fn business_day_rules_match_numpy() {
        let fri = days_from_civil(2026, 11, 20);
        let mon = days_from_civil(2026, 11, 23);
        let sat = days_from_civil(2026, 11, 21);
        assert_eq!(busday_count(fri, mon), 1); // only Friday is in [Fri, Mon)
        assert_eq!(busday_count(mon, fri), -1);
        assert_eq!(busday_count(fri, fri + 1), 1);
        assert_eq!(busday_count(sat, mon), 0);
        assert_eq!(busday_offset(fri, 1, true), mon);
        assert_eq!(busday_offset(sat, 0, true), mon);
        assert_eq!(busday_offset(sat, 0, false), fri);
        assert_eq!(busday_offset(mon, -4, false), days_from_civil(2026, 11, 17));
        assert_eq!(busday_offset(mon, 11, true), days_from_civil(2026, 12, 8));
    }

    #[test]
    fn iso_week_numbers() {
        assert_eq!(iso_week(days_from_civil(2026, 1, 1)), 1);
        assert_eq!(iso_week(days_from_civil(2021, 1, 3)), 53);
        assert_eq!(iso_week(days_from_civil(2024, 12, 30)), 1);
        assert_eq!(iso_week(days_from_civil(2026, 11, 19)), 47);
    }
}
