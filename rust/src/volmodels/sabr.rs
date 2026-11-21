//! SABR smile of one expiry: the implied volatility formula and a fit of alpha, rho, nu with beta fixed.
//!
//! Sources:
//! - Hagan, Kumar, Lesniewski, Woodward (2002), "Managing smile risk", Wilmott Magazine, July 2002: the time term
//!   `1 + [...] T` and the ATM form. The paper itself was NOT read for this code. Its equation numbers (2.17b, 2.18, A.65,
//!   A.69) are from memory, not checked. The time term is checked against eq. (3) of Obloj (2008), which repeats it as `I^1_H`.
//! - The fixed answers of the tests come from an independent implementation (Quality and Risk, 2026-11-21), not from a paper.
//! - Obloj (2008), "Fine-tune your smile: correction to Hagan et al.", arXiv:0708.0998v3 (read): the zero order term
//!   `I^0(x) = nu x / ln[(sqrt(1 - 2 rho z + z^2) + z - rho) / (1 - rho)]`, `x = ln(F/K)`,
//!   `z = (nu / alpha) (F^(1-beta) - K^(1-beta)) / (1 - beta)` (Table 1, row `I^0(x), beta < 1`, column "Berestycki et al.";
//!   it replaces the term of Hagan et al.). The note has no equation numbers for it, only Table 1. For beta = 1 it is the
//!   formula of Hagan et al. (2.17a with beta = 1). For nu = 0 it is `alpha (1 - beta) x / (F^(1-beta) - K^(1-beta))` (Table 1, nu = 0).
//!
//! The implied volatility is `I^0(x) * (1 + I^1 T)` with `I^1` from eq. (3) of Obloj (2008).
//! The loss is `(model vol - mid vol) / spread in vol` (floor `MIN_SPREAD`, as the Heston loss). A model vol is not a market price.

use super::base::{Series, MIN_SPREAD};
use super::calib::{at_bound, lm_core, LmStatus};
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sabr {
    pub alpha: f64,
    pub beta: f64,
    pub rho: f64,
    pub nu: f64,
}

impl Sabr {
    /// Black implied volatility (decimal) of strike `k` for forward `f`, `t` years.
    pub fn implied_vol(&self, f: f64, k: f64, t: f64) -> f64 {
        let Sabr { alpha, beta, rho, nu } = *self;
        let x = (f / k).ln();
        let b = 1.0 - beta;
        let fb = f.powf(b);
        // g = (F^(1-beta) - K^(1-beta)) / (1 - beta), written without a cancellation; it is x F^(1-beta) -> x as beta -> 1.
        let bx = b * x;
        let (g, x_over_g) = if bx.abs() < 1e-12 {
            (fb * x, f.powf(-b))
        } else {
            let g = fb * (-(-bx).exp_m1()) / b;
            (g, x / g)
        };
        // Obloj (2008), Table 1: I^0 = alpha * (x / g) * (z / X(z)), z = (nu / alpha) g, X(z) = ln[(sqrt(1 - 2 rho z + z^2) + z - rho) / (1 - rho)].
        let z = nu / alpha * g;
        let z_over_xz = if z.abs() < 1e-7 { 1.0 - 0.5 * rho * z } else { z / (((1.0 - 2.0 * rho * z + z * z).sqrt() + z - rho) / (1.0 - rho)).ln() };
        let i0 = alpha * x_over_g * z_over_xz;
        // Obloj (2008) eq. (3) (the time term of Hagan et al.; their equation number is from memory, not checked): (FK)^((1-beta)/2) = F^(1-beta) exp(-(1-beta) x / 2).
        let fk_half = fb * (-0.5 * bx).exp();
        let i1 = b * b / 24.0 * alpha * alpha / (fk_half * fk_half) + 0.25 * rho * nu * alpha * beta / fk_half + (2.0 - 3.0 * rho * rho) / 24.0 * nu * nu;
        i0 * (1.0 + i1 * t)
    }

    /// The ATM volatility of Hagan et al. (2002) (eq. 2.18 from memory, not checked; the same as eq. (3) of Obloj at K = F).
    pub fn atm_vol(&self, f: f64, t: f64) -> f64 {
        let Sabr { alpha, beta, rho, nu } = *self;
        let b = 1.0 - beta;
        let fb = f.powf(b);
        alpha / fb * (1.0 + (b * b / 24.0 * alpha * alpha / (fb * fb) + 0.25 * rho * beta * alpha * nu / fb + (2.0 - 3.0 * rho * rho) / 24.0 * nu * nu) * t)
    }
}

