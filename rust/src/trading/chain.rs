//! Live option chains for the paper trader: the container of the quotes of one underlying.
//!
//! The Cboe delayed-quote client is removed: the Cboe page prohibits automatic downloads (ticket
//! T-20261121-cboe-off). `fa trade paper` needs bid, ask, implied volatility and delta of each option. The
//! Alpaca indicative chain (`market::alpaca`) is a different feed (modified quotes), and the paper trader
//! has no reviewed adapter for it. So `Chains::get` refuses, and `fa trade paper` refuses to run with a live
//! chain (use `--no-live`). The `Chain` type and the option name parser stay: the paper trader prices with a
//! chain that a caller inserts, and `market::options` and `market::option_history` use `parse_option`.

use super::calendar::{days_from_civil, Date};
use super::pricing::Quote;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveQuote {
    pub bid: f64,
    pub ask: f64,
    pub iv: f64,
    pub delta: f64,
    pub volume: f64,
    pub open_interest: f64,
}

/// All quotes of one underlying.
#[derive(Debug)]
pub struct Chain {
    pub symbol: String,
    /// The time stamp of the quotes (UTC), as the source gives it.
    pub timestamp: String,
    /// The last price of the underlying, as the source gives it.
    pub spot: Option<f64>,
    pub last_trade_time: Option<String>,
    /// Key: expiry, call, strike in thousandths of a dollar.
    quotes: HashMap<(Date, bool, i64), LiveQuote>,
}

/// `SPY261120C00728000`: root, expiry YYMMDD, C or P, strike in thousandths. `None` for any other
/// root (for example `B1SPY...`, the adjusted options) or a bad text.
pub(crate) fn parse_option(text: &str, root: &str) -> Option<(Date, bool, i64)> {
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
    /// A chain from its parts (the caller owns the source). `timestamp` is a text for the report.
    pub fn new(symbol: &str, timestamp: &str, spot: Option<f64>, last_trade_time: Option<String>, quotes: HashMap<(Date, bool, i64), LiveQuote>) -> Chain {
        Chain { symbol: symbol.to_string(), timestamp: timestamp.to_string(), spot, last_trade_time, quotes }
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

/// The chains of one run. No chain is downloaded: `get` refuses (T-20261121-cboe-off). A test or a later source inserts chains.
#[derive(Default)]
pub struct Chains {
    cache: HashMap<String, Arc<Chain>>,
}

/// The message of the refusal of a live chain.
pub const REFUSED: &str = "no live option chain: the Cboe delayed quotes are not read (the Cboe page prohibits automatic downloads) and the Alpaca chain has no adapter for the paper trader yet (T-20261121-cboe-off); use --no-live";

impl Chains {
    /// Put a chain in (the tests use this).
    pub fn insert(&mut self, chain: Chain) {
        self.cache.insert(chain.symbol.clone(), Arc::new(chain));
    }

    /// The chain of the symbol if it was inserted. No request is made.
    pub fn get(&mut self, symbol: &str) -> Result<Arc<Chain>, String> {
        self.cache.get(symbol).cloned().ok_or_else(|| REFUSED.to_string())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Two expiries of SPY, one option with no ask, one with a missing field (a hand-made fixture, no Cboe reply).
    pub fn fixture() -> Chain {
        let (d1, d2) = (days_from_civil(2026, 11, 20), days_from_civil(2026, 11, 27));
        let lq = |bid: f64, ask: f64, iv: f64, delta: f64, volume: f64, oi: f64| LiveQuote { bid, ask, iv, delta, volume, open_interest: oi };
        let nan = f64::NAN;
        let mut q = HashMap::new();
        q.insert((d1, true, 736_000), lq(3.1, 3.2, 0.15, 0.48, 5000.0, 100.0));
        q.insert((d1, true, 737_000), lq(2.5, 2.6, 0.15, 0.4, nan, nan));
        q.insert((d1, false, 735_000), lq(2.0, 2.1, 0.16, -0.45, nan, nan));
        q.insert((d1, false, 730_000), lq(0.0, 0.0, 0.0, 0.0, nan, nan));
        q.insert((d1, false, 725_000), lq(0.0, 0.03, 0.2, -0.05, nan, nan));
        q.insert((d2, true, 736_500), lq(4.0, 4.2, nan, nan, nan, nan));
        Chain::new("SPY", "2026-11-19 15:10:00", Some(735.5), Some("2026-11-19T10:09:58".into()), q)
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
        let c = fixture();
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
    fn a_live_chain_is_refused_and_no_address_is_built() {
        let mut chains = Chains::default();
        let e = chains.get("SPY").unwrap_err();
        assert!(e.contains("T-20261121-cboe-off") && e.contains("--no-live"), "{e}");
        chains.insert(fixture());
        assert_eq!(chains.get("SPY").unwrap().len(), 6);
    }
}
