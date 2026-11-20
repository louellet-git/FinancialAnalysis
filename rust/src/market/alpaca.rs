//! Alpaca Market Data API: daily and intraday stock bars, option contracts, option bars and option
//! chain snapshots. A published API with a free key (the Basic plan; a paper account is enough, see #230).
//!
//! The code follows the official documentation read on 2026-11-19 (URLs in each function). It was NOT run
//! against the real server: there was no key. Each parser is a function of the reply text, and each
//! test uses a text built from the documented reply.
//!
//! - Base addresses: `https://data.alpaca.markets` (data) and `https://paper-api.alpaca.markets` (the contract
//!   list of the trading API; `APCA_API_BASE_URL` changes it).
//! - Authentication: the headers `APCA-API-KEY-ID` and `APCA-API-SECRET-KEY`, never the URL. The keys come from
//!   the environment (`APCA_API_KEY_ID`, `APCA_API_SECRET_KEY`), else from `<repo>/.env`. Each error text has
//!   the key values taken out.
//! - Free plan: the feed `iex` (stocks) and `indicative` (options), 200 requests each minute. This client waits
//!   0.4 second between two requests (150 each minute).
//! - Paging: a reply has `next_page_token`; the next request gives it as `page_token`. The `limit` counts data
//!   points of all symbols together, and the points are sorted by symbol and then by time.

use super::keys::{read_key, scrub_all, urlenc};
use super::price_source::{finite_rows, sort_dedupe_rows, FetchError, Get, PriceSource, Row};
use super::round6;
use crate::trading::calendar::{iso, ny_offset, parse_iso};
use serde_json::Value;
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub const KEY_ID_ENV: &str = "APCA_API_KEY_ID";
pub const SECRET_ENV: &str = "APCA_API_SECRET_KEY";
pub const TRADING_BASE_ENV: &str = "APCA_API_BASE_URL";
pub const FEED_ENV: &str = "APCA_DATA_FEED";
pub const DATA_BASE: &str = "https://data.alpaca.markets";
pub const TRADING_BASE: &str = "https://paper-api.alpaca.markets";
/// Seconds between two requests (the free plan allows 200 each minute).
pub const PAUSE_SECONDS: f64 = 0.4;
/// The stock history of the free plan starts in 2016 (`docs/about-market-data-api`).
pub const FIRST_DAY: &str = "2016-01-01";
/// The largest page of bars (`limit`, stock bars and option bars).
pub const BARS_LIMIT: usize = 10_000;
/// Symbols in one request of stock bars. The documentation names no limit for stock bars; the option
/// endpoints name 100. This is the cautious value.
pub const SYMBOLS_PER_REQUEST: usize = 100;

#[derive(Clone)]
pub struct Keys {
    pub id: String,
    pub secret: String,
}

impl Keys {
    /// Both keys, or `None` when one is missing.
    pub fn read(repo: &Path) -> Option<Keys> {
        Some(Keys { id: read_key(repo, KEY_ID_ENV)?, secret: read_key(repo, SECRET_ENV)? })
    }
    pub fn scrub(&self, text: &str) -> String {
        scrub_all(text, &[&self.id, &self.secret])
    }
}

/// The feed of the stock data: `APCA_DATA_FEED`, else `iex` (the only feed without a paid plan).
pub fn feed_from_env() -> String {
    std::env::var(FEED_ENV).ok().map(|f| f.trim().to_lowercase()).filter(|f| matches!(f.as_str(), "iex" | "sip" | "otc")).unwrap_or_else(|| "iex".into())
}

pub fn trading_base() -> String {
    std::env::var(TRADING_BASE_ENV).ok().map(|b| b.trim().trim_end_matches('/').to_string()).filter(|b| b.starts_with("https://")).unwrap_or_else(|| TRADING_BASE.into())
}

/// A client that sends the keys in headers, counts its requests, and takes the keys out of each error.
pub struct Client {
    keys: Keys,
    get: Get,
    count: Cell<usize>,
}

impl Client {
    pub fn new(keys: Keys, get: Get) -> Client {
        Client { keys, get, count: Cell::new(0) }
    }

