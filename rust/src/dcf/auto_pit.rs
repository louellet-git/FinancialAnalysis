//! The point-in-time test of the automatic model (ticket `T-20261119-dcf-auto-model`, part 3). `fa dcf pit`.
//!
//! For each company of the standard template and each fiscal year `T` that has 5 fiscal years in sequence up to `T`
//! and 3 after it (the rule of `calib_bands`), the automatic model is made from the data of the fiscal years up to
//! `T` only (`model_at`), and the P10, P50 and P90 of the revenue and of the free cash flow of `T+1` to `T+3` are
//! compared with the values that the company reported later.
//!
//! The free cash flow of the test is the one of the model (`fcff_of_rules`): `OCF - capex_total + (1 - t) (interest paid -
//! interest income) - SBC` with the default rules.
//!
//! Limits (they are printed with the result):
//! - The parameter table of the model was estimated on all years of the cache (also the years after `T`). The
//!   test is not out of sample. With `--cut YEAR` the table is estimated again from the years up to `YEAR` only, and
//!   only base years from `YEAR` on are used.
//! - The mapping takes, for a fiscal year, the value of the latest filing. A later restatement of a year before `T`
//!   enters the data of `T`. The effect is not measured.
//! - The companies are those that exist now (survivorship, see `docs/ARCHITECTURE.md`).
//! - The value is not tested here: `fa dcf returns` (`auto_returns.rs`) tests the P50 against the later returns.

use super::auto::*;
use super::auto_cli::load_table;
use super::calib_report::fade_cells;
use super::calibrate::{estimate_table, FadeCell, ProcessTable, TableMeta};
use super::paths::{Mods, PathEngine, F_FCFF, F_REV};
use super::panel::{bucket, build_panel, clean_series, CompanyPanel, RuleCounts, BUCKET_LABELS};
use super::process::PathsSpec;
use super::stats::{percentile, sorted_copy};
use super::DcfError;
use mapcheck::checks::filer_template;
use mapcheck::facts::{period_ends, Forms};
use mapcheck::mapper::resolve_period_with;
use mapcheck::registry::Registry;
use rayon::prelude::*;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

type R<T> = Result<T, DcfError>;

pub const HORIZON: usize = 3;
/// Fiscal years in sequence up to `T` that the test needs.
pub const NEED_BEFORE: usize = 5;

pub const USAGE: &str = "\
fa dcf pit [OPTIONS]
  Point-in-time test of the automatic model: for each company of the standard template and each fiscal year T with
  5 fiscal years up to T and 3 after, make the model from the data up to T only, and place the real revenue and free
  cash flow of T+1 to T+3 against the P10 to P90 band of the model. No network.
  --cache DIR      company facts (default <repo>/data/raw)
  --table FILE     parameter table (default analysis/priors/process_priors_2026-11-19.toml)
  --cut YEAR       estimate the table again from the years up to YEAR; use base years from YEAR on (out of sample)
  --paths N (2000) paths for each model          --seed S (7)
  --rules K=V,..   change a rule; each is a number (a flag is 0 or 1). The list: `fa dcf auto --help`
  --out FILE       write the text of the result to FILE
  --limit N        use the first N companies only (a quick check)";

/// One base year of one company, with the model band and the real values.
#[derive(Debug, Clone, PartialEq)]
pub struct PitObs {
    pub ticker: String,
    pub year: i32,
    pub size: usize,
    pub base: f64,
    /// Growth of the base year (`None`: a jump left out, or no year before).
    pub g_last: Option<f64>,
    /// Range of the free cash flow margin over the last 5 years.
    pub margin_range: f64,
    pub has_jump: bool,
    /// Mean of `acquisitions_net_of_cash` / revenue over the last 5 years up to T (a missing value counts as 0).
    pub acq_past: f64,
    /// P10, P50, P90 of the revenue of T+1 to T+3.
    pub rev_q: [[f64; 3]; HORIZON],
    pub rev_act: [f64; HORIZON],
    pub fcff_q: [[f64; 3]; HORIZON],
    pub fcff_act: [Option<f64>; HORIZON],
}

