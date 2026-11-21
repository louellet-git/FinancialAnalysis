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
//!   company (interest paid less interest income, after tax) is added back.
//! - SBC: a ratio, the mean of the last 3 years, a cost of the model (the repo convention).
//! - Net debt: `total_debt` less `cash_and_short_term_investments` and `long_term_investments`; the after-tax interest
//!   income is out of the margin (stock-and-flow rule of the IBM review F15).
//! - Shares: the count at the end of the latest period plus the dilution of the period.
//! - Discount: from the model date; the part of year 1 before the balance sheet date is out (it is in the bridge).
//! - The other rules of `Rules` are alternatives for measurement and for the owner (`--rules K=V`).

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
/// The long-run nominal growth of the economy: the longer-run median of the real GDP growth (2.0%) plus the PCE
/// inflation (2.0%) of the FOMC Summary of Economic Projections of 2026-09-16 (record A-001).
pub const LONG_RUN_GROWTH: f64 = 0.040;
/// The risk-free rate when the yield curve is not available (stated default, with a warning).
pub const DEFAULT_RF: f64 = 0.042;

/// The cap of the past spend ratio in the persistence fit of the charge (`Rules::acq_persist`): a mean spend above a quarter
/// of the revenue is a change of the company, not a spend that recurs.
pub const ACQ_PERSIST_CAP: f64 = 0.25;
/// The default of `Rules::acq_persist`: `(a, b)` of the charge base `a + b x min(past, cap)`. The fit through the origin of
/// half 0 (`fa dcf pit --half 0 --diag acq`, record A-002): a company with no past purchases has no bought growth in its
/// history, so the charge that pays for credited growth is 0 for it (decision of 2026-11-21, review cycle 1).
pub const ACQ_PERSIST_DEFAULT: (f64, f64) = (0.0, 0.5226);

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
    // ---- Alternatives for measurement and for the owner's decision (`--rules K=V`). The defaults keep the model of
    // 2026-11-19 except where a correction of the investigation of 2026-11-20 changed it (see `docs/ARCHITECTURE.md`).
    /// A fixed risk-free rate for the discount (the curve still gives the cap of the growth). `None`: the curve.
    pub rf_fixed: Option<f64>,
    /// The equity weight is 1 (the cost of equity for the whole company).
    pub equity_weight_one: bool,
    /// The mean terminal growth is the 10-year zero yield (the long-run growth of the implied ERP of Damodaran).
    pub terminal_rf: bool,
    /// The level that the company growth fades to. `None`: the 10-year zero yield.
    pub long_cap: Option<f64>,
    /// One long-run growth for the fade level and the mean terminal growth: the lower of the 10-year zero yield and this
    /// number (the stable growth is at most the risk-free rate and at most the nominal growth of the economy). `long_cap`
    /// and `terminal_rf` win over it. Default `Some(LONG_RUN_GROWTH)` (decision of 2026-11-20, record A-001).
    /// `None` (`--rules long_run=off`): the old model, the fade to the yield and the terminal mean `terminal_growth`.
    pub long_run: Option<f64>,
    /// The length of the ramp of the fade in years. `None`: from the half-life of the fade table.
    pub fade_years: Option<usize>,
    /// The weight of the history in the company mean of the growth (`None`: the shrink rule).
    pub growth_w: Option<f64>,
    /// The persistence of the growth shocks (`None`: the table).
    pub growth_phi: Option<f64>,
    /// The weight of the history in the company mean of the margin (`None`: the shrink rule).
    pub margin_w: Option<f64>,
    /// The target margin is the margin of the base year (a company with a rising margin keeps it).
    pub margin_last: bool,
    /// SBC is a cost of the model (the repo convention). Off: SBC is not a cost.
    pub sbc_cost: bool,
    /// The bridge subtracts `long_term_investments` (marketable securities outside the current assets).
    pub bridge_lti: bool,
    /// The bridge adds `operating_lease_liabilities` as debt.
    pub bridge_leases: bool,
    /// The margin has no interest income: the after-tax `interest_income` is taken out of the free cash flow, because the
    /// bridge already counts the cash and the investments that earn it (IBM review F1a and F15).
    pub no_interest_income: bool,
    /// The share count is the count at the end of the latest period (`shares_outstanding`) plus the dilution of the
    /// period (`shares_diluted - shares_basic`), not the weighted average of the period.
    pub shares_latest: bool,
    /// The discount starts at the date of the model and the cash flow of year 1 before the balance sheet date is left
    /// out (it is already in the bridge).
    pub stub: bool,
    /// The cash flows come in the middle of each year.
    pub mid_year: bool,
    /// The cash paid for businesses (`acquisitions_net_of_cash` / revenue, mean of the margin history years) is a cost of
    /// the model: the growth history keeps the growth that the acquisitions bought (IBM review F2 and F16).
    pub acq_charge: bool,
    /// The paired rule of the conventions study: a share `f` (0 to 1) of the acquisition spend recurs. The model charges
    /// `f` times the mean ratio of `acquisitions_net_of_cash` to revenue, and the growth history keeps only `f` of the
    /// growth that the spend bought (`acq_mult` revenue for each dollar, half in the year of the spend and half in the next
    /// year): each growth value of the history is reduced by `(1 - f) acq_mult` times the mean of the spend ratios of that
    /// year and of the year before (spend / revenue of the year before the spend). `f = 1` is `acq_charge`; `f = 0` is
    /// organic growth only, with no charge. `None`: off.
    pub acq_f: Option<f64>,
    /// Revenue that one dollar of acquisition spend buys (used with `acq_f`).
    pub acq_mult: f64,
    /// The charge is the share `w` of the mean spend ratio, with `w` the weight of the history in the company mean of the
    /// growth (the shrink). The growth history stays as it is: the model credits `w` of the growth that the spend bought,
    /// so it charges `w` of the spend. Needs no `acq_mult`. Default on (decision of 2026-11-20, record A-002); `acq_w=0`
    /// gives the old model. `acq_charge` and `acq_f` win over it.
    pub acq_w: bool,
    /// The spend of the charge (and the real spend of the point-in-time test) has the second line of the purchases of
    /// businesses (`YearData::acq_spend`). Default on (record A-002, decision of 2026-11-21); `acq_second=0` gives the
    /// first line only.
    pub acq_second: bool,
    /// The base of the charge is the persistence fit of the past spend, not the past spend: `a + b x min(past, 25%)`, with
    /// `past` the mean spend ratio of the margin history (`--rules acq_persist_a=..,acq_persist_b=..`; the fit of the
    /// next 3 years on the past 5 years of the panel, `fa dcf pit --diag acq`). Default `(0.0, 0.5226)`, the fit through the origin of half 0 (record A-002, decision of
    /// 2026-11-21); `acq_persist=off` gives the past spend. It applies to the `acq_w` charge only: `acq_charge` and
    /// `acq_f` use the past spend, and so does the `chain` cut of the growth history.
    pub acq_persist: Option<(f64, f64)>,
    /// With `acq_w` and `acq_fade`: the charge does not fall to zero when the company mean `mu` is above the fade level `L`.
    /// It keeps the share `L / mu`, the share of the credited growth that stays (the spend buys growth in proportion to the
    /// mean), and the share `(mu - L) / mu` fades with the growth above `L`. A mean just above `L` keeps almost all of the
    /// charge, so the charge has no step at `mu = L`. Off: the charge falls to zero (the record A-002 rule of 2026-11-20).
    pub acq_keep: bool,
    // ---- Used by the implied search (`fa dcf implied`, `auto_implied.rs`) only. They are not `--rules` keys and no default
    // reads a market-implied result: a market-implied value is a measurement, not an input (`docs/QUALITY-STANDARDS.md`, 6).
    /// Added to the company mean of the growth and to the growth of year 1. The long-run level and the terminal rule are
    /// those of the builder for the shifted mean. 0: the model of the rules.
    pub growth_shift: f64,
    /// Added to the start and to the target of the free cash flow margin. 0: the model of the rules.
    pub margin_shift: f64,
    /// With `acq_w`: the charge fades with the growth that it pays for. When the company mean of the growth is above the
    /// fade level, the credited growth above the level falls linearly to zero along the drift ramp (`ramp_n` years); the
    /// charge falls from `w` x the mean spend ratio in year 1 to zero in the same years (the same timing as the drift,
    /// same ramp of the engine). Without such a drift the charge is constant. Default on (record A-002); `acq_fade=0` gives
    /// the constant charge. `acq_charge` and `acq_f` are never faded.
    pub acq_fade: bool,
    /// The growth of the base year, when it is below this floor, enters the company mean and the first step of the growth
    /// process at the floor (a year of decline is the start of a recovery, not a rate that stays: A-003). The history
    /// before the base year and the shock size keep the real values. `None`: off (`--rules last_floor=-1`).
    pub last_floor: Option<f64>,
    /// The weight (in years) of the table shock in the shock size of the margin. `None`: `own_sigma_nu`.
    pub margin_sigma_nu: Option<f64>,
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
            rf_fixed: None,
            equity_weight_one: false,
            terminal_rf: false,
            long_cap: None,
            long_run: Some(LONG_RUN_GROWTH),
            fade_years: None,
            growth_w: None,
            growth_phi: None,
            margin_w: None,
            margin_last: false,
            sbc_cost: true,
            bridge_lti: true,
            bridge_leases: false,
            no_interest_income: true,
            shares_latest: true,
            stub: true,
            mid_year: false,
            acq_charge: false,
            acq_f: None,
            acq_mult: 0.0,
            acq_w: true,
            acq_second: true,
            acq_persist: Some(ACQ_PERSIST_DEFAULT),
            acq_keep: false,
            acq_fade: true,
            last_floor: Some(0.0),
            margin_sigma_nu: None,
            growth_shift: 0.0,
            margin_shift: 0.0,
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
    /// `acquisitions_net_of_cash_second` (positive): the second line of the purchases of businesses (`YearData::acq_spend`).
    pub acquisitions_second: Option<f64>,
    pub total_debt: Option<f64>,
    /// `cash_and_short_term_investments`, else `cash_and_equivalents` plus `short_term_investments`.
    pub cash: Option<f64>,
    pub shares: Option<f64>,
    /// `interest_income` (positive).
    pub interest_income: Option<f64>,
    /// `long_term_investments` (balance).
    pub long_term_investments: Option<f64>,
    /// `operating_lease_liabilities` (balance).
    pub operating_leases: Option<f64>,
    /// `shares_outstanding` (the count at the end of the period) and `shares_basic` (weighted average).
    pub shares_outstanding: Option<f64>,
    pub shares_basic: Option<f64>,
    /// `dividends_paid` (positive).
    pub dividends: Option<f64>,
}

