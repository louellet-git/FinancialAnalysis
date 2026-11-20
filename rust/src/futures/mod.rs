//! Futures data from free public sources.
//!
//! Sources: Yahoo Finance chart endpoint (CME and CBOT contracts), Cboe (VIX futures
//! settlement files), CFTC (Commitments of Traders) and the Atlanta Fed Market
//! Probability Tracker. Each value has a date, a source, a URL and a price type.
//! Rates are decimals (0.0507 = 5.07%). Prices stay in the units of the exchange.

pub mod cboe;
pub mod cftc;
pub mod check;
pub mod cli;
pub mod facts;
pub mod mpt;
pub mod yahoo;

use mapcheck::dates;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub type Result<T> = std::result::Result<T, String>;

pub const USER_AGENT: &str = "FinancialAnalysis fa (personal research)";
/// Seconds between two requests to one host.
const DELAY_SECONDS: f64 = 0.5;

/// One value of one date.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Observation {
    pub date: String,
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_interest: Option<f64>,
    /// Decimal. Only for rate futures: (100 - price) / 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implied_rate: Option<f64>,
}

/// The last trade of a session that is not finished. It is not a settlement.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Live {
    pub time_utc: String,
    pub price: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Series {
    pub symbol: String,
    pub name: String,
    pub source: String,
    pub url: String,
    pub price_type: String,
    pub time_zone: String,
    /// The caveats that the caller must see.
    pub note: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live: Option<Live>,
    pub observations: Vec<Observation>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CurvePoint {
    pub contract: String,
    /// The date of a daily value, or the time (UTC) of a last trade.
    pub as_of: String,
    pub price_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implied_rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Curve {
    pub root: String,
    pub name: String,
    pub date: String,
    pub requested_date: String,
    pub source: String,
    pub url: String,
    pub price_type: String,
    pub note: String,
    pub points: Vec<CurvePoint>,
}

/// (100 - price) / 100, rounded to 8 digits.
pub fn implied_rate(price: f64) -> f64 {
    ((100.0 - price) / 100.0 * 1e8).round() / 1e8
}

/// Time as ISO text, from epoch seconds.
pub fn time_utc(secs: i64) -> String {
    let rest = secs.rem_euclid(86_400);
    format!("{}T{:02}:{:02}:{:02}Z", dates::format(secs.div_euclid(86_400)), rest / 3600, rest % 3600 / 60, rest % 60)
}

/// The weekday of a day number: 0 = Monday ... 6 = Sunday. (1970-01-01 was a Thursday.)
pub fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// The last weekday before today. A holiday is not known here.
pub fn last_weekday_before_today() -> i64 {
    let mut day = dates::today() - 1;
    while weekday(day) >= 5 {
        day -= 1;
    }
    day
}

pub fn cache_dir(repo: &Path) -> PathBuf {
    repo.join("data").join("raw").join("futures")
}

/// One GET request. It goes through the queue of its host (`hostnet`) with a pause of 0.5 seconds.
pub fn download(url: &str) -> Result<Vec<u8>> {
    crate::hostnet::get(url, &[], DELAY_SECONDS)
}

/// Ignore the age of the cache files for one run: a file that parses is still used when `enough` says that it holds
/// what the caller needs, but an old-by-the-clock file is asked again (`fa sync` has decided that the data is old).
pub fn ignore_cache_age(on: bool) {
    IGNORE_AGE.store(on, std::sync::atomic::Ordering::SeqCst);
}

static IGNORE_AGE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn age(path: &Path) -> Option<Duration> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| SystemTime::now().duration_since(t).ok())
}

