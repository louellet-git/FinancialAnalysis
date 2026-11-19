//! The price cache: `data/raw/<T>/prices.json` and the record `prices.meta.json` beside it.
//!
//! `prices.json` keeps the form that the older readers use (a list of `date, open, high, low,
//! close, volume`). The record says where the rows came from:
//!
//! - `source`: tiingo, yahoo or nasdaq.
//! - `adjusted`: `splits+dividends` (Tiingo, Yahoo) or `splits` (Nasdaq). The closes of two sources
//!   with a different `adjusted` are not one series: the cache never mixes them. A new source
//!   replaces the whole file or does not touch it.
//! - `updated`: the date of the last successful update.
//!
//! A cache file with no record (written by the older loader, `yfinance` rules) is Yahoo with
//! `splits+dividends`. A record that does not agree with the file (another program changed the
//! file) is ignored for the same reason.
//!
//! Update rules (`update_ticker`):
//! 1. A ticker that was updated today is current.
//! 2. The source of the cache comes first. It gives the days from 10 calendar days before the last
//!    row. Its rows replace the cached rows of these days (a source can correct its last days).
//! 3. If the rows of the overlap do not agree with the cache (largest relative difference of the
//!    close above 0.02%), a split or a dividend changed the adjusted history: the program gets the
//!    full history again.
//! 4. A full history replaces the file only if it has at least 90% of the cached rows. A source
//!    that fails never changes or shortens the file.
//! 5. On HTTP 429 the program waits one time, tries again, and then stops that source for the run.

use super::price_source::{add_days, FetchError, PriceSource, Row};
use super::prices::{read_cache, PricePoint};
use super::{parse_date, ticker_dir, write_atomic};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A difference of the closes of the overlap above this is a change of the adjusted history.
pub const OVERLAP_TOLERANCE: f64 = 2e-4;
/// Calendar days before the last cached row where an update starts (more than 5 trading days).
pub const OVERLAP_DAYS: i64 = 10;
/// A last close older than this many weekdays gets a warning.
pub const STALE_WEEKDAYS: i64 = 5;
pub const ADJUSTED_ALL: &str = "splits+dividends";
pub const ADJUSTED_SPLITS: &str = "splits";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub source: String,
    pub adjusted: String,
    pub updated: String,
    pub first_date: String,
    pub last_date: String,
    pub rows: usize,
}

pub fn prices_path(cache: &Path, ticker: &str) -> PathBuf {
    ticker_dir(cache, ticker).join("prices.json")
}

pub fn meta_path(cache: &Path, ticker: &str) -> PathBuf {
    ticker_dir(cache, ticker).join("prices.meta.json")
}

fn meta_for(rows: &[PricePoint], source: &str, adjusted: &str, updated: &str) -> Meta {
    Meta {
        source: source.into(),
        adjusted: adjusted.into(),
        updated: updated.into(),
        first_date: rows.first().map(|r| r.date.clone()).unwrap_or_default(),
        last_date: rows.last().map(|r| r.date.clone()).unwrap_or_default(),
        rows: rows.len(),
    }
}

/// The record, if it agrees with the rows of the file.
fn valid_meta(cache: &Path, ticker: &str, rows: &[PricePoint]) -> Option<Meta> {
    let m: Meta = serde_json::from_str(&std::fs::read_to_string(meta_path(cache, ticker)).ok()?).ok()?;
    let agrees = m.rows == rows.len() && rows.last().is_some_and(|r| r.date == m.last_date) && rows.first().is_some_and(|r| r.date == m.first_date);
    agrees.then_some(m)
}

/// The history of a ticker (as the older readers see it), if there is a cache file.
pub fn history(cache: &Path, ticker: &str) -> Option<Vec<PricePoint>> {
    let path = prices_path(cache, ticker);
    path.is_file().then(|| read_cache(&path).ok()).flatten()
}

#[derive(Debug, Clone, PartialEq)]
pub struct LastClose {
    pub date: String,
    pub close: f64,
    /// The source of the rows, or `cache (no source record)`.
    pub source: String,
}