impl YearData {
    /// The cash paid for businesses in the year (positive; 0 when no line has a value). With `second`, the second line
    /// (`acquisitions_net_of_cash_second`) is added only when its value differs from the first line, or when the first
    /// line has no value: a filer that renames the tag between 10-Ks has the same value in both lines for one year, and
    /// that is one purchase (trap `2026-11-20-second-business-purchase-line`). Without `second`: the first line only.
    pub fn acq_spend(&self, second: bool) -> f64 {
        match (self.acquisitions, self.acquisitions_second) {
            (Some(a), Some(b)) if second && (a - b).abs() > 1e-9 * a.max(b).max(1.0) => a + b,
            (None, Some(b)) if second => b,
            (a, _) => a.unwrap_or(0.0),
        }
    }

    /// The three share counts are multiplied by `F(ticker, s)` (`share_basis::Basis`), so that they are on the share basis of
    /// today, the basis of the split-adjusted closes of the store. `s` is the FILING date of the fact that the model
    /// takes for the count (the `filed` of the tag fact of the variable that gives the value: `PeriodData::filed_of`), not the period end: a filing issued
    /// after a split shows the counts already adjusted for it (ASC 260), so only the splits after the filing count. When
    /// the value has no tag fact (a formula value), `s` is the filing date of the period (`PeriodData::filed`), and
    /// when that is missing, the period end. The cover-page share count (dei) is not used by the model. This is the one
    /// place of the correction.
    pub fn from_period(registry: &Registry, p: &PeriodData, basis: &super::share_basis::Basis) -> YearData {
        // the filing date of the variable that gives the value, else the filing of the period, else the period end
        let factor = |name: &str| -> f64 { basis.factor(p.filed_of(registry, name).or(p.filed.as_deref()).unwrap_or(&p.end)) };
        // `shares` is the diluted count, and the count at the end of the period when there is no diluted count
        let diluted_name = if p.get(registry, "shares_diluted").is_some() { "shares_diluted" } else { "shares_outstanding" };
        let (f_diluted, f_out, f_basic) = (factor(diluted_name), factor("shares_outstanding"), factor("shares_basic"));
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
            acquisitions_second: p.get(registry, "acquisitions_net_of_cash_second").map(f64::abs),
            total_debt: p.get(registry, "total_debt"),
            cash,
            shares: p.get(registry, "shares_diluted").or_else(|| p.get(registry, "shares_outstanding")).map(|v| v * f_diluted),
            interest_income: p.get(registry, "interest_income").map(f64::abs),
            long_term_investments: p.get(registry, "long_term_investments"),
            operating_leases: p.get(registry, "operating_lease_liabilities"),
            shares_outstanding: p.get(registry, "shares_outstanding").map(|v| v * f_out),
            shares_basic: p.get(registry, "shares_basic").map(|v| v * f_basic),
            dividends: p.get(registry, "dividends_paid").map(f64::abs),
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
    pub long_term_investments: Option<f64>,
    /// The count at the end of the period, and the weighted basic count of the period.
    pub shares_outstanding: Option<f64>,
    pub shares_basic: Option<f64>,
}

impl BalanceData {
    pub fn from_period(registry: &Registry, p: &PeriodData, form: &str, basis: &super::share_basis::Basis) -> BalanceData {
        let y = YearData::from_period(registry, p, basis);
        BalanceData {
            end: p.end.clone(),
            form: form.to_string(),
            total_debt: y.total_debt,
            cash: y.cash,
            shares: y.shares,
            pension: p.get(registry, "pension_liabilities_noncurrent"),
            minority: p.get(registry, "noncontrolling_interest"),
            operating_leases: y.operating_leases,
            long_term_investments: y.long_term_investments,
            shares_outstanding: y.shares_outstanding,
            shares_basic: y.shares_basic,
        }
    }

