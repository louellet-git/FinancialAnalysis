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

/// The Sunday on or after `d`.
fn sunday_on_or_after(d: Date) -> Date {
    d + (6 - weekday(d) as i32)
}

/// The last Sunday of a month.
fn last_sunday(y: i32, m: u32) -> Date {
    let next = if m == 12 { days_from_civil(y + 1, 1, 1) } else { days_from_civil(y, m + 1, 1) };
    next - 1 - ((weekday(next - 1) as i32 + 1) % 7)
}

/// First and last day (both Sundays, both inclusive) of daylight saving time in New York for a
/// year, as the time zone database has them (1960 to now). The change is at 02:00 local time, so
/// the whole of both Sundays after 02:00 is in daylight saving time.
fn ny_dst_days(y: i32) -> (Date, Date) {
    match y {
        2007.. => (sunday_on_or_after(days_from_civil(y, 3, 8)), sunday_on_or_after(days_from_civil(y, 11, 1))),
        1987..=2006 => (sunday_on_or_after(days_from_civil(y, 4, 1)), last_sunday(y, 10)),
        1976..=1986 | 1967..=1973 => (last_sunday(y, 4), last_sunday(y, 10)),
        1975 => (days_from_civil(1975, 2, 23), last_sunday(y, 10)),
        1974 => (days_from_civil(1974, 1, 6), last_sunday(y, 10)),
        _ => (last_sunday(y, 4), last_sunday(y, 10)), // before 1967: New York City rule (checked from 1960)
    }
}

/// Seconds east of UTC in New York at the UTC second `utc`: -18000 (EST) or -14400 (EDT).
pub fn ny_offset(utc: i64) -> i64 {
    let day = (utc - 5 * 3600).div_euclid(86_400) as Date;
    let (y, _, _) = civil_from_days(day);
    let (start, end) = ny_dst_days(y);
    let begin = start as i64 * 86_400 + 7 * 3600; // 02:00 EST
    let finish = end as i64 * 86_400 + 6 * 3600; // 02:00 EDT
    if utc >= begin && utc < finish {
        -4 * 3600
    } else {
        -5 * 3600
    }
}

/// `-05:00` or `-04:00` for an offset in seconds.
pub fn offset_text(offset: i64) -> String {
    let a = offset.abs();
    format!("{}{:02}:{:02}", if offset < 0 { '-' } else { '+' }, a / 3600, a % 3600 / 60)
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
    fn new_york_offset_follows_the_us_rules() {
        let at = |y, m, d, h: i64, mi: i64| days_from_civil(y, m, d) as i64 * 86_400 + h * 3600 + mi * 60;
        // 2026: starts Sunday March 8 at 02:00 EST (07:00 UTC), ends Sunday November 1 at 02:00 EDT (06:00 UTC).
        assert_eq!(ny_offset(at(2026, 3, 8, 6, 59)), -18_000);
        assert_eq!(ny_offset(at(2026, 3, 8, 7, 0)), -14_400);
        assert_eq!(ny_offset(at(2026, 11, 1, 5, 59)), -14_400);
        assert_eq!(ny_offset(at(2026, 11, 1, 6, 0)), -18_000);
        assert_eq!(ny_offset(at(2026, 11, 18, 20, 30)), -18_000);
        assert_eq!(ny_offset(at(2026, 8, 26, 13, 30)), -14_400);
        // 1995 (first Sunday of April to last Sunday of October), 1980 (last Sunday of April), 1974 (January 6).
        assert_eq!(ny_offset(at(1995, 4, 2, 7, 0)), -14_400);
        assert_eq!(ny_offset(at(1995, 4, 1, 12, 0)), -18_000);
        assert_eq!(ny_offset(at(1980, 4, 27, 12, 0)), -14_400);
        assert_eq!(ny_offset(at(1980, 4, 20, 12, 0)), -18_000);
        assert_eq!(ny_offset(at(1974, 1, 7, 12, 0)), -14_400);
        assert_eq!(ny_offset(at(1962, 10, 20, 12, 0)), -14_400);
        assert_eq!(offset_text(-18_000), "-05:00");
        assert_eq!(offset_text(-14_400), "-04:00");
    }

    #[test]
    fn iso_week_numbers() {
        assert_eq!(iso_week(days_from_civil(2026, 1, 1)), 1);
        assert_eq!(iso_week(days_from_civil(2021, 1, 3)), 53);
        assert_eq!(iso_week(days_from_civil(2024, 12, 30)), 1);
        assert_eq!(iso_week(days_from_civil(2026, 11, 19)), 47);
    }
}