/// The free cash flow of a fiscal year as the model defines it.
pub fn fcff_of(y: &YearData, tax: f64) -> Option<f64> {
    Some(y.operating_cash_flow? - y.capex? + (1.0 - tax) * y.interest.unwrap_or(0.0) - y.sbc.map_or(0.0, f64::abs))
}

/// The free cash flow of a fiscal year as the model of `rules` defines it: `fcff_of`, less the after-tax interest income
/// (`no_interest_income`), less the cash paid for businesses (`acq_charge`), with no SBC cost when `sbc_cost` is off.
pub fn fcff_of_rules(y: &YearData, tax: f64, rules: &Rules) -> Option<f64> {
    let mut v = fcff_of(y, tax)?;
    if !rules.sbc_cost {
        v += y.sbc.map_or(0.0, f64::abs);
    }
    if rules.no_interest_income {
        v -= (1.0 - tax) * y.interest_income.unwrap_or(0.0);
    }
    if rules.acq_charge || rules.acq_f.is_some() || rules.acq_w {
        v -= y.acquisitions.unwrap_or(0.0).abs();
    }
    Some(v)
}

/// The model of a company at the end of a fiscal year, from the years up to and including `years[b]` only. Nothing of
/// the years after `b` enters (a test changes them and checks that the model is the same).
pub fn model_at(ticker: &str, years: &[YearData], b: usize, table: &ProcessTable, fade: &[FadeCell], rules: &Rules) -> Result<AutoModel, Refusal> {
    let inputs = AutoInputs { ticker: ticker.to_string(), template: "standard".into(), status: "PASS".into(), years: years[..=b].to_vec(), ..Default::default() };
    build(&inputs, table, fade, &Market::default(), rules, BuildOptions { valuation: false })
}

/// The base years of a company that the test uses: the index `b` in `years` and the label year of the fiscal year.
pub fn base_years(years: &[YearData]) -> Vec<(usize, i32, bool)> {
    let mut counts = RuleCounts::default();
    let rows = clean_series(raw_years(years), &mut counts);
    let idx_of = |end: &str| years.iter().position(|y| y.end == end);
    let mut out = Vec::new();
    for b in (NEED_BEFORE - 1)..rows.len().saturating_sub(HORIZON) {
        let lo = b + 1 - (NEED_BEFORE - 1);
        if !rows[lo..=b + HORIZON].iter().all(|r| r.prev_ok) {
            continue;
        }
        let has_jump = rows[lo..=b + HORIZON].iter().any(|r| r.jump);
        if let Some(i) = idx_of(&rows[b].end) {
            out.push((i, rows[b].year, has_jump));
        }
    }
    out
}

/// What a run of the test needs besides the data and the table.
pub struct PitParams<'a> {
    pub rules: &'a Rules,
    /// Paths of each model.
    pub paths: usize,
    pub seed: u64,
    /// Base years before this label year are not used (the out of sample test).
    pub min_year: Option<i32>,
}