/// The last close of the cache: date, close and source.
pub fn last_close(cache: &Path, ticker: &str) -> Option<LastClose> {
    let rows = history(cache, ticker)?;
    let last = rows.iter().rev().find(|r| r.close.is_finite() && r.close > 0.0)?;
    let source = valid_meta(cache, ticker, &rows).map_or_else(|| "cache (no source record)".to_string(), |m| m.source);
    Some(LastClose { date: last.date.clone(), close: last.close, source })
}

/// Weekdays after `from` up to and including `to` (holidays are not counted out: the age is an upper bound).
pub fn weekdays_between(from: &str, to: &str) -> Option<i64> {
    let (a, b) = (parse_date(from)?, parse_date(to)?);
    Some((a + 1..=b).filter(|d| (d + 3).rem_euclid(7) < 5).count() as i64)
}

/// The row as the cache keeps it. With `dividends` the prices are dividend adjusted (each of open, high and low is
/// multiplied by `adj_close / close`, as `yfinance` does); a source with no `adj_close` keeps its split adjusted prices.
pub fn to_point(r: &Row, dividends: bool) -> PricePoint {
    let ratio = match (dividends, r.adj_close) {
        (true, Some(a)) if r.close > 0.0 => a / r.close,
        _ => 1.0,
    };
    let scale = |x: f64| super::round6(x * ratio);
    PricePoint { date: r.date.clone(), open: scale(r.open), high: scale(r.high), low: scale(r.low), close: if ratio == 1.0 { r.close } else { super::round6(r.adj_close.unwrap_or(r.close)) }, volume: r.volume }
}

/// The largest relative difference of the closes on the dates that are in both lists from `from` on, and the number of such dates.
pub fn overlap_difference(old: &[PricePoint], new: &[PricePoint], from: &str) -> (usize, f64) {
    let map: std::collections::HashMap<&str, f64> = old.iter().filter(|p| p.date.as_str() >= from).map(|p| (p.date.as_str(), p.close)).collect();
    let (mut n, mut worst) = (0, 0.0f64);
    for p in new {
        if let Some(&c) = map.get(p.date.as_str()) {
            n += 1;
            worst = worst.max(((p.close - c) / c).abs());
        }
    }
    (n, worst)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Current,
    Updated { source: String, added: usize, full: bool, note: String },
    Failed(String),
}

pub struct Options {
    pub today: String,
    /// Get the full history again.
    pub full: bool,
    /// The person named the source: it is the only one that is asked, and it may replace a cache of another source.
    pub forced: bool,
    /// The wait after HTTP 429, before the one new try.
    pub retry_wait: Duration,
}

/// What the run knows: the sources that answered HTTP 429 twice.
#[derive(Default)]
pub struct Run {
    pub blocked: BTreeSet<String>,
}

enum Attempt {
    RateLimited(String),
    Failed(String),
}

fn fetch(src: &dyn PriceSource, ticker: &str, from: Option<&str>, opt: &Options, run: &mut Run) -> Result<Vec<Row>, Attempt> {
    if run.blocked.contains(src.name()) {
        return Err(Attempt::Failed(format!("{} was stopped for this run (HTTP 429)", src.name())));
    }
    let mut tries = 0;
    loop {
        match src.fetch(ticker, from, &opt.today) {
            Ok(rows) => return Ok(rows),
            Err(FetchError::RateLimited(m)) => {
                tries += 1;
                if tries >= 2 {
                    run.blocked.insert(src.name().to_string());
                    return Err(Attempt::RateLimited(format!("{m} (waited once, then stopped {} for this run; the cache is unchanged)", src.name())));
                }
                eprintln!("fa: {} answered HTTP 429; waiting {} s before one new try", src.name(), opt.retry_wait.as_secs());
                std::thread::sleep(opt.retry_wait);
            }
            Err(FetchError::Failed(m)) => return Err(Attempt::Failed(m)),
        }
    }
}

type Existing = Option<(Vec<PricePoint>, Meta)>;

