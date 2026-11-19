//! Download of the bars into the CSV cache: `fa trade fetch` (and the refresh of `fa trade paper`).
//!
//! Source: the Yahoo chart endpoint (free, with the honest client of `futures`). The cache files
//! have the format and the rules of the earlier Python code (`yfinance` plus pandas):
//!
//! - One file for each symbol and interval: `SPY_1d.csv`, `_VIX_1h.csv`. A `^` in the symbol is `_`.
//! - Header `Date,open,high,low,close,volume` (daily) or `Datetime,...` (intraday). The time is
//!   New York wall time with the offset of that day: `2026-11-18 15:30:00-05:00`. A daily bar is at
//!   00:00.
//! - Daily bars: one row for each New York date (the last row of a date stays). Intraday bars: the
//!   regular session only (09:30 up to, not including, 16:00), and no row with seconds (the live
//!   price row of Yahoo is not a bar).
//! - A bar that has not ended is not written (a bot decides on the close of a bar; the earlier Python
//!   code wrote the partial bar of the running session, and a later refresh replaced it).
//! - A row with no open, high, low or close is dropped.
//! - Merge: the fresh rows and the cached rows go together. At the same time, the fresh row
//!   replaces the cached row. The file never loses a row (the free source has 730 days of hourly
//!   bars and 60 days of 5-minute bars; the cache keeps what it has).

use super::calendar::{iso, ny_offset, offset_text};
use super::data::{file_name, parse_stamp, Interval};
use crate::futures::yahoo::{parse_raw, RawChart};
use std::collections::BTreeMap;
use std::path::Path;

pub const SYMBOLS: [&str; 2] = ["SPY", "QQQ"];
/// The 30-day vol index of each symbol, and the 1-day vol index.
pub const VOL_INDEXES: [&str; 2] = ["^VIX", "^VXN"];
pub const SHORT_VOL_INDEX: &str = "^VIX1D";
pub const RATE_INDEX: &str = "^IRX";
pub const INTERVALS: [Interval; 3] = [Interval::Daily, Interval::Hourly, Interval::FiveMinute];

const BASE: &str = "https://query1.finance.yahoo.com/v8/finance/chart/";
/// 1950-01-01: the start of "all history" (yfinance uses a date this old for `max`).
const FIRST_DAY_SECONDS: i64 = -631_159_200;

/// Every file that `fa trade fetch` keeps: the symbols, the vol indexes (all three intervals) and
/// the short rate (daily).
pub fn all_files() -> Vec<(String, Interval)> {
    let mut v = Vec::new();
    for s in SYMBOLS.iter().chain(VOL_INDEXES.iter()).chain([&SHORT_VOL_INDEX]) {
        for iv in INTERVALS {
            v.push((s.to_string(), iv));
        }
    }
    v.push((RATE_INDEX.to_string(), Interval::Daily));
    v
}

/// The files that `load_market` reads for these symbols and this interval.
pub fn files_needed(symbols: &[&str], interval: Interval) -> Vec<(String, Interval)> {
    let mut v: Vec<(String, Interval)> = Vec::new();
    let mut add = |s: &str, iv: Interval| {
        if !v.iter().any(|(x, i)| x == s && *i == iv) {
            v.push((s.to_string(), iv));
        }
    };
    for s in symbols {
        add(s, interval);
    }
    for s in [SHORT_VOL_INDEX, "^VIX"].into_iter().chain(symbols.iter().map(|s| if *s == "QQQ" { "^VXN" } else { "^VIX" })) {
        add(s, Interval::Daily);
        add(s, interval);
    }
    add(RATE_INDEX, Interval::Daily);
    if interval != Interval::Daily {
        for s in symbols {
            add(s, Interval::Daily);
        }
    }
    v
}

fn encode(symbol: &str) -> String {
    symbol.replace('^', "%5E")
}

