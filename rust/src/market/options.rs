//! Option chain snapshots and their metrics (the Rust form of `options.py`).
//!
//! The source is the Cboe delayed quotes file of one underlying (`trading::chain` reads the same
//! file for the paper trader). It has all listed expiries, with bid, ask, last trade, implied
//! volatility, volume and open interest. The Python code used yfinance (Yahoo). Two things are
//! different: the implied volatility is the one of Cboe (a fraction, as 0.31), and the spot price is the
//! last price of the underlying in the same Cboe file (about 15 minutes late), not the last daily
//! close.
//!
//! A pull is a dated snapshot, `data/raw/<T>/options/<YYYY-MM-DD>.json` (one for each day; a second
//! pull of the same day reads the file unless `refresh`). The format is the format of the Python code.
//! The metrics (`compute_chain_metrics`) are the same calculation as in Python: fixtures from the Python
//! functions are in `rust/tests/fixtures/market/options_metrics.json`.

use super::{clean, lenient_json, round6, ticker_dir, write_atomic};
use crate::trading::calendar::iso;
use crate::trading::chain::parse_option;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const URL: &str = "https://cdn-api.cboe.com/api/global/delayed_quotes/options/{symbol}.json";

/// Moneyness targets for the skew (the downside put wing against the upside call wing).
pub const SKEW_PUT_MONEYNESS: f64 = 0.90;
pub const SKEW_CALL_MONEYNESS: f64 = 1.10;
/// An implied volatility below this is an unsolved quote, and it is a missing value.
pub const MIN_VALID_IV: f64 = 1e-4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionQuote {
    pub contract_symbol: String,
    pub option_type: String,
    pub expiry: String,
    pub strike: f64,
    #[serde(default)]
    pub last_price: Option<f64>,
    #[serde(default)]
    pub bid: Option<f64>,
    #[serde(default)]
    pub ask: Option<f64>,
    #[serde(default)]
    pub volume: Option<i64>,
    #[serde(default)]
    pub open_interest: Option<i64>,
    #[serde(default)]
    pub implied_volatility: Option<f64>,
    #[serde(default)]
    pub in_the_money: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionChainSnapshot {
    pub ticker: String,
    pub snapshot_date: String,
    #[serde(default)]
    pub spot_price: Option<f64>,
    pub quotes: Vec<OptionQuote>,
    /// Where the quotes came from: `alpaca` or `alpaca-history`. A file with no value is a Cboe delayed snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The feed of the source (`indicative` for Alpaca live chains).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<String>,
    /// What a quote is: `close` (a daily close of the contract: no bid and no ask) or `indicative` (modified quotes of
    /// the free Alpaca feed). A file with no value has real bid and ask (Cboe).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote_kind: Option<String>,
    /// A text of the method (how the implied volatility was calculated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The time of a live snapshot, UTC to the minute (`2026-11-20T15:42Z`). A file with no value (Cboe, closes, files before
    /// 2026-11-21) has no time and the name `<date>.json`; a file with a time has the name `<date>T<hhmm>.json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_time: Option<String>,
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

pub fn options_dir(cache: &Path, ticker: &str) -> PathBuf {
    ticker_dir(cache, ticker).join("options")
}

pub fn snapshot_path(cache: &Path, ticker: &str, date: &str) -> PathBuf {
    options_dir(cache, ticker).join(format!("{date}.json"))
}

/// The file of a live snapshot with a time: `<date>T<hhmm>.json` (hhmm in UTC).
pub fn timed_path(cache: &Path, ticker: &str, date: &str, hhmm: &str) -> PathBuf {
    options_dir(cache, ticker).join(format!("{date}T{hhmm}.json"))
}

/// The time stamp (`YYYY-MM-DDTHH:MMZ`) of a time in seconds since 1970 (UTC), and its `hhmm`.
pub fn utc_stamp(secs: i64) -> (String, String) {
    let day = secs.div_euclid(86_400);
    let minute = secs.rem_euclid(86_400) / 60;
    let (h, m) = (minute / 60, minute % 60);
    (format!("{}T{h:02}:{m:02}Z", iso(day as i32)), format!("{h:02}{m:02}"))
}

/// `hhmm` of a time stamp `YYYY-MM-DDTHH:MMZ`; `None` for a text of another form.
pub fn hhmm_of_stamp(stamp: &str) -> Option<String> {
    let b = stamp.as_bytes();
    let ok = b.len() == 17 && b[10] == b'T' && b[13] == b':' && b[16] == b'Z' && [11, 12, 14, 15].iter().all(|&i| b[i].is_ascii_digit());
    ok.then(|| format!("{}{}", &stamp[11..13], &stamp[14..16]))
}

fn valid_hhmm(t: &str) -> bool {
    t.len() == 4 && t.bytes().all(|c| c.is_ascii_digit()) && t[..2].parse::<u32>().is_ok_and(|h| h < 24) && t[2..].parse::<u32>().is_ok_and(|m| m < 60)
}

/// The path that `save_snapshot` writes: with a time in the snapshot, the timed name; else the date name.
pub fn path_of(cache: &Path, snapshot: &OptionChainSnapshot) -> PathBuf {
    match snapshot.snapshot_time.as_deref().and_then(hhmm_of_stamp) {
        Some(hhmm) => timed_path(cache, &snapshot.ticker, &snapshot.snapshot_date, &hhmm),
        None => snapshot_path(cache, &snapshot.ticker, &snapshot.snapshot_date),
    }
}

pub fn save_snapshot(snapshot: &OptionChainSnapshot, cache: &Path) -> Result<PathBuf, String> {
    let path = path_of(cache, snapshot);
    write_atomic(&path, &serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?)?;
    Ok(path)
}

/// The stem of a snapshot file name: `<date>` or `<date>T<hhmm>`. `None` for another name.
fn parse_stem(stem: &str) -> Option<&str> {
    match stem.split_once('T') {
        None => super::parse_date(stem).map(|_| stem),
        Some((d, t)) => (valid_hhmm(t) && super::parse_date(d).is_some()).then_some(d),
    }
}

/// All snapshot files of a ticker: stem (`<date>` or `<date>T<hhmm>`) -> path, sorted by stem. A name `<stem>.json.gz`
/// (the compact copy of `fa options archive`) counts when no `<stem>.json` exists. Another name is ignored.
pub fn snapshot_files(cache: &Path, ticker: &str) -> BTreeMap<String, PathBuf> {
    let mut out: BTreeMap<String, PathBuf> = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(options_dir(cache, ticker)) else { return out };
    for e in entries.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().to_string();
        let (stem, gz) = match (name.strip_suffix(".json"), name.strip_suffix(".json.gz")) {
            (Some(s), _) => (s, false),
            (_, Some(s)) => (s, true),
            _ => continue,
        };
        if parse_stem(stem).is_none() {
            continue;
        }
        if gz && out.contains_key(stem) {
            continue;
        }
        out.insert(stem.to_string(), e.path());
    }
    out
}

/// The dates of the snapshots of a ticker, oldest first, each date once (a date with several files is one date).
pub fn list_snapshots(cache: &Path, ticker: &str) -> Vec<String> {
    let mut dates: Vec<String> = snapshot_files(cache, ticker).keys().filter_map(|s| parse_stem(s).map(String::from)).collect();
    dates.dedup();
    dates
}

/// The stems of the files of one date, oldest first (`<date>` first, then `<date>T<hhmm>` by time).
pub fn stems_of_date(cache: &Path, ticker: &str, date: &str) -> Vec<String> {
    snapshot_files(cache, ticker).into_keys().filter(|s| parse_stem(s) == Some(date)).collect()
}

/// The text of a snapshot file; a `.gz` file is decoded.
pub fn read_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if path.extension().is_some_and(|x| x == "gz") {
        use std::io::Read;
        let mut text = String::new();
        flate2::read::GzDecoder::new(&bytes[..]).read_to_string(&mut text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(text)
    } else {
        String::from_utf8(bytes).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// A snapshot by date; `None` is the latest date. A date with several files gives the LAST one (the latest time).
pub fn load_snapshot(cache: &Path, ticker: &str, date: Option<&str>) -> Result<OptionChainSnapshot, String> {
    load_snapshot_at(cache, ticker, date, None)
}

/// `load_snapshot` with a time: `time` is `hhmm` (UTC) and gives the file `<date>T<hhmm>`; `None` gives the last file of the date.
pub fn load_snapshot_at(cache: &Path, ticker: &str, date: Option<&str>, time: Option<&str>) -> Result<OptionChainSnapshot, String> {
    let date = match date {
        Some(d) => d.to_string(),
        None => list_snapshots(cache, ticker)
            .pop()
            .ok_or_else(|| format!("No option-chain snapshots for {} under {}", ticker.to_uppercase(), options_dir(cache, ticker).display()))?,
    };
    let files = snapshot_files(cache, ticker);
    let stem = match time {
        Some(t) => {
            if !valid_hhmm(t) {
                return Err(format!("bad --time '{t}' (use HHMM in UTC, for example 1930)"));
            }
            format!("{date}T{t}")
        }
        None => stems_of_date(cache, ticker, &date).pop().unwrap_or_else(|| date.clone()),
    };
    let path = files.get(&stem).cloned().unwrap_or_else(|| snapshot_path(cache, ticker, &stem));
    let text = read_text(&path).map_err(|e| format!("No option-chain snapshot for {} on {date}{} at {}: {e}", ticker.to_uppercase(), time.map(|t| format!(" at {t} UTC")).unwrap_or_default(), path.display()))?;
    serde_json::from_str(&lenient_json(&text)).map_err(|e| format!("{}: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// The Cboe file
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Reply {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    symbol: String,
    #[serde(default)]
    current_price: Option<f64>,
    #[serde(default)]
    options: Vec<Opt>,
}

#[derive(Deserialize)]
struct Opt {
    option: String,
    #[serde(default)]
    bid: Option<f64>,
    #[serde(default)]
    ask: Option<f64>,
    #[serde(default)]
    iv: Option<f64>,
    #[serde(default)]
    volume: Option<f64>,
    #[serde(default)]
    open_interest: Option<f64>,
    #[serde(default)]
    last_trade_price: Option<f64>,
}

pub fn url(ticker: &str) -> String {
    URL.replace("{symbol}", &ticker.to_uppercase())
}

fn count(x: Option<f64>) -> Option<i64> {
    clean(x).map(|v| v as i64)
}

/// The snapshot of a Cboe reply. A contract of another root (the adjusted options, `B1IBM...`) is not in it.
/// The quotes are in the order of the Python code: expiry, call before put, strike.
pub fn snapshot_from_cboe(bytes: &[u8], ticker: &str, snapshot_date: &str) -> Result<OptionChainSnapshot, String> {
    let ticker = ticker.to_uppercase();
    let reply: Reply = serde_json::from_slice(bytes).map_err(|e| format!("Cboe reply for {ticker} is not the expected JSON: {e}"))?;
    if !reply.data.symbol.eq_ignore_ascii_case(&ticker) {
        return Err(format!("Cboe reply is for {}, not {ticker}", reply.data.symbol));
    }
    let spot = clean(reply.data.current_price);
    let mut quotes = Vec::new();
    for o in &reply.data.options {
        let Some((expiry, call, thousandths)) = parse_option(&o.option, &ticker) else { continue };
        let strike = thousandths as f64 / 1000.0;
        quotes.push(OptionQuote {
            contract_symbol: o.option.clone(),
            option_type: if call { "call" } else { "put" }.to_string(),
            expiry: iso(expiry),
            strike,
            last_price: clean(o.last_trade_price),
            bid: clean(o.bid),
            ask: clean(o.ask),
            volume: count(o.volume),
            open_interest: count(o.open_interest),
            implied_volatility: clean(o.iv),
            in_the_money: spot.map(|s| if call { strike < s } else { strike > s }),
        });
    }
    quotes.sort_by(|a, b| (&a.expiry, &a.option_type).cmp(&(&b.expiry, &b.option_type)).then(a.strike.total_cmp(&b.strike)));
    Ok(OptionChainSnapshot { ticker, snapshot_date: snapshot_date.to_string(), spot_price: spot, quotes, source: None, feed: None, quote_kind: None, note: None, snapshot_time: None })
}

#[derive(Debug, PartialEq)]
pub enum Source {
    Cache,
    Cboe,
}

/// The option chain: today's snapshot from the cache, or the Cboe file (and a new snapshot file).
pub fn fetch_option_chain(
    cache: &Path,
    ticker: &str,
    refresh: bool,
    today: &str,
    fetch: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<(OptionChainSnapshot, Source), String> {
    let ticker = ticker.to_uppercase();
    if !refresh && snapshot_path(cache, &ticker, today).is_file() {
        match load_snapshot(cache, &ticker, Some(today)) {
            Ok(snapshot) => return Ok((snapshot, Source::Cache)),
            Err(e) => eprintln!("fa: corrupt options snapshot for {ticker} ({e}); asking Cboe again"),
        }
    }
    let bytes = fetch(&url(&ticker)).map_err(|e| format!("Cboe: {e}"))?;
    let snapshot = snapshot_from_cboe(&bytes, &ticker, today)?;
    if snapshot.quotes.is_empty() {
        eprintln!("fa: no listed options found for {ticker}; saving an empty snapshot");
    }
    save_snapshot(&snapshot, cache)?;
    Ok((snapshot, Source::Cboe))
}

// ---------------------------------------------------------------------------
// Derived metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExpiryMetrics {
    pub expiry: String,
    pub atm_strike: Option<f64>,
    pub atm_iv: Option<f64>,
    pub straddle_mid: Option<f64>,
    pub implied_move: Option<f64>,
    pub skew: Option<f64>,
    pub call_volume: i64,
    pub put_volume: i64,
    pub call_open_interest: i64,
    pub put_open_interest: i64,
}

fn ratio(put: i64, call: i64) -> Option<f64> {
    (call != 0).then(|| round6(put as f64 / call as f64))
}

impl ExpiryMetrics {
    pub fn put_call_volume_ratio(&self) -> Option<f64> {
        ratio(self.put_volume, self.call_volume)
    }
    pub fn put_call_oi_ratio(&self) -> Option<f64> {
        ratio(self.put_open_interest, self.call_open_interest)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChainMetrics {
    pub ticker: String,
    pub snapshot_date: String,
    pub spot_price: Option<f64>,
    pub expiries: Vec<ExpiryMetrics>,
    pub put_call_volume_ratio: Option<f64>,
    pub put_call_oi_ratio: Option<f64>,
}

fn valid_iv(q: Option<&OptionQuote>) -> Option<f64> {
    let iv = q?.implied_volatility?;
    (!iv.is_nan() && iv >= MIN_VALID_IV).then_some(iv)
}

/// The bid and ask mid where both sides are quoted; else the last trade.
fn quote_price(q: Option<&OptionQuote>) -> Option<f64> {
    let q = q?;
    if let (Some(bid), Some(ask)) = (q.bid, q.ask) {
        if bid > 0.0 && ask > 0.0 {
            return Some((bid + ask) / 2.0);
        }
    }
    q.last_price.filter(|p| *p > 0.0)
}

/// The quote with the strike nearest to the target (the first one in the list for a tie).
fn nearest_by_strike<'a>(quotes: &[&'a OptionQuote], target: f64) -> Option<&'a OptionQuote> {
    quotes.iter().copied().min_by(|a, b| (a.strike - target).abs().total_cmp(&(b.strike - target).abs()))
}

/// ATM implied volatility, implied move, skew and put/call ratios for each expiry. A metric
/// whose inputs are missing is `None`, never a made-up number.
pub fn compute_chain_metrics(snapshot: &OptionChainSnapshot) -> ChainMetrics {
    let spot = snapshot.spot_price;
    let mut by_expiry: BTreeMap<&str, Vec<&OptionQuote>> = BTreeMap::new();
    for q in &snapshot.quotes {
        by_expiry.entry(q.expiry.as_str()).or_default().push(q);
    }
    let mut expiries = Vec::new();
    for (expiry, group) in by_expiry {
        let calls: Vec<&OptionQuote> = group.iter().copied().filter(|q| q.option_type == "call").collect();
        let puts: Vec<&OptionQuote> = group.iter().copied().filter(|q| q.option_type == "put").collect();
        let sum = |qs: &[&OptionQuote], f: fn(&OptionQuote) -> Option<i64>| qs.iter().map(|q| f(q).unwrap_or(0)).sum::<i64>();
        let mut em = ExpiryMetrics {
            expiry: expiry.to_string(),
            call_volume: sum(&calls, |q| q.volume),
            put_volume: sum(&puts, |q| q.volume),
            call_open_interest: sum(&calls, |q| q.open_interest),
            put_open_interest: sum(&puts, |q| q.open_interest),
            ..Default::default()
        };
        if let Some(spot) = spot.filter(|s| *s > 0.0) {
            let mut strikes: Vec<f64> = group.iter().map(|q| q.strike).collect();
            strikes.sort_by(f64::total_cmp);
            strikes.dedup();
            let atm = strikes.iter().copied().min_by(|a, b| (a - spot).abs().total_cmp(&(b - spot).abs())).expect("a group has a quote");
            em.atm_strike = Some(atm);
            let atm_call = calls.iter().copied().find(|q| q.strike == atm);
            let atm_put = puts.iter().copied().find(|q| q.strike == atm);
            let ivs: Vec<f64> = [valid_iv(atm_call), valid_iv(atm_put)].into_iter().flatten().collect();
            if !ivs.is_empty() {
                em.atm_iv = Some(round6(ivs.iter().sum::<f64>() / ivs.len() as f64));
            }
            if let (Some(c), Some(p)) = (quote_price(atm_call), quote_price(atm_put)) {
                let straddle = round6(c + p);
                em.straddle_mid = Some(straddle);
                em.implied_move = Some(round6(straddle / spot));
            }
            let put_wing: Vec<&OptionQuote> = puts.iter().copied().filter(|q| valid_iv(Some(q)).is_some()).collect();
            let call_wing: Vec<&OptionQuote> = calls.iter().copied().filter(|q| valid_iv(Some(q)).is_some()).collect();
            if let (Some(p), Some(c)) = (nearest_by_strike(&put_wing, spot * SKEW_PUT_MONEYNESS), nearest_by_strike(&call_wing, spot * SKEW_CALL_MONEYNESS)) {
                em.skew = Some(round6(valid_iv(Some(p)).expect("valid") - valid_iv(Some(c)).expect("valid")));
            }
        }
        expiries.push(em);
    }
    let total = |f: fn(&ExpiryMetrics) -> i64| expiries.iter().map(f).sum::<i64>();
    let (cv, pv, co, po) = (total(|e| e.call_volume), total(|e| e.put_volume), total(|e| e.call_open_interest), total(|e| e.put_open_interest));
    ChainMetrics {
        ticker: snapshot.ticker.clone(),
        snapshot_date: snapshot.snapshot_date.clone(),
        spot_price: spot,
        put_call_volume_ratio: (cv > 0).then(|| round6(pv as f64 / cv as f64)),
        put_call_oi_ratio: (co > 0).then(|| round6(po as f64 / co as f64)),
        expiries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn cases() -> Vec<Value> {
        let path = format!("{}/tests/fixtures/market/options_metrics.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn opt(v: &Value) -> Option<f64> {
        v.as_f64()
    }

    #[test]
    fn metrics_match_the_python_functions() {
        let mut n_quotes = 0;
        for case in cases() {
            let name = case["name"].as_str().unwrap();
            let snapshot: OptionChainSnapshot = serde_json::from_value(case["snapshot"].clone()).unwrap();
            n_quotes += snapshot.quotes.len();
            let m = compute_chain_metrics(&snapshot);
            let e = &case["expected"];
            assert_eq!(m.spot_price, opt(&e["spot_price"]), "{name}");
            assert_eq!(m.put_call_volume_ratio, opt(&e["put_call_volume_ratio"]), "{name} total ratio");
            assert_eq!(m.put_call_oi_ratio, opt(&e["put_call_oi_ratio"]), "{name} total oi ratio");
            let expected = e["expiries"].as_array().unwrap();
            assert_eq!(m.expiries.len(), expected.len(), "{name}");
            for (got, want) in m.expiries.iter().zip(expected) {
                let at = format!("{name} {}", got.expiry);
                assert_eq!(got.expiry, want["expiry"].as_str().unwrap(), "{at}");
                assert_eq!(got.atm_strike, opt(&want["atm_strike"]), "{at} atm_strike");
                assert_eq!(got.atm_iv, opt(&want["atm_iv"]), "{at} atm_iv");
                assert_eq!(got.straddle_mid, opt(&want["straddle_mid"]), "{at} straddle_mid");
                assert_eq!(got.implied_move, opt(&want["implied_move"]), "{at} implied_move");
                assert_eq!(got.skew, opt(&want["skew"]), "{at} skew");
                assert_eq!(got.call_volume, want["call_volume"].as_i64().unwrap(), "{at}");
                assert_eq!(got.put_volume, want["put_volume"].as_i64().unwrap(), "{at}");
                assert_eq!(got.call_open_interest, want["call_open_interest"].as_i64().unwrap(), "{at}");
                assert_eq!(got.put_open_interest, want["put_open_interest"].as_i64().unwrap(), "{at}");
                assert_eq!(got.put_call_volume_ratio(), opt(&want["put_call_volume_ratio"]), "{at}");
                assert_eq!(got.put_call_oi_ratio(), opt(&want["put_call_oi_ratio"]), "{at}");
            }
        }
        assert!(n_quotes > 500, "the fixtures hold the real IBM and JNJ chains: {n_quotes}");
    }

    /// A small Cboe reply: two expiries, one adjusted option of another root, one option with no quote.
    fn cboe_reply() -> Vec<u8> {
        serde_json::json!({
            "timestamp": "2026-11-19 17:40:51",
            "data": {
                "symbol": "TEST", "current_price": 100.0,
                "options": [
                    {"option": "TEST261218P00100000", "bid": 4.7, "ask": 4.9, "iv": 0.32, "volume": 60.0, "open_interest": 400.0, "last_trade_price": 4.8},
                    {"option": "TEST261120C00110000", "bid": 0.0, "ask": 0.0, "iv": 0.0, "volume": 0.0, "open_interest": 30.0, "last_trade_price": 1.2},
                    {"option": "TEST261120C00100000", "bid": 4.9, "ask": 5.1, "iv": 0.30, "volume": 120.0, "open_interest": 800.0, "last_trade_price": 5.0},
                    {"option": "TEST261120P00100000", "bid": 4.7, "ask": 4.9, "iv": 0.31, "volume": 40.0, "open_interest": 500.0, "last_trade_price": 4.8},
                    {"option": "B1TEST261120C00100000", "bid": 1.0, "ask": 2.0, "iv": 0.5, "volume": 1.0, "open_interest": 1.0, "last_trade_price": 1.5},
                    {"option": "TEST261120P00097500", "bid": null, "ask": null, "iv": null, "volume": null, "open_interest": null, "last_trade_price": null}
                ]
            }
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn cboe_reply_becomes_a_snapshot_in_the_python_order() {
        let s = snapshot_from_cboe(&cboe_reply(), "test", "2026-11-19").unwrap();
        assert_eq!((s.ticker.as_str(), s.spot_price, s.quotes.len()), ("TEST", Some(100.0), 5));
        let symbols: Vec<&str> = s.quotes.iter().map(|q| q.contract_symbol.as_str()).collect();
        assert_eq!(symbols, ["TEST261120C00100000", "TEST261120C00110000", "TEST261120P00097500", "TEST261120P00100000", "TEST261218P00100000"]);
        let call = &s.quotes[0];
        assert_eq!((call.strike, call.expiry.as_str(), call.volume, call.open_interest), (100.0, "2026-11-20", Some(120), Some(800)));
        assert_eq!((call.last_price, call.bid, call.ask, call.implied_volatility), (Some(5.0), Some(4.9), Some(5.1), Some(0.30)));
        assert_eq!(call.in_the_money, Some(false));
        assert_eq!(s.quotes[2].strike, 97.5);
        assert_eq!(s.quotes[2].in_the_money, Some(false)); // put at 97.5, spot 100
        assert_eq!(s.quotes[2].bid, None);
        assert_eq!(s.quotes[3].in_the_money, Some(false)); // put at 100, spot 100: not in the money
        let m = compute_chain_metrics(&s);
        assert_eq!(m.expiries[0].atm_strike, Some(100.0));
        assert_eq!(m.expiries[0].atm_iv, Some(0.305));
        assert_eq!(m.expiries[0].straddle_mid, Some(9.8));
    }

    #[test]
    fn a_reply_for_another_symbol_is_an_error() {
        let e = snapshot_from_cboe(&cboe_reply(), "OTHER", "2026-11-19").unwrap_err();
        assert!(e.contains("not OTHER"), "{e}");
        assert!(snapshot_from_cboe(b"<html>", "TEST", "2026-11-19").is_err());
    }

    #[test]
    fn persistence_round_trip_listing_and_the_same_day_rule() {
        let dir = std::env::temp_dir().join(format!("fa-options-{}", std::process::id()));
        let mut s = snapshot_from_cboe(&cboe_reply(), "TEST", "2026-11-18").unwrap();
        save_snapshot(&s, &dir).unwrap();
        s.snapshot_date = "2026-11-19".into();
        save_snapshot(&s, &dir).unwrap();
        std::fs::write(options_dir(&dir, "TEST").join("notes.json"), "{}").unwrap();
        assert_eq!(list_snapshots(&dir, "TEST"), ["2026-11-18", "2026-11-19"]);
        assert_eq!(load_snapshot(&dir, "TEST", None).unwrap(), s, "the latest");
        assert_eq!(list_snapshots(&dir, "NONE"), Vec::<String>::new());
        assert!(load_snapshot(&dir, "NONE", None).unwrap_err().contains("No option-chain snapshots"));
        assert!(load_snapshot(&dir, "TEST", Some("1999-01-01")).unwrap_err().contains("on 1999-01-01"), "a date that has no file");
        // Same day: the file is read, and there is no request.
        let never = |_: &str| -> Result<Vec<u8>, String> { panic!("no request expected") };
        let (got, source) = fetch_option_chain(&dir, "test", false, "2026-11-19", &never).unwrap();
        assert_eq!((got, source), (s.clone(), Source::Cache));
        // A new day: one request, and a new file.
        let reply = cboe_reply();
        let fetch = |u: &str| -> Result<Vec<u8>, String> {
            assert!(u.ends_with("/options/TEST.json"), "{u}");
            Ok(reply.clone())
        };
        let (got, source) = fetch_option_chain(&dir, "TEST", false, "2026-11-20", &fetch).unwrap();
        assert_eq!((got.snapshot_date.as_str(), source), ("2026-11-20", Source::Cboe));
        assert!(snapshot_path(&dir, "TEST", "2026-11-20").is_file());
        // A failed request leaves no file.
        let fail = |u: &str| -> Result<Vec<u8>, String> { Err(format!("HTTP 404 for {u}")) };
        assert!(fetch_option_chain(&dir, "TEST", false, "2026-11-21", &fail).unwrap_err().contains("HTTP 404"));
        assert!(!snapshot_path(&dir, "TEST", "2026-11-21").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_python_snapshot_file_is_read() {
        let text = r#"{"ticker": "T", "snapshot_date": "2026-11-19", "spot_price": 1.5, "quotes": [
          {"contract_symbol": "x", "option_type": "call", "expiry": "2026-11-20", "strike": 1.0, "last_price": null, "bid": null,
           "ask": null, "volume": 5, "open_interest": 0, "implied_volatility": null, "in_the_money": true}]}"#;
        let s: OptionChainSnapshot = serde_json::from_str(text).unwrap();
        assert_eq!((s.quotes[0].volume, s.quotes[0].in_the_money, s.quotes[0].last_price), (Some(5), Some(true), None));
    }
}
