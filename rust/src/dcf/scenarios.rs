//! Bear/base/bull presets for the classic engine and the capital structure
//! (`financial_analysis/scenarios.py`).

use super::classic::{Company, DcfAssumptions, Period};
use super::stats::err;
use super::DcfError;

pub const SCENARIO_SEED: u64 = 7;

#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub key: &'static str,
    pub label: &'static str,
    pub assumptions: DcfAssumptions,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapitalStructure {
    pub net_debt: f64,
    pub shares: f64,
}

fn preset(growth: f64, wacc: f64, terminal: f64) -> DcfAssumptions {
    DcfAssumptions {
        growth_rate_mean: growth,
        growth_rate_std: 0.02,
        wacc_mean: wacc,
        wacc_std: 0.01,
        terminal_growth_mean: terminal,
        terminal_growth_std: 0.005,
        seed: Some(SCENARIO_SEED),
        ..DcfAssumptions::default()
    }
}

/// v3 framework (analysis/IBM/REVIEW.md): FCFF after SBC with paired M&A charge.
fn ibm_v3(growth: f64, wacc: f64, terminal: f64, margin: f64) -> DcfAssumptions {
    DcfAssumptions {
        growth_rate_mean: growth,
        growth_rate_std: 0.02,
        wacc_mean: wacc,
        wacc_std: 0.0075,
        terminal_growth_mean: terminal,
        terminal_growth_std: 0.005,
        fcf_margin_mean: Some(margin),
        fcf_margin_std: Some(0.02),
        seed: Some(SCENARIO_SEED),
        ..DcfAssumptions::default()
    }
}

/// First-pass consensus-anchored presets for DUOL, not a vetted driver build.
fn duol(growth: f64, wacc: f64, terminal: f64, margin: f64, growth_std: f64) -> DcfAssumptions {
    DcfAssumptions {
        growth_rate_mean: growth,
        growth_rate_std: growth_std,
        wacc_mean: wacc,
        wacc_std: 0.0125,
        terminal_growth_mean: terminal,
        terminal_growth_std: 0.005,
        fcf_margin_mean: Some(margin),
        fcf_margin_std: Some(0.04),
        seed: Some(SCENARIO_SEED),
        ..DcfAssumptions::default()
    }
}

pub fn default_scenarios() -> Vec<Scenario> {
    vec![
        Scenario { key: "bear", label: "Bear  (growth stalls)", assumptions: preset(0.01, 0.095, 0.015) },
        Scenario { key: "base", label: "Base  (mature grower, historical trend)", assumptions: preset(0.04, 0.085, 0.025) },
        Scenario { key: "bull", label: "Bull  (growth re-accelerates)", assumptions: preset(0.06, 0.08, 0.03) },
    ]
}

pub fn scenarios_for(ticker: &str) -> Vec<Scenario> {
    match ticker.to_uppercase().as_str() {
        "DUOL" => vec![
            Scenario { key: "bear", label: "Bear  (AI commoditizes app learning)", assumptions: duol(0.04, 0.115, 0.02, 0.24, 0.05) },
            Scenario { key: "base", label: "Base  (consensus deceleration glide)", assumptions: duol(0.14, 0.105, 0.03, 0.30, 0.05) },
            Scenario { key: "bull", label: "Bull  (AI-native re-acceleration)", assumptions: duol(0.22, 0.10, 0.035, 0.36, 0.06) },
        ],
        "IBM" => vec![
            Scenario { key: "bear", label: "Bear  (disruption: g 1%, 100% M&A charged)", assumptions: ibm_v3(0.01, 0.0825, 0.015, 0.092) },
            Scenario { key: "base", label: "Base  (organic + half-paid acquired growth)", assumptions: ibm_v3(0.0215, 0.075, 0.0225, 0.145) },
            Scenario { key: "bull", label: "Bull  (guidance-anchored recovery)", assumptions: ibm_v3(0.045, 0.0725, 0.025, 0.161) },
        ],
        _ => default_scenarios(),
    }
}

/// Reviewed capital structure of a ticker, if the registry has one.
pub fn capital_override(ticker: &str) -> Option<CapitalStructure> {
    match ticker.to_uppercase().as_str() {
        // v3 bridge (analysis/IBM/REVIEW.md F15)
        "IBM" => Some(CapitalStructure { net_debt: 54.0e9, shares: 948_675_228.0 }),
        // Debt-free; net cash $1.1746B; diluted shares.
        "DUOL" => Some(CapitalStructure { net_debt: -1.1746e9, shares: 48_308_000.0 }),
        _ => None,
    }
}

/// The latest annual period: the one with the largest `period_end` (first one on a tie).
pub fn latest_annual(company: &Company) -> Result<&Period, DcfError> {
    let mut best: Option<&Period> = None;
    for p in company.periods.iter().filter(|p| p.is_annual()) {
        if best.is_none_or(|b| p.period_end > b.period_end) {
            best = Some(p);
        }
    }
    best.ok_or_else(|| DcfError(format!("{}: no annual (10-K) periods available.", company.ticker)))
}

/// Net debt and share count for equity per share. A ticker override wins; otherwise
/// `total_debt - cash` of the latest annual period and its share count.
pub fn capital_structure_for(company: &Company) -> Result<CapitalStructure, DcfError> {
    if let Some(o) = capital_override(&company.ticker) {
        return Ok(o);
    }
    let latest = latest_annual(company)?;
    let mut total_debt = latest.total_debt;
    if total_debt.is_none() {
        if let Some(ltd) = latest.long_term_debt {
            total_debt = Some(ltd + latest.debt_current.unwrap_or(0.0));
        }
    }
    let (debt, cash) = match (total_debt, latest.cash_and_equivalents) {
        (Some(d), Some(c)) => (d, c),
        _ => {
            return err(format!(
                "{}: cannot derive net debt - latest annual period ({}) is missing total_debt or cash_and_equivalents. Add an override to scenarios.CAPITAL_OVERRIDES.",
                company.ticker, latest.period_end
            ))
        }
    };
    let shares = latest.shares_outstanding.filter(|s| *s != 0.0).or(latest.shares_diluted.filter(|s| *s != 0.0));
    let shares = match shares {
        Some(s) => s,
        None => {
            return err(format!(
                "{}: no shares_outstanding or shares_diluted in the latest annual period ({}). Add an override to scenarios.CAPITAL_OVERRIDES.",
                company.ticker, latest.period_end
            ))
        }
    };
    Ok(CapitalStructure { net_debt: debt - cash, shares })
}
