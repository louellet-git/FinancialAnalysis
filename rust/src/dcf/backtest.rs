//! Point-in-time vintage backtest of mechanical operating projections (the Rust form of
//! `financial_analysis/backtest.py` and `scripts/backtest.py`).
//!
//! At each fiscal-year-end vintage date the company is rebuilt from the annual periods that
//! had been filed by that date. Parameters come from the data only (no judgment input).
//! Revenue and FCF are projected forward and scored against the fiscal years that were
//! reported later.
//!
//! Honesty rules (same as before):
//! 1. Estimates are point in time: a period is visible to a vintage only if `filed_date` is
//!    on or before the as-of date. Facts from the companyfacts API carry the date of a
//!    later filing, so a vintage can see fewer periods than a real observer: never more.
//! 2. Actuals are the figures as reported now (they can include restatements). This is on
//!    purpose and the output says it.
//! 3. Known data defects are shown (`data_flags`), not repaired.
//!
//! Scope: revenue and FCF only. No WACC, no terminal value, no valuation.
//!
//! The random numbers are the numpy numbers (`nprandom`), so the output is the same as the
//! Python output for the same seed.

use super::classic::{Company, Period};
use super::nprandom::Pcg64;
use super::stats::{mean, percentile, sorted_copy, std_pop};
use super::DcfError;
use crate::curve::{civil_from_days, days_from_civil};
use std::collections::BTreeMap;

/// Up to the last 5 clean annual periods give the parameters (same as the classic engine).
pub const PARAM_LOOKBACK: usize = 5;
pub const MIN_USABLE_PERIODS: usize = 3;
/// Floor of both derived stds, so a smooth history cannot collapse the bands to a point.
pub const STD_FLOOR: f64 = 0.01;
/// A vintage sits at fiscal-year-end plus this lag, after the 10-K is normally filed.
pub const FILING_LAG_DAYS: i64 = 90;

/// Known input defects: ticker, fiscal year, text. A vintage whose window has the year shows
/// the text. Source: `analysis/IBM/README.md`, `REVIEW.md` F13.
const IBM_FLAG: &str = "FY2019-20 recast mismatch: continuing-ops revenue vs unrecast consolidated OCF (margin/growth distorted)";

pub fn data_flag(ticker: &str, year: i32) -> Option<&'static str> {
    match (ticker, year) {
        ("IBM", 2019 | 2020) => Some(IBM_FLAG),
        _ => None,
    }
}

fn year_of(date: &str) -> i32 {
    date.get(..4).and_then(|y| y.parse().ok()).unwrap_or(0)
}

/// `date + days` for an ISO date text.
pub fn add_days(date: &str, days: i64) -> String {
    let p: Vec<i64> = date.split('-').filter_map(|x| x.parse().ok()).collect();
    if p.len() != 3 {
        return date.to_string();
    }
    let (y, m, d) = civil_from_days(days_from_civil(p[0], p[1], p[2]) + days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Annual (10-K) periods that the backtest can use, sorted by period end. Both revenue and
/// operating cash flow must exist (this removes stray rows with one or two fields). A 10-K
/// and a 10-K/A of the same fiscal year give one row: the one filed last.
pub fn clean_annuals(company: &Company) -> Vec<&Period> {
    let mut rows: Vec<&Period> =
        company.periods.iter().filter(|p| p.is_annual() && p.revenue.is_some() && p.operating_cash_flow.is_some()).collect();
    rows.sort_by(|a, b| (&a.period_end, &a.filed_date).cmp(&(&b.period_end, &b.filed_date)));
    let mut by_fy: BTreeMap<i32, &Period> = BTreeMap::new();
    for p in rows {
        by_fy.insert(year_of(&p.period_end), p);
    }
    by_fy.into_values().collect()
}

/// Clean annual periods with `filed_date <= as_of`.
pub fn vintage_periods<'a>(company: &'a Company, as_of: &str) -> Vec<&'a Period> {
    clean_annuals(company).into_iter().filter(|p| p.filed_date.as_str() <= as_of).collect()
}

/// Projection inputs from the periods of one vintage.
#[derive(Debug, Clone, PartialEq)]
pub struct VintageParams {
    pub ticker: String,
    pub as_of: Option<String>,
    pub base_fiscal_year: i32,
    pub base_revenue: f64,
    /// Trailing revenue CAGR over the window.
    pub growth: f64,
    pub growth_std: f64,
    pub fcf_margin_mean: f64,
    pub fcf_margin_std: f64,
    pub window_years: Vec<i32>,
    pub data_flags: Vec<String>,
}

