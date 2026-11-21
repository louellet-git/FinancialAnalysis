//! US interest rates and inflation from official, free, keyless sources (the Rust form of
//! `financial_analysis/rates.py`).
//!
//! - U.S. Treasury Daily Par Yield Curve (nominal, 1M-30Y, from 1990) and Par Real Yield
//!   Curve (TIPS, 5Y-30Y, from 2003): a CSV file for each year.
//! - Federal Reserve Bank of New York reference rates: SOFR, EFFR (with the FOMC target
//!   range), OBFR, TGCR, BGCR.
//! - Bureau of Labor Statistics CPI-U (headline and core), public API v1.
//! - FRED (St. Louis Fed) public `fredgraph.csv`: cross-check and long history.
//!
//! Derived inflation measures use the FRED formulas, so they cross-check exactly:
//! breakeven = nominal par yield - real par yield on the same date; 5y5y forward =
//! `((1 + be10)^10 / (1 + be5)^5)^(1/5) - 1`.
//!
//! UNITS: every rate is a decimal (0.0507 = 5.07%). The sources publish percent.
//!
//! The cache is `data/raw/rates/` (same file names as the Python code had, so an old cache
//! stays valid). The requests use the client name `FinancialAnalysis fa (personal research)`
//! and wait 2 seconds between requests. A refusal is an error: the code does not retry and
//! does not change its identity. If a download fails and a cache file exists, the old file
//! is used and a warning goes to stderr.
//!
//! Tests do not use the network: `Rates::with_net` takes a fake source.

use crate::curve::{civil_from_days, parse_us};
use crate::{CheckResult, Curve, FomcTarget, Inflation, RateObservation, Snapshot, TENORS};
use mapcheck::dates;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub type Res<T> = Result<T, String>;

pub const USER_AGENT: &str = crate::refdata::USER_AGENT;
pub const DELAY_SECONDS: f64 = crate::refdata::DELAY_SECONDS;

pub const TREASURY_CSV: &str = "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/\
daily-treasury-rates.csv/{year}/all?type={kind}&field_tdr_date_value={year}&page&_format=csv";
pub const NYFED_URL: &str = "https://markets.newyorkfed.org/api/rates/{group}/{rate}/search.json?startDate={start}&endDate={end}";
pub const BLS_URL: &str = "https://api.bls.gov/publicAPI/v1/timeseries/data/";
pub const FRED_CSV: &str = "https://fred.stlouisfed.org/graph/fredgraph.csv?id={series}&cosd={start}";

pub const BREAKEVEN_TENORS: [&str; 5] = ["5Y", "7Y", "10Y", "20Y", "30Y"];
const CPI_SERIES: [(&str, &str); 2] = [("headline", "CUUR0000SA0"), ("core", "CUUR0000SA0L1E")];
const FRED_NOMINAL: [(&str, &str); 11] = [
    ("1M", "DGS1MO"),
    ("3M", "DGS3MO"),
    ("6M", "DGS6MO"),
    ("1Y", "DGS1"),
    ("2Y", "DGS2"),
    ("3Y", "DGS3"),
    ("5Y", "DGS5"),
    ("7Y", "DGS7"),
    ("10Y", "DGS10"),
    ("20Y", "DGS20"),
    ("30Y", "DGS30"),
];
const FRED_REAL: [(&str, &str); 5] = [("5Y", "DFII5"), ("7Y", "DFII7"), ("10Y", "DFII10"), ("20Y", "DFII20"), ("30Y", "DFII30")];
const FRED_CPI: [(&str, &str); 2] = [("headline", "CPIAUCNS"), ("core", "CPILFENS")];
const NYFED_RATES: [(&str, &str); 5] =
    [("SOFR", "secured"), ("TGCR", "secured"), ("BGCR", "secured"), ("EFFR", "unsecured"), ("OBFR", "unsecured")];

pub fn treasury_first_year(real: bool) -> i64 {
    if real {
        2003
    } else {
        1990
    }
}

// ---------------------------------------------------------------------------
// Source of text: the network, or a fake in a test
// ---------------------------------------------------------------------------

pub trait Net {
    fn get(&mut self, url: &str) -> Res<String>;
    fn post_json(&mut self, url: &str, body: &str) -> Res<String>;
}

/// The network. Each request goes through the queue of its host (`hostnet`): the pause of `DELAY_SECONDS` is a pause
/// for each host.
pub struct Live;

impl Live {
    pub fn new() -> Live {
        Live
    }
}

impl Default for Live {
    fn default() -> Self {
        Live::new()
    }
}

