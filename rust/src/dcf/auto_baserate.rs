//! `fa dcf baserates`: how fast the revenue of companies grew in the past, by size (item R1 of the ticket
//! `T-20261121-dcf-engine-integration`). A fixed procedure with no random part and no price.
//!
//! A window is a base fiscal year and the fiscal year `years` years after it. Its growth is the compound annual growth of
//! the mapped variable `revenue` (no raw tag; a year with no mapped value has no window: the engine never patches). A window is
//! point in time: the end year, and the base year, were filed on or before the date D. The size bucket is the revenue of the
//! base year, in nominal dollars. The last window of a company is the window of its last fiscal year.
//!
//! A base rate is a measurement of the past. It is never an input of the model (`docs/QUALITY-STANDARDS.md`, section 6).
//!
//! Limits: the list is a list of today's names (survivorship, which moves the rates up); the windows of one company overlap;
//! acquired revenue is in the growth; a spin-off lowers it; revenue is the value of the latest filing that the mapping reads.

use super::auto::{AutoInputs, YearData};
use super::auto_cli::{cache_dir, jobs_of, map_inputs_pit_periods, num, tickers_of, Opts};
use super::stats::{percentile, sorted_copy};
use super::DcfError;
use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

type R<T> = Result<T, DcfError>;

fn e<T>(m: impl Into<String>) -> R<T> {
    Err(DcfError(m.into()))
}

pub const BASERATE_USAGE: &str = "\
fa dcf baserates [--tickers-file FILE | TICKER ...] --as-of D [--years 10] [OPTIONS]
  How fast the revenue of companies grew, by size. A fixed procedure with no random part and no price. It reads the mapped
  variable `revenue` only (no raw tag): a company with no mapped value in a year has no window with that year, and the
  command does not patch it. A base rate is a measurement of the past. It is never an input of the model.
  A window is a base fiscal year and the fiscal year YEARS years after it (the end dates are YEARS years apart, plus or minus
  45 days). Its growth is the compound annual growth of the revenue. A window is point in time: its end year and its base
  year were filed on or before D. The size bucket is the revenue of the base year, in nominal dollars: below 1B, 1B to 10B,
  10B to 50B, 50B or more.
  The last window of a company is the window of its last fiscal year. A company with a last year that ended more than 18
  months before D has no last window. A company with no revenue in the base year has \"no base-year value\".
  One row for each company: two tickers with the same SEC number (GOOG and GOOGL) are one company, and the first ticker
  of the list is used.
  Output, for each bucket and for all: the last windows (one for each company) and the pooled windows (every window of
  every company): n, median, 75th, 90th and 95th percentile, maximum with its ticker; and the count of companies with no
  base-year value. The limits are printed: the list is a list of today's names (survivorship), the windows of one company
  overlap, acquired revenue is in the growth.
  --as-of D       the date (YYYY-MM-DD); only facts filed on or before it are used (required)
  --years N       the years of a window (default 10)
  --cache DIR     as for `fa dcf auto`
  --out FILE      write the table as CSV: set,bucket,n,median,p75,p90,p95,max,max_ticker (set: last, pooled or no_base_value)
  --jobs N        threads (default: all); the result does not depend on N";

/// Fiscal years that the mapping reads for the base rates. The model builder reads `MAP_PERIODS`; the windows need more.
pub const BASERATE_PERIODS: usize = 40;
/// The default years of a window.
pub const BASERATE_YEARS: usize = 10;
/// The end dates of a window are `years` years apart, plus or minus this number of days (a 52 or 53 week year).
pub const END_TOLERANCE_DAYS: i64 = 45;
/// A last fiscal year that ended more than this number of days before D is not the last year of a live company.
pub const LAST_YEAR_MAX_AGE_DAYS: i64 = 548;

/// The size buckets of the base rates: the revenue of the base year (nominal dollars). Fixed.
pub const BUCKETS: [&str; 4] = ["below 1B", "1B to 10B", "10B to 50B", "50B or more"];
const BUCKET_EDGES: [f64; 3] = [1e9, 10e9, 50e9];

/// The bucket of a base revenue (an edge belongs to the higher bucket).
pub fn bucket_of(revenue: f64) -> usize {
    BUCKET_EDGES.iter().take_while(|edge| revenue >= **edge).count()
}

