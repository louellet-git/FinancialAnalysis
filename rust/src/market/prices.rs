//! Daily prices: the Yahoo chart endpoint (the Rust form of `load_prices` of `market_data.py`).
//!
//! The Python code used the library yfinance: `history(period="max")` with its default
//! `auto_adjust=True`. The rules of that call are the rules here:
//!
//! - Open, high, low and close are adjusted for splits and dividends: each of open, high and low
//!   is multiplied by `adjclose / close`, and the close is the adjusted close.
//! - One row for each date of the exchange. When the last two rows have the same date (Yahoo adds
//!   a live row during the session), the last row stays. For any other double date, the first row stays.
//! - A row with no open, high, low or close is dropped. (The Python code wrote `NaN` for such a row,
//!   which is not JSON.) A row with no price and no volume (all zero or empty) is dropped.
//! - Prices and the volume are in the cache as `round(x, 6)` and an integer.
//!
//! The cache file is `data/raw/<T>/prices.json`. A cache file is used as it is, whatever its age
//! (as in the Python code); `refresh` asks Yahoo again. Yahoo answers HTTP 429 (too many
//! requests) to many requests from a cloud container. Then the cache file stays as it is and
//! the program says so.

use super::{ticker_dir, write_atomic};
use crate::futures::yahoo::parse_raw;
use crate::trading::calendar::{iso, ny_offset};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

pub const BASE: &str = "https://query1.finance.yahoo.com/v8/finance/chart/";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PricePoint {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
}

#[derive(Deserialize)]
struct CachedPrice {
    date: String,
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
    volume: Option<f64>,
}

pub fn url(ticker: &str) -> String {
    // `period1` and `period2` give daily rows for the whole history; `range=max` can give a coarser interval
    // (164 rows for MSFT in a test on 2026-11-19).
    let end = (mapcheck::dates::today() + 2) * 86_400;
    format!("{BASE}{}?period1=0&period2={end}&interval=1d&includeAdjustedClose=true", ticker.to_uppercase())
}

/// The price points of a chart reply.
pub fn from_chart(bytes: &[u8]) -> Result<Vec<PricePoint>, String> {
    let raw = parse_raw(bytes)?;
    let root: Value = serde_json::from_slice(bytes).map_err(|e| format!("Yahoo response is not JSON: {e}"))?;
    let n = raw.stamps.len();
    let adj: Option<Vec<Option<f64>>> = root["chart"]["result"][0]["indicators"]["adjclose"][0]["adjclose"].as_array().map(|a| a.iter().map(Value::as_f64).collect());
    if let Some(a) = &adj {
        if a.len() != n {
            return Err(format!("Yahoo adjclose array has {} items; the time stamp array has {n}", a.len()));
        }
    }
    // The date of each row in the time zone of the exchange (New York for all U.S. stocks).
    let new_york = raw.time_zone == "America/New_York";
    let mut rows: Vec<(String, [Option<f64>; 4], Option<f64>)> = Vec::new(); // date, open/high/low/close, volume
    for i in 0..n {
        let utc = raw.stamps[i];
        let day = (utc + if new_york { ny_offset(utc) } else { raw.offset }).div_euclid(86_400);
        let (open, high, low, close) = (raw.open[i], raw.high[i], raw.low[i], raw.close[i]);
        // yfinance: ratio = adjclose / close; a missing adjclose means "no adjustment".
        let adjusted = |x: Option<f64>| -> Option<f64> {
            match (x, close, adj.as_ref().and_then(|a| a[i])) {
                (Some(x), Some(c), Some(a)) => Some(x * (a / c)),
                (Some(x), _, None) => Some(x),
                _ => None,
            }
        };
        let close_adjusted = match (adj.as_ref().and_then(|a| a[i]), close) {
            (Some(a), _) => Some(a),
            (None, c) => c,
        };
        rows.push((iso(day as i32), [adjusted(open), adjusted(high), adjusted(low), close_adjusted], raw.volume[i]));
    }
    // Yahoo returns the live row of today besides the daily row: the last row stays.
    if rows.len() > 1 && rows[rows.len() - 1].0 == rows[rows.len() - 2].0 {
        rows.remove(rows.len() - 2);
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (date, [open, high, low, close], volume) in rows {
        if !seen.insert(date.clone()) {
            continue; // any other double date: the first row stays
        }
        let empty = |x: Option<f64>| x.is_none_or(|v| v == 0.0);
        if empty(open) && empty(high) && empty(low) && empty(close) && empty(volume) {
            continue;
        }
        let (Some(open), Some(high), Some(low), Some(close)) = (open, high, low, close) else { continue };
        if ![open, high, low, close].iter().all(|v| v.is_finite()) {
            continue;
        }
        out.push(PricePoint {
            date,
            open: super::round6(open),
            high: super::round6(high),
            low: super::round6(low),
            close: super::round6(close),
            volume: volume.filter(|v| v.is_finite()).unwrap_or(0.0) as i64,
        });
    }
    Ok(out)
}

pub fn cache_path(cache: &Path, ticker: &str) -> std::path::PathBuf {
    ticker_dir(cache, ticker).join("prices.json")
}

/// The cache file as it is in `data/raw/<T>/prices.json`.
pub fn read_cache(path: &Path) -> Result<Vec<PricePoint>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let raw: Vec<CachedPrice> = serde_json::from_str(&super::lenient_json(&text)).map_err(|e| format!("{}: {e}", path.display()))?;
    let nan = f64::NAN;
    Ok(raw
        .into_iter()
        .map(|r| PricePoint {
            date: r.date,
            open: r.open.unwrap_or(nan),
            high: r.high.unwrap_or(nan),
            low: r.low.unwrap_or(nan),
            close: r.close.unwrap_or(nan),
            volume: r.volume.unwrap_or(0.0) as i64,
        })
        .collect())
}

