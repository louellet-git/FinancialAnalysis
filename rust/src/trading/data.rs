//! Market data for the backtest, read from the CSV cache (`fetch` writes it).
//!
//! The cache directory is `$TRADING_DATA_DIR`, else `<repo>/data/raw/trading/`. This module
//! never downloads data (`fetch` does). Each CSV file is read one time into arrays, and every timestamp is
//! turned into integers one time, so the engine indexes by position only.
//!
//! Time rules (same as the Python code): all times are New York wall time. A daily bar is at
//! 00:00 and trades from 09:30 to 16:00. An hourly or 5-minute bar starts at its timestamp and
//! ends at the next bar time, but not after 16:00.

use super::calendar::{days_from_civil, Date};
use super::pricing::CLOSE_MINUTE;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const DEFAULT_RATE: f64 = 0.04;
/// Minute of the day of the session open (09:30).
pub const OPEN_MINUTE: i32 = 9 * 60 + 30;
const FFILL_LIMIT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interval {
    Daily,
    Hourly,
    FiveMinute,
}

impl Interval {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "1d" => Some(Interval::Daily),
            "1h" => Some(Interval::Hourly),
            "5m" => Some(Interval::FiveMinute),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Interval::Daily => "1d",
            Interval::Hourly => "1h",
            Interval::FiveMinute => "5m",
        }
    }

    /// Minutes in one intraday bar (0 for a daily bar).
    pub fn bar_minutes(self) -> i32 {
        match self {
            Interval::Daily => 0,
            Interval::Hourly => 60,
            Interval::FiveMinute => 5,
        }
    }

    /// Minute of the day when the bar starts trading.
    pub fn open_minute(self, bar_minute: i32) -> i32 {
        if self == Interval::Daily {
            OPEN_MINUTE
        } else {
            bar_minute
        }
    }

    /// Minute of the day when the bar ends.
    pub fn close_minute(self, bar_minute: i32) -> i32 {
        if self == Interval::Daily {
            CLOSE_MINUTE
        } else {
            (bar_minute + self.bar_minutes()).min(CLOSE_MINUTE)
        }
    }
}

/// Bars of one symbol as parallel arrays.
#[derive(Debug, Clone, Default)]
pub struct Ohlcv {
    pub open: Vec<f64>,
    pub high: Vec<f64>,
    pub low: Vec<f64>,
    pub close: Vec<f64>,
    pub volume: Vec<f64>,
}

impl Ohlcv {
    fn take(&self, rows: &[usize]) -> Ohlcv {
        let pick = |v: &Vec<f64>| rows.iter().map(|&r| v[r]).collect::<Vec<f64>>();
        Ohlcv { open: pick(&self.open), high: pick(&self.high), low: pick(&self.low), close: pick(&self.close), volume: pick(&self.volume) }
    }
}

/// One CSV file.
#[derive(Debug, Default)]
pub struct Series {
    pub date: Vec<Date>,
    pub minute: Vec<i32>,
    /// Seconds since 1970-01-01 00:00 UTC.
    pub utc: Vec<i64>,
    pub px: Ohlcv,
}

impl Series {
    pub fn len(&self) -> usize {
        self.date.len()
    }

    pub fn is_empty(&self) -> bool {
        self.date.is_empty()
    }

    fn key(&self, i: usize) -> i64 {
        key(self.date[i], self.minute[i])
    }
}

fn key(date: Date, minute: i32) -> i64 {
    date as i64 * 1440 + minute as i64
}

/// Parse `YYYY-MM-DD HH:MM:SS-05:00` into (date, minute of the day, UTC seconds).
pub(super) fn parse_stamp(s: &str) -> Result<(Date, i32, i64), String> {
    let bad = || format!("bad timestamp '{s}'");
    let b = s.as_bytes();
    if b.len() < 25 || b[4] != b'-' || b[7] != b'-' || b[13] != b':' {
        return Err(bad());
    }
    let n = |a: usize, z: usize| -> Result<i32, String> { s.get(a..z).and_then(|t| t.parse().ok()).ok_or_else(bad) };
    let date = days_from_civil(n(0, 4)?, n(5, 7)? as u32, n(8, 10)? as u32);
    let minute = n(11, 13)? * 60 + n(14, 16)?;
    let sign = match b[19] {
        b'-' => -1,
        b'+' => 1,
        _ => return Err(bad()),
    };
    let offset = sign * (n(20, 22)? * 3600 + n(23, 25)? * 60);
    let utc = date as i64 * 86_400 + minute as i64 * 60 - offset as i64;
    Ok((date, minute, utc))
}

