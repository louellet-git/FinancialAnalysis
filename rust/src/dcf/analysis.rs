//! Generic analyses over a driver model: runs with overrides, one-at-a-time sensitivities
//! and the driver value that a price implies. One feature set for all models.

use super::driver::DriverDcfResult;
use super::model::{bisect, ModelSpec, Overrides};
use super::stats::{percentile, sorted_copy};
use super::DcfError;
use serde::Serialize;

/// Quantiles of a run without the attribution loop (`_quick` of the Python runners).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Quick {
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub tg_pinned: f64,
}

/// Run `spec` with `overrides`: same seed, no attribution.
pub fn quick(spec: &ModelSpec, overrides: &Overrides, n: usize, seed: u64) -> Result<Quick, DcfError> {
    let eng = spec.with_overrides(overrides)?.engine()?;
    let v = eng.run_paths(n, seed, &[])?;
    let s = sorted_copy(&v.per_share);
    Ok(Quick {
        p10: percentile(&s, 10.0),
        p25: percentile(&s, 25.0),
        p50: percentile(&s, 50.0),
        p75: percentile(&s, 75.0),
        p90: percentile(&s, 90.0),
        tg_pinned: v.tg_pinned_fraction,
    })
}

/// The full run (with attribution) of `spec` with `overrides`.
pub fn full(spec: &ModelSpec, overrides: &Overrides, n: usize, seed: u64) -> Result<DriverDcfResult, DcfError> {
    spec.with_overrides(overrides)?.engine()?.run(n, Some(seed))
}

/// Value per share with every driver at its mean.
pub fn at_means(spec: &ModelSpec, overrides: &Overrides) -> Result<f64, DcfError> {
    spec.with_overrides(overrides)?.engine()?.value_at_means()
}

/// P50 per share when the mean of `driver` is `value` (the other overrides in `base` stay).
pub fn p50_with_mean(spec: &ModelSpec, base: &Overrides, driver: &str, value: f64, n: usize, seed: u64) -> Result<f64, DcfError> {
    let mut o = base.clone();
    o.means.retain(|(d, _)| d != driver);
    o.means.push((driver.to_string(), value));
    Ok(quick(spec, &o, n, seed)?.p50)
}

/// The mean of `driver` on `[lo, hi]` for which the P50 per share is `price`. `None` when
/// the price is out of range. The P50 may rise or fall with the driver.
#[allow(clippy::too_many_arguments)]
pub fn implied_mean(
    spec: &ModelSpec,
    base: &Overrides,
    driver: &str,
    lo: f64,
    hi: f64,
    price: f64,
    n: usize,
    seed: u64,
) -> Result<Option<f64>, DcfError> {
    bisect(|x| p50_with_mean(spec, base, driver, x, n, seed), lo, hi, price)
}

/// One-at-a-time sensitivity: P50 for each value of one driver mean.
pub fn sensitivity(
    spec: &ModelSpec,
    base: &Overrides,
    driver: &str,
    values: &[f64],
    n: usize,
    seed: u64,
) -> Result<Vec<(f64, f64)>, DcfError> {
    values.iter().map(|v| Ok((*v, p50_with_mean(spec, base, driver, *v, n, seed)?))).collect()
}