pub fn write_cache(path: &Path, prices: &[PricePoint]) -> Result<(), String> {
    write_atomic(path, &serde_json::to_string_pretty(prices).map_err(|e| e.to_string())?)
}

#[derive(Debug, PartialEq)]
pub enum Source {
    Cache,
    Fetched,
    /// The refresh failed; the cache was used. The text is the reason.
    CacheAfterFailure(String),
}

/// The message for a failed Yahoo request. HTTP 429 gets a clear text.
pub fn explain(error: &str) -> String {
    if error.starts_with("HTTP 429 ") {
        "Yahoo answered HTTP 429 (too many requests). The program does not get around this. Try again later".to_string()
    } else {
        error.to_string()
    }
}

/// The prices of a ticker: the cache file, or Yahoo (`fetch` gives the bytes of the chart reply).
pub fn load_prices(
    cache: &Path,
    ticker: &str,
    refresh: bool,
    fetch: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<(Vec<PricePoint>, Source), String> {
    let path = cache_path(cache, ticker);
    let mut cached: Option<Vec<PricePoint>> = None;
    if path.is_file() {
        match read_cache(&path) {
            Ok(prices) => {
                if !refresh {
                    return Ok((prices, Source::Cache));
                }
                cached = Some(prices);
            }
            Err(e) => eprintln!("fa: corrupt price cache for {ticker} ({e}); asking Yahoo again"),
        }
    }
    let result = fetch(&url(ticker)).and_then(|bytes| from_chart(&bytes));
    match result {
        Ok(prices) if prices.is_empty() => {
            eprintln!("fa: Yahoo returned no prices for {ticker}");
            Ok((Vec::new(), Source::Fetched))
        }
        Ok(prices) => {
            write_cache(&path, &prices)?;
            Ok((prices, Source::Fetched))
        }
        Err(e) => match cached {
            Some(prices) => Ok((prices, Source::CacheAfterFailure(explain(&e)))),
            None => Err(explain(&e)),
        },
    }
}

/// The last close (the price of the verdict), if there is one.
pub fn last_close(prices: &[PricePoint]) -> Option<f64> {
    prices.last().map(|p| p.close).filter(|c| c.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        let path = format!("{}/tests/fixtures/market/yahoo_prices.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn points(v: &Value) -> Vec<PricePoint> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|r| PricePoint {
                date: r["date"].as_str().unwrap().to_string(),
                open: r["open"].as_f64().unwrap(),
                high: r["high"].as_f64().unwrap(),
                low: r["low"].as_f64().unwrap(),
                close: r["close"].as_f64().unwrap(),
                volume: r["volume"].as_i64().unwrap(),
            })
            .collect()
    }

    #[test]
    fn chart_gives_the_prices_that_yfinance_gives() {
        // Expected rows: the yfinance functions (parse_quotes, live-row fix, auto_adjust) on this reply.
        let f = fixture();
        let got = from_chart(f["chart"].to_string().as_bytes()).unwrap();
        assert_eq!(got, points(&f["expected"]));
    }

    #[test]
    fn cache_round_trip_keeps_the_python_format() {
        let dir = std::env::temp_dir().join(format!("fa-prices-{}", std::process::id()));
        let path = dir.join("TEST").join("prices.json");
        let prices = points(&fixture()["expected"]);
        write_cache(&path, &prices).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("[\n  {\n    \"date\": \"2024-03-07\",\n    \"open\": 98.362929,"), "{text}");
        assert_eq!(read_cache(&path).unwrap(), prices);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cache_file_is_used_and_a_refresh_that_fails_keeps_it() {
        let dir = std::env::temp_dir().join(format!("fa-prices-429-{}", std::process::id()));
        let prices = points(&fixture()["expected"]);
        write_cache(&cache_path(&dir, "test"), &prices).unwrap();
        let never = |_: &str| -> Result<Vec<u8>, String> { panic!("no request expected") };
        let (got, source) = load_prices(&dir, "TEST", false, &never).unwrap();
        assert_eq!((got.len(), source), (5, Source::Cache));
        let too_many = |u: &str| -> Result<Vec<u8>, String> { Err(format!("HTTP 429 for {u}")) };
        let (got, source) = load_prices(&dir, "TEST", true, &too_many).unwrap();
        assert_eq!(got, prices);
        match source {
            Source::CacheAfterFailure(m) => assert!(m.contains("HTTP 429") && m.contains("too many requests"), "{m}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(read_cache(&cache_path(&dir, "TEST")).unwrap(), prices, "the cache file stays");
        // No cache and no answer: an error with the clear text.
        let e = load_prices(&dir, "NONE", false, &too_many).unwrap_err();
        assert!(e.contains("HTTP 429"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_fetch_writes_the_cache() {
        let dir = std::env::temp_dir().join(format!("fa-prices-fetch-{}", std::process::id()));
        let reply = fixture()["chart"].to_string().into_bytes();
        let fetch = |u: &str| -> Result<Vec<u8>, String> {
            assert!(u.contains("/TEST?period1=0&period2="), "{u}");
            Ok(reply.clone())
        };
        let (got, source) = load_prices(&dir, "test", false, &fetch).unwrap();
        assert_eq!((got.len(), source), (5, Source::Fetched));
        assert_eq!(read_cache(&cache_path(&dir, "TEST")).unwrap(), got);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nan_in_a_cache_file_is_read() {
        let dir = std::env::temp_dir().join(format!("fa-prices-nan-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("T")).unwrap();
        std::fs::write(dir.join("T/prices.json"), r#"[{"date":"2020-01-02","open":NaN,"high":2.0,"low":1.0,"close":1.5,"volume":10}]"#).unwrap();
        let p = read_cache(&dir.join("T/prices.json")).unwrap();
        assert!(p[0].open.is_nan() && p[0].high == 2.0 && p[0].volume == 10);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn last_close_ignores_nan() {
        assert_eq!(last_close(&[]), None);
        let mut p = points(&fixture()["expected"]);
        assert_eq!(last_close(&p), Some(108.0));
        p.last_mut().unwrap().close = f64::NAN;
        assert_eq!(last_close(&p), None);
    }
}