/// The value of `url`, from the cache file or from the network.
/// - A cache file is used when it parses and it is younger than `max_age`, or when
///   `enough` says that it holds what the caller needs (old days do not change).
/// - A new file goes to the cache only after it parses.
/// - If the download or the parse fails and an old file parses, the old file is used and a
///   warning goes to stderr.
pub fn load<T>(
    repo: &Path,
    file: &str,
    url: &str,
    max_age: Duration,
    parse: impl Fn(&[u8]) -> Result<T>,
    enough: impl Fn(&T) -> bool,
) -> Result<T> {
    let path = cache_dir(repo).join(file);
    let old = std::fs::read(&path).ok().and_then(|bytes| parse(&bytes).ok());
    if let Some(value) = old.as_ref() {
        if (age(&path).is_some_and(|a| a < max_age) && !IGNORE_AGE.load(std::sync::atomic::Ordering::SeqCst)) || enough(value) {
            return Ok(old.unwrap());
        }
    }
    let fetched = download(url).and_then(|bytes| parse(&bytes).map(|v| (bytes, v)));
    match fetched {
        Ok((bytes, value)) => {
            let dir = path.parent().expect("cache path has a parent");
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let tmp = path.with_extension("tmp");
            std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
            std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(value)
        }
        Err(e) => match old {
            Some(value) => {
                eprintln!("fa: download failed ({e}); using the old file {}", path.display());
                Ok(value)
            }
            None => Err(e),
        },
    }
}

pub fn text(bytes: &[u8], what: &str) -> Result<String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| format!("{what} is not UTF-8 text"))
}

/// Read a number from text. Rejects NaN and infinity.
pub fn number(field: &str, what: &str) -> Result<f64> {
    let v: f64 = field.trim().parse().map_err(|_| format!("{what}: bad number '{field}'"))?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{what}: '{field}' is not finite"))
    }
}