/// All observations of one company.
pub fn company_observations(ticker: &str, years: &[YearData], table: &ProcessTable, fade: &[FadeCell], p: &PitParams) -> Vec<PitObs> {
    let mut counts = RuleCounts::default();
    let rows = clean_series(raw_years(years), &mut counts);
    let mut out = Vec::new();
    for (b, year, has_jump) in base_years(years) {
        if p.min_year.is_some_and(|m| year < m) {
            continue;
        }
        let Ok(model) = model_at(ticker, years, b, table, fade, p.rules) else { continue };
        let Ok(mut spec) = PathsSpec::from_toml(&model.toml) else { continue };
        spec.projection_years = HORIZON;
        let Ok(eng) = PathEngine::new(&spec) else { continue };
        let Ok(sim) = eng.simulate_one_thread(p.paths, p.seed, &Mods::default(), true) else { continue };
        let q = |t: usize, f: usize| -> [f64; 3] {
            let s = sorted_copy(&sim.column(t, f));
            [percentile(&s, 10.0), percentile(&s, 50.0), percentile(&s, 90.0)]
        };
        // the later years: the years after `b` in sequence
        let later: Vec<&YearData> = years[b + 1..].iter().take(HORIZON).collect();
        if later.len() < HORIZON {
            continue;
        }
        let tax = model.summary.tax;
        let mut obs = PitObs {
            ticker: ticker.to_string(),
            year,
            size: bucket(model.summary.base_revenue),
            base: model.summary.base_revenue,
            g_last: None,
            margin_range: 0.0,
            has_jump,
            acq_past: years[..=b].iter().rev().take(5).map(|y| y.acquisitions.unwrap_or(0.0).abs() / y.revenue.filter(|r| *r > 0.0).unwrap_or(f64::INFINITY)).sum::<f64>() / years[..=b].iter().rev().take(5).count() as f64,
            rev_q: [[0.0; 3]; HORIZON],
            rev_act: [0.0; HORIZON],
            fcff_q: [[0.0; 3]; HORIZON],
            fcff_act: [None; HORIZON],
        };
        let mut ok = true;
        for (h, year_h) in later.iter().enumerate() {
            obs.rev_q[h] = q(h, F_REV);
            obs.fcff_q[h] = q(h, F_FCFF);
            match year_h.revenue {
                Some(r) => obs.rev_act[h] = r,
                None => ok = false,
            }
            obs.fcff_act[h] = fcff_of_rules(year_h, tax, p.rules);
        }
        if !ok {
            continue;
        }
        // growth of the base year and the margin range from the data up to T
        let k = rows.iter().position(|r| r.end == years[b].end);
        if let Some(k) = k {
            obs.g_last = if rows[k].prev_ok { rows[k].growth_organic() } else { None };
        }
        let margins: Vec<f64> = years[..=b].iter().rev().take(5).filter_map(|y| y.fcf_margin()).collect();
        if margins.len() >= 2 {
            let (lo, hi) = (margins.iter().cloned().fold(f64::INFINITY, f64::min), margins.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
            obs.margin_range = hi - lo;
        }
        out.push(obs);
    }
    out
}

/// Counts of a set of observations at one horizon.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Cover {
    pub n: usize,
    pub below: usize,
    pub inside: usize,
    pub above: usize,
    /// The real value is below the P50.
    pub below_p50: usize,
}

