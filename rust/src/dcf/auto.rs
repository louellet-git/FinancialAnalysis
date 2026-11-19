//! The automatic model: a path model (`engine = "paths"`) for a ticker, made from the standard variables,
//! the parameter table of the calibration and the yield curve, with no hand input (ticket
//! `T-20261119-dcf-auto-model`).
//!
//! The model builder does not read the network. It reads values that the caller gives (`AutoInputs`, `Market`) and
//! the table. Each number has a stated source and rule (`ExplainRow`); the rules are written in
//! `docs/ARCHITECTURE.md`, section "The automatic model". An automatic model is a screen, not a reviewed valuation.
//!
//! Rules in short (symbols: `phi`, `mean`, `tau`, `sigma` are the table values of the cell of the size bucket and
//! the template `standard`; `n` is the number of values in the history):
//!
//! - Base revenue: revenue of the latest fiscal year.
//! - Growth: the AR(1) of the table around the company mean. The company mean is the mean of the last 4 growth
//!   values shrunk to the cell mean with the weight `w = tau^2 / (tau^2 + sigma^2 * mvf(phi, n))` (the weight that
//!   `calib_bands` uses). Year 1: `mean_c + phi (last - mean_c)` with the standard deviation
//!   `sqrt((1 - phi)^2 (1 - w) tau^2 + sigma^2)`. Slow part: a linear fade of the company mean to the long-run
//!   cap (the 10-year zero yield) over the horizon, with the half-life of the fade table. Fast part: `phi` and
//!   `sigma` of the cell.
//! - Margin: free cash flow before SBC and before interest: `(OCF - capex_total + (1 - t) interest) / revenue`. The
//!   history is shrunk in the space of the table (cash flow after interest) and the mean interest ratio of the
//!   company is added back.
//! - SBC: a ratio, the mean of the last 3 years, a cost of the model (the repo convention).
//! - Net debt: `total_debt` less `cash_and_short_term_investments`.

use super::calibrate::{level_spread_ratio, mean_variance_factor, slow_half_life, Ar1Params, FadeCell, ProcessPriors, ProcessTable, GROWTH_LIMITS, MARGIN_LIMIT};
use super::panel::{bucket, clean_series, RawYear, RuleCounts, BUCKET_LABELS};
use mapcheck::mapper::PeriodData;
use mapcheck::registry::Registry;
use std::fmt::{self, Write};

/// The path of the parameter table, relative to the repository.
pub const DEFAULT_TABLE: &str = "analysis/priors/process_priors_2026-11-19.toml";

/// Number of growth values of the history (as in `calib_bands`).
pub const HIST_GROWTH: usize = 4;
/// Number of margin values of the history.
pub const HIST_MARGIN: usize = 5;
/// Years of the history for the SBC ratio, the interest ratio and the tax rate.
pub const HIST_RATIO: usize = 3;
/// The risk-free rate when the yield curve is not available (stated default, with a warning).
pub const DEFAULT_RF: f64 = 0.042;

/// The values that the repo uses as a convention and the rules that have a number. Each has a reason in
/// `docs/ARCHITECTURE.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Rules {
    pub years: usize,
    /// Equity risk premium: mean, standard deviation (Damodaran implied value, `analysis/ANT/driver_model_paths.toml`
    /// of 2026-11-19; 4.20% on 2026-10-01 in `analysis/JNJ/driver_model.toml`; the standard deviation is the IBM convention).
    pub erp: (f64, f64),
    pub kd_spread: f64,
    pub terminal_growth: (f64, f64),
    pub statutory_tax: f64,
    /// Weight of the regression beta in the shrunk beta (the rest is 1.0).
    pub beta_weight: f64,
    /// Rate model of the repo (IBM and ANT files): Hull-White `a` and `sigma`.
    pub hw_a: f64,
    pub hw_sigma: f64,
    /// The company growth fades to the long-run cap (the 10-year zero yield). Off: the company mean stays.
    pub fade_to_cap: bool,
    /// Used when the table has no fade cells (years).
    pub default_slow_half_life: f64,
    /// Prior weight, in years of data, of the table shock size in the shock size of a company (the company residuals
    /// of its own history have the rest). 0: the table value for all companies.
    pub own_sigma_nu: f64,
    /// The shock of the growth is at least `kappa` times the level of the growth (a company in a high-growth state has a
    /// larger spread). Negative: the ratio of the fade table (spread of the growth one year after a start year, over the
    /// start growth). 0: off.
    pub level_kappa: f64,
    /// Smallest number of fiscal years in the chain (three: two growth values).
    pub min_years: usize,
    /// The growth history keeps a jump that has no cash for businesses behind it (organic: NVDA 2024); a jump with
    /// cash for businesses, or a jump that reverses, is left out. Off: every jump is left out (the rule of the table).
    pub history_organic: bool,
    /// A margin cell whose phi has a standard error above this value gives the phi of the cell of all sizes (the mean,
    /// tau and sigma stay those of the size cell). 0 or less: never.
    pub margin_pool_se: f64,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            years: 10,
            erp: (0.0441, 0.0075),
            kd_spread: 0.015,
            terminal_growth: (0.0225, 0.0075),
            statutory_tax: 0.21,
            beta_weight: 2.0 / 3.0,
            hw_a: 0.14,
            hw_sigma: 0.0075,
            fade_to_cap: true,
            default_slow_half_life: 4.0,
            own_sigma_nu: 4.0,
            level_kappa: -1.0,
            min_years: 3,
            history_organic: true,
            margin_pool_se: 0.08,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------------------------

/// One fiscal year, as plain values of the standard variables. A flow is the value of the year; a balance is the
/// value at the end of the year.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct YearData {
    pub end: String,
    pub filed: String,
    pub revenue: Option<f64>,
    pub operating_cash_flow: Option<f64>,
    /// `capex_total` (equipment and capitalized software), else `capex`. Positive.
    pub capex: Option<f64>,
    pub sbc: Option<f64>,
    /// `interest_paid`, else `interest_expense`. Positive.
    pub interest: Option<f64>,
    pub income_tax: Option<f64>,
    pub pretax_income: Option<f64>,
    pub acquisitions: Option<f64>,
    pub total_debt: Option<f64>,
    /// `cash_and_short_term_investments`, else `cash_and_equivalents` plus `short_term_investments`.
    pub cash: Option<f64>,
    pub shares: Option<f64>,
}