pub fn read_series(path: &Path) -> Result<Series, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut s = Series::default();
    for (n, line) in text.lines().enumerate().skip(1) {
        if line.is_empty() {
            continue;
        }
        let mut f = line.split(',');
        let stamp = f.next().unwrap_or("");
        let (d, m, u) = parse_stamp(stamp).map_err(|e| format!("{} line {}: {e}", path.display(), n + 1))?;
        let mut v = [f64::NAN; 5];
        for slot in v.iter_mut() {
            *slot = match f.next() {
                Some("") | None => f64::NAN,
                Some(x) => x.parse::<f64>().map_err(|_| format!("{} line {}: bad number '{x}'", path.display(), n + 1))?,
            };
        }
        s.date.push(d);
        s.minute.push(m);
        s.utc.push(u);
        s.px.open.push(v[0]);
        s.px.high.push(v[1]);
        s.px.low.push(v[2]);
        s.px.close.push(v[3]);
        s.px.volume.push(v[4]);
    }
    Ok(s)
}

/// The name of the cache file of one symbol and interval, for example `_VIX_1h.csv`.
pub fn file_name(symbol: &str, interval: &str) -> String {
    format!("{}_{}.csv", symbol.replace('^', "_"), interval)
}

/// The CSV cache. Each file is read one time and shared between threads.
pub struct Store {
    dir: PathBuf,
    files: Mutex<HashMap<String, Option<Arc<Series>>>>,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Store { dir, files: Mutex::new(HashMap::new()) }
    }

    /// `$TRADING_DATA_DIR`, else `<repo>/data/raw/trading`.
    pub fn from_env(repo: &Path) -> Self {
        let dir = std::env::var_os("TRADING_DATA_DIR").map(PathBuf::from).unwrap_or_else(|| repo.join("data/raw/trading"));
        Store::new(dir)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// `None` when the file does not exist.
    pub fn get(&self, symbol: &str, interval: &str) -> Result<Option<Arc<Series>>, String> {
        let name = file_name(symbol, interval);
        if let Some(hit) = self.files.lock().expect("cache lock").get(&name) {
            return Ok(hit.clone());
        }
        let path = self.dir.join(&name);
        let loaded = if path.exists() { Some(Arc::new(read_series(&path)?)) } else { None };
        self.files.lock().expect("cache lock").insert(name, loaded.clone());
        Ok(loaded)
    }

    fn need(&self, symbol: &str, interval: &str) -> Result<Arc<Series>, String> {
        self.get(symbol, interval)?
            .ok_or_else(|| format!("no cache file for {symbol} {interval} in {} (run `fa trade fetch`)", self.dir.display()))
    }
}

/// Daily bars with their dates (the full history, or the aligned bars for a daily bot).
#[derive(Debug, Clone, Default)]
pub struct DailyBars {
    pub date: Vec<Date>,
    pub px: Ohlcv,
}

/// Vol per bar as decimals (0.18 is 18%): `atm_*` is the 30-day index, `short_*` is based on the 1-day index.
#[derive(Debug, Clone, Default)]
pub struct VolFrame {
    pub atm_open: Vec<f64>,
    pub atm_close: Vec<f64>,
    pub short_open: Vec<f64>,
    pub short_close: Vec<f64>,
}

