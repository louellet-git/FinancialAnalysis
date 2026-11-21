//! Calibration: Levenberg-Marquardt with a numerical Jacobian and random starts (fixed seed).
//!
//! The residuals are `(model price - mid) / (vega * spread)` (`base::residuals`), so the loss counts the error in units of
//! the quoted spread. The bounds are the bounds of the model; a step is clamped into them. The columns of the Jacobian
//! run in parallel (rayon), and the series of one residual run in parallel too.

use super::base::{loss, residuals, CharModel, QuoteSet};
use crate::ratemodels::rng::Rng;
use rayon::prelude::*;

/// Iterations of stage 1 of `fit` for each refined start.
pub const STAGE1_ITER: usize = 3;

#[derive(Debug, Clone)]
pub struct FitOptions {
    /// The number of random starts (the first one is the plain start).
    pub starts: usize,
    /// The number of the best starts (by loss) that LM refines.
    pub refine: usize,
    pub seed: u64,
    pub max_iter: usize,
}

impl Default for FitOptions {
    fn default() -> Self {
        FitOptions { starts: 20, refine: 3, seed: 7, max_iter: 60 }
    }
}

#[derive(Debug, Clone)]
pub struct FitResult<M> {
    pub model: M,
    pub loss: f64,
    pub n_quotes: usize,
    pub iterations: usize,
    /// The loss of each refined start after stage 1 (`STAGE1_ITER` iterations). It is not a final loss: only the best start goes on.
    pub refined_losses: Vec<f64>,
    /// How LM ended for the best start (final).
    pub status: LmStatus,
    /// How stage 1 ended for each refined start (not final).
    pub statuses: Vec<LmStatus>,
}

/// How a Levenberg-Marquardt run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LmStatus {
    /// The last accepted step lowered the loss by a relative 1e-10 or less.
    Converged,
    /// `max_iter` iterations, and the loss still fell.
    IterationCap,
    /// No step lowered the loss, even with the damping at its cap (a bound or a flat valley).
    LambdaCap,
}

impl LmStatus {
    pub fn text(self) -> &'static str {
        match self {
            LmStatus::Converged => "converged",
            LmStatus::IterationCap => "NOT converged: iteration cap",
            LmStatus::LambdaCap => "stopped: no step lowers the loss (damping cap)",
        }
    }
}

/// Which parameters are within 0.1% of their range from a bound.
pub fn at_bound(p: &[f64], bounds: &[(f64, f64)]) -> Vec<bool> {
    p.iter().zip(bounds).map(|(x, (lo, hi))| (x - lo).abs() <= 1e-3 * (hi - lo) || (hi - x).abs() <= 1e-3 * (hi - lo)).collect()
}

/// ATM volatility and skew (the slope of mid volatility on ln(K / F)) of the series with the most quotes.
pub fn atm_and_skew(set: &QuoteSet) -> (f64, f64) {
    let Some(s) = set.series.iter().max_by_key(|s| s.quotes.len()) else { return (0.2, -0.1) };
    let pts: Vec<(f64, f64)> = s.quotes.iter().map(|q| ((q.strike / s.forward).ln(), q.iv_mid)).collect();
    let n = pts.len() as f64;
    let (mx, my) = (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n);
    let (sxy, sxx) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + (p.0 - mx) * (p.1 - my), a.1 + (p.0 - mx) * (p.0 - mx)));
    let slope = if sxx > 0.0 { sxy / sxx } else { -0.1 };
    let atm = pts.iter().min_by(|a, b| a.0.abs().total_cmp(&b.0.abs())).map_or(my, |p| p.1);
    (atm.clamp(0.02, 3.0), slope)
}

fn clamp_into(p: &mut [f64], b: &[(f64, f64)]) {
    for (x, (lo, hi)) in p.iter_mut().zip(b) {
        *x = x.clamp(*lo, *hi);
    }
}