impl YearData {
    pub fn from_period(registry: &Registry, p: &PeriodData) -> YearData {
        let capex = p.get(registry, "capex_total").or_else(|| p.get(registry, "capex")).map(f64::abs);
        let interest = p.get(registry, "interest_paid").or_else(|| p.get(registry, "interest_expense")).map(f64::abs);
        let cash = p
            .get(registry, "cash_and_short_term_investments")
            .or_else(|| p.get(registry, "cash_and_equivalents").map(|c| c + p.get(registry, "short_term_investments").unwrap_or(0.0)));
        YearData {
            end: p.end.clone(),
            filed: p.filed.clone().unwrap_or_default(),
            revenue: p.get(registry, "revenue"),
            operating_cash_flow: p.get(registry, "operating_cash_flow"),
            capex,
            sbc: p.get(registry, "sbc_total"),
            interest,
            income_tax: p.get(registry, "income_tax_expense"),
            pretax_income: p.get(registry, "income_before_tax"),
            acquisitions: p.get(registry, "acquisitions_net_of_cash").map(f64::abs),
            total_debt: p.get(registry, "total_debt"),
            cash,
            shares: p.get(registry, "shares_diluted").or_else(|| p.get(registry, "shares_outstanding")),
        }
    }

    /// Operating cash flow less capex, divided by revenue (the free cash flow margin of the table).
    pub fn fcf_margin(&self) -> Option<f64> {
        let r = self.revenue.filter(|r| *r > 0.0)?;
        Some((self.operating_cash_flow? - self.capex?) / r)
    }
}

/// The balance sheet and the share count of the latest period. `memo` items are not in the bridge.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BalanceData {
    pub end: String,
    pub form: String,
    pub total_debt: Option<f64>,
    pub cash: Option<f64>,
    pub shares: Option<f64>,
    pub pension: Option<f64>,
    pub minority: Option<f64>,
    pub operating_leases: Option<f64>,
}

impl BalanceData {
    pub fn from_period(registry: &Registry, p: &PeriodData, form: &str) -> BalanceData {
        let y = YearData::from_period(registry, p);
        BalanceData {
            end: p.end.clone(),
            form: form.to_string(),
            total_debt: y.total_debt,
            cash: y.cash,
            shares: y.shares,
            pension: p.get(registry, "pension_liabilities_noncurrent"),
            minority: p.get(registry, "noncontrolling_interest"),
            operating_leases: p.get(registry, "operating_lease_liabilities"),
        }
    }

    /// The balance of a fiscal year of the history (no memo items).
    pub fn from_year(y: &YearData) -> BalanceData {
        BalanceData { end: y.end.clone(), form: "10-K".into(), total_debt: y.total_debt, cash: y.cash, shares: y.shares, ..Default::default() }
    }
}

/// One finding of the mapping checks (`mapcheck`).
#[derive(Debug, Clone, PartialEq)]
pub struct FindingText {
    pub severity: String,
    pub code: String,
    pub detail: String,
}

/// All that the model builder reads about a company.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AutoInputs {
    pub ticker: String,
    pub name: String,
    /// `standard`, `bank`, `insurer`, `reit`, `fund`, `foreign`, `new`, `non-usd`, `no-revenue`.
    pub template: String,
    /// `PASS`, `WARN`, `FAIL`, `N/A` or `ERROR` (`mapcheck`).
    pub status: String,
    pub error: Option<String>,
    pub findings: Vec<FindingText>,
    /// Fiscal years, oldest first.
    pub years: Vec<YearData>,
    pub balance: Option<BalanceData>,
}