pub fn url(symbol: &str, interval: Interval, now_utc: i64) -> String {
    let query = match interval {
        Interval::Daily => format!("period1={FIRST_DAY_SECONDS}&period2={}&interval=1d", now_utc + 2 * 86_400),
        Interval::Hourly => "range=730d&interval=1h".to_string(),
        Interval::FiveMinute => "range=60d&interval=5m".to_string(),
    };
    format!("{BASE}{}?{query}", encode(symbol))
}

/// One cache row: the UTC second (the key) and the text of the CSV line.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub utc: i64,
    pub line: String,
}

/// A price as Python writes a float: `3.0`, `730.2000122070312`.
///
/// Python and Rust both print the shortest text that reads back as the same number. When two texts
/// of 16 or 17 digits are equally short, Python takes the one that is nearest to the exact value, and
/// on a tie the even digit (`136.60000610351562`); Rust can take the other one. So for 16 digits or
/// more the digits come from the correctly rounded exponent form.
fn price(x: f64) -> String {
    let short = format!("{x:?}");
    let a = x.abs();
    if !(1e-4..1e16).contains(&a) {
        return short;
    }
    let digits = short.trim_start_matches('-').chars().filter(|c| c.is_ascii_digit()).collect::<String>();
    let n = digits.trim_start_matches('0').trim_end_matches('0').len();
    if n < 16 {
        return short;
    }
    let sci = format!("{:.*e}", n - 1, x.abs()); // d.ddddde[-]x
    let (mant, exp) = sci.split_once('e').expect("exponent");
    let exp: i32 = exp.parse().expect("exponent number");
    if sci.parse::<f64>().ok() != Some(a) {
        return short;
    }
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let body = if exp >= 0 {
        let int_len = exp as usize + 1;
        if digits.len() <= int_len {
            format!("{digits}{}.0", "0".repeat(int_len - digits.len()))
        } else {
            format!("{}.{}", &digits[..int_len], &digits[int_len..])
        }
    } else {
        format!("0.{}{digits}", "0".repeat((-exp - 1) as usize))
    };
    if x < 0.0 {
        format!("-{body}")
    } else {
        body
    }
}

/// A volume as pandas writes an integer column: `1003200`.
fn volume(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x:?}")
    }
}

/// The cache line of one intraday bar that starts at the UTC second `utc`: the New York time with its offset,
/// then open, high, low, close, volume (the same text as a row of the Yahoo reader). Other sources use it (Alpaca).
pub(crate) fn cache_line(utc: i64, o: f64, h: f64, l: f64, c: f64, v: f64) -> String {
    let off = ny_offset(utc);
    let local = utc + off;
    let (day, secs) = (local.div_euclid(86_400), local.rem_euclid(86_400));
    format!("{} {:02}:{:02}:{:02}{},{},{},{},{},{}", iso(day as i32), secs / 3600, secs % 3600 / 60, secs % 60, offset_text(off), price(o), price(h), price(l), price(c), volume(v))
}

