//! Sources of daily prices: Tiingo, the Yahoo chart endpoint and the Nasdaq site API.
//!
//! One trait (`PriceSource`) for all three. A source gives `Row`s. The close of a row is split
//! adjusted and not dividend adjusted (the price that the exchange showed, in the share count of
//! today). `adj_close` is set when the source also adjusts for dividends. The cache (`price_cache`)
//! chooses which of the two it keeps.
//!
//! | Source | Key | Splits | Dividends | Depth | Official API |
//! |---|---|---|---|---|---|
//! | Tiingo | `TIINGO_API_KEY` | adjusted | adjusted (`adjClose`) | 30 years and more | yes |
//! | Yahoo chart | none | adjusted | adjusted (`adjclose`) | from the first trade | no |
//! | Nasdaq site | none | adjusted | not adjusted | 10 years | no |
//! | Alpaca | `APCA_API_KEY_ID`, `APCA_API_SECRET_KEY` | adjusted | not adjusted | since 2016 | yes (`alpaca.rs`) |
//!
//! The Tiingo loader follows the documentation (`tiingo.com/documentation/end-of-day`, read on
//! 2026-11-19). It was not run against the real site: there was no key.
//!
//! The parsers are functions of the reply bytes, so the tests need no network.

use super::{download, download_with_headers, parse_date, round6};
use crate::futures::yahoo::parse_raw;
use crate::trading::calendar::{iso, ny_offset};
use serde_json::Value;
use std::path::Path;

pub const TIINGO_KEY_ENV: &str = "TIINGO_API_KEY";
pub const TIINGO_BASE: &str = "https://api.tiingo.com/tiingo/daily";
pub const YAHOO_BASE: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
pub const NASDAQ_BASE: &str = "https://api.nasdaq.com/api/quote";
/// The start date of a full history request.
pub const FULL_FROM: &str = "1970-01-01";

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    /// Split adjusted, not dividend adjusted.
    pub close: f64,
    pub volume: i64,
    /// Split and dividend adjusted close, when the source has it.
    pub adj_close: Option<f64>,
    /// Cash dividend with this ex-date.
    pub dividend: Option<f64>,
    /// Split ratio with this date (2.0 for a 2-for-1 split), when the source has it.
    pub split: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FetchError {
    /// HTTP 429. The caller waits one time and then stops the source for the run.
    RateLimited(String),
    Failed(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::RateLimited(m) | FetchError::Failed(m) => write!(f, "{m}"),
        }
    }
}

pub(crate) fn classify(error: String) -> FetchError {
    if error.starts_with("HTTP 429 ") {
        FetchError::RateLimited(error)
    } else {
        FetchError::Failed(error)
    }
}

pub trait PriceSource {
    fn name(&self) -> &'static str;
    /// Does the source give a dividend adjusted close?
    fn adjusts_dividends(&self) -> bool;
    /// Rows from `from` (YYYY-MM-DD, `None` for the full history) to `to`, oldest first.
    fn fetch(&self, ticker: &str, from: Option<&str>, to: &str) -> Result<Vec<Row>, FetchError>;
    /// Tickers that one request can carry (1 when the source has one ticker for each request).
    fn batch_size(&self) -> usize {
        1
    }
    /// Rows of many tickers. The default asks one ticker after the other; a source with a multi-symbol
    /// request (Alpaca) answers with fewer requests. A ticker with no rows is in the map with an empty list.
    fn fetch_many(&self, tickers: &[String], from: Option<&str>, to: &str) -> Result<std::collections::HashMap<String, Vec<Row>>, FetchError> {
        let mut out = std::collections::HashMap::new();
        for t in tickers {
            out.insert(t.clone(), self.fetch(t, from, to)?);
        }
        Ok(out)
    }
    /// What the source did besides its name (the feed and the adjustment). It goes into `prices.meta.json`.
    fn detail(&self) -> Option<String> {
        None
    }
}

/// A request function: URL in, bytes out. The tests give their own.
pub type Get = Box<dyn Fn(&str) -> Result<Vec<u8>, String>>;

// ---------------------------------------------------------------- key

/// The text with the key taken out.
pub fn scrub(text: &str, key: &str) -> String {
    super::fmp::scrub(text, key)
}

