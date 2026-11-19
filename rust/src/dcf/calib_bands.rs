//! Calibration of revenue bands (ticket `T-20261119-dcf-process-calibration`, step 4).
//!
//! For each company and each base year T with 5 consecutive fiscal years up to T (4 growth values) and 3
//! consecutive fiscal years after T, a P10 to P90 band of revenue for T+1, T+2 and T+3 is made by four
//! methods, and the real revenue is placed against the band:
//!
//! - `legacy`: the rule of `backtest.rs`: one growth draw for all years, normal, mean = the revenue CAGR of
//!   the 5 years, standard deviation = the standard deviation of the 4 growth values (at least 0.01). The
//!   quantiles are exact (revenue is increasing in the growth), no draws.
//! - `ar1 pooled`: an AR(1) of growth, with the pooled parameters of the size bucket of T (phi from the
//!   pooled estimate, the group mean, the pooled residual deviation), started at the growth of year T.
//! - `ar1 fe+N`: phi with the within estimate and the Nickell correction, the shock of the within model, and
//!   the long-run mean of the company: the mean of its 4 growth values shrunk to the group mean (weight
//!   `tau^2 / (tau^2 + noise)`, from the spread of the company means).
//! - `ar1 fe+N, group mean`: the same phi and shock, the group mean for all companies.
//!
//! The AR(1) bands use the same normal draws for all companies (`draws` paths, fixed seed). Two samples:
//! parameters from all years (in sample, the target years are in the estimate), and parameters from the
//! years up to `TRAIN_END` with the bases from `TRAIN_END` on (out of sample).

use super::calibrate::{fit_ar1, mean_variance_factor, pairs, GROWTH_LIMITS};
use super::nprandom::Pcg64;
use super::panel::{bucket, CompanyPanel, BUCKET_LABELS};
use rayon::prelude::*;

