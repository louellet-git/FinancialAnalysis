//! Cash distributions of a fund (ex-date, amount per share), for the price total return of `fa fund attribution`.
//!
//! Source: the Yahoo chart endpoint with `events=div` (one request for the full history; no key; the client name and the
//! 429 rule of `price_source`). The Nasdaq dividend endpoint has no rows for URA (a NYSE Arca symbol: "Dividend History for
//! Non-Nasdaq symbols is not available", checked 2026-11-26). Yahoo gives the ex-date and the amount, not the pay date.
//! The cache file is `DIR/TICKER/distributions.json` with a record: source, date of the request, address.
//!
//! `fa fund distributions TICKER [--cache DIR]` makes the request (at most one) and writes the file.

use super::price_source::{classify, FetchError, Yahoo};
use super::{download, write_atomic};
use crate::trading::calendar::{iso, ny_offset};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Distribution {
    pub ex_date: String,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub source: String,
    pub fetched: String,
    pub address: String,
    pub note: String,
    pub distributions: Vec<Distribution>,
}

pub fn path(cache: &Path, ticker: &str) -> PathBuf {
    cache.join(ticker).join("distributions.json")
}

/// The cash distributions of a Yahoo chart reply (`events.dividends`), oldest first. The ex-date is the New York day of the event time.
pub fn parse_yahoo_distributions(bytes: &[u8]) -> Result<Vec<Distribution>, String> {
    let root: Value = serde_json::from_slice(bytes).map_err(|e| format!("Yahoo response is not JSON: {e}"))?;
    if let Some(desc) = root["chart"]["error"]["description"].as_str() {
        return Err(format!("Yahoo error: {}", desc.chars().take(200).collect::<String>()));
    }
    let result = &root["chart"]["result"][0];
    if result.is_null() {
        return Err("Yahoo reply has no result".into());
    }
    let mut out = Vec::new();
    if let Some(map) = result["events"]["dividends"].as_object() {
        for ev in map.values() {
            if let (Some(ts), Some(amount)) = (ev["date"].as_i64(), ev["amount"].as_f64()) {
                if amount.is_finite() && amount > 0.0 {
                    out.push(Distribution { ex_date: iso((ts + ny_offset(ts)).div_euclid(86_400) as i32), amount: super::round6(amount) });
                }
            }
        }
    }
    out.sort_by(|a, b| a.ex_date.cmp(&b.ex_date));
    out.dedup_by(|b, a| a.ex_date == b.ex_date);
    Ok(out)
}

pub fn read(cache: &Path, ticker: &str) -> Option<Record> {
    serde_json::from_str(&std::fs::read_to_string(path(cache, ticker)).ok()?).ok()
}

/// The sum of the distributions with an ex-date in (d0, d1] (after d0, up to and with d1).
pub fn sum_between(d: &[Distribution], d0: &str, d1: &str) -> f64 {
    // `0.0 +` keeps an empty sum at +0.0 (the sum of nothing is -0.0 in Rust).
    0.0 + d.iter().filter(|x| x.ex_date.as_str() > d0 && x.ex_date.as_str() <= d1).map(|x| x.amount).sum::<f64>()
}

/// The price total return of a quarter: (close_end + distributions with an ex-date in (d0, d1]) / close_start - 1.
/// The distributions are not reinvested inside the quarter (a small second-order effect: the cash earns the fund return
/// from the ex-date to d1, about 0.1 pp or less for a yearly distribution of 1% to 2% and a move of 10% in the rest of the quarter).
pub fn total_return(close0: f64, close1: f64, dist: &[Distribution], d0: &str, d1: &str) -> f64 {
    (close1 + sum_between(dist, d0, d1)) / close0 - 1.0
}

pub const USAGE: &str = "fa fund distributions TICKER [--cache DIR]   one Yahoo request: the cash distributions into DIR/TICKER/distributions.json (default data/raw)";

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    let t = args.first().ok_or(USAGE)?;
    if matches!(t.as_str(), "-h" | "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let ticker = t.to_uppercase();
    if !super::fund_holdings::valid_ticker(&ticker) {
        return Err(format!("{t}: not a ticker"));
    }
    let mut cache = repo.join("data").join("raw");
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--cache" => cache = PathBuf::from(it.next().ok_or("--cache needs a folder")?),
            x => return Err(format!("unknown option {x}")),
        }
    }
    let today = super::today_iso();
    let url = Yahoo::url(&ticker, None, &today);
    let bytes = download(&url).map_err(|e| match classify(e) {
        FetchError::RateLimited(m) => format!("{m}: the host blocks; no retry in this run"),
        FetchError::Failed(m) => m,
    })?;
    let rows = parse_yahoo_distributions(&bytes)?;
    let rec = Record {
        source: "Yahoo chart (events=div)".into(),
        fetched: today,
        address: url,
        note: "ex-date and cash amount per share (share count of the fetch date); no pay date in this source".into(),
        distributions: rows,
    };
    write_atomic(&path(&cache, &ticker), &serde_json::to_string_pretty(&rec).map_err(|e| e.to_string())?)?;
    println!("{ticker}: {} distributions into {} (source {}, {})", rec.distributions.len(), path(&cache, &ticker).display(), rec.source, rec.fetched);
    for d in &rec.distributions {
        println!("  ex-date {}  {:.6} per share", d.ex_date, d.amount);
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed form of a Yahoo chart answer: two events (13:30 UTC = 09:30 New York) and the other fields that are not read.
    const ANSWER: &str = r#"{"chart":{"result":[{"meta":{"symbol":"URA","exchangeTimezoneName":"America/New_York"},
        "events":{"dividends":{"1734618600":{"amount":0.5,"date":1734618600},"1702996200":{"amount":0.3149,"date":1702996200}}}}],"error":null}}"#;

    fn d(date: &str, amount: f64) -> Distribution {
        Distribution { ex_date: date.into(), amount }
    }

    #[test]
    fn parse_gives_ex_dates_oldest_first() {
        let rows = parse_yahoo_distributions(ANSWER.as_bytes()).unwrap();
        assert_eq!(rows, vec![d("2023-12-19", 0.3149), d("2024-12-19", 0.5)]);
        assert!(parse_yahoo_distributions(br#"{"chart":{"result":[{"events":null}],"error":null}}"#).unwrap().is_empty());
        assert!(parse_yahoo_distributions(br#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found"}}}"#).unwrap_err().contains("Yahoo error"));
        assert!(parse_yahoo_distributions(b"<html>").is_err());
    }

    #[test]
    fn total_return_counts_a_distribution_inside_the_quarter_only() {
        let dist = vec![d("2024-12-19", 0.5)];
        // Quarter 2024-09-30 to 2024-12-31 holds the ex-date: (30 + 0.5) / 31 - 1.
        let inside = total_return(31.0, 30.0, &dist, "2024-09-30", "2024-12-31");
        assert!((inside - (30.5 / 31.0 - 1.0)).abs() < 1e-12);
        // Quarter 2024-12-31 to 2025-03-31 does not: price return.
        let outside = total_return(30.0, 33.0, &dist, "2024-12-31", "2025-03-31");
        assert!((outside - 0.1).abs() < 1e-12);
        // The start date is out and the end date is in: (d0, d1].
        assert_eq!(sum_between(&dist, "2024-12-19", "2025-03-31"), 0.0);
        assert_eq!(sum_between(&dist, "2024-09-30", "2024-12-19"), 0.5);
    }
}
