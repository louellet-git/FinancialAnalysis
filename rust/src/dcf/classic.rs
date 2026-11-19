//! Classic Monte Carlo DCF (the Rust form of `financial_analysis/dcf.py`): growth, FCF margin,
//! WACC and terminal growth are drawn directly, one normal draw for each path.

use super::nprandom::Pcg64;
use super::stats::{err, mean, percentile, sorted_copy, std_pop};
use super::{clock_seed, DcfError};
use serde::{Deserialize, Serialize};

pub const MIN_POSITIVE_FCF_PERIODS: usize = 4;
pub const MARGIN_LOOKBACK: usize = 5;

/// Fields of one period that the engines read. The JSON of `data/processed/<T>.json` has
/// many more fields; they are ignored.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Period {
    #[serde(default)]
    pub form: String,
    /// ISO date `YYYY-MM-DD` (text order is date order).
    #[serde(default)]
    pub period_end: String,
    #[serde(default)]
    pub revenue: Option<f64>,
    #[serde(default)]
    pub operating_cash_flow: Option<f64>,
    #[serde(default)]
    pub capex_total: Option<f64>,
    #[serde(default)]
    pub capex: Option<f64>,
    #[serde(default)]
    pub shares_outstanding: Option<f64>,
    #[serde(default)]
    pub shares_diluted: Option<f64>,
    #[serde(default)]
    pub total_debt: Option<f64>,
    #[serde(default)]
    pub long_term_debt: Option<f64>,
    #[serde(default)]
    pub debt_current: Option<f64>,
    #[serde(default)]
    pub cash_and_equivalents: Option<f64>,
}

impl Period {
    pub fn is_annual(&self) -> bool {
        self.form == "10-K" || self.form == "10-K/A"
    }

    /// Operating cash flow less capex (`capex_total`, else `capex`); an outflow of either sign.
    pub fn free_cash_flow(&self) -> Option<f64> {
        let capex = self.capex_total.or(self.capex);
        match (self.operating_cash_flow, capex) {
            (Some(ocf), Some(cx)) => Some(ocf - cx.abs()),
            _ => None,
        }
    }

