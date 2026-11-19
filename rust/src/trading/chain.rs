//! Live option chains for the paper trader: the Cboe delayed quotes (JSON of one underlying).
//!
//! Source: `https://cdn-api.cboe.com/api/global/delayed_quotes/options/{SYMBOL}.json`. It is the
//! file that the delayed-quotes pages of cboe.com read; it is free, it needs no cookie and no key,
//! and it answers the honest client of `fa futures`. It is not a published API: Cboe can change
//! it. The quotes are delayed (about 15 minutes during the session) and one file holds all
//! expiries of one underlying (SPY: about 12,000 options, 5.7 MB). The caller fetches one file for
//! each underlying in each run.
//!
//! Yahoo is no source here: its option endpoint needs a cookie and a crumb, and it answers the
//! honest client with HTTP 429 when it asks for the crumb (tested 2026-11-19).

use super::calendar::{days_from_civil, Date};
use super::pricing::Quote;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

pub const URL: &str = "https://cdn-api.cboe.com/api/global/delayed_quotes/options/{symbol}.json";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveQuote {
    pub bid: f64,
    pub ask: f64,
    pub iv: f64,
    pub delta: f64,
    pub volume: f64,
    pub open_interest: f64,
}

#[derive(Debug, Deserialize)]
struct Reply {
    timestamp: Option<String>,
    data: Data,
}

#[derive(Debug, Deserialize)]
struct Data {
    symbol: String,
    current_price: Option<f64>,
    last_trade_time: Option<String>,
    options: Vec<Opt>,
}

#[derive(Debug, Deserialize)]
struct Opt {
    option: String,
    bid: Option<f64>,
    ask: Option<f64>,
    iv: Option<f64>,
    delta: Option<f64>,
    volume: Option<f64>,
    open_interest: Option<f64>,
}

/// All quotes of one underlying.
#[derive(Debug)]
pub struct Chain {
    pub symbol: String,
    /// The time stamp of the file, as Cboe gives it. It is UTC (checked 2026-11-19: the file of 15:43:13 came at 15:43:58 UTC).
    /// The last trade times in the file are New York time, and they are about 15 minutes older.
    pub timestamp: String,
    /// The last price of the underlying, as Cboe gives it (delayed).
    pub spot: Option<f64>,
    pub last_trade_time: Option<String>,
    /// Key: expiry, call, strike in thousandths of a dollar.
    quotes: HashMap<(Date, bool, i64), LiveQuote>,
}

/// `SPY261120C00728000`: root, expiry YYMMDD, C or P, strike in thousandths. `None` for any other
/// root (for example `B1SPY...`, the adjusted options) or a bad text.
fn parse_option(text: &str, root: &str) -> Option<(Date, bool, i64)> {
    let rest = text.strip_prefix(root)?;
    let b = rest.as_bytes();
    if b.len() != 15 || !b.iter().enumerate().all(|(i, c)| if i == 6 { *c == b'C' || *c == b'P' } else { c.is_ascii_digit() }) {
        return None;
    }
    let n = |a: usize, z: usize| rest[a..z].parse::<i64>().ok();
    let date = days_from_civil(2000 + n(0, 2)? as i32, n(2, 4)? as u32, n(4, 6)? as u32);
    Some((date, b[6] == b'C', n(7, 15)?))
}

impl Chain {
    pub fn parse(bytes: &[u8], symbol: &str) -> Result<Chain, String> {
        let reply: Reply = serde_json::from_slice(bytes).map_err(|e| format!("Cboe reply for {symbol} is not the expected JSON: {e}"))?;
        if !reply.data.symbol.eq_ignore_ascii_case(symbol) {
            return Err(format!("Cboe reply is for {}, not {symbol}", reply.data.symbol));
        }
        let mut quotes = HashMap::new();
        for o in &reply.data.options {
            let Some(key) = parse_option(&o.option, symbol) else { continue };
            let v = |x: Option<f64>| x.filter(|y| y.is_finite()).unwrap_or(f64::NAN);
            quotes.insert(key, LiveQuote { bid: v(o.bid), ask: v(o.ask), iv: v(o.iv), delta: v(o.delta), volume: v(o.volume), open_interest: v(o.open_interest) });
        }
        if quotes.is_empty() {
            return Err(format!("Cboe reply for {symbol} has no option of that root"));
        }
        Ok(Chain {
            symbol: symbol.to_string(),
            timestamp: reply.timestamp.unwrap_or_default(),
            spot: reply.data.current_price,
            last_trade_time: reply.data.last_trade_time,
            quotes,
        })
    }

    /// Text for the report: the time of the file and the last trade of the underlying (the quotes are
    /// about 15 minutes late).
    pub fn age(&self) -> String {
        format!("file {} UTC, underlying last trade {} New York time", self.timestamp, self.last_trade_time.as_deref().unwrap_or("unknown"))
    }

    pub fn len(&self) -> usize {
        self.quotes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.quotes.is_empty()
    }

    /// The raw quote of one option, if the chain lists it.
    pub fn get(&self, call: bool, strike: f64, expiry: Date) -> Option<&LiveQuote> {
        self.quotes.get(&(expiry, call, (strike * 1000.0).round() as i64))
    }