/// The value of `TIINGO_API_KEY` in the text of a `.env` file.
pub fn key_from_env_file(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        if let Some(value) = line.strip_prefix(TIINGO_KEY_ENV).and_then(|r| r.trim_start().strip_prefix('=')) {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// The key of the environment variable, else of the `.env` file in `repo`.
pub fn tiingo_key(repo: &Path) -> Option<String> {
    if let Ok(v) = std::env::var(TIINGO_KEY_ENV) {
        if !v.trim().is_empty() {
            return Some(v.trim().to_string());
        }
    }
    key_from_env_file(&std::fs::read_to_string(repo.join(".env")).ok()?)
}

// ---------------------------------------------------------------- symbols and dates

/// The text of the day `n` days after the date text (`n` can be negative).
pub fn add_days(date: &str, n: i64) -> Option<String> {
    Some(mapcheck::dates::format(parse_date(date)? + n))
}

fn us_date(text: &str) -> Option<String> {
    // MM/DD/YYYY
    let p: Vec<&str> = text.trim().split('/').collect();
    if p.len() != 3 || p[0].len() != 2 || p[1].len() != 2 || p[2].len() != 4 {
        return None;
    }
    let iso = format!("{}-{}-{}", p[2], p[0], p[1]);
    parse_date(&iso).map(|_| iso)
}

fn sort_dedupe(mut rows: Vec<Row>) -> Vec<Row> {
    rows.sort_by(|a, b| a.date.cmp(&b.date));
    rows.dedup_by(|b, a| a.date == b.date); // the first row of a date stays
    rows
}

pub(crate) fn sort_dedupe_rows(rows: Vec<Row>) -> Vec<Row> {
    sort_dedupe(rows)
}

pub(crate) fn finite_rows(rows: Vec<Row>) -> Vec<Row> {
    finite(rows)
}

fn finite(rows: Vec<Row>) -> Vec<Row> {
    rows.into_iter().filter(|r| [r.open, r.high, r.low, r.close].iter().all(|v| v.is_finite() && *v > 0.0)).collect()
}

// ---------------------------------------------------------------- Tiingo

/// Tiingo rows. The reply is a list of days. `close` is the price of that day (not split adjusted):
/// the split factors of the later days make it split adjusted. An error is an object with a `detail` text.
pub fn parse_tiingo(bytes: &[u8]) -> Result<Vec<Row>, String> {
    let value: Value = serde_json::from_slice(bytes).map_err(|e| format!("Tiingo reply is not JSON: {e}"))?;
    let items = match &value {
        Value::Array(a) => a,
        Value::Object(o) => {
            let detail = o.get("detail").and_then(Value::as_str).unwrap_or("not a list");
            let detail: String = detail.chars().take(200).collect();
            return Err(format!("Tiingo error: {detail}"));
        }
        _ => return Err("Tiingo reply is not a list".into()),
    };
    struct Day {
        date: String,
        o: f64,
        h: f64,
        l: f64,
        c: f64,
        v: i64,
        adj: Option<f64>,
        div: f64,
        split: f64,
    }
    let mut days = Vec::new();
    for item in items {
        let date = item["date"].as_str().map(|d| d.chars().take(10).collect::<String>()).filter(|d| parse_date(d).is_some());
        let (Some(date), Some(c)) = (date, item["close"].as_f64()) else { continue };
        let num = |k: &str| item[k].as_f64().unwrap_or(c);
        days.push(Day {
            date,
            o: num("open"),
            h: num("high"),
            l: num("low"),
            c,
            v: item["volume"].as_f64().unwrap_or(0.0) as i64,
            adj: item["adjClose"].as_f64(),
            div: item["divCash"].as_f64().unwrap_or(0.0),
            split: item["splitFactor"].as_f64().filter(|s| *s > 0.0).unwrap_or(1.0),
        });
    }
    days.sort_by(|a, b| a.date.cmp(&b.date));
    // A day before a split has a price in the old share count: divide it by the product of the later split factors.
    let mut later = 1.0;
    let mut rows = Vec::with_capacity(days.len());
    for d in days.iter().rev() {
        rows.push(Row {
            date: d.date.clone(),
            open: round6(d.o / later),
            high: round6(d.h / later),
            low: round6(d.l / later),
            close: round6(d.c / later),
            volume: d.v,
            adj_close: d.adj.map(round6),
            // `divCash` is in the share count of its own day (checked 2026-11-20 with NVDA: the check of the adjustment gave an error
            // of 25% of the price when the dividend was not divided). Like the prices, it is divided by the later split factors.
            dividend: (d.div != 0.0).then_some(round6(d.div / later)),
            split: (d.split != 1.0).then_some(d.split),
        });
        later *= d.split;
    }
    rows.reverse();
    Ok(finite(sort_dedupe(rows)))
}

pub struct Tiingo {
    key: String,
    get: Get,
}

impl Tiingo {
    pub fn new(key: &str, get: Get) -> Tiingo {
        Tiingo { key: key.to_string(), get }
    }

    /// The real server: the key goes in the header `Authorization: Token <key>`, never in the URL.
    pub fn live(key: &str) -> Tiingo {
        let header = format!("Token {key}");
        let key_copy = key.to_string();
        Tiingo::new(key, Box::new(move |url| download_with_headers(url, &[("Authorization", &header), ("Content-Type", "application/json")]).map_err(|e| scrub(&e, &key_copy))))
    }

    pub fn url(ticker: &str, from: Option<&str>, to: &str) -> String {
        format!("{TIINGO_BASE}/{}/prices?startDate={}&endDate={to}", ticker.to_lowercase(), from.unwrap_or(FULL_FROM))
    }
}

impl PriceSource for Tiingo {
    fn name(&self) -> &'static str {
        "tiingo"
    }
    fn adjusts_dividends(&self) -> bool {
        true
    }
    fn fetch(&self, ticker: &str, from: Option<&str>, to: &str) -> Result<Vec<Row>, FetchError> {
        let bytes = (self.get)(&Tiingo::url(ticker, from, to)).map_err(|e| classify(scrub(&e, &self.key)))?;
        parse_tiingo(&bytes).map_err(|e| FetchError::Failed(scrub(&e, &self.key)))
    }
}

// ---------------------------------------------------------------- Yahoo

/// Yahoo rows from a chart reply (with `events=div|split`).
pub fn parse_yahoo(bytes: &[u8]) -> Result<Vec<Row>, String> {
    let raw = parse_raw(bytes)?;
    let root: Value = serde_json::from_slice(bytes).map_err(|e| format!("Yahoo response is not JSON: {e}"))?;
    let result = &root["chart"]["result"][0];
    let n = raw.stamps.len();
    let adj: Option<Vec<Option<f64>>> = result["indicators"]["adjclose"][0]["adjclose"].as_array().map(|a| a.iter().map(Value::as_f64).collect());
    if let Some(a) = &adj {
        if a.len() != n {
            return Err(format!("Yahoo adjclose array has {} items; the time stamp array has {n}", a.len()));
        }
    }
    let new_york = raw.time_zone == "America/New_York";
    let day_of = |utc: i64| iso((utc + if new_york { ny_offset(utc) } else { raw.offset }).div_euclid(86_400) as i32);
    let mut rows: Vec<Row> = Vec::new();
    for i in 0..n {
        let Some(close) = raw.close[i].filter(|c| c.is_finite() && *c > 0.0) else { continue };
        let pick = |x: Option<f64>| x.filter(|v| v.is_finite() && *v > 0.0).unwrap_or(close);
        rows.push(Row {
            date: day_of(raw.stamps[i]),
            open: round6(pick(raw.open[i])),
            high: round6(pick(raw.high[i])),
            low: round6(pick(raw.low[i])),
            close: round6(close),
            volume: raw.volume[i].filter(|v| v.is_finite()).unwrap_or(0.0) as i64,
            adj_close: adj.as_ref().and_then(|a| a[i]).filter(|v| v.is_finite()).map(round6),
            dividend: None,
            split: None,
        });
    }
    // Yahoo adds a live row of today besides the daily row: the last row stays.
    if rows.len() > 1 && rows[rows.len() - 1].date == rows[rows.len() - 2].date {
        rows.remove(rows.len() - 2);
    }
    if let Some(map) = result["events"]["dividends"].as_object() {
        for ev in map.values() {
            if let (Some(ts), Some(amount)) = (ev["date"].as_i64(), ev["amount"].as_f64()) {
                let d = day_of(ts);
                if let Some(r) = rows.iter_mut().find(|r| r.date == d) {
                    r.dividend = Some(amount);
                }
            }
        }
    }
    if let Some(map) = result["events"]["splits"].as_object() {
        for ev in map.values() {
            if let (Some(ts), Some(num), Some(den)) = (ev["date"].as_i64(), ev["numerator"].as_f64(), ev["denominator"].as_f64()) {
                let d = day_of(ts);
                if let (Some(r), true) = (rows.iter_mut().find(|r| r.date == d), den > 0.0) {
                    r.split = Some(num / den);
                }
            }
        }
    }
    Ok(sort_dedupe(rows))
}

pub struct Yahoo {
    get: Get,
}

impl Yahoo {
    pub fn new(get: Get) -> Yahoo {
        Yahoo { get }
    }
    pub fn live() -> Yahoo {
        Yahoo::new(Box::new(download))
    }
    /// `period1` and `period2` give daily rows for the whole history (`range=max` can give a coarser interval).
    pub fn url(ticker: &str, from: Option<&str>, to: &str) -> String {
        let p1 = from.and_then(parse_date).map_or(0, |d| d * 86_400);
        let p2 = (parse_date(to).unwrap_or_else(mapcheck::dates::today) + 2) * 86_400;
        format!("{YAHOO_BASE}/{ticker}?period1={p1}&period2={p2}&interval=1d&includeAdjustedClose=true&events=div%7Csplit")
    }
}

impl PriceSource for Yahoo {
    fn name(&self) -> &'static str {
        "yahoo"
    }
    fn adjusts_dividends(&self) -> bool {
        true
    }
    fn fetch(&self, ticker: &str, from: Option<&str>, to: &str) -> Result<Vec<Row>, FetchError> {
        let bytes = (self.get)(&Yahoo::url(&ticker.to_uppercase(), from, to)).map_err(classify)?;
        parse_yahoo(&bytes).map_err(FetchError::Failed)
    }
}

