//! A driver model as data: drivers, correlations, segments, net debt and shares.
//! Files: `analysis/<TICKER>/driver_model.toml` (the same shape as JSON in the test fixtures).
//!
//! A run can change a model without a change of the file: `Overrides` (driver means, flat or
//! ramp schedules, correlations, net debt, shares, horizon, year-1 pins). One set of
//! overrides serves all models; it replaces the build arguments and `dataclasses.replace`
//! calls of the three Python runners.

use super::driver::{CorrEntry, Driver, DriverDcf, DriverModel, Ramp, SegmentSpec};
use super::stats::err;
use super::DcfError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn default_years() -> usize {
    10
}

/// The content of a model file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSpec {
    #[serde(default)]
    pub ticker: String,
    /// The status text of the model (reviewed or not) and its warnings.
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub drivers: Vec<Driver>,
    #[serde(default)]
    pub corr: Vec<CorrEntry>,
    #[serde(default)]
    pub segments: Vec<SegmentSpec>,
    pub net_debt: f64,
    pub shares: f64,
    #[serde(default = "default_years")]
    pub projection_years: usize,
    /// Segment revenue by fiscal year, in dollars (record only; the engine does not read it).
    #[serde(default)]
    pub segment_history: BTreeMap<String, BTreeMap<String, f64>>,
}

pub fn model_path(repo: &Path, ticker: &str) -> PathBuf {
    repo.join("analysis").join(ticker.to_uppercase()).join("driver_model.toml")
}

/// Tickers that have a model file, sorted.
pub fn available_models(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(repo.join("analysis")) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().join("driver_model.toml").is_file() {
                out.push(name);
            }
        }
    }
    out.sort();
    out
}

impl ModelSpec {
    pub fn from_toml(text: &str) -> Result<ModelSpec, DcfError> {
        toml::from_str(text).map_err(|e| DcfError(format!("model file: {e}")))
    }