    /// The real server.
    pub fn live(keys: Keys) -> Client {
        let (id, secret) = (keys.id.clone(), keys.secret.clone());
        let scrub_keys = keys.clone();
        let get: Get = Box::new(move |url| {
            let headers = [("APCA-API-KEY-ID", id.as_str()), ("APCA-API-SECRET-KEY", secret.as_str())];
            crate::hostnet::get(url, &headers, PAUSE_SECONDS).map_err(|e| scrub_keys.scrub(&e))
        });
        Client::new(keys, get)
    }

    /// The number of requests so far.
    pub fn requests(&self) -> usize {
        self.count.get()
    }

    fn call(&self, url: &str) -> Result<Value, FetchError> {
        self.count.set(self.count.get() + 1);
        let bytes = (self.get)(url).map_err(|e| super::price_source::classify(self.keys.scrub(&e)))?;
        serde_json::from_slice(&bytes).map_err(|e| FetchError::Failed(self.keys.scrub(&format!("Alpaca reply is not JSON: {e}"))))
    }

    /// Request `base` and its next pages. `each` reads one page. `base` has a query already.
    pub fn paged(&self, base: &str, mut each: impl FnMut(&Value) -> Result<(), String>) -> Result<(), FetchError> {
        let mut token: Option<String> = None;
        for _ in 0..1_000_000 {
            let url = match &token {
                Some(t) => format!("{base}&page_token={}", urlenc(t)),
                None => base.to_string(),
            };
            let page = self.call(&url)?;
            each(&page).map_err(|e| FetchError::Failed(self.keys.scrub(&e)))?;
            token = page["next_page_token"].as_str().filter(|t| !t.is_empty()).map(String::from);
            if token.is_none() {
                return Ok(());
            }
        }
        Err(FetchError::Failed("Alpaca paging did not end".into()))
    }
}

// ---------------------------------------------------------------- time

/// UTC seconds of an RFC 3339 text (`2024-01-18T05:00:00Z`, with a fraction or an offset `-05:00`).
pub fn parse_rfc3339(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() < 20 || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let day = parse_iso(text)? as i64;
    let n = |a: usize, z: usize| text.get(a..z)?.parse::<i64>().ok();
    let secs = day * 86_400 + n(11, 13)? * 3600 + n(14, 16)? * 60 + n(17, 19)?;
    let mut rest = &text[19..];
    if let Some(r) = rest.strip_prefix('.') {
        rest = r.trim_start_matches(|c: char| c.is_ascii_digit());
    }
    match rest {
        "Z" | "z" => Some(secs),
        _ if rest.len() == 6 && (rest.starts_with('+') || rest.starts_with('-')) && rest.as_bytes()[3] == b':' => {
            let off = rest[1..3].parse::<i64>().ok()? * 3600 + rest[4..6].parse::<i64>().ok()? * 60;
            Some(if rest.starts_with('+') { secs - off } else { secs + off })
        }
        _ => None,
    }
}

/// The New York date (`YYYY-MM-DD`) and the second of that day for a UTC second.
pub fn ny_day(utc: i64) -> (String, i64) {
    let local = utc + ny_offset(utc);
    (iso(local.div_euclid(86_400) as i32), local.rem_euclid(86_400))
}

pub fn now_utc() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

// ---------------------------------------------------------------- bars

#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub utc: i64,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
}

/// The bars of one page, by symbol. Documentation: `https://docs.alpaca.markets/reference/stockbars` and
/// `.../optionbars`: `{"bars": {"AAPL": [{"t","o","h","l","c","v","n","vw"}]}, "next_page_token": ...}`.
/// `"bars": null` is an empty page. An error reply has a `message`.
pub fn parse_bars(page: &Value) -> Result<BTreeMap<String, Vec<Bar>>, String> {
    let bars = match page.get("bars") {
        Some(Value::Object(m)) => m,
        Some(Value::Null) => return Ok(BTreeMap::new()),
        _ => {
            let msg = page["message"].as_str().map(|m| m.chars().take(200).collect::<String>());
            return Err(match msg {
                Some(m) => format!("Alpaca error: {m}"),
                None => "Alpaca reply has no bars".into(),
            });
        }
    };
    let mut out = BTreeMap::new();
    for (symbol, list) in bars {
        let mut v = Vec::new();
        for b in list.as_array().map(Vec::as_slice).unwrap_or(&[]) {
            let (Some(utc), Some(o), Some(h), Some(l), Some(c)) = (b["t"].as_str().and_then(parse_rfc3339), b["o"].as_f64(), b["h"].as_f64(), b["l"].as_f64(), b["c"].as_f64()) else { continue };
            v.push(Bar { utc, o, h, l, c, v: b["v"].as_f64().unwrap_or(0.0) });
        }
        out.insert(symbol.clone(), v);
    }
    Ok(out)
}