impl Cover {
    fn add(&mut self, actual: f64, q: &[f64; 3]) {
        self.n += 1;
        if actual < q[0] {
            self.below += 1;
        } else if actual > q[2] {
            self.above += 1;
        } else {
            self.inside += 1;
        }
        if actual < q[1] {
            self.below_p50 += 1;
        }
    }
    pub fn shares(&self) -> [f64; 3] {
        let n = self.n.max(1) as f64;
        [self.below as f64 / n * 100.0, self.inside as f64 / n * 100.0, self.above as f64 / n * 100.0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Series {
    Revenue,
    Fcff,
}

/// Coverage of the observations that pass `keep`, at horizon `h`.
pub fn coverage(obs: &[PitObs], s: Series, h: usize, keep: &dyn Fn(&PitObs) -> bool) -> Cover {
    let mut c = Cover::default();
    for o in obs.iter().filter(|o| keep(o)) {
        match s {
            Series::Revenue => c.add(o.rev_act[h], &o.rev_q[h]),
            Series::Fcff => {
                if let Some(a) = o.fcff_act[h] {
                    c.add(a, &o.fcff_q[h]);
                }
            }
        }
    }
    c
}

/// Median error of the P50: revenue as a share (`actual / P50 - 1`); free cash flow as a share of the base revenue.
pub fn median_error(obs: &[PitObs], s: Series, h: usize, keep: &dyn Fn(&PitObs) -> bool) -> Option<f64> {
    let mut v: Vec<f64> = obs
        .iter()
        .filter(|o| keep(o))
        .filter_map(|o| match s {
            Series::Revenue => Some(o.rev_act[h] / o.rev_q[h][1] - 1.0),
            Series::Fcff => o.fcff_act[h].map(|a| (a - o.fcff_q[h][1]) / o.base),
        })
        .collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    Some(percentile(&v, 50.0))
}

fn growth_class(o: &PitObs) -> usize {
    match o.g_last {
        None => 5,
        Some(g) if g < 0.0 => 0,
        Some(g) if g < 0.10 => 1,
        Some(g) if g < 0.20 => 2,
        Some(g) if g < 0.50 => 3,
        Some(_) => 4,
    }
}

const GROWTH_CLASSES: [&str; 6] = ["growth < 0%", "0 to 10%", "10 to 20%", "20 to 50%", "above 50%", "no growth value"];

fn margin_class(o: &PitObs) -> usize {
    if o.margin_range < 0.05 {
        0
    } else if o.margin_range < 0.10 {
        1
    } else {
        2
    }
}

fn acq_class(o: &PitObs) -> usize {
    if o.acq_past < 0.005 {
        0
    } else if o.acq_past < 0.02 {
        1
    } else if o.acq_past < 0.05 {
        2
    } else {
        3
    }
}

const ACQ_CLASSES: [&str; 4] = ["spend < 0.5%", "0.5% to 2%", "2% to 5%", "above 5%"];

const MARGIN_CLASSES: [&str; 3] = ["margin range < 5 pt", "5 to 10 pt", "above 10 pt"];

fn pct_cell(c: &Cover) -> String {
    if c.n == 0 {
        return "      -        ".to_string();
    }
    let s = c.shares();
    format!("{:>4.1}/{:>4.1}/{:>4.1}", s[0], s[1], s[2])
}

fn err_cell(x: Option<f64>) -> String {
    x.map_or("     -".to_string(), |v| format!("{:>+6.1}%", v * 100.0))
}

/// A subset of the observations.
type SizeFilter = Box<dyn Fn(&PitObs) -> bool>;

/// The text of the result.
pub fn render(obs: &[PitObs], header: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{header}");
    let _ = writeln!(out, "Observations: {} company-years of {} companies.", obs.len(), obs.iter().map(|o| o.ticker.as_str()).collect::<std::collections::BTreeSet<_>>().len());
    let _ = writeln!(out, "Cells: real value below P10 / inside P10 to P90 / above P90, in percent (target 10 / 80 / 10). n is the number of company-years.");
    let all = |_: &PitObs| true;
    let nojump = |o: &PitObs| !o.has_jump;
    for (s, name) in [(Series::Revenue, "REVENUE"), (Series::Fcff, "FREE CASH FLOW (model definition)")] {
        let _ = writeln!(out, "\n{name} by size bucket (revenue of T) and horizon");
        let _ = writeln!(out, "  {:<8} {:>6}  {:^15} {:^15} {:^15}   {:>7} {:>7} {:>7}   real below P50: h1 h2 h3 (target 50%)", "size", "n", "T+1", "T+2", "T+3", "err T+1", "err T+2", "err T+3");
        let mut rows: Vec<(String, SizeFilter)> = (0..5).map(|b| (BUCKET_LABELS[b].to_string(), Box::new(move |o: &PitObs| o.size == b) as SizeFilter)).collect();
        rows.push(("all".into(), Box::new(all)));
        rows.push(("no jump".into(), Box::new(nojump)));
        for (label, keep) in &rows {
            let c: Vec<Cover> = (0..HORIZON).map(|h| coverage(obs, s, h, keep.as_ref())).collect();
            if c[0].n == 0 {
                continue;
            }
            let _ = writeln!(
                out,
                "  {:<8} {:>6}  {:^15} {:^15} {:^15}   {} {} {}   {:>4.0}% {:>4.0}% {:>4.0}%",
                label,
                c[0].n,
                pct_cell(&c[0]),
                pct_cell(&c[1]),
                pct_cell(&c[2]),
                err_cell(median_error(obs, s, 0, keep.as_ref())),
                err_cell(median_error(obs, s, 1, keep.as_ref())),
                err_cell(median_error(obs, s, 2, keep.as_ref())),
                c[0].below_p50 as f64 / c[0].n as f64 * 100.0,
                c[1].below_p50 as f64 / c[1].n.max(1) as f64 * 100.0,
                c[2].below_p50 as f64 / c[2].n.max(1) as f64 * 100.0,
            );
        }
        let _ = writeln!(out, "  (err = median of the real value over the P50 less 1, for revenue; median of (real - P50) / revenue of T, for the free cash flow)");
        let _ = writeln!(out, "\n{name} by growth of the base year (organic)");
        for (k, label) in GROWTH_CLASSES.iter().enumerate() {
            let keep = move |o: &PitObs| growth_class(o) == k;
            let c: Vec<Cover> = (0..HORIZON).map(|h| coverage(obs, s, h, &keep)).collect();
            if c[0].n < 20 {
                continue;
            }
            let _ = writeln!(
                out,
                "  {:<16} {:>6}  {:^15} {:^15} {:^15}   {} {} {}",
                label,
                c[0].n,
                pct_cell(&c[0]),
                pct_cell(&c[1]),
                pct_cell(&c[2]),
                err_cell(median_error(obs, s, 0, &keep)),
                err_cell(median_error(obs, s, 1, &keep)),
                err_cell(median_error(obs, s, 2, &keep))
            );
        }
        let _ = writeln!(out, "\n{name} by acquisition spend of the last 5 years (mean of acquisitions_net_of_cash / revenue)");
        for (k, label) in ACQ_CLASSES.iter().enumerate() {
            let keep = move |o: &PitObs| acq_class(o) == k;
            let c: Vec<Cover> = (0..HORIZON).map(|h| coverage(obs, s, h, &keep)).collect();
            if c[0].n < 20 {
                continue;
            }
            let _ = writeln!(
                out,
                "  {:<22} {:>6}  {:^15} {:^15} {:^15}   {} {} {}   real below P50: {:>3.0}% {:>3.0}% {:>3.0}%",
                label,
                c[0].n,
                pct_cell(&c[0]),
                pct_cell(&c[1]),
                pct_cell(&c[2]),
                err_cell(median_error(obs, s, 0, &keep)),
                err_cell(median_error(obs, s, 1, &keep)),
                err_cell(median_error(obs, s, 2, &keep)),
                c[0].below_p50 as f64 / c[0].n as f64 * 100.0,
                c[1].below_p50 as f64 / c[1].n.max(1) as f64 * 100.0,
                c[2].below_p50 as f64 / c[2].n.max(1) as f64 * 100.0
            );
        }
        let _ = writeln!(out, "\n{name} by range of the free cash flow margin of the last 5 years (a cyclical or unstable margin has a large range)");
        for (k, label) in MARGIN_CLASSES.iter().enumerate() {
            let keep = move |o: &PitObs| margin_class(o) == k;
            let c: Vec<Cover> = (0..HORIZON).map(|h| coverage(obs, s, h, &keep)).collect();
            if c[0].n < 20 {
                continue;
            }
            let _ = writeln!(
                out,
                "  {:<22} {:>6}  {:^15} {:^15} {:^15}   {} {} {}",
                label,
                c[0].n,
                pct_cell(&c[0]),
                pct_cell(&c[1]),
                pct_cell(&c[2]),
                err_cell(median_error(obs, s, 0, &keep)),
                err_cell(median_error(obs, s, 1, &keep)),
                err_cell(median_error(obs, s, 2, &keep))
            );
        }
    }
    out
}

pub const LIMITS: &str = "\
LIMITS
  - Not fully out of sample unless the result says OUT OF SAMPLE: the parameter table was estimated on all years of the cache.
  - Restated years: the mapping takes the value of the latest filing for each year, so a restatement after T enters the data of T.
  - Survivorship: the cache has the companies that exist now.
  - The value (the P50) is not tested here: `fa dcf returns` tests it against the later stock returns.
  - The model of the test has no failure hazard, no price and a fixed default rate; the revenue and the free cash flow do not depend on them.";

pub fn load_company_years(registry: &Registry, cache: &Path, ticker: &str) -> Option<(String, Vec<YearData>)> {
    let path = mapcheck::run::facts_path(cache, ticker)?;
    let facts = mapcheck::run::read_facts(&path).ok()?;
    if !facts.has_us_gaap() {
        return None;
    }
    let ends = period_ends(&facts, &registry.anchors, Forms::Annual, None);
    let periods: Vec<_> = ends.iter().map(|e| resolve_period_with(registry, &facts, e, Forms::Annual, None)).collect();
    let template = periods.last().map(|p| filer_template(registry, p)).unwrap_or("standard").to_string();
    Some((template, periods.iter().map(|p| YearData::from_period(registry, p)).collect()))
}

/// The rules of a `--rules` text.
pub fn parse_rules(text: &str) -> R<Rules> {
    let mut r = Rules::default();
    for part in text.split(',').filter(|p| !p.is_empty()) {
        let (k, v) = part.split_once('=').ok_or_else(|| DcfError(format!("--rules: expected K=V, got '{part}'")))?;
        // `off` is 0, only for `long_run` (0 or less is off) and for the flags (0 or 1 rules); any other key is an error
        const OFF_KEYS: [&str; 16] = [
            "long_run", "history_organic", "fade_to_cap", "equity_weight_one", "terminal_rf", "margin_last", "sbc_cost", "bridge_lti", "bridge_leases",
            "no_interest_income", "shares_latest", "stub", "mid_year", "acq_charge", "acq_w", "acq_fade",
        ];
        let x: f64 = if v == "off" {
            if !OFF_KEYS.contains(&k) {
                return Err(DcfError(format!("--rules: '{k}=off' is not allowed (only long_run and the flags take 'off'); give a number")));
            }
            0.0
        } else {
            v.parse().map_err(|_| DcfError(format!("--rules: bad number '{v}'")))?
        };
        match k {
            "history_organic" => r.history_organic = x != 0.0,
            "margin_pool_se" => r.margin_pool_se = x,
            "fade_to_cap" => r.fade_to_cap = x != 0.0,
            "own_sigma_nu" => r.own_sigma_nu = x,
            "level_kappa" => r.level_kappa = x,
            "erp" => r.erp.0 = x,
            "beta_weight" => r.beta_weight = x,
            // an explicit mean terminal growth is the old model for that number: it turns the one long-run growth off
            // (a later `long_run=` turns it on again)
            "terminal_growth" => {
                r.terminal_growth.0 = x;
                r.long_run = None;
            }
            "years" => r.years = (x as usize).clamp(3, 40),
            "rf" => r.rf_fixed = Some(x),
            "equity_weight_one" => r.equity_weight_one = x != 0.0,
            "terminal_rf" => r.terminal_rf = x != 0.0,
            "long_cap" => r.long_cap = Some(x),
            "long_run" => r.long_run = if x > 0.0 { Some(x) } else { None },
            "fade_years" => r.fade_years = Some(x as usize),
            "growth_w" => r.growth_w = Some(x),
            "growth_phi" => r.growth_phi = Some(x),
            "margin_w" => r.margin_w = Some(x),
            "margin_last" => r.margin_last = x != 0.0,
            "sbc_cost" => r.sbc_cost = x != 0.0,
            "bridge_lti" => r.bridge_lti = x != 0.0,
            "bridge_leases" => r.bridge_leases = x != 0.0,
            "no_interest_income" => r.no_interest_income = x != 0.0,
            "shares_latest" => r.shares_latest = x != 0.0,
            "stub" => r.stub = x != 0.0,
            "mid_year" => r.mid_year = x != 0.0,
            "acq_charge" => r.acq_charge = x != 0.0,
            "acq_f" => r.acq_f = Some(x),
            "acq_mult" => r.acq_mult = x,
            "acq_w" => r.acq_w = x != 0.0,
            "acq_fade" => r.acq_fade = x != 0.0,
            _ => return Err(DcfError(format!("--rules: unknown rule '{k}'"))),
        }
    }
    Ok(r)
}

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

/// `fa dcf pit`. Returns the exit code.
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
        println!("{USAGE}");
        return Ok(());
    }
    let known = ["--cache", "--table", "--cut", "--paths", "--seed", "--rules", "--out", "--limit"];
    let mut i = 0;
    while i < args.len() {
        if known.contains(&args[i].as_str()) {
            i += 1;
        } else {
            return Err(DcfError(format!("unknown option '{}'", args[i])));
        }
        i += 1;
    }
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let table_file = opt(args, "--table").map(PathBuf::from).unwrap_or_else(|| repo.join(DEFAULT_TABLE));
    let paths: usize = opt(args, "--paths").map_or(Ok(2000), |v| v.parse().map_err(|_| DcfError(format!("bad --paths '{v}'"))))?;
    let seed: u64 = opt(args, "--seed").map_or(Ok(7), |v| v.parse().map_err(|_| DcfError(format!("bad --seed '{v}'"))))?;
    let cut: Option<i32> = opt(args, "--cut").map(|v| v.parse().map_err(|_| DcfError(format!("bad --cut '{v}'")))).transpose()?;
    let limit: Option<usize> = opt(args, "--limit").map(|v| v.parse().map_err(|_| DcfError(format!("bad --limit '{v}'")))).transpose()?;
    let rules = parse_rules(opt(args, "--rules").unwrap_or(""))?;

    let t0 = Instant::now();
    let registry = Registry::embedded().map_err(DcfError)?;
    let mut tickers = super::panel::cache_tickers(&cache).map_err(DcfError)?;
    if let Some(l) = limit {
        tickers.truncate(l);
    }
    let companies: Vec<(String, Vec<YearData>)> = tickers
        .par_iter()
        .filter_map(|t| load_company_years(&registry, &cache, t).filter(|(tpl, _)| tpl == "standard").map(|(_, y)| (t.clone(), y)))
        .collect();
    let load_s = t0.elapsed().as_secs_f64();

    let (table, fade, label) = match cut {
        None => {
            let (t, f) = load_table(&table_file)?;
            (t, f, format!("IN SAMPLE: parameter table {} (estimated on all years)", table_file.display()))
        }
        Some(y) => {
            let panel = build_panel(&cache, None).map_err(DcfError)?;
            let trunc: Vec<CompanyPanel> = panel
                .companies
                .iter()
                .map(|c| CompanyPanel { rows: c.rows.iter().filter(|r| r.year <= y).cloned().collect(), ..c.clone() })
                .filter(|c| !c.rows.is_empty())
                .collect();
            let cells = estimate_table(&trunc, 2);
            let fade = fade_cells(&trunc, &cells);
            let meta = TableMeta { date: String::new(), command: format!("estimated from the years up to {y}"), companies: trunc.len(), company_years: 0, growth_observations: 0, note: String::new() };
            (ProcessTable { meta, cell: cells }, fade, format!("OUT OF SAMPLE: parameter table estimated from the years up to {y}; base years {y} and later"))
        }
    };
    let t1 = Instant::now();
    let obs: Vec<PitObs> = companies.par_iter().flat_map(|(t, years)| company_observations(t, years, &table, &fade, &PitParams { rules: &rules, paths, seed, min_year: cut })).collect();
    let run_s = t1.elapsed().as_secs_f64();
    let header = format!(
        "POINT-IN-TIME TEST of the automatic model. {label}\nCommand: fa dcf pit {}\nRules: {:?}\n{} companies of the standard template; load {load_s:.1} s, models and paths {run_s:.1} s ({paths} paths each, seed {seed}).",
        args.join(" "),
        rules,
        companies.len()
    );
    let mut text = render(&obs, &header);
    text.push('\n');
    text.push_str(LIMITS);
    text.push('\n');
    print!("{text}");
    if let Some(path) = opt(args, "--out") {
        std::fs::write(path, &text).map_err(|x| DcfError(format!("{path}: {x}")))?;
    }
    Ok(())
}
