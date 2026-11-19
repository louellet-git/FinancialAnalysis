//! `fa prices intraday`: 1-minute and 5-minute bars with years of history, from Alpaca, into the trading bars
//! cache (`data/raw/trading/<SYMBOL>_5m.csv`, `<SYMBOL>_1m.csv`).
//!
//! The file form and the merge rule are those of `trading::fetch` (a row with the New York time and its offset; a fresh row
//! replaces the cached row of the same time; the file never loses a row; the regular session only; no bar that has not
//! ended). The bot reader (`trading::data`) reads `_1d`, `_1h` and `_5m` files; the `_1m` file is for the work on realized
//! variance and is read by no bot.
//!
//! The range is cut in calendar months. One month of 5-minute bars is 1,800 points or less and one month of 1-minute bars is
//! 9,000 points or less (the page limit is 10,000), so each month is one request. The file is written after each month: a run that stops
//! (an error, HTTP 429, a closed laptop) keeps the months that are done, and the same command continues from the last cached day.
//!
//! The free plan has the IEX feed only: the volume is the volume of the IEX exchange, a small part of the market volume, and a
//! minute with no IEX trade has no bar. The prices are the prices of the IEX trades.

use super::alpaca::{api_symbol, now_utc, Client, Keys, StockQuery};
use super::{today_iso, write_atomic};
use crate::trading::calendar::{civil_from_days, days_from_civil, iso, parse_iso};
use crate::trading::data::{file_name, Interval, Store};
use crate::trading::fetch::merge;
use std::path::Path;

#[derive(Debug, Default, PartialEq)]
pub struct Stats {
    pub chunks: usize,
    pub rows_added: usize,
    pub rows_total: usize,
    pub bytes: u64,
    /// The first day of the first request (after the resume rule).
    pub started_at: String,
}

/// The date text of the first and the last data line of a cache file.
fn first_last_dates(text: &str) -> Option<(String, String)> {
    let mut lines = text.lines().skip(1).filter(|l| !l.is_empty());
    let first = lines.next()?;
    let last = lines.last().unwrap_or(first);
    Some((first.chars().take(10).collect(), last.chars().take(10).collect()))
}

/// The windows of the range: calendar months, the first and the last cut to the range.
pub fn month_windows(from: &str, to: &str) -> Vec<(String, String)> {
    let (Some(a), Some(b)) = (parse_iso(from), parse_iso(to)) else { return Vec::new() };
    let mut out = Vec::new();
    let mut start = a;
    while start <= b {
        let (y, m, _) = civil_from_days(start);
        let next = if m == 12 { days_from_civil(y + 1, 1, 1) } else { days_from_civil(y, m + 1, 1) };
        let end = (next - 1).min(b);
        out.push((iso(start), iso(end)));
        start = next;
    }
    out
}

/// Get the bars of `from` to `to` into `file`. Returns the numbers; on an error the months done stay in the file.
#[allow(clippy::too_many_arguments)]
pub fn sync(client: &Client, file: &Path, symbol: &str, minutes: i64, from: &str, to: &str, feed: &str, now: i64, progress: &mut dyn FnMut(&str)) -> Result<Stats, String> {
    let today = iso(now.div_euclid(86_400) as i32);
    let mut text = match std::fs::read_to_string(file) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("{}: {e}", file.display())),
    };
    // Resume: when the cache starts at the start of the range (a prefix is done), continue from the last cached day.
    let mut start = from.to_string();
    if let Some((first, last)) = text.as_deref().and_then(first_last_dates) {
        if let (Some(f0), Some(f1)) = (parse_iso(from), parse_iso(&first)) {
            if f1 <= f0 + 7 && last.as_str() > from {
                start = last;
            }
        }
    }
    let windows = month_windows(&start, to);
    let mut stats = Stats { started_at: start, rows_total: text.as_deref().map_or(0, |t| t.lines().count().saturating_sub(1)), ..Stats::default() };
    let timeframe = format!("{minutes}Min");
    for (k, (a, b)) in windows.iter().enumerate() {
        let bars = client
            .stock_bars(&StockQuery { symbols: std::slice::from_ref(&api_symbol(symbol)), timeframe: &timeframe, start: a, to: b, today: &today, adjustment: "split", feed })
            .map_err(|e| format!("{e}. {} of {} windows are done and kept in {}; run the same command again to continue", k, windows.len(), file.display()))?;
        let rows = bars.get(&api_symbol(symbol)).map(|v| super::alpaca::intraday_rows(v, minutes, now)).unwrap_or_default();
        if !rows.is_empty() {
            // Only the header depends on the interval, and it is the same text for 1-minute and 5-minute files.
            let (new_text, merged) = merge(text.as_deref(), &rows, Interval::FiveMinute)?;
            write_atomic(file, &new_text)?;
            stats.rows_added += merged.added;
            stats.rows_total = merged.after;
            text = Some(new_text);
        }
        stats.chunks += 1;
        progress(&format!("{a} to {b}: {} bars, {} rows in the file", rows.len(), stats.rows_total));
    }
    stats.bytes = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
    Ok(stats)
}