/// Solve `A x = b` (Gaussian elimination with partial pivoting). `None` for a singular matrix.
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for c in 0..n {
        let piv = (c..n).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))?;
        if a[piv][c].abs() < 1e-300 {
            return None;
        }
        a.swap(c, piv);
        b.swap(c, piv);
        for r in c + 1..n {
            let f = a[r][c] / a[c][c];
            let pivot_row = a[c].clone();
            for (x, y) in a[r][c..].iter_mut().zip(&pivot_row[c..]) {
                *x -= f * y;
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let s: f64 = (r + 1..n).map(|k| a[r][k] * x[k]).sum();
        x[r] = (b[r] - s) / a[r][r];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

fn sumsq(r: &[f64]) -> f64 {
    r.iter().map(|x| x * x).sum()
}

/// Levenberg-Marquardt from `p0` for the residuals of a model on a quote set. Returns the parameters, the loss and the number of iterations.
pub fn levenberg_marquardt<M: CharModel>(set: &QuoteSet, p0: &[f64], max_iter: usize) -> (Vec<f64>, f64, usize, LmStatus) {
    lm_core(&|p: &[f64]| residuals(&M::from_vec(p), set), &M::bounds(), p0, max_iter)
}

/// The core of `levenberg_marquardt` for any residual function (SABR uses it with its own residuals).
pub fn lm_core(resid: &(dyn Fn(&[f64]) -> Vec<f64> + Sync), bounds: &[(f64, f64)], p0: &[f64], max_iter: usize) -> (Vec<f64>, f64, usize, LmStatus) {
    let np = p0.len();
    let mut p = p0.to_vec();
    clamp_into(&mut p, bounds);
    let mut r = resid(&p);
    let mut f = sumsq(&r);
    let mut lambda = 1e-2;
    let mut iters = 0;
    let mut status = LmStatus::IterationCap;
    while iters < max_iter {
        iters += 1;
        // Jacobian by forward differences (backward at the upper bound), the columns in parallel.
        let cols: Vec<Vec<f64>> = (0..np)
            .into_par_iter()
            .map(|j| {
                let h = 1e-6 * p[j].abs().max(1e-2);
                let step = if p[j] + h <= bounds[j].1 { h } else { -h };
                let mut q = p.clone();
                q[j] += step;
                let rj = resid(&q);
                
                rj.iter().zip(&r).map(|(a, b)| (a - b) / step).collect()
            })
            .collect();
        let jtj: Vec<Vec<f64>> = (0..np).map(|a| (0..np).map(|b| cols[a].iter().zip(&cols[b]).map(|(x, y)| x * y).sum()).collect()).collect();
        let g: Vec<f64> = (0..np).map(|a| cols[a].iter().zip(&r).map(|(x, y)| x * y).sum()).collect();
        let mut improved = false;
        let mut small_gain = false;
        while lambda < 1e10 {
            let mut a = jtj.clone();
            for (i, row) in a.iter_mut().enumerate() {
                row[i] += lambda * jtj[i][i].max(1e-12);
            }
            let Some(delta) = solve(a, g.iter().map(|x| -x).collect()) else {
                lambda *= 10.0;
                continue;
            };
            let mut q: Vec<f64> = p.iter().zip(&delta).map(|(x, d)| x + d).collect();
            clamp_into(&mut q, bounds);
            let rq = resid(&q);
            let fq = sumsq(&rq);
            if fq.is_finite() && fq < f {
                let rel = (f - fq) / f.max(1e-300);
                p = q;
                r = rq;
                f = fq;
                lambda = (lambda / 3.0).max(1e-9);
                improved = true;
                small_gain = rel <= 1e-10;
                break;
            }
            lambda *= 4.0;
        }
        if !improved {
            // A loss under 1e-12 squared spreads is an exact fit: no step can lower it further.
            status = if f < 1e-12 { LmStatus::Converged } else { LmStatus::LambdaCap };
            break;
        }
        if small_gain {
            status = LmStatus::Converged;
            break;
        }
    }
    (p, f, iters, status)
}

/// The start vector number `i`: the plain start for `i = 0`, else the plain start with a log-normal (a normal for rho) disturbance.
pub fn random_start<M: CharModel>(base: &[f64], seed: u64, i: usize) -> Vec<f64> {
    let bounds = M::bounds();
    let mut p = base.to_vec();
    if i > 0 {
        let mut rng = Rng::for_path(seed, i as u64);
        for (j, x) in p.iter_mut().enumerate() {
            let z = rng.normal();
            *x = if M::names()[j] == "rho" { *x + 0.3 * z } else { *x * (0.5 * z).exp() };
        }
    }
    clamp_into(&mut p, &bounds);
    p
}

/// Fit: `starts` random starts, the loss at each, LM from the best `refine`, and the best result.
pub fn fit<M: CharModel>(set: &QuoteSet, opts: &FitOptions) -> Result<FitResult<M>, String> {
    let (atm, skew) = atm_and_skew(set);
    let base = M::start(atm, skew);
    let starts: Vec<Vec<f64>> = (0..opts.starts.max(1)).map(|i| random_start::<M>(&base, opts.seed, i)).collect();
    let mut scored: Vec<(f64, Vec<f64>)> = starts.into_par_iter().map(|p| (loss(&M::from_vec(&p), set), p)).collect();
    scored.retain(|(l, _)| l.is_finite());
    scored.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut best: Option<(Vec<f64>, f64, LmStatus)> = None;
    let (mut iterations, mut refined_losses, mut statuses) = (0, Vec::new(), Vec::new());
    // Stage 1: each of the best starts gets `STAGE1_ITER` iterations (the same effort for each). Stage 2: the best one goes on.
    for (_, p0) in scored.iter().take(opts.refine.max(1)) {
        let (p, f, it, st) = levenberg_marquardt::<M>(set, p0, STAGE1_ITER.min(opts.max_iter));
        iterations += it;
        refined_losses.push(f);
        statuses.push(st);
        if f.is_finite() && best.as_ref().is_none_or(|b| f < b.1) {
            best = Some((p, f, st));
        }
    }
    let (p, f, status) = best.ok_or("the loss is not finite at any start (the characteristic function gave a non-finite value)")?;
    let (p, f, status) = if status == LmStatus::IterationCap && opts.max_iter > STAGE1_ITER {
        let (p2, f2, it2, st2) = levenberg_marquardt::<M>(set, &p, opts.max_iter - STAGE1_ITER);
        iterations += it2;
        (p2, f2, st2)
    } else {
        (p, f, status)
    };
    Ok(FitResult { model: M::from_vec(&p), loss: f, n_quotes: set.n_quotes(), iterations, refined_losses, status, statuses })
}
