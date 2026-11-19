//! Parallel, reproducible path simulation.
//!
//! The paths run in parallel with rayon. Path `p` uses the stream `Rng::for_path(seed, p)`,
//! so the result is the same with 1 thread and with 100 threads. Each task owns one buffer
//! for a path; there is no allocation inside the time loop.

use super::models::{RateModel, Result};
use super::rng::Rng;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct SimConfig {
    pub years: f64,
    pub steps_per_year: usize,
    pub paths: usize,
    pub seed: u64,
    /// The rate at time 0. `None`: the default of the model.
    pub r0: Option<f64>,
}

impl SimConfig {
    pub fn steps(&self) -> usize {
        ((self.years * self.steps_per_year as f64).round() as usize).max(1)
    }
    pub fn dt(&self) -> f64 {
        self.years / self.steps() as f64
    }
    fn check(&self) -> Result<()> {
        if !(self.years.is_finite() && self.years > 0.0) {
            return Err(format!("years must be positive; got {}", self.years));
        }
        if self.steps_per_year == 0 || self.paths == 0 {
            return Err("steps-per-year and paths must be at least 1".into());
        }
        Ok(())
    }
}

/// All paths in one array. Path p is `rates[p*(steps+1) .. (p+1)*(steps+1)]`; index i is time i*dt.
pub struct Paths {
    pub dt: f64,
    pub steps: usize,
    pub paths: usize,
    pub rates: Vec<f64>,
}

impl Paths {
    pub fn path(&self, p: usize) -> &[f64] {
        &self.rates[p * (self.steps + 1)..(p + 1) * (self.steps + 1)]
    }
}

#[inline]
fn fill_path<M: RateModel>(m: &M, prep: &M::Prep, r0: f64, seed: u64, p: usize, buf: &mut [f64]) {
    let mut rng = Rng::for_path(seed, p as u64);
    buf[0] = r0;
    let mut r = r0;
    for i in 0..buf.len() - 1 {
        r = m.step(prep, i, r, &mut rng);
        buf[i + 1] = r;
    }
}

/// Simulate and keep all paths. The memory is paths * (steps+1) * 8 bytes.
pub fn simulate_paths<M: RateModel>(m: &M, cfg: &SimConfig) -> Result<Paths> {
    m.validate()?;
    cfg.check()?;
    let (steps, dt) = (cfg.steps(), cfg.dt());
    let prep = m.prepare(dt, steps);
    let r0 = cfg.r0.unwrap_or_else(|| m.default_r0());
    let mut rates = vec![0.0; cfg.paths * (steps + 1)];
    rates.par_chunks_mut(steps + 1).enumerate().for_each(|(p, buf)| fill_path(m, &prep, r0, cfg.seed, p, buf));
    Ok(Paths { dt, steps, paths: cfg.paths, rates })
}

/// Simulate and reduce each path at once with `f(path_index, rates)`. The memory is small.
/// The results come in path order.
pub fn simulate_map<M: RateModel, T: Send>(m: &M, cfg: &SimConfig, f: impl Fn(usize, &[f64]) -> T + Sync + Send) -> Result<Vec<T>> {
    m.validate()?;
    cfg.check()?;
    let (steps, dt) = (cfg.steps(), cfg.dt());
    let prep = m.prepare(dt, steps);
    let r0 = cfg.r0.unwrap_or_else(|| m.default_r0());
    Ok((0..cfg.paths)
        .into_par_iter()
        .map_init(
            || vec![0.0; steps + 1],
            |buf, p| {
                fill_path(m, &prep, r0, cfg.seed, p, buf);
                f(p, buf)
            },
        )
        .collect())
}

/// The integral of r from time 0 to step `upto` by the trapezoid rule. exp(-integral) is the discount factor.
/// The error is of order dt^2.
pub fn trapezoid(path: &[f64], dt: f64, upto: usize) -> f64 {
    let mut s = 0.5 * (path[0] + path[upto]);
    for v in &path[1..upto] {
        s += v;
    }
    s * dt
}

/// Mean, standard deviation, and standard error of the mean.
pub fn mean_sd_se(x: &[f64]) -> (f64, f64, f64) {
    let n = x.len() as f64;
    let mean = x.iter().sum::<f64>() / n;
    let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1.0).max(1.0);
    (mean, var.sqrt(), (var / n).sqrt())
}

/// The q-quantile (0 <= q <= 1) of a sorted slice, by linear interpolation.
pub fn quantile_sorted(sorted: &[f64], q: f64) -> f64 {
    let pos = q * (sorted.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}