fn text(bytes: Vec<u8>, url: &str) -> Res<String> {
    String::from_utf8(bytes).map_err(|_| format!("{url}: the answer is not UTF-8 text"))
}

impl Net for Live {
    fn get(&mut self, url: &str) -> Res<String> {
        let b = crate::hostnet::get(url, &[], DELAY_SECONDS)?;
        text(b, url)
    }
    fn post_json(&mut self, url: &str, body: &str) -> Res<String> {
        let b = crate::hostnet::post_json(url, body, DELAY_SECONDS)?;
        text(b, url)
    }
}

/// The cache folder and the source of the data. One value for one command.
pub struct Rates {
    /// `data/raw`
    pub cache: PathBuf,
    /// `data/local`: the folder of the data that the terms of the source do not let us store in the public data store
    /// (FRED). It is git-ignored. In a test it is `<cache>/local`.
    pub local: PathBuf,
    net: Box<dyn Net>,
    /// Today as a day number (a test sets it).
    pub today: i64,
    pub(crate) fred_memo: HashMap<String, BTreeMap<i64, f64>>,
    /// The FRED API key (`FRED_API_KEY`). With a key the FRED loaders use the API first, then the keyless CSV.
    pub(crate) fred_key: Option<String>,
}

impl Rates {
    /// The network, the cache `<repo>/data/raw`, today from the clock.
    pub fn live(repo: &Path) -> Rates {
        let mut r = Rates::with_net(repo.join("data").join("raw"), Box::new(Live::new()), dates::today());
        r.local = repo.join("data").join("local");
        r.with_fred_key(crate::market::keys::read_key(repo, crate::market::fred_api::KEY_ENV))
    }

    /// Use the FRED API with this key as the first source of the FRED series (`None`: the keyless CSV only).
    pub fn with_fred_key(mut self, key: Option<String>) -> Rates {
        self.fred_key = key;
        self
    }

    pub fn with_net(cache: PathBuf, net: Box<dyn Net>, today: i64) -> Rates {
        Rates { local: cache.join("local"), cache, net, today, fred_memo: HashMap::new(), fred_key: None }
    }

    pub(crate) fn get(&mut self, url: &str) -> Res<String> {
        self.net.get(url)
    }

    /// `on`, or today.
    pub fn on_or_today(&self, on: Option<i64>) -> i64 {
        on.unwrap_or(self.today)
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// `round(x, 6)` of Python: the decimal text with 6 places, read back.
pub fn round_dp(x: f64, places: usize) -> f64 {
    format!("{x:.places$}").parse().unwrap_or(x)
}

pub fn round6(x: f64) -> f64 {
    round_dp(x, 6)
}

/// A percent cell as a decimal. `None` for "", ".", "N/A", "NA", "ND".
pub fn pct_to_decimal(raw: &str) -> Res<Option<f64>> {
    let raw = raw.trim();
    if matches!(raw, "" | "." | "N/A" | "NA" | "ND") {
        return Ok(None);
    }
    let v: f64 = raw.parse().map_err(|_| format!("could not convert string to float: '{raw}'"))?;
    Ok(Some(round6(v / 100.0)))
}

fn is_fresh(path: &Path, max_age_hours: f64) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < Duration::from_secs_f64(max_age_hours * 3600.0))
}

pub(crate) fn is_fresh_file(path: &Path, max_age_hours: f64) -> bool {
    is_fresh(path, max_age_hours)
}

fn write_cache(path: &Path, text: &str) -> Res<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn save_cache(path: &Path, text: &str) -> Res<()> {
    write_cache(path, text)
}

pub(crate) fn iso(day: i64) -> String {
    dates::format(day)
}

fn year_of(day: i64) -> i64 {
    civil_from_days(day).0
}

