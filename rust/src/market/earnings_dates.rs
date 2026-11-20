//! Earnings dates: the past dates, the next date, the calendar cache and the check against the SEC filing list.
//!
//! Files (all under `data/raw`, not committed):
//!
//! | File | Content |
//! |---|---|
//! | `<T>/earnings_dates.json` | `past`: the report dates of the Nasdaq surprise table (the dates add up over the runs; the source shows the last 4 quarters). `next`: the next date from the calendar scan, with time of day, EPS forecast and number of estimates. |
//! | `<T>/sec_8k_earnings.json` | the forms 8-K with item 2.02 of the SEC filing list (made by `mapcheck fetch --submissions`) |
//! | `_calendar/earnings-<date>.json` | the Nasdaq earnings calendar of one date, all companies |
//!
//! Calendar cache rule: the file of a date is final when it was fetched after the date (the list of a past date does
//! not change). Else it is current when it was fetched today; a future date is asked again on a later day.
//! The calendar answers the date of a company only a few weeks ahead, so the scan covers a window of days.

use super::nasdaq_est::{self, calendar_url, CalendarRow, Client, SourceError};
use super::{parse_date, ticker_dir, write_atomic};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PastDate {
    pub date: String,
    pub fiscal_qtr_end: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NextDate {
    pub date: String,
    pub time: String,
    pub fiscal_quarter: String,
    pub eps_forecast: Option<f64>,
    pub estimates: Option<f64>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DatesFile {
    pub ticker: String,
    pub updated: String,
    #[serde(default)]
    pub past: Vec<PastDate>,
    #[serde(default)]
    pub next: Option<NextDate>,
    /// The last calendar date that the scan covered when it set `next` (`None`: no scan yet).
    #[serde(default)]
    pub scanned_through: Option<String>,
}

pub fn dates_path(cache: &Path, ticker: &str) -> PathBuf {
    ticker_dir(cache, ticker).join("earnings_dates.json")
}

pub fn load_dates(cache: &Path, ticker: &str) -> Option<DatesFile> {
    let text = std::fs::read_to_string(dates_path(cache, ticker)).ok()?;
    serde_json::from_str(&text).ok()
}

fn save_dates(cache: &Path, f: &DatesFile) -> Result<(), String> {
    write_atomic(&dates_path(cache, &f.ticker), &serde_json::to_string_pretty(f).map_err(|e| e.to_string())?)
}

/// Add past report dates (a date that exists is replaced by the new row). Newest first.
pub fn add_past(cache: &Path, ticker: &str, today: &str, rows: &[PastDate]) -> Result<(), String> {
    let mut f = load_dates(cache, ticker).unwrap_or_else(|| DatesFile { ticker: ticker.to_uppercase(), ..Default::default() });
    for r in rows {
        f.past.retain(|p| p.date != r.date);
        f.past.push(r.clone());
    }
    f.past.sort_by(|a, b| b.date.cmp(&a.date));
    f.updated = today.to_string();
    save_dates(cache, &f)
}

/// Set the next date (or `None`: the scan found none) of a ticker.
pub fn set_next(cache: &Path, ticker: &str, today: &str, next: Option<NextDate>, scanned_through: &str) -> Result<(), String> {
    let mut f = load_dates(cache, ticker).unwrap_or_else(|| DatesFile { ticker: ticker.to_uppercase(), ..Default::default() });
    f.next = next;
    f.scanned_through = Some(scanned_through.to_string());
    f.updated = today.to_string();
    save_dates(cache, &f)
}

// ---------------------------------------------------------------- calendar cache

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarDay {
    pub date: String,
    pub fetched: String,
    pub source: String,
    pub rows: Vec<CalendarRow>,
}

pub fn calendar_path(cache: &Path, date: &str) -> PathBuf {
    cache.join("_calendar").join(format!("earnings-{date}.json"))
}

pub fn load_calendar_day(cache: &Path, date: &str) -> Option<CalendarDay> {
    let text = std::fs::read_to_string(calendar_path(cache, date)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Do we need no new request for this file? (ISO dates compare as text.)
pub fn calendar_is_current(date: &str, fetched: &str, today: &str) -> bool {
    fetched > date || fetched == today
}

/// A Saturday or a Sunday (the calendar has no rows on these days; the scan makes no request for them).
pub fn is_weekend(date: &str) -> bool {
    parse_date(date).is_none_or(|d| (d + 3).rem_euclid(7) >= 5)
}

fn date_plus(date: &str, n: i64) -> Option<String> {
    Some(mapcheck::dates::format(parse_date(date)? + n))
}

/// The dates of the scan: `days` calendar days from `from`, with no Saturday and no Sunday.
pub fn scan_dates(from: &str, days: i64) -> Vec<String> {
    (0..days).filter_map(|i| date_plus(from, i)).filter(|d| !is_weekend(d)).collect()
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ScanStats {
    pub days: usize,
    pub requests: usize,
    pub cached: usize,
    pub failed: Vec<(String, String)>,
    /// The date at which the scan stopped on a block (the dates after it are not scanned).
    pub stopped_at: Option<String>,
    pub rows: usize,
}

/// Get the calendar of each date that the cache does not have (or that it has no more current), and cache it.
/// A block stops the scan; the days before stay in the cache.
pub fn scan(cache: &Path, dates: &[String], today: &str, refresh: bool, client: &mut Client, progress: &dyn Fn(usize, &str, &str)) -> ScanStats {
    let mut st = ScanStats { days: dates.len(), ..Default::default() };
    for (i, date) in dates.iter().enumerate() {
        if let Some(day) = load_calendar_day(cache, date) {
            if !refresh && calendar_is_current(date, &day.fetched, today) {
                st.cached += 1;
                st.rows += day.rows.len();
                progress(i, date, "cached");
                continue;
            }
        }
        match nasdaq_est::fetch(client, &calendar_url(date), nasdaq_est::parse_calendar) {
            Ok(rows) => {
                st.rows += rows.len();
                progress(i, date, &format!("{} companies", rows.len()));
                let day = CalendarDay { date: date.clone(), fetched: today.to_string(), source: "nasdaq calendar/earnings".into(), rows };
                match serde_json::to_string_pretty(&day).map_err(|e| e.to_string()).and_then(|t| write_atomic(&calendar_path(cache, date), &t)) {
                    Ok(()) => {}
                    Err(e) => st.failed.push((date.clone(), e)),
                }
            }
            Err(SourceError::Blocked(why)) => {
                progress(i, date, &format!("BLOCKED {why}"));
                st.stopped_at = Some(date.clone());
                break;
            }
            Err(SourceError::Failed(why)) => {
                progress(i, date, &format!("FAILED {why}"));
                st.failed.push((date.clone(), why));
            }
        }
    }
    st.requests = client.requests;
    st
}

/// The next report of each symbol from the cached calendar days of `dates` (the first date of a symbol wins).
/// The key is the symbol of the site (`BRK.B`).
pub fn next_from_cache(cache: &Path, dates: &[String]) -> std::collections::HashMap<String, (String, CalendarRow)> {
    let mut out = std::collections::HashMap::new();
    for date in dates {
        let Some(day) = load_calendar_day(cache, date) else { continue };
        for r in day.rows {
            out.entry(nasdaq_est::symbol(&r.symbol)).or_insert_with(|| (date.clone(), r));
        }
    }
    out
}

/// The next report date of one ticker after a scan.
#[derive(Debug, Clone, PartialEq)]
pub struct TickerNext {
    pub ticker: String,
    /// The calendar day and row, when the calendar has the ticker.
    pub calendar: Option<(String, CalendarRow)>,
    /// The date that was written: the calendar date, else the estimate from the past dates, else `None`.
    pub value: Option<NextDate>,
    /// `value` is an estimate (the calendar has no date).
    pub estimated: bool,
}

/// After a scan: find the next date of each ticker in the cached calendar days of `covered`, else estimate it from the
/// past dates, and write it into the file of the ticker (`earnings_dates.json`) when the scan covered at least one day.
/// `fa earnings calendar` and `fa sync` both call this function.
pub fn apply_calendar(cache: &Path, tickers: &[String], covered: &[String], from: &str, today: &str) -> Vec<TickerNext> {
    let next = next_from_cache(cache, covered);
    let through = covered.last().cloned().unwrap_or_else(|| from.to_string());
    let mut out = Vec::new();
    for t in tickers {
        let hit = next.get(&nasdaq_est::symbol(t)).cloned();
        let (value, estimated) = match &hit {
            Some((d, r)) => (Some(next_date_of(d, r)), false),
            None => {
                let e = estimate_next(&known_past_dates(cache, t), today);
                let estimated = e.is_some();
                (e, estimated)
            }
        };
        if !covered.is_empty() {
            let _ = set_next(cache, t, today, value.clone(), &through);
        }
        out.push(TickerNext { ticker: t.clone(), calendar: hit, value, estimated });
    }
    out
}

pub fn next_date_of(date: &str, r: &CalendarRow) -> NextDate {
    NextDate { date: date.to_string(), time: r.time.clone(), fiscal_quarter: r.fiscal_quarter.clone(), eps_forecast: r.eps_forecast, estimates: r.estimates, source: "nasdaq calendar".into() }
}

/// The calendar of the vendor reaches about 8 weeks ahead, so most companies have no date there yet. The estimate of
/// the next report date: the report date of each past quarter plus 364 days (the same weekday one year later); the
/// first one that is not before `today`. It is `None` when that date is more than 100 days away (the past dates then
/// miss the quarter). The estimate is a guess: its `source` says so.
pub fn estimate_next(past: &[String], today: &str) -> Option<NextDate> {
    let now = parse_date(today)?;
    let best = past.iter().filter_map(|d| parse_date(d)).map(|d| d + 364).filter(|d| *d >= now && *d - now <= 100).min()?;
    Some(NextDate { date: mapcheck::dates::format(best), time: "-".into(), fiscal_quarter: String::new(), eps_forecast: None, estimates: None, source: ESTIMATE_SOURCE.into() })
}

pub const ESTIMATE_SOURCE: &str = "estimate: report date of a year ago + 364 days";

/// The past report dates that the cache knows: the Nasdaq surprise dates and the SEC 8-K filing dates.
pub fn known_past_dates(cache: &Path, ticker: &str) -> Vec<String> {
    let mut out: Vec<String> = load_dates(cache, ticker).map(|f| f.past.into_iter().map(|p| p.date).collect()).unwrap_or_default();
    out.extend(load_sec(cache, ticker).unwrap_or_default().into_iter().map(|f| f.filed));
    out.sort();
    out.dedup();
    out
}

/// The next report date of a ticker: the file `earnings_dates.json` (a date of the calendar), else the cached
/// calendar days from `today` for 120 days, else the estimate of `estimate_next`.
pub fn resolve_next(cache: &Path, ticker: &str, today: &str) -> Option<NextDate> {
    let from_file = load_dates(cache, ticker).and_then(|d| d.next).filter(|n| n.date.as_str() >= today && n.source != ESTIMATE_SOURCE);
    from_file
        .or_else(|| next_from_cache(cache, &scan_dates(today, 120)).remove(&nasdaq_est::symbol(ticker)).map(|(d, r)| next_date_of(&d, &r)))
        .or_else(|| estimate_next(&known_past_dates(cache, ticker), today))
}

// ---------------------------------------------------------------- SEC check

#[derive(Debug, Clone, PartialEq)]
pub struct SecFiling {
    pub filed: String,
    pub report_date: String,
}

/// `<T>/sec_8k_earnings.json` (made by `mapcheck fetch --submissions T`), newest first. `None` when there is no file.
pub fn load_sec(cache: &Path, ticker: &str) -> Option<Vec<SecFiling>> {
    let text = std::fs::read_to_string(ticker_dir(cache, ticker).join("sec_8k_earnings.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let s = |r: &serde_json::Value, k: &str| r[k].as_str().unwrap_or("").to_string();
    let mut out: Vec<SecFiling> = v["filings"].as_array()?.iter().map(|r| SecFiling { filed: s(r, "filed"), report_date: s(r, "report_date") }).filter(|f| parse_date(&f.filed).is_some()).collect();
    out.sort_by(|a, b| b.filed.cmp(&a.filed));
    Some(out)
}

/// A row of the comparison of the two sources of past dates.
#[derive(Debug, Clone, PartialEq)]
pub struct DateRow {
    pub nasdaq: Option<String>,
    pub sec: Option<String>,
    /// Days between the two dates (the SEC date that is nearest to the Nasdaq date).
    pub days: Option<i64>,
    pub flag: Flag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Same,
    /// The two dates are more than the tolerance apart.
    Differs,
    /// Nasdaq has the date and the SEC list has no 8-K with item 2.02 near it.
    NasdaqOnly,
    /// The SEC list has a filing in the period of the Nasdaq dates and Nasdaq has no date near it.
    SecOnly,
}

/// Two dates that are this many days apart or less are the same event with two dates (a difference to report).
/// Dates that are farther apart are two different events.
pub const SAME_EVENT_DAYS: i64 = 10;

/// Compare the Nasdaq report dates with the SEC filings. For each Nasdaq date the nearest SEC date is the nearest of
/// `filed` and `reportDate` of any filing. A SEC filing from the first Nasdaq date on that no Nasdaq date is near
/// (within `SAME_EVENT_DAYS`) is `SecOnly` (an 8-K with item 2.02 can also be a pre-announcement).
/// `tolerance` is in days (1: a difference of 2 days or more is a difference).
pub fn compare_dates(nasdaq: &[String], sec: &[SecFiling], tolerance: i64) -> Vec<DateRow> {
    let day = |d: &str| parse_date(d);
    let sec_days: Vec<(i64, &SecFiling)> = sec.iter().filter_map(|f| day(&f.filed).map(|d| (d, f))).collect();
    let mut rows = Vec::new();
    let mut matched = vec![false; sec_days.len()];
    let mut nasdaq: Vec<&String> = nasdaq.iter().collect();
    nasdaq.sort();
    nasdaq.reverse();
    for n in &nasdaq {
        let Some(nd) = day(n) else { continue };
        let mut best: Option<(i64, usize, String)> = None;
        for (i, (fd, f)) in sec_days.iter().enumerate() {
            for (cand_day, cand_text) in [(Some(*fd), f.filed.clone()), (day(&f.report_date), f.report_date.clone())] {
                let Some(cd) = cand_day else { continue };
                let diff = (cd - nd).abs();
                if best.as_ref().is_none_or(|b| diff < b.0) {
                    best = Some((diff, i, cand_text));
                }
            }
        }
        match best {
            Some((diff, i, text)) if diff <= SAME_EVENT_DAYS => {
                matched[i] = true;
                rows.push(DateRow { nasdaq: Some((*n).clone()), sec: Some(text), days: Some(diff), flag: if diff <= tolerance { Flag::Same } else { Flag::Differs } });
            }
            _ => rows.push(DateRow { nasdaq: Some((*n).clone()), sec: None, days: None, flag: Flag::NasdaqOnly }),
        }
    }
    if let Some(first) = nasdaq.iter().filter_map(|n| day(n)).min() {
        for (i, (fd, f)) in sec_days.iter().enumerate() {
            if !matched[i] && *fd >= first - SAME_EVENT_DAYS {
                rows.push(DateRow { nasdaq: None, sec: Some(f.filed.clone()), days: None, flag: Flag::SecOnly });
            }
        }
    }
    rows.sort_by(|a, b| b.nasdaq.as_ref().or(b.sec.as_ref()).cmp(&a.nasdaq.as_ref().or(a.sec.as_ref())));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fa-dates-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn calendar_cache_rule() {
        // A past date fetched after the date is final.
        assert!(calendar_is_current("2026-11-10", "2026-11-11", "2026-11-19"));
        // A past date fetched on the same day was not complete: ask again.
        assert!(!calendar_is_current("2026-11-10", "2026-11-10", "2026-11-19"));
        // A future date: current only on the day of the fetch.
        assert!(calendar_is_current("2026-12-01", "2026-11-19", "2026-11-19"));
        assert!(!calendar_is_current("2026-12-01", "2026-11-18", "2026-11-19"));
        // Today, fetched today: current.
        assert!(calendar_is_current("2026-11-19", "2026-11-19", "2026-11-19"));
    }

    #[test]
    fn scan_has_no_weekend() {
        let d = scan_dates("2026-11-19", 7);
        assert_eq!(d, ["2026-11-19", "2026-11-20", "2026-11-23", "2026-11-24", "2026-11-25"]);
        assert!(is_weekend("2026-11-21") && is_weekend("2026-11-22") && !is_weekend("2026-11-20"));
    }

    const DAY: &str = include_str!("../../tests/fixtures/market/nasdaq/calendar_2026-11-20.json");

    #[test]
    fn scan_asks_only_for_the_days_that_the_cache_does_not_have() {
        use std::cell::RefCell;
        let cache = dir("scan");
        let urls = RefCell::new(Vec::<String>::new());
        let get = |u: &str| -> Result<Vec<u8>, String> {
            urls.borrow_mut().push(u.to_string());
            Ok(DAY.as_bytes().to_vec())
        };
        let dates = scan_dates("2026-11-19", 3);
        let mut client = Client::new(&get, Duration::ZERO);
        let st = scan(&cache, &dates, "2026-11-19", false, &mut client, &|_, _, _| {});
        assert_eq!((st.days, st.requests, st.cached, st.rows), (2, 2, 0, 8));
        // The second run the same day: no request.
        let mut client = Client::new(&get, Duration::ZERO);
        let st = scan(&cache, &dates, "2026-11-19", false, &mut client, &|_, _, _| {});
        assert_eq!((st.requests, st.cached), (0, 2));
        // A day later the dates in the future are asked again; today's date has become the past (not final yet).
        let mut client = Client::new(&get, Duration::ZERO);
        let st = scan(&cache, &dates, "2026-11-20", false, &mut client, &|_, _, _| {});
        assert_eq!(st.requests, 2);
        // A day after the date, the file is final.
        let mut client = Client::new(&get, Duration::ZERO);
        let st = scan(&cache, &dates, "2026-11-21", false, &mut client, &|_, _, _| {});
        assert_eq!(st.requests, 1, "only 2026-11-20 was fetched on that date; the file of 2026-11-19 is final");
        let next = next_from_cache(&cache, &dates);
        // The first date of a symbol wins; `MOG.B` keeps its dot.
        assert_eq!(next["BJ"].0, "2026-11-19");
        assert_eq!(next["MOG.B"].1.estimates, Some(1.0));
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn a_block_stops_the_scan_and_keeps_the_cache() {
        let cache = dir("block");
        let calls = std::cell::Cell::new(0);
        let get = |_: &str| -> Result<Vec<u8>, String> {
            calls.set(calls.get() + 1);
            if calls.get() == 1 { Ok(DAY.as_bytes().to_vec()) } else { Err("HTTP 429 for x".into()) }
        };
        let dates = scan_dates("2026-11-19", 5);
        let mut client = Client::new(&get, Duration::ZERO);
        let st = scan(&cache, &dates, "2026-11-19", false, &mut client, &|_, _, _| {});
        assert_eq!(st.stopped_at.as_deref(), Some("2026-11-20"));
        assert!(load_calendar_day(&cache, "2026-11-19").is_some(), "the day before the block stays");
        assert!(load_calendar_day(&cache, "2026-11-20").is_none());
        let _ = std::fs::remove_dir_all(&cache);
    }

    fn f(filed: &str, report: &str) -> SecFiling {
        SecFiling { filed: filed.into(), report_date: report.into() }
    }

    #[test]
    fn nasdaq_and_sec_dates_are_compared_with_a_tolerance_of_one_day() {
        let nasdaq: Vec<String> = ["2026-10-28", "2026-07-29", "2026-04-29", "2026-01-28"].iter().map(|s| s.to_string()).collect();
        let sec = vec![f("2026-10-28", "2026-10-28"), f("2026-07-30", "2026-07-29"), f("2026-04-27", "2026-04-27"), f("2026-03-02", "2026-03-02")];
        let rows = compare_dates(&nasdaq, &sec, 1);
        let by = |d: &str| rows.iter().find(|r| r.nasdaq.as_deref() == Some(d)).unwrap();
        assert_eq!(by("2026-10-28").flag, Flag::Same);
        // The filing date is one day later, the report date is the same day: same.
        assert_eq!((by("2026-07-29").flag, by("2026-07-29").days), (Flag::Same, Some(0)));
        // Two days apart: a difference.
        assert_eq!((by("2026-04-29").flag, by("2026-04-29").days), (Flag::Differs, Some(2)));
        // The SEC filing of 2026-03-02 is 33 days from 2026-01-28: not the same event. Nasdaq has a date that the SEC list has not.
        assert_eq!(by("2026-01-28").flag, Flag::NasdaqOnly);
        // A SEC filing in the period with no Nasdaq date near it is listed.
        assert!(rows.iter().any(|r| r.flag == Flag::SecOnly && r.sec.as_deref() == Some("2026-03-02")));
        // No SEC data at all: Nasdaq only.
        assert!(compare_dates(&nasdaq, &[], 1).iter().all(|r| r.flag == Flag::NasdaqOnly));
        // Newest first.
        assert_eq!(rows[0].nasdaq.as_deref(), Some("2026-10-28"));
    }

    #[test]
    fn next_date_is_a_calendar_date_else_an_estimate_from_last_year() {
        let past: Vec<String> = ["2026-10-28", "2026-07-29", "2026-04-29", "2026-01-28"].iter().map(|s| s.to_string()).collect();
        // 2026-01-28 + 364 days is 2027-01-27 (a Wednesday, as the date a year before).
        let e = estimate_next(&past, "2026-11-19").unwrap();
        assert_eq!(e.date, "2027-01-27");
        assert!(e.source.starts_with("estimate"));
        // A date that is more than 100 days away is not given (the past misses the quarter), nor is a date in the past.
        assert!(estimate_next(&past[..1], "2026-11-19").is_none());
        assert!(estimate_next(&past, "2027-02-01").is_some_and(|n| n.date == "2027-04-28"));
        assert!(estimate_next(&[], "2026-11-19").is_none());
        // resolve_next: a calendar date wins over the estimate.
        let cache = dir("resolve");
        add_past(&cache, "MSFT", "2026-11-19", &past.iter().map(|d| PastDate { date: d.clone(), fiscal_qtr_end: String::new(), source: "nasdaq".into() }).collect::<Vec<_>>()).unwrap();
        assert_eq!(resolve_next(&cache, "MSFT", "2026-11-19").unwrap().date, "2027-01-27");
        let day = CalendarDay { date: "2026-12-02".into(), fetched: "2026-11-19".into(), source: "x".into(), rows: nasdaq_est::parse_calendar(DAY.as_bytes()).unwrap().into_iter().take(1).map(|mut r| { r.symbol = "MSFT".into(); r }).collect() };
        write_atomic(&calendar_path(&cache, "2026-12-02"), &serde_json::to_string(&day).unwrap()).unwrap();
        let n = resolve_next(&cache, "MSFT", "2026-11-19").unwrap();
        assert_eq!((n.date.as_str(), n.source.as_str()), ("2026-12-02", "nasdaq calendar"));
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn dates_file_adds_up_the_past_and_keeps_the_next() {
        let cache = dir("file");
        let row = |d: &str| PastDate { date: d.into(), fiscal_qtr_end: "2026-09".into(), source: "nasdaq".into() };
        add_past(&cache, "msft", "2026-11-19", &[row("2026-10-28"), row("2026-07-29")]).unwrap();
        let next = NextDate { date: "2027-01-27".into(), time: "after-hours".into(), fiscal_quarter: "2026-12".into(), eps_forecast: Some(4.86), estimates: Some(14.0), source: "nasdaq calendar".into() };
        set_next(&cache, "MSFT", "2026-11-19", Some(next.clone()), "2027-01-31").unwrap();
        // A later run adds a date and does not delete the old ones or the next date.
        add_past(&cache, "MSFT", "2027-02-01", &[row("2027-01-27"), row("2026-10-28")]).unwrap();
        let f = load_dates(&cache, "MSFT").unwrap();
        assert_eq!(f.past.iter().map(|p| p.date.as_str()).collect::<Vec<_>>(), ["2027-01-27", "2026-10-28", "2026-07-29"]);
        assert_eq!(f.next, Some(next));
        assert_eq!(f.scanned_through.as_deref(), Some("2027-01-31"));
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn sec_file_of_the_bridge_is_read() {
        let cache = dir("sec");
        std::fs::create_dir_all(cache.join("IBM")).unwrap();
        std::fs::write(cache.join("IBM/sec_8k_earnings.json"), r#"{"ticker":"IBM","filings":[{"filed":"2026-07-22","report_date":"2026-07-22"},{"filed":"2026-10-21","report_date":""},{"filed":"bad","report_date":""}]}"#).unwrap();
        let s = load_sec(&cache, "IBM").unwrap();
        assert_eq!(s.iter().map(|x| x.filed.as_str()).collect::<Vec<_>>(), ["2026-10-21", "2026-07-22"]);
        assert!(load_sec(&cache, "NONE").is_none());
        let _ = std::fs::remove_dir_all(&cache);
    }
}
