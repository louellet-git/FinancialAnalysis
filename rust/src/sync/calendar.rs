//! The market calendar for the age of data: weekdays and the holidays of the New York Stock Exchange.
//!
//! A data set with a maximum age of 1 day is not old on a weekend or on a market holiday: no new data exists on
//! those days. The age of such a data set is the number of market days after its date, up to today.
//!
//! The holidays follow the rules of the exchange (a holiday on a Saturday is taken on the Friday before; on a Sunday,
//! on the Monday after; New Year's Day on a Saturday has no day off). Good Friday comes from the date of Easter.
//! The special closing days (a day of national mourning) are not known here: for such a day the age is one day too
//! high, and the sync asks the source once more. This is a safe error.

use crate::market::parse_date;
use mapcheck::dates;

/// Day number to (year, month, day).
fn civil(day: i64) -> (i64, i64, i64) {
    let text = dates::format(day);
    (text[..4].parse().unwrap_or(1970), text[5..7].parse().unwrap_or(1), text[8..10].parse().unwrap_or(1))
}

/// 0 = Monday ... 6 = Sunday.
pub fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// The `n`-th (1 to 4) given weekday (0 = Monday) of a month.
fn nth_weekday(year: i64, month: i64, wd: i64, n: i64) -> i64 {
    let first = dates::days_from_civil(year, month, 1);
    first + (wd - weekday(first)).rem_euclid(7) + 7 * (n - 1)
}

/// The last given weekday of a month.
fn last_weekday(year: i64, month: i64, wd: i64) -> i64 {
    let next = if month == 12 { dates::days_from_civil(year + 1, 1, 1) } else { dates::days_from_civil(year, month + 1, 1) };
    let last = next - 1;
    last - (weekday(last) - wd).rem_euclid(7)
}

/// Easter Sunday (the Gregorian rule of Meeus, Jones and Butcher).
pub fn easter(year: i64) -> i64 {
    let a = year % 19;
    let (b, c) = (year / 100, year % 100);
    let (d, e) = (b / 4, b % 4);
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    dates::days_from_civil(year, month, day)
}

/// The day that a fixed holiday is taken: a Saturday on the Friday before, a Sunday on the Monday after.
fn observed(day: i64) -> Option<i64> {
    match weekday(day) {
        5 => Some(day - 1),
        6 => Some(day + 1),
        _ => Some(day),
    }
}

/// The market holidays of one year (some of them can fall in the neighbor year: New Year's Day of the next year on a
/// Saturday is not a holiday at all).
pub fn holidays(year: i64) -> Vec<i64> {
    let mut out = Vec::new();
    // New Year's Day: on a Saturday there is no day off on the Friday.
    let ny = dates::days_from_civil(year, 1, 1);
    if weekday(ny) != 5 {
        out.extend(observed(ny));
    }
    out.push(nth_weekday(year, 1, 0, 3)); // Martin Luther King Jr. Day
    out.push(nth_weekday(year, 2, 0, 3)); // Washington's Birthday
    out.push(easter(year) - 2); // Good Friday
    out.push(last_weekday(year, 5, 0)); // Memorial Day
    if year >= 2022 {
        out.extend(observed(dates::days_from_civil(year, 6, 19))); // Juneteenth
    }
    out.extend(observed(dates::days_from_civil(year, 7, 4))); // Independence Day
    out.push(nth_weekday(year, 9, 0, 1)); // Labor Day
    out.push(nth_weekday(year, 11, 3, 4)); // Thanksgiving Day
    out.extend(observed(dates::days_from_civil(year, 12, 25))); // Christmas Day
    out
}

/// Is the day a day on which the market trades (a weekday that is no holiday)?
pub fn is_market_day(day: i64) -> bool {
    if weekday(day) >= 5 {
        return false;
    }
    let (year, _, _) = civil(day);
    // The holiday of a neighbor year can fall on a day of this year (1 January observed on 31 December is not a rule of
    // the exchange, so only the own year counts).
    !holidays(year).contains(&day)
}

/// The last market day on or before `day`.
pub fn reference_day(day: i64) -> i64 {
    let mut d = day;
    while !is_market_day(d) {
        d -= 1;
    }
    d
}