/// Python `list` text of strings: `['a', 'b']`.
fn py_list(items: &[String]) -> String {
    format!("[{}]", items.iter().map(|s| format!("'{s}'")).collect::<Vec<_>>().join(", "))
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

fn month_year(day: i64) -> String {
    let (y, m, _) = civil_from_days(day);
    format!("{} {y}", MONTHS[(m - 1) as usize])
}

/// The text of an observation, as the Python `str()`: `UST 10Y 5.07% on 2026-11-18 (U.S. Treasury)`.
pub fn obs_text(o: &RateObservation) -> String {
    let s = format!("{} {:.2}% on {} ({})", o.series, o.value * 100.0, o.date, o.source);
    if o.note.is_empty() {
        s
    } else {
        format!("{s} [{}]", o.note)
    }
}

pub fn observation(series: impl Into<String>, day: i64, value: f64, source: &str, note: impl Into<String>) -> RateObservation {
    RateObservation { series: series.into(), date: iso(day), value, source: source.to_string(), note: note.into() }
}

/// One CSV line: fields split at commas, with double quotes (a doubled quote is one quote).
pub fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

// ---------------------------------------------------------------------------
// U.S. Treasury par yield curves
// ---------------------------------------------------------------------------

/// `1 Mo` / `1.5 Month` / `10 Yr` / `10 YR` -> `1M` / `1.5M` / `10Y` / `10Y`.
pub fn normalize_tenor(header: &str) -> Res<String> {
    let h = header.trim();
    let split = h.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(h.len());
    let (num, unit) = (&h[..split], h[split..].trim().to_lowercase());
    let ok_unit = matches!(unit.as_str(), "mo" | "month" | "months" | "yr" | "year" | "years");
    if num.is_empty() || !ok_unit {
        return Err(format!("unrecognized Treasury tenor header: '{header}'"));
    }
    Ok(format!("{num}{}", if unit.starts_with("mo") { 'M' } else { 'Y' }))
}

/// `{tenor: decimal}` of one day.
pub type Tenors = BTreeMap<String, f64>;

pub type CurveByDay = BTreeMap<i64, Tenors>;

/// A Treasury daily-rates CSV as `{day: {tenor: decimal}}`. A blank cell is not in the map
/// (for example the 30Y was not issued from 2002 to 2006).
pub fn parse_treasury_csv(text: &str) -> Res<CurveByDay> {
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let header = split_csv_line(lines.next().unwrap_or(""));
    if header.len() < 2 || header[0].trim() != "Date" {
        return Err(format!("not a Treasury rates CSV (header: {:?})", &header[..header.len().min(3)]));
    }
    let tenors: Vec<String> = header[1..].iter().map(|h| normalize_tenor(h)).collect::<Res<_>>()?;
    let mut out = CurveByDay::new();
    for line in lines {
        let row = split_csv_line(line);
        if row.is_empty() || row[0].trim().is_empty() {
            continue;
        }
        let day = parse_us(&row[0]).ok_or_else(|| format!("bad Treasury date '{}'", row[0].trim()))?;
        let mut curve = BTreeMap::new();
        for (tenor, cell) in tenors.iter().zip(&row[1..]) {
            if let Some(v) = pct_to_decimal(cell)? {
                curve.insert(tenor.clone(), v);
            }
        }
        out.insert(day, curve);
    }
    Ok(out)
}

/// Tenors of a curve in curve order (1M first, 30Y last); tenors that are not in the list go last.
pub fn tenors_in_order(curve: &BTreeMap<String, f64>) -> Vec<String> {
    let mut v: Vec<String> = curve.keys().cloned().collect();
    v.sort_by_key(|t| TENORS.iter().position(|x| x == t).unwrap_or(usize::MAX));
    v
}

impl Rates {
    fn treasury_path(&self, year: i64, real: bool) -> PathBuf {
        self.cache.join("rates").join(format!("treasury_{}_{year}.csv", if real { "real" } else { "nominal" }))
    }

    /// All daily curves of one calendar year. A past year stays in the cache for ever. The
    /// current year is downloaded again when the cache is older than 6 hours.
    pub fn load_treasury_curve(&mut self, year: i64, real: bool, refresh: bool) -> Res<CurveByDay> {
        let path = self.treasury_path(year, real);
        let current = year >= year_of(self.today);
        let use_cache = path.exists() && !refresh && (!current || is_fresh(&path, 6.0));
        let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
        if use_cache {
            return parse_treasury_csv(&read(&path)?);
        }
        let kind = if real { "daily_treasury_real_yield_curve" } else { "daily_treasury_yield_curve" };
        let url = TREASURY_CSV.replace("{year}", &year.to_string()).replace("{kind}", kind);
        let fetched = self.net.get(&url).and_then(|t| parse_treasury_csv(&t).map(|p| (t, p)));
        match fetched {
            Ok((t, parsed)) => {
                write_cache(&path, &t)?;
                Ok(parsed)
            }
            Err(e) => {
                if path.exists() {
                    eprintln!("fa: Treasury download failed for {kind} {year} ({e}); using the cached copy");
                    parse_treasury_csv(&read(&path)?)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// The latest full curve on or before `on` (default: today). A weekend or a holiday gives
    /// the business day before, across the year end if needed.
    pub fn treasury_curve(&mut self, on: Option<i64>, real: bool) -> Res<(i64, BTreeMap<String, f64>)> {
        let on = self.on_or_today(on);
        for year in [year_of(on), year_of(on) - 1] {
            if year < treasury_first_year(real) {
                break;
            }
            let curves = self.load_treasury_curve(year, real, false)?;
            if let Some((d, c)) = curves.range(..=on).next_back() {
                return Ok((*d, c.clone()));
            }
        }
        Err(format!("no Treasury {} curve on or before {}", if real { "real" } else { "nominal" }, iso(on)))
    }

    /// One tenor's par yield on or before `on`.
    pub fn treasury_yield(&mut self, tenor: &str, on: Option<i64>, real: bool) -> Res<RateObservation> {
        let (d, curve) = self.treasury_curve(on, real)?;
        let v = *curve
            .get(tenor)
            .ok_or_else(|| format!("tenor {tenor} not published on {}; available: {}", iso(d), py_list(&tenors_in_order(&curve))))?;
        Ok(observation(format!("UST {}{tenor}", if real { "real " } else { "" }), d, v, "U.S. Treasury", ""))
    }

    pub fn curve(&mut self, on: Option<i64>, real: bool) -> Res<Curve> {
        let (d, c) = self.treasury_curve(on, real)?;
        Ok(Curve { date: iso(d), source: "U.S. Treasury".into(), real, tenors: c })
    }

    /// Daily series of one tenor from `start` to `end` (inclusive).
    pub fn treasury_history(&mut self, tenor: &str, start: Option<i64>, end: Option<i64>, real: bool) -> Res<BTreeMap<i64, f64>> {
        let end = end.unwrap_or(self.today);
        let start = start.unwrap_or_else(|| dates::days_from_civil(year_of(end), 1, 1));
        let mut out = BTreeMap::new();
        for year in year_of(start).max(treasury_first_year(real))..=year_of(end) {
            for (d, c) in self.load_treasury_curve(year, real, false)? {
                if start <= d && d <= end {
                    if let Some(v) = c.get(tenor) {
                        out.insert(d, *v);
                    }
                }
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Market-implied inflation
// ---------------------------------------------------------------------------

pub fn breakeven_of(nom: &BTreeMap<String, f64>, real: &BTreeMap<String, f64>, tenor: &str) -> Res<f64> {
    match (nom.get(tenor), real.get(tenor)) {
        (Some(n), Some(r)) => Ok(round6(n - r)),
        _ => Err(format!("breakeven needs {tenor} on both curves; tenors: {BREAKEVEN_TENORS:?}")),
    }
}

/// 5y5y forward inflation from the 5Y and 10Y breakevens (FRED T5YIFR).
pub fn forward_5y5y(be5: f64, be10: f64) -> f64 {
    round6(((1.0 + be10).powf(10.0) / (1.0 + be5).powf(5.0)).powf(0.2) - 1.0)
}

const BREAKEVEN_SOURCE: &str = "U.S. Treasury (nominal minus TIPS par yield)";
const FORWARD_SOURCE: &str = "U.S. Treasury (from 5Y and 10Y breakevens)";

impl Rates {
    /// (day, nominal curve, real curve) for the latest day on or before `on` that has both curves.
    pub fn same_day_curves(&mut self, on: Option<i64>) -> Res<(i64, Tenors, Tenors)> {
        let on_day = self.on_or_today(on);
        let (mut d, _) = self.treasury_curve(Some(on_day), true)?;
        for _ in 0..10 {
            let (nd, nom) = self.treasury_curve(Some(d), false)?;
            let (rd, real) = self.treasury_curve(Some(nd), true)?;
            if nd == rd {
                return Ok((nd, nom, real));
            }
            d = nd.min(rd);
        }
        Err(format!("no date on or before {} has both Treasury curves", iso(on_day)))
    }

    pub fn breakeven_inflation(&mut self, tenor: &str, on: Option<i64>) -> Res<RateObservation> {
        let (d, nom, real) = self.same_day_curves(on)?;
        Ok(observation(format!("Breakeven inflation {tenor}"), d, breakeven_of(&nom, &real, tenor)?, BREAKEVEN_SOURCE, ""))
    }

    pub fn forward_inflation_5y5y(&mut self, on: Option<i64>) -> Res<RateObservation> {
        let (d, nom, real) = self.same_day_curves(on)?;
        let v = forward_5y5y(breakeven_of(&nom, &real, "5Y")?, breakeven_of(&nom, &real, "10Y")?);
        Ok(observation("5y5y forward inflation", d, v, FORWARD_SOURCE, ""))
    }

    /// Market-implied and realized inflation on or before `on`.
    pub fn inflation_snapshot(&mut self, on: Option<i64>) -> Res<Inflation> {
        let (d, nom, real) = self.same_day_curves(on)?;
        let mut be: BTreeMap<&str, f64> = BTreeMap::new();
        for t in BREAKEVEN_TENORS {
            if nom.contains_key(t) && real.contains_key(t) {
                be.insert(t, breakeven_of(&nom, &real, t)?);
            }
        }
        let pick = |t: &str| be.get(t).copied().ok_or_else(|| format!("breakeven needs {t} on both curves; tenors: {BREAKEVEN_TENORS:?}"));
        let mk = |t: &str, v: f64| observation(format!("Breakeven inflation {t}"), d, v, BREAKEVEN_SOURCE, "");
        let (be5, be10) = (pick("5Y")?, pick("10Y")?);
        let other = be
            .iter()
            .filter(|(t, _)| **t != "5Y" && **t != "10Y")
            .map(|(t, v)| (format!("breakeven_{}", t.to_lowercase()), mk(t, *v)))
            .collect();
        let real10 = *real.get("10Y").ok_or("KeyError: '10Y'")?;
        Ok(Inflation {
            breakeven_5y: mk("5Y", be5),
            breakeven_10y: mk("10Y", be10),
            forward_5y5y: observation("5y5y forward inflation", d, forward_5y5y(be5, be10), FORWARD_SOURCE, ""),
            real_10y: observation("UST real 10Y", d, real10, "U.S. Treasury", ""),
            cpi_12m: self.cpi_inflation("headline", on)?,
            core_cpi_12m: self.cpi_inflation("core", on)?,
            other,
        })
    }
}

// ---------------------------------------------------------------------------
// Realized inflation: BLS CPI-U
// ---------------------------------------------------------------------------

/// `{series id: {month start: index}}`. It skips M13 (annual average) and missing values
/// ("-"), for example October 2025, which BLS did not collect.
pub fn parse_bls_json(text: &str) -> Res<BTreeMap<String, BTreeMap<i64, f64>>> {
    let d: Value = serde_json::from_str(text).map_err(|e| format!("BLS answer is not JSON: {e}"))?;
    let status = d.get("status").and_then(|s| s.as_str()).unwrap_or("None");
    if status != "REQUEST_SUCCEEDED" {
        let message = match d.get("message") {
            Some(Value::Array(a)) => py_list(&a.iter().map(|m| m.as_str().unwrap_or("").to_string()).collect::<Vec<_>>()),
            Some(Value::String(s)) => s.clone(),
            _ => "None".to_string(),
        };
        return Err(format!("BLS request failed: {status} {message}"));
    }
    let series = d.pointer("/Results/series").and_then(|s| s.as_array()).ok_or("BLS answer has no Results.series")?;
    let mut out = BTreeMap::new();
    for s in series {
        let id = s.get("seriesID").and_then(|x| x.as_str()).ok_or("BLS series has no seriesID")?;
        let mut rows = BTreeMap::new();
        for x in s.get("data").and_then(|x| x.as_array()).ok_or("BLS series has no data")? {
            let p = x.get("period").and_then(|v| v.as_str()).unwrap_or("");
            let month: i64 = match p.strip_prefix('M').and_then(|m| m.parse().ok()) {
                Some(m) if p.len() == 3 && (1..=12).contains(&m) => m,
                _ => continue,
            };
            let v = x.get("value").and_then(|v| v.as_str()).unwrap_or("").trim();
            if v.is_empty() || v == "-" {
                continue;
            }
            let year: i64 = x.get("year").and_then(|y| y.as_str()).and_then(|y| y.parse().ok()).ok_or("BLS row has a bad year")?;
            let value: f64 = v.parse().map_err(|_| format!("could not convert string to float: '{v}'"))?;
            rows.insert(dates::days_from_civil(year, month, 1), value);
        }
        out.insert(id.to_string(), rows);
    }
    Ok(out)
}

fn year_before(m: i64) -> i64 {
    let (y, mo, _) = civil_from_days(m);
    dates::days_from_civil(y - 1, mo, 1)
}

impl Rates {
    /// CPI-U index levels for the last 10 years, headline and core, in one BLS request. The
    /// cache lasts 24 hours because API v1 allows 25 requests a day.
    pub fn load_cpi(&mut self, refresh: bool) -> Res<BTreeMap<String, BTreeMap<i64, f64>>> {
        let path = self.cache.join("rates").join("bls_cpi.json");
        let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
        if path.exists() && !refresh && is_fresh(&path, 24.0) {
            return parse_bls_json(&read(&path)?);
        }
        let end = year_of(self.today);
        let ids: Vec<&str> = CPI_SERIES.iter().map(|(_, id)| *id).collect();
        let body = serde_json::json!({"seriesid": ids, "startyear": (end - 9).to_string(), "endyear": end.to_string()}).to_string();
        let fetched = self.net.post_json(BLS_URL, &body).and_then(|t| parse_bls_json(&t).map(|p| (t, p)));
        match fetched {
            Ok((t, parsed)) => {
                write_cache(&path, &t)?;
                Ok(parsed)
            }
            Err(e) => {
                if path.exists() {
                    eprintln!("fa: BLS download failed ({e}); using the cached CPI");
                    parse_bls_json(&read(&path)?)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// The 12-month CPI-U change for the latest month on or before `on` that has an index 12
    /// months before. If a gap moves the month back, `note` says so. The date is the
    /// reference month; BLS publishes it about two weeks after the month ends.
    pub fn cpi_inflation(&mut self, kind: &str, on: Option<i64>) -> Res<RateObservation> {
        let id = CPI_SERIES.iter().find(|(k, _)| *k == kind).map(|(_, id)| *id).ok_or("kind must be one of ['core', 'headline']")?;
        let on = self.on_or_today(on);
        let all = self.load_cpi(false)?;
        let idx = all.get(id).ok_or_else(|| format!("KeyError: '{id}'"))?;
        let months: Vec<i64> = idx.keys().copied().filter(|m| *m <= on).collect();
        let latest = *months.last().ok_or_else(|| format!("no CPI data on or before {}", iso(on)))?;
        let m = *months
            .iter()
            .rev()
            .find(|m| idx.contains_key(&year_before(**m)))
            .ok_or_else(|| format!("no 12-month CPI change on or before {}", iso(on)))?;
        let note = if m != latest {
            format!(
                "latest month {} has no 12-month change: BLS has no index for {}",
                month_year(latest),
                month_year(year_before(latest))
            )
        } else {
            String::new()
        };
        let label = if kind == "headline" { "CPI-U 12-month" } else { "Core CPI-U 12-month" };
        Ok(observation(label, m, round6(idx[&m] / idx[&year_before(m)] - 1.0), "Bureau of Labor Statistics", note))
    }
}

// ---------------------------------------------------------------------------
// New York Fed reference rates
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct NyfedRow {
    pub day: i64,
    pub kind: String,
    pub value: f64,
    pub target_from: Option<f64>,
    pub target_to: Option<f64>,
}

pub fn parse_nyfed_json(text: &str) -> Res<Vec<NyfedRow>> {
    let d: Value = serde_json::from_str(text).map_err(|e| format!("New York Fed answer is not JSON: {e}"))?;
    let mut rows = Vec::new();
    for r in d.get("refRates").and_then(|r| r.as_array()).map(Vec::as_slice).unwrap_or(&[]) {
        let field = |k: &str| -> Res<f64> { r.get(k).and_then(|v| v.as_f64()).ok_or_else(|| format!("New York Fed row has no number '{k}'")) };
        let day = r.get("effectiveDate").and_then(|v| v.as_str()).and_then(dates::parse).ok_or("New York Fed row has a bad effectiveDate")?;
        rows.push(NyfedRow {
            day,
            kind: r.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            value: round6(field("percentRate")? / 100.0),
            target_from: r.get("targetRateFrom").map(|_| field("targetRateFrom").map(|v| round6(v / 100.0))).transpose()?,
            target_to: r.get("targetRateTo").map(|_| field("targetRateTo").map(|v| round6(v / 100.0))).transpose()?,
        });
    }
    rows.sort_by_key(|r| r.day);
    Ok(rows)
}

impl Rates {
    /// Daily rows of a NY Fed reference rate (decimals). SOFR starts 2018-04-02. EFFR rows also
    /// have the FOMC target range.
    pub fn nyfed_history(&mut self, rate: &str, start: Option<i64>, end: Option<i64>) -> Res<Vec<NyfedRow>> {
        let rate = rate.to_uppercase();
        let group = NYFED_RATES
            .iter()
            .find(|(r, _)| *r == rate)
            .map(|(_, g)| *g)
            .ok_or_else(|| format!("unknown NY Fed rate '{rate}'; choose from ['BGCR', 'EFFR', 'OBFR', 'SOFR', 'TGCR']"))?;
        let end = end.unwrap_or(self.today);
        let start = start.unwrap_or(end - 14);
        let url = NYFED_URL
            .replace("{group}", group)
            .replace("{rate}", &rate.to_lowercase())
            .replace("{start}", &iso(start))
            .replace("{end}", &iso(end));
        parse_nyfed_json(&self.net.get(&url)?)
    }

    /// The latest published rate on or before `on`. SOFR of day T is published on the morning
    /// of T+1, so "today" gives the business day before.
    pub fn nyfed_rate(&mut self, rate: &str, on: Option<i64>) -> Res<RateObservation> {
        let on = self.on_or_today(on);
        let rows: Vec<NyfedRow> = self.nyfed_history(rate, Some(on - 14), Some(on))?.into_iter().filter(|r| r.day <= on).collect();
        let r = rows.last().ok_or_else(|| format!("no {rate} observation in the 14 days up to {}", iso(on)))?;
        Ok(observation(rate.to_uppercase(), r.day, r.value, "Federal Reserve Bank of New York", ""))
    }
}

// ---------------------------------------------------------------------------
// FRED
// ---------------------------------------------------------------------------

/// `{day: value}`. `percent` converts percent to decimal; `false` keeps the levels (index series).
pub fn parse_fred_csv(text: &str, percent: bool) -> Res<BTreeMap<i64, f64>> {
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    lines.next();
    let mut out = BTreeMap::new();
    for line in lines {
        let row = split_csv_line(line);
        if row.len() < 2 || matches!(row[1].trim(), "" | ".") {
            continue;
        }
        let day = dates::parse(row[0].trim()).ok_or_else(|| format!("Invalid isoformat string: '{}'", row[0]))?;
        let v = if percent {
            pct_to_decimal(&row[1])?.unwrap_or(f64::NAN)
        } else {
            row[1].trim().parse::<f64>().map_err(|_| format!("could not convert string to float: '{}'", row[1].trim()))?
        };
        out.insert(day, v);
    }
    Ok(out)
}

impl Rates {
    /// Any FRED series. Examples: DGS10, DFII10, T10YIE (percent), CPIAUCNS (raw).
    pub fn fred_series(&mut self, series_id: &str, start: Option<i64>, percent: bool) -> Res<BTreeMap<i64, f64>> {
        let start = start.unwrap_or(self.today - 30);
        let text = self.fred_text(series_id, &iso(start))?;
        parse_fred_csv(&text, percent)
    }

    /// The CSV text of a FRED series from `start`: the keyed API first (when `FRED_API_KEY` is set), then the keyless address.
    /// The text of the API is turned into the CSV form, so each cache file and parser stays as it was. The key is in the URL
    /// of the API: each error text and log line has it taken out.
    pub(crate) fn fred_text(&mut self, series_id: &str, start: &str) -> Res<String> {
        let mut keyed_error = None;
        if let Some(key) = self.fred_key.clone() {
            let net = &mut self.net;
            match crate::market::fred_api::fetch_csv(series_id, start, &key, &mut |u: &str| net.get(u)) {
                Ok(text) => return Ok(text),
                Err(e) => {
                    eprintln!("{}", crate::market::fred_api::fallback_message(series_id, &e, &key));
                    keyed_error = Some(crate::market::fred_api::scrub(&e, &key));
                }
            }
        }
        let url = FRED_CSV.replace("{series}", series_id).replace("{start}", start);
        self.get(&url).map_err(|e| match keyed_error {
            Some(k) => format!("{e}; the FRED API with a key failed too: {k}"),
            None => e,
        })
    }
}

pub fn compare(label: &str, ours: &BTreeMap<i64, f64>, fred: &BTreeMap<i64, f64>, fred_id: &str, tol: f64) -> CheckResult {
    let mut compared = 0;
    let mut mismatches = Vec::new();
    for (d, o) in ours {
        if let Some(f) = fred.get(d) {
            compared += 1;
            if (o - f).abs() > tol {
                mismatches.push((iso(*d), *o, *f));
            }
        }
    }
    CheckResult { check: label.to_string(), fred_series: fred_id.to_string(), compared, mismatches }
}

impl Rates {
    /// Treasury's own curve against the FRED copy over the last `days`. FRED can be one day
    /// late, so only the days that both sources have are compared.
    pub fn cross_check(&mut self, tenor: &str, real: bool, days: i64, tol_bp: f64) -> Res<CheckResult> {
        let table: &[(&str, &str)] = if real { &FRED_REAL } else { &FRED_NOMINAL };
        let fred_id = table.iter().find(|(t, _)| *t == tenor).map(|(_, id)| *id).ok_or_else(|| format!("KeyError: '{tenor}'"))?;
        let start = self.today - days;
        let ust = self.treasury_history(tenor, Some(start), None, real)?;
        let label = format!("UST {}{tenor}", if real { "real " } else { "" });
        let fred = self.fred_series(fred_id, Some(start), true)?;
        Ok(compare(&label, &ust, &fred, fred_id, tol_bp / 10_000.0))
    }

    /// Our breakevens and 5y5y against FRED T10YIE and T5YIFR (FRED rounds to 0.01%), and the
    /// BLS index levels against FRED CPIAUCNS and CPILFENS (exact).
    pub fn cross_check_inflation(&mut self, days: i64) -> Res<Vec<CheckResult>> {
        let start = self.today - days;
        let nom10 = self.treasury_history("10Y", Some(start), None, false)?;
        let real10 = self.treasury_history("10Y", Some(start), None, true)?;
        let nom5 = self.treasury_history("5Y", Some(start), None, false)?;
        let real5 = self.treasury_history("5Y", Some(start), None, true)?;
        let all: Vec<i64> =
            nom10.keys().filter(|d| real10.contains_key(d) && nom5.contains_key(d) && real5.contains_key(d)).copied().collect();
        let be10: BTreeMap<i64, f64> = all.iter().map(|d| (*d, round6(nom10[d] - real10[d]))).collect();
        let fwd: BTreeMap<i64, f64> = all.iter().map(|d| (*d, round_dp(forward_5y5y(nom5[d] - real5[d], be10[d]), 4))).collect();
        let mut out = vec![
            compare("Breakeven 10Y", &be10, &self.fred_series("T10YIE", Some(start), true)?, "T10YIE", 0.5e-4),
            compare("5y5y forward", &fwd, &self.fred_series("T5YIFR", Some(start), true)?, "T5YIFR", 0.5e-4 + 1e-9),
        ];
        let cpi_start = dates::days_from_civil(year_of(self.today) - 2, 1, 1);
        let cpi = self.load_cpi(false)?;
        for (kind, fred_id) in FRED_CPI {
            let id = CPI_SERIES.iter().find(|(k, _)| *k == kind).map(|(_, id)| *id).unwrap_or("");
            let ours: BTreeMap<i64, f64> = cpi.get(id).map(|m| m.iter().filter(|(d, _)| **d >= cpi_start).map(|(d, v)| (*d, *v)).collect()).unwrap_or_default();
            let fred = self.fred_series(fred_id, Some(cpi_start), false)?;
            out.push(compare(&format!("CPI-U {kind} index"), &ours, &fred, fred_id, 1e-9));
        }
        Ok(out)
    }

    /// The checks of `fa rates check`.
    pub fn check(&mut self, days: i64) -> Res<Vec<CheckResult>> {
        let mut out = Vec::new();
        for (t, r) in [("10Y", false), ("2Y", false), ("10Y", true)] {
            out.push(self.cross_check(t, r, days, 0.5)?);
        }
        out.extend(self.cross_check_inflation(days)?);
        Ok(out)
    }

    /// Curves, SOFR, EFFR, the FOMC range and inflation.
    pub fn snapshot(&mut self, on: Option<i64>) -> Res<Snapshot> {
        let nominal_curve = self.curve(on, false)?;
        let mut real_curve = self.curve(on, true)?;
        real_curve.real = true;
        let on_day = self.on_or_today(on);
        // One request gives the EFFR rate and the target range (the same 14 days).
        let effr_rows: Vec<NyfedRow> = self.nyfed_history("EFFR", None, on)?.into_iter().filter(|r| r.day <= on_day).collect();
        let sofr = self.nyfed_rate("SOFR", on)?;
        let last = effr_rows.last().ok_or("no EFFR observation in the last 14 days")?;
        let effr = observation("EFFR", last.day, last.value, "Federal Reserve Bank of New York", "");
        Ok(Snapshot {
            nominal_curve,
            real_curve,
            sofr,
            effr,
            fomc_target: FomcTarget { lower: last.target_from, upper: last.target_to },
            inflation: self.inflation_snapshot(on)?,
        })
    }
}

/// `D` is `YYYY-MM-DD` or `today`. `None` stays `None` (the latest).
pub fn parse_on(arg: Option<&str>) -> Res<Option<i64>> {
    match arg {
        None => Ok(None),
        Some(t) => crate::forwards::parse_date_arg(t).map(|(_, d)| Some(d)),
    }
}
