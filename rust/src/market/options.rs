//! Option chain snapshots and their metrics (the Rust form of `options.py`).
//!
//! The live source is the Alpaca option chain (`options_cli`, `alpaca`). The Cboe delayed quotes are not
//! read any more: the Cboe page prohibits automatic downloads (ticket T-20261121-cboe-off). The files
//! of older Cboe pulls stay in the store; they have no `source` field and the snapshot format reads them.
//!
//! A pull is a dated snapshot, `data/raw/<T>/options/<YYYY-MM-DD>.json` (Cboe, old) or
//! `<YYYY-MM-DD>T<hhmm>.json` (Alpaca, with the time). The format is the format of the Python code.
//! The metrics (`compute_chain_metrics`) are the same calculation as in Python: fixtures from the Python
//! functions are in `rust/tests/fixtures/market/options_metrics.json`.

use super::{lenient_json, round6, ticker_dir, write_atomic};
use crate::trading::calendar::iso;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// Where the quotes came from: `alpaca` or `alpaca-history`. A file with no value is an old Cboe delayed snapshot (no new one is made).
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

/// The stem of the file that `load_snapshot_at` reads: `<date>T<time>`, or the last stem of the date (`<date>` if none exists).
pub fn stem_of(cache: &Path, ticker: &str, date: &str, time: Option<&str>) -> Result<String, String> {
    match time {
        Some(t) if !valid_hhmm(t) => Err(format!("bad --time '{t}' (use HHMM in UTC, for example 1930)")),
        Some(t) => Ok(format!("{date}T{t}")),
        None => Ok(stems_of_date(cache, ticker, date).pop().unwrap_or_else(|| date.to_string())),
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
    let stem = stem_of(cache, ticker, &date, time)?;
    let path = files.get(&stem).cloned().unwrap_or_else(|| snapshot_path(cache, ticker, &stem));
    let text = read_text(&path).map_err(|e| format!("No option-chain snapshot for {} on {date}{} at {}: {e}", ticker.to_uppercase(), time.map(|t| format!(" at {t} UTC")).unwrap_or_default(), path.display()))?;
    serde_json::from_str(&lenient_json(&text)).map_err(|e| format!("{}: {e}", path.display()))
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

    fn q(symbol: &str, call: bool, expiry: &str, strike: f64, (bid, ask, iv): (Option<f64>, Option<f64>, Option<f64>), (volume, oi): (Option<i64>, Option<i64>), last: Option<f64>) -> OptionQuote {
        OptionQuote { contract_symbol: symbol.into(), option_type: if call { "call" } else { "put" }.into(), expiry: expiry.into(), strike, last_price: last, bid, ask, volume, open_interest: oi, implied_volatility: iv, in_the_money: Some(if call { strike < 100.0 } else { strike > 100.0 }) }
    }

    /// A small snapshot: two expiries, one option with no quote (a hand-made fixture, no Cboe reply).
    fn small_snapshot(date: &str) -> OptionChainSnapshot {
        let quotes = vec![
            q("TEST261120C00100000", true, "2026-11-20", 100.0, (Some(4.9), Some(5.1), Some(0.30)), (Some(120), Some(800)), Some(5.0)),
            q("TEST261120C00110000", true, "2026-11-20", 110.0, (Some(0.0), Some(0.0), Some(0.0)), (Some(0), Some(30)), Some(1.2)),
            q("TEST261120P00097500", false, "2026-11-20", 97.5, (None, None, None), (None, None), None),
            q("TEST261120P00100000", false, "2026-11-20", 100.0, (Some(4.7), Some(4.9), Some(0.31)), (Some(40), Some(500)), Some(4.8)),
            q("TEST261218P00100000", false, "2026-12-18", 100.0, (Some(4.7), Some(4.9), Some(0.32)), (Some(60), Some(400)), Some(4.8)),
        ];
        OptionChainSnapshot { ticker: "TEST".into(), snapshot_date: date.into(), spot_price: Some(100.0), quotes, source: None, feed: None, quote_kind: None, note: None, snapshot_time: None }
    }

    #[test]
    fn metrics_of_a_small_snapshot() {
        let s = small_snapshot("2026-11-19");
        let m = compute_chain_metrics(&s);
        assert_eq!(m.expiries[0].atm_strike, Some(100.0));
        assert_eq!(m.expiries[0].atm_iv, Some(0.305));
        assert_eq!(m.expiries[0].straddle_mid, Some(9.8));
    }

    #[test]
    fn persistence_round_trip_and_listing() {
        let dir = std::env::temp_dir().join(format!("fa-options-{}", std::process::id()));
        let mut s = small_snapshot("2026-11-18");
        save_snapshot(&s, &dir).unwrap();
        s.snapshot_date = "2026-11-19".into();
        save_snapshot(&s, &dir).unwrap();
        std::fs::write(options_dir(&dir, "TEST").join("notes.json"), "{}").unwrap();
        assert_eq!(list_snapshots(&dir, "TEST"), ["2026-11-18", "2026-11-19"]);
        assert_eq!(load_snapshot(&dir, "TEST", None).unwrap(), s, "the latest");
        assert_eq!(list_snapshots(&dir, "NONE"), Vec::<String>::new());
        assert!(load_snapshot(&dir, "NONE", None).unwrap_err().contains("No option-chain snapshots"));
        assert!(load_snapshot(&dir, "TEST", Some("1999-01-01")).unwrap_err().contains("on 1999-01-01"), "a date that has no file");
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

    #[test]
    fn a_timed_alpaca_file_has_its_own_name() {
        let dir = std::env::temp_dir().join(format!("fa-options-timed-{}", std::process::id()));
        let mut s = small_snapshot("2026-11-21");
        s.source = Some("alpaca".into());
        s.snapshot_time = Some("2026-11-21T15:27Z".into());
        let timed = save_snapshot(&s, &dir).unwrap();
        assert!(timed.to_string_lossy().ends_with("2026-11-21T1527.json"), "{}", timed.display());
        std::fs::remove_dir_all(&dir).ok();
    }
}