/// The day of the end of a request: no `end` when the window reaches today (the free plan cannot ask
/// for the latest 15 minutes, and the default of the API is 15 minutes ago), else the end of that day.
fn end_param(to: &str, today: &str) -> String {
    if to >= today {
        String::new()
    } else {
        format!("&end={to}T23:59:59Z")
    }
}

/// A request of stock bars. `timeframe` is `1Day`, `5Min`, ... `adjustment` is `raw`, `split`, `dividend` or `all`.
/// `today` is the date of today (a window that reaches it has no `end`).
pub struct StockQuery<'a> {
    pub symbols: &'a [String],
    pub timeframe: &'a str,
    pub start: &'a str,
    pub to: &'a str,
    pub today: &'a str,
    pub adjustment: &'a str,
    pub feed: &'a str,
}

impl Client {
    /// Bars of stocks. Documentation: `GET /v2/stocks/bars` (https://docs.alpaca.markets/reference/stockbars).
    pub fn stock_bars(&self, q: &StockQuery) -> Result<BTreeMap<String, Vec<Bar>>, FetchError> {
        let base = format!(
            "{DATA_BASE}/v2/stocks/bars?symbols={}&timeframe={}&start={}{}&limit={BARS_LIMIT}&adjustment={}&feed={}&sort=asc",
            q.symbols.join(","),
            q.timeframe,
            q.start,
            end_param(q.to, q.today),
            q.adjustment,
            q.feed
        );
        let mut all: BTreeMap<String, Vec<Bar>> = BTreeMap::new();
        self.paged(&base, |page| {
            for (s, mut v) in parse_bars(page)? {
                all.entry(s).or_default().append(&mut v);
            }
            Ok(())
        })?;
        Ok(all)
    }

    /// Daily bars of option contracts, up to 100 symbols in one request.
    /// Documentation: `GET /v1beta1/options/bars` (https://docs.alpaca.markets/reference/optionbars).
    pub fn option_bars(&self, contracts: &[String], from: &str, to: &str) -> Result<BTreeMap<String, Vec<Bar>>, FetchError> {
        let base = format!("{DATA_BASE}/v1beta1/options/bars?symbols={}&timeframe=1Day&start={from}&end={to}T23:59:59Z&limit={BARS_LIMIT}&sort=asc", contracts.join(","));
        let mut all: BTreeMap<String, Vec<Bar>> = BTreeMap::new();
        self.paged(&base, |page| {
            for (s, mut v) in parse_bars(page)? {
                all.entry(s).or_default().append(&mut v);
            }
            Ok(())
        })?;
        Ok(all)
    }
}

/// Alpaca writes a share class with a dot (`BRK.B`).
pub fn api_symbol(ticker: &str) -> String {
    ticker.to_uppercase().replace('-', ".")
}

/// The daily rows of the bars of one stock. A bar is at midnight New York time (the API gives 04:00 or
/// 05:00 UTC). The row of today stays out until 16:20 New York time (the session ended, and the free plan
/// is 15 minutes late).
pub fn daily_rows(bars: &[Bar], now_utc: i64) -> Vec<Row> {
    let (today, now_secs) = ny_day(now_utc);
    let rows = bars
        .iter()
        .filter_map(|b| {
            let (date, _) = ny_day(b.utc);
            if date == today && now_secs < 16 * 3600 + 20 * 60 {
                return None;
            }
            Some(Row { date, open: round6(b.o), high: round6(b.h), low: round6(b.l), close: round6(b.c), volume: b.v as i64, adj_close: None, dividend: None, split: None })
        })
        .collect();
    finite_rows(sort_dedupe_rows(rows))
}

