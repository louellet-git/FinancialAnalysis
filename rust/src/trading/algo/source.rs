//! The inputs of the engine. `Replay` reads a recorded file or stored bars. `Live` polls a `QuoteFeed`.
//!
//! `QuoteFeed` has one read function. No type here has a function that sends an order.

use super::Input;
use crate::market::alpaca::{self, ny_day, Client, DATA_BASE};
use crate::trading::calendar::{parse_iso, weekday};
use crate::trading::data::{file_name, read_series};
use std::path::Path;

/// The spread that a stored bar gets (USD): a bar has no bid and ask. One cent is the usual SPY spread.
pub const BAR_SPREAD: f64 = 0.01;
/// A live quote older than this (seconds) is stale: the engine does not use it.
pub const STALE_SECS: i64 = 300;
/// 09:30 New York, in seconds of the day.
pub const OPEN_SECS: i64 = 9 * 3600 + 30 * 60;

pub enum Next {
    Input(Input),
    End(String),
}

pub trait Source {
    fn next(&mut self) -> Next;
}

// ---------------------------------------------------------------- replay

pub struct Replay {
    inputs: std::vec::IntoIter<Input>,
}

impl Replay {
    pub fn new(inputs: Vec<Input>) -> Replay {
        Replay { inputs: inputs.into_iter() }
    }
}

impl Source for Replay {
    fn next(&mut self) -> Next {
        match self.inputs.next() {
            Some(i) => Next::Input(i),
            None => Next::End("end of input".into()),
        }
    }
}

/// The lines of a recorded input file (`<strategy>-inputs.jsonl`).
pub fn read_recorded(path: &Path) -> Result<Vec<Input>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.lines().filter(|l| !l.trim().is_empty()).enumerate().map(|(n, l)| serde_json::from_str(l).map_err(|e| format!("{} line {}: {e}", path.display(), n + 1))).collect()
}

/// The bars of one New York date from the store: `<SYMBOL>_1m.csv`, else `<SYMBOL>_5m.csv`. A bar becomes known at
/// its close (open + the bar length): that is its time. Returns the inputs and the file name used.
pub fn read_stored_bars(store_dir: &Path, symbol: &str, date: &str) -> Result<(Vec<Input>, String), String> {
    let day = parse_iso(date).ok_or_else(|| format!("bad date '{date}' (use YYYY-MM-DD)"))?;
    let mut tried = vec![];
    for (interval, minutes) in [("1m", 1i64), ("5m", 5)] {
        let name = file_name(symbol, interval);
        let path = store_dir.join(&name);
        if !path.exists() {
            continue;
        }
        let s = read_series(&path)?;
        let out: Vec<Input> = (0..s.len())
            .filter(|&i| s.date[i] == day && s.px.close[i].is_finite())
            .map(|i| {
                let ms = (s.utc[i] + minutes * 60) * 1000;
                let c = s.px.close[i];
                Input { quote_ms: ms, read_ms: ms, symbol: symbol.into(), bid: c - BAR_SPREAD / 2.0, ask: c + BAR_SPREAD / 2.0, kind: "bar".into() }
            })
            .collect();
        if !out.is_empty() {
            return Ok((out, name));
        }
        let (a, b) = (s.date.first().map(|d| crate::trading::calendar::iso(*d)), s.date.last().map(|d| crate::trading::calendar::iso(*d)));
        tried.push(format!("{name} has no bars on {date} (it covers {} to {})", a.unwrap_or_default(), b.unwrap_or_default()));
    }
    if tried.is_empty() {
        tried.push(format!("no {} or {} in {}", file_name(symbol, "1m"), file_name(symbol, "5m"), store_dir.display()));
    }
    Err(tried.join("; "))
}

// ---------------------------------------------------------------- live

/// One read of the latest quote. It is a read function only.
pub trait QuoteFeed {
    /// `(quote time in ms, bid, ask)`.
    fn read(&mut self) -> Result<(i64, f64, f64), String>;
    /// Requests sent so far (for the request rate).
    fn requests(&self) -> usize;
}

/// The Alpaca latest quote of one stock: `GET /v2/stocks/quotes/latest` through the read function `Client::paged`.
pub struct AlpacaQuotes {
    client: Client,
    symbol: String,
    feed: String,
}

impl AlpacaQuotes {
    pub fn new(client: Client, symbol: &str, feed: &str) -> AlpacaQuotes {
        AlpacaQuotes { client, symbol: alpaca::api_symbol(symbol), feed: feed.to_string() }
    }
}