/// The cache rows of a chart reply (same rules as `_normalize` of the Python code).
pub fn rows_from_chart(chart: &RawChart, interval: Interval, now_utc: i64) -> Vec<Row> {
    // Time stamp of each bar in New York time, then the rule "one row for each time, the last one stays".
    let mut by_time: BTreeMap<i64, (usize, i64)> = BTreeMap::new(); // key -> (index in the reply, offset)
    for (i, &utc) in chart.stamps.iter().enumerate() {
        if interval == Interval::Daily {
            let day = (utc + ny_offset(utc)).div_euclid(86_400);
            let off = ny_offset(day * 86_400 + 5 * 3600); // the offset at 00:00 of that date
            by_time.insert(day * 86_400 - off, (i, off));
        } else {
            by_time.insert(utc, (i, ny_offset(utc)));
        }
    }
    let mut rows = Vec::new();
    for (&key, &(i, off)) in &by_time {
        let (Some(o), Some(h), Some(l), Some(c)) = (chart.open[i], chart.high[i], chart.low[i], chart.close[i]) else {
            continue;
        };
        let local = key + off;
        let (day, secs) = (local.div_euclid(86_400), local.rem_euclid(86_400));
        // The regular session of Yahoo (a day with an early close ends at 13:00; Yahoo adds a row with the
        // close time and no volume), and the session 09:30 to 16:00 New York.
        let in_period = chart.regular_periods.is_empty() || chart.regular_periods.iter().any(|&(a, b)| a <= key && key < b);
        if interval != Interval::Daily && (!in_period || !(9 * 3600 + 30 * 60..16 * 3600).contains(&secs) || secs % 60 != 0) {
            // Outside the session, or a row with seconds: during the session Yahoo adds a row with the
            // time of the last trade (for example 09:35:26). It is the live price, not a bar.
            continue;
        }
        // A bar that has not ended is not in the cache: a bot decides on the close of a bar. Daily: the
        // session ends at 16:00. Intraday: the bar ends after its length, but not after 16:00.
        let end_secs = if interval == Interval::Daily { 16 * 3600 } else { (secs + interval.bar_minutes() as i64 * 60).min(16 * 3600) };
        let start_secs = if interval == Interval::Daily { 0 } else { secs };
        if key + end_secs - start_secs > now_utc {
            continue;
        }
        let line = format!(
            "{} {:02}:{:02}:{:02}{},{},{},{},{},{}",
            iso(day as i32),
            secs / 3600,
            secs % 3600 / 60,
            secs % 60,
            offset_text(off),
            price(o),
            price(h),
            price(l),
            price(c),
            volume(chart.volume[i].unwrap_or(0.0))
        );
        rows.push(Row { utc: key, line });
    }
    rows
}

pub fn parse_bars(bytes: &[u8], interval: Interval, now_utc: i64) -> Result<Vec<Row>, String> {
    Ok(rows_from_chart(&parse_raw(bytes)?, interval, now_utc))
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct MergeStats {
    pub before: usize,
    pub after: usize,
    /// Rows that the cache did not have.
    pub added: usize,
    /// Rows that the cache had with other numbers (the fresh row replaced them).
    pub changed: usize,
}

fn header(interval: Interval) -> &'static str {
    if interval == Interval::Daily {
        "Date,open,high,low,close,volume"
    } else {
        "Datetime,open,high,low,close,volume"
    }
}

/// Merge the fresh rows into the text of a cache file. Returns the new text.
pub fn merge(existing: Option<&str>, fresh: &[Row], interval: Interval) -> Result<(String, MergeStats), String> {
    let mut head = header(interval).to_string();
    let mut rows: BTreeMap<i64, String> = BTreeMap::new();
    if let Some(text) = existing {
        let mut lines = text.lines();
        if let Some(h) = lines.next() {
            head = h.to_string();
        }
        for (n, line) in lines.enumerate() {
            if line.is_empty() {
                continue;
            }
            let utc = parse_stamp(line.split(',').next().unwrap_or("")).map_err(|e| format!("cache line {}: {e}", n + 2))?.2;
            rows.insert(utc, line.to_string());
        }
    }
    let mut stats = MergeStats { before: rows.len(), ..Default::default() };
    for r in fresh {
        match rows.insert(r.utc, r.line.clone()) {
            None => stats.added += 1,
            Some(old) if old != r.line => stats.changed += 1,
            Some(_) => {}
        }
    }
    stats.after = rows.len();
    let mut out = head;
    out.push('\n');
    for line in rows.values() {
        out.push_str(line);
        out.push('\n');
    }
    Ok((out, stats))
}

#[derive(Debug)]
pub struct Outcome {
    pub symbol: String,
    pub interval: Interval,
    pub result: Result<MergeStats, String>,
}