/// The inputs of a mapping report of `mapcheck`.
pub fn inputs_from_report(registry: &Registry, r: &mapcheck::run::TickerReport) -> AutoInputs {
    let mut out = AutoInputs {
        ticker: r.ticker.clone(),
        name: r.name.clone(),
        template: r.template.clone(),
        status: r.status.label().to_string(),
        error: r.error.clone(),
        findings: r.findings.iter().map(|f| FindingText { severity: f.severity.label().to_string(), code: f.code.clone(), detail: f.detail.clone() }).collect(),
        ..Default::default()
    };
    if let Some(c) = &r.company {
        out.years = c.annual.iter().map(|p| YearData::from_period(registry, p)).collect();
        // The latest period with a balance sheet: a later 10-Q first, then the fiscal year.
        let last_year = c.annual.last();
        let newer_quarter = c.quarterly.last().filter(|q| last_year.is_none_or(|y| q.end > y.end));
        let usable = |b: &BalanceData| b.cash.is_some() && b.shares.is_some();
        let mut balance = None;
        if let Some(q) = newer_quarter {
            let b = BalanceData::from_period(registry, q, "10-Q");
            if usable(&b) {
                balance = Some(b);
            }
        }
        if balance.is_none() {
            balance = last_year.map(|y| BalanceData::from_period(registry, y, "10-K"));
        }
        out.balance = balance;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Market inputs: the yield curve, the price, the beta
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct BetaFit {
    pub raw: f64,
    pub adjusted: f64,
    pub weeks: usize,
    pub first: String,
    pub last: String,
    pub market: String,
}

/// What the model needs from the market. All parts are optional; a missing part gets a stated default and a warning.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Market {
    /// The date of the model (the curve is the last GSW row on or before it).
    pub as_of: String,
    /// The six numbers of one GSW day, as published (`beta` in percent).
    pub curve: Option<CurveNumbers>,
    pub price: Option<f64>,
    pub price_source: String,
    pub beta: Option<BetaFit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurveNumbers {
    pub date: String,
    pub beta0: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub beta3: f64,
    pub tau1: f64,
    pub tau2: f64,
    /// Zero-coupon yield of 10 years (decimal).
    pub zero10: f64,
}

/// The last GSW row on or before `as_of` (`YYYY-MM-DD`) from the file of the cache (`<cache>/rates/gsw_feds200628.csv`).
pub fn load_curve(cache: &std::path::Path, as_of: &str) -> Result<CurveNumbers, String> {
    let path = cache.join("rates").join("gsw_feds200628.csv");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let ds = crate::forwards::parse_gsw(&text)?;
    let (date, day) = crate::forwards::parse_date_arg(as_of)?;
    let f = crate::forwards::forwards_on(&ds, &date, day)?;
    let p = |n: &str| f.parameters.get(n).copied().ok_or_else(|| format!("GSW row has no {n}"));
    let zero10 = f.zero_yields.iter().find(|z| z.years == 10).map(|z| z.rate).ok_or("GSW row has no 10-year zero yield")?;
    Ok(CurveNumbers { date: f.date, beta0: p("BETA0")?, beta1: p("BETA1")?, beta2: p("BETA2")?, beta3: p("BETA3")?, tau1: p("TAU1")?, tau2: p("TAU2")?, zero10 })
}

fn week_of(date: &str) -> Option<i64> {
    let day = mapcheck::dates::parse(date.get(..10)?)?;
    Some((day + 3).div_euclid(7))
}

/// Weekly close (the last close of each week) of a list of `(date, close)`, as `(week, close)`.
fn weekly(prices: &[(String, f64)]) -> Vec<(i64, f64)> {
    let mut out: Vec<(i64, f64)> = Vec::new();
    for (d, c) in prices {
        if !(c.is_finite() && *c > 0.0) {
            continue;
        }
        let Some(w) = week_of(d) else { continue };
        match out.last_mut() {
            Some(last) if last.0 == w => last.1 = *c,
            _ => out.push((w, *c)),
        }
    }
    out
}

/// The regression beta of weekly returns of a stock on a market index, `weeks` weeks at most, up to `as_of`. The
/// raw beta is `cov / var`. The shrunk beta is `weight * raw + (1 - weight)`. Weeks with no price in one of the
/// two series are skipped (a return needs two weeks in sequence). At least 104 weekly returns are needed.
pub fn regression_beta(stock: &[(String, f64)], market: &[(String, f64)], as_of: &str, weeks: usize, weight: f64, name: &str) -> Option<BetaFit> {
    let cut = week_of(as_of)?;
    let s: std::collections::HashMap<i64, f64> = weekly(stock).into_iter().filter(|(w, _)| *w <= cut).collect();
    let m = weekly(market);
    let mut pairs: Vec<(f64, f64, i64)> = Vec::new();
    for k in 1..m.len() {
        let (w0, w1) = (m[k - 1].0, m[k].0);
        if w1 > cut || w1 - w0 != 1 {
            continue;
        }
        let (Some(s0), Some(s1)) = (s.get(&w0), s.get(&w1)) else { continue };
        pairs.push((s1 / s0 - 1.0, m[k].1 / m[k - 1].1 - 1.0, w1));
    }
    if pairs.len() > weeks {
        pairs.drain(..pairs.len() - weeks);
    }
    if pairs.len() < 104 {
        return None;
    }
    let n = pairs.len() as f64;
    let (ms, mm) = (pairs.iter().map(|p| p.0).sum::<f64>() / n, pairs.iter().map(|p| p.1).sum::<f64>() / n);
    let cov: f64 = pairs.iter().map(|p| (p.0 - ms) * (p.1 - mm)).sum();
    let var: f64 = pairs.iter().map(|p| (p.1 - mm) * (p.1 - mm)).sum();
    if var <= 0.0 {
        return None;
    }
    let raw = cov / var;
    let day = |w: i64| mapcheck::dates::format(w * 7 - 3);
    Some(BetaFit { raw, adjusted: weight * raw + (1.0 - weight), weeks: pairs.len(), first: day(pairs[0].2), last: day(pairs[pairs.len() - 1].2), market: name.to_string() })
}

fn price_series(cache: &std::path::Path, ticker: &str) -> Option<Vec<(String, f64)>> {
    let p = crate::market::prices::read_cache(&crate::market::prices::cache_path(cache, ticker)).ok()?;
    Some(p.into_iter().map(|x| (x.date, x.close)).collect())
}

pub fn index_from_cache(cache: &std::path::Path) -> Option<Vec<(String, f64)>> {
    // The price cache of SPY first: it has the same source and the same adjustment as the price files of
    // the tickers (`fa prices get SPY`). Then the daily file of the trading cache.
    if let Some(series) = price_series(cache, "SPY").filter(|s| s.len() >= 520) {
        return Some(series);
    }
    let text = std::fs::read_to_string(cache.join("trading").join("SPY_1d.csv")).ok()?;
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() >= 5 {
            if let Ok(c) = f[4].parse::<f64>() {
                out.push((f[0].to_string(), c));
            }
        }
    }
    Some(out)
}

/// The market inputs of a ticker from the cache: the curve, the last close, the regression beta of 5 years of
/// weekly returns against SPY (the index series of the trading cache). No network.
pub fn market_from_cache(cache: &std::path::Path, ticker: &str, as_of: &str, price: Option<f64>, rules: &Rules, curve: Option<&CurveNumbers>, index: Option<&[(String, f64)]>) -> Market {
    let stock = price_series(cache, ticker);
    let (price, source) = match (price, &stock) {
        (Some(p), _) => (Some(p), "--price".to_string()),
        (None, Some(s)) => s.last().map(|(d, c)| (Some(*c), format!("last close of the price cache ({d})"))).unwrap_or((None, String::new())),
        _ => (None, String::new()),
    };
    let beta = match (&stock, index) {
        (Some(s), Some(m)) => regression_beta(s, m, as_of, 260, rules.beta_weight, "SPY (price cache, else trading cache; daily close)"),
        _ => None,
    };
    Market { as_of: as_of.to_string(), curve: curve.cloned(), price, price_source: source, beta }
}

// ---------------------------------------------------------------------------------------------
// The result of the builder
// ---------------------------------------------------------------------------------------------

/// One number of the model: its value, its source and its rule.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplainRow {
    pub input: String,
    pub value: String,
    pub source: String,
    pub rule: String,
}

/// The numbers of the model that a screen row shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Summary {
    pub base_revenue: f64,
    pub base_end: String,
    pub growth_start: f64,
    pub growth_company_mean: f64,
    pub growth_long_run: f64,
    pub margin_start: f64,
    pub margin_target: f64,
    pub sbc_ratio: f64,
    pub net_debt: f64,
    pub shares: f64,
    pub beta: f64,
    pub equity_weight: f64,
    pub tax: f64,
    pub size: String,
    pub growth_obs: usize,
    pub margin_obs: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutoModel {
    /// The model file (TOML): `fa dcf drivers TICKER --model FILE` runs it.
    pub toml: String,
    pub rows: Vec<ExplainRow>,
    pub warnings: Vec<String>,
    pub summary: Summary,
}

/// The reasons for no value.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    /// No data (no company facts, not us-gaap).
    NoData(String),
    /// A template that has no model: bank, insurer, reit, fund, or a company that the mapping does not judge.
    Template(String),
    /// The mapping status is FAIL.
    Fail(String),
    /// A necessary variable has no value.
    Missing(Vec<String>),
    /// Too few fiscal years in a sequence.
    ShortHistory(String),
    /// The table has no cell for the company.
    NoPriors(String),
}

