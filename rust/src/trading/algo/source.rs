//! The inputs of the engine. `Replay` reads a recorded file or stored bars. `Live` polls a `QuoteFeed`.
//!
//! `QuoteFeed` has one read function. No type here has a function that sends an order.

use super::Input;
use crate::market::alpaca::{self, ny_day, Client, DATA_BASE};
use std::path::PathBuf;
use crate::trading::calendar::{parse_iso, weekday};
use crate::trading::data::{file_name, read_series};
use std::path::Path;

/// The spread that a stored bar gets (USD): a bar has no bid and ask. One cent is the usual SPY spread.
pub const BAR_SPREAD: f64 = 0.01;
/// A live quote older than this (seconds) is stale while the market is open: the engine does not use it.
pub const STALE_SECS: i64 = 10;
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

/// The lines of a recorded input file (`<strategy>-inputs.jsonl`). It refuses an input with a data time after
/// its read time, a bad price, or a read time that goes back (a look-ahead guard).
pub fn read_recorded(path: &Path) -> Result<Vec<Input>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out: Vec<Input> = vec![];
    for (n, l) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let i: Input = serde_json::from_str(l).map_err(|e| format!("{} line {}: {e}", path.display(), n + 1))?;
        i.check().map_err(|e| format!("{} line {}: {e}", path.display(), n + 1))?;
        if out.last().is_some_and(|p| i.read_ms < p.read_ms) {
            return Err(format!("{} line {}: read_ms goes back (inputs out of order)", path.display(), n + 1));
        }
        out.push(i);
    }
    Ok(out)
}

/// The inputs of a list of bars `(open ms, bar minutes, open, close)`. A bar gives two inputs:
/// the CLOSE (kind `bar`, known at open + length) is what the strategy decides on, and the next bar's OPEN
/// (kind `bar_open`, at the open time) is where an order fills. Order: by time, a close before an open
/// of the same time.
pub fn bar_inputs(symbol: &str, bars: &[(i64, i64, f64, f64)]) -> Vec<Input> {
    let mk = |ms: i64, px: f64, kind: &str| Input { quote_ms: ms, read_ms: ms, symbol: symbol.into(), bid: px - BAR_SPREAD / 2.0, ask: px + BAR_SPREAD / 2.0, kind: kind.into() };
    let mut ev: Vec<(i64, u8, Input)> = vec![];
    for &(open_ms, minutes, o, c) in bars {
        ev.push((open_ms, 1, mk(open_ms, o, "bar_open")));
        let close_ms = open_ms + minutes * 60_000;
        ev.push((close_ms, 0, mk(close_ms, c, "bar")));
    }
    ev.sort_by_key(|(t, rank, _)| (*t, *rank));
    ev.into_iter().map(|(_, _, i)| i).collect()
}