impl QuoteFeed for AlpacaQuotes {
    fn read(&mut self) -> Result<(i64, f64, f64), String> {
        let url = format!("{DATA_BASE}/v2/stocks/quotes/latest?symbols={}&feed={}", self.symbol, self.feed);
        let mut out = Err("no quote in the reply".to_string());
        self.client.paged(&url, |page| {
            out = parse_latest_quote(page, &self.symbol);
            Ok(())
        })
        .map_err(|e| format!("{e:?}"))?;
        out
    }
    fn requests(&self) -> usize {
        self.client.requests()
    }
}

/// `{"quotes": {"SPY": {"t": "...Z", "ap": 1.0, "bp": 1.0, ...}}}`. Documentation: `GET /v2/stocks/quotes/latest`.
pub fn parse_latest_quote(page: &serde_json::Value, symbol: &str) -> Result<(i64, f64, f64), String> {
    let q = &page["quotes"][symbol];
    let (Some(t), Some(ap), Some(bp)) = (q["t"].as_str(), q["ap"].as_f64(), q["bp"].as_f64()) else {
        return Err(match page["message"].as_str() {
            Some(m) => format!("Alpaca error: {}", m.chars().take(200).collect::<String>()),
            None => format!("no quote for {symbol} in the reply"),
        });
    };
    let secs = alpaca::parse_rfc3339(t).ok_or_else(|| format!("bad quote time '{t}'"))?;
    // The fraction of a second, in ms.
    let frac = t.split_once('.').map(|(_, r)| r.chars().take_while(|c| c.is_ascii_digit()).take(3).collect::<String>()).unwrap_or_default();
    let ms = if frac.is_empty() { 0 } else { format!("{frac:0<3}").parse::<i64>().unwrap_or(0) };
    if !(bp > 0.0 && ap >= bp) {
        return Err(format!("bad quote: bid {bp}, ask {ap}"));
    }
    Ok((secs * 1000 + ms, bp, ap))
}

pub struct Live<F: QuoteFeed> {
    feed: F,
    symbol: String,
    poll_ms: u64,
    until_secs: i64,
    now_ms: Box<dyn FnMut() -> i64>,
    sleep_ms: Box<dyn FnMut(u64)>,
    started: bool,
    errors: usize,
}

impl<F: QuoteFeed> Live<F> {
    pub fn new(feed: F, symbol: &str, poll_ms: u64, until_secs: i64, now_ms: Box<dyn FnMut() -> i64>, sleep_ms: Box<dyn FnMut(u64)>) -> Live<F> {
        Live { feed, symbol: symbol.into(), poll_ms, until_secs, now_ms, sleep_ms, started: false, errors: 0 }
    }
    pub fn requests(&self) -> usize {
        self.feed.requests()
    }
}