fn day(date: &str) -> Option<i64> {
    mapcheck::dates::parse(date.get(..10)?)
}

/// One window of one company.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub base_end: String,
    pub end: String,
    pub base_revenue: f64,
    /// The compound annual growth (a decimal).
    pub growth: f64,
}

/// The fiscal years that were public on `d` and have a revenue above 0: (year, end day, revenue). A year with no filing date
/// is not proved public, so it is out.
fn usable_years<'a>(years: &'a [YearData], d: &str) -> Vec<(&'a YearData, i64, f64)> {
    let Some(dd) = day(d) else { return Vec::new() };
    years
        .iter()
        .filter(|y| day(&y.filed).is_some_and(|f| f <= dd))
        .filter_map(|y| Some((y, day(&y.end)?, y.revenue.filter(|r| r.is_finite() && *r > 0.0)?)))
        .collect()
}

/// The windows of `years` fiscal years of one company that were public on `d`, oldest end year first: the end year and the base
/// year were filed on or before `d`. The base year is the year whose end date is nearest to `years` years before the end date
/// (within `END_TOLERANCE_DAYS`).
pub fn windows(years: &[YearData], n: usize, d: &str) -> Vec<Window> {
    if n == 0 {
        return Vec::new();
    }
    let usable = usable_years(years, d);
    let nominal = (365.25 * n as f64).round() as i64;
    let mut out: Vec<Window> = usable
        .iter()
        .filter_map(|(ye, de, re)| {
            let (yb, _, rb) = usable.iter().filter(|(_, db, _)| (de - db - nominal).abs() <= END_TOLERANCE_DAYS).min_by_key(|(_, db, _)| (de - db - nominal).abs())?;
            Some(Window { base_end: yb.end.clone(), end: ye.end.clone(), base_revenue: *rb, growth: (re / rb).powf(1.0 / n as f64) - 1.0 })
        })
        .collect();
    out.sort_by(|a, b| a.end.cmp(&b.end));
    out
}

/// The last fiscal year of a company that was public on `d` (with a revenue): its end date and revenue.
pub fn last_year(years: &[YearData], d: &str) -> Option<(String, f64)> {
    usable_years(years, d).into_iter().max_by_key(|(_, de, _)| *de).map(|(y, _, r)| (y.end.clone(), r))
}

/// The last window of a company, or the reason that it has none.
#[derive(Debug, Clone, PartialEq)]
pub enum Last {
    /// No fiscal year with a revenue that was public on D.
    NoYear,
    /// The last fiscal year ended more than `LAST_YEAR_MAX_AGE_DAYS` before D.
    Stale,
    /// The last year has a revenue, and the base year has none (or there is no year at that distance).
    NoBase,
    Window(Window),
}

pub fn last_window(years: &[YearData], n: usize, d: &str) -> Last {
    let Some((end, _)) = last_year(years, d) else { return Last::NoYear };
    let (Some(de), Some(dd)) = (day(&end), day(d)) else { return Last::NoYear };
    if dd - de > LAST_YEAR_MAX_AGE_DAYS {
        return Last::Stale;
    }
    match windows(years, n, d).into_iter().find(|w| w.end == end) {
        Some(w) => Last::Window(w),
        None => Last::NoBase,
    }
}

/// One window of the table: the company, the bucket of the base year, the growth.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub ticker: String,
    pub bucket: usize,
    pub growth: f64,
    pub end: String,
}

/// The base rates of a list at a date.
#[derive(Debug, Clone, PartialEq)]
pub struct BaseRates {
    pub years: usize,
    pub as_of: String,
    /// Tickers of the list, and companies (one for each SEC number).
    pub tickers: usize,
    pub companies: usize,
    pub no_year: Vec<String>,
    pub stale: Vec<String>,
    /// Companies with a last year and no base-year value.
    pub no_base: Vec<String>,
    /// The last window of each company, and every window of every company.
    pub last: Vec<Case>,
    pub pooled: Vec<Case>,
}