/// The bars of one New York date from the store: `<SYMBOL>_1m.csv`, else `<SYMBOL>_5m.csv`.
/// Returns the inputs, the file name used and the bar length in minutes.
pub fn read_stored_bars(store_dir: &Path, symbol: &str, date: &str) -> Result<(Vec<Input>, String, i64), String> {
    let day = parse_iso(date).ok_or_else(|| format!("bad date '{date}' (use YYYY-MM-DD)"))?;
    let mut tried = vec![];
    for (interval, minutes) in [("1m", 1i64), ("5m", 5)] {
        let name = file_name(symbol, interval);
        let path = store_dir.join(&name);
        if !path.exists() {
            continue;
        }
        let s = read_series(&path)?;
        let bars: Vec<(i64, i64, f64, f64)> = (0..s.len()).filter(|&i| s.date[i] == day && s.px.close[i].is_finite() && s.px.open[i].is_finite()).map(|i| (s.utc[i] * 1000, minutes, s.px.open[i], s.px.close[i])).collect();
        if !bars.is_empty() {
            let out = bar_inputs(symbol, &bars);
            for i in &out {
                i.check()?;
            }
            return Ok((out, name, minutes));
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

/// A URL that this module can send: `GET` on the Alpaca DATA host, a stock-data path only. The field is private:
/// the only ways to make one are the functions below. A test asserts the host and the method.
pub struct DataUrl(String);

impl DataUrl {
    pub const HOST: &'static str = DATA_BASE;
    pub const METHOD: &'static str = "GET";
    fn new(path: &str) -> DataUrl {
        assert!(path.starts_with("/v2/stocks/"), "algo sends stock data paths only");
        DataUrl(format!("{}{path}", DataUrl::HOST))
    }
    pub fn latest_quote(symbol: &str, feed: &str) -> DataUrl {
        DataUrl::new(&format!("/v2/stocks/quotes/latest?symbols={symbol}&feed={feed}"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The only client of the algo code. It has one function, a read (`GET`) of a `DataUrl`.
pub struct DataClient {
    client: Client,
}

impl DataClient {
    pub fn new(client: Client) -> DataClient {
        DataClient { client }
    }
    pub fn get(&self, url: &DataUrl, each: impl FnMut(&serde_json::Value) -> Result<(), String>) -> Result<(), String> {
        self.client.paged(url.as_str(), each).map_err(|e| format!("{e:?}"))
    }
    pub fn requests(&self) -> usize {
        self.client.requests()
    }
}

/// The Alpaca latest quote of one stock: `GET /v2/stocks/quotes/latest`.
pub struct AlpacaQuotes {
    client: DataClient,
    symbol: String,
    feed: String,
}

impl AlpacaQuotes {
    pub fn new(client: DataClient, symbol: &str, feed: &str) -> AlpacaQuotes {
        AlpacaQuotes { client, symbol: alpaca::api_symbol(symbol), feed: feed.to_string() }
    }
}

impl QuoteFeed for AlpacaQuotes {
    fn read(&mut self) -> Result<(i64, f64, f64), String> {
        let url = DataUrl::latest_quote(&self.symbol, &self.feed);
        let mut out = Err("no quote in the reply".to_string());
        self.client.get(&url, |page| {
            out = parse_latest_quote(page, &self.symbol);
            Ok(())
        })?;
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
    stale_secs: i64,
    stop_file: PathBuf,
    now_ms: Box<dyn FnMut() -> i64>,
    sleep_ms: Box<dyn FnMut(u64)>,
    started: bool,
    errors: usize,
}

impl<F: QuoteFeed> Live<F> {
    pub fn new(feed: F, symbol: &str, poll_ms: u64, until_secs: i64, stale_secs: i64, stop_file: PathBuf, now_ms: Box<dyn FnMut() -> i64>, sleep_ms: Box<dyn FnMut(u64)>) -> Live<F> {
        Live { feed, symbol: symbol.into(), poll_ms, until_secs, stale_secs, stop_file, now_ms, sleep_ms, started: false, errors: 0 }
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
            if self.stop_file.exists() {
                return Next::End("kill file".into());
            }
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
            let open_ms = (secs - tod + OPEN_SECS) * 1000;
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
                    // A stale quote (a holiday, a halt, or the quote of the close of the day before) is not used.
                    if read_ms - quote_ms > self.stale_secs * 1000 || quote_ms < open_ms {
                        continue;
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

    const T0: i64 = 1_790_000_000_000; // 2026-09-21 10:13:20 New York, a Monday

    fn no_stop() -> PathBuf {
        std::env::temp_dir().join("fa-algo-no-such-stop-file")
    }

    fn live_with(feed: Fake, clock: &Rc<RefCell<i64>>, until: i64, stale: i64, stop: PathBuf) -> Live<Fake> {
        let (c1, c2) = (clock.clone(), clock.clone());
        Live::new(feed, "SPY", 1000, until, stale, stop, Box::new(move || *c1.borrow()), Box::new(move |ms| *c2.borrow_mut() += ms as i64))
    }

    #[test]
    fn live_loop_with_a_fake_feed() {
        let clock = Rc::new(RefCell::new(T0));
        let feed = Fake(vec![Ok((T0 - 40, 99.9, 100.1)), Err("boom".into()), Ok((T0 + 1900, 100.0, 100.2))], 0);
        let mut live = live_with(feed, &clock, 10 * 3600 + 14 * 60, STALE_SECS, no_stop());
        let Next::Input(a) = live.next() else { panic!() };
        assert_eq!((a.quote_ms, a.read_ms, a.kind.as_str()), (T0 - 40, T0, "quote"));
        let Next::Input(b) = live.next() else { panic!() }; // the error is skipped
        assert_eq!(b.read_ms, T0 + 2000);
        assert!(a.check().is_ok() && b.check().is_ok());
        assert_eq!(live.requests(), 3);
        // The clock passes 10:14:00: the source ends.
        *clock.borrow_mut() = T0 + 41_000;
        assert!(matches!(live.next(), Next::End(_)));
    }

    #[test]
    fn live_ends_after_five_errors() {
        let clock = Rc::new(RefCell::new(T0));
        let feed = Fake((0..5).map(|_| Err("x".to_string())).collect(), 0);
        let mut live = live_with(feed, &clock, 16 * 3600, STALE_SECS, no_stop());
        assert!(matches!(live.next(), Next::End(m) if m.contains("5 read errors")));
    }

    #[test]
    fn live_skips_a_quote_older_than_10_seconds_and_a_quote_older_than_the_open() {
        let clock = Rc::new(RefCell::new(T0));
        // 15 s old: stale. The next is 0.5 s old.
        let feed = Fake(vec![Ok((T0 - 15_000, 99.9, 100.1)), Ok((T0 + 500, 99.9, 100.1))], 0);
        let mut live = live_with(feed, &clock, 16 * 3600, STALE_SECS, no_stop());
        let Next::Input(i) = live.next() else { panic!() };
        assert_eq!(i.quote_ms, T0 + 500);
        // With a stale limit of one day, the quote of 08:00 (before the 09:30 open) is still skipped.
        let before_open = T0 - 3 * 3600 * 1000;
        let feed = Fake(vec![Ok((before_open, 99.9, 100.1)), Ok((T0 + 5_500, 99.9, 100.1))], 0);
        let mut live = live_with(feed, &clock, 16 * 3600, 86_400, no_stop());
        let Next::Input(i) = live.next() else { panic!() };
        assert!(i.quote_ms >= T0);
    }

    #[test]
    fn live_checks_the_kill_file_while_it_waits_for_the_open() {
        let path = std::env::temp_dir().join(format!("fa-algo-live-stop-{}", std::process::id()));
        std::fs::write(&path, "stop").unwrap();
        let clock = Rc::new(RefCell::new(T0 - 2 * 3600 * 1000)); // 08:13 New York: before the open
        let mut live = live_with(Fake(vec![], 0), &clock, 16 * 3600, STALE_SECS, path.clone());
        let next = live.next();
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(next, Next::End(m) if m == "kill file"));
        assert_eq!(live.requests(), 0);
    }

    #[test]
    fn the_data_client_sends_get_on_the_data_host_only() {
        assert_eq!(DataUrl::METHOD, "GET");
        let seen: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
        let s2 = seen.clone();
        let get: crate::market::price_source::Get = Box::new(move |url| {
            s2.borrow_mut().push(url.to_string());
            Ok(br#"{"quotes": {"SPY": {"t": "2026-09-21T14:13:20.5Z", "ap": 100.1, "bp": 99.9}}}"#.to_vec())
        });
        let client = Client::new(alpaca::Keys { id: "id".into(), secret: "secret".into() }, get);
        let mut quotes = AlpacaQuotes::new(DataClient::new(client), "SPY", "iex");
        assert_eq!(quotes.read().unwrap(), (1_790_000_000_500, 99.9, 100.1));
        let urls = seen.borrow();
        assert_eq!(urls.len(), 1);
        assert!(urls[0].starts_with("https://data.alpaca.markets/v2/stocks/"), "{}", urls[0]);
        assert!(!urls[0].contains("order") && !urls[0].contains("paper"));
        assert_eq!(quotes.requests(), 1);
        // A path outside the stock data paths panics at once.
        let r = std::panic::catch_unwind(|| DataUrl::new("/v2/account"));
        assert!(r.is_err());
    }

    fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                rs_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    /// The text without quotes, plus signs, commas, brackets and white space: `"a" + "b"` and `concat!("a", "b")` join.
    fn squeeze(text: &str) -> String {
        text.chars().filter(|c| !matches!(c, '"' | '+' | ',' | '(' | ')' | '[' | ']' | ' ' | '\t' | '\n' | '\r')).collect()
    }

    /// Every `*.rs` file under `rust/src`. The words are built from pieces that sit in different statements, so that this
    /// test does not match itself. Allowed: the existing GET-only uses of the trading base in `market/alpaca.rs`
    /// (the contract list) and the host list of `hostnet.rs`, each with the count of its lines; `rates.rs` posts to the BLS.
    #[test]
    fn no_order_code_in_the_repo() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = vec![];
        rs_files(&root, &mut files);
        assert!(files.len() > 100, "the walk found {} files", files.len());
        let w1 = "/v2/";
        let w2 = ["ord", "ers"].concat();
        let order_path = format!("{w1}{w2}");
        let slash_word = format!("/{w2}");
        let quoted_word = format!("\"{w2}\"");
        let h1 = "paper";
        let h2 = "-api";
        let paper_host = format!("{h1}{h2}");
        let t1 = "trading";
        let t2 = "_base";
        let tbase = format!("{t1}{t2}");
        let m1 = "api.alpaca";
        let api_host = format!("{m1}.markets");
        let p1 = "hostnet::";
        let p2 = "post_json";
        let post = format!("{p1}{p2}");
        // (file under src, text, allowed count in the squeezed text)
        let allow = [
            ("hostnet.rs", paper_host.as_str(), 1usize),
            ("market/alpaca.rs", paper_host.as_str(), 2),
            ("market/alpaca.rs", tbase.as_str(), 6),
            ("hostnet.rs", api_host.as_str(), 2),
            ("market/alpaca.rs", api_host.as_str(), 2),
            ("rates.rs", post.as_str(), 1),
        ];
        for f in &files {
            let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            let raw = std::fs::read_to_string(f).unwrap();
            let sq = squeeze(&raw);
            let sq_lower = sq.to_lowercase();
            assert!(!sq.contains(&order_path), "{rel} has an orders endpoint");
            assert!(!raw.contains(&quoted_word) && !sq.contains(&slash_word), "{rel} has an orders path or word");
            for needle in [&paper_host, &tbase, &api_host, &post] {
                let n = sq_lower.matches(&needle.to_lowercase()).count();
                let ok = allow.iter().find(|(file, w, _)| *file == rel && *w == needle.as_str()).map(|a| a.2).unwrap_or(0);
                assert_eq!(n, ok, "{rel}: '{needle}' found {n} times, allowed {ok}");
            }
        }
    }

    #[test]
    fn stored_bars_become_close_and_open_inputs() {
        let dir = std::env::temp_dir().join(format!("fa-algo-bars-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SPY_5m.csv"), "Datetime,open,high,low,close,volume\n2026-09-21 09:30:00-04:00,99,1,1,100,5\n2026-09-21 09:35:00-04:00,102,1,1,101,5\n2026-09-22 09:30:00-04:00,1,1,1,102,5\n").unwrap();
        let (v, name, minutes) = read_stored_bars(&dir, "SPY", "2026-09-21").unwrap();
        assert_eq!((name.as_str(), minutes, v.len()), ("SPY_5m.csv", 5, 4));
        // 09:30 EDT = T0 - 2600 s. Order: open 1, close 1 (09:35) before open 2 (09:35), close 2.
        let t = (1_790_000_000 - 2600) * 1000;
        let shape: Vec<(i64, &str)> = v.iter().map(|i| (i.read_ms, i.kind.as_str())).collect();
        assert_eq!(shape, vec![(t, "bar_open"), (t + 300_000, "bar"), (t + 300_000, "bar_open"), (t + 600_000, "bar")]);
        assert!((v[2].mid() - 102.0).abs() < 1e-9 && (v[1].mid() - 100.0).abs() < 1e-9);
        // No input has a data time after its read time.
        assert!(v.iter().all(|i| i.check().is_ok() && i.quote_ms <= i.read_ms));
        assert!(read_stored_bars(&dir, "SPY", "2026-01-05").unwrap_err().contains("no bars"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_look_ahead_guard_refuses_a_planted_read_time_and_a_disorder() {
        // A close bar with read_ms = the bar OPEN: its data (the close) is after its read time.
        let bars = [(T0, 5, 100.0, 101.0)];
        let mut v = bar_inputs("SPY", &bars);
        let close = v.iter_mut().find(|i| i.kind == "bar").unwrap();
        close.read_ms = T0;
        assert!(close.check().unwrap_err().contains("after"));
        let dir = std::env::temp_dir().join(format!("fa-algo-rec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let line = |i: &Input| serde_json::to_string(i).unwrap();
        let good = bar_inputs("SPY", &bars);
        let p = dir.join("planted.jsonl");
        std::fs::write(&p, v.iter().map(line).collect::<Vec<_>>().join("\n")).unwrap();
        assert!(read_recorded(&p).unwrap_err().contains("after"));
        let p2 = dir.join("order.jsonl");
        std::fs::write(&p2, [line(&good[1]), line(&good[0])].join("\n")).unwrap();
        assert!(read_recorded(&p2).unwrap_err().contains("out of order"));
        let p3 = dir.join("good.jsonl");
        std::fs::write(&p3, good.iter().map(line).collect::<Vec<_>>().join("\n")).unwrap();
        assert_eq!(read_recorded(&p3).unwrap(), good);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