// ---------------------------------------------------------------- Nasdaq

fn money(text: &str) -> Option<f64> {
    let t: String = text.trim().chars().filter(|c| *c != '$' && *c != ',').collect();
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Nasdaq rows: the values are text (`"$508.29"`, `"22,050,090"`, date `MM/DD/YYYY`), newest first.
/// An empty window gives `rows: null` (an empty list here). An error has a `status.bCodeMessage`.
pub fn parse_nasdaq(bytes: &[u8]) -> Result<Vec<Row>, String> {
    let root: Value = serde_json::from_slice(bytes).map_err(|e| format!("Nasdaq reply is not JSON: {e}"))?;
    if let Some(msg) = root["status"]["bCodeMessage"].as_array().and_then(|a| a.first()).map(|m| m["errorMessage"].as_str().unwrap_or("error").to_string()) {
        return Err(format!("Nasdaq error: {}", msg.chars().take(200).collect::<String>()));
    }
    if let Some(code) = root["status"]["rCode"].as_i64().filter(|c| *c != 200) {
        return Err(format!("Nasdaq status {code}"));
    }
    if root["data"].is_null() {
        return Err("Nasdaq reply has no data".into());
    }
    // An unknown symbol in the class `stocks` gives `"tradesTable": null` with HTTP 200 (BF.B on 2026-11-19).
    let Some(table) = root["data"]["tradesTable"].as_object() else { return Ok(Vec::new()) };
    let Some(rows) = table.get("rows").and_then(Value::as_array) else { return Ok(Vec::new()) };
    let mut out = Vec::new();
    for r in rows {
        let text = |k: &str| r[k].as_str().and_then(money);
        let (Some(date), Some(close)) = (r["date"].as_str().and_then(us_date), text("close")) else { continue };
        out.push(Row {
            date,
            open: round6(text("open").unwrap_or(close)),
            high: round6(text("high").unwrap_or(close)),
            low: round6(text("low").unwrap_or(close)),
            close: round6(close),
            volume: text("volume").unwrap_or(0.0) as i64,
            adj_close: None,
            dividend: None,
            split: None,
        });
    }
    Ok(finite(sort_dedupe(out)))
}

/// ETFs need the asset class `etf`. These are known; any other ticker with an empty reply is asked again as an ETF.
const KNOWN_ETFS: [&str; 12] = ["SPY", "QQQ", "IWM", "DIA", "VTI", "VOO", "IVV", "GLD", "TLT", "XLF", "XLK", "XLE"];

pub struct Nasdaq {
    get: Get,
}

impl Nasdaq {
    pub fn new(get: Get) -> Nasdaq {
        Nasdaq { get }
    }
    pub fn live() -> Nasdaq {
        Nasdaq::new(Box::new(download))
    }
    /// The window must end today: a window that ends in the past gave an empty reply in a test (2026-11-19).
    pub fn url(symbol: &str, class: &str, from: &str, to: &str) -> String {
        format!("{NASDAQ_BASE}/{symbol}/historical?assetclass={class}&fromdate={from}&limit=10000&todate={to}")
    }
    /// Nasdaq writes a share class with a dot, as in `BRK.B`.
    pub fn symbol(ticker: &str) -> String {
        ticker.to_uppercase().replace('-', ".")
    }
}

impl PriceSource for Nasdaq {
    fn name(&self) -> &'static str {
        "nasdaq"
    }
    fn adjusts_dividends(&self) -> bool {
        false
    }
    fn fetch(&self, ticker: &str, from: Option<&str>, to: &str) -> Result<Vec<Row>, FetchError> {
        let t = ticker.to_uppercase();
        let symbol = Nasdaq::symbol(&t);
        // About 10 years are the depth of the source: a longer window gives the same rows.
        let from = from.unwrap_or(FULL_FROM);
        let first = if KNOWN_ETFS.contains(&t.as_str()) { "etf" } else { "stocks" };
        let second = if first == "etf" { "stocks" } else { "etf" };
        let mut rows = Vec::new();
        for class in [first, second] {
            let bytes = (self.get)(&Nasdaq::url(&symbol, class, from, to)).map_err(classify)?;
            rows = parse_nasdaq(&bytes).map_err(FetchError::Failed)?;
            if !rows.is_empty() {
                break;
            }
        }
        Ok(rows)
    }
}