/// The base rates of mapped inputs (with at least `n + 1` fiscal years each: `BASERATE_PERIODS`). `cik_of(ticker)` gives the SEC
/// number; the first ticker of a number is the company, a ticker with no number is a company of its own.
pub fn base_rates(inputs: &[AutoInputs], cik_of: &dyn Fn(&str) -> Option<String>, n: usize, d: &str) -> BaseRates {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = BaseRates { years: n, as_of: d.to_string(), tickers: inputs.len(), companies: 0, no_year: vec![], stale: vec![], no_base: vec![], last: vec![], pooled: vec![] };
    for inp in inputs {
        if let Some(cik) = cik_of(&inp.ticker) {
            if !seen.insert(cik) {
                continue;
            }
        }
        out.companies += 1;
        let case = |w: &Window| Case { ticker: inp.ticker.clone(), bucket: bucket_of(w.base_revenue), growth: w.growth, end: w.end.clone() };
        match last_window(&inp.years, n, d) {
            Last::NoYear => out.no_year.push(inp.ticker.clone()),
            Last::Stale => out.stale.push(inp.ticker.clone()),
            Last::NoBase => out.no_base.push(inp.ticker.clone()),
            Last::Window(w) => out.last.push(case(&w)),
        }
        out.pooled.extend(windows(&inp.years, n, d).iter().map(case));
    }
    out
}

/// The numbers of one set of windows.
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub n: usize,
    pub median: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub max: f64,
    pub max_ticker: String,
}

/// The numbers of a set (`None`: no window). The maximum with several tickers is the first in the order of the list.
pub fn stats(cases: &[&Case]) -> Option<Stats> {
    if cases.is_empty() {
        return None;
    }
    let sorted = sorted_copy(&cases.iter().map(|c| c.growth).collect::<Vec<_>>());
    let top = cases.iter().fold(cases[0], |best, c| if c.growth > best.growth { c } else { best });
    Some(Stats {
        n: cases.len(),
        median: percentile(&sorted, 50.0),
        p75: percentile(&sorted, 75.0),
        p90: percentile(&sorted, 90.0),
        p95: percentile(&sorted, 95.0),
        max: top.growth,
        max_ticker: top.ticker.clone(),
    })
}

impl BaseRates {
    /// The cases of a set in a bucket (`None`: all buckets).
    pub fn in_bucket(set: &[Case], bucket: Option<usize>) -> Vec<&Case> {
        set.iter().filter(|c| bucket.is_none_or(|b| c.bucket == b)).collect()
    }
    /// The growths of the last windows in a bucket.
    pub fn last_growths(&self, bucket: usize) -> Vec<f64> {
        Self::in_bucket(&self.last, Some(bucket)).iter().map(|c| c.growth).collect()
    }
}

/// Where a growth stands among the growths of a set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    /// The share of the set that is below the growth (percent, 0 to 100).
    pub pctile: f64,
    /// The growth is above the maximum of the set.
    pub above_all: bool,
    pub n: usize,
}

/// The place of `x` in `values`: the share of the values that are below `x`, and whether `x` is above all of them. `None` for
/// an empty set or a number that is not finite.
pub fn position_in(values: &[f64], x: f64) -> Option<Position> {
    let v: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if v.is_empty() || !x.is_finite() {
        return None;
    }
    let below = v.iter().filter(|a| **a < x).count();
    Some(Position { pctile: 100.0 * below as f64 / v.len() as f64, above_all: v.iter().all(|a| *a < x), n: v.len() })
}

fn pct(x: f64) -> String {
    format!("{:.1}%", x * 100.0)
}

/// The table of the text: one line for each bucket and for all.
pub fn table_text(title: &str, set: &[Case]) -> String {
    let mut s = format!("  {title}\n    {:<12} {:>5} {:>8} {:>8} {:>8} {:>8} {:>8}  maximum\n", "bucket", "n", "median", "75th", "90th", "95th", "max");
    let rows = BUCKETS.iter().enumerate().map(|(i, b)| (b.to_string(), Some(i))).chain(std::iter::once(("all".to_string(), None)));
    for (name, b) in rows {
        match stats(&BaseRates::in_bucket(set, b)) {
            Some(x) => s.push_str(&format!("    {:<12} {:>5} {:>8} {:>8} {:>8} {:>8} {:>8}  {}\n", name, x.n, pct(x.median), pct(x.p75), pct(x.p90), pct(x.p95), pct(x.max), x.max_ticker)),
            None => s.push_str(&format!("    {:<12} {:>5}\n", name, 0)),
        }
    }
    s
}