/// Cache rows of intraday bars: the regular session only (09:30 up to, not including, 16:00 New York time),
/// no bar that has not ended. The line is the line of `trading::fetch`.
pub fn intraday_rows(bars: &[Bar], minutes: i64, now_utc: i64) -> Vec<crate::trading::fetch::Row> {
    let mut out = Vec::new();
    for b in bars {
        let (_, secs) = ny_day(b.utc);
        if !(9 * 3600 + 30 * 60..16 * 3600).contains(&secs) || secs % 60 != 0 || !(b.o > 0.0 && b.h > 0.0 && b.l > 0.0 && b.c > 0.0) {
            continue;
        }
        let end = b.utc + (minutes * 60).min(16 * 3600 - secs);
        if end > now_utc {
            continue;
        }
        out.push(crate::trading::fetch::Row { utc: b.utc, line: crate::trading::fetch::cache_line(b.utc, b.o, b.h, b.l, b.c, b.v) });
    }
    out
}

// ---------------------------------------------------------------- price source

/// The price source `alpaca`: daily bars of the stock bars endpoint with `adjustment=split` (the same
/// series as the Nasdaq source: split adjusted, not dividend adjusted; the record of the cache says so).
/// Many symbols go in one request, so 1,000 tickers are a small number of requests.
pub struct Alpaca {
    client: Client,
    feed: String,
    now: fn() -> i64,
}

impl Alpaca {
    pub fn new(client: Client, feed: &str, now: fn() -> i64) -> Alpaca {
        Alpaca { client, feed: feed.to_string(), now }
    }
    pub fn live(keys: Keys) -> Alpaca {
        Alpaca::new(Client::live(keys), &feed_from_env(), now_utc)
    }
    pub fn requests(&self) -> usize {
        self.client.requests()
    }
}