/// The sources in the order of use: Tiingo (when its key exists), Yahoo, Nasdaq, Alpaca (when its keys exist; last, see below). `only` forces one.
pub fn sources(repo: &Path, only: Option<&str>) -> Result<Vec<Box<dyn PriceSource>>, String> {
    let mut all: Vec<Box<dyn PriceSource>> = Vec::new();
    match tiingo_key(repo) {
        Some(key) => all.push(Box::new(Tiingo::live(&key))),
        None if only == Some("tiingo") => return Err(format!("{TIINGO_KEY_ENV} is not set in the environment or in .env")),
        None => {}
    }
    all.push(Box::new(Yahoo::live()));
    all.push(Box::new(Nasdaq::live()));
    // Alpaca is last: its free feed is IEX (the close is the IEX close, not the official close; the volume is the IEX volume;
    // the history starts 2020-07-27). It is the check source and the fallback, not the first choice (#241).
    match super::alpaca::Keys::read(repo) {
        Some(keys) => all.push(Box::new(super::alpaca::Alpaca::live(keys))),
        None if only == Some("alpaca") => return Err(format!("{} and {} are not both set in the environment or in .env", super::alpaca::KEY_ID_ENV, super::alpaca::SECRET_ENV)),
        None => {}
    }
    match only {
        None => Ok(all),
        Some(name) => {
            let chosen: Vec<_> = all.into_iter().filter(|s| s.name() == name).collect();
            if chosen.is_empty() {
                Err(format!("unknown source '{name}'; the sources are tiingo, alpaca, yahoo and nasdaq"))
            } else {
                Ok(chosen)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NASDAQ: &str = r#"{"data":{"symbol":"NVDA","totalRecords":3,"tradesTable":{"asOf":null,"headers":{"date":"Date"},"rows":[
        {"date":"06/11/2024","close":"$120.91","volume":"222,551,200","open":"$121.77","high":"$122.87","low":"$118.74"},
        {"date":"06/10/2024","close":"$121.79","volume":"314,162,700","open":"$120.37","high":"$123.10","low":"$117.01"},
        {"date":"06/07/2024","close":"$120.888","volume":"N/A","open":"$119.77","high":"$121.6917","low":"$118.022"}]}},
        "message":null,"status":{"rCode":200,"bCodeMessage":null,"developerMessage":null}}"#;

    #[test]
    fn nasdaq_text_formats() {
        let rows = parse_nasdaq(NASDAQ.as_bytes()).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-06-07", "oldest first");
        assert_eq!(rows[0].close, 120.888);
        assert_eq!(rows[0].volume, 0, "N/A volume");
        assert_eq!(rows[2].volume, 222_551_200);
        assert_eq!(rows[2].open, 121.77);
        assert!(rows.iter().all(|r| r.adj_close.is_none()));
    }

    #[test]
    fn nasdaq_empty_and_error() {
        let empty = r#"{"data":{"symbol":"X","totalRecords":0,"tradesTable":{"asOf":null,"headers":null,"rows":null}},"message":null,"status":{"rCode":200,"bCodeMessage":null}}"#;
        assert!(parse_nasdaq(empty.as_bytes()).unwrap().is_empty());
        let err = r#"{"data":null,"message":null,"status":{"rCode":400,"bCodeMessage":[{"code":1001,"errorMessage":"Symbol not exists"}],"developerMessage":null}}"#;
        assert!(parse_nasdaq(err.as_bytes()).unwrap_err().contains("Symbol not exists"));
        assert!(parse_nasdaq(b"<html>blocked</html>").unwrap_err().contains("not JSON"));
        let no_data = r#"{"data":null,"status":{"rCode":200}}"#;
        assert!(parse_nasdaq(no_data.as_bytes()).is_err());
        let unknown = r#"{"data":{"symbol":null,"totalRecords":0,"tradesTable":null},"message":null,"status":{"rCode":200,"bCodeMessage":null}}"#;
        assert!(parse_nasdaq(unknown.as_bytes()).unwrap().is_empty());
    }

    #[test]
    fn nasdaq_missing_day_and_bad_rows_are_dropped() {
        let text = r#"{"data":{"tradesTable":{"rows":[
            {"date":"01/05/2024","close":"$10.00","volume":"1","open":"$10","high":"$11","low":"$9"},
            {"date":"13/45/2024","close":"$10.00"},
            {"date":"01/02/2024","close":"N/A"},
            {"date":"01/02/2024","close":"$9.50","volume":"2","open":"$9","high":"$10","low":"$9"}]}},"status":{"rCode":200}}"#;
        let rows = parse_nasdaq(text.as_bytes()).unwrap();
        assert_eq!(rows.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(), ["2024-01-02", "2024-01-05"], "a missing day is a gap, not a row");
    }

    #[test]
    fn nasdaq_symbols_and_etf_retry() {
        assert_eq!(Nasdaq::symbol("brk-b"), "BRK.B");
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let seen = calls.clone();
        let src = Nasdaq::new(Box::new(move |url| {
            seen.borrow_mut().push(url.to_string());
            Ok(if url.contains("assetclass=etf") { NASDAQ.as_bytes().to_vec() } else { br#"{"data":{"tradesTable":{"rows":null}},"status":{"rCode":200}}"#.to_vec() })
        }));
        let rows = src.fetch("XYZ", Some("2024-06-01"), "2024-06-30").unwrap();
        assert_eq!(rows.len(), 3);
        let calls = calls.borrow();
        assert!(calls[0].contains("assetclass=stocks") && calls[1].contains("assetclass=etf"));
        assert!(calls[0].contains("fromdate=2024-06-01") && calls[0].contains("todate=2024-06-30"));
        // A known ETF goes to the ETF class first.
        let seen = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
        let s2 = seen.clone();
        let spy = Nasdaq::new(Box::new(move |url| {
            *s2.borrow_mut() = url.to_string();
            Ok(NASDAQ.as_bytes().to_vec())
        }));
        spy.fetch("SPY", None, "2024-06-30").unwrap();
        assert!(seen.borrow().contains("/SPY/historical?assetclass=etf"));
    }

    /// Fields of the documentation: date, close, high, low, open, volume, adjClose, adjHigh, adjLow, adjOpen, adjVolume, divCash, splitFactor.
    const TIINGO: &str = r#"[
      {"date":"2024-06-07T00:00:00.000Z","close":1208.88,"high":1217.0,"low":1180.2,"open":1197.7,"volume":41238000,"adjClose":1207.9,"adjHigh":1216.0,"adjLow":1179.2,"adjOpen":1196.7,"adjVolume":41238000,"divCash":0.0,"splitFactor":1.0},
      {"date":"2024-06-10T00:00:00.000Z","close":121.79,"high":123.1,"low":117.01,"open":120.37,"volume":314162700,"adjClose":121.7,"adjHigh":123.0,"adjLow":116.9,"adjOpen":120.3,"adjVolume":314162700,"divCash":0.01,"splitFactor":10.0},
      {"date":"2024-06-11T00:00:00.000Z","close":120.91,"high":122.87,"low":118.74,"open":121.77,"volume":222551200,"adjClose":120.91,"adjHigh":122.87,"adjLow":118.74,"adjOpen":121.77,"adjVolume":222551200,"divCash":0.0,"splitFactor":1.0}]"#;

    #[test]
    fn tiingo_fixture_split_adjusts_old_days() {
        let rows = parse_tiingo(TIINGO.as_bytes()).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-06-07");
        assert!((rows[0].close - 120.888).abs() < 1e-9, "1208.88 / 10: {}", rows[0].close);
        assert_eq!(rows[1].close, 121.79, "the split day is in the new share count");
        assert_eq!(rows[1].split, Some(10.0));
        assert_eq!(rows[1].dividend, Some(0.01));
        assert_eq!(rows[0].split, None);
        assert_eq!(rows[0].adj_close, Some(1207.9));
    }

    #[test]
    fn tiingo_dividend_before_a_split_is_split_adjusted_and_passes_the_event_check() {
        // Day 2 pays a dividend of 10 in the old share count (close 990); day 3 is a 10:1 split day (close 99). Per new share the
        // dividend is 1 on a close of 100: 1%. The adjusted closes are in the current share count.
        let text = r#"[
          {"date":"2024-06-03T00:00:00.000Z","close":1000.0,"high":1000.0,"low":1000.0,"open":1000.0,"volume":1,"adjClose":99.0,"divCash":0.0,"splitFactor":1.0},
          {"date":"2024-06-04T00:00:00.000Z","close":990.0,"high":990.0,"low":990.0,"open":990.0,"volume":1,"adjClose":99.0,"divCash":10.0,"splitFactor":1.0},
          {"date":"2024-06-05T00:00:00.000Z","close":99.0,"high":99.0,"low":99.0,"open":99.0,"volume":1,"adjClose":99.0,"divCash":0.0,"splitFactor":10.0}]"#;
        let rows = parse_tiingo(text.as_bytes()).unwrap();
        assert_eq!(rows[1].dividend, Some(1.0), "10 / 10");
        let e = super::super::price_check::check_events(&rows);
        assert_eq!(e.dividends_checked, 1);
        assert!(e.dividend_max_error < 1e-9, "{}", e.dividend_max_error);
    }

    #[test]
    fn tiingo_error_object_and_empty() {
        assert!(parse_tiingo(br#"{"detail":"Error: Ticker 'ZZZ' not found"}"#).unwrap_err().contains("not found"));
        assert!(parse_tiingo(b"[]").unwrap().is_empty());
        assert!(parse_tiingo(b"nope").is_err());
    }

    #[test]
    fn tiingo_key_is_in_a_header_and_scrubbed_from_errors() {
        let url = Tiingo::url("BRK-B", Some("2024-01-01"), "2024-02-01");
        assert_eq!(url, "https://api.tiingo.com/tiingo/daily/brk-b/prices?startDate=2024-01-01&endDate=2024-02-01");
        let src = Tiingo::new("SECRETKEY", Box::new(|_| Err("HTTP 500 for x; Authorization: Token SECRETKEY".into())));
        let e = src.fetch("IBM", None, "2024-02-01").unwrap_err();
        assert!(!e.to_string().contains("SECRETKEY") && e.to_string().contains("***"), "{e}");
        let bad = Tiingo::new("SECRETKEY", Box::new(|_| Ok(br#"{"detail":"bad token SECRETKEY"}"#.to_vec())));
        assert!(!bad.fetch("IBM", None, "2024-02-01").unwrap_err().to_string().contains("SECRETKEY"));
        let limited = Tiingo::new("K", Box::new(|_| Err("HTTP 429 for https://api.tiingo.com/x".into())));
        assert!(matches!(limited.fetch("IBM", None, "2024-02-01"), Err(FetchError::RateLimited(_))));
    }

    #[test]
    fn env_file_key() {
        assert_eq!(key_from_env_file("A=1\nexport TIINGO_API_KEY=\"abc\"\n").as_deref(), Some("abc"));
        assert_eq!(key_from_env_file("FMP_API_KEY=x\n"), None);
    }

    fn yahoo_fixture() -> String {
        // 2024-06-07 and 2024-06-10 at 13:30 UTC (09:30 New York, UTC-4), a dividend on 06-10, a 10-for-1 split on 06-10.
        let (t1, t2, t3) = (1_717_767_000i64, 1_718_026_200i64, 1_718_112_600i64);
        format!(
            r#"{{"chart":{{"error":null,"result":[{{"meta":{{"symbol":"NVDA","gmtoffset":-14400,"exchangeTimezoneName":"America/New_York","shortName":"N"}},
            "timestamp":[{t1},{t2},{t3}],
            "events":{{"dividends":{{"{t2}":{{"amount":0.01,"date":{t2}}}}},"splits":{{"{t2}":{{"date":{t2},"numerator":10,"denominator":1,"splitRatio":"10:1"}}}}}},
            "indicators":{{"quote":[{{"open":[119.77,120.37,121.77],"high":[121.69,123.1,122.87],"low":[118.02,117.01,118.74],"close":[120.888,121.79,null],"volume":[412385800,314162700,null]}}],
            "adjclose":[{{"adjclose":[120.8,121.78,null]}}]}}}}]}}}}"#
        )
    }

    #[test]
    fn yahoo_fixture_rows_dividends_splits() {
        let rows = parse_yahoo(yahoo_fixture().as_bytes()).unwrap();
        assert_eq!(rows.len(), 2, "a row with no close is dropped");
        assert_eq!(rows[0].date, "2024-06-07");
        assert_eq!(rows[1].date, "2024-06-10");
        assert_eq!(rows[0].adj_close, Some(120.8));
        assert_eq!(rows[1].dividend, Some(0.01));
        assert_eq!(rows[1].split, Some(10.0));
        assert!(Yahoo::url("MSFT", None, "2026-11-19").contains("period1=0&period2="));
        assert!(Yahoo::url("MSFT", Some("1970-01-02"), "2026-11-19").contains("period1=86400"));
        assert!(parse_yahoo(br#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found"}}}"#).unwrap_err().contains("Yahoo error"));
    }

    #[test]
    fn yahoo_429_is_rate_limited() {
        let y = Yahoo::new(Box::new(|u| Err(format!("HTTP 429 for {u}"))));
        assert!(matches!(y.fetch("IBM", None, "2026-11-19"), Err(FetchError::RateLimited(_))));
        let y = Yahoo::new(Box::new(|_| Err("connection reset".into())));
        assert!(matches!(y.fetch("IBM", None, "2026-11-19"), Err(FetchError::Failed(_))));
    }

    #[test]
    fn source_choice() {
        let dir = std::env::temp_dir().join(format!("fa_src_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let names = |only: Option<&str>| sources(&dir, only).map(|v| v.iter().map(|s| s.name()).collect::<Vec<_>>());
        assert_eq!(names(Some("nasdaq")).unwrap(), ["nasdaq"]);
        assert!(names(Some("stooq")).is_err());
        std::fs::write(dir.join(".env"), "TIINGO_API_KEY=k\n").unwrap();
        if std::env::var(TIINGO_KEY_ENV).is_err() {
            // With Alpaca keys in the environment of the person, `alpaca` is in the list too.
            let all = names(None).unwrap();
            assert_eq!(all.iter().copied().filter(|n| *n != "alpaca").collect::<Vec<_>>(), ["tiingo", "yahoo", "nasdaq"]);
            assert!(all.iter().position(|n| *n == "alpaca").is_none_or(|i| i == all.len() - 1), "Alpaca (IEX close, short history) is last: {all:?}");
        }
        assert!(sources(&dir, Some("alpaca")).is_err() || std::env::var(super::super::alpaca::KEY_ID_ENV).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