    /// Load `analysis/<TICKER>/driver_model.toml`. `Ok(None)` when the file does not exist.
    pub fn load(repo: &Path, ticker: &str) -> Result<Option<ModelSpec>, DcfError> {
        let path = model_path(repo, ticker);
        if !path.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path).map_err(|e| DcfError(format!("{}: {e}", path.display())))?;
        let mut spec = ModelSpec::from_toml(&text).map_err(|e| DcfError(format!("{}: {e}", path.display())))?;
        if spec.ticker.is_empty() {
            spec.ticker = ticker.to_uppercase();
        }
        Ok(Some(spec))
    }

    /// The drivers with each ramp written into its schedule for the horizon.
    fn resolved_drivers(&self) -> Result<Vec<Driver>, DcfError> {
        let mut drivers = self.drivers.clone();
        for d in drivers.iter_mut() {
            if let Some(r) = &d.ramp {
                if d.schedule.is_some() {
                    return err(format!("Driver {:?} has a ramp and a schedule: use one of them.", d.name));
                }
                if r.ramp_years < 2 {
                    return err(format!("Driver {:?}: ramp_years must be at least 2.", d.name));
                }
                if d.mean != r.start {
                    return err(format!(
                        "Driver {:?}: mean={} must equal the ramp start {} (the mean of a scheduled driver is its year-1 value).",
                        d.name, d.mean, r.start
                    ));
                }
                d.schedule = Some(r.schedule(self.projection_years));
            }
        }
        Ok(drivers)
    }

    /// Build the engine.
    pub fn engine(&self) -> Result<DriverDcf, DcfError> {
        let model = DriverModel::new(self.resolved_drivers()?, self.corr.clone())?;
        DriverDcf::new(model, self.segments.clone(), self.net_debt, self.shares, self.projection_years)
    }

    pub fn driver(&self, name: &str) -> Result<&Driver, DcfError> {
        self.drivers.iter().find(|d| d.name == name).ok_or_else(|| DcfError(format!("Unknown driver {name:?}.")))
    }

    /// A copy with the overrides applied. Nothing is checked here beyond the names; `engine()`
    /// applies the checks of the engine.
    pub fn with_overrides(&self, o: &Overrides) -> Result<ModelSpec, DcfError> {
        let mut m = self.clone();
        let core0 = m.drivers.iter().find(|d| d.name == "core_margin").map(|d| d.mean);
        if let Some(y) = o.years {
            m.projection_years = y;
        }
        if let Some(v) = o.net_debt {
            m.net_debt = v;
        }
        if let Some(v) = o.shares {
            m.shares = v;
        }
        let find = |m: &ModelSpec, name: &str| -> Result<usize, DcfError> {
            m.drivers.iter().position(|d| d.name == name).ok_or_else(|| DcfError(format!("Unknown driver {name:?}.")))
        };
        for (name, value) in &o.means {
            let i = find(&m, name)?;
            let d = &mut m.drivers[i];
            if d.schedule.is_some() || d.ramp.is_some() {
                return err(format!(
                    "Driver {name:?} has a schedule: set a flat value (flat) or a new ramp (ramps) instead of a mean."
                ));
            }
            let shift = value - d.mean;
            d.mean = *value;
            if o.shift_bounds {
                d.lo = d.lo.map(|v| v + shift);
                d.hi = d.hi.map(|v| v + shift);
            }
        }
        for (name, value) in &o.flat {
            let i = find(&m, name)?;
            let d = &mut m.drivers[i];
            d.schedule = None;
            d.ramp = None;
            d.mean = *value;
        }
        for (name, r) in &o.ramps {
            let i = find(&m, name)?;
            let d = &mut m.drivers[i];
            d.schedule = None;
            d.ramp = Some(r.clone());
            d.mean = r.start;
        }
        if o.keep_year1_margin {
            let core1 = m.drivers.iter().find(|d| d.name == "core_margin").map(|d| d.mean);
            if let (Some(c0), Some(c1)) = (core0, core1) {
                if c0 != c1 && !o.ramps.iter().any(|(n, _)| n == "margin_mix_drift") {
                    let d = m
                        .drivers
                        .iter_mut()
                        .find(|d| d.name == "margin_mix_drift")
                        .ok_or_else(|| DcfError("keep_year1_margin needs the driver margin_mix_drift.".into()))?;
                    let ramp = d
                        .ramp
                        .as_mut()
                        .ok_or_else(|| DcfError("keep_year1_margin needs a ramp on margin_mix_drift.".into()))?;
                    ramp.start += c0 - c1;
                    d.mean = ramp.start;
                    d.schedule = None;
                }
            }
        }
        for (a, b, rho) in &o.corr {
            if let Some(e) = m.corr.iter_mut().find(|e| (e.a == *a && e.b == *b) || (e.a == *b && e.b == *a)) {
                e.rho = *rho;
            } else {
                m.corr.push(CorrEntry { a: a.clone(), b: b.clone(), rho: *rho, note: String::new() });
            }
        }
        let seg = |m: &mut ModelSpec, name: &str| -> Result<usize, DcfError> {
            m.segments.iter().position(|s| s.name == name).ok_or_else(|| DcfError(format!("Unknown segment {name:?}.")))
        };
        for (name, growth) in &o.year1_growth {
            let i = seg(&mut m, name)?;
            m.segments[i].year1_growth = *growth;
        }
        for (name, revenue) in &o.year1_revenue {
            let i = seg(&mut m, name)?;
            let base = m.segments[i].base_revenue;
            m.segments[i].year1_growth = Some(revenue / base - 1.0);
        }
        Ok(m)
    }
}