impl Refusal {
    /// A short key for tallies.
    pub fn key(&self) -> String {
        match self {
            Refusal::NoData(_) => "no data (not us-gaap, or no file)".to_string(),
            Refusal::Template(t) => format!("template {t}: no model"),
            Refusal::Fail(_) => "mapping status FAIL".to_string(),
            Refusal::Missing(v) => format!("missing {}", v.join("+")),
            Refusal::ShortHistory(_) => "history too short".to_string(),
            Refusal::NoPriors(_) => "no table cell".to_string(),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::NoData(m) => write!(f, "no data: {m}"),
            Refusal::Template(t) => write!(f, "no model for this template ({t}): a free-cash-flow DCF is the wrong tool for it"),
            Refusal::Fail(m) => write!(f, "the mapping status is FAIL: {m}"),
            Refusal::Missing(v) => write!(f, "necessary variable with no value: {}", v.join(", ")),
            Refusal::ShortHistory(m) => write!(f, "history too short: {m}"),
            Refusal::NoPriors(m) => write!(f, "no table cell: {m}"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Pieces of the rules (public so that a test can check each one with a fixed answer)
// ---------------------------------------------------------------------------------------------

/// The AR(1) of one series for one company: the shrunk company mean and the first step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ar1Fit {
    pub n: usize,
    pub mean_history: f64,
    /// Weight of the history mean in the company mean.
    pub w: f64,
    /// Company mean (shrunk).
    pub mu: f64,
    /// Variance of the company mean after the shrink.
    pub var_post: f64,
    pub phi: f64,
    pub sigma: f64,
    /// Expected value of year 1 and its standard deviation.
    pub start_mean: f64,
    pub start_sd: f64,
    /// Correlation of the start with the company mean.
    pub corr: f64,
}

/// The shrink and the first step. `hist`: the history values, oldest first; `last`: the value of the base year (if it
/// is not in `hist`, the first step starts at the company mean).
pub fn ar1_fit(hist: &[f64], last: Option<f64>, p: &Ar1Params) -> Ar1Fit {
    let n = hist.len();
    let phi = p.phi.clamp(-0.9, 0.99);
    let tau2 = p.mean_sd_units * p.mean_sd_units;
    let noise = if n == 0 { f64::INFINITY } else { p.sigma * p.sigma * mean_variance_factor(phi, n as f64) };
    let w = if n == 0 || tau2 + noise <= 0.0 { 0.0 } else { tau2 / (tau2 + noise) };
    let mean_history = if n == 0 { p.mean } else { hist.iter().sum::<f64>() / n as f64 };
    let mu = w * mean_history + (1.0 - w) * p.mean;
    let var_post = (1.0 - w) * tau2;
    let start_mean = match last {
        Some(l) => mu + phi * (l - mu),
        None => mu,
    };
    let start_sd = ((1.0 - phi) * (1.0 - phi) * var_post + p.sigma * p.sigma).sqrt();
    let corr = if start_sd > 0.0 && var_post > 0.0 { (1.0 - phi) * var_post.sqrt() / start_sd } else { 0.0 };
    Ar1Fit { n, mean_history, w, mu, var_post, phi, sigma: p.sigma, start_mean, start_sd, corr }
}

/// The shock size of a company: the table value with the weight `nu0` (in years) and the residuals of the company's
/// own history around its mean with one weight for each residual. `nu0 <= 0` or fewer than 3 values: the table value.
pub fn own_sigma(hist: &[f64], p: &Ar1Params, nu0: f64) -> f64 {
    let n = hist.len();
    if nu0 <= 0.0 || n < 3 {
        return p.sigma;
    }
    let m = hist.iter().sum::<f64>() / n as f64;
    let phi = p.phi.clamp(-0.9, 0.99);
    let e2: f64 = hist.windows(2).map(|w| ((w[1] - m) - phi * (w[0] - m)).powi(2)).sum::<f64>() * n as f64 / (n as f64 - 1.0);
    let k = (n - 1) as f64;
    ((nu0 * p.sigma * p.sigma + e2) / (nu0 + k)).sqrt()
}

/// Half-life in years of a persistence `phi` (years). `phi` is cut to [0.02, 0.95].
pub fn half_life_of(phi: f64) -> f64 {
    std::f64::consts::LN_2 / -(phi.clamp(0.02, 0.95)).ln()
}

/// The ramp of the linear fade that equals, in year `years`, the exponential fade of half-life `h`:
/// `(years - 1) / (N - 1) = 1 - 2^(-(years - 1)/h)`.
pub fn fade_ramp_years(h: f64, years: usize) -> usize {
    let share = 1.0 - 2f64.powf(-((years - 1) as f64) / h.max(0.1));
    ((1.0 + (years - 1) as f64 / share).round() as usize).clamp(2, 60)
}

/// Effective tax rate: the median of the tax over the pre-tax income of the last 5 years if there are 3 values or
/// more, each between 5% and 40% with a range of 10 points or less; else the statutory rate.
pub fn tax_rate(years: &[YearData], statutory: f64) -> (f64, String) {
    let vals: Vec<f64> = years.iter().rev().take(5).filter_map(|y| match (y.income_tax, y.pretax_income) {
        (Some(t), Some(p)) if p > 0.0 => Some(t / p),
        _ => None,
    }).collect();
    if vals.len() >= 3 {
        let (lo, hi) = (vals.iter().cloned().fold(f64::INFINITY, f64::min), vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        if lo >= 0.05 && hi <= 0.40 && hi - lo <= 0.10 {
            let mut s = vals.clone();
            s.sort_by(|a, b| a.total_cmp(b));
            return (s[s.len() / 2], format!("effective rate: median of {} years ({:.1}% to {:.1}%)", s.len(), lo * 100.0, hi * 100.0));
        }
        return (statutory, format!("statutory: the effective rates are not stable ({:.1}% to {:.1}% in {} years)", lo * 100.0, hi * 100.0, vals.len()));
    }
    (statutory, "statutory: fewer than 3 years with tax and positive pre-tax income".to_string())
}

/// The cell of the table for a margin: the cell of the size bucket and the template, else the size bucket, else the
/// template, else all, the first that has the series (`fcf_margin`).
fn margin_params(table: &ProcessTable, size: &str, template: &str, pool_se: f64) -> Option<(Ar1Params, String)> {
    let enough = |p: &Ar1Params| p.n_obs >= super::calibrate::MIN_OBS && p.n_units >= super::calibrate::MIN_UNITS;
    let find = |s: &str, t: &str| table.cell.iter().find(|c| c.size == s && c.template == t).and_then(|c| c.fcf_margin.as_ref()).filter(|p| enough(p));
    let (p, label) = [(size, template), (size, "all"), ("all", template), ("all", "all")].iter().find_map(|(s, t)| find(s, t).map(|p| (p.clone(), format!("size {s}, template {t}"))))?;
    if pool_se > 0.0 && p.phi_se > pool_se {
        if let Some(a) = find("all", template) {
            let note = format!("{label}; phi {:.3} (standard error {:.3}) is replaced by {:.3} of the cell of all sizes", p.phi, p.phi_se, a.phi);
            return Some((Ar1Params { phi: a.phi, phi_se: a.phi_se, half_life: a.half_life, ..p }, note));
        }
    }
    Some((p, label))
}

/// The cell for the growth of a company (`ProcessTable::priors_for`).
fn growth_cell(table: &ProcessTable, revenue: f64, template: &str) -> Option<ProcessPriors> {
    table.priors_for(revenue, template, None)
}

fn pct(x: f64) -> String {
    format!("{:.2}%", x * 100.0)
}

fn money(x: f64) -> String {
    if x.abs() >= 1e9 {
        format!("${:.2}B", x / 1e9)
    } else {
        format!("${:.1}M", x / 1e6)
    }
}

/// A float as a TOML float (always a decimal point or an exponent).
fn f(x: f64) -> String {
    format!("{x:?}")
}

fn q(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

/// A drawn number as a TOML inline table, or a plain number when the deviation is 0.
fn draw(mean: f64, sd: f64, lo: f64, hi: f64) -> String {
    if sd > 1e-9 {
        format!("{{ mean = {}, std = {}, lo = {}, hi = {} }}", f(mean), f(sd), f(lo), f(hi))
    } else {
        f(mean)
    }
}

/// The fiscal years as the rows of the panel rules (`clean_series`).
pub fn raw_years(years: &[YearData]) -> Vec<RawYear> {
    years
        .iter()
        .map(|y| RawYear {
            end: y.end.clone(),
            filed: y.filed.clone(),
            revenue: y.revenue,
            operating_income: None,
            operating_cash_flow: y.operating_cash_flow,
            capex: y.capex,
            sbc: y.sbc,
            acquisitions: y.acquisitions,
        })
        .collect()
}

/// The trailing chain of fiscal years with revenue and 350 to 380 days between the ends, as years of `YearData` and
/// the growth of each (`None` for the first year and for a jump: more than 60% and revenue of $1B or more).
fn chain(years: &[YearData], organic: bool) -> (Vec<YearData>, Vec<Option<f64>>, RuleCounts) {
    let raws = raw_years(years);
    let mut counts = RuleCounts::default();
    let rows = clean_series(raws, &mut counts);
    // the trailing chain
    let mut start = rows.len();
    while start > 0 {
        start -= 1;
        if !rows[start].prev_ok {
            break;
        }
    }
    let by_end: std::collections::HashMap<&str, &YearData> = years.iter().map(|y| (y.end.as_str(), y)).collect();
    let mut data = Vec::new();
    let mut growth = Vec::new();
    for (i, r) in rows[start..].iter().enumerate() {
        if let Some(y) = by_end.get(r.end.as_str()) {
            let mut y = (*y).clone();
            y.revenue = Some(r.revenue);
            data.push(y);
            growth.push(if i == 0 { None } else { (if organic { r.growth_organic() } else { r.growth_clean() }).filter(|g| *g > GROWTH_LIMITS.0 && *g < GROWTH_LIMITS.1) });
        }
    }
    (data, growth, counts)
}

// ---------------------------------------------------------------------------------------------
// The builder
// ---------------------------------------------------------------------------------------------

/// Which parts of the data the builder needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildOptions {
    /// False: a model for the revenue and the free cash flow only (the point-in-time test): net debt and shares are
    /// not read (a dummy value is used) and the template and the status are not judged.
    pub valuation: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        BuildOptions { valuation: true }
    }
}

/// Make the model of a company. `fade` are the cells of the fade table (empty: the default half-life, with a warning).
pub fn build(inputs: &AutoInputs, table: &ProcessTable, fade: &[FadeCell], market: &Market, rules: &Rules, opts: BuildOptions) -> Result<AutoModel, Refusal> {
    let ticker = inputs.ticker.as_str();
    let mut warnings: Vec<String> = Vec::new();
    let mut rows: Vec<ExplainRow> = Vec::new();
    let mut row = |input: &str, value: String, source: &str, rule: &str| rows.push(ExplainRow { input: input.into(), value, source: source.into(), rule: rule.into() });

    if opts.valuation {
        if inputs.status == "ERROR" {
            return Err(Refusal::NoData(inputs.error.clone().unwrap_or_else(|| "the data cannot be read".into())));
        }
        if inputs.template != "standard" {
            return Err(Refusal::Template(if inputs.template.is_empty() { "unknown".into() } else { inputs.template.clone() }));
        }
        if inputs.status == "FAIL" {
            let fails: Vec<String> = inputs.findings.iter().filter(|x| x.severity == "FAIL").map(|x| format!("{} ({})", x.code, x.detail)).collect();
            return Err(Refusal::Fail(fails.join("; ")));
        }
    }
    // the history
    let (hist, growth_all, _) = chain(&inputs.years, rules.history_organic);
    if hist.len() < rules.min_years {
        return Err(Refusal::ShortHistory(format!("{} fiscal years in sequence with revenue; {} are needed", hist.len(), rules.min_years)));
    }
    let last = hist.last().expect("not empty");
    let base_revenue = last.revenue.expect("chain has revenue");
    // necessary variables of the latest fiscal year
    let mut missing: Vec<String> = Vec::new();
    if last.operating_cash_flow.is_none() {
        missing.push("operating_cash_flow".into());
    }
    if last.capex.is_none() {
        missing.push("capex".into());
    }
    let balance = if opts.valuation {
        match &inputs.balance {
            Some(b) => b.clone(),
            None => return Err(Refusal::Missing(vec!["balance sheet".into()])),
        }
    } else {
        BalanceData::from_year(last)
    };
    if opts.valuation {
        if balance.cash.is_none() {
            missing.push("cash_and_equivalents".into());
        }
        if balance.shares.is_none() {
            missing.push("shares_diluted".into());
        }
    }
    if !missing.is_empty() {
        return Err(Refusal::Missing(missing));
    }
    let size = BUCKET_LABELS[bucket(base_revenue)].to_string();
    let gcell = growth_cell(table, base_revenue, "standard").ok_or_else(|| Refusal::NoPriors(format!("revenue {} template standard", money(base_revenue))))?;
    let gp = gcell.growth.clone().ok_or_else(|| Refusal::NoPriors("the cell has no growth series".into()))?;
    let (mp, mp_label) = margin_params(table, &size, "standard", rules.margin_pool_se).ok_or_else(|| Refusal::NoPriors("no cell has a free cash flow margin series".into()))?;

    // ---- tax
    let (tax, tax_rule) = tax_rate(&inputs.years, rules.statutory_tax);

    // ---- growth
    let growth_hist: Vec<f64> = growth_all.iter().rev().take(HIST_GROWTH).filter_map(|g| *g).rev().collect();
    let last_growth = growth_all.last().copied().flatten();
    if growth_all.last().is_some_and(|g| g.is_none()) && hist.len() >= 2 {
        let raw = hist[hist.len() - 1].revenue.unwrap_or(0.0) / hist[hist.len() - 2].revenue.unwrap_or(1.0) - 1.0;
        if raw.abs() > super::panel::JUMP_GROWTH {
            warnings.push(format!("the revenue growth of the latest year ({:+.0}%) is a jump (an acquisition or a spin-off is likely): it is not used for the growth history", raw * 100.0));
        }
    }
    let kappa = if rules.level_kappa < 0.0 {
        level_spread_ratio(fade).unwrap_or_else(|| {
            warnings.push("the table has no fade cells: the spread of high growth is the default 0.45 times the level".into());
            0.45
        })
    } else {
        rules.level_kappa
    };
    let sg_own = own_sigma(&growth_hist, &gp, rules.own_sigma_nu);
    let g_level = growth_hist.iter().map(|g| g.abs()).fold(last_growth.map_or(0.0, f64::abs), f64::max);
    let sg_eff = sg_own.max(kappa * g_level);
    let gfit = ar1_fit(&growth_hist, last_growth, &Ar1Params { sigma: sg_eff, ..gp.clone() });
    // the long-run cap and the company mean
    let rf_long = market.curve.as_ref().map_or(DEFAULT_RF, |c| c.zero10);
    if market.curve.is_none() {
        warnings.push(format!("no GSW yield curve: the risk-free rate is the default {} (a fixed rate)", pct(DEFAULT_RF)));
    }
    let mu = gfit.mu.clamp(-0.5, 1.5);
    let g_long = if rules.fade_to_cap { mu.min(rf_long) } else { mu };
    let slow_h = match slow_half_life(fade) {
        Some(h) => h.clamp(1.0, 10.0),
        None => {
            warnings.push(format!("the table has no fade cells: the half-life of the slow fade is the default {} years", rules.default_slow_half_life));
            rules.default_slow_half_life
        }
    };
    let ramp_n = fade_ramp_years(slow_h, rules.years);
    let drift_end = (g_long - mu) * 1.0;
    let g_start = gfit.start_mean.clamp(GROWTH_LIMITS.0 + 0.01, GROWTH_LIMITS.1 - 0.01);
    let g_half = half_life_of(gfit.phi);
    let g_lo = GROWTH_LIMITS.0;
    let g_hi = GROWTH_LIMITS.1;

    // ---- margin (free cash flow before SBC and before interest)
    let mhist_all: Vec<(f64, f64)> = hist
        .iter()
        .filter_map(|y| {
            let m = y.fcf_margin().filter(|m| m.abs() <= MARGIN_LIMIT)?;
            let rev = y.revenue?;
            Some((m, (1.0 - tax) * y.interest.unwrap_or(0.0) / rev))
        })
        .collect();
    let mhist: Vec<f64> = mhist_all.iter().rev().take(HIST_MARGIN).rev().map(|x| x.0).collect();
    let interest_ratio = {
        let v: Vec<f64> = mhist_all.iter().rev().take(HIST_RATIO).map(|x| x.1).collect();
        if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
    };
    let last_margin = last.fcf_margin().filter(|m| m.abs() <= MARGIN_LIMIT);
    let sm_own = own_sigma(&mhist, &mp, rules.own_sigma_nu);
    let mfit = ar1_fit(&mhist, last_margin, &Ar1Params { sigma: sm_own, ..mp.clone() });
    let m_start = (mfit.start_mean + interest_ratio).clamp(-MARGIN_LIMIT + 0.01, MARGIN_LIMIT - 0.01);
    let m_target = (mfit.mu + interest_ratio).clamp(-MARGIN_LIMIT + 0.01, MARGIN_LIMIT - 0.01);
    let m_half = half_life_of(mfit.phi);
    if m_target <= 0.0 {
        warnings.push(format!("the long-run free cash flow margin before SBC is {} (not above zero): the value is zero or negative", pct(m_target)));
    }
    if mhist.len() < 3 {
        warnings.push(format!("the free cash flow margin has {} values in the history: the company mean is mostly the table mean", mhist.len()));
    }
    if growth_hist.len() < 3 {
        warnings.push(format!("the growth history has {} values: the company mean is mostly the table mean", growth_hist.len()));
    }

    // ---- SBC
    let sbc_vals: Vec<f64> = hist.iter().rev().take(HIST_RATIO).filter_map(|y| Some(y.sbc?.abs() / y.revenue?)).collect();
    let sbc_ratio = if sbc_vals.is_empty() {
        warnings.push("no value of sbc_total in the last 3 years: the SBC ratio is 0".into());
        0.0
    } else {
        sbc_vals.iter().sum::<f64>() / sbc_vals.len() as f64
    };

    // ---- bridge and shares
    let debt = balance.total_debt.unwrap_or_else(|| {
        if opts.valuation {
            warnings.push("total_debt has no value: it is taken as 0".into());
        }
        0.0
    });
    let cash = balance.cash.unwrap_or(0.0);
    let net_debt = debt - cash;
    let shares = balance.shares.unwrap_or(1.0);

    // ---- discount
    let beta = match &market.beta {
        Some(b) => b.adjusted,
        None => {
            if opts.valuation {
                warnings.push("no price history of the ticker and of the market index in the cache: beta is 1.0".into());
            }
            1.0
        }
    };
    let weight = match market.price {
        Some(p) if opts.valuation && p > 0.0 => {
            let e = p * shares;
            (e / (e + debt.max(0.0))).clamp(0.2, 1.0)
        }
        _ => {
            if opts.valuation {
                warnings.push("no price: the equity weight is 1.0 (the cost of capital is the cost of equity)".into());
            }
            1.0
        }
    };

    // ---------------- explain rows
    row("base revenue", money(base_revenue), &format!("revenue, fiscal year {}", last.end), "latest fiscal year (same as the point-in-time test; the trailing four quarters are not used)");
    row(
        "growth: history",
        format!("{} values, mean {}; last {}", growth_hist.len(), pct(gfit.mean_history), last_growth.map_or("none".into(), pct)),
        "revenue, fiscal years in sequence (350-380 days); a jump above 60% (revenue of $1B or more) is left out",
        "the last 4 values",
    );
    row(
        "growth: table cell",
        format!("size {size}, phi {:.3}, mean {}, tau {}, sigma {}", gp.phi, pct(gp.mean), pct(gp.mean_sd_units), pct(gp.sigma)),
        &format!("{} ({} observations, {} companies)", DEFAULT_TABLE, gp.n_obs, gp.n_units),
        "cell of the size bucket and the template standard (priors_for)",
    );
    row(
        "growth: company mean",
        pct(gfit.mu),
        "history mean and cell mean",
        &format!("shrink: w = tau^2 / (tau^2 + sigma^2 mvf) = {:.3}; mean = w history + (1 - w) cell", gfit.w),
    );
    row(
        "growth: start (year 1)",
        format!("mean {}, std {}", pct(g_start), pct(gfit.start_sd)),
        "company mean, last growth, phi",
        "mean + phi (last - mean); std = sqrt((1 - phi)^2 var_mean + sigma^2): the AR(1) of the table",
    );
    row(
        "growth: long run",
        format!("{} (cap: 10-year zero yield {}); company mean {}", pct(g_long), pct(rf_long), pct(mu)),
        if market.curve.is_some() { "GSW curve of the cache" } else { "default rate" },
        &format!("the company mean fades linearly to the cap by the horizon: ramp of {ramp_n} years, equal in year {} to an exponential fade with half-life {slow_h:.1} years (fade table, k = 3 and 10)", rules.years),
    );
    row(
        "growth: fast part",
        format!("phi {:.3} (half-life {:.2} years), shock sigma {} (table {}), year-1 sigma {}, shock at least {:.2} times the level", gfit.phi, g_half, pct(sg_own), pct(gp.sigma), pct(sg_eff), kappa),
        "table cell; the residuals of the company history; fade table",
        &format!("AR(1) shock around the slow part. Company sigma = sqrt(({} gp.sigma^2 + sum of residual^2) / ({} + residuals)); the shock is at least kappa x the level (kappa = spread of the growth one year after a start year over the start growth, fade table)", rules.own_sigma_nu, rules.own_sigma_nu),
    );
    row(
        "margin: history",
        format!("{} values, mean {}; last {}; interest ratio (after tax) {}", mhist.len(), pct(mfit.mean_history), last_margin.map_or("none".into(), pct), pct(interest_ratio)),
        "operating_cash_flow, capex_total (else capex), interest_paid (else interest_expense), revenue",
        &format!("FCFF margin before SBC = (OCF - capex + (1 - t) interest) / revenue, t = {}. The history is shrunk as cash flow after interest (the table definition); the mean interest ratio of the last 3 years is added back", pct(tax)),
    );
    row(
        "margin: table cell",
        format!("phi {:.3}, mean {}, tau {}, sigma {}", mp.phi, pct(mp.mean), pct(mp.mean_sd_units), pct(mp.sigma)),
        &format!("{} (free cash flow margin series, {} observations)", DEFAULT_TABLE, mp.n_obs),
        &format!("cell: {mp_label}. Rule: the cell of the size bucket and the template standard; if it has no such series, the cell of the size, the template, then all; a phi with a standard error above {} is replaced by the phi of all sizes", rules.margin_pool_se),
    );
    row(
        "margin: start and target",
        format!("start {} (std {}), target {} (std {}), half-life {:.2} years, sigma {}", pct(m_start), pct(mfit.start_sd), pct(m_target), pct(mfit.var_post.sqrt()), m_half, pct(mfit.sigma)),
        "company mean (shrunk) and the table cell",
        &format!("same shrink as growth, w = {:.3}; target = shrunk mean + interest ratio; AR(1) with the half-life of phi", mfit.w),
    );
    row(
        "SBC ratio",
        pct(sbc_ratio),
        "sbc_total / revenue, last 3 years",
        "mean ratio, fixed; SBC is a cost of the model (repo convention: a cash-equivalent cost; existing awards are in the share count)",
    );
    row("tax rate", pct(tax), "income_tax_expense, income_before_tax", &tax_rule);
    row(
        "net debt",
        format!("{} = debt {} - cash {}", money(net_debt), money(debt), money(cash)),
        &format!("total_debt less cash_and_short_term_investments (cash_and_equivalents + short_term_investments), balance sheet {} ({})", balance.end, balance.form),
        "simple bridge. Left out: pension and retiree liabilities, operating leases, minority interests, litigation reserves, investments and equity-method stakes (the hand models of IBM and JNJ add some of them)",
    );
    let memo: Vec<String> = [("pension_liabilities_noncurrent", balance.pension), ("noncontrolling_interest", balance.minority), ("operating_lease_liabilities", balance.operating_leases)]
        .iter()
        .filter_map(|(n, v)| v.filter(|v| v.abs() > 0.0).map(|v| format!("{n} {} ({:+.2} a share)", money(v), -v / shares)))
        .collect();
    if !memo.is_empty() && opts.valuation {
        row("memo: not in the bridge", memo.join("; "), "balance sheet variables", "the value per share would change by the amount shown if the item were a claim in the bridge");
    }
    row("shares", format!("{:.1}M", shares / 1e6), &format!("shares_diluted (weighted average), period {} ({})", balance.end, balance.form), "latest period");
    row(
        "beta",
        match &market.beta {
            Some(b) => format!("{:.3} (regression {:.3}, {} weekly returns {} to {})", b.adjusted, b.raw, b.weeks, b.first, b.last),
            None => "1.000".into(),
        },
        match &market.beta {
            Some(b) => b.market.as_str(),
            None => "default",
        },
        &format!("weekly returns of 5 years against the market, shrunk to 1: {:.2} raw + {:.2} one; fixed (IBM F-review: beta is an estimation constant)", rules.beta_weight, 1.0 - rules.beta_weight),
    );
    row("equity risk premium", format!("{} (std {})", pct(rules.erp.0), pct(rules.erp.1)), "repo convention: Damodaran implied ERP, 4.41% in analysis/ANT/driver_model_paths.toml (2026-11-19)", "drawn, lower bound 2%");
    row(
        "risk-free rate",
        match &market.curve {
            Some(c) => format!("Hull-White on the GSW curve of {}, 10-year zero yield {}", c.date, pct(c.zero10)),
            None => format!("fixed {}", pct(DEFAULT_RF)),
        },
        "data/raw/rates/gsw_feds200628.csv (Federal Reserve Board, GSW)",
        &format!("last row on or before {}; a = {}, sigma = {} (the IBM and ANT files); long_flat", market.as_of, rules.hw_a, rules.hw_sigma),
    );
    row(
        "equity weight",
        format!("{:.3}", weight),
        &if market.price.is_some() { format!("price {:.2} ({}) x shares, total_debt", market.price.unwrap_or(0.0), market.price_source) } else { "no price".to_string() },
        "equity / (equity + debt); 1.0 when no price is known",
    );
    row("cost of debt", format!("risk-free + {}", pct(rules.kd_spread)), "constant", "judgment (a spread of an investment grade issuer); it counts only through the equity weight");
    row("terminal growth", format!("{} (std {})", pct(rules.terminal_growth.0), pct(rules.terminal_growth.1)), "repo convention (IBM and ANT files)", "capped by the target of the growth process, the risk-free rate and the cost of capital less 2% (the engine)");
    row("horizon", format!("{} years", rules.years), "owner rule", "annual steps, Gordon terminal value");

    // ---------------- the model file
    let mut t = String::new();
    let tw = &mut t;
    let _ = writeln!(tw, "# Automatic model of {ticker}, made by `fa dcf auto` on {}. A screen, not a reviewed valuation.", market.as_of);
    let _ = writeln!(tw, "engine = \"paths\"");
    let _ = writeln!(tw, "ticker = {}", q(ticker));
    let _ = writeln!(
        tw,
        "notes = {}",
        q(&format!(
            "Automatic model of {ticker} ({}). A screen, not a reviewed valuation. Made from the standard variables, the parameter table {DEFAULT_TABLE} and the yield curve; no hand input. Base: fiscal year {}.",
            inputs.name, last.end
        ))
    );
    let _ = writeln!(tw, "net_debt = {}", f(net_debt));
    let _ = writeln!(tw, "shares = {}", f(shares));
    let _ = writeln!(tw, "projection_years = {}\n", rules.years);
    let _ = writeln!(tw, "[[segments]]\nname = \"Total\"\nbase_revenue = {}\nprocess = \"growth\"\n", f(base_revenue));
    let rationale_g = format!(
        "Revenue growth. History {} values, mean {}; cell {size} (phi {:.3}, mean {}, tau {}, sigma {}); company mean {} (weight of the history {:.3}); year 1 mean {} std {}. Slow part: the company mean fades linearly to the long-run cap {} (10-year zero yield) with the half-life {slow_h:.1} years of the fade table (ramp {ramp_n} years). Fast part: phi {:.3}, sigma {}.",
        growth_hist.len(), pct(gfit.mean_history), gp.phi, pct(gp.mean), pct(gp.mean_sd_units), pct(gp.sigma), pct(gfit.mu), gfit.w, pct(g_start), pct(gfit.start_sd), pct(g_long), gfit.phi, pct(gfit.sigma)
    );
    let _ = writeln!(tw, "[process.growth]\nrole = \"growth\"");
    let _ = writeln!(tw, "start = {}", draw(g_start, gfit.start_sd, g_lo, g_hi));
    let _ = writeln!(tw, "target = {}", draw(gfit.mu.clamp(g_lo + 0.01, g_hi - 0.01), gfit.var_post.sqrt(), g_lo, g_hi));
    let _ = writeln!(tw, "half_life = {}", f(g_half));
    let _ = writeln!(tw, "sigma = {}", f(sg_own));
    if kappa > 0.0 {
        let _ = writeln!(tw, "sigma_level = {}", f(kappa));
    }
    let _ = writeln!(tw, "lo = {}\nhi = {}", f(g_lo), f(g_hi));
    if drift_end.abs() > 1e-12 {
        let _ = writeln!(tw, "drift = {{ start = 0.0, end = {}, ramp_years = {} }}", f(drift_end), ramp_n);
    }
    let _ = writeln!(tw, "rationale = {}\n", q(&rationale_g));
    let rationale_m = format!(
        "Free cash flow margin before SBC and before interest: (OCF - capex_total + (1 - t) interest) / revenue. History {} values, mean {} (as cash flow after interest); cell phi {:.3}, mean {}, tau {}, sigma {}; weight of the history {:.3}; interest ratio {}; target {}, start {}.",
        mhist.len(), pct(mfit.mean_history), mp.phi, pct(mp.mean), pct(mp.mean_sd_units), pct(mp.sigma), mfit.w, pct(interest_ratio), pct(m_target), pct(m_start)
    );
    let _ = writeln!(tw, "[process.margin]\nrole = \"margin\"");
    let _ = writeln!(tw, "start = {}", draw(m_start, mfit.start_sd, -MARGIN_LIMIT, MARGIN_LIMIT));
    let _ = writeln!(tw, "target = {}", draw(m_target, mfit.var_post.sqrt(), -MARGIN_LIMIT, MARGIN_LIMIT));
    let _ = writeln!(tw, "half_life = {}", f(m_half));
    let _ = writeln!(tw, "sigma = {}", f(mfit.sigma));
    let _ = writeln!(tw, "lo = {}\nhi = {}", f(-MARGIN_LIMIT), f(MARGIN_LIMIT));
    let _ = writeln!(tw, "rationale = {}\n", q(&rationale_m));
    let _ = writeln!(tw, "[process.sbc]\nrole = \"sbc\"\nstart = {}\ntarget = {}\nshape = \"ramp\"\nramp_years = 2", f(sbc_ratio), f(sbc_ratio));
    let _ = writeln!(tw, "rationale = {}\n", q(&format!("SBC ratio: mean of the last {} years, fixed. SBC is a cost of the model (repo convention).", sbc_vals.len())));
    let _ = writeln!(tw, "[discount]\nbeta = {}\nerp = {{ mean = {}, std = {}, lo = 0.02 }}\nkd_spread = {}\nequity_weight = {}\n", f(beta), f(rules.erp.0), f(rules.erp.1), f(rules.kd_spread), f(weight));
    let _ = writeln!(tw, "[tax]\nrate = {}\n", f(tax));
    match &market.curve {
        Some(c) => {
            let _ = writeln!(
                tw,
                "[rates]\nmodel = \"hullwhite\"\ntenor = 10.0\nmode = \"long_flat\"\na = {}\nsigma = {}\ncurve = {{ date = {}, beta0 = {}, beta1 = {}, beta2 = {}, beta3 = {}, tau1 = {}, tau2 = {} }}\n",
                f(rules.hw_a), f(rules.hw_sigma), q(&c.date), f(c.beta0), f(c.beta1), f(c.beta2), f(c.beta3), f(c.tau1), f(c.tau2)
            );
        }
        None => {
            let _ = writeln!(tw, "[rates]\nmodel = \"fixed\"\nrf = {}\n", f(DEFAULT_RF));
        }
    }
    let _ = writeln!(tw, "[terminal]\ngrowth = {{ mean = {}, std = {} }}\ncap_by_process = \"growth\"\nrf_margin = 0.0\n", f(rules.terminal_growth.0), f(rules.terminal_growth.1));
    let _ = writeln!(tw, "[reinvestment]\nmode = \"margin\"");
    for (name, fit) in [("growth", &gfit), ("margin", &mfit)] {
        if fit.var_post.sqrt() > 1e-9 && fit.start_sd > 1e-9 {
            let _ = writeln!(
                tw,
                "\n[[corr]]\na = \"process.{name}.start\"\nb = \"process.{name}.target\"\nrho = {}\nnote = \"AR(1): the year-1 value and the company mean share the uncertainty of the company mean\"",
                f(fit.corr.clamp(-0.99, 0.99))
            );
        }
    }

    let summary = Summary {
        base_revenue,
        base_end: last.end.clone(),
        growth_start: g_start,
        growth_company_mean: gfit.mu,
        growth_long_run: g_long,
        margin_start: m_start,
        margin_target: m_target,
        sbc_ratio,
        net_debt,
        shares,
        beta,
        equity_weight: weight,
        tax,
        size,
        growth_obs: growth_hist.len(),
        margin_obs: mhist.len(),
    };
    // The findings that touch the model inputs
    if opts.valuation && inputs.status == "WARN" {
        let keys = ["revenue", "operating_cash_flow", "capex", "sbc", "interest", "debt", "cash", "shares", "income_tax", "income_before_tax", "stale", "fiscal_year", "fcf", "filled_from_filing", "share_count"];
        for x in inputs.findings.iter().filter(|x| x.severity == "WARN") {
            let hay = format!("{} {}", x.code, x.detail);
            if keys.iter().any(|k| hay.contains(k)) {
                warnings.push(format!("mapping WARN {}: {}", x.code, x.detail));
            }
        }
    }
    Ok(AutoModel { toml: t, rows, warnings, summary })
}

#[cfg(test)]
#[path = "auto_tests.rs"]
mod tests;