/// Everything a backtest needs, aligned on one bar index. Symbols are in the order the bot asks.
#[derive(Debug)]
pub struct MarketData {
    pub interval: Interval,
    pub symbols: Vec<&'static str>,
    pub date: Vec<Date>,
    pub minute: Vec<i32>,
    pub utc: Vec<i64>,
    pub bars: Vec<Ohlcv>,
    pub vol: Vec<VolFrame>,
    /// Annualized short rate for each bar (the prior-day ^IRX close, so no lookahead).
    pub rate: Vec<f64>,
    pub daily: Vec<DailyBars>,
}

impl MarketData {
    pub fn len(&self) -> usize {
        self.date.len()
    }

    pub fn is_empty(&self) -> bool {
        self.date.is_empty()
    }

    pub fn sym_index(&self, symbol: &str) -> usize {
        self.symbols.iter().position(|s| *s == symbol).unwrap_or_else(|| panic!("symbol {symbol} is not in this market data"))
    }
}

fn vol_symbol(symbol: &str) -> &'static str {
    if symbol == "QQQ" {
        "^VXN"
    } else {
        "^VIX"
    }
}

/// For each bar, the PREVIOUS daily row's close (looked up by the bar date), NaN when missing.
fn daily_prev_close(daily: &Series, dates: &[Date]) -> Vec<f64> {
    dates
        .iter()
        .map(|d| match daily.date.binary_search(d) {
            Ok(r) if r > 0 => daily.px.close[r - 1],
            _ => f64::NAN,
        })
        .collect()
}

/// Rows of `src` at the exact time of each bar (NaN where the bar time is missing).
fn reindex(src: &Series, keys: &[i64], col: &[f64]) -> Vec<f64> {
    let mut out = vec![f64::NAN; keys.len()];
    let mut j = 0;
    for (k, &want) in keys.iter().enumerate() {
        while j < src.len() && src.key(j) < want {
            j += 1;
        }
        if j < src.len() && src.key(j) == want {
            out[k] = col[j];
        }
    }
    out
}

/// pandas `ffill(limit)`: copy the last value into at most `limit` NaN rows in a row.
fn ffill(v: &mut [f64], limit: usize) {
    let mut last = f64::NAN;
    let mut run = 0;
    for x in v.iter_mut() {
        if x.is_nan() {
            if !last.is_nan() && run < limit {
                *x = last;
                run += 1;
            }
        } else {
            last = *x;
            run = 0;
        }
    }
}

fn bfill(v: &mut [f64]) {
    let mut next = f64::NAN;
    for x in v.iter_mut().rev() {
        if x.is_nan() {
            *x = next;
        } else {
            next = *x;
        }
    }
}

/// Open and close vol for each bar as decimals. An intraday series that has no value at a bar
/// falls back to the previous daily close.
fn vol_frame(store: &Store, dates: &[Date], keys: &[i64], interval: Interval, vol_sym: &str) -> Result<(Vec<f64>, Vec<f64>), String> {
    let daily = store.need(vol_sym, "1d")?;
    let (mut open, mut close) = if interval == Interval::Daily {
        (reindex(&daily, keys, &daily.px.open), reindex(&daily, keys, &daily.px.close))
    } else {
        match store.get(vol_sym, interval.label())? {
            Some(intr) if !intr.is_empty() => {
                let mut o = reindex(&intr, keys, &intr.px.open);
                let mut c = reindex(&intr, keys, &intr.px.close);
                ffill(&mut o, FFILL_LIMIT);
                ffill(&mut c, FFILL_LIMIT);
                (o, c)
            }
            _ => (vec![f64::NAN; keys.len()], vec![f64::NAN; keys.len()]),
        }
    };
    let fallback = daily_prev_close(&daily, dates);
    for k in 0..keys.len() {
        if open[k].is_nan() {
            open[k] = fallback[k];
        }
        if close[k].is_nan() {
            close[k] = fallback[k];
        }
        open[k] /= 100.0;
        close[k] /= 100.0;
    }
    Ok((open, close))
}