impl<F: QuoteFeed> Source for Live<F> {
    fn next(&mut self) -> Next {
        loop {
            if self.started {
                (self.sleep_ms)(self.poll_ms);
            }
            self.started = true;
            let now = (self.now_ms)();
            let secs = now.div_euclid(1000);
            let (date, tod) = ny_day(secs);
            if parse_iso(&date).is_some_and(|d| weekday(d) >= 5) {
                return Next::End("market closed (weekend)".into());
            }
            if tod >= self.until_secs {
                return Next::End("end time".into());
            }
            if tod < OPEN_SECS {
                continue; // wait for the open
            }
            match self.feed.read() {
                Err(e) => {
                    self.errors += 1;
                    if self.errors >= 5 {
                        return Next::End(format!("5 read errors in a row, the last: {e}"));
                    }
                }
                Ok((quote_ms, bid, ask)) => {
                    self.errors = 0;
                    let read_ms = (self.now_ms)();
                    if read_ms - quote_ms > STALE_SECS * 1000 {
                        continue; // a stale quote (a holiday, or a halt)
                    }
                    return Next::Input(Input { quote_ms, read_ms, symbol: self.symbol.clone(), bid, ask, kind: "quote".into() });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn parses_a_documented_quote_reply() {
        let page = serde_json::json!({"quotes": {"SPY": {"t": "2026-09-21T14:13:20.123456789Z", "ap": 100.1, "as": 1, "bp": 99.9, "bs": 2}}});
        let (ms, bid, ask) = parse_latest_quote(&page, "SPY").unwrap();
        assert_eq!((ms, bid, ask), (1_790_000_000_123, 99.9, 100.1));
        assert!(parse_latest_quote(&serde_json::json!({"message": "forbidden"}), "SPY").unwrap_err().contains("forbidden"));
        let crossed = serde_json::json!({"quotes": {"SPY": {"t": "2026-09-21T14:13:20Z", "ap": 99.0, "bp": 99.9}}});
        assert!(parse_latest_quote(&crossed, "SPY").is_err());
    }

    struct Fake(Vec<Result<(i64, f64, f64), String>>, usize);
    impl QuoteFeed for Fake {
        fn read(&mut self) -> Result<(i64, f64, f64), String> {
            self.1 += 1;
            self.0.remove(0)
        }
        fn requests(&self) -> usize {
            self.1
        }
    }

    const T0: i64 = 1_790_000_000_000; // 10:13:20 New York, a Monday

    #[test]
    fn live_loop_with_a_fake_feed() {
        let clock = Rc::new(RefCell::new(T0));
        let (c1, c2) = (clock.clone(), clock.clone());
        let feed = Fake(vec![Ok((T0 - 40, 99.9, 100.1)), Err("boom".into()), Ok((T0 + 1900, 100.0, 100.2))], 0);
        let mut live = Live::new(feed, "SPY", 1000, 10 * 3600 + 14 * 60, Box::new(move || *c1.borrow()), Box::new(move |ms| *c2.borrow_mut() += ms as i64));
        let Next::Input(a) = live.next() else { panic!() };
        assert_eq!((a.quote_ms, a.read_ms, a.kind.as_str()), (T0 - 40, T0, "quote"));
        let Next::Input(b) = live.next() else { panic!() }; // the error is skipped
        assert_eq!(b.read_ms, T0 + 2000);
        assert_eq!(live.requests(), 3);
        // The clock passes 10:14:00: the source ends.
        *clock.borrow_mut() = T0 + 41_000;
        assert!(matches!(live.next(), Next::End(_)));
    }

    #[test]
    fn live_ends_after_five_errors_and_skips_stale_quotes() {
        let clock = Rc::new(RefCell::new(T0));
        let (c1, c2) = (clock.clone(), clock.clone());
        let feed = Fake((0..5).map(|_| Err("x".to_string())).collect(), 0);
        let mut live = Live::new(feed, "SPY", 1000, 16 * 3600, Box::new(move || *c1.borrow()), Box::new(move |ms| *c2.borrow_mut() += ms as i64));
        assert!(matches!(live.next(), Next::End(m) if m.contains("5 read errors")));
        let feed = Fake(vec![Ok((T0 - 400_000, 99.9, 100.1)), Ok((T0 + 500, 99.9, 100.1))], 0);
        let (c1, c2) = (clock.clone(), clock.clone());
        let mut live = Live::new(feed, "SPY", 1000, 16 * 3600, Box::new(move || *c1.borrow()), Box::new(move |ms| *c2.borrow_mut() += ms as i64));
        let Next::Input(i) = live.next() else { panic!() };
        assert_eq!(i.quote_ms, T0 + 500);
    }

    #[test]
    fn no_order_code_in_the_engine() {
        // The words are built from pieces so that this test does not match itself.
        let banned = [["/v2/", "orders"].concat(), ["paper", "-api"].concat(), ["trading", "_base"].concat(), ["TRADING", "_BASE"].concat(), ["post", "_json"].concat(), ["\"PO", "ST\""].concat(), ["Method", "::Post"].concat()];
        let files = [("mod.rs", include_str!("mod.rs")), ("cli.rs", include_str!("cli.rs")), ("demo.rs", include_str!("demo.rs")), ("engine.rs", include_str!("engine.rs")), ("source.rs", include_str!("source.rs"))];
        for (name, text) in files {
            for word in &banned {
                assert!(!text.contains(word.as_str()), "{name} has '{word}'");
            }
        }
    }

    #[test]
    fn stored_bars_become_inputs_at_their_close() {
        let dir = std::env::temp_dir().join(format!("fa-algo-bars-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SPY_5m.csv"), "Datetime,open,high,low,close,volume\n2026-09-21 09:30:00-04:00,1,1,1,100,5\n2026-09-21 09:35:00-04:00,1,1,1,101,5\n2026-09-22 09:30:00-04:00,1,1,1,102,5\n").unwrap();
        let (v, name) = read_stored_bars(&dir, "SPY", "2026-09-21").unwrap();
        assert_eq!(name, "SPY_5m.csv");
        assert_eq!(v.len(), 2);
        // 09:30 EDT = 13:30 UTC = T0 - 2600 s; the bar closes 300 s later.
        assert_eq!(v[0].quote_ms, (1_790_000_000 - 2300) * 1000);
        assert!((v[1].mid() - 101.0).abs() < 1e-9);
        assert!(read_stored_bars(&dir, "SPY", "2026-01-05").unwrap_err().contains("no bars"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
