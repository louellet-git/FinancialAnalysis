//! Investor verdict over simulated per-share values (`financial_analysis/verdict.py`).
//!
//! The deciding number is the chance that the price is too high: the share of paths valued
//! below the price.

use super::stats::share_below;

pub fn chance_price_too_high(per_share_paths: &[f64], price: f64) -> f64 {
    share_below(per_share_paths, price)
}

/// One of five verdicts for the chance that the price is too high.
pub fn verdict_band(p_too_high: f64) -> &'static str {
    if p_too_high < 0.25 {
        "Strong opportunity"
    } else if p_too_high < 0.40 {
        "Opportunity"
    } else if p_too_high <= 0.60 {
        "No edge"
    } else if p_too_high <= 0.75 {
        "Overpriced"
    } else {
        "Strongly overpriced"
    }
}