    /// A usable quote: the chain lists the option with `bid >= 0`, `ask > 0` and `ask >= bid`.
    /// Otherwise `None` (the caller uses the model price: a quote with no ask is not a price).
    pub fn quote(&self, call: bool, strike: f64, expiry: Date) -> Option<Quote> {
        let q = self.get(call, strike, expiry)?;
        if !(q.bid >= 0.0 && q.ask > 0.0 && q.ask >= q.bid) {
            return None;
        }
        Some(Quote { bid: q.bid, ask: q.ask, mid: 0.5 * (q.bid + q.ask), iv: if q.iv.is_nan() { 0.0 } else { q.iv }, delta: if q.delta.is_nan() { 0.0 } else { q.delta } })
    }
}

/// The chains of one run: each underlying is downloaded one time.
#[derive(Default)]
pub struct Chains {
    cache: HashMap<String, Result<Arc<Chain>, String>>,
    downloads: usize,
}

/// Seconds between two chain downloads (each file is 4 to 6 MB; Cboe answered HTTP 429 to a second
/// download 0.5 s after the first one, tested 2026-11-19).
const PAUSE_SECONDS: f64 = 3.0;

impl Chains {
    /// Put a chain in (the tests use this; the command downloads).
    pub fn insert(&mut self, chain: Chain) {
        self.cache.insert(chain.symbol.clone(), Ok(Arc::new(chain)));
    }

    pub fn get(&mut self, symbol: &str) -> Result<Arc<Chain>, String> {
        if let Some(hit) = self.cache.get(symbol) {
            return hit.clone();
        }
        if self.downloads > 0 {
            std::thread::sleep(std::time::Duration::from_secs_f64(PAUSE_SECONDS));
        }
        self.downloads += 1;
        let loaded = crate::futures::download(&URL.replace("{symbol}", symbol)).and_then(|bytes| Chain::parse(&bytes, symbol)).map(Arc::new);
        self.cache.insert(symbol.to_string(), loaded.clone());
        loaded
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Two expiries of SPY, one adjusted option (`B1SPY`), one option with no ask, one with a missing field.
    pub fn fixture() -> String {
        r#"{"timestamp":"2026-11-19 15:10:00","data":{"symbol":"SPY","current_price":735.5,"last_trade_time":"2026-11-19T10:09:58","options":[
          {"option":"SPY261120C00736000","bid":3.1,"bid_size":10,"ask":3.2,"iv":0.15,"open_interest":100,"volume":5000,"delta":0.48},
          {"option":"SPY261120C00737000","bid":2.5,"ask":2.6,"iv":0.15,"delta":0.4},
          {"option":"SPY261120P00735000","bid":2.0,"ask":2.1,"iv":0.16,"delta":-0.45},
          {"option":"SPY261120P00730000","bid":0.0,"ask":0.0,"iv":0.0,"delta":0.0},
          {"option":"SPY261120P00725000","bid":0.0,"ask":0.03,"iv":0.2,"delta":-0.05},
          {"option":"SPY261127C00736500","bid":4.0,"ask":4.2},
          {"option":"B1SPY261120C00736000","bid":9.0,"ask":9.5}]}}"#
            .to_string()
    }

    #[test]
    fn parses_the_option_names() {
        assert_eq!(parse_option("SPY261120C00728000", "SPY"), Some((days_from_civil(2026, 11, 20), true, 728_000)));
        assert_eq!(parse_option("SPY261120P00728500", "SPY"), Some((days_from_civil(2026, 11, 20), false, 728_500)));
        assert_eq!(parse_option("B1SPY261120C00728000", "SPY"), None);
        assert_eq!(parse_option("SPYG261120C00728000", "SPY"), None);
        assert_eq!(parse_option("SPY261120X00728000", "SPY"), None);
        assert_eq!(parse_option("SPY2611", "SPY"), None);
    }

    #[test]
    fn quotes_come_from_the_chain_and_bad_ones_do_not() {
        let c = Chain::parse(fixture().as_bytes(), "SPY").unwrap();
        // The adjusted option is not in the chain.
        assert_eq!(c.len(), 6);
        assert_eq!((c.spot, c.timestamp.as_str()), (Some(735.5), "2026-11-19 15:10:00"));
        let exp = days_from_civil(2026, 11, 20);
        let q = c.quote(true, 736.0, exp).unwrap();
        assert_eq!((q.bid, q.ask, q.iv, q.delta), (3.1, 3.2, 0.15, 0.48));
        assert!((q.mid - 3.15).abs() < 1e-12);
        // A strike with decimals matches in thousandths.
        assert!(c.get(true, 736.5, days_from_civil(2026, 11, 27)).is_some());
        // The put with no ask is no price. A put at bid 0 and ask 0.03 is a price. The missing strike and expiry are none.
        assert!(c.get(false, 730.0, exp).is_some() && c.quote(false, 730.0, exp).is_none());
        assert_eq!(c.quote(false, 725.0, exp).unwrap().bid, 0.0);
        assert!(c.quote(true, 999.0, exp).is_none());
        assert!(c.quote(true, 736.0, exp + 7).is_none());
        // A field that is missing is not a failure; the quote still has bid and ask.
        let q = c.quote(true, 736.5, days_from_civil(2026, 11, 27)).unwrap();
        assert_eq!((q.bid, q.ask, q.iv), (4.0, 4.2, 0.0));
    }

    #[test]
    fn rejects_other_replies() {
        assert!(Chain::parse(b"<html>", "SPY").unwrap_err().contains("not the expected JSON"));
        assert!(Chain::parse(fixture().as_bytes(), "QQQ").unwrap_err().contains("not QQQ"));
        let empty = r#"{"data":{"symbol":"SPY","options":[]}}"#;
        assert!(Chain::parse(empty.as_bytes(), "SPY").unwrap_err().contains("no option"));
    }
}