fn load_existing(cache: &Path, ticker: &str) -> Existing {
    let rows = history(cache, ticker).filter(|r| !r.is_empty())?;
    let meta = valid_meta(cache, ticker, &rows).unwrap_or_else(|| Meta { source: "yahoo".into(), adjusted: ADJUSTED_ALL.into(), updated: String::new(), ..meta_for(&rows, "yahoo", ADJUSTED_ALL, "") });
    Some((rows, meta))
}

fn save(cache: &Path, ticker: &str, rows: &[PricePoint], meta: &Meta) -> Result<(), String> {
    write_atomic(&prices_path(cache, ticker), &serde_json::to_string(rows).map_err(|e| e.to_string())?)?;
    write_atomic(&meta_path(cache, ticker), &serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?)
}

fn attempt(src: &dyn PriceSource, ticker: &str, existing: &Existing, opt: &Options, run: &mut Run) -> Result<(Vec<PricePoint>, usize, bool, String), Attempt> {
    let dividends = src.adjusts_dividends();
    let adjusted = if dividends { ADJUSTED_ALL } else { ADJUSTED_SPLITS };
    let mut reason = String::from("no cache");
    if let Some((old, meta)) = existing {
        if !opt.full && meta.source == src.name() && meta.adjusted == adjusted {
            let last = old.last().map(|p| p.date.clone()).unwrap_or_default();
            if let Some(from) = add_days(&last, -OVERLAP_DAYS) {
                let new: Vec<PricePoint> = fetch(src, ticker, Some(&from), opt, run)?.iter().map(|r| to_point(r, dividends)).collect();
                if new.is_empty() {
                    return Ok((old.clone(), 0, false, "the source gave no new rows".into()));
                }
                let (n, worst) = overlap_difference(old, &new, &from);
                if n > 0 && worst <= OVERLAP_TOLERANCE {
                    let mut merged: Vec<PricePoint> = old.iter().filter(|p| p.date.as_str() < from.as_str()).cloned().collect();
                    merged.extend(new);
                    merged.dedup_by(|b, a| a.date == b.date);
                    let added = merged.len().saturating_sub(old.len());
                    return Ok((merged, added, false, format!("{n} overlap days agree (largest difference {:.4}%)", worst * 100.0)));
                }
                reason = if n == 0 { "no common day in the overlap".into() } else { format!("the overlap differs by {:.3}% (a split or a dividend)", worst * 100.0) };
            }
        } else if meta.source != src.name() || meta.adjusted != adjusted {
            reason = format!("the cache is from {} ({}); the series are not mixed", meta.source, meta.adjusted);
        } else {
            reason = "full history asked".into();
        }
    }
    let new: Vec<PricePoint> = fetch(src, ticker, None, opt, run)?.iter().map(|r| to_point(r, dividends)).collect();
    if new.is_empty() {
        return Err(Attempt::Failed("the source gave no rows".into()));
    }
    if let Some((old, _)) = existing {
        if new.len() * 10 < old.len() * 9 {
            return Err(Attempt::Failed(format!("the full history has {} rows; the cache has {}: the cache stays", new.len(), old.len())));
        }
    }
    let added = new.len().saturating_sub(existing.as_ref().map_or(0, |(o, _)| o.len()));
    Ok((new, added, true, reason))
}

