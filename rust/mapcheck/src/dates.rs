//! Calendar dates as day numbers (days from 1970-01-01), with no external crate.

/// Days from 1970-01-01 for a proleptic Gregorian date.
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Days in a month of a proleptic Gregorian year (the month is 1 to 12).
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// "YYYY-MM-DD" -> day number. None if the text is not a date.
pub fn parse(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: i64 = text[0..4].parse().ok()?;
    let month: i64 = text[5..7].parse().ok()?;
    let day: i64 = text[8..10].parse().ok()?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

/// Day number -> "YYYY-MM-DD".
pub fn format(days: i64) -> String {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    format!("{year:04}-{month:02}-{day:02}")
}

/// Today (UTC) as a day number.
pub fn today() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs / 86_400
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for text in ["1970-01-01", "2000-02-29", "2024-12-31", "2026-11-19"] {
            assert_eq!(format(parse(text).unwrap()), text);
        }
        assert_eq!(parse("1970-01-01"), Some(0));
        assert_eq!(parse("2024-12-31").unwrap() - parse("2024-01-01").unwrap(), 365);
    }

    #[test]
    fn rejects_text_that_is_not_a_date() {
        assert_eq!(parse("2024-13-01"), None);
        assert_eq!(parse("20241231"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn rejects_a_day_beyond_the_month_length() {
        assert_eq!(parse("2026-02-31"), None);
        assert_eq!(parse("2026-04-31"), None);
        assert_eq!(parse("2026-02-29"), None);
        assert_eq!(parse("1900-02-29"), None);
        assert!(parse("2024-02-29").is_some() && parse("2000-02-29").is_some());
        assert_eq!(parse("2026-02-28").map(|d| d + 1), parse("2026-03-01"));
        assert_eq!(parse("2026-01-00"), None);
    }
}