    /// The balance of a fiscal year of the history (no memo items).
    pub fn from_year(y: &YearData) -> BalanceData {
        BalanceData {
            end: y.end.clone(),
            form: "10-K".into(),
            total_debt: y.total_debt,
            cash: y.cash,
            shares: y.shares,
            operating_leases: y.operating_leases,
            long_term_investments: y.long_term_investments,
            shares_outstanding: y.shares_outstanding,
            shares_basic: y.shares_basic,
            ..Default::default()
        }
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
///
/// The share counts are on the share basis of the price file (`splits`, see `market/splits.rs` and `share_basis.rs`): each count gets the factor of
/// the splits after the filing date of its fact up to `to`, the last date of the price file of the ticker
/// (`share_basis::basis_to`; the closes are adjusted only up to that date) (`YearData::from_period`).
pub fn inputs_from_report(registry: &Registry, r: &mapcheck::run::TickerReport, splits: &[crate::market::splits::Split], to: &str) -> AutoInputs {
    let basis = super::share_basis::Basis { splits, ticker: &r.ticker, to };
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
        out.years = c.annual.iter().map(|p| YearData::from_period(registry, p, &basis)).collect();
        // The latest period with a balance sheet: a later 10-Q first, then the fiscal year.
        let last_year = c.annual.last();
        let newer_quarter = c.quarterly.last().filter(|q| last_year.is_none_or(|y| q.end > y.end));
        let usable = |b: &BalanceData| b.cash.is_some() && b.shares.is_some();
        let mut balance = None;
        if let Some(q) = newer_quarter {
            let b = BalanceData::from_period(registry, q, "10-Q", &basis);
            if usable(&b) {
                balance = Some(b);
            }
        }
        if balance.is_none() {
            balance = last_year.map(|y| BalanceData::from_period(registry, y, "10-K", &basis));
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
    curve_on(&ds, as_of)
}

/// The curve of the last FedInvest price file on or before `as_of` (`<cache>/prices/fedinvest_<date>.csv`): the NSS curve
/// that `fa recon rich-cheap` fits to the end-of-day prices of the Treasury notes and bonds (`richcheap::analyse`, settlement
/// one business day later). The same family as GSW, but a different fit: use one source at both ends of a comparison. The
/// parameters are put in the GSW units (BETA in percent, TAU in years); the zero yields are continuous, as GSW.
pub fn load_curve_fedinvest(cache: &std::path::Path, as_of: &str) -> Result<CurveNumbers, String> {
    let limit = crate::curve::parse_iso(as_of.get(..10).unwrap_or(as_of)).ok_or_else(|| format!("'{as_of}' is not a date"))?;
    let dir = cache.join("prices");
    let mut days: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|n| n.strip_prefix("fedinvest_")?.strip_suffix(".csv").map(str::to_string))
        .filter(|d| crate::curve::parse_iso(d).is_some_and(|x| x <= limit))
        .collect();
    days.sort();
    let date = days.pop().ok_or_else(|| format!("no FedInvest price file on or before {as_of} in {}", dir.display()))?;
    let quotes = crate::richcheap::cached_quotes(cache, &date)?;
    let day = crate::curve::parse_iso(&date).ok_or("bad date")?;
    let a = crate::richcheap::analyse(&date, crate::curve::add_business_days(day, 1), &quotes)?;
    Ok(curve_of_nss(&date, &a.nss))
}

/// The tenors of the Treasury par yield curve that enter the par fit (years). The bill tenors (1 to 6 months) are left out:
/// the fit is of coupon securities only, as `fa recon rich-cheap` (life of 3 months or more).
pub const PAR_TENORS: [(&str, i64); 8] = [("1 Yr", 1), ("2 Yr", 2), ("3 Yr", 3), ("5 Yr", 5), ("7 Yr", 7), ("10 Yr", 10), ("20 Yr", 20), ("30 Yr", 30)];

/// The curve of the last row on or before `as_of` of the Treasury daily par yield curve
/// (`<cache>/rates/treasury_nominal_<year>.csv`, the year of `as_of` and the year before). Each tenor is a par bond (a coupon
/// equal to the par yield, semiannual, clean price 100, maturity `n` years after the date); the NSS curve is fitted to these
/// bonds with the fit of `fa recon rich-cheap` (`fit::fit_curve`). Use one source at both ends of a comparison.
pub fn load_curve_par(cache: &std::path::Path, as_of: &str) -> Result<CurveNumbers, String> {
    let limit = crate::curve::parse_iso(as_of.get(..10).unwrap_or(as_of)).ok_or_else(|| format!("'{as_of}' is not a date"))?;
    let year: i64 = as_of.get(..4).and_then(|y| y.parse().ok()).ok_or("bad year")?;
    for y in [year, year - 1] {
        let path = cache.join("rates").join(format!("treasury_nominal_{y}.csv"));
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        if let Some((date, yields)) = par_row_on(&text, limit)? {
            return par_fit(&date, &yields);
        }
    }
    Err(format!("no Treasury par yield row on or before {as_of} in {}", cache.join("rates").display()))
}

/// The last row on or before `limit` of a Treasury par yield file: the ISO date and the yields (percent) of `PAR_TENORS`.
pub fn par_row_on(text: &str, limit: crate::curve::Day) -> Result<Option<(String, Vec<f64>)>, String> {
    let mut lines = text.lines();
    let head: Vec<String> = lines.next().ok_or("empty par yield file")?.split(',').map(|h| h.trim().trim_matches('"').to_string()).collect();
    let cols: Vec<usize> = PAR_TENORS.iter().map(|(n, _)| head.iter().position(|h| h == n).ok_or_else(|| format!("par yield file has no column '{n}'"))).collect::<Result<_, _>>()?;
    let mut best: Option<(crate::curve::Day, String, Vec<f64>)> = None;
    for line in lines {
        let f: Vec<&str> = line.split(',').map(|x| x.trim().trim_matches('"')).collect();
        let mdy: Vec<i64> = f[0].split('/').filter_map(|x| x.parse().ok()).collect();
        let [m, d, y] = mdy[..] else { continue };
        let day = crate::curve::days_from_civil(y, m, d);
        if day > limit || best.as_ref().is_some_and(|b| b.0 >= day) {
            continue;
        }
        let ys: Option<Vec<f64>> = cols.iter().map(|&c| f.get(c).and_then(|x| x.parse::<f64>().ok())).collect();
        if let Some(ys) = ys {
            best = Some((day, format!("{y:04}-{m:02}-{d:02}"), ys));
        }
    }
    Ok(best.map(|(_, d, y)| (d, y)))
}

/// The NSS fit to the par bonds of one row (yields in percent).
pub fn par_fit(date: &str, yields_pct: &[f64]) -> Result<CurveNumbers, String> {
    let settle = crate::curve::parse_iso(date).ok_or("bad date")?;
    let mut items = Vec::new();
    let mut points = Vec::new();
    for ((_, n), y) in PAR_TENORS.iter().zip(yields_pct) {
        let bond = crate::curve::Bond { coupon: y / 100.0, maturity: crate::curve::add_months(settle, 12 * n, false) };
        let (inst, yld, _) = crate::fit::make_inst(&bond, settle, 100.0).ok_or_else(|| format!("par bond of {n} years on {date}: no yield"))?;
        items.push(inst);
        points.push((*n as f64, yld));
    }
    let (nss, _) = crate::fit::fit_curve(&items, &points);
    Ok(curve_of_nss(date, &nss))
}

/// The curve numbers of a fitted NSS curve (decimal parameters) in the GSW units.
pub fn curve_of_nss(date: &str, n: &crate::fit::Nss) -> CurveNumbers {
    CurveNumbers { date: date.to_string(), beta0: n.b0 * 100.0, beta1: n.b1 * 100.0, beta2: n.b2 * 100.0, beta3: n.b3 * 100.0, tau1: n.tau1, tau2: n.tau2, zero10: n.zero(10.0) }
}

/// The curve numbers of the last row on or before `as_of` of a parsed GSW file.
pub fn curve_on(ds: &crate::forwards::Dataset, as_of: &str) -> Result<CurveNumbers, String> {
    let (date, day) = crate::forwards::parse_date_arg(as_of)?;
    let f = crate::forwards::forwards_on(ds, &date, day)?;
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
    // the bars after the date are cut first: the weekly bar of the week of the date takes the last close on or before the date
    let day = as_of.get(..10)?;
    let upto = |v: &[(String, f64)]| -> Vec<(String, f64)> { v.iter().filter(|(d, _)| d.get(..10).is_some_and(|d| d <= day)).cloned().collect() };
    let s: std::collections::HashMap<i64, f64> = weekly(&upto(stock)).into_iter().filter(|(w, _)| *w <= cut).collect();
    let m = weekly(&upto(market));
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

pub(crate) fn price_series(cache: &std::path::Path, ticker: &str) -> Option<Vec<(String, f64)>> {
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
    market_at(cache, ticker, as_of, price, None, rules, curve, index)
}

/// Longest gap in days between the price date and the last close that is used for it (a halted ticker has no price).
pub const PRICE_GAP_DAYS: i64 = 10;

/// The last close on or before `date` of a series `(date, close)`, if it is not more than `PRICE_GAP_DAYS` older than `date`.
pub fn close_on_or_before(series: &[(String, f64)], date: &str) -> Option<(String, f64)> {
    let day = mapcheck::dates::parse(date.get(..10)?)?;
    // the latest date at or before `date` (the series need not be sorted)
    let (d, c) = series.iter().filter(|(d, c)| d.get(..10).is_some_and(|d| d <= &date[..10]) && c.is_finite() && *c > 0.0).max_by(|a, b| a.0[..10].cmp(&b.0[..10]))?;
    let age = day - mapcheck::dates::parse(d.get(..10)?)?;
    (age <= PRICE_GAP_DAYS).then(|| (d.clone(), *c))
}

/// `market_from_cache` with a price date: `price_on = Some(D)` takes the last close on or before `D` (`close_on_or_before`)
/// when `price` is `None`. `price_on = None`: the last close of the cache. The beta, the curve date and the model date are
/// those of `as_of`.
#[allow(clippy::too_many_arguments)]
pub fn market_at(cache: &std::path::Path, ticker: &str, as_of: &str, price: Option<f64>, price_on: Option<&str>, rules: &Rules, curve: Option<&CurveNumbers>, index: Option<&[(String, f64)]>) -> Market {
    let stock = price_series(cache, ticker);
    let (price, source) = match (price, &stock, price_on) {
        (Some(p), _, _) => (Some(p), "--price".to_string()),
        (None, Some(s), Some(d)) => close_on_or_before(s, d).map(|(day, c)| (Some(c), format!("close of {day} (last on or before {d})"))).unwrap_or((None, String::new())),
        (None, Some(s), None) => s.last().map(|(d, c)| (Some(*c), format!("last close of the price cache ({d})"))).unwrap_or((None, String::new())),
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
    /// The acquisition charge (0 when the rule is off).
    pub acq_ratio: f64,
    pub net_debt: f64,
    pub shares: f64,
    pub beta: f64,
    pub equity_weight: f64,
    pub tax: f64,
    pub size: String,
    pub growth_obs: usize,
    pub margin_obs: usize,
    /// The risk-free rate of the discount at its mean (the 10-year zero yield, or the fixed rate), the equity risk
    /// premium and the spread of the cost of debt.
    pub rf: f64,
    pub erp: f64,
    pub kd_spread: f64,
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
    ar1_fit_w(hist, last, p, None)
}

/// `ar1_fit` with a fixed weight of the history (`Some(w)`, cut to [0, 1]; a history with no value keeps 0).
pub fn ar1_fit_w(hist: &[f64], last: Option<f64>, p: &Ar1Params, weight: Option<f64>) -> Ar1Fit {
    let n = hist.len();
    let phi = p.phi.clamp(-0.9, 0.99);
    let tau2 = p.mean_sd_units * p.mean_sd_units;
    let noise = if n == 0 { f64::INFINITY } else { p.sigma * p.sigma * mean_variance_factor(phi, n as f64) };
    let w = if n == 0 || tau2 + noise <= 0.0 { 0.0 } else { weight.map_or(tau2 / (tau2 + noise), |w| w.clamp(0.0, 1.0)) };
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

/// The timing of the discount of a model made on `as_of`: the years from `as_of` to the end of year 1 (the base fiscal
/// year end plus one year), cut to [0, 1], and the share of year 1 after the balance sheet date `balance_end`, cut to
/// [0, 1]. `None` if a date does not parse.
pub fn stub_and_share(base_end: &str, balance_end: &str, as_of: &str) -> Option<(f64, f64)> {
    let d = |x: &str| mapcheck::dates::parse(x.get(..10)?);
    let (e0, b, v) = (d(base_end)?, d(balance_end)?, d(as_of)?);
    let e1 = e0 + 365;
    let stub = ((e1 - v) as f64 / 365.0).clamp(0.0, 1.0);
    let share = ((e1 - b.max(e0)) as f64 / 365.0).clamp(0.0, 1.0);
    Some((stub, share))
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
pub(crate) fn margin_params(table: &ProcessTable, size: &str, template: &str, pool_se: f64) -> Option<(Ar1Params, String)> {
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
/// the growth of each (`None` for the first year and for a jump: more than 60% and revenue of $1B or more). `bought` is
/// the revenue that the growth history takes out for each dollar of acquisition spend (`bought * acquisitions / revenue
/// of the year before`); 0 leaves the growth as it is.
fn chain(years: &[YearData], organic: bool, bought: f64, second: bool) ->(Vec<YearData>, Vec<Option<f64>>, RuleCounts) {
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
            // the bought growth comes half in the year of the spend and half in the next year (panel regression of the
            // conventions study: both lags have the same coefficient)
            let cut = if i == 0 || bought == 0.0 {
                0.0
            } else {
                let spend = |k: usize, base: f64| data[k].acq_spend(second) / base;
                let a_now = spend(data.len() - 1, rows[start + i - 1].revenue);
                let a_before = if i >= 2 && data.len() >= 2 { spend(data.len() - 2, rows[start + i - 2].revenue) } else { a_now };
                bought * 0.5 * (a_now + a_before)
            };
            growth.push(if i == 0 { None } else { (if organic { r.growth_organic() } else { r.growth_clean() }).map(|g| g - cut).filter(|g| *g > GROWTH_LIMITS.0 && *g < GROWTH_LIMITS.1) });
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
    let acq_share = rules.acq_f.map_or(if rules.acq_charge { 1.0 } else { 0.0 }, |f| f.clamp(0.0, 1.0));
    let (hist, growth_all, _) = chain(&inputs.years, rules.history_organic, if rules.acq_f.is_some() { (1.0 - acq_share) * rules.acq_mult } else { 0.0 }, rules.acq_second);
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
    let floor = |g: f64| rules.last_floor.map_or(g, |f| g.max(f));
    let mut fit_hist = growth_hist.clone();
    if growth_all.last().is_some_and(|g| g.is_some()) {
        if let Some(l) = fit_hist.last_mut() {
            *l = floor(*l);
        }
    }
    let gfit = ar1_fit_w(&fit_hist, last_growth.map(floor), &Ar1Params { sigma: sg_eff, phi: rules.growth_phi.unwrap_or(gp.phi), ..gp.clone() }, rules.growth_w);
    // the long-run cap and the company mean
    let rf_long = market.curve.as_ref().map_or(DEFAULT_RF, |c| c.zero10);
    if market.curve.is_none() {
        warnings.push(format!("no GSW yield curve: the risk-free rate is the default {} (a fixed rate)", pct(DEFAULT_RF)));
    }
    let mu_company = if rules.growth_shift != 0.0 { gfit.mu + rules.growth_shift } else { gfit.mu };
    let mu = mu_company.clamp(-0.5, 1.5);
    // terminal_rf sets both the fade level and the terminal mean to the yield (it wins over long_run); long_cap sets the level
    let cap = rules.long_cap.unwrap_or_else(|| if rules.terminal_rf { rf_long } else { rules.long_run.map_or(rf_long, |x| x.min(rf_long)) });
    // the mean terminal growth: the same number L as the fade level (terminal_rf and long_run), else the rule value
    let tg_mean = if rules.terminal_rf {
        rf_long
    } else if let Some(x) = rules.long_run {
        x.min(rf_long)
    } else {
        rules.terminal_growth.0
    };
    let g_long = if rules.fade_to_cap { mu.min(cap) } else { mu };
    let slow_h = match slow_half_life(fade) {
        Some(h) => h.clamp(1.0, 10.0),
        None => {
            warnings.push(format!("the table has no fade cells: the half-life of the slow fade is the default {} years", rules.default_slow_half_life));
            rules.default_slow_half_life
        }
    };
    let ramp_n = rules.fade_years.map_or_else(|| fade_ramp_years(slow_h, rules.years), |n| n.clamp(2, 60));
    let drift_end = (g_long - mu) * 1.0;
    let g_start_raw = if rules.growth_shift != 0.0 { gfit.start_mean + rules.growth_shift } else { gfit.start_mean };
    let g_start = g_start_raw.clamp(GROWTH_LIMITS.0 + 0.01, GROWTH_LIMITS.1 - 0.01);
    let g_half = half_life_of(gfit.phi);
    let g_lo = GROWTH_LIMITS.0;
    let g_hi = GROWTH_LIMITS.1;

    // ---- margin (free cash flow before SBC and before interest)
    let mhist_all: Vec<(f64, f64)> = hist
        .iter()
        .filter_map(|y| {
            let m = y.fcf_margin().filter(|m| m.abs() <= MARGIN_LIMIT)?;
            let rev = y.revenue?;
            let income = if rules.no_interest_income { y.interest_income.unwrap_or(0.0) } else { 0.0 };
            Some((m, (1.0 - tax) * (y.interest.unwrap_or(0.0) - income) / rev))
        })
        .collect();
    let mhist: Vec<f64> = mhist_all.iter().rev().take(HIST_MARGIN).rev().map(|x| x.0).collect();
    let interest_ratio = {
        let v: Vec<f64> = mhist_all.iter().rev().take(HIST_RATIO).map(|x| x.1).collect();
        if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
    };
    let last_margin = last.fcf_margin().filter(|m| m.abs() <= MARGIN_LIMIT);
    let sm_own = own_sigma(&mhist, &mp, rules.margin_sigma_nu.unwrap_or(rules.own_sigma_nu));
    let mfit = ar1_fit_w(&mhist, last_margin, &Ar1Params { sigma: sm_own, ..mp.clone() }, rules.margin_w);
    let m_shift = rules.margin_shift;
    let m_start = (mfit.start_mean + interest_ratio + m_shift).clamp(-MARGIN_LIMIT + 0.01, MARGIN_LIMIT - 0.01);
    let m_mu = match (rules.margin_last, last_margin) {
        (true, Some(l)) => l,
        _ => mfit.mu,
    };
    let m_target = (m_mu + interest_ratio + m_shift).clamp(-MARGIN_LIMIT + 0.01, MARGIN_LIMIT - 0.01);
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
    let sbc_ratio = if !rules.sbc_cost {
        0.0
    } else if sbc_vals.is_empty() {
        warnings.push("no value of sbc_total in the last 3 years: the SBC ratio is 0".into());
        0.0
    } else {
        sbc_vals.iter().sum::<f64>() / sbc_vals.len() as f64
    };

    // ---- acquisitions: the mean ratio of the years of the margin history (a year with no value counts as 0)
    let acq_vals: Vec<f64> = hist.iter().rev().take(HIST_MARGIN).filter_map(|y| Some(y.acq_spend(rules.acq_second) / y.revenue.filter(|r| *r > 0.0)?)).collect();
    // acq_charge and acq_f are explicit and win over the default acq_w
    let acq_paired = rules.acq_w && !rules.acq_charge && rules.acq_f.is_none();
    let acq_share = if acq_paired { gfit.w } else { acq_share };
    // the paired charge fades with the credited growth above the fade level (the drift ramp), else it is constant
    let acq_fades = acq_paired && rules.acq_fade && drift_end < -1e-12;
    let acq_past = if acq_vals.is_empty() { 0.0 } else { acq_vals.iter().sum::<f64>() / acq_vals.len() as f64 };
    // the persistence fit applies to the paired charge only; acq_charge and acq_f use the past spend
    let acq_base = match rules.acq_persist {
        Some((a, b)) if acq_paired => (a + b * acq_past.min(ACQ_PERSIST_CAP)).max(0.0),
        _ => acq_past,
    };
    let acq_ratio = if acq_share > 0.0 && !acq_vals.is_empty() { acq_share * acq_base } else { 0.0 };

    // ---- bridge and shares
    let debt = balance.total_debt.unwrap_or_else(|| {
        if opts.valuation {
            warnings.push("total_debt has no value: it is taken as 0".into());
        }
        0.0
    });
    let lti = if rules.bridge_lti { balance.long_term_investments.unwrap_or(0.0).max(0.0) } else { 0.0 };
    let leases = if rules.bridge_leases { balance.operating_leases.unwrap_or(0.0).max(0.0) } else { 0.0 };
    let cash = balance.cash.unwrap_or(0.0) + lti;
    let net_debt = debt + leases - cash;
    let weighted = balance.shares.unwrap_or(1.0);
    let shares = match (rules.shares_latest && opts.valuation, balance.shares_outstanding) {
        (true, Some(o)) if o > 0.0 => {
            let latest = o + (weighted - balance.shares_basic.unwrap_or(weighted)).max(0.0);
            if (0.85..=1.10).contains(&(latest / weighted)) {
                latest
            } else {
                warnings.push(format!("shares_outstanding {:.1}M at the end of the period is not within 85% to 110% of the weighted diluted count {:.1}M (one class of several, or a unit error): the weighted count is used", o / 1e6, weighted / 1e6));
                weighted
            }
        }
        _ => weighted,
    };
    // the timing of the discount: from the date of the model to the end of year 1; the part of year 1 before the
    // balance sheet date is in the bridge
    let timing = if rules.stub && opts.valuation { stub_and_share(&last.end, &balance.end, &market.as_of) } else { None };

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
        _ if rules.equity_weight_one => 1.0,
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
        format!("{} (cap {}: 10-year zero yield {}); company mean {}", pct(g_long), pct(cap), pct(rf_long), pct(mu)),
        if market.curve.is_some() { "GSW curve of the cache" } else { "default rate" },
        &format!("{}the company mean fades linearly to the cap by the horizon: ramp of {ramp_n} years, equal in year {} to an exponential fade with half-life {slow_h:.1} years (fade table, k = 3 and 10)", if rules.long_cap.is_some() { "level set by long_cap. " } else if rules.terminal_rf { "terminal_rf: the level is the 10-year zero yield, the same as the mean terminal growth. " } else if rules.long_run.is_some() { "one long-run growth L = the lower of the 10-year zero yield and the growth of the economy (record A-001), the same L as the mean terminal growth; a company mean below L keeps its own mean. " } else { "long_run is off: the level is the 10-year zero yield. " }, rules.years),
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
        &format!("FCFF margin before SBC = (OCF - capex + (1 - t) (interest paid - interest income)) / revenue, t = {}; the interest income is out when no_interest_income is on (the cash that earns it is in the bridge). The history is shrunk as cash flow after interest (the table definition); the mean interest ratio of the last 3 years is added back", pct(tax)),
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
    if acq_share > 0.0 {
        row("acquisition charge", pct(acq_ratio), "acquisitions_net_of_cash / revenue, the years of the margin history", &format!("mean ratio times the share f (default, acq_w: f = w, the weight of the history in the growth mean, because the growth history keeps the bought growth; f = 1 with acq_charge; acq_f gives f); {}; a cost of the model (record A-002; IBM review F2 and F16)", if acq_fades { if rules.acq_keep { format!("year 1 as shown, then it falls linearly in {ramp_n} years to the share L / mean of it, like the credited growth above the long-run level L (acq_fade, acq_keep)") } else { format!("year 1 as shown, then it falls linearly to zero in {ramp_n} years like the credited growth above the long-run level (acq_fade)") } } else { "constant".to_string() }));
    }
    row("tax rate", pct(tax), "income_tax_expense, income_before_tax", &tax_rule);
    row(
        "net debt",
        format!("{} = debt {} + leases {} - cash and investments {} (of which long-term investments {})", money(net_debt), money(debt), money(leases), money(cash), money(lti)),
        &format!("total_debt (+ operating_lease_liabilities if bridge_leases) less cash_and_short_term_investments (cash_and_equivalents + short_term_investments) and long_term_investments (if bridge_lti), balance sheet {} ({})", balance.end, balance.form),
        "stock-and-flow rule (IBM review F15): a stock is in the bridge only if its flow is not in the margin. Left out: pension and retiree liabilities, operating leases (their cost is in the operating cash flow), minority interests, litigation reserves, and equity-method stakes (their dividends are in the operating cash flow)",
    );
    let memo: Vec<String> = [("pension_liabilities_noncurrent", balance.pension), ("noncontrolling_interest", balance.minority), ("operating_lease_liabilities", balance.operating_leases)]
        .iter()
        .filter_map(|(n, v)| v.filter(|v| v.abs() > 0.0).map(|v| format!("{n} {} ({:+.2} a share)", money(v), -v / shares)))
        .collect();
    if !memo.is_empty() && opts.valuation {
        row("memo: not in the bridge", memo.join("; "), "balance sheet variables", "the value per share would change by the amount shown if the item were a claim in the bridge");
    }
    let shares_source = if (shares - weighted).abs() > 0.5 {
        format!("shares_outstanding at the end of the period plus the dilution (shares_diluted - shares_basic), period {} ({})", balance.end, balance.form)
    } else {
        format!("shares_diluted (weighted average), period {} ({})", balance.end, balance.form)
    };
    row("shares", format!("{:.1}M (weighted diluted {:.1}M)", shares / 1e6, weighted / 1e6), &shares_source, "the count on the date of the balance sheet; the weighted count when the end count is not known or not plausible");
    if let Some((stub, share)) = timing {
        row(
            "timing of the discount",
            format!("year 1 ends {:.2} years after the model date; {:.0}% of year 1 is after the balance sheet", stub, share * 100.0),
            &format!("model date {}, base fiscal year {}, balance sheet {}", market.as_of, last.end, balance.end),
            "the discount starts on the model date; the part of year 1 before the balance sheet date is in the cash and the debt of the bridge, so it is left out of the flows",
        );
    }
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
    let (tg_source, tg_rule) = if rules.terminal_rf {
        ("rule terminal_rf: the 10-year zero yield", "mean = the 10-year zero yield; capped by the target of the growth process, the risk-free rate and the cost of capital less 2% (the engine)")
    } else if let Some(x) = rules.long_run {
        ("one long-run growth L = the lower of the 10-year zero yield and the growth of the economy (FOMC SEP, record A-001)", if x.min(rf_long) < x { "L is the 10-year zero yield now (below the growth of the economy); the mean is L; capped by the target of the growth process, the risk-free rate and the cost of capital less 2% (the engine)" } else { "L is the growth of the economy now (below the 10-year zero yield); the mean is L; capped by the target of the growth process, the risk-free rate and the cost of capital less 2% (the engine)" })
    } else {
        ("repo convention (IBM and ANT files); long_run is off", "capped by the target of the growth process, the risk-free rate and the cost of capital less 2% (the engine)")
    };
    row("terminal growth", format!("{} (std {})", pct(tg_mean), pct(rules.terminal_growth.1)), tg_source, tg_rule);
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
    let floored = last_growth.zip(rules.last_floor).is_some_and(|(g, f)| g < f);
    let floor_note = if floored { format!(" The growth of the base year ({}) is below {}: it enters the company mean and the first step as {} (A-003).", pct(last_growth.unwrap_or(0.0)), pct(rules.last_floor.unwrap_or(0.0)), pct(rules.last_floor.unwrap_or(0.0))) } else { String::new() };
    let rationale_g = format!(
        "Revenue growth. History {} values, mean {}; cell {size} (phi {:.3}, mean {}, tau {}, sigma {}); company mean {} (weight of the history {:.3}); year 1 mean {} std {}. Slow part: the company mean fades linearly to the long-run cap {} (10-year zero yield) with the half-life {slow_h:.1} years of the fade table (ramp {ramp_n} years). Fast part: phi {:.3}, sigma {}.{floor_note}",
        growth_hist.len(), pct(gfit.mean_history), gp.phi, pct(gp.mean), pct(gp.mean_sd_units), pct(gp.sigma), pct(gfit.mu), gfit.w, pct(g_start), pct(gfit.start_sd), pct(g_long), gfit.phi, pct(gfit.sigma)
    );
    let _ = writeln!(tw, "[process.growth]\nrole = \"growth\"");
    let _ = writeln!(tw, "start = {}", draw(g_start, gfit.start_sd, g_lo, g_hi));
    let _ = writeln!(tw, "target = {}", draw(mu_company.clamp(g_lo + 0.01, g_hi - 0.01), gfit.var_post.sqrt(), g_lo, g_hi));
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
    if acq_ratio > 0.0 {
        if acq_fades {
            let kept = if rules.acq_keep && mu > 0.0 { g_long / mu } else { 0.0 };
            let _ = writeln!(tw, "[process.charge]\nrole = \"charge\"\nstart = {}\ntarget = {}\nshape = \"ramp\"\nramp_years = {}", f(acq_ratio), f(acq_ratio * kept), ramp_n);
        } else {
            let _ = writeln!(tw, "[process.charge]\nrole = \"charge\"\nstart = {}\ntarget = {}\nshape = \"ramp\"\nramp_years = 2", f(acq_ratio), f(acq_ratio));
        }
        let fade_text = if acq_fades { if rules.acq_keep { format!(" The charge falls linearly in {ramp_n} years to the share L / mean of its year 1 value (the credited growth that stays), as the credited growth above the long-run level does (the drift of the growth).") } else { format!(" The charge falls linearly to zero in {ramp_n} years, as the credited growth above the long-run level does (the drift of the growth).") } } else { " The charge is constant.".to_string() };
        let _ = writeln!(tw, "rationale = {}\n", q(&format!("Cash paid for businesses: mean of acquisitions_net_of_cash / revenue of the last {} years, times the share f.{} The growth history keeps the bought growth, so the cost is a cost of the model (IBM review F2, F16; record A-002).", acq_vals.len(), fade_text)));
    }
    // the lower bound of the draw is 2%; a mean below it (the implied search) moves the bound to the mean
    let erp_lo = rules.erp.0.min(0.02);
    let _ = write!(tw, "[discount]\nbeta = {}\nerp = {{ mean = {}, std = {}, lo = {} }}\nkd_spread = {}\nequity_weight = {}\n", f(beta), f(rules.erp.0), f(rules.erp.1), f(erp_lo), f(rules.kd_spread), f(weight));
    if let Some((stub, share)) = timing {
        let _ = write!(tw, "stub_years = {}\nyear1_share = {}\n", f(stub), f(share));
    }
    if rules.mid_year {
        let _ = writeln!(tw, "mid_year = true");
    }
    let _ = writeln!(tw);
    let _ = writeln!(tw, "[tax]\nrate = {}\n", f(tax));
    match (&market.curve, rules.rf_fixed) {
        (_, Some(rf)) => {
            let _ = writeln!(tw, "[rates]\nmodel = \"fixed\"\nrf = {}\n", f(rf));
        }
        (Some(c), None) => {
            let _ = writeln!(
                tw,
                "[rates]\nmodel = \"hullwhite\"\ntenor = 10.0\nmode = \"long_flat\"\na = {}\nsigma = {}\ncurve = {{ date = {}, beta0 = {}, beta1 = {}, beta2 = {}, beta3 = {}, tau1 = {}, tau2 = {} }}\n",
                f(rules.hw_a), f(rules.hw_sigma), q(&c.date), f(c.beta0), f(c.beta1), f(c.beta2), f(c.beta3), f(c.tau1), f(c.tau2)
            );
        }
        (None, None) => {
            let _ = writeln!(tw, "[rates]\nmodel = \"fixed\"\nrf = {}\n", f(DEFAULT_RF));
        }
    }
    let _ = writeln!(tw, "[terminal]\ngrowth = {{ mean = {}, std = {} }}\ncap_by_process = \"growth\"\nrf_margin = 0.0\n", f(tg_mean), f(rules.terminal_growth.1));
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
        growth_company_mean: mu_company,
        growth_long_run: g_long,
        margin_start: m_start,
        margin_target: m_target,
        sbc_ratio,
        acq_ratio,
        net_debt,
        shares,
        beta,
        equity_weight: weight,
        tax,
        size,
        growth_obs: growth_hist.len(),
        margin_obs: mhist.len(),
        rf: rules.rf_fixed.unwrap_or(rf_long),
        erp: rules.erp.0,
        kd_spread: rules.kd_spread,
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
pub(crate) mod tests;