/// Parameters from the periods of a vintage (sorted by period end by the caller or here).
pub fn mechanical_params(periods: &[&Period], ticker: &str, as_of: Option<&str>) -> Result<VintageParams, DcfError> {
    if periods.len() < MIN_USABLE_PERIODS {
        return Err(DcfError(format!(
            "only {} usable annual period(s) available \u{2014} at least {MIN_USABLE_PERIODS} are required to derive mechanical backtest parameters (revenue CAGR and FCF margin distribution).",
            periods.len()
        )));
    }
    let mut sorted: Vec<&Period> = periods.to_vec();
    sorted.sort_by(|a, b| a.period_end.cmp(&b.period_end));
    let window = &sorted[sorted.len().saturating_sub(PARAM_LOOKBACK)..];
    let (first, last) = (window[0], window[window.len() - 1]);
    let span = year_of(&last.period_end) - year_of(&first.period_end);
    let first_rev = first.revenue.unwrap_or(0.0);
    if span <= 0 || first_rev == 0.0 || first_rev <= 0.0 {
        return Err(DcfError(format!(
            "{ticker}: cannot derive trailing revenue CAGR \u{2014} window spans {span} year(s) with first revenue {}.",
            match first.revenue {
                Some(r) => format!("{r:?}"),
                None => "None".to_string(),
            }
        )));
    }
    let last_rev = last.revenue.unwrap_or(0.0);
    let growth = (last_rev / first_rev).powf(1.0 / span as f64) - 1.0;
    let yoy: Vec<f64> = window
        .windows(2)
        .filter(|w| w[0].revenue.is_some_and(|r| r != 0.0) && w[1].revenue.is_some())
        .map(|w| w[1].revenue.unwrap_or(0.0) / w[0].revenue.unwrap_or(0.0) - 1.0)
        .collect();
    let growth_std = if yoy.is_empty() { STD_FLOOR } else { std_pop(&yoy).max(STD_FLOOR) };
    let margins: Vec<f64> = window.iter().filter_map(|p| p.fcf_margin()).collect();
    if margins.is_empty() {
        return Err(DcfError(format!(
            "{ticker}: no annual period in the vintage window has both FCF and revenue \u{2014} FCF margin cannot be derived."
        )));
    }
    let ticker = ticker.to_uppercase();
    let window_years: Vec<i32> = window.iter().map(|p| year_of(&p.period_end)).collect();
    let mut flags: Vec<String> = window_years.iter().filter_map(|y| data_flag(&ticker, *y)).map(str::to_string).collect();
    flags.sort();
    flags.dedup();
    Ok(VintageParams {
        ticker,
        as_of: as_of.map(str::to_string),
        base_fiscal_year: year_of(&last.period_end),
        base_revenue: last_rev,
        growth,
        growth_std,
        fcf_margin_mean: mean(&margins),
        fcf_margin_std: std_pop(&margins).max(STD_FLOOR),
        window_years,
        data_flags: flags,
    })
}

/// Percentiles of revenue and FCF for the years +1..+horizon.
#[derive(Debug, Clone, PartialEq)]
pub struct VintageProjection {
    pub horizon: usize,
    pub n: usize,
    pub seed: u64,
    pub target_fiscal_years: Vec<i32>,
    pub rev: [Vec<f64>; 3],
    pub fcf: [Vec<f64>; 3],
}

/// One growth draw and one margin draw for each path (normal); revenue_t = base * (1+g)^t,
/// FCF_t = revenue_t * margin; margins are clipped to [0, 1]. P10, P50 and P90 for each year.
pub fn project(params: &VintageParams, horizon: usize, n: usize, seed: u64) -> VintageProjection {
    let mut rng = Pcg64::new(seed);
    let growth = rng.normal_vec(params.growth, params.growth_std, n);
    let margins: Vec<f64> = rng.normal_vec(params.fcf_margin_mean, params.fcf_margin_std, n).into_iter().map(|m| m.clamp(0.0, 1.0)).collect();
    let mut rev = [Vec::new(), Vec::new(), Vec::new()];
    let mut fcf = [Vec::new(), Vec::new(), Vec::new()];
    for t in 1..=horizon {
        let revenues: Vec<f64> = growth.iter().map(|g| params.base_revenue * (1.0 + g).powf(t as f64)).collect();
        let fcfs: Vec<f64> = revenues.iter().zip(&margins).map(|(r, m)| r * m).collect();
        let (rs, fs) = (sorted_copy(&revenues), sorted_copy(&fcfs));
        for (k, q) in [10.0, 50.0, 90.0].iter().enumerate() {
            rev[k].push(percentile(&rs, *q));
            fcf[k].push(percentile(&fs, *q));
        }
    }
    VintageProjection {
        horizon,
        n,
        seed,
        target_fiscal_years: (1..=horizon as i32).map(|t| params.base_fiscal_year + t).collect(),
        rev,
        fcf,
    }
}