fn now_utc() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Download one file and merge it into the cache. The cache file changes only after the reply
/// parses and the merge works; a failure leaves it as it is.
pub fn refresh_file(dir: &Path, symbol: &str, interval: Interval) -> Result<MergeStats, String> {
    let now = now_utc();
    let bytes = crate::futures::download(&url(symbol, interval, now))?;
    let fresh = parse_bars(&bytes, interval, now).map_err(|e| format!("{symbol} {}: {e}", interval.label()))?;
    let path = dir.join(file_name(symbol, interval.label()));
    let old = match std::fs::read_to_string(&path) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    if fresh.is_empty() && old.is_none() {
        return Ok(MergeStats::default()); // nothing to write
    }
    let (text, stats) = merge(old.as_deref(), &fresh, interval)?;
    if old.as_deref() != Some(text.as_str()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(stats)
}

/// Seconds between two downloads of this module. From this cloud container, Yahoo answered HTTP 429 (too
/// many requests) to about half of the 16 files at 0.5 s and 2 s between requests (tested 2026-11-19; the
/// network address is shared, so the limit may be lower here than on a home network).
const PAUSE_SECONDS: f64 = 2.0;

/// After an HTTP 429 the program waits this long and asks one more time. If the answer is 429 again, it
/// stops the downloads of this run (the cache files stay as they are; run the command again later).
const BACKOFF_SECONDS: f64 = 30.0;

fn is_too_many_requests(e: &str) -> bool {
    e.starts_with("HTTP 429 ")
}

/// Refresh the files one after the other, with a pause between two downloads.
pub fn refresh(dir: &Path, files: &[(String, Interval)]) -> Vec<Outcome> {
    let mut out = Vec::new();
    let mut stopped = false;
    for (k, (s, iv)) in files.iter().enumerate() {
        if stopped {
            out.push(Outcome { symbol: s.clone(), interval: *iv, result: Err("not tried: Yahoo answered HTTP 429 twice, so the run stopped".into()) });
            continue;
        }
        if k > 0 {
            std::thread::sleep(std::time::Duration::from_secs_f64(PAUSE_SECONDS));
        }
        let mut result = refresh_file(dir, s, *iv);
        if result.as_ref().is_err_and(|e| is_too_many_requests(e)) {
            eprintln!("fa: Yahoo answered HTTP 429 (too many requests); waiting {BACKOFF_SECONDS:.0} s, then one more try");
            std::thread::sleep(std::time::Duration::from_secs_f64(BACKOFF_SECONDS));
            result = refresh_file(dir, s, *iv);
            stopped = result.as_ref().is_err_and(|e| is_too_many_requests(e));
        }
        out.push(Outcome { symbol: s.clone(), interval: *iv, result });
    }
    out
}

/// `fa trade fetch [--symbol S]... [--interval I]...`
pub fn command(dir: &Path, args: &[String]) -> Result<i32, String> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let (mut symbols, mut intervals) = (Vec::<String>::new(), Vec::<Interval>::new());
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--symbol" => symbols.push(it.next().ok_or("--symbol needs a symbol")?.clone()),
            "--interval" => {
                let v = it.next().ok_or("--interval needs 1d, 1h or 5m")?;
                intervals.push(Interval::parse(v).ok_or_else(|| format!("bad interval '{v}' (use 1d, 1h or 5m)"))?);
            }
            other => return Err(format!("unknown option '{other}'")),
        }
    }
    let files: Vec<(String, Interval)> = all_files()
        .into_iter()
        .filter(|(s, iv)| (symbols.is_empty() || symbols.contains(s)) && (intervals.is_empty() || intervals.contains(iv)))
        .collect();
    if files.is_empty() {
        return Err("no file matches --symbol and --interval".into());
    }
    let t0 = std::time::Instant::now();
    let mut failed = 0;
    for o in refresh(dir, &files) {
        match o.result {
            Ok(s) => println!("{:<7} {}  rows {:>6} -> {:>6}  (new {}, changed {})", o.symbol, o.interval.label(), s.before, s.after, s.added, s.changed),
            Err(e) => {
                failed += 1;
                eprintln!("{:<7} {}  FAILED: {e} (the cache file is not changed)", o.symbol, o.interval.label());
            }
        }
    }
    println!("{} files, {} failed, {:.1}s; cache {}", files.len(), failed, t0.elapsed().as_secs_f64(), dir.display());
    Ok(if failed > 0 { 1 } else { 0 })
}

