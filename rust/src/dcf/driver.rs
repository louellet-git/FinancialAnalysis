//! Driver-based Monte Carlo DCF engine (the Rust form of `financial_analysis/driver_dcf.py`).
//!
//! Drivers are the only random objects. They are drawn together from correlated standard
//! normals (Cholesky factor of the correlation matrix), then `mean + std * z`, then clipped
//! to `[lo, hi]` where a driver has bounds (clip after correlation: see the Python module
//! text, which gave the reasons). The top-level items (revenue, FCFF margin, WACC, terminal
//! value) are plain formulas over the driver values.
//!
//! The order of the floating-point operations is the order of the Python code.

use super::nprandom::Pcg64;
use super::stats::{cholesky, err, min_eigenvalue, percentile, share_below, sorted_copy};
use super::{clock_seed, DcfError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The Gordon denominator is at least this on every path (T-DCF-2).
pub const TG_WACC_GAP: f64 = 0.02;

/// Drivers that the engine needs by exact name (beyond the growth driver of each segment).
pub const ROLE_DRIVERS: [&str; 12] = [
    "core_margin",
    "margin_mix_drift",
    "sbc_ratio",
    "ma_ratio",
    "ma_recurrence",
    "rf",
    "beta",
    "erp",
    "kd_spread",
    "tax_rate",
    "equity_weight",
    "terminal_growth",
];

fn default_dist() -> String {
    "normal".to_string()
}

/// A per-year ramp that gives a schedule for any number of years:
/// `start + (end - start) * min(i, ramp_years - 1) / (ramp_years - 1)` for year `i + 1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ramp {
    pub start: f64,
    pub end: f64,
    pub ramp_years: usize,
}

impl Ramp {
    pub fn schedule(&self, years: usize) -> Vec<f64> {
        let last_year = self.ramp_years.max(2) - 1;
        let last = last_year as f64;
        (0..years)
            .map(|i| self.start + (self.end - self.start) * (i.min(last_year) as f64) / last)
            .collect()
    }
}

/// One model input. See the Python class `Driver` for the rules; they are the same here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Driver {
    pub name: String,
    pub stochastic: bool,
    pub mean: f64,
    #[serde(default)]
    pub std: f64,
    #[serde(default = "default_dist")]
    pub dist: String,
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub hi: Option<f64>,
    #[serde(default)]
    pub rationale: String,
    /// Per-year values (deterministic drivers only). Mean must equal `schedule[0]`.
    #[serde(default)]
    pub schedule: Option<Vec<f64>>,
    /// A schedule as a ramp: the engine builds `schedule` from it for the horizon.
    #[serde(default)]
    pub ramp: Option<Ramp>,
}

impl Driver {
    pub fn new(name: &str, stochastic: bool, mean: f64) -> Driver {
        Driver {
            name: name.to_string(),
            stochastic,
            mean,
            std: 0.0,
            dist: default_dist(),
            lo: None,
            hi: None,
            rationale: String::new(),
            schedule: None,
            ramp: None,
        }
    }