/// One vintage x target fiscal year x metric row. `actual`, `error_pct` and `in_band` are
/// `None` when the target year is not reported yet. `error_pct = (actual / P50 - 1) * 100`.
#[derive(Debug, Clone, PartialEq)]
pub struct VintageOutcome {
    pub target_fiscal_year: i32,
    pub metric: &'static str,
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
    pub actual: Option<f64>,
    pub error_pct: Option<f64>,
    pub in_band: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vintage {
    pub params: VintageParams,
    pub projection: VintageProjection,
    pub outcomes: Vec<VintageOutcome>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BacktestResult {
    pub ticker: String,
    pub horizon: usize,
    pub vintages: Vec<Vintage>,
    /// (as-of date, reason) of the vintage dates with too little history.
    pub skipped: Vec<(String, String)>,
}

impl BacktestResult {
    /// (realized outcomes inside [P10, P90], all realized outcomes).
    pub fn coverage(&self) -> (usize, usize) {
        let realized: Vec<bool> = self.vintages.iter().flat_map(|v| v.outcomes.iter()).filter_map(|o| o.in_band).collect();
        (realized.iter().filter(|b| **b).count(), realized.len())
    }

    /// Horizon year, then metric: (mean signed error in percent, number of realized rows).
    /// Positive: the actuals came in above the P50 (the model was too low).
    pub fn bias_by_horizon(&self) -> BTreeMap<i32, BTreeMap<&'static str, (f64, usize)>> {
        let mut buckets: BTreeMap<(i32, &'static str), Vec<f64>> = BTreeMap::new();
        for v in &self.vintages {
            for o in &v.outcomes {
                if let Some(e) = o.error_pct {
                    buckets.entry((o.target_fiscal_year - v.params.base_fiscal_year, o.metric)).or_default().push(e);
                }
            }
        }
        let mut out: BTreeMap<i32, BTreeMap<&'static str, (f64, usize)>> = BTreeMap::new();
        for ((h, metric), errs) in buckets {
            out.entry(h).or_default().insert(metric, (mean(&errs), errs.len()));
        }
        out
    }
}

/// Walk-forward backtest: one vintage for each clean fiscal-year-end.
pub fn run_backtest(company: &Company, horizon: usize, n: usize, seed: u64) -> BacktestResult {
    let clean = clean_annuals(company);
    let actuals: BTreeMap<i32, &Period> = clean.iter().map(|p| (year_of(&p.period_end), *p)).collect();
    let mut vintages = Vec::new();
    let mut skipped = Vec::new();
    for anchor in &clean {
        let as_of = add_days(&anchor.period_end, FILING_LAG_DAYS);
        let visible = vintage_periods(company, &as_of);
        let params = match mechanical_params(&visible, &company.ticker, Some(&as_of)) {
            Ok(p) => p,
            Err(e) => {
                skipped.push((as_of, e.0));
                continue;
            }
        };
        let proj = project(&params, horizon, n, seed);
        let mut outcomes = Vec::new();
        for (i, fy) in proj.target_fiscal_years.iter().enumerate() {
            let actual_period = actuals.get(fy);
            for (metric, bands) in [("revenue", &proj.rev), ("fcf", &proj.fcf)] {
                let actual = actual_period.and_then(|p| if metric == "revenue" { p.revenue } else { p.free_cash_flow() });
                let (p10, p50, p90) = (bands[0][i], bands[1][i], bands[2][i]);
                outcomes.push(VintageOutcome {
                    target_fiscal_year: *fy,
                    metric,
                    p10,
                    p50,
                    p90,
                    actual,
                    error_pct: actual.filter(|_| p50 != 0.0).map(|a| (a / p50 - 1.0) * 100.0),
                    in_band: actual.map(|a| p10 <= a && a <= p90),
                });
            }
        }
        vintages.push(Vintage { params, projection: proj, outcomes });
    }
    BacktestResult { ticker: company.ticker.to_uppercase(), horizon, vintages, skipped }
}

// ---------------------------------------------------------------------------
// Text output (the format of `scripts/backtest.py`)
// ---------------------------------------------------------------------------

fn billions(v: Option<f64>) -> String {
    v.map_or("      .".to_string(), |v| format!("{:7.1}", v / 1e9))
}

fn error_text(v: Option<f64>) -> String {
    v.map_or("     .".to_string(), |v| format!("{v:+6.1}"))
}

fn band_text(v: Option<bool>) -> &'static str {
    match v {
        None => "  .",
        Some(true) => "yes",
        Some(false) => " NO",
    }
}

/// Thousands separator, `{:,}` of Python.
fn grouped(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The text of `python -m scripts.backtest TICKER` for a result that has vintages.
pub fn render(company_name: &str, ticker: &str, result: &BacktestResult, n: usize, seed: u64) -> String {
    use std::fmt::Write;
    let mut flag_index: Vec<&str> = Vec::new();
    for v in &result.vintages {
        for f in &v.params.data_flags {
            if !flag_index.contains(&f.as_str()) {
                flag_index.push(f);
            }
        }
    }
    let mut out = String::new();
    let _ = writeln!(out, "\n{company_name} ({ticker}) -- point-in-time vintage backtest");
    let _ = writeln!(out, "  horizon {}y | {} paths | seed {seed}", result.horizon, grouped(n));
    let _ = writeln!(out, "  estimates: point-in-time (filed_date <= as-of) | actuals: as currently reported (may be restated)");
    for metric in ["revenue", "fcf"] {
        let label = if metric == "revenue" { "REVENUE" } else { "FREE CASH FLOW" };
        let _ = writeln!(out, "\n{label} ($B)");
        let _ = writeln!(out, "  {:<12}{:>6}{:>8}{:>9}{:>9}{:>9}{:>9}{:>8}{:>6}  flags", "vintage", "base", "target", "P10", "P50", "P90", "actual", "err%", "band");
        for v in &result.vintages {
            for o in v.outcomes.iter().filter(|o| o.metric == metric) {
                let flags: Vec<String> =
                    v.params.data_flags.iter().filter_map(|f| flag_index.iter().position(|x| x == f)).map(|i| (i + 1).to_string()).collect();
                let flags = if flags.is_empty() { "-".to_string() } else { flags.join(",") };
                let _ = writeln!(
                    out,
                    "  {:<12}{:>6}{:>8}{:>9}{:>9}{:>9}{:>9}{:>8}{:>6}  {flags}",
                    v.params.as_of.as_deref().unwrap_or(""),
                    v.params.base_fiscal_year,
                    o.target_fiscal_year,
                    billions(Some(o.p10)),
                    billions(Some(o.p50)),
                    billions(Some(o.p90)),
                    billions(o.actual),
                    error_text(o.error_pct),
                    band_text(o.in_band),
                );
            }
        }
    }
    let (inside, total) = result.coverage();
    out.push('\n');
    if total > 0 {
        let _ = writeln!(
            out,
            "  Coverage: {inside} of {total} realized outcomes inside the P10-P90 band ({:.0}%; nominal 80%)",
            inside as f64 / total as f64 * 100.0
        );
    } else {
        let _ = writeln!(out, "  Coverage: no realized outcomes yet \u{2014} every target fiscal year is still unreported.");
    }
    let bias = result.bias_by_horizon();
    if !bias.is_empty() {
        let _ = writeln!(out, "  Mean signed error (actual vs P50; + = model underestimated):");
        for (h, metrics) in &bias {
            // reverse alphabetical order: revenue before fcf (as in the Python output)
            let parts: Vec<String> = metrics.iter().rev().map(|(m, (mean, k))| format!("{m} {mean:+.1}% (n={k})")).collect();
            let _ = writeln!(out, "    +{h}y: {}", parts.join(" | "));
        }
    }
    if !flag_index.is_empty() {
        let _ = writeln!(out, "  Data flags (inputs surfaced, not fixed):");
        for (i, f) in flag_index.iter().enumerate() {
            let _ = writeln!(out, "    [{}] {f}", i + 1);
        }
    }
    if !result.skipped.is_empty() {
        let _ = writeln!(out, "  Skipped vintages ({} -- insufficient point-in-time history):", result.skipped.len());
        for (d, reason) in &result.skipped {
            let _ = writeln!(out, "    {d}: {reason}");
        }
    }
    out
}

/// The text when no vintage has enough history (the CLI exits with 1).
pub fn render_no_vintages(ticker: &str, result: &BacktestResult) -> String {
    let mut out = format!("{ticker}: no usable vintages \u{2014} history is too thin at every fiscal-year-end vintage date.\n");
    for (d, reason) in &result.skipped {
        out.push_str(&format!("  {d}: {reason}\n"));
    }
    out
}