pub const USAGE: &str = "\
fa trade fetch - download the market data of the backtester into the CSV cache

USAGE:
  fa trade fetch [--symbol S]... [--interval 1d|1h|5m]...

Source: the Yahoo chart endpoint (free; the honest client of `fa futures`, 2 s between requests). After
an HTTP 429 the command waits 30 s and tries once more; then it stops (the cache stays as it is).
Files: SPY and QQQ, ^VIX, ^VXN, ^VIX1D (1d, 1h, 5m each) and ^IRX (1d). Each reply is merged into
the file of the cache: a fresh row replaces the cached row of the same time, and no row is lost.
Yahoo gives 730 days of hourly bars and 60 days of 5-minute bars, so run the command now and then.
The cache directory is $TRADING_DATA_DIR, else data/raw/trading/ of the repository.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::calendar::days_from_civil;

    const FUTURE: i64 = 4_000_000_000;

    /// SPY hourly: the Friday before the end of daylight saving time (2026-10-30, EDT) and the Monday
    /// after (2026-11-02, EST). A bar at 16:00 and a bar at 09:00 are outside the session. One bar has no close.
    fn hourly_fixture() -> String {
        let t = |d: (i32, u32, u32), h: i64, m: i64, off: i64| days_from_civil(d.0, d.1, d.2) as i64 * 86_400 + h * 3600 + m * 60 - off;
        let stamps = [
            t((2026, 10, 30), 9, 0, -14_400),
            t((2026, 10, 30), 9, 30, -14_400),
            t((2026, 10, 30), 15, 30, -14_400),
            t((2026, 10, 30), 16, 0, -14_400),
            t((2026, 11, 2), 9, 30, -18_000),
            t((2026, 11, 2), 10, 30, -18_000),
        ];
        let list = stamps.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
        format!(
            r#"{{"chart":{{"result":[{{"meta":{{"symbol":"SPY","gmtoffset":-18000,"exchangeTimezoneName":"America/New_York","regularMarketTime":{}}},
            "timestamp":[{list}],"indicators":{{"quote":[{{"open":[1,2.5,3,4,5,6],"high":[1,2.75,3,4,5,6],"low":[1,2.25,3,4,5,6],
            "close":[1,2.5,3.125,4,5,null],"volume":[9,100,200,0,300,400]}}]}}}}],"error":null}}}}"#,
            stamps[5]
        )
    }

    #[test]
    fn hourly_rows_follow_the_cache_format() {
        let rows = parse_bars(hourly_fixture().as_bytes(), Interval::Hourly, FUTURE).unwrap();
        let lines: Vec<&str> = rows.iter().map(|r| r.line.as_str()).collect();
        assert_eq!(
            lines,
            [
                "2026-10-30 09:30:00-04:00,2.5,2.75,2.25,2.5,100",
                "2026-10-30 15:30:00-04:00,3.0,3.0,3.0,3.125,200",
                "2026-11-02 09:30:00-05:00,5.0,5.0,5.0,5.0,300",
            ]
        );
        // The key is the UTC second, so the two offsets sort in time order.
        assert!(rows[1].utc < rows[2].utc);
        // The line parses with the reader of the backtester and gives the same instant.
        assert_eq!(parse_stamp(lines[2].split(',').next().unwrap()).unwrap().2, rows[2].utc);
    }

    #[test]
    fn a_bar_that_has_not_ended_is_not_written() {
        // 2026-11-19 10:00 EST (15:00 UTC): the hourly bar of 09:30 is not over (it ends at 10:30), the 5-minute
        // bar of 09:55 is over, and the daily bar of today is not over (the session ends at 16:00).
        let t = days_from_civil(2026, 11, 19) as i64 * 86_400;
        let now = t + 15 * 3600;
        let chart = |stamps: &[i64]| RawChart {
            symbol: "SPY".into(),
            name: String::new(),
            time_zone: "America/New_York".into(),
            offset: -18_000,
            last_trade: None,
            regular_periods: vec![],
            stamps: stamps.to_vec(),
            open: vec![Some(1.0); stamps.len()],
            high: vec![Some(1.0); stamps.len()],
            low: vec![Some(1.0); stamps.len()],
            close: vec![Some(1.0); stamps.len()],
            volume: vec![Some(1.0); stamps.len()],
        };
        let hourly = rows_from_chart(&chart(&[t - 86_400 + 20 * 3600 + 30 * 60, t + 14 * 3600 + 30 * 60]), Interval::Hourly, now);
        assert_eq!(hourly.len(), 1);
        assert!(hourly[0].line.starts_with("2026-11-18 15:30:00-05:00")); // the last hourly bar of a day ends at 16:00
        let five = rows_from_chart(&chart(&[t + 14 * 3600 + 55 * 60, t + 15 * 3600]), Interval::FiveMinute, now);
        assert_eq!(five.len(), 1);
        assert!(five[0].line.starts_with("2026-11-19 09:55:00-05:00"));
        let daily = rows_from_chart(&chart(&[t - 86_400 + 14 * 3600 + 30 * 60, t + 14 * 3600 + 30 * 60]), Interval::Daily, now);
        assert_eq!(daily.len(), 1);
        assert!(daily[0].line.starts_with("2026-11-18 00:00:00-05:00"));
    }

    #[test]
    fn the_close_row_of_an_early_close_day_is_not_a_bar() {
        // 2024-07-03 closed at 13:00 EDT. Yahoo gives the bars 09:30 to 12:30 and a row at 13:00 with no volume.
        // meta.tradingPeriods has the regular period of the day: 09:30 to 13:00.
        let t = days_from_civil(2024, 7, 3) as i64 * 86_400;
        let (start, end) = (t + 13 * 3600 + 30 * 60, t + 17 * 3600);
        let text = format!(
            r#"{{"chart":{{"result":[{{"meta":{{"symbol":"SPY","gmtoffset":-18000,"tradingPeriods":[[{{"start":{start},"end":{end},"timezone":"EDT","gmtoffset":-14400}}]]}},
            "timestamp":[{},{},{end}],"indicators":{{"quote":[{{"open":[1,2,3],"high":[1,2,3],"low":[1,2,3],"close":[1,2,3],"volume":[5,6,0]}}]}}}}],"error":null}}}}"#,
            start,
            end - 1800
        );
        let rows = parse_bars(text.as_bytes(), Interval::Hourly, FUTURE).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows[1].line.starts_with("2024-07-03 12:30:00-04:00"));
        // The same reply as an object with the key `regular` gives the same rows.
        let object = text.replace("\"tradingPeriods\":[[", "\"tradingPeriods\":{\"pre\":[],\"regular\":[[").replace("}]]}", "}]]}}");
        assert_eq!(parse_bars(object.as_bytes(), Interval::Hourly, FUTURE).unwrap().len(), 2);
    }

    #[test]
    fn the_live_price_row_is_not_a_bar() {
        // 2026-11-19 14:35:26 UTC = 09:35:26 EST: the row with the time of the last trade.
        let t = days_from_civil(2026, 11, 19) as i64 * 86_400;
        let text = format!(
            r#"{{"chart":{{"result":[{{"meta":{{"symbol":"SPY","gmtoffset":-18000}},"timestamp":[{},{}],
            "indicators":{{"quote":[{{"open":[1,2],"high":[1,2],"low":[1,2],"close":[1,2],"volume":[5,0]}}]}}}}],"error":null}}}}"#,
            t + 14 * 3600 + 30 * 60,
            t + 14 * 3600 + 35 * 60 + 26
        );
        let rows = parse_bars(text.as_bytes(), Interval::FiveMinute, FUTURE).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].line.starts_with("2026-11-19 09:30:00-05:00,"));
    }

    #[test]
    fn daily_rows_are_midnight_of_the_new_york_date() {
        // Yahoo stamps a daily bar at 09:30 New York (14:30 UTC in winter, 13:30 UTC in summer), and a running
        // session again with the time of the last trade. One row for each date: the last one stays.
        let w = days_from_civil(2026, 11, 18) as i64 * 86_400 + 14 * 3600 + 30 * 60;
        let s = days_from_civil(2026, 7, 1) as i64 * 86_400 + 13 * 3600 + 30 * 60;
        let late = days_from_civil(2026, 11, 18) as i64 * 86_400 + 21 * 3600;
        let text = format!(
            r#"{{"chart":{{"result":[{{"meta":{{"symbol":"^VIX","gmtoffset":-21600,"exchangeTimezoneName":"America/Chicago"}},
            "timestamp":[{s},{w},{late}],"indicators":{{"quote":[{{"open":[10,20,21],"high":[11,22,23],"low":[9,19,20],
            "close":[10.5,21.5,22.5],"volume":[0,0,0]}}]}}}}],"error":null}}}}"#
        );
        let rows = parse_bars(text.as_bytes(), Interval::Daily, FUTURE).unwrap();
        let lines: Vec<&str> = rows.iter().map(|r| r.line.as_str()).collect();
        assert_eq!(lines, ["2026-07-01 00:00:00-04:00,10.0,11.0,9.0,10.5,0", "2026-11-18 00:00:00-05:00,21.0,23.0,20.0,22.5,0"]);
        assert_eq!(rows[0].utc % 86_400, 4 * 3600);
        assert_eq!(rows[1].utc % 86_400, 5 * 3600);
    }

    #[test]
    fn the_last_row_of_a_time_stays_even_if_it_has_no_close() {
        // pandas: the duplicates go first (keep last), then the rows with no price go. So a null row
        // that comes later removes the date.
        let w = days_from_civil(2026, 11, 18) as i64 * 86_400 + 14 * 3600 + 30 * 60;
        let text = format!(
            r#"{{"chart":{{"result":[{{"meta":{{"symbol":"SPY","gmtoffset":-18000}},"timestamp":[{w},{}],
            "indicators":{{"quote":[{{"open":[1,null],"high":[1,null],"low":[1,null],"close":[1,null],"volume":[1,null]}}]}}}}],"error":null}}}}"#,
            w + 6 * 3600
        );
        assert!(parse_bars(text.as_bytes(), Interval::Daily, FUTURE).unwrap().is_empty());
    }

    #[test]
    fn merge_replaces_the_same_time_and_never_loses_a_row() {
        let old = "Datetime,open,high,low,close,volume\n\
                   2026-11-16 09:30:00-05:00,1.0,1.0,1.0,1.0,1\n\
                   2026-11-17 09:30:00-05:00,2.0,2.0,2.0,2.0,2\n";
        let mk = |t: &str, x: &str| {
            let line = format!("{t},{x},{x},{x},{x},5");
            Row { utc: parse_stamp(t).unwrap().2, line }
        };
        let fresh = [mk("2026-11-17 09:30:00-05:00", "2.5"), mk("2026-11-18 09:30:00-05:00", "3.0"), mk("2026-11-16 09:30:00-05:00", "1.0")];
        let (text, st) = merge(Some(old), &fresh, Interval::Hourly).unwrap();
        // One row is new, one changed. The third has other volume text, so it counts as changed too.
        assert_eq!((st.before, st.after, st.added, st.changed), (2, 3, 1, 2));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "Datetime,open,high,low,close,volume");
        assert_eq!(lines[2], "2026-11-17 09:30:00-05:00,2.5,2.5,2.5,2.5,5");
        assert_eq!(lines[3], "2026-11-18 09:30:00-05:00,3.0,3.0,3.0,3.0,5");
        // Fresh rows alone (no file yet): header by interval. An empty reply keeps the old rows.
        let (t2, _) = merge(None, &fresh, Interval::Daily).unwrap();
        assert!(t2.starts_with("Date,open"));
        let (t3, s3) = merge(Some(old), &[], Interval::Hourly).unwrap();
        assert_eq!((t3.as_str(), s3.after), (old, 2));
        // The rows in the order of time, also across the change of the offset.
        let w = "Date,open,high,low,close,volume\n2026-10-30 00:00:00-04:00,1.0,1.0,1.0,1.0,1\n";
        let f = Row { utc: parse_stamp("2026-11-02 00:00:00-05:00").unwrap().2, line: "2026-11-02 00:00:00-05:00,2.0,2.0,2.0,2.0,2".into() };
        let (t4, _) = merge(Some(w), &[f], Interval::Daily).unwrap();
        assert_eq!(t4.lines().count(), 3);
        assert!(t4.lines().nth(2).unwrap().starts_with("2026-11-02"));
    }

    #[test]
    fn prices_are_written_like_python_floats() {
        // Both texts read back as the same number; Python writes the first one (the even digit on a tie).
        for (x, want) in [
            (136.60000610351563, "136.60000610351562"),
            (730.2000122070313, "730.2000122070312"),
            (135.22000122070313, "135.22000122070312"),
            (4.138000011444092, "4.138000011444092"),
            (43.96875, "43.96875"),
            (3.0, "3.0"),
            (0.0, "0.0"),
            (0.05, "0.05"),
            (-1.5, "-1.5"),
            (471.3299865722656, "471.3299865722656"),
        ] {
            assert_eq!(price(x), want);
            assert_eq!(price(x).parse::<f64>().unwrap(), x);
        }
    }

    #[test]
    fn a_429_is_recognized() {
        assert!(is_too_many_requests("HTTP 429 for https://query1.finance.yahoo.com/v8/finance/chart/SPY"));
        assert!(!is_too_many_requests("HTTP 404 for https://x"));
        assert!(!is_too_many_requests("SPY 1d: Yahoo response is not JSON"));
    }

    #[test]
    fn the_files_and_the_urls() {
        assert_eq!(all_files().len(), 5 * 3 + 1);
        let need = files_needed(&["SPY"], Interval::FiveMinute);
        let names: Vec<String> = need.iter().map(|(s, iv)| file_name(s, iv.label())).collect();
        for want in ["SPY_5m.csv", "_VIX1D_1d.csv", "_VIX1D_5m.csv", "_VIX_1d.csv", "_VIX_5m.csv", "_IRX_1d.csv", "SPY_1d.csv"] {
            assert!(names.iter().any(|n| n == want), "{want} in {names:?}");
        }
        assert!(!names.iter().any(|n| n.contains("QQQ") || n.contains("VXN")));
        let daily = files_needed(&["SPY", "QQQ"], Interval::Daily);
        assert_eq!(daily.len(), 2 + 3 + 1); // SPY, QQQ, VIX1D, VIX, VXN, IRX
        assert_eq!(url("^VIX", Interval::Hourly, 0), "https://query1.finance.yahoo.com/v8/finance/chart/%5EVIX?range=730d&interval=1h");
        assert!(url("SPY", Interval::Daily, 1_000_000).contains("period1=-631159200&period2=1172800&interval=1d"));
    }
}