    /// The same checks as `Driver.__post_init__`.
    pub fn validate(&self) -> Result<(), DcfError> {
        let n = &self.name;
        if n.is_empty() {
            return err("Driver name must be non-empty.");
        }
        if let Some(s) = &self.schedule {
            if self.stochastic {
                return err(format!(
                    "Driver {n:?} is stochastic but declares a schedule - schedules are for deterministic per-year paths only."
                ));
            }
            if s.is_empty() {
                return err(format!("Driver {n:?}: schedule must be non-empty."));
            }
            if s[0] != self.mean {
                return err(format!(
                    "Driver {n:?}: mean={} must equal schedule[0]={} - the mean of a scheduled driver is its year-1 value by convention.",
                    self.mean, s[0]
                ));
            }
        }
        if self.dist != "normal" && self.dist != "truncnorm" {
            return err(format!("Driver {n:?}: dist must be one of (\"normal\", \"truncnorm\"), got {:?}.", self.dist));
        }
        if self.stochastic {
            if self.std <= 0.0 {
                return err(format!(
                    "Driver {n:?} is stochastic but std={} - stochastic drivers require std > 0.",
                    self.std
                ));
            }
        } else {
            if self.std != 0.0 {
                return err(format!(
                    "Driver {n:?} is deterministic but std={} - set std=0 (or mark the driver stochastic).",
                    self.std
                ));
            }
            if self.lo.is_some() || self.hi.is_some() {
                return err(format!(
                    "Driver {n:?} is deterministic but declares bounds - bounds only apply to stochastic truncnorm drivers."
                ));
            }
        }
        if self.dist == "truncnorm" {
            if self.lo.is_none() && self.hi.is_none() {
                return err(format!("Driver {n:?}: dist='truncnorm' requires lo and/or hi."));
            }
        } else if self.lo.is_some() || self.hi.is_some() {
            return err(format!(
                "Driver {n:?}: lo/hi set but dist='normal' - declare dist='truncnorm' to make the truncation explicit."
            ));
        }
        if let (Some(lo), Some(hi)) = (self.lo, self.hi) {
            if lo >= hi {
                return err(format!("Driver {n:?}: lo={lo} must be < hi={hi}."));
            }
        }
        if let Some(lo) = self.lo {
            if self.mean < lo {
                return err(format!("Driver {n:?}: mean={} below lo={lo}.", self.mean));
            }
        }
        if let Some(hi) = self.hi {
            if self.mean > hi {
                return err(format!("Driver {n:?}: mean={} above hi={hi}.", self.mean));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorrEntry {
    pub a: String,
    pub b: String,
    pub rho: f64,
    /// Why this entry has this value (text of the model file; not used in the calculation).
    #[serde(default)]
    pub note: String,
}

/// One revenue segment: base revenue and the name of the growth driver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentSpec {
    pub name: String,
    pub base_revenue: f64,
    pub growth_driver: String,
    #[serde(default)]
    pub year1_growth: Option<f64>,
}

/// The drivers and the pairwise correlations of the stochastic ones.
#[derive(Debug, Clone, PartialEq)]
pub struct DriverModel {
    pub drivers: Vec<Driver>,
    pub corr: Vec<CorrEntry>,
}

/// The values that a driver takes in a run.
#[derive(Debug, Clone)]
pub enum Draw {
    /// One value for every path and year.
    Scalar(f64),
    /// One value per path (stochastic drivers).
    PerPath(Vec<f64>),
    /// One value per year (scheduled drivers).
    PerYear(Vec<f64>),
}

impl Draw {
    #[inline]
    fn get(&self, year: usize, path: usize) -> f64 {
        match self {
            Draw::Scalar(v) => *v,
            Draw::PerPath(v) => v[path],
            Draw::PerYear(v) => v[year],
        }
    }
}

pub type Draws = HashMap<String, Draw>;

impl DriverModel {
    pub fn new(drivers: Vec<Driver>, corr: Vec<CorrEntry>) -> Result<DriverModel, DcfError> {
        let mut seen = std::collections::HashSet::new();
        let mut dupes: Vec<&str> = Vec::new();
        for d in &drivers {
            if !seen.insert(d.name.as_str()) {
                dupes.push(&d.name);
            }
        }
        if !dupes.is_empty() {
            dupes.sort();
            return err(format!("Duplicate driver names: {dupes:?}."));
        }
        for d in &drivers {
            d.validate()?;
        }
        Ok(DriverModel { drivers, corr })
    }

    pub fn driver(&self, name: &str) -> Result<&Driver, DcfError> {
        self.drivers
            .iter()
            .find(|d| d.name == name)
            .ok_or_else(|| DcfError(format!("Unknown driver {name:?}.")))
    }

    pub fn stochastic_drivers(&self) -> Vec<&Driver> {
        self.drivers.iter().filter(|d| d.stochastic).collect()
    }

    /// Correlation matrix of the stochastic drivers, declaration order. Checked, never repaired.
    pub fn correlation_matrix(&self) -> Result<Vec<Vec<f64>>, DcfError> {
        let stoch = self.stochastic_drivers();
        let index: HashMap<&str, usize> = stoch.iter().enumerate().map(|(i, d)| (d.name.as_str(), i)).collect();
        let det: std::collections::HashSet<&str> =
            self.drivers.iter().filter(|d| !d.stochastic).map(|d| d.name.as_str()).collect();
        let k = stoch.len();
        let mut mat = vec![vec![0.0; k]; k];
        for (i, row) in mat.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        let mut seen: HashMap<(String, String), f64> = HashMap::new();
        for e in &self.corr {
            let (a, b, rho) = (&e.a, &e.b, e.rho);
            for name in [a, b] {
                if det.contains(name.as_str()) {
                    return err(format!(
                        "Correlation ({a:?}, {b:?}) references deterministic driver {name:?} - only stochastic drivers correlate."
                    ));
                }
                if !index.contains_key(name.as_str()) {
                    return err(format!("Correlation ({a:?}, {b:?}) references unknown driver {name:?}."));
                }
            }
            if a == b {
                return err(format!("Self-correlation ({a:?}, {a:?}) is implicit (1.0) - remove it."));
            }
            if !(-1.0..=1.0).contains(&rho) {
                return err(format!("Correlation ({a:?}, {b:?}) = {rho} outside [-1, 1]."));
            }
            let key = if a < b { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
            if let Some(prev) = seen.get(&key) {
                if *prev != rho {
                    return err(format!(
                        "Conflicting correlations for pair {key:?}: {prev} vs {rho} - the matrix must be symmetric."
                    ));
                }
            }
            seen.insert(key, rho);
            let (i, j) = (index[a.as_str()], index[b.as_str()]);
            mat[i][j] = rho;
            mat[j][i] = rho;
        }
        if cholesky(&mat).is_err() {
            return err(format!(
                "Driver correlation matrix is not positive definite: minimum eigenvalue {:.6e} (must be > 0). Revise the pairwise correlations - the matrix is not silently repaired.",
                min_eigenvalue(&mat)
            ));
        }
        Ok(mat)
    }

    /// The correlated standard normals of the stochastic drivers (`chol @ rng.standard_normal((k, n))`):
    /// `k` rows of `n` values. The same seed gives the same rows for any means, so a run
    /// with shifted means can use them again.
    pub fn correlated_z(&self, n: usize, seed: u64) -> Result<Vec<Vec<f64>>, DcfError> {
        if n == 0 {
            return err("n must be positive, got 0.");
        }
        let k = self.stochastic_drivers().len();
        let chol = cholesky(&self.correlation_matrix()?).map_err(|_| DcfError("Cholesky failed.".into()))?;
        let mut rng = Pcg64::new(seed);
        let zraw = rng.standard_normal_matrix(k, n);
        // z = chol @ zraw, sum over the inner index in order
        let mut z = vec![vec![0.0; n]; k];
        for i in 0..k {
            for l in 0..k {
                let c = chol[i][l];
                if c == 0.0 {
                    continue;
                }
                for j in 0..n {
                    z[i][j] += c * zraw[l][j];
                }
            }
        }
        Ok(z)
    }

    /// Draw `n` joint paths for every driver (see `DriverModel.sample` in Python).
    /// `mean_overrides` shifts the means and keeps the same z-draws.
    pub fn sample(&self, n: usize, seed: u64, mean_overrides: &[(String, f64)]) -> Result<Draws, DcfError> {
        self.check_overrides(mean_overrides)?;
        let z = self.correlated_z(n, seed)?;
        self.draws_from_z(&z, mean_overrides)
    }

    fn check_overrides(&self, mean_overrides: &[(String, f64)]) -> Result<(), DcfError> {
        let mut unknown: Vec<&str> = mean_overrides
            .iter()
            .map(|(k, _)| k.as_str())
            .filter(|k| !self.drivers.iter().any(|d| d.name == *k))
            .collect();
        if !unknown.is_empty() {
            unknown.sort();
            return err(format!("mean_overrides for unknown drivers: {unknown:?}."));
        }
        Ok(())
    }

    /// The driver draws for given z rows (from `correlated_z`) and mean overrides.
    pub fn draws_from_z(&self, z: &[Vec<f64>], mean_overrides: &[(String, f64)]) -> Result<Draws, DcfError> {
        self.check_overrides(mean_overrides)?;
        let overrides: HashMap<&str, f64> = mean_overrides.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let mut draws: Draws = HashMap::new();
        for (i, d) in self.stochastic_drivers().iter().enumerate() {
            let mean = overrides.get(d.name.as_str()).copied().unwrap_or(d.mean);
            let mut x: Vec<f64> = z[i].iter().map(|zv| mean + d.std * zv).collect();
            if d.lo.is_some() || d.hi.is_some() {
                for v in x.iter_mut() {
                    if let Some(lo) = d.lo {
                        *v = v.max(lo);
                    }
                    if let Some(hi) = d.hi {
                        *v = v.min(hi);
                    }
                }
            }
            draws.insert(d.name.clone(), Draw::PerPath(x));
        }
        for d in &self.drivers {
            if !d.stochastic {
                let o = overrides.get(d.name.as_str()).copied().unwrap_or(d.mean);
                if let Some(s) = &d.schedule {
                    let shift = o - d.mean;
                    draws.insert(d.name.clone(), Draw::PerYear(s.iter().map(|v| v + shift).collect()));
                } else {
                    draws.insert(d.name.clone(), Draw::Scalar(o));
                }
            }
        }
        Ok(draws)
    }
}

// ---------------------------------------------------------------------------
// The vetted formulas
// ---------------------------------------------------------------------------

/// FCFF margin = core + drift - sbc - recurrence * ma_ratio (this order).
#[inline]
pub fn fcff_margin(core: f64, drift: f64, sbc: f64, ma_ratio: f64, ma_rec: f64) -> f64 {
    core + drift - sbc - ma_rec * ma_ratio
}

/// WACC with the cost of debt after tax derived from the risk-free draw.
#[inline]
pub fn wacc_capm(rf: f64, beta: f64, erp: f64, kd_spread: f64, tax: f64, ew: f64) -> f64 {
    let kd_after_tax = (rf + kd_spread) * (1.0 - tax);
    ew * (rf + beta * erp) + (1.0 - ew) * kd_after_tax
}

#[inline]
pub fn effective_terminal_growth(tg: f64, wacc: f64) -> f64 {
    tg.min(wacc - TG_WACC_GAP)
}

/// Present value of the Gordon terminal value at the end of year `years`.
#[inline]
pub fn terminal_value(fcff_final: f64, tg: f64, wacc: f64, years: usize) -> f64 {
    let tg_eff = effective_terminal_growth(tg, wacc);
    let tv = fcff_final * (1.0 + tg_eff) / (wacc - tg_eff);
    tv / (1.0 + wacc).powf(years as f64)
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// P50 per-share change when the mean of one driver moves by -1 / +1 std (same seed).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DriverAttribution {
    pub driver: String,
    pub p50_minus_1sd: f64,
    pub p50_plus_1sd: f64,
}

/// Result of a valuation over a set of draws.
#[derive(Debug, Clone)]
pub struct Valuation {
    pub n: usize,
    pub years: usize,
    pub per_share: Vec<f64>,
    /// year-major: `fcff[t * n + j]`
    pub fcff: Vec<f64>,
    pub pv_fcff: Vec<f64>,
    pub pv_terminal: Vec<f64>,
    pub wacc: Vec<f64>,
    pub tg_pinned_fraction: f64,
}

#[derive(Debug, Clone)]
pub struct DriverDcfResult {
    pub per_share_p10: f64,
    pub per_share_p50: f64,
    pub per_share_p90: f64,
    pub tv_ev_share_at_means: f64,
    pub tg_pinned_fraction: f64,
    pub value_at_means: f64,
    pub attribution: Vec<DriverAttribution>,
    pub per_share_paths: Vec<f64>,
    pub valuation: Valuation,
    pub driver_draws: Vec<(String, Vec<f64>)>,
    pub seed: u64,
}

impl DriverDcfResult {
    /// Quantiles at 10, 25, 50, 75, 90.
    pub fn quantiles(&self) -> [f64; 5] {
        let s = sorted_copy(&self.per_share_paths);
        [10.0, 25.0, 50.0, 75.0, 90.0].map(|q| percentile(&s, q))
    }

    pub fn share_below(&self, price: f64) -> f64 {
        share_below(&self.per_share_paths, price)
    }
}

#[derive(Debug, Clone)]
pub struct DriverDcf {
    pub model: DriverModel,
    pub segments: Vec<SegmentSpec>,
    pub net_debt: f64,
    pub shares: f64,
    pub projection_years: usize,
}

impl DriverDcf {
    pub fn new(
        model: DriverModel,
        segments: Vec<SegmentSpec>,
        net_debt: f64,
        shares: f64,
        projection_years: usize,
    ) -> Result<DriverDcf, DcfError> {
        if segments.is_empty() {
            return err("At least one revenue segment is required.");
        }
        if shares <= 0.0 {
            return err(format!("shares must be positive, got {shares}."));
        }
        if projection_years == 0 {
            return err("projection_years must be positive, got 0.");
        }
        let names: Vec<&str> = segments.iter().map(|s| s.name.as_str()).collect();
        let mut uniq = names.clone();
        uniq.sort();
        uniq.dedup();
        if uniq.len() != names.len() {
            return err(format!("Duplicate segment names: {names:?}."));
        }
        for s in &segments {
            if s.base_revenue <= 0.0 {
                return err(format!("Segment {:?}: base_revenue must be positive, got {}.", s.name, s.base_revenue));
            }
        }
        let mut missing: Vec<String> = Vec::new();
        for need in ROLE_DRIVERS.iter().map(|s| s.to_string()).chain(segments.iter().map(|s| s.growth_driver.clone())) {
            if !model.drivers.iter().any(|d| d.name == need) && !missing.contains(&need) {
                missing.push(need);
            }
        }
        if !missing.is_empty() {
            missing.sort();
            return err(format!(
                "DriverModel is missing required driver(s): {missing:?}. Roles {ROLE_DRIVERS:?} plus each segment's growth driver must exist."
            ));
        }
        // A per-year value only makes sense for the margin drivers: the CAPM and terminal
        // drivers give one WACC per path, and a growth draw is one rate for all years.
        const SCHEDULE_ROLES: [&str; 5] = ["core_margin", "margin_mix_drift", "sbc_ratio", "ma_ratio", "ma_recurrence"];
        for d in &model.drivers {
            if let Some(s) = &d.schedule {
                if !SCHEDULE_ROLES.contains(&d.name.as_str()) {
                    return err(format!(
                        "Driver {:?}: a schedule is allowed for {SCHEDULE_ROLES:?} only (the Rust engine has one WACC and one growth rate for each path).",
                        d.name
                    ));
                }
                if s.len() != projection_years {
                    return err(format!(
                        "Driver {:?}: schedule has {} entries but projection_years={projection_years} - a schedule must declare exactly one value per projection year.",
                        d.name,
                        s.len()
                    ));
                }
            }
        }
        Ok(DriverDcf { model, segments, net_debt, shares, projection_years })
    }

    /// The valuation over `n` paths of `draws`. `full` keeps FCFF and its present value for
    /// each year and path; without it only the per-share values and the WACC are kept.
    /// Paths are independent, so they run on several threads without any change of a result.
    pub fn valuation_with(&self, draws: &Draws, n: usize, full: bool) -> Result<Valuation, DcfError> {
        let years = self.projection_years;
        let get = |name: &str| -> &Draw { &draws[name] };
        let growth: Vec<&Draw> = self.segments.iter().map(|s| get(&s.growth_driver)).collect();
        let (core, drift, sbc, ma_ratio, ma_rec) = (
            get("core_margin"),
            get("margin_mix_drift"),
            get("sbc_ratio"),
            get("ma_ratio"),
            get("ma_recurrence"),
        );
        let (rf, beta, erp, kd, tax, ew, tg) = (
            get("rf"),
            get("beta"),
            get("erp"),
            get("kd_spread"),
            get("tax_rate"),
            get("equity_weight"),
            get("terminal_growth"),
        );
        struct PathOut {
            per_share: f64,
            wacc: f64,
            pv_term: f64,
            pinned: bool,
            fcff: Vec<f64>,
            pv: Vec<f64>,
        }
        let one = |j: usize| -> Result<PathOut, DcfError> {
            let w = wacc_capm(rf.get(0, j), beta.get(0, j), erp.get(0, j), kd.get(0, j), tax.get(0, j), ew.get(0, j));
            if w <= 0.0 {
                return err("WACC is non-positive on at least one path - bound the CAPM drivers (dist='truncnorm') so discounting stays meaningful.");
            }
            let tgj = tg.get(0, j);
            let mut ev_pv = 0.0;
            let mut last = 0.0;
            let (mut fcff, mut pv_all) = if full { (Vec::with_capacity(years), Vec::with_capacity(years)) } else { (vec![], vec![]) };
            // Powers by repeated multiplication: (1+g)^t = (1+g)^(t-1) * (1+g). Python used
            // `**` on arrays; the two ways differ by about 1e-16 relative (see HIDDEN-ASSUMPTIONS).
            let mut grow: Vec<(f64, f64, f64)> = self
                .segments
                .iter()
                .zip(&growth)
                .map(|(s, g)| {
                    let base_g = 1.0 + g.get(0, j);
                    match s.year1_growth {
                        Some(y1) => (s.base_revenue * (1.0 + y1), base_g, 1.0),
                        None => (s.base_revenue, base_g, base_g),
                    }
                })
                .collect();
            let base_w = 1.0 + w;
            let mut disc = base_w;
            for t in 0..years {
                let mut total = 0.0;
                for (scale, base_g, pw) in grow.iter_mut() {
                    total += *scale * *pw;
                    *pw *= *base_g;
                }
                let margin = fcff_margin(core.get(t, j), drift.get(t, j), sbc.get(t, j), ma_ratio.get(t, j), ma_rec.get(t, j));
                let f = total * margin;
                let pv = f / disc;
                disc *= base_w;
                if full {
                    fcff.push(f);
                    pv_all.push(pv);
                }
                ev_pv = if t == 0 { pv } else { ev_pv + pv };
                last = f;
            }
            let tv = terminal_value(last, tgj, w, years);
            Ok(PathOut {
                per_share: ((ev_pv + tv) - self.net_debt) / self.shares,
                wacc: w,
                pv_term: tv,
                pinned: tgj > w - TG_WACC_GAP,
                fcff,
                pv: pv_all,
            })
        };
        let outs: Vec<PathOut> = if n >= 512 {
            use rayon::prelude::*;
            (0..n).into_par_iter().map(one).collect::<Result<_, _>>()?
        } else {
            (0..n).map(one).collect::<Result<_, _>>()?
        };
        let mut v = Valuation {
            n,
            years,
            per_share: Vec::with_capacity(n),
            fcff: if full { vec![0.0; years * n] } else { vec![] },
            pv_fcff: if full { vec![0.0; years * n] } else { vec![] },
            pv_terminal: Vec::with_capacity(n),
            wacc: Vec::with_capacity(n),
            tg_pinned_fraction: 0.0,
        };
        let mut pinned = 0usize;
        for (j, o) in outs.iter().enumerate() {
            v.per_share.push(o.per_share);
            v.pv_terminal.push(o.pv_term);
            v.wacc.push(o.wacc);
            pinned += o.pinned as usize;
            if full {
                for t in 0..years {
                    v.fcff[t * n + j] = o.fcff[t];
                    v.pv_fcff[t * n + j] = o.pv[t];
                }
            }
        }
        v.tg_pinned_fraction = pinned as f64 / n as f64;
        Ok(v)
    }

    /// The valuation with all detail (`valuation_with(.., true)`).
    pub fn valuation(&self, draws: &Draws, n: usize) -> Result<Valuation, DcfError> {
        self.valuation_with(draws, n, true)
    }

    /// Every driver at its mean: one deterministic path. Scheduled drivers keep the schedule.
    pub fn mean_draws(&self) -> Draws {
        self.model
            .drivers
            .iter()
            .map(|d| {
                let v = match &d.schedule {
                    Some(s) => Draw::PerYear(s.clone()),
                    None => Draw::Scalar(d.mean),
                };
                (d.name.clone(), v)
            })
            .collect()
    }

    /// Value per share with every driver at its mean.
    pub fn value_at_means(&self) -> Result<f64, DcfError> {
        Ok(self.valuation(&self.mean_draws(), 1)?.per_share[0])
    }

    /// Sample and value one set of draws; no attribution, only per-share values and WACC.
    pub fn run_paths(&self, n: usize, seed: u64, overrides: &[(String, f64)]) -> Result<Valuation, DcfError> {
        let draws = self.model.sample(n, seed, overrides)?;
        self.valuation_with(&draws, n, false)
    }

    fn p50(v: &[f64]) -> f64 {
        percentile(&sorted_copy(v), 50.0)
    }

    /// The simulation and the +-1 std attribution (`DriverDCF.run` in Python).
    ///
    /// The shifted runs use the same z rows as the base run (as in Python, where the same seed
    /// gives them again), so the rows are made once, and the runs go on several threads.
    pub fn run(&self, n: usize, seed: Option<u64>) -> Result<DriverDcfResult, DcfError> {
        let seed = seed.unwrap_or_else(clock_seed);
        let z = self.model.correlated_z(n, seed)?;
        let draws = self.model.draws_from_z(&z, &[])?;
        let base = self.valuation_with(&draws, n, true)?;
        let sorted = sorted_copy(&base.per_share);
        let base_p50 = percentile(&sorted, 50.0);

        let stoch = self.model.stochastic_drivers();
        let jobs: Vec<(usize, f64)> = (0..stoch.len()).flat_map(|i| [(i, -1.0), (i, 1.0)]).collect();
        let one = |&(i, dir): &(usize, f64)| -> Result<f64, DcfError> {
            let d = stoch[i];
            let shifted = self.model.draws_from_z(&z, &[(d.name.clone(), d.mean + dir * d.std)])?;
            Ok(Self::p50(&self.valuation_with(&shifted, n, false)?.per_share) - base_p50)
        };
        let deltas: Vec<f64> = {
            use rayon::prelude::*;
            jobs.par_iter().map(one).collect::<Result<_, _>>()?
        };
        let attribution = stoch
            .iter()
            .enumerate()
            .map(|(i, d)| DriverAttribution {
                driver: d.name.clone(),
                p50_minus_1sd: deltas[2 * i],
                p50_plus_1sd: deltas[2 * i + 1],
            })
            .collect::<Vec<_>>();

        let at_means = self.valuation(&self.mean_draws(), 1)?;
        let mean_ev = at_means.pv_fcff.iter().sum::<f64>() + at_means.pv_terminal[0];
        let tv_ev = at_means.pv_terminal[0] / mean_ev;
        let driver_draws = self
            .model
            .stochastic_drivers()
            .iter()
            .map(|d| match &draws[&d.name] {
                Draw::PerPath(v) => (d.name.clone(), v.clone()),
                _ => unreachable!("stochastic drivers are per-path"),
            })
            .collect();
        Ok(DriverDcfResult {
            per_share_p10: percentile(&sorted, 10.0),
            per_share_p50: base_p50,
            per_share_p90: percentile(&sorted, 90.0),
            tv_ev_share_at_means: tv_ev,
            tg_pinned_fraction: base.tg_pinned_fraction,
            value_at_means: at_means.per_share[0],
            attribution,
            per_share_paths: base.per_share.clone(),
            valuation: base,
            driver_draws,
            seed,
        })
    }
}