/// Changes of a model for one run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Overrides {
    /// New mean of a driver without a schedule.
    pub means: Vec<(String, f64)>,
    /// With `means`: the bounds `lo` and `hi` move by the same amount as the mean.
    pub shift_bounds: bool,
    /// A scheduled driver becomes one flat value.
    pub flat: Vec<(String, f64)>,
    /// A driver gets this ramp as its schedule.
    pub ramps: Vec<(String, Ramp)>,
    /// New correlation of a pair (it is added when the pair is not in the model).
    pub corr: Vec<(String, String, f64)>,
    pub net_debt: Option<f64>,
    pub shares: Option<f64>,
    pub years: Option<usize>,
    /// Year-1 growth of a segment (`None` removes the pin).
    pub year1_growth: Vec<(String, Option<f64>)>,
    /// Year-1 revenue of a segment, in dollars: the pin is `revenue / base - 1`.
    pub year1_revenue: Vec<(String, f64)>,
    /// A change of the `core_margin` mean keeps the margin of year 1: the ramp of
    /// `margin_mix_drift` starts at the old start plus the old mean minus the new mean.
    pub keep_year1_margin: bool,
}

impl Overrides {
    pub fn mean(name: &str, v: f64) -> Overrides {
        Overrides { means: vec![(name.into(), v)], ..Overrides::default() }
    }
}

/// Independent closed-form DCF with every driver at its mean (a check of the engine).
/// Returns `(value per share, EV, PV of the terminal value)`.
pub fn hand_dcf_at_means(spec: &ModelSpec) -> Result<(f64, f64, f64), DcfError> {
    let drivers = spec.resolved_drivers()?;
    let get = |name: &str| -> Result<&Driver, DcfError> {
        drivers.iter().find(|d| d.name == name).ok_or_else(|| DcfError(format!("Unknown driver {name:?}.")))
    };
    let at = |name: &str, t: usize| -> Result<f64, DcfError> {
        let d = get(name)?;
        Ok(d.schedule.as_ref().map_or(d.mean, |s| s[t - 1]))
    };
    let m = |name: &str| -> Result<f64, DcfError> { Ok(get(name)?.mean) };
    let wacc = m("equity_weight")? * (m("rf")? + m("beta")? * m("erp")?)
        + (1.0 - m("equity_weight")?) * (m("rf")? + m("kd_spread")?) * (1.0 - m("tax_rate")?);
    let years = spec.projection_years;
    let (mut pv, mut last) = (0.0, 0.0);
    for t in 1..=years {
        let mut revenue = 0.0;
        for s in &spec.segments {
            let g = m(&s.growth_driver)?;
            revenue += match s.year1_growth {
                Some(y1) => s.base_revenue * (1.0 + y1) * (1.0 + g).powi(t as i32 - 1),
                None => s.base_revenue * (1.0 + g).powi(t as i32),
            };
        }
        let margin = at("core_margin", t)? + at("margin_mix_drift", t)? - at("sbc_ratio", t)?
            - at("ma_recurrence", t)? * at("ma_ratio", t)?;
        last = revenue * margin;
        pv += last / (1.0 + wacc).powi(t as i32);
    }
    let tg = m("terminal_growth")?.min(wacc - super::driver::TG_WACC_GAP);
    let tv = last * (1.0 + tg) / (wacc - tg) / (1.0 + wacc).powi(years as i32);
    Ok(((pv + tv - spec.net_debt) / spec.shares, pv + tv, tv))
}

/// Solve `f(x) = target` on `[lo, hi]` by bisection (40 steps), `None` when the target is not
/// between `f(lo)` and `f(hi)`. `f` may increase or decrease.
pub fn bisect<F: FnMut(f64) -> Result<f64, DcfError>>(
    mut f: F,
    lo: f64,
    hi: f64,
    target: f64,
) -> Result<Option<f64>, DcfError> {
    let (flo, fhi) = (f(lo)?, f(hi)?);
    let sign = if flo <= fhi { 1.0 } else { -1.0 };
    let (flo, fhi, target) = (sign * flo, sign * fhi, sign * target);
    if !(flo <= target && target <= fhi) {
        return Ok(None);
    }
    let (mut lo, mut hi) = (lo, hi);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if sign * f(mid)? < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(Some(0.5 * (lo + hi)))
}