    pub fn fcf_margin(&self) -> Option<f64> {
        let fcf = self.free_cash_flow()?;
        match self.revenue {
            Some(r) if r != 0.0 => Some(fcf / r),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Company {
    #[serde(default)]
    pub ticker: String,
    #[serde(default)]
    pub company_name: String,
    #[serde(default)]
    pub periods: Vec<Period>,
}

impl Company {
    pub fn annuals(&self) -> Vec<&Period> {
        self.periods.iter().filter(|p| p.is_annual()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DcfAssumptions {
    pub n_simulations: usize,
    pub projection_years: usize,
    pub growth_rate_mean: f64,
    pub growth_rate_std: f64,
    pub terminal_growth_mean: f64,
    pub terminal_growth_std: f64,
    pub wacc_mean: f64,
    pub wacc_std: f64,
    pub fcf_margin_mean: Option<f64>,
    pub fcf_margin_std: Option<f64>,
    pub seed: Option<u64>,
}

impl Default for DcfAssumptions {
    fn default() -> Self {
        DcfAssumptions {
            n_simulations: 10_000,
            projection_years: 10,
            growth_rate_mean: 0.15,
            growth_rate_std: 0.05,
            terminal_growth_mean: 0.03,
            terminal_growth_std: 0.005,
            wacc_mean: 0.10,
            wacc_std: 0.01,
            fcf_margin_mean: None,
            fcf_margin_std: None,
            seed: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DcfResult {
    pub median_value: f64,
    pub p10_value: f64,
    pub p90_value: f64,
    pub mean_value: f64,
    pub std_value: f64,
    pub median_per_share: Option<f64>,
    pub simulations: Vec<f64>,
    /// year-major, `n` columns
    pub fcfs: Vec<f64>,
    pub pv_fcfs: Vec<f64>,
    pub pv_terminal: Vec<f64>,
    pub n: usize,
    pub years: usize,
    pub seed: u64,
}

/// Historical FCF margin (mean, std) over the last annual periods.
pub fn derive_fcf_margin(company: &Company) -> Result<(f64, f64), DcfError> {
    let annuals = company.annuals();
    let start = annuals.len().saturating_sub(MARGIN_LOOKBACK);
    let margins: Vec<f64> = annuals[start..].iter().filter_map(|p| p.fcf_margin()).collect();
    if margins.is_empty() {
        return err(format!(
            "{}: FCF margin cannot be derived - no annual periods with both FCF and revenue data.",
            company.ticker
        ));
    }
    Ok((mean(&margins), std_pop(&margins).max(0.01)))
}

fn check_minimum_history(company: &Company, annuals: &[&Period]) -> Result<(), DcfError> {
    let positive = annuals.iter().filter(|p| p.free_cash_flow().is_some_and(|f| f > 0.0)).count();
    if positive < MIN_POSITIVE_FCF_PERIODS {
        return err(format!(
            "{}: only {positive} annual period(s) with positive FCF - at least {MIN_POSITIVE_FCF_PERIODS} are required to fit Monte Carlo input distributions. Short or loss-making histories (e.g. recently IPO'd companies) cannot be valued meaningfully with this model.",
            company.ticker
        ));
    }
    Ok(())
}

fn base_revenue(company: &Company, annuals: &[&Period]) -> Result<f64, DcfError> {
    for p in annuals.iter().rev() {
        if let Some(r) = p.revenue {
            return Ok(r);
        }
    }
    err(format!("{}: no annual revenue data available.", company.ticker))
}

fn shares_outstanding(annuals: &[&Period]) -> Option<f64> {
    for p in annuals.iter().rev() {
        if let Some(s) = p.shares_outstanding.filter(|s| *s != 0.0) {
            return Some(s);
        }
        if let Some(s) = p.shares_diluted.filter(|s| *s != 0.0) {
            return Some(s);
        }
    }
    None
}

/// Run the Monte Carlo DCF (`StochasticDCF.run`).
pub fn run_classic(company: &Company, a: &DcfAssumptions) -> Result<DcfResult, DcfError> {
    let n = a.n_simulations;
    if n == 0 {
        return err("n_simulations must be positive, got 0.");
    }
    if a.projection_years == 0 {
        return err("projection_years must be positive, got 0.");
    }
    for (name, s) in [
        ("growth_rate_std", a.growth_rate_std),
        ("terminal_growth_std", a.terminal_growth_std),
        ("wacc_std", a.wacc_std),
    ] {
        if s < 0.0 {
            return err(format!("{name} < 0"));
        }
    }
    let annuals = company.annuals();
    check_minimum_history(company, &annuals)?;
    let (m_mean, m_std) = match (a.fcf_margin_mean, a.fcf_margin_std) {
        (Some(m), Some(s)) => (m, s),
        _ => derive_fcf_margin(company)?,
    };
    if m_std < 0.0 {
        return err("fcf_margin_std < 0");
    }
    let base = base_revenue(company, &annuals)?;
    let seed = a.seed.unwrap_or_else(clock_seed);
    let mut rng = Pcg64::new(seed);

    let growth = rng.normal_vec(a.growth_rate_mean, a.growth_rate_std, n);
    let tgrowth = rng.normal_vec(a.terminal_growth_mean, a.terminal_growth_std, n);
    let wacc: Vec<f64> = rng.normal_vec(a.wacc_mean, a.wacc_std, n).into_iter().map(|w| w.max(0.01)).collect();
    let margins: Vec<f64> =
        rng.normal_vec(m_mean, m_std, n).into_iter().map(|m| m.clamp(0.0, 1.0)).collect();

    let years = a.projection_years;
    let mut fcfs = vec![0.0; years * n];
    let mut pv_fcfs = vec![0.0; years * n];
    let mut total = vec![0.0; n];
    let mut pv_terminal = vec![0.0; n];
    for j in 0..n {
        let mut acc = 0.0;
        for t in 0..years {
            let tt = (t + 1) as f64;
            let revenue = base * (1.0 + growth[j]).powf(tt);
            let fcf = revenue * margins[j];
            let discount = 1.0 / (1.0 + wacc[j]).powf(tt);
            let pv = fcf * discount;
            fcfs[t * n + j] = fcf;
            pv_fcfs[t * n + j] = pv;
            acc = if t == 0 { pv } else { acc + pv };
        }
        let terminal_fcf = fcfs[(years - 1) * n + j] * (1.0 + tgrowth[j]);
        let tv = terminal_fcf / (wacc[j] - tgrowth[j]).max(0.001);
        pv_terminal[j] = tv / (1.0 + wacc[j]).powf(years as f64);
        total[j] = acc + pv_terminal[j];
    }
    let sorted = sorted_copy(&total);
    let median = percentile(&sorted, 50.0);
    let shares = shares_outstanding(&annuals);
    Ok(DcfResult {
        median_value: median,
        p10_value: percentile(&sorted, 10.0),
        p90_value: percentile(&sorted, 90.0),
        mean_value: mean(&total),
        std_value: std_pop(&total),
        median_per_share: shares.map(|s| median / s),
        simulations: total,
        fcfs,
        pv_fcfs,
        pv_terminal,
        n,
        years,
        seed,
    })
}