impl PriceSource for Alpaca {
    fn name(&self) -> &'static str {
        "alpaca"
    }
    fn adjusts_dividends(&self) -> bool {
        false
    }
    fn detail(&self) -> Option<String> {
        Some(format!("feed={} adjustment=split", self.feed))
    }
    fn batch_size(&self) -> usize {
        SYMBOLS_PER_REQUEST
    }
    fn fetch(&self, ticker: &str, from: Option<&str>, to: &str) -> Result<Vec<Row>, FetchError> {
        Ok(self.fetch_many(&[ticker.to_uppercase()], from, to)?.remove(&ticker.to_uppercase()).unwrap_or_default())
    }
    fn fetch_many(&self, tickers: &[String], from: Option<&str>, to: &str) -> Result<HashMap<String, Vec<Row>>, FetchError> {
        let now = (self.now)();
        let today = iso((now.div_euclid(86_400)) as i32);
        let start = from.filter(|f| *f > FIRST_DAY).unwrap_or(FIRST_DAY);
        let mut out = HashMap::new();
        for chunk in tickers.chunks(SYMBOLS_PER_REQUEST) {
            let api: Vec<String> = chunk.iter().map(|t| api_symbol(t)).collect();
            let bars = self.client.stock_bars(&StockQuery { symbols: &api, timeframe: "1Day", start, to, today: &today, adjustment: "split", feed: &self.feed })?;
            for t in chunk {
                let rows = bars.get(&api_symbol(t)).map(|b| daily_rows(b, now)).unwrap_or_default();
                out.insert(t.to_uppercase(), rows);
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------- option contracts and chain

#[derive(Debug, Clone, PartialEq)]
pub struct ContractInfo {
    pub symbol: String,
    pub expiry: String,
    pub call: bool,
    pub strike: f64,
}

fn number(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

/// One page of `GET /v2/options/contracts` (https://docs.alpaca.markets/reference/get-options-contracts):
/// `{"option_contracts": [{"symbol","expiration_date","type","strike_price", ...}], "next_page_token"}`.
/// The strike can be text or a number.
pub fn parse_contracts(page: &Value) -> Result<Vec<ContractInfo>, String> {
    let list = match page.get("option_contracts") {
        Some(Value::Array(a)) => a,
        Some(Value::Null) => return Ok(Vec::new()),
        _ => return Err(format!("Alpaca error: {}", page["message"].as_str().unwrap_or("no option_contracts").chars().take(200).collect::<String>())),
    };
    let mut out = Vec::new();
    for c in list {
        let (Some(symbol), Some(expiry), Some(kind), Some(strike)) = (c["symbol"].as_str(), c["expiration_date"].as_str(), c["type"].as_str(), number(&c["strike_price"])) else { continue };
        out.push(ContractInfo { symbol: symbol.to_string(), expiry: expiry.chars().take(10).collect(), call: kind.eq_ignore_ascii_case("call"), strike });
    }
    Ok(out)
}

impl Client {
    /// The contracts of an underlying with an expiry in a window and a strike in a range. `status` is `active` or
    /// `inactive` (expired). The depth of the inactive list is not in the documentation (#231).
    pub fn contracts(&self, underlying: &str, status: &str, expiry_from: &str, expiry_to: &str, strike_min: f64, strike_max: f64) -> Result<Vec<ContractInfo>, FetchError> {
        let base = format!(
            "{}/v2/options/contracts?underlying_symbols={underlying}&status={status}&expiration_date_gte={expiry_from}&expiration_date_lte={expiry_to}&strike_price_gte={strike_min:.2}&strike_price_lte={strike_max:.2}&limit=10000",
            trading_base()
        );
        let mut all = Vec::new();
        self.paged(&base, |page| {
            all.extend(parse_contracts(page)?);
            Ok(())
        })?;
        Ok(all)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChainItem {
    pub symbol: String,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub last: Option<f64>,
    pub iv: Option<f64>,
    pub volume: Option<f64>,
}

/// One page of `GET /v1beta1/options/snapshots/{underlying}` (https://docs.alpaca.markets/reference/optionchain):
/// `{"snapshots": {"AAPL240426C00162500": {"latestQuote": {"bp","ap",...}, "latestTrade": {"p",...}, "greeks": {...},
/// "impliedVolatility": 0.337, "dailyBar": {...}}}, "next_page_token": ...}`.
pub fn parse_chain(page: &Value) -> Result<Vec<ChainItem>, String> {
    let Some(map) = page.get("snapshots").and_then(Value::as_object) else {
        return Err(format!("Alpaca error: {}", page["message"].as_str().unwrap_or("no snapshots").chars().take(200).collect::<String>()));
    };
    let pos = |v: &Value| v.as_f64().filter(|x| x.is_finite() && *x > 0.0);
    Ok(map
        .iter()
        .map(|(symbol, s)| ChainItem {
            symbol: symbol.clone(),
            bid: pos(&s["latestQuote"]["bp"]),
            ask: pos(&s["latestQuote"]["ap"]),
            last: pos(&s["latestTrade"]["p"]),
            iv: s["impliedVolatility"].as_f64().filter(|x| x.is_finite()),
            volume: s["dailyBar"]["v"].as_f64(),
        })
        .collect())
}

impl Client {
    /// The whole chain of an underlying, all pages. `feed` is `indicative` (free) or `opra`.
    pub fn chain(&self, underlying: &str, feed: &str) -> Result<Vec<ChainItem>, FetchError> {
        let base = format!("{DATA_BASE}/v1beta1/options/snapshots/{}?feed={feed}&limit=1000", underlying.to_uppercase());
        let mut all = Vec::new();
        self.paged(&base, |page| {
            all.extend(parse_chain(page)?);
            Ok(())
        })?;
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn keys() -> Keys {
        Keys { id: "KEYIDVALUE".into(), secret: "SECRETVALUE99".into() }
    }

    /// A request of daily bars from 2024-01-01 to `to`; today is 2026-11-19.
    fn q<'a>(symbols: &'a [&str], to: &'a str) -> StockQuery<'a> {
        let list: &'a [String] = Box::leak(symbols.iter().map(|s| s.to_string()).collect::<Vec<_>>().into_boxed_slice());
        StockQuery { symbols: list, timeframe: "1Day", start: "2024-01-01", to, today: "2026-11-19", adjustment: "split", feed: "iex" }
    }

    /// A reply of the documentation: https://docs.alpaca.markets/reference/stockbars
    const STOCK: &str = r#"{"bars":{"AAPL":[
        {"t":"2022-01-03T05:00:00Z","o":177.83,"h":182.88,"l":177.71,"c":182.01,"v":104487900,"n":656078,"vw":181.1},
        {"t":"2022-01-04T05:00:00Z","o":182.63,"h":182.94,"l":179.12,"c":179.7,"v":99310400,"n":595980,"vw":180.9},
        {"t":"bad","o":1,"h":1,"l":1,"c":1}]},
        "next_page_token":"QUFQTHxEfDIwMjItMDEtMDRUMDU6MDA6MDAuMDAwMDAwMDAwWg=="}"#;

    #[test]
    fn rfc3339() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339("2024-01-18T05:00:00Z"), Some(1_705_554_000));
        assert_eq!(parse_rfc3339("2024-01-18T00:00:00-05:00"), Some(1_705_554_000));
        assert_eq!(parse_rfc3339("2024-01-18T05:00:00.123456789Z"), Some(1_705_554_000));
        assert_eq!(parse_rfc3339("2024-01-18"), None);
        assert_eq!(parse_rfc3339("2024-01-18T05:00:00"), None);
    }

    #[test]
    fn stock_bar_page_and_errors() {
        let page: Value = serde_json::from_str(STOCK).unwrap();
        let bars = parse_bars(&page).unwrap();
        assert_eq!(bars["AAPL"].len(), 2, "a bar with a bad time is dropped");
        assert_eq!(bars["AAPL"][0].c, 182.01);
        assert_eq!(parse_bars(&serde_json::json!({"bars": null, "next_page_token": null})).unwrap().len(), 0);
        let err = parse_bars(&serde_json::json!({"message": "forbidden."})).unwrap_err();
        assert!(err.contains("forbidden"), "{err}");
        assert!(parse_bars(&serde_json::json!({})).is_err());
    }

    #[test]
    fn daily_rows_use_the_new_york_date_and_skip_a_running_day() {
        let bars = parse_bars(&serde_json::from_str(STOCK).unwrap()).unwrap().remove("AAPL").unwrap();
        // 2022-01-04 15:00 UTC is 10:00 New York: that day is running.
        let rows = daily_rows(&bars, parse_rfc3339("2022-01-04T15:00:00Z").unwrap());
        assert_eq!(rows.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(), ["2022-01-03"]);
        // After 16:20 New York time the day is complete.
        let rows = daily_rows(&bars, parse_rfc3339("2022-01-04T21:30:00Z").unwrap());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].close, 179.7);
        assert_eq!(rows[1].volume, 99_310_400);
        assert!(rows.iter().all(|r| r.adj_close.is_none()));
        // A summer bar (04:00 UTC) has the same date.
        assert_eq!(ny_day(parse_rfc3339("2024-06-07T04:00:00Z").unwrap()).0, "2024-06-07");
    }

    #[test]
    fn intraday_rows_keep_the_regular_session_and_complete_bars() {
        let t = |s: &str| parse_rfc3339(s).unwrap();
        let bar = |utc: i64, c: f64| Bar { utc, o: c, h: c, l: c, c, v: 10.0 };
        // 2024-06-07: UTC-4. 13:25Z is 09:25 (before the open), 13:30Z is 09:30, 19:55Z is 15:55, 20:00Z is 16:00 (not in).
        let bars = [bar(t("2024-06-07T13:25:00Z"), 1.0), bar(t("2024-06-07T13:30:00Z"), 2.0), bar(t("2024-06-07T19:55:00Z"), 3.0), bar(t("2024-06-07T20:00:00Z"), 4.0)];
        let rows = intraday_rows(&bars, 5, t("2024-06-10T00:00:00Z"));
        assert_eq!(rows.len(), 2);
        assert!(rows[0].line.starts_with("2024-06-07 09:30:00-04:00,2.0,"), "{}", rows[0].line);
        assert!(rows[1].line.starts_with("2024-06-07 15:55:00-04:00,3.0,"));
        // The bar of 15:55 has not ended at 15:57.
        let rows = intraday_rows(&bars, 5, t("2024-06-07T19:57:00Z"));
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn the_keys_are_in_headers_and_not_in_errors() {
        let seen = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let s2 = seen.clone();
        let client = Client::new(keys(), Box::new(move |url| {
            s2.borrow_mut().push(url.to_string());
            Err("HTTP 401 for x; headers KEYIDVALUE SECRETVALUE99".into())
        }));
        let e = client.stock_bars(&q(&["IBM"], "2024-02-01")).unwrap_err();
        let text = e.to_string();
        assert!(!text.contains("KEYIDVALUE") && !text.contains("SECRETVALUE99") && text.contains("***"), "{text}");
        assert!(seen.borrow().iter().all(|u| !u.contains("KEYIDVALUE") && !u.contains("SECRETVALUE99") && !u.contains("api_key")));
        // A reply that repeats the key is scrubbed too.
        let client = Client::new(keys(), Box::new(|_| Ok(br#"{"message":"bad key KEYIDVALUE"}"#.to_vec())));
        let e = client.stock_bars(&q(&["IBM"], "2024-02-01")).unwrap_err();
        assert!(!e.to_string().contains("KEYIDVALUE"), "{e}");
        let limited = Client::new(keys(), Box::new(|_| Err("HTTP 429 for https://data.alpaca.markets/x".into())));
        assert!(matches!(limited.stock_bars(&q(&["IBM"], "2024-02-01")), Err(FetchError::RateLimited(_))));
    }

    #[test]
    fn the_url_of_stock_bars() {
        let seen = Rc::new(std::cell::RefCell::new(String::new()));
        let s2 = seen.clone();
        let client = Client::new(keys(), Box::new(move |url| {
            *s2.borrow_mut() = url.to_string();
            Ok(br#"{"bars":{},"next_page_token":null}"#.to_vec())
        }));
        client.stock_bars(&q(&["AAPL", "BRK.B"], "2024-02-01")).unwrap();
        let url = seen.borrow().clone();
        assert_eq!(url, "https://data.alpaca.markets/v2/stocks/bars?symbols=AAPL,BRK.B&timeframe=1Day&start=2024-01-01&end=2024-02-01T23:59:59Z&limit=10000&adjustment=split&feed=iex&sort=asc");
        // A window that reaches today has no `end`.
        client.stock_bars(&q(&["AAPL"], "2026-11-19")).unwrap();
        assert!(!seen.borrow().contains("end="));
        assert_eq!(api_symbol("brk-b"), "BRK.B");
    }

    // ---- a fake server with the paging rule of the documentation

    fn param(url: &str, name: &str) -> Option<String> {
        let q = url.split_once('?')?.1;
        q.split('&').find_map(|kv| kv.strip_prefix(&format!("{name}="))).map(|v| v.replace("%3D", "="))
    }

    /// Serves bars for the symbols `T0000`... Each symbol has `rows` daily bars. The page has `limit` points of all
    /// symbols together, sorted by symbol, then by time (the rule of the documentation); the token is an offset.
    fn fake_server(rows: usize, calls: Rc<Cell<usize>>) -> Get {
        Box::new(move |url| {
            calls.set(calls.get() + 1);
            let symbols: Vec<String> = param(url, "symbols").unwrap().split(',').map(String::from).collect();
            let limit: usize = param(url, "limit").map_or(1000, |l| l.parse().unwrap());
            let offset: usize = param(url, "page_token").map_or(0, |t| t.trim_start_matches("QUFQ").trim_end_matches('=').parse().unwrap());
            let total = symbols.len() * rows;
            let mut bars = serde_json::Map::new();
            for i in offset..total.min(offset + limit) {
                let (s, r) = (&symbols[i / rows], i % rows);
                let day = 17_000 + r as i64; // days since 1970; midnight New York is 05:00 UTC in winter, close enough for a fake
                let t = format!("{}T05:00:00Z", iso(day as i32));
                bars.entry(s.clone()).or_insert_with(|| Value::Array(vec![])).as_array_mut().unwrap().push(serde_json::json!({"t": t, "o": 10.0, "h": 11.0, "l": 9.0, "c": 10.5, "v": 100}));
            }
            let next = if offset + limit < total { Value::String(format!("QUFQ{}==", offset + limit)) } else { Value::Null };
            Ok(serde_json::to_vec(&serde_json::json!({"bars": bars, "next_page_token": next})).unwrap())
        })
    }

    fn tickers(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("T{i:04}")).collect()
    }

    fn fixed_now() -> i64 {
        parse_rfc3339("2026-11-19T12:00:00Z").unwrap()
    }

    #[test]
    fn one_thousand_tickers_need_ten_requests_for_an_update() {
        let calls = Rc::new(Cell::new(0));
        let src = Alpaca::new(Client::new(keys(), fake_server(7, calls.clone())), "iex", fixed_now);
        let out = src.fetch_many(&tickers(1000), Some("2026-11-07"), "2026-11-19").unwrap();
        assert_eq!(out.len(), 1000);
        assert!(out.values().all(|r| r.len() == 7), "7 rows of 7 days for each ticker");
        assert_eq!(calls.get(), 10, "100 symbols in a request, 700 points fit in one page");
        assert_eq!(src.requests(), 10);
    }

    #[test]
    fn paging_follows_the_token_and_a_full_history_counts_its_pages() {
        // 200 tickers with 2,520 days (10 years): 100 symbols * 2,520 = 252,000 points = 26 pages of 10,000; two chunks.
        let calls = Rc::new(Cell::new(0));
        let src = Alpaca::new(Client::new(keys(), fake_server(2520, calls.clone())), "iex", fixed_now);
        let out = src.fetch_many(&tickers(200), None, "2026-11-19").unwrap();
        assert!(out.values().all(|r| r.len() == 2520), "every row of every page, in order");
        assert_eq!(calls.get(), 52);
        // 1,000 tickers of 10 years: 10 chunks * 26 pages = 260 requests, 156 seconds at 0.4 s.
        assert_eq!(10 * 26, 260);
        let one = out.values().next().unwrap();
        assert!(one.windows(2).all(|w| w[0].date < w[1].date));
    }

    #[test]
    fn the_source_form() {
        let one_page = STOCK.split(",\n        \"next_page_token\"").next().unwrap().to_string() + ",\"next_page_token\":null}";
        let src = Alpaca::new(Client::new(keys(), Box::new(move |_| Ok(one_page.clone().into_bytes()))), "iex", fixed_now);
        assert_eq!(src.name(), "alpaca");
        assert!(!src.adjusts_dividends());
        assert_eq!(src.detail().as_deref(), Some("feed=iex adjustment=split"));
        assert_eq!(src.batch_size(), 100);
        let rows = src.fetch("aapl", None, "2026-11-19").unwrap();
        assert_eq!(rows.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(), ["2022-01-03", "2022-01-04"]);
    }

    #[test]
    fn contracts_and_chain_fixtures() {
        // https://docs.alpaca.markets/reference/get-options-contracts
        let page: Value = serde_json::from_str(r#"{"option_contracts":[
            {"id":"a","symbol":"SPY240419C00500000","name":"SPY Apr 19 2024 500 Call","status":"inactive","tradable":false,"expiration_date":"2024-04-19","root_symbol":"SPY","underlying_symbol":"SPY","type":"call","style":"american","strike_price":"500","size":"100","open_interest":"10","close_price":"1.5"},
            {"symbol":"SPY240419P00500500","expiration_date":"2024-04-19","type":"put","strike_price":500.5},
            {"symbol":"broken"}],"next_page_token":null}"#).unwrap();
        let c = parse_contracts(&page).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0], ContractInfo { symbol: "SPY240419C00500000".into(), expiry: "2024-04-19".into(), call: true, strike: 500.0 });
        assert!(!c[1].call && c[1].strike == 500.5);
        // https://docs.alpaca.markets/reference/optionchain (the sample contract of the page)
        let chain: Value = serde_json::from_str(r#"{"snapshots":{"AAPL240426C00162500":{"latestTrade":{"t":"2024-04-23T19:59:59Z","x":"C","p":4.1,"s":1,"c":"I"},
            "latestQuote":{"t":"2024-04-23T20:00:00Z","ax":"C","ap":4.3,"as":10,"bx":"C","bp":4.15,"bs":12,"c":"A"},
            "greeks":{"delta":0.5,"gamma":0.1,"theta":-0.2,"vega":0.1,"rho":0.01},"impliedVolatility":0.337},
            "AAPL240426P00162500":{"latestQuote":{"bp":0,"ap":0}}},"next_page_token":null}"#).unwrap();
        let items = parse_chain(&chain).unwrap();
        assert_eq!(items.len(), 2);
        let call = items.iter().find(|i| i.symbol.contains("C00162500")).unwrap();
        assert_eq!((call.bid, call.ask, call.last, call.iv), (Some(4.15), Some(4.3), Some(4.1), Some(0.337)));
        let put = items.iter().find(|i| i.symbol.contains("P00162500")).unwrap();
        assert_eq!((put.bid, put.ask, put.iv), (None, None, None), "a zero quote is no quote");
        assert!(parse_chain(&serde_json::json!({"message":"x"})).is_err());
    }
}