/// Load bars, vol and rates for `symbols` on one bar index: the bars all symbols have in
/// common, from `start` (a date) to `end` (a date, inclusive).
pub fn load_market(store: &Store, symbols: &[&'static str], interval: Interval, start: Option<Date>, end: Option<Date>) -> Result<MarketData, String> {
    let series: Vec<Arc<Series>> = symbols.iter().map(|s| store.need(s, interval.label())).collect::<Result<_, _>>()?;
    // Common bar times (all series are sorted, with no duplicates).
    let mut common: Vec<i64> = (0..series[0].len()).map(|i| series[0].key(i)).collect();
    for s in &series[1..] {
        let mut keep = Vec::new();
        let mut j = 0;
        for &k in &common {
            while j < s.len() && s.key(j) < k {
                j += 1;
            }
            if j < s.len() && s.key(j) == k {
                keep.push(k);
            }
        }
        common = keep;
    }
    if let Some(d) = start {
        common.retain(|&k| k >= key(d, 0));
    }
    if let Some(d) = end {
        common.retain(|&k| k <= key(d + 1, 0));
    }
    let rows_of = |s: &Series| -> Vec<usize> {
        let mut rows = Vec::with_capacity(common.len());
        let mut j = 0;
        for &k in &common {
            while s.key(j) < k {
                j += 1;
            }
            rows.push(j);
        }
        rows
    };
    let first_rows = rows_of(&series[0]);
    let date: Vec<Date> = first_rows.iter().map(|&r| series[0].date[r]).collect();
    let minute: Vec<i32> = first_rows.iter().map(|&r| series[0].minute[r]).collect();
    let utc: Vec<i64> = first_rows.iter().map(|&r| series[0].utc[r]).collect();
    let bars: Vec<Ohlcv> = series.iter().map(|s| s.px.take(&rows_of(s))).collect();

    let (short_o, short_c) = vol_frame(store, &date, &common, interval, "^VIX1D")?;
    let (vix_o, vix_c) = vol_frame(store, &date, &common, interval, "^VIX")?;
    let mut vol = Vec::new();
    for &s in symbols {
        let (atm_o, atm_c) = vol_frame(store, &date, &common, interval, vol_symbol(s))?;
        // QQQ has no 1-day index: scale VIX1D by the VXN/VIX ratio.
        let ratio = |atm: f64, vix: f64| if s == "SPY" { 1.0 } else if vix > 0.0 { atm / vix } else { 1.0 };
        let n = common.len();
        let mut f = VolFrame { atm_open: atm_o, atm_close: atm_c, short_open: vec![0.0; n], short_close: vec![0.0; n] };
        for k in 0..n {
            let so = short_o[k] * ratio(f.atm_open[k], vix_o[k]);
            let sc = short_c[k] * ratio(f.atm_close[k], vix_c[k]);
            f.short_open[k] = if so.is_nan() { f.atm_open[k] } else { so };
            f.short_close[k] = if sc.is_nan() { f.atm_close[k] } else { sc };
        }
        for col in [&mut f.atm_open, &mut f.atm_close, &mut f.short_open, &mut f.short_close] {
            ffill(col, usize::MAX);
            bfill(col);
        }
        vol.push(f);
    }

    let rate = match store.get("^IRX", "1d")? {
        Some(irx) => {
            let mut r: Vec<f64> = daily_prev_close(&irx, &date).into_iter().map(|x| x / 100.0).collect();
            ffill(&mut r, usize::MAX);
            r.into_iter().map(|x| if x.is_nan() { DEFAULT_RATE } else { x }).collect()
        }
        None => vec![DEFAULT_RATE; date.len()],
    };

    let daily = if interval == Interval::Daily {
        bars.iter().map(|b| DailyBars { date: date.clone(), px: b.clone() }).collect()
    } else {
        let mut v = Vec::new();
        for s in symbols {
            let d = store.need(s, "1d")?;
            v.push(DailyBars { date: d.date.clone(), px: d.px.clone() });
        }
        v
    };
    Ok(MarketData { interval, symbols: symbols.to_vec(), date, minute, utc, bars, vol, rate, daily })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_keep_wall_time_and_offset() {
        let (d, m, u) = parse_stamp("2026-11-18 15:30:00-05:00").unwrap();
        assert_eq!(super::super::calendar::iso(d), "2026-11-18");
        assert_eq!(m, 15 * 60 + 30);
        assert_eq!(u, d as i64 * 86_400 + (15 * 60 + 30) * 60 + 5 * 3600);
        let (_, _, summer) = parse_stamp("2026-07-01 00:00:00-04:00").unwrap();
        assert_eq!(summer % 86_400, 4 * 3600);
        assert!(parse_stamp("2026-11-18").is_err());
    }

    #[test]
    fn fill_rules_match_pandas() {
        let n = f64::NAN;
        let mut v = [1.0, n, n, 2.0, n];
        ffill(&mut v, 1);
        assert_eq!(v[1], 1.0);
        assert!(v[2].is_nan());
        assert_eq!(v[4], 2.0);
        let mut w = [n, n, 3.0, n];
        ffill(&mut w, usize::MAX);
        bfill(&mut w);
        assert_eq!(w, [3.0, 3.0, 3.0, 3.0]);
    }

    #[test]
    fn bar_time_rules() {
        assert_eq!(Interval::Daily.open_minute(0), 570);
        assert_eq!(Interval::Daily.close_minute(0), 960);
        assert_eq!(Interval::Hourly.close_minute(9 * 60 + 30), 10 * 60 + 30);
        assert_eq!(Interval::Hourly.close_minute(15 * 60 + 30), 960); // the last hourly bar is a half hour
        assert_eq!(Interval::FiveMinute.close_minute(955), 960);
    }

    #[test]
    fn market_from_small_fixture_files() {
        let dir = std::env::temp_dir().join(format!("fa-trading-data-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let w = |name: &str, rows: &[&str]| {
            let mut t = String::from("Date,open,high,low,close,volume\n");
            for r in rows {
                t.push_str(r);
                t.push('\n');
            }
            std::fs::write(dir.join(name), t).unwrap();
        };
        w("SPY_1d.csv", &[
            "2026-11-16 00:00:00-05:00,100,101,99,100,10",
            "2026-11-17 00:00:00-05:00,100,102,99,101,10",
            "2026-11-18 00:00:00-05:00,101,103,100,102,10",
        ]);
        w("_VIX_1d.csv", &[
            "2026-11-16 00:00:00-05:00,20,21,19,20,0",
            "2026-11-17 00:00:00-05:00,20,22,19,21,0",
            "2026-11-18 00:00:00-05:00,21,23,20,22,0",
        ]);
        w("_VIX1D_1d.csv", &["2026-11-17 00:00:00-05:00,10,11,9,12,0", "2026-11-18 00:00:00-05:00,12,13,11,14,0"]);
        w("_IRX_1d.csv", &["2026-11-16 00:00:00-05:00,4,4,4,4.0,0", "2026-11-17 00:00:00-05:00,4,4,4,4.5,0"]);
        let store = Store::new(dir.clone());
        let md = load_market(&store, &["SPY"], Interval::Daily, None, None).unwrap();
        assert_eq!(md.len(), 3);
        // Vol is a decimal. The first day has no VIX1D row and no previous day: the 30-day value is used.
        assert!((md.vol[0].atm_close[1] - 0.21).abs() < 1e-15);
        assert!((md.vol[0].short_close[1] - 0.12).abs() < 1e-15);
        assert!((md.vol[0].short_close[0] - 0.20).abs() < 1e-15);
        // The rate is the PREVIOUS day's ^IRX close; the first day has none, so the default.
        assert_eq!(md.rate[0], DEFAULT_RATE);
        assert!((md.rate[1] - 0.04).abs() < 1e-15);
        // A date without a ^IRX row takes the last rate (forward fill).
        assert!((md.rate[2] - 0.04).abs() < 1e-15);
        // `start` cuts the index.
        let cut = load_market(&store, &["SPY"], Interval::Daily, Some(days_from_civil(2026, 11, 17)), None).unwrap();
        assert_eq!(cut.len(), 2);
        std::fs::remove_dir_all(dir).ok();
    }
}