pub const Z90: f64 = 1.2815515655446004;
pub const TRAIN_END: i32 = 2015;
pub const HISTORY: usize = 4;
pub const HORIZON: usize = 3;
pub const METHODS: [&str; 4] = ["legacy", "ar1 pooled", "ar1 fe+N", "ar1 fe+N, group mean"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupParams {
    pub phi_pooled: f64,
    pub mean_pooled: f64,
    pub sigma_pooled: f64,
    pub phi_fe: f64,
    pub mean_fe: f64,
    pub sigma_fe: f64,
    pub tau: f64,
}

/// Growth parameters of each size bucket from the pairs of the years up to `max_year`.
pub fn group_params(companies: &[CompanyPanel], max_year: Option<i32>) -> [Option<GroupParams>; 5] {
    let all = pairs(companies, &|r| r.growth_clean().filter(|g| *g > GROWTH_LIMITS.0 && *g < GROWTH_LIMITS.1));
    let mut out = [None; 5];
    for (b, slot) in out.iter_mut().enumerate() {
        let sel: Vec<_> = all.iter().filter(|p| p.size as usize == b && max_year.is_none_or(|y| p.year <= y)).copied().collect();
        if sel.len() < 100 {
            continue;
        }
        if let Some(f) = fit_ar1(&sel) {
            *slot = Some(GroupParams {
                phi_pooled: f.pooled.phi,
                mean_pooled: f.pooled.mean,
                sigma_pooled: f.pooled.sigma,
                phi_fe: f.fe_corrected.phi,
                mean_fe: f.fe_corrected.mean,
                sigma_fe: f.fe_corrected.sigma,
                tau: f.mean_sd_units,
            });
        }
    }
    out
}

/// One base year of one company.
#[derive(Debug, Clone, PartialEq)]
pub struct BandObs {
    pub ticker: String,
    pub year: i32,
    pub size: usize,
    pub base: f64,
    /// The growth of the 4 years up to the base year, oldest first.
    pub history: [f64; HISTORY],
    pub actual: [f64; HORIZON],
    /// A jump flag in the history or in the target years.
    pub has_jump: bool,
}

pub fn band_observations(companies: &[CompanyPanel]) -> Vec<BandObs> {
    let mut out = Vec::new();
    for c in companies {
        let n = c.rows.len();
        if n < HISTORY + 1 + HORIZON {
            continue;
        }
        for b in HISTORY..n - HORIZON {
            let chain = c.rows[b - HISTORY + 1..=b + HORIZON].iter().all(|r| r.prev_ok);
            if !chain {
                continue;
            }
            let mut history = [0.0; HISTORY];
            for (j, h) in history.iter_mut().enumerate() {
                *h = c.rows[b - HISTORY + 1 + j].growth.unwrap_or(0.0);
            }
            let mut actual = [0.0; HORIZON];
            for (j, a) in actual.iter_mut().enumerate() {
                *a = c.rows[b + 1 + j].revenue;
            }
            let has_jump = c.rows[b - HISTORY + 1..=b + HORIZON].iter().any(|r| r.jump);
            out.push(BandObs { ticker: c.ticker.clone(), year: c.rows[b].year, size: bucket(c.rows[b].revenue), base: c.rows[b].revenue, history, actual, has_jump });
        }
    }
    out
}

/// P10 and P90 of the revenue for the horizons 1 to 3, by the rule of the backtest.
pub fn legacy_band(o: &BandObs) -> [(f64, f64); HORIZON] {
    let first = o.base / o.history.iter().fold(1.0, |acc, g| acc * (1.0 + g)); // revenue 4 years before
    let growth = (o.base / first).powf(1.0 / HISTORY as f64) - 1.0;
    let m = o.history.iter().sum::<f64>() / HISTORY as f64;
    let std = (o.history.iter().map(|g| (g - m) * (g - m)).sum::<f64>() / HISTORY as f64).sqrt().max(0.01);
    let mut out = [(0.0, 0.0); HORIZON];
    for (h, slot) in out.iter_mut().enumerate() {
        let t = (h + 1) as i32;
        *slot = (o.base * (1.0 + growth - Z90 * std).powi(t), o.base * (1.0 + growth + Z90 * std).powi(t));
    }
    out
}

/// P10 and P90 of the revenue from an AR(1) of growth with shared normal draws `z` (draws x HORIZON).
pub fn ar1_band(base: f64, g0: f64, mu: f64, phi: f64, sigma: f64, z: &[[f64; HORIZON]]) -> [(f64, f64); HORIZON] {
    let n = z.len();
    let mut rev = vec![[0.0; HORIZON]; n];
    for (i, zi) in z.iter().enumerate() {
        let (mut g, mut r) = (g0, base);
        for h in 0..HORIZON {
            g = (mu + phi * (g - mu) + sigma * zi[h]).max(-0.9);
            r *= 1.0 + g;
            rev[i][h] = r;
        }
    }
    let mut out = [(0.0, 0.0); HORIZON];
    for (h, slot) in out.iter_mut().enumerate() {
        let mut v: Vec<f64> = rev.iter().map(|r| r[h]).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let q = |p: f64| v[((p * (n as f64 - 1.0)).round() as usize).min(n - 1)];
        *slot = (q(0.10), q(0.90));
    }
    out
}

fn clamp_growth(g: f64) -> f64 {
    g.clamp(GROWTH_LIMITS.0 + 1e-9, GROWTH_LIMITS.1 - 1e-9)
}

/// The four bands of one observation.
pub fn bands_for(o: &BandObs, p: &GroupParams, z: &[[f64; HORIZON]]) -> [[(f64, f64); HORIZON]; 4] {
    let g0 = clamp_growth(o.history[HISTORY - 1]);
    let hist: Vec<f64> = o.history.iter().map(|g| clamp_growth(*g)).collect();
    let mean_i = hist.iter().sum::<f64>() / HISTORY as f64;
    let noise = p.sigma_fe * p.sigma_fe * mean_variance_factor(p.phi_fe.clamp(-0.9, 0.99), HISTORY as f64);
    let w = if p.tau * p.tau + noise > 0.0 { p.tau * p.tau / (p.tau * p.tau + noise) } else { 0.0 };
    let mu_i = w * mean_i + (1.0 - w) * p.mean_fe;
    [
        legacy_band(o),
        ar1_band(o.base, g0, p.mean_pooled, p.phi_pooled, p.sigma_pooled, z),
        ar1_band(o.base, g0, mu_i, p.phi_fe, p.sigma_fe, z),
        ar1_band(o.base, g0, p.mean_fe, p.phi_fe, p.sigma_fe, z),
    ]
}

/// Counts: [below P10, inside, above P90].
pub type Count = [usize; 3];

/// `counts[subset][method][size (5 = all sizes)][horizon]`; subset 0: all observations, 1: no jump flag.
#[derive(Debug, Clone, PartialEq)]
pub struct Coverage {
    pub label: String,
    pub counts: Vec<Count>,
    pub n_obs: usize,
}

fn index(subset: usize, method: usize, size: usize, h: usize) -> usize {
    ((subset * METHODS.len() + method) * 6 + size) * HORIZON + h
}

impl Coverage {
    pub fn get(&self, subset: usize, method: usize, size: usize, h: usize) -> Count {
        self.counts[index(subset, method, size, h)]
    }
}

pub fn evaluate(label: &str, obs: &[BandObs], params: &[Option<GroupParams>; 5], draws: usize) -> Coverage {
    let mut rng = Pcg64::new(2026);
    let z: Vec<[f64; HORIZON]> = (0..draws).map(|_| [rng.standard_normal(), rng.standard_normal(), rng.standard_normal()]).collect();
    let size = 2 * METHODS.len() * 6 * HORIZON;
    let counts = obs
        .par_iter()
        .filter(|o| params[o.size].is_some())
        .fold(
            || vec![[0usize; 3]; size],
            |mut acc, o| {
                let p = params[o.size].as_ref().expect("params");
                let bands = bands_for(o, p, &z);
                for (m, band) in bands.iter().enumerate() {
                    for h in 0..HORIZON {
                        let (lo, hi) = band[h];
                        let slot = if o.actual[h] < lo {
                            0
                        } else if o.actual[h] > hi {
                            2
                        } else {
                            1
                        };
                        for subset in 0..2 {
                            if subset == 1 && o.has_jump {
                                continue;
                            }
                            acc[index(subset, m, o.size, h)][slot] += 1;
                            acc[index(subset, m, 5, h)][slot] += 1;
                        }
                    }
                }
                acc
            },
        )
        .reduce(
            || vec![[0usize; 3]; size],
            |mut a, b| {
                for (x, y) in a.iter_mut().zip(&b) {
                    for k in 0..3 {
                        x[k] += y[k];
                    }
                }
                a
            },
        );
    let n_obs = obs.iter().filter(|o| params[o.size].is_some()).count();
    Coverage { label: label.to_string(), counts, n_obs }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageReport {
    pub in_sample: Coverage,
    pub out_of_sample: Coverage,
}

pub fn coverage_report(companies: &[CompanyPanel], draws: usize) -> CoverageReport {
    let obs = band_observations(companies);
    let all = group_params(companies, None);
    let in_sample = evaluate("IN SAMPLE: parameters from all years", &obs, &all, draws);
    let train = group_params(companies, Some(TRAIN_END));
    let later: Vec<BandObs> = obs.iter().filter(|o| o.year >= TRAIN_END).cloned().collect();
    let out_of_sample = evaluate(&format!("OUT OF SAMPLE: parameters from years up to {TRAIN_END}, base years from {TRAIN_END}"), &later, &train, draws);
    CoverageReport { in_sample, out_of_sample }
}

pub fn render_coverage(out: &mut String, r: &CoverageReport) {
    use std::fmt::Write;
    let _ = writeln!(
        out,
        "\nBAND COVERAGE. A band is P10 to P90 of the revenue of T+h. Target: inside 80%, below 10%, above 10%. Cells: below% / inside% / above%.\n  Base year T needs {} consecutive fiscal years up to T and {} after. Size: revenue of T. Actuals are the values as now reported.",
        HISTORY + 1,
        HORIZON
    );
    for cov in [&r.in_sample, &r.out_of_sample] {
        for (subset, name) in [(0, "all observations"), (1, "no jump flag in the years used")] {
            let _ = writeln!(out, "\n  {} ({} observations), {name}", cov.label, cov.n_obs);
            let _ = write!(out, "  {:<8}{:>2}{:>7}", "size", "h", "n");
            for m in METHODS {
                let _ = write!(out, "  {:<22}", m);
            }
            let _ = writeln!(out);
            for size in 0..6 {
                for h in 0..HORIZON {
                    let n: usize = cov.get(subset, 0, size, h).iter().sum();
                    if n == 0 {
                        continue;
                    }
                    let label = BUCKET_LABELS.get(size).copied().unwrap_or("all");
                    let _ = write!(out, "  {label:<8}{:>2}{n:>7}", h + 1);
                    for m in 0..METHODS.len() {
                        let c = cov.get(subset, m, size, h);
                        let t = c.iter().sum::<usize>() as f64;
                        let _ = write!(out, "  {:>5.1} /{:>5.1} /{:>5.1}   ", c[0] as f64 / t * 100.0, c[1] as f64 / t * 100.0, c[2] as f64 / t * 100.0);
                    }
                    let _ = writeln!(out);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legacy_band_is_the_cagr_band() {
        let o = BandObs { ticker: "T".into(), year: 2020, size: 1, base: 100.0, history: [0.1; 4], actual: [110.0, 121.0, 133.1], has_jump: false };
        let b = legacy_band(&o);
        // std = 0 -> floor 0.01
        assert!((b[0].0 - 100.0 * (1.1 - Z90 * 0.01)).abs() < 1e-9);
        assert!((b[2].1 - 100.0 * (1.1 + Z90 * 0.01f64).powi(3)).abs() < 1e-9);
    }

    #[test]
    fn an_ar1_band_with_no_shock_is_a_point_on_the_mean_reverting_path() {
        let z = vec![[0.0; HORIZON]; 10];
        let b = ar1_band(100.0, 0.2, 0.05, 0.5, 0.0, &z);
        let g1 = 0.05 + 0.5 * 0.15;
        let g2 = 0.05 + 0.5 * (g1 - 0.05);
        let g3 = 0.05 + 0.5 * (g2 - 0.05);
        assert!((b[2].0 - 100.0 * (1.0 + g1) * (1.0 + g2) * (1.0 + g3)).abs() < 1e-9);
        assert_eq!(b[2].0, b[2].1);
    }
}