/// The limits of the base rates, for the text.
pub const LIMITS: &str = "  Limits: the list is a list of today's names (survivorship: companies that left it are not in the set, which moves the rates up); the windows of one company overlap (they are not independent); acquired revenue is in the growth and a spin-off lowers it; a company with no mapped revenue in the base year has no window (the engine does not patch it); the size is the revenue of the base year in nominal dollars; a base rate is a measurement of the past, never an input of the model.\n";

pub fn text(b: &BaseRates) -> String {
    let names = |v: &[String]| if v.is_empty() { String::new() } else { format!(": {}", v.join(" ")) };
    let mut s = format!(
        "BASE RATES of {} fiscal years at {}: {} tickers, {} companies (one for each SEC number); windows filed on or before {}.\n  last window: {} companies; no base-year value: {}{}; no fiscal year with revenue: {}{}; last year older than {} days: {}{}\n",
        b.years,
        b.as_of,
        b.tickers,
        b.companies,
        b.as_of,
        b.last.len(),
        b.no_base.len(),
        names(&b.no_base),
        b.no_year.len(),
        names(&b.no_year),
        LAST_YEAR_MAX_AGE_DAYS,
        b.stale.len(),
        names(&b.stale)
    );
    s.push_str(&table_text("LAST WINDOW of each company (compound annual growth of the revenue)", &b.last));
    s.push_str(&table_text("POOLED WINDOWS (every window of every company)", &b.pooled));
    s.push_str(LIMITS);
    s
}

fn csv_rows(set: &str, cases: &[Case], out: &mut String) {
    let rows = BUCKETS.iter().enumerate().map(|(i, b)| (b.to_string(), Some(i))).chain(std::iter::once(("all".to_string(), None)));
    for (name, b) in rows {
        let name = name.replace(',', ";");
        match stats(&BaseRates::in_bucket(cases, b)) {
            Some(x) => out.push_str(&format!("{set},{name},{},{},{},{},{},{},{}\n", x.n, num(x.median, 4), num(x.p75, 4), num(x.p90, 4), num(x.p95, 4), num(x.max, 4), x.max_ticker)),
            None => out.push_str(&format!("{set},{name},0,,,,,,\n")),
        }
    }
}

pub const CSV_HEADER: &str = "set,bucket,n,median,p75,p90,p95,max,max_ticker";

pub fn csv_text(b: &BaseRates) -> String {
    let mut s = format!("{CSV_HEADER}\n");
    csv_rows("last", &b.last, &mut s);
    csv_rows("pooled", &b.pooled, &mut s);
    s.push_str(&format!("no_base_value,all,{},,,,,,{}\n", b.no_base.len(), b.no_base.join(" ")));
    s
}

/// `fa dcf baserates`. Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> u8 {
    match command_inner(repo, args) {
        Ok(()) => 0,
        Err(x) => {
            eprintln!("fa: {x}");
            2
        }
    }
}

fn command_inner(repo: &Path, args: &[String]) -> R<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{BASERATE_USAGE}");
        return Ok(());
    }
    let o = Opts::parse(args, &["--cache", "--out", "--jobs", "--as-of", "--years", "--tickers-file"], &[])?;
    let cache = cache_dir(repo, &o);
    let d = o.get("--as-of").ok_or_else(|| DcfError("--as-of DATE is required".into()))?.to_string();
    let Some(dd) = mapcheck::dates::parse(&d) else { return e(format!("--as-of '{d}' is not YYYY-MM-DD")) };
    let years: usize = o.num("--years", BASERATE_YEARS)?;
    if years == 0 || years + 1 > BASERATE_PERIODS {
        return e(format!("--years must be 1 to {}", BASERATE_PERIODS - 1));
    }
    let jobs = jobs_of(&o)?;
    let tickers = tickers_of(&o, &cache)?;
    let t0 = Instant::now();
    let (inputs, map_s) = map_inputs_pit_periods(&cache, &tickers, &d, dd, BASERATE_PERIODS, jobs)?;
    let b = base_rates(&inputs, &|t| super::auto_attribute::cik_of(&cache, t), years, &d);
    if let Some(path) = o.get("--out") {
        std::fs::write(path, csv_text(&b)).map_err(|x| DcfError(format!("{path}: {x}")))?;
    }
    print!("{}", text(&b));
    println!("TIME: mapping {map_s:.2} s | total {:.2} s", t0.elapsed().as_secs_f64());
    Ok(())
}

#[cfg(test)]
#[path = "auto_baserate_tests.rs"]
mod tests;