/// A short HTTP-status check for the callers that skip a missing contract.
pub fn is_not_found(error: &str) -> bool {
    error.starts_with("HTTP 404 ")
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

use crate::forwards::parse_date_arg;
use facts::{Contract, Symbol};

fn series_for(repo: &Path, symbol: &Symbol, start: i64, end: i64, latest: bool) -> Result<Series> {
    match symbol {
        Symbol::Continuous(r) if r.root == "VX" => Err("VX needs a contract, for example VXZ26 (the curve of one date: fa futures curve VX)".into()),
        Symbol::Contract(c) if c.root.root == "VX" => cboe::series(repo, c, start, end, latest),
        _ => yahoo::series(repo, symbol, start, end, latest),
    }
}

/// The last daily value on or before `date` (default: today), and the running session if any.
pub fn quote(repo: &Path, symbol: &str, date: Option<&str>) -> Result<Series> {
    let (_, day) = parse_date_arg(date.unwrap_or("today"))?;
    series_for(repo, &facts::parse_symbol(symbol)?, 0, day, true)
}

pub fn history(repo: &Path, symbol: &str, start: &str, end: Option<&str>) -> Result<Series> {
    let (_, from) = parse_date_arg(start)?;
    let (_, to) = parse_date_arg(end.unwrap_or("today"))?;
    if to < from {
        return Err("--end is before --start".into());
    }
    series_for(repo, &facts::parse_symbol(symbol)?, from, to, false)
}

/// The listed contracts of a root from the month of `day`, in order. At most 12.
fn candidates(root: &'static facts::Root, day: i64) -> Vec<Contract> {
    let civil = dates::format(day);
    let (mut year, mut month): (i64, u32) = (civil[..4].parse().unwrap_or(2000), civil[5..7].parse().unwrap_or(1));
    let mut out = Vec::new();
    for _ in 0..48 {
        if root.months.contains(facts::month_letter(month)) && out.len() < 12 {
            out.push(Contract { root, month, year });
        }
        month += 1;
        if month == 13 {
            month = 1;
            year += 1;
        }
    }
    out
}

/// All listed contracts of one root on one date. Without a date: the last day of the first contract.
pub fn curve(repo: &Path, root: &str, date: Option<&str>) -> Result<Curve> {
    let root = facts::root_or_error(root)?;
    let requested = date.unwrap_or("latest").to_string();
    if root.root == "VX" {
        if date.is_some() {
            let (text, day) = parse_date_arg(date.unwrap_or_default())?;
            return cboe::curve(repo, &text, day);
        }
        let mut last = String::new();
        for back in 0..7 {
            match cboe::curve(repo, &requested, dates::today() - 1 - back) {
                Ok(c) => return Ok(c),
                Err(e) if e.starts_with("Cboe has no VX settlement") => last = e,
                Err(e) => return Err(e),
            }
        }
        return Err(last);
    }
    let fixed = match date {
        Some(d) => Some(parse_date_arg(d)?.1),
        None => None,
    };
    let (mut points, mut misses) = (Vec::new(), 0);
    for c in candidates(root, fixed.unwrap_or_else(dates::today)) {
        let yahoo = yahoo::yahoo_symbol(&Symbol::Contract(c.clone()))?;
        let chart = match yahoo::chart(repo, &yahoo, fixed) {
            Ok(chart) => chart,
            Err(e) if is_not_found(&e) => {
                misses += 1;
                if !points.is_empty() && misses >= 2 {
                    break;
                }
                continue;
            }
            Err(e) => return Err(e),
        };
        misses = 0;
        // With a date: the daily bar of the date. Without: the last trade, else the last daily bar.
        let found = match fixed {
            Some(day) => chart.bars.iter().find(|b| b.day == day).map(|b| (b.close, dates::format(day), yahoo::DAILY)),
            None => chart
                .live
                .map(|(ts, price)| (price, time_utc(ts), yahoo::LAST_TRADE))
                .or_else(|| chart.bars.last().map(|b| (b.close, dates::format(b.day), yahoo::DAILY))),
        };
        if let Some((price, as_of, price_type)) = found {
            points.push(CurvePoint {
                contract: c.code(),
                as_of,
                price_type,
                expiry: None,
                price,
                implied_rate: root.implied_rate.is_some().then(|| implied_rate(price)),
            });
        }
    }
    if points.is_empty() {
        return Err(format!(
            "no Yahoo contract of {} has a value for {requested}; Yahoo keeps only contracts that have not expired{}",
            root.root,
            if root.implied_rate.is_some() { ", and its daily rows of SOFR futures are mostly empty: try without --date" } else { "" }
        ));
    }
    let date = points.iter().map(|p| p.as_of[..10].to_string()).max().unwrap_or_default();
    Ok(Curve {
        root: root.root.into(),
        name: root.name.into(),
        date,
        requested_date: requested,
        source: "Yahoo Finance chart endpoint (CME Group data)".into(),
        url: yahoo::URL.into(),
        price_type: if fixed.is_some() { yahoo::PRICE_TYPE } else { "each point has its own: last trade, or the daily close when there is no running session" }.into(),
        note: "One Yahoo request for each contract. A contract that has expired is not on Yahoo, so the curve of a past date has only the contracts that still trade. \
               A contract with no bar on the date is left out. Without a date the curve has the last trades, each with its time."
            .into(),
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_contracts() {
        let day = dates::parse("2026-11-19").unwrap();
        let zq = candidates(facts::root("ZQ").unwrap(), day);
        assert_eq!((zq.len(), zq[0].code().as_str(), zq[1].code().as_str(), zq[2].code().as_str()), (12, "ZQX26", "ZQZ26", "ZQF27"));
        let es = candidates(facts::root("ES").unwrap(), day);
        assert_eq!((es[0].code().as_str(), es[1].code().as_str(), es[3].code().as_str()), ("ESZ26", "ESH27", "ESU27"));
    }

    #[test]
    fn rates_and_time() {
        assert_eq!(implied_rate(95.702), 0.04298);
        assert_eq!(time_utc(1_795_060_082), "2026-11-19T03:48:02Z");
        assert_eq!(weekday(dates::parse("2026-11-19").unwrap()), 3); // Thursday
        assert_eq!(weekday(dates::parse("2026-11-21").unwrap()), 5);
    }

    #[test]
    fn number_rejects_bad_input() {
        assert_eq!(number(" 4.5 ", "x").unwrap(), 4.5);
        assert!(number("abc", "x").is_err());
        assert!(number("NaN", "x").is_err());
        assert!(number("inf", "x").is_err());
    }
}