/// The last market day before `day`.
pub fn previous_market_day(day: i64) -> i64 {
    reference_day(day - 1)
}

/// The market days in `(from, to]`.
pub fn market_days_between(from: i64, to: i64) -> i64 {
    ((from + 1)..=to).filter(|d| is_market_day(*d)).count() as i64
}

/// The age of data that was made on `updated` (text), measured on `today` (a day number), in market days. `None` for
/// a text that is not a date. A date in the future has age 0.
pub fn market_age(updated: &str, today: i64) -> Option<i64> {
    let from = parse_date(updated)?;
    Some(market_days_between(from, today).max(0))
}

/// The age in calendar days. `None` for a text that is not a date.
pub fn calendar_age(updated: &str, today: i64) -> Option<i64> {
    Some((today - parse_date(updated)?).max(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> i64 {
        dates::parse(text).unwrap()
    }

    #[test]
    fn easter_dates() {
        assert_eq!(dates::format(easter(2026)), "2026-04-05");
        assert_eq!(dates::format(easter(2027)), "2027-03-28");
        assert_eq!(dates::format(easter(2024)), "2024-03-31");
        assert_eq!(dates::format(easter(2025)), "2025-04-20");
    }

    #[test]
    fn holidays_of_2026_and_2027() {
        let names = |y: i64| holidays(y).into_iter().map(dates::format).collect::<Vec<_>>();
        assert_eq!(
            names(2026),
            ["2026-01-01", "2026-01-19", "2026-02-16", "2026-04-03", "2026-05-25", "2026-06-19", "2026-07-03", "2026-09-07", "2026-11-26", "2026-12-25"]
        );
        // 2027: 1 January is a Friday; 19 June is a Saturday (taken on Friday 18 June); 4 July is a Sunday (Monday 5 July);
        // 25 December is a Saturday (Friday 24 December).
        assert_eq!(
            names(2027),
            ["2027-01-01", "2027-01-18", "2027-02-15", "2027-03-26", "2027-05-31", "2027-06-18", "2027-07-05", "2027-09-06", "2027-11-25", "2027-12-24"]
        );
        // 1 January 2028 is a Saturday: no day off on 31 December 2027.
        assert!(is_market_day(d("2027-12-31")));
        assert!(!holidays(2028).contains(&d("2027-12-31")));
    }

    #[test]
    fn weekends_and_holidays_are_no_market_days() {
        assert!(is_market_day(d("2026-11-19"))); // a Thursday
        assert!(!is_market_day(d("2026-11-21"))); // Saturday
        assert!(!is_market_day(d("2026-11-22"))); // Sunday
        assert!(!is_market_day(d("2026-11-26"))); // Thanksgiving
        assert!(!is_market_day(d("2026-04-03"))); // Good Friday
        assert_eq!(dates::format(reference_day(d("2026-11-22"))), "2026-11-20");
        assert_eq!(dates::format(reference_day(d("2026-11-27"))), "2026-11-27");
        assert_eq!(dates::format(reference_day(d("2026-11-26"))), "2026-11-25");
        assert_eq!(dates::format(previous_market_day(d("2026-11-23"))), "2026-11-20");
    }

    #[test]
    fn the_age_does_not_grow_on_a_weekend_or_a_holiday() {
        // Data of Friday 2026-11-20 is current on Saturday and Sunday (age 0), one session old on Monday.
        assert_eq!(market_age("2026-11-20", d("2026-11-20")), Some(0));
        assert_eq!(market_age("2026-11-20", d("2026-11-21")), Some(0));
        assert_eq!(market_age("2026-11-20", d("2026-11-22")), Some(0));
        assert_eq!(market_age("2026-11-20", d("2026-11-23")), Some(1));
        // Wednesday data on Thursday 26 (Thanksgiving) and Friday 27.
        assert_eq!(market_age("2026-11-25", d("2026-11-26")), Some(0));
        assert_eq!(market_age("2026-11-25", d("2026-11-27")), Some(1));
        assert_eq!(market_age("2026-11-20", d("2026-11-19")), Some(0), "a date in the future");
        assert_eq!(market_age("not a date", d("2026-11-19")), None);
        assert_eq!(calendar_age("2026-11-12", d("2026-11-19")), Some(7));
    }
}