pub fn command(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let (mut symbol, mut interval, mut from, mut to, mut feed, mut source) = (None::<String>, "5m".to_string(), None::<String>, today_iso(), super::alpaca::feed_from_env(), "alpaca".to_string());
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().cloned().filter(|v| !v.starts_with("--")).ok_or_else(|| format!("{name} needs a value"));
        match a.as_str() {
            "--interval" => interval = value("--interval")?,
            "--from" => from = Some(value("--from")?),
            "--to" => to = value("--to")?,
            "--feed" => feed = value("--feed")?.to_lowercase(),
            "--source" => source = value("--source")?,
            s if s.starts_with("--") => return Err(format!("unknown option {s}; see fa prices --help")),
            s => {
                if symbol.replace(s.to_uppercase()).is_some() {
                    return Err("intraday takes one symbol".into());
                }
            }
        }
    }
    let symbol = symbol.ok_or("give a symbol: fa prices intraday SPY --interval 5m --from 2020-01-01")?;
    let minutes: i64 = match interval.as_str() {
        "1m" => 1,
        "5m" => 5,
        other => return Err(format!("bad --interval '{other}' (use 1m or 5m)")),
    };
    if source != "alpaca" {
        return Err(format!("intraday bars come from alpaca only (got '{source}')"));
    }
    if !matches!(feed.as_str(), "iex" | "sip") {
        return Err(format!("bad --feed '{feed}' (use iex or sip)"));
    }
    let from = from.ok_or("give --from YYYY-MM-DD (the first day; the free plan has data since 2016-01-01)")?;
    if parse_iso(&from).is_none() || parse_iso(&to).is_none() || from > to {
        return Err(format!("bad range {from} to {to}"));
    }
    let keys = Keys::read(repo).ok_or_else(|| format!("{} and {} are not both set in the environment or in .env", super::alpaca::KEY_ID_ENV, super::alpaca::SECRET_ENV))?;
    let client = Client::live(keys);
    let file = Store::from_env(repo).dir().join(file_name(&symbol, &interval));
    eprintln!("fa: {symbol} {interval} bars, feed {feed}, {from} to {to}, into {}", file.display());
    let stats = sync(&client, &file, &symbol, minutes, &from, &to, &feed, now_utc(), &mut |m| eprintln!("fa: {m}"))?;
    println!("intraday {symbol} {interval}: {} windows from {}, {} rows added, {} rows and {} bytes in {}; {} requests", stats.chunks, stats.started_at, stats.rows_added, stats.rows_total, stats.bytes, file.display(), client.requests());
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::alpaca::{parse_rfc3339, Keys};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn param(url: &str, name: &str) -> Option<String> {
        url.split_once('?')?.1.split('&').find_map(|kv| kv.strip_prefix(&format!("{name}="))).map(String::from)
    }

    /// A server of the documented form: for each day of the window, three 5-minute bars (09:30, 09:35 and 16:00 New
    /// York time, the last is outside the session). January to March 2024 is standard time (UTC-5).
    fn server(fail_on: Option<usize>, calls: Rc<Cell<usize>>, seen: Rc<RefCell<Vec<String>>>) -> crate::market::price_source::Get {
        Box::new(move |url| {
            calls.set(calls.get() + 1);
            seen.borrow_mut().push(url.to_string());
            if Some(calls.get()) == fail_on {
                return Err("HTTP 500 for x".into());
            }
            let a = parse_iso(&param(url, "start").unwrap()).unwrap();
            let b = parse_iso(&param(url, "end").unwrap()[..10]).unwrap();
            let mut list = Vec::new();
            for d in a..=b {
                for (h, m, price) in [(14, 30, 100.0), (14, 35, 101.0), (21, 0, 102.0)] {
                    list.push(serde_json::json!({"t": format!("{}T{h:02}:{m:02}:00Z", iso(d)), "o": price, "h": price, "l": price, "c": price, "v": 1000}));
                }
            }
            Ok(serde_json::to_vec(&serde_json::json!({"bars": {"SPY": list}, "next_page_token": null})).unwrap())
        })
    }

    fn keys() -> Keys {
        Keys { id: "KEYIDVALUE".into(), secret: "SECRETVALUE99".into() }
    }

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("fa_intra_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn months() {
        assert_eq!(month_windows("2024-01-15", "2024-03-10"), [("2024-01-15".into(), "2024-01-31".into()), ("2024-02-01".into(), "2024-02-29".into()), ("2024-03-01".into(), "2024-03-10".into())]);
        assert_eq!(month_windows("2023-12-30", "2024-01-02").len(), 2);
        assert!(month_windows("2024-02-01", "2024-01-01").is_empty());
    }

    #[test]
    fn range_into_the_cache_stop_and_continue() {
        let d = dir("range");
        let file = d.join("SPY_5m.csv");
        let now = parse_rfc3339("2026-11-19T12:00:00Z").unwrap();
        let (calls, seen) = (Rc::new(Cell::new(0)), Rc::new(RefCell::new(vec![])));
        // The third request fails: two months are done and kept.
        let client = Client::new(keys(), server(Some(3), calls.clone(), seen.clone()));
        let err = sync(&client, &file, "SPY", 5, "2024-01-15", "2024-03-10", "iex", now, &mut |_| {}).unwrap_err();
        assert!(err.contains("2 of 3 windows are done"), "{err}");
        let text = std::fs::read_to_string(&file).unwrap();
        let rows = text.lines().count() - 1;
        assert_eq!(rows, (17 + 29) * 2, "17 days of January and 29 of February, two bars in the session each");
        assert!(text.starts_with("Datetime,open,high,low,close,volume\n2024-01-15 09:30:00-05:00,100.0,"), "{}", &text[..80]);
        // The same command again: it continues from the last cached day (2024-02-29), not from the start.
        let (calls2, seen2) = (Rc::new(Cell::new(0)), Rc::new(RefCell::new(vec![])));
        let client = Client::new(keys(), server(None, calls2.clone(), seen2.clone()));
        let stats = sync(&client, &file, "SPY", 5, "2024-01-15", "2024-03-10", "iex", now, &mut |_| {}).unwrap();
        assert_eq!(stats.started_at, "2024-02-29");
        assert_eq!(calls2.get(), 2, "the last day of February, then March");
        assert!(seen2.borrow()[0].contains("start=2024-02-29") && seen2.borrow()[0].contains("timeframe=5Min"));
        assert_eq!(stats.rows_total, (17 + 29 + 10) * 2);
        assert_eq!(stats.rows_added, 10 * 2, "the 29th was in the cache already");
        // A third run with nothing new adds no row.
        let stats = sync(&client, &file, "SPY", 5, "2024-01-15", "2024-03-10", "iex", now, &mut |_| {}).unwrap();
        assert_eq!(stats.rows_added, 0);
        assert!(stats.bytes > 0);
        // No key in any URL.
        assert!(seen.borrow().iter().chain(seen2.borrow().iter()).all(|u| !u.contains("KEYIDVALUE") && !u.contains("SECRETVALUE99")));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn size_of_a_year_of_bars() {
        // A typical line: 62 bytes with the newline. A year has 252 sessions: 78 bars of 5 minutes or 390 bars of 1 minute.
        let line = crate::trading::fetch::cache_line(1_705_588_200, 478.31, 478.52, 478.2, 478.45, 41_276.0);
        assert_eq!(line, "2024-01-18 09:30:00-05:00,478.31,478.52,478.2,478.45,41276");
        let bytes = (line.len() + 1) as u64;
        assert_eq!(bytes, 59);
        assert!(252 * 78 * bytes < 1_250_000, "5-minute: 19,656 rows, about 1.2 MB");
        assert!(252 * 390 * bytes < 6_100_000, "1-minute: 98,280 rows, about 5.8 MB");
    }
}