/// Update the cache of one ticker from the sources (in the order of use). The cache source goes first
/// unless the person forced a source or asked for the full history.
pub fn update_ticker(cache: &Path, ticker: &str, sources: &[Box<dyn PriceSource>], opt: &Options, run: &mut Run) -> Status {
    let existing = load_existing(cache, ticker);
    if !opt.full && !opt.forced {
        if let Some((_, m)) = &existing {
            if m.updated == opt.today {
                return Status::Current;
            }
        }
    }
    let mut order: Vec<&dyn PriceSource> = sources.iter().map(|s| s.as_ref()).collect();
    if !opt.full && !opt.forced {
        if let Some((_, m)) = &existing {
            order.sort_by_key(|s| s.name() != m.source);
        }
    }
    let mut reasons = Vec::new();
    for src in order {
        match attempt(src, ticker, &existing, opt, run) {
            Ok((rows, added, full, note)) => {
                let meta = meta_for(&rows, src.name(), if src.adjusts_dividends() { ADJUSTED_ALL } else { ADJUSTED_SPLITS }, &opt.today);
                return match save(cache, ticker, &rows, &meta) {
                    Ok(()) => Status::Updated { source: src.name().into(), added, full, note },
                    Err(e) => Status::Failed(e),
                };
            }
            Err(Attempt::RateLimited(m)) | Err(Attempt::Failed(m)) => reasons.push(format!("{}: {m}", src.name())),
        }
    }
    Status::Failed(reasons.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A source with canned rows. `calls` records `(from)` of each request.
    struct Fake {
        name: &'static str,
        dividends: bool,
        rows: Rc<RefCell<Vec<Row>>>,
        calls: Rc<RefCell<Vec<Option<String>>>>,
        error: Rc<RefCell<Option<FetchError>>>,
    }

    impl PriceSource for Fake {
        fn name(&self) -> &'static str {
            self.name
        }
        fn adjusts_dividends(&self) -> bool {
            self.dividends
        }
        fn fetch(&self, _t: &str, from: Option<&str>, _to: &str) -> Result<Vec<Row>, FetchError> {
            self.calls.borrow_mut().push(from.map(String::from));
            if let Some(e) = self.error.borrow().clone() {
                return Err(e);
            }
            Ok(self.rows.borrow().iter().filter(|r| from.is_none_or(|f| r.date.as_str() >= f)).cloned().collect())
        }
    }

    struct Rig {
        rows: Rc<RefCell<Vec<Row>>>,
        calls: Rc<RefCell<Vec<Option<String>>>>,
        error: Rc<RefCell<Option<FetchError>>>,
    }

    fn fake(name: &'static str, dividends: bool, rows: Vec<Row>) -> (Box<dyn PriceSource>, Rig) {
        let rig = Rig { rows: Rc::new(RefCell::new(rows)), calls: Rc::new(RefCell::new(vec![])), error: Rc::new(RefCell::new(None)) };
        (Box::new(Fake { name, dividends, rows: rig.rows.clone(), calls: rig.calls.clone(), error: rig.error.clone() }), rig)
    }

    fn row(date: &str, close: f64) -> Row {
        Row { date: date.into(), open: close, high: close, low: close, close, volume: 100, adj_close: Some(close), dividend: None, split: None }
    }

    /// Weekdays of January 2024 from the 1st (a Monday), `n` rows, close 100 + day index.
    fn series(n: usize) -> Vec<Row> {
        (0..n).map(|i| row(&mapcheck::dates::format(parse_date("2024-01-01").unwrap() + i as i64), 100.0 + i as f64)).collect()
    }

    fn opt(today: &str) -> Options {
        Options { today: today.into(), full: false, forced: false, retry_wait: Duration::ZERO }
    }

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fa_pc_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn first_load_then_incremental_with_corrected_last_days() {
        let d = dir("incr");
        let (src, rig) = fake("yahoo", true, series(30));
        let srcs = vec![src];
        let mut run = Run::default();
        let s = update_ticker(&d, "tst", &srcs, &opt("2024-01-30"), &mut run);
        assert!(matches!(s, Status::Updated { full: true, added: 30, .. }), "{s:?}");
        assert_eq!(rig.calls.borrow()[0], None);
        assert_eq!(last_close(&d, "TST").unwrap().date, "2024-01-30");
        assert_eq!(last_close(&d, "TST").unwrap().source, "yahoo");
        // Updated today: current, no request.
        assert_eq!(update_ticker(&d, "TST", &srcs, &opt("2024-01-30"), &mut run), Status::Current);
        assert_eq!(rig.calls.borrow().len(), 1);
        // Two new days; the last cached day is corrected by 0.01%: an incremental update, the correction is taken.
        {
            let mut rows = rig.rows.borrow_mut();
            rows.push(row("2024-01-31", 130.0));
            rows.push(row("2024-02-01", 131.0));
            let l = rows.len() - 3;
            rows[l].close = 128.99 * 1.0001;
            rows[l].adj_close = Some(128.99 * 1.0001);
        }
        let s = update_ticker(&d, "TST", &srcs, &opt("2024-02-02"), &mut run);
        assert!(matches!(s, Status::Updated { full: false, added: 2, .. }), "{s:?}");
        assert_eq!(rig.calls.borrow()[1].as_deref(), Some("2024-01-20"), "10 days before the last row");
        let h = history(&d, "TST").unwrap();
        assert_eq!(h.len(), 32);
        assert!((h[29].close - 128.99 * 1.0001).abs() < 1e-5, "the corrected day replaced the old row");
        assert_eq!(valid_meta(&d, "TST", &h).unwrap().updated, "2024-02-02");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn split_or_dividend_in_the_overlap_gets_the_full_history() {
        let d = dir("split");
        let (src, rig) = fake("yahoo", true, series(30));
        let srcs = vec![src];
        let mut run = Run::default();
        update_ticker(&d, "TST", &srcs, &opt("2024-01-30"), &mut run);
        // A 2-for-1 split: the whole adjusted history is half.
        {
            let mut rows = rig.rows.borrow_mut();
            for r in rows.iter_mut() {
                r.close /= 2.0;
                r.open /= 2.0;
                r.high /= 2.0;
                r.low /= 2.0;
                r.adj_close = Some(r.close);
            }
        }
        let s = update_ticker(&d, "TST", &srcs, &opt("2024-01-31"), &mut run);
        match &s {
            Status::Updated { full: true, note, .. } => assert!(note.contains("differs"), "{note}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(rig.calls.borrow().len(), 3, "incremental request, then the full request");
        assert_eq!(history(&d, "TST").unwrap()[0].close, 50.0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn failed_source_keeps_the_cache_and_429_stops_the_source() {
        let d = dir("fail");
        let (src, rig) = fake("yahoo", true, series(30));
        let srcs = vec![src];
        let mut run = Run::default();
        update_ticker(&d, "TST", &srcs, &opt("2024-01-30"), &mut run);
        let before = std::fs::read_to_string(prices_path(&d, "TST")).unwrap();
        *rig.error.borrow_mut() = Some(FetchError::RateLimited("HTTP 429 for x".into()));
        let s = update_ticker(&d, "TST", &srcs, &opt("2024-02-05"), &mut run);
        assert!(matches!(&s, Status::Failed(m) if m.contains("429") && m.contains("cache is unchanged")), "{s:?}");
        assert_eq!(rig.calls.borrow().len(), 3, "one request, one new try, then no more");
        assert!(run.blocked.contains("yahoo"));
        // The next ticker does not ask the stopped source.
        let s = update_ticker(&d, "TST", &srcs, &opt("2024-02-05"), &mut run);
        assert!(matches!(s, Status::Failed(_)));
        assert_eq!(rig.calls.borrow().len(), 3);
        assert_eq!(std::fs::read_to_string(prices_path(&d, "TST")).unwrap(), before);
        // A plain error and an empty reply also keep the file.
        let mut run = Run::default();
        *rig.error.borrow_mut() = Some(FetchError::Failed("boom".into()));
        assert!(matches!(update_ticker(&d, "TST", &srcs, &opt("2024-02-06"), &mut run), Status::Failed(m) if m.contains("boom")));
        *rig.error.borrow_mut() = None;
        rig.rows.borrow_mut().clear();
        let mut o = opt("2024-02-06");
        o.full = true;
        assert!(matches!(update_ticker(&d, "TST", &srcs, &o, &mut run), Status::Failed(_)));
        assert_eq!(std::fs::read_to_string(prices_path(&d, "TST")).unwrap(), before);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn second_source_does_not_shorten_or_mix_the_cache() {
        let d = dir("mix");
        let (yahoo, _) = fake("yahoo", true, series(100));
        let mut run = Run::default();
        update_ticker(&d, "TST", &[yahoo], &opt("2024-04-10"), &mut run);
        // Yahoo fails; Nasdaq (splits only) has 20 rows: it does not replace 100 rows.
        let (y2, y2rig) = fake("yahoo", true, vec![]);
        *y2rig.error.borrow_mut() = Some(FetchError::Failed("down".into()));
        let (nas, _) = fake("nasdaq", false, series(20));
        let s = update_ticker(&d, "TST", &[y2, nas], &opt("2024-04-11"), &mut run);
        assert!(matches!(&s, Status::Failed(m) if m.contains("down") && m.contains("the cache stays")), "{s:?}");
        assert_eq!(history(&d, "TST").unwrap().len(), 100);
        assert_eq!(last_close(&d, "TST").unwrap().source, "yahoo");
        // A forced source with as many rows replaces the file; the record then says nasdaq and splits.
        let (nas, _) = fake("nasdaq", false, series(100));
        let mut o = opt("2024-04-11");
        o.forced = true;
        let s = update_ticker(&d, "TST", &[nas], &o, &mut run);
        assert!(matches!(&s, Status::Updated { source, full: true, note, .. } if source == "nasdaq" && note.contains("not mixed")), "{s:?}");
        let h = history(&d, "TST").unwrap();
        assert_eq!(valid_meta(&d, "TST", &h).unwrap().adjusted, ADJUSTED_SPLITS);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn legacy_cache_without_record_is_yahoo_and_a_record_that_disagrees_is_ignored() {
        let d = dir("legacy");
        std::fs::create_dir_all(d.join("OLD")).unwrap();
        let rows: Vec<PricePoint> = series(30).iter().map(|r| to_point(r, true)).collect();
        std::fs::write(prices_path(&d, "OLD"), serde_json::to_string(&rows).unwrap()).unwrap();
        assert_eq!(last_close(&d, "OLD").unwrap().source, "cache (no source record)");
        let (y, rig) = fake("yahoo", true, series(35));
        let s = update_ticker(&d, "OLD", &[y], &opt("2024-02-05"), &mut Run::default());
        assert!(matches!(s, Status::Updated { full: false, added: 5, .. }), "{s:?}");
        assert!(rig.calls.borrow()[0].is_some());
        // Another program wrote more rows: the record is stale.
        let mut more = history(&d, "OLD").unwrap();
        more.push(PricePoint { date: "2024-03-01".into(), open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: 1 });
        std::fs::write(prices_path(&d, "OLD"), serde_json::to_string(&more).unwrap()).unwrap();
        assert_eq!(last_close(&d, "OLD").unwrap().source, "cache (no source record)");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn conversion_and_age() {
        let mut r = row("2024-01-02", 100.0);
        r.adj_close = Some(90.0);
        r.open = 102.0;
        let p = to_point(&r, true);
        assert_eq!((p.close, p.open), (90.0, 91.8));
        let p = to_point(&r, false);
        assert_eq!((p.close, p.open), (100.0, 102.0));
        assert_eq!(weekdays_between("2024-01-05", "2024-01-08"), Some(1), "Friday to Monday");
        assert_eq!(weekdays_between("2024-01-02", "2024-01-02"), Some(0));
        assert_eq!(weekdays_between("2024-01-02", "2024-01-12"), Some(8));
    }

    #[test]
    fn a_thousand_tickers_update_in_seconds_without_network() {
        let d = dir("speed");
        let t0 = std::time::Instant::now();
        let (src, _) = fake("yahoo", true, series(260));
        let srcs = vec![src];
        let mut run = Run::default();
        for i in 0..1000 {
            let s = update_ticker(&d, &format!("T{i}"), &srcs, &opt("2024-09-30"), &mut run);
            assert!(matches!(s, Status::Updated { .. }));
        }
        assert!(t0.elapsed().as_secs() < 20, "{:?}", t0.elapsed());
        let _ = std::fs::remove_dir_all(&d);
    }
}