pub const PARAM_NAMES: [&str; 3] = ["alpha", "rho", "nu"];

/// The fit of one series.
#[derive(Debug, Clone)]
pub struct SabrFit {
    pub root: String,
    pub expiry: String,
    pub days: i64,
    pub forward: f64,
    pub n: usize,
    pub model: Sabr,
    pub status: LmStatus,
    pub at_bound: [bool; 3],
    /// Loss in squared spreads.
    pub loss: f64,
    pub rms_points: f64,
    pub max_points: f64,
    /// The largest error is more than `POOR_FACTOR` times the median spread of the series.
    pub poor: bool,
}

/// A fit is poor when its largest error (vol points) is more than this factor times the median spread (vol points).
/// Author choice, no study (A-139). It marks 7 of 53 series of SPX 2026-11-20.
pub const POOR_FACTOR: f64 = 5.0;

/// The bounds of alpha, rho, nu. Alpha scales with `F^(1-beta)` (alpha is about the volatility times `F^(1-beta)`).
pub fn bounds(f: f64, beta: f64) -> Vec<(f64, f64)> {
    let s = f.powf(1.0 - beta);
    vec![(1e-4 * s, 10.0 * s), (-0.999, 0.999), (1e-4, 10.0)]
}

fn model_of(p: &[f64], beta: f64) -> Sabr {
    Sabr { alpha: p[0], beta, rho: p[1], nu: p[2] }
}

/// Residuals `(model vol - mid vol) / spread` of one series.
pub fn residuals(s: &Series, beta: f64, p: &[f64]) -> Vec<f64> {
    let m = model_of(p, beta);
    s.quotes.iter().map(|q| (m.implied_vol(s.forward, q.strike, s.t) - q.iv_mid) / q.iv_spread().max(MIN_SPREAD)).collect()
}

/// Fit alpha, rho, nu of one series: LM from a fixed set of starts (rho and nu), the best final loss wins.
pub fn fit_series(s: &Series, beta: f64, max_iter: usize) -> Result<SabrFit, String> {
    if s.quotes.is_empty() {
        return Err(format!("{} {}: no quotes", s.root, s.expiry));
    }
    let b = bounds(s.forward, beta);
    let atm = s.quotes.iter().min_by(|a, c| (a.strike / s.forward).ln().abs().total_cmp(&(c.strike / s.forward).ln().abs())).map_or(0.2, |q| q.iv_mid);
    let a0 = atm * s.forward.powf(1.0 - beta);
    let resid = |p: &[f64]| residuals(s, beta, p);
    let mut best: Option<(Vec<f64>, f64, LmStatus)> = None;
    for &rho0 in &[-0.7, -0.3, 0.0] {
        for &nu0 in &[0.3, 1.0] {
            let (p, f, _, st) = lm_core(&resid, &b, &[a0, rho0, nu0], max_iter);
            if f.is_finite() && best.as_ref().is_none_or(|x| f < x.1) {
                best = Some((p, f, st));
            }
        }
    }
    let (p, loss, status) = best.ok_or_else(|| format!("{} {}: the loss is not finite at any start", s.root, s.expiry))?;
    let model = model_of(&p, beta);
    let errs: Vec<f64> = s.quotes.iter().map(|q| 100.0 * (model.implied_vol(s.forward, q.strike, s.t) - q.iv_mid)).collect();
    let rms = (errs.iter().map(|e| e * e).sum::<f64>() / errs.len() as f64).sqrt();
    let max = errs.iter().fold(0.0f64, |a, e| a.max(e.abs()));
    let mut sp: Vec<f64> = s.quotes.iter().map(|q| 100.0 * q.iv_spread()).collect();
    sp.sort_by(|a, c| a.total_cmp(c));
    let poor = max > POOR_FACTOR * sp[sp.len() / 2];
    let m = at_bound(&p, &b);
    Ok(SabrFit { root: s.root.clone(), expiry: s.expiry.clone(), days: s.days, forward: s.forward, n: s.quotes.len(), model, status, at_bound: [m[0], m[1], m[2]], loss, rms_points: rms, max_points: max, poor })
}

/// All series in parallel (rayon). A series that fails is returned as an error text next to the others.
pub fn fit_all(series: &[Series], beta: f64, max_iter: usize) -> Vec<Result<SabrFit, String>> {
    series.par_iter().map(|s| fit_series(s, beta, max_iter)).collect()
}
