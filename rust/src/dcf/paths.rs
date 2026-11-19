//! The path engine: each element of the cash flows is a process for each year of a path.
//!
//! The old engine (`driver.rs`) draws each driver one time for each path. This engine draws
//! the parameters one time for each path (uncertainty of a parameter: the long-run growth, the
//! target margin, the half-life, beta, the equity premium) and then moves the processes year by
//! year with shocks (revenue growth, margin, SBC ratio, other costs, sales-to-capital ratio, the
//! short rate). A path is one reproducible stream: the stream of path `p` depends on `(seed, p)`
//! only, so a run gives the same numbers with any number of threads (`ratemodels::rng`).
//!
//! For each path and year `t = 1..N` (annual steps; `dt` is a field, the formulas use it):
//!
//! 1. growth of each segment: process (see `process.rs`), pinned in year 1 where the segment has a pin;
//!    `revenue(t) = revenue(t-1) * (1 + g(t))`;
//! 2. margin `m`, SBC ratio `s`, other costs `c`: processes;
//! 3. mode `margin` (option A): `FCFF = revenue * (m - s - c)`;
//!    mode `sales_to_capital` (option B): `EBIT = revenue * (m - s - c)`, tax on the positive
//!    EBIT after the loss carry-forward, `FCFF = EBIT - tax - max(revenue(t) - revenue(t-1), 0) / s2c`;
//! 4. cost of capital of year `t`: `k = w (rf + beta erp) + (1 - w) (rf + spread) (1 - tax)`, with
//!    `rf` from the rate factor at the start of year `t` (`pathrate.rs`); the discount factor is the
//!    product of `1 / (1 + k)` over the years;
//! 5. failure (optional): a yearly hazard, with a recovery value;
//! 6. terminal value at the end of year N with the rate of that time (Gordon; see `TerminalSpec`).
//!
//! The correlated random numbers: the parameters have a correlation matrix (`[[corr]]`), the yearly
//! shocks have another (`[[shock_corr]]`). Each is made positive definite or the model is refused.

use super::pathrate::{RateFactor, RateKind, RfMode};
use super::process::*;
use super::stats::{cholesky, err, min_eigenvalue, percentile, share_below, sorted_copy};
use super::{DcfError, driver::TG_WACC_GAP};
use crate::ratemodels::curve::ZeroCurve;
use crate::ratemodels::rng::Rng;
use rayon::prelude::*;
use std::collections::HashMap;

/// Fields of the detail of one path-year.
pub const NF: usize = 8;
pub const F_REV: usize = 0;
pub const F_GROWTH: usize = 1;
pub const F_MARGIN: usize = 2;
pub const F_SBC: usize = 3;
pub const F_REINV: usize = 4;
pub const F_FCFF: usize = 5;
pub const F_DISC: usize = 6;
pub const F_RATE: usize = 7;

/// The flags of a terminal growth that a cap changed.
pub const CAP_PROCESS: u8 = 1;
pub const CAP_RF: u8 = 2;
pub const CAP_GAP: u8 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDef {
    pub name: String,
    pub mean: f64,
    pub std: f64,
    pub lo: Option<f64>,
    pub hi: Option<f64>,
    /// Index into `PathEngine::groups`.
    pub group: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Ou,
    Ramp(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum StartC {
    Tied,
    Param(usize),
}

#[derive(Debug, Clone)]
struct ProcC {
    name: String,
    start: StartC,
    target: usize,
    half: Option<usize>,
    shape: Shape,
    sigma: f64,
    sigma_level: f64,
    rho: Option<f64>,
    lo: f64,
    hi: f64,
    drift: Vec<f64>,
    chan: Option<usize>,
}

#[derive(Debug, Clone)]
struct SegC {
    base: f64,
    pin: Option<f64>,
    proc: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RoicC {
    None,
    Wacc,
    Param(usize),
}

#[derive(Debug, Clone, Copy)]
struct FailC {
    hazard: usize,
    multiple: f64,
    expected: bool,
}

/// Changes of one run that keep the random numbers: a shift of the mean of a parameter, and
/// groups that are frozen (their parameters take the mean, their shocks are zero).
#[derive(Debug, Clone, Default)]
pub struct Mods {
    pub shift: Vec<(usize, f64)>,
    pub frozen: Vec<usize>,
    /// Only these groups are active (all other groups are frozen). `None`: no such rule.
    pub only: Option<usize>,
    /// All yearly shocks are zero (the parameters stay random).
    pub no_shocks: bool,
}

/// The result of a simulation.
#[derive(Debug, Clone)]
pub struct Sim {
    pub n: usize,
    pub years: usize,
    pub per_share: Vec<f64>,
    pub pv_explicit: Vec<f64>,
    pub pv_terminal: Vec<f64>,
    pub flags: Vec<u8>,
    pub k_terminal: Vec<f64>,
    pub failed: usize,
    /// `detail[(j * years + t) * NF + f]`; empty when the run did not ask for it.
    pub detail: Vec<f64>,
}

impl Sim {
    pub fn quantiles(&self) -> [f64; 5] {
        let s = sorted_copy(&self.per_share);
        [10.0, 25.0, 50.0, 75.0, 90.0].map(|q| percentile(&s, q))
    }
    pub fn p50(&self) -> f64 {
        percentile(&sorted_copy(&self.per_share), 50.0)
    }
    pub fn share_below(&self, price: f64) -> f64 {
        share_below(&self.per_share, price)
    }
    /// The field `f` of year `t` (0-based) of path `j`.
    pub fn at(&self, j: usize, t: usize, f: usize) -> f64 {
        self.detail[(j * self.years + t) * NF + f]
    }
    /// Values of the field `f` in year `t` over all paths.
    pub fn column(&self, t: usize, f: usize) -> Vec<f64> {
        (0..self.n).map(|j| self.at(j, t, f)).collect()
    }
    pub fn mean_of(&self, t: usize, f: usize) -> f64 {
        (0..self.n).map(|j| self.at(j, t, f)).sum::<f64>() / self.n as f64
    }
    /// Share of the terminal value in the enterprise value, `E[PV TV] / E[EV]`.
    pub fn tv_share(&self) -> f64 {
        let tv: f64 = self.pv_terminal.iter().sum();
        let ex: f64 = self.pv_explicit.iter().sum();
        tv / (tv + ex)
    }
    /// Share of the paths where a cap changed the terminal growth (`CAP_*` bit).
    pub fn capped_share(&self, bit: u8) -> f64 {
        self.flags.iter().filter(|f| **f & bit != 0).count() as f64 / self.n as f64
    }
}

struct PathOut {
    per_share: f64,
    pv_explicit: f64,
    pv_terminal: f64,
    flags: u8,
    k_terminal: f64,
    failed: bool,
    detail: Vec<f64>,
}

struct Scratch {
    vals: Vec<f64>,
    zr: Vec<f64>,
    zc: Vec<f64>,
    eps: Vec<f64>,
    c: Vec<f64>,
    st: Vec<f64>,
    u: Vec<f64>,
    tgt: Vec<f64>,
    phi: Vec<f64>,
    rho: Vec<f64>,
    x: Vec<f64>,
    rev: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct PathEngine {
    pub years: usize,
    pub dt: f64,
    pub net_debt: f64,
    pub shares: f64,
    pub params: Vec<ParamDef>,
    pub groups: Vec<String>,
    stoch: Vec<usize>,
    chol_p: Vec<Vec<f64>>,
    procs: Vec<ProcC>,
    segs: Vec<SegC>,
    p_margin: usize,
    p_sbc: usize,
    p_charge: Option<usize>,
    p_s2c: Option<usize>,
    chan_names: Vec<String>,
    chan_group: Vec<usize>,
    chol_c: Vec<Vec<f64>>,
    rate: Option<RateFactor>,
    rate_chan: Option<usize>,
    rf_param: Option<usize>,
    shift_param: Option<usize>,
    beta: usize,
    erp: usize,
    kd_spread: f64,
    equity_weight: f64,
    tax: f64,
    mode_b: bool,
    opening_loss: f64,
    loss_limit: f64,
    term_g: usize,
    cap_proc: Option<usize>,
    rf_margin: Option<f64>,
    roic: RoicC,
    failure: Option<FailC>,
}

fn check_param(name: &str, p: &ParamSpec) -> Result<(), DcfError> {
    if let ParamSpec::Draw(d) = p {
        if !(d.mean.is_finite() && d.std.is_finite()) {
            return err(format!("{name}: mean and std must be finite."));
        }
        if d.std < 0.0 {
            return err(format!("{name}: std must be 0 or more, got {}.", d.std));
        }
        if let (Some(lo), Some(hi)) = (d.lo, d.hi) {
            if lo >= hi {
                return err(format!("{name}: lo={lo} must be below hi={hi}."));
            }
        }
        if d.lo.is_some_and(|lo| d.mean < lo) || d.hi.is_some_and(|hi| d.mean > hi) {
            return err(format!("{name}: the mean {} is outside [lo, hi].", d.mean));
        }
        if d.std == 0.0 && (d.lo.is_some() || d.hi.is_some()) {
            return err(format!("{name}: bounds without std: remove them or draw the number."));
        }
    } else if let ParamSpec::Fixed(v) = p {
        // only a half-life can be infinite (no reversion)
        if v.is_nan() || !v.is_finite() && !name.ends_with(".half_life") {
            return err(format!("{name}: the number must be finite."));
        }
    }
    Ok(())
}

fn corr_matrix(entries: &[CorrSpec], names: &[String], what: &str) -> Result<Vec<Vec<f64>>, DcfError> {
    let k = names.len();
    let mut mat = vec![vec![0.0; k]; k];
    for (i, row) in mat.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    let mut seen: HashMap<(usize, usize), f64> = HashMap::new();
    for e in entries {
        let find = |n: &str| names.iter().position(|x| x == n);
        let (Some(i), Some(j)) = (find(&e.a), find(&e.b)) else {
            return err(format!(
                "{what} ({:?}, {:?}): each name must be one of {names:?}. Only drawn parameters (std > 0) and shock sources correlate.",
                e.a, e.b
            ));
        };
        if i == j {
            return err(format!("{what} ({:?}, {:?}): a self-correlation is 1 by definition; remove the entry.", e.a, e.b));
        }
        if !(-1.0..=1.0).contains(&e.rho) {
            return err(format!("{what} ({:?}, {:?}) = {} is outside [-1, 1].", e.a, e.b, e.rho));
        }
        let key = (i.min(j), i.max(j));
        if let Some(prev) = seen.get(&key) {
            if *prev != e.rho {
                return err(format!("{what}: conflicting values for the pair ({:?}, {:?}): {prev} and {}.", e.a, e.b, e.rho));
            }
        }
        seen.insert(key, e.rho);
        mat[i][j] = e.rho;
        mat[j][i] = e.rho;
    }
    if k > 0 && cholesky(&mat).is_err() {
        return err(format!(
            "{what}: the correlation matrix is not positive definite (smallest eigenvalue {:.3e}). It is not repaired: change the entries.",
            min_eigenvalue(&mat)
        ));
    }
    Ok(mat)
}

impl PathEngine {
    pub fn new(spec: &PathsSpec) -> Result<PathEngine, DcfError> {
        if spec.projection_years == 0 {
            return err("projection_years must be positive, got 0.");
        }
        if !(spec.shares.is_finite() && spec.shares > 0.0) {
            return err(format!("shares must be positive, got {}.", spec.shares));
        }
        if spec.segments.is_empty() {
            return err("At least one revenue segment is required.");
        }
        let mut names: Vec<&str> = spec.segments.iter().map(|s| s.name.as_str()).collect();
        names.sort();
        let len = names.len();
        names.dedup();
        if names.len() != len {
            return err("Duplicate segment names.");
        }
        for s in &spec.segments {
            if !(s.base_revenue.is_finite() && s.base_revenue > 0.0) {
                return err(format!("Segment {:?}: base_revenue must be positive, got {}.", s.name, s.base_revenue));
            }
            if s.year1_growth.is_some_and(|g| !g.is_finite() || g <= -1.0) {
                return err(format!("Segment {:?}: year1_growth must be above -1.", s.name));
            }
        }
        let mode_b = match spec.reinvestment.mode.as_str() {
            "margin" => false,
            "sales_to_capital" => true,
            m => return err(format!("reinvestment.mode must be \"margin\" or \"sales_to_capital\", got {m:?}.")),
        };

        let mut groups: Vec<String> = Vec::new();
        let group_of = |g: &str, groups: &mut Vec<String>| -> usize {
            match groups.iter().position(|x| x == g) {
                Some(i) => i,
                None => {
                    groups.push(g.to_string());
                    groups.len() - 1
                }
            }
        };
        let mut params: Vec<ParamDef> = Vec::new();
        let add = |name: String, p: &ParamSpec, group: usize, params: &mut Vec<ParamDef>| -> Result<usize, DcfError> {
            check_param(&name, p)?;
            let (lo, hi) = p.bounds();
            params.push(ParamDef { name, mean: p.mean(), std: p.std(), lo, hi, group });
            Ok(params.len() - 1)
        };

        // processes
        let mut procs: Vec<ProcC> = Vec::new();
        let mut chan_names: Vec<String> = Vec::new();
        let mut chan_group: Vec<usize> = Vec::new();
        let (mut p_margin, mut p_sbc, mut p_charge, mut p_s2c) = (None, None, None, None);
        for (pname, p) in &spec.process {
            let role = p.role.as_str();
            if !["growth", "margin", "sbc", "charge", "s2c"].contains(&role) {
                return err(format!("process.{pname}: role must be growth, margin, sbc, charge or s2c, got {role:?}."));
            }
            let g = group_of(role, &mut groups);
            let idx = procs.len();
            let slot = match role {
                "margin" => Some(&mut p_margin),
                "sbc" => Some(&mut p_sbc),
                "charge" => Some(&mut p_charge),
                "s2c" => Some(&mut p_s2c),
                _ => None,
            };
            if let Some(slot) = slot {
                if slot.is_some() {
                    return err(format!("process.{pname}: there is more than one process with the role {role:?}."));
                }
                *slot = Some(idx);
            }
            let start = match &p.start {
                StartSpec::Tied(s) if s == "target" => StartC::Tied,
                StartSpec::Tied(s) => return err(format!("process.{pname}.start: the text must be \"target\", got {s:?}.")),
                StartSpec::Value(v) => StartC::Param(add(format!("process.{pname}.start"), v, g, &mut params)?),
            };
            let target = add(format!("process.{pname}.target"), &p.target, g, &mut params)?;
            let (shape, half) = match p.shape.as_str() {
                "ou" => {
                    if p.ramp_years.is_some() {
                        return err(format!("process.{pname}: ramp_years is for the shape \"ramp\"."));
                    }
                    let h = p.half_life.as_ref().ok_or_else(|| DcfError(format!("process.{pname}: the shape \"ou\" needs half_life (years; inf for no reversion).")))?;
                    let ok = match h {
                        ParamSpec::Fixed(v) => *v > 0.0,
                        ParamSpec::Draw(d) => d.lo.is_some_and(|lo| lo > 0.0) || d.std == 0.0 && d.mean > 0.0,
                    };
                    if !ok {
                        return err(format!("process.{pname}.half_life must be above 0 (a drawn half-life needs lo > 0)."));
                    }
                    (Shape::Ou, Some(add(format!("process.{pname}.half_life"), h, g, &mut params)?))
                }
                "ramp" => {
                    if p.half_life.is_some() {
                        return err(format!("process.{pname}: half_life is for the shape \"ou\"."));
                    }
                    let r = p.ramp_years.ok_or_else(|| DcfError(format!("process.{pname}: the shape \"ramp\" needs ramp_years.")))?;
                    if r < 2 {
                        return err(format!("process.{pname}: ramp_years must be at least 2."));
                    }
                    (Shape::Ramp(r), None)
                }
                s => return err(format!("process.{pname}: shape must be \"ou\" or \"ramp\", got {s:?}.")),
            };
            if !(p.sigma.is_finite() && p.sigma >= 0.0) {
                return err(format!("process.{pname}.sigma must be 0 or more, got {}.", p.sigma));
            }
            if !(p.sigma_level.is_finite() && p.sigma_level >= 0.0) {
                return err(format!("process.{pname}.sigma_level must be 0 or more."));
            }
            if p.persistence.is_some_and(|r| !(-1.0..=1.0).contains(&r)) {
                return err(format!("process.{pname}.persistence must be in [-1, 1]."));
            }
            let (lo, hi) = (p.lo.unwrap_or(f64::NEG_INFINITY), p.hi.unwrap_or(f64::INFINITY));
            if lo >= hi {
                return err(format!("process.{pname}: lo={lo} must be below hi={hi}."));
            }
            let start_positive = match &p.start {
                StartSpec::Tied(_) => true,
                StartSpec::Value(v) => v.std() == 0.0 && v.mean() > 0.0,
            };
            let fixed_positive = p.sigma == 0.0 && p.target.std() == 0.0 && p.target.mean() > 0.0 && start_positive;
            if role == "s2c" && !(lo > 0.0 || fixed_positive) {
                return err(format!("process.{pname}: the sales-to-capital ratio needs lo > 0 (or a fixed positive start and target without shocks)."));
            }
            let drift = match &p.drift {
                Some(r) => {
                    if r.ramp_years < 2 {
                        return err(format!("process.{pname}.drift: ramp_years must be at least 2."));
                    }
                    r.schedule(spec.projection_years)
                }
                None => vec![0.0; spec.projection_years],
            };
            let chan = if p.sigma > 0.0 {
                chan_names.push(pname.clone());
                chan_group.push(g);
                Some(chan_names.len() - 1)
            } else {
                None
            };
            procs.push(ProcC { name: pname.clone(), start, target, half, shape, sigma: p.sigma, sigma_level: p.sigma_level, rho: p.persistence, lo, hi, drift, chan });
        }
        let (Some(p_margin), Some(p_sbc)) = (p_margin, p_sbc) else {
            return err("The model needs one process with the role \"margin\" and one with the role \"sbc\".");
        };
        if mode_b && p_s2c.is_none() {
            return err("reinvestment.mode = \"sales_to_capital\" needs one process with the role \"s2c\".");
        }
        if !mode_b && p_s2c.is_some() {
            return err("A process with the role \"s2c\" needs reinvestment.mode = \"sales_to_capital\".");
        }
        let mut segs = Vec::new();
        for s in &spec.segments {
            let pi = procs
                .iter()
                .position(|p| p.name == s.process && spec.process[&p.name].role == "growth")
                .ok_or_else(|| DcfError(format!("Segment {:?}: the process {:?} does not exist or has not the role \"growth\".", s.name, s.process)))?;
            segs.push(SegC { base: s.base_revenue, pin: s.year1_growth, proc: pi });
        }
        for (pi, p) in procs.iter().enumerate() {
            if spec.process[&p.name].role == "growth" && !segs.iter().any(|s| s.proc == pi) {
                return err(format!("process.{}: no segment uses this growth process.", p.name));
            }
        }

        // discount
        let d = &spec.discount;
        if !(0.0..=1.0).contains(&d.equity_weight) {
            return err(format!("discount.equity_weight must be in [0, 1], got {}.", d.equity_weight));
        }
        if !(0.0..1.0).contains(&spec.tax.rate) {
            return err(format!("tax.rate must be in [0, 1), got {}.", spec.tax.rate));
        }
        if !(spec.tax.loss_limit_share > 0.0 && spec.tax.loss_limit_share <= 1.0) || spec.tax.opening_loss < 0.0 {
            return err("tax.loss_limit_share must be in (0, 1] and tax.opening_loss must be 0 or more.");
        }
        let gd = group_of("discount", &mut groups);
        let beta = add("discount.beta".into(), &d.beta, gd, &mut params)?;
        let erp = add("discount.erp".into(), &d.erp, gd, &mut params)?;

        // rates
        let r = &spec.rates;
        let gr = group_of("rates", &mut groups);
        let years = spec.projection_years;
        let (mut rate, mut rf_param, mut rate_chan) = (None, None, None);
        let rf_mode = match r.mode.as_str() {
            "long" => RfMode::Long,
            "long_flat" => RfMode::LongFlat,
            "short" => RfMode::Short,
            m => return err(format!("rates.mode must be \"long\", \"long_flat\" or \"short\", got {m:?}.")),
        };
        let need = |v: Option<f64>, n: &str| v.ok_or_else(|| DcfError(format!("rates.{n} is needed for the model {:?}.", r.model)));
        let kind = match r.model.as_str() {
            "fixed" => {
                let rf = r.rf.as_ref().ok_or_else(|| DcfError("rates.model = \"fixed\" needs rates.rf.".into()))?;
                rf_param = Some(add("rates.rf".into(), rf, gr, &mut params)?);
                None
            }
            "hullwhite" => {
                let c = r.curve.as_ref().ok_or_else(|| DcfError("rates.model = \"hullwhite\" needs rates.curve (the six numbers of one GSW day).".into()))?;
                if !(c.tau1 > 0.0 && c.tau2 > 0.0) {
                    return err("rates.curve: tau1 and tau2 must be positive.");
                }
                let curve = ZeroCurve { beta0: c.beta0 / 100.0, beta1: c.beta1 / 100.0, beta2: c.beta2 / 100.0, beta3: c.beta3 / 100.0, tau1: c.tau1, tau2: c.tau2 };
                Some(RateKind::HullWhite { a: need(r.a, "a")?, sigma: need(r.sigma, "sigma")?, curve })
            }
            m @ ("vasicek" | "cir") => {
                let (kappa, theta, sigma) = (need(r.kappa, "kappa")?, need(r.theta, "theta")?, need(r.sigma, "sigma")?);
                let r0 = r.r0.unwrap_or(theta);
                Some(if m == "vasicek" { RateKind::Vasicek { kappa, theta, sigma, r0 } } else { RateKind::Cir { kappa, theta, sigma, r0 } })
            }
            m => return err(format!("rates.model must be fixed, hullwhite, vasicek or cir, got {m:?}.")),
        };
        if let Some(kind) = kind {
            if r.rf.is_some() {
                return err("rates.rf is for the model \"fixed\" only.");
            }
            let f = RateFactor::new(kind, 1.0, years, rf_mode, r.tenor, r.spread).map_err(DcfError)?;
            rate = Some(f);
            chan_names.push("rate".into());
            chan_group.push(gr);
            rate_chan = Some(chan_names.len() - 1);
        }
        let shift_param = match &r.shift {
            Some(s) => Some(add("rates.shift".into(), s, gr, &mut params)?),
            None => None,
        };

        // terminal
        let t = &spec.terminal;
        let gt = group_of("terminal", &mut groups);
        let term_g = add("terminal.growth".into(), &t.growth, gt, &mut params)?;
        let cap_proc = match &t.cap_by_process {
            Some(n) => Some(
                procs
                    .iter()
                    .position(|p| &p.name == n && spec.process[&p.name].role == "growth")
                    .ok_or_else(|| DcfError(format!("terminal.cap_by_process: {n:?} is not a growth process.")))?,
            ),
            None => None,
        };
        let roic = match &t.roic {
            None => RoicC::None,
            Some(StartSpec::Tied(s)) if s == "wacc" => RoicC::Wacc,
            Some(StartSpec::Tied(s)) => return err(format!("terminal.roic: the text must be \"wacc\", got {s:?}.")),
            Some(StartSpec::Value(v)) => RoicC::Param(add("terminal.roic".into(), v, gt, &mut params)?),
        };
        if mode_b && roic == RoicC::None {
            return err("reinvestment.mode = \"sales_to_capital\" needs terminal.roic (a number, a draw or \"wacc\").");
        }
        if let RoicC::Param(i) = roic {
            let ok = if params[i].std > 0.0 { params[i].lo.is_some_and(|lo| lo > 0.0) } else { params[i].mean > 0.0 };
            if !ok {
                return err("terminal.roic must be above 0 (a drawn value needs lo > 0).");
            }
        }

        // failure
        let failure = match &spec.failure {
            None => None,
            Some(f) => {
                let gf = group_of("failure", &mut groups);
                let ok = match &f.hazard {
                    ParamSpec::Fixed(h) => (0.0..1.0).contains(h),
                    ParamSpec::Draw(d) => d.std == 0.0 && (0.0..1.0).contains(&d.mean) || d.lo.is_some_and(|lo| lo >= 0.0) && d.hi.is_some_and(|hi| hi < 1.0),
                };
                if !ok {
                    return err("failure.hazard must be in [0, 1) (a drawn hazard needs lo >= 0 and hi < 1).");
                }
                if !(f.recovery_revenue_multiple.is_finite() && f.recovery_revenue_multiple >= 0.0) {
                    return err("failure.recovery_revenue_multiple must be 0 or more.");
                }
                let expected = match f.mode.as_str() {
                    "sampled" => false,
                    "expected" => true,
                    m => return err(format!("failure.mode must be \"sampled\" or \"expected\", got {m:?}.")),
                };
                Some(FailC { hazard: add("failure.hazard".into(), &f.hazard, gf, &mut params)?, multiple: f.recovery_revenue_multiple, expected })
            }
        };

        // correlations
        let stoch: Vec<usize> = (0..params.len()).filter(|i| params[*i].std > 0.0).collect();
        let stoch_names: Vec<String> = stoch.iter().map(|i| params[*i].name.clone()).collect();
        let cp = corr_matrix(&spec.corr, &stoch_names, "corr")?;
        let chol_p = if stoch.is_empty() { vec![] } else { cholesky(&cp).map_err(|_| DcfError("corr: Cholesky failed.".into()))? };
        let cc = corr_matrix(&spec.shock_corr, &chan_names, "shock_corr")?;
        let chol_c = if chan_names.is_empty() { vec![] } else { cholesky(&cc).map_err(|_| DcfError("shock_corr: Cholesky failed.".into()))? };

        Ok(PathEngine {
            years,
            dt: 1.0,
            net_debt: spec.net_debt,
            shares: spec.shares,
            params,
            groups,
            stoch,
            chol_p,
            procs,
            segs,
            p_margin,
            p_sbc,
            p_charge,
            p_s2c,
            chan_names,
            chan_group,
            chol_c,
            rate,
            rate_chan,
            rf_param,
            shift_param,
            beta,
            erp,
            kd_spread: d.kd_spread,
            equity_weight: d.equity_weight,
            tax: spec.tax.rate,
            mode_b,
            opening_loss: spec.tax.opening_loss,
            loss_limit: spec.tax.loss_limit_share,
            term_g,
            cap_proc,
            rf_margin: t.rf_margin,
            roic,
            failure,
        })
    }

    /// Names of the parameters that are drawn, in the order of the rows of the normals.
    pub fn stochastic_names(&self) -> Vec<&str> {
        self.stoch.iter().map(|i| self.params[*i].name.as_str()).collect()
    }
    pub fn stochastic_indices(&self) -> &[usize] {
        &self.stoch
    }
    pub fn channel_names(&self) -> &[String] {
        &self.chan_names
    }
    pub fn group_index(&self, name: &str) -> Option<usize> {
        self.groups.iter().position(|g| g == name)
    }

    fn scratch(&self) -> Scratch {
        let np = self.procs.len();
        Scratch {
            vals: vec![0.0; self.params.len()],
            zr: vec![0.0; self.stoch.len()],
            zc: vec![0.0; self.stoch.len()],
            eps: vec![0.0; self.chan_names.len()],
            c: vec![0.0; np],
            st: vec![0.0; np],
            u: vec![0.0; np],
            tgt: vec![0.0; np],
            phi: vec![0.0; np],
            rho: vec![0.0; np],
            x: vec![0.0; np],
            rev: vec![0.0; self.segs.len()],
        }
    }

    /// The cost of capital for the risk-free rate `rf` and the draws of the path.
    #[inline]
    fn cost_of_capital(&self, rf: f64, beta: f64, erp: f64) -> f64 {
        self.equity_weight * (rf + beta * erp) + (1.0 - self.equity_weight) * (rf + self.kd_spread) * (1.0 - self.tax)
    }

    #[allow(clippy::too_many_arguments)]
    fn eval(
        &self,
        j: usize,
        seed: u64,
        inj: Option<&[Vec<f64>]>,
        frozen: &[bool],
        shift: &[f64],
        det: bool,
        want_detail: bool,
        sc: &mut Scratch,
    ) -> Result<PathOut, DcfError> {
        let years = self.years;
        let mut rng = Rng::for_path(seed, j as u64);
        // 1. parameters
        for (i, p) in self.params.iter().enumerate() {
            sc.vals[i] = p.mean + shift[i];
        }
        if !det && !self.stoch.is_empty() {
            match inj {
                Some(z) => {
                    for (q, row) in z.iter().enumerate() {
                        sc.zc[q] = row[j];
                    }
                }
                None => {
                    for q in 0..self.stoch.len() {
                        sc.zr[q] = rng.normal();
                    }
                    for q in 0..self.stoch.len() {
                        let mut s = 0.0;
                        for l in 0..=q {
                            s += self.chol_p[q][l] * sc.zr[l];
                        }
                        sc.zc[q] = s;
                    }
                }
            }
            for (q, &i) in self.stoch.iter().enumerate() {
                let p = &self.params[i];
                if frozen[p.group] {
                    continue;
                }
                let mut x = sc.vals[i] + p.std * sc.zc[q];
                if let Some(lo) = p.lo {
                    x = x.max(lo);
                }
                if let Some(hi) = p.hi {
                    x = x.min(hi);
                }
                sc.vals[i] = x;
            }
        }
        // 2. processes
        const LN2: f64 = std::f64::consts::LN_2;
        for (p, pr) in self.procs.iter().enumerate() {
            sc.tgt[p] = sc.vals[pr.target];
            let start = match pr.start {
                StartC::Tied => sc.tgt[p],
                StartC::Param(i) => sc.vals[i],
            };
            sc.c[p] = start;
            sc.st[p] = start;
            sc.u[p] = 0.0;
            let phi = match pr.half {
                Some(i) => (-LN2 * self.dt / sc.vals[i]).exp(),
                None => 0.0,
            };
            sc.phi[p] = phi;
            sc.rho[p] = pr.rho.unwrap_or(if matches!(pr.shape, Shape::Ou) { phi } else { 0.0 });
        }
        // 3. discount parameters
        let (beta, erp) = (sc.vals[self.beta], sc.vals[self.erp]);
        let shift_rf = self.shift_param.map_or(0.0, |i| sc.vals[i]);
        let hazard = self.failure.map_or(0.0, |f| sc.vals[f.hazard]);
        let mut rate_state = self.rate.as_ref().map_or(0.0, |r| r.state0);
        for (s, seg) in self.segs.iter().enumerate() {
            sc.rev[s] = seg.base;
        }
        let mut total_prev: f64 = self.segs.iter().map(|s| s.base).sum();
        let mut disc = 1.0;
        let mut pv = 0.0;
        let mut loss = self.opening_loss;
        let mut surv = 1.0;
        let mut failed = false;
        let mut last_fcff = 0.0;
        let mut last_ebit = 0.0;
        let mut detail = if want_detail { vec![0.0; years * NF] } else { Vec::new() };
        let sampled_failure = self.failure.is_some_and(|f| !f.expected) && !det && hazard > 0.0;
        let expected_failure = self.failure.is_some_and(|f| f.expected || det) && hazard > 0.0;
        let mut t_done = years;
        for t in 1..=years {
            // shocks of year t
            if !self.chan_names.is_empty() {
                if det {
                    sc.eps.iter_mut().for_each(|e| *e = 0.0);
                } else {
                    let k = self.chan_names.len();
                    for q in 0..k {
                        let z = rng.normal();
                        // correlated: eps[q] = sum_l L[q][l] z_l, built in order (z_l for l <= q needs all earlier z)
                        sc.eps[q] = z;
                    }
                    // eps now holds the independent normals; apply the Cholesky factor in place from the last row
                    for q in (0..k).rev() {
                        let mut s = 0.0;
                        for l in 0..=q {
                            s += self.chol_c[q][l] * sc.eps[l];
                        }
                        sc.eps[q] = s;
                    }
                    for q in 0..k {
                        if frozen[self.chan_group[q]] || frozen[self.groups.len()] {
                            sc.eps[q] = 0.0;
                        }
                    }
                }
            }
            // processes
            for (p, pr) in self.procs.iter().enumerate() {
                if t >= 2 {
                    match pr.shape {
                        Shape::Ou => sc.c[p] = sc.tgt[p] + sc.phi[p] * (sc.c[p] - sc.tgt[p]),
                        Shape::Ramp(r) => {
                            let w = (t - 1).min(r - 1) as f64 / (r - 1) as f64;
                            sc.c[p] = sc.st[p] + (sc.tgt[p] - sc.st[p]) * w;
                        }
                    }
                    if let Some(ch) = pr.chan {
                        let sd = if pr.sigma_level > 0.0 { pr.sigma.max(pr.sigma_level * sc.c[p].abs()) } else { pr.sigma };
                        sc.u[p] = sc.rho[p] * sc.u[p] + sd * sc.eps[ch];
                    }
                }
                sc.x[p] = (sc.c[p] + sc.u[p] + pr.drift[t - 1]).clamp(pr.lo, pr.hi);
            }
            // revenue
            let mut total = 0.0;
            for (s, seg) in self.segs.iter().enumerate() {
                let g = if t == 1 { seg.pin.unwrap_or(sc.x[seg.proc]) } else { sc.x[seg.proc] };
                sc.rev[s] *= 1.0 + g;
                total += sc.rev[s];
            }
            let m = sc.x[self.p_margin];
            let sbc = sc.x[self.p_sbc];
            let ch = self.p_charge.map_or(0.0, |p| sc.x[p]);
            // cost of capital of year t, from the state at the start of the year
            let rf = match (&self.rate, self.rf_param) {
                (Some(r), _) => r.rf(t - 1, rate_state) + shift_rf,
                (None, Some(i)) => sc.vals[i] + shift_rf,
                (None, None) => unreachable!(),
            };
            let k = self.cost_of_capital(rf, beta, erp);
            if k <= 0.0 {
                return err("The cost of capital is not positive on at least one path: bound the draws of the discount rate (lo) or reduce the rate volatility.");
            }
            disc /= 1.0 + k;
            // cash flow
            let ebit_or_fcff = total * (m - sbc - ch);
            let (fcff, reinv) = if self.mode_b {
                let s2c = sc.x[self.p_s2c.expect("checked")];
                let pos = ebit_or_fcff.max(0.0);
                let used = loss.min(pos * self.loss_limit);
                let tax = self.tax * (pos - used);
                loss = loss - used + (-ebit_or_fcff).max(0.0);
                let reinv = (total - total_prev).max(0.0) / s2c;
                last_ebit = ebit_or_fcff;
                (ebit_or_fcff - tax - reinv, reinv)
            } else {
                (ebit_or_fcff, 0.0)
            };
            if want_detail {
                let d = &mut detail[(t - 1) * NF..t * NF];
                d[F_REV] = total;
                d[F_GROWTH] = total / total_prev - 1.0;
                d[F_MARGIN] = m;
                d[F_SBC] = sbc;
                d[F_REINV] = reinv;
                d[F_FCFF] = fcff;
                d[F_DISC] = disc;
                d[F_RATE] = k;
            }
            // failure and present value
            if let (Some(f), true) = (self.failure, hazard > 0.0) {
                let recovery = f.multiple * total_prev;
                if expected_failure {
                    pv += surv * ((1.0 - hazard) * fcff + hazard * recovery) * disc;
                    surv *= 1.0 - hazard;
                } else if sampled_failure && rng.uniform() < hazard {
                    pv += recovery * disc;
                    failed = true;
                    if want_detail {
                        let d = &mut detail[(t - 1) * NF..t * NF];
                        d[F_REV] = 0.0;
                        d[F_GROWTH] = -1.0;
                        d[F_FCFF] = recovery;
                    }
                    t_done = t - 1;
                    break;
                } else {
                    pv += fcff * disc;
                }
            } else {
                pv += fcff * disc;
            }
            last_fcff = fcff;
            total_prev = total;
            if let (Some(r), Some(ch)) = (&self.rate, self.rate_chan) {
                rate_state = r.step(t - 1, rate_state, sc.eps[ch]);
            }
        }
        let _ = t_done;
        // terminal value at the end of year N
        let (mut flags, mut k_terminal, mut pv_terminal) = (0u8, 0.0, 0.0);
        if !failed {
            let rf_n = match (&self.rate, self.rf_param) {
                (Some(r), _) => r.rf(years, rate_state) + shift_rf,
                (None, Some(i)) => sc.vals[i] + shift_rf,
                (None, None) => unreachable!(),
            };
            let k_t = self.cost_of_capital(rf_n, beta, erp);
            if k_t <= 0.0 {
                return err("The terminal cost of capital is not positive on at least one path.");
            }
            k_terminal = k_t;
            let mut tg = sc.vals[self.term_g];
            if let Some(cp) = self.cap_proc {
                if tg > sc.tgt[cp] {
                    tg = sc.tgt[cp];
                    flags |= CAP_PROCESS;
                }
            }
            if let Some(m) = self.rf_margin {
                if tg > rf_n + m {
                    tg = rf_n + m;
                    flags |= CAP_RF;
                }
            }
            if tg > k_t - TG_WACC_GAP {
                tg = k_t - TG_WACC_GAP;
                flags |= CAP_GAP;
            }
            let fcff_next = if self.mode_b {
                let nopat = if last_ebit > 0.0 { last_ebit * (1.0 + tg) * (1.0 - self.tax) } else { last_ebit * (1.0 + tg) };
                let roic = match self.roic {
                    RoicC::Wacc => k_t,
                    RoicC::Param(i) => sc.vals[i],
                    RoicC::None => unreachable!(),
                };
                let reinv = if nopat > 0.0 { tg / roic * nopat } else { 0.0 };
                nopat - reinv
            } else {
                last_fcff * (1.0 + tg)
            };
            pv_terminal = fcff_next / (k_t - tg) * disc * surv;
        }
        Ok(PathOut {
            per_share: (pv + pv_terminal - self.net_debt) / self.shares,
            pv_explicit: pv,
            pv_terminal,
            flags,
            k_terminal,
            failed,
            detail,
        })
    }

    fn mods_arrays(&self, mods: &Mods) -> (Vec<bool>, Vec<f64>) {
        let mut frozen = vec![false; self.groups.len() + 1];
        for g in &mods.frozen {
            frozen[*g] = true;
        }
        if let Some(only) = mods.only {
            for (g, f) in frozen.iter_mut().enumerate().take(self.groups.len()) {
                *f = g != only;
            }
        }
        frozen[self.groups.len()] = mods.no_shocks;
        let mut shift = vec![0.0; self.params.len()];
        for (i, d) in &mods.shift {
            shift[*i] += d;
        }
        (frozen, shift)
    }

    /// The simulation of `n` paths. `inj` gives the correlated standard normals of the drawn
    /// parameters from the caller (rows in the order of `stochastic_names`, `n` values each);
    /// without it the paths make their own. `detail` keeps the values of each year and path.
    pub fn simulate(&self, n: usize, seed: u64, mods: &Mods, inj: Option<&[Vec<f64>]>, detail: bool) -> Result<Sim, DcfError> {
        if n == 0 {
            return err("n must be positive, got 0.");
        }
        if let Some(z) = inj {
            if z.len() != self.stoch.len() || z.iter().any(|r| r.len() != n) {
                return err(format!("The injected normals need {} rows of {n} values.", self.stoch.len()));
            }
        }
        let (frozen, shift) = self.mods_arrays(mods);
        let one = |sc: &mut Scratch, j: usize| self.eval(j, seed, inj, &frozen, &shift, false, detail, sc);
        let outs: Vec<PathOut> = if n >= 256 {
            (0..n).into_par_iter().map_init(|| self.scratch(), one).collect::<Result<_, _>>()?
        } else {
            let mut sc = self.scratch();
            (0..n).map(|j| one(&mut sc, j)).collect::<Result<_, _>>()?
        };
        Ok(self.collect(outs, n, detail))
    }

    fn collect(&self, outs: Vec<PathOut>, n: usize, detail: bool) -> Sim {
        let mut sim = Sim {
            n,
            years: self.years,
            per_share: Vec::with_capacity(n),
            pv_explicit: Vec::with_capacity(n),
            pv_terminal: Vec::with_capacity(n),
            flags: Vec::with_capacity(n),
            k_terminal: Vec::with_capacity(n),
            failed: 0,
            detail: if detail { Vec::with_capacity(n * self.years * NF) } else { Vec::new() },
        };
        for o in outs {
            sim.per_share.push(o.per_share);
            sim.pv_explicit.push(o.pv_explicit);
            sim.pv_terminal.push(o.pv_terminal);
            sim.flags.push(o.flags);
            sim.k_terminal.push(o.k_terminal);
            sim.failed += o.failed as usize;
            if detail {
                sim.detail.extend_from_slice(&o.detail);
            }
        }
        sim
    }

    /// One path with every parameter at its mean and every shock at zero. The rate is the
    /// expected rate of the model (for Hull-White: the curve forward plus the convexity term).
    /// A failure hazard is the expected value (survival weights).
    pub fn deterministic(&self) -> Result<Sim, DcfError> {
        let mods = Mods::default();
        let (frozen, shift) = self.mods_arrays(&mods);
        let mut sc = self.scratch();
        let o = self.eval(0, 0, None, &frozen, &shift, true, true, &mut sc)?;
        Ok(self.collect(vec![o], 1, true))
    }
}

/// P50 change of one parameter at its mean minus and plus one standard deviation.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamEffect {
    pub name: String,
    pub p50_minus_1sd: f64,
    pub p50_plus_1sd: f64,
}

/// Variance of the value for each group of random elements.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupShare {
    pub group: String,
    /// Variance of the value when only this group is random, as a share of the variance with all groups random.
    pub alone: f64,
    /// Share of the variance that disappears when this group is fixed at its means.
    pub removed: f64,
}

fn variance(x: &[f64]) -> f64 {
    let m = x.iter().sum::<f64>() / x.len() as f64;
    x.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / (x.len() as f64 - 1.0).max(1.0)
}

impl PathEngine {
    /// P50 change for each drawn parameter at mean -1 sd / +1 sd (same random numbers; the
    /// bounds do not move, as in the old engine).
    pub fn attribution(&self, n: usize, seed: u64, base_p50: f64) -> Result<Vec<ParamEffect>, DcfError> {
        let jobs: Vec<(usize, f64)> = self.stoch.iter().flat_map(|i| [(*i, -1.0), (*i, 1.0)]).collect();
        let deltas: Vec<f64> = jobs
            .par_iter()
            .map(|&(i, dir)| {
                let mods = Mods { shift: vec![(i, dir * self.params[i].std)], ..Mods::default() };
                Ok(self.simulate_serial(n, seed, &mods)?.p50() - base_p50)
            })
            .collect::<Result<_, DcfError>>()?;
        Ok(self
            .stoch
            .iter()
            .enumerate()
            .map(|(q, i)| ParamEffect { name: self.params[*i].name.clone(), p50_minus_1sd: deltas[2 * q], p50_plus_1sd: deltas[2 * q + 1] })
            .collect())
    }

    /// The same as `simulate` without a thread pool (for use inside a parallel job).
    fn simulate_serial(&self, n: usize, seed: u64, mods: &Mods) -> Result<Sim, DcfError> {
        let (frozen, shift) = self.mods_arrays(mods);
        let mut sc = self.scratch();
        let outs: Vec<PathOut> = (0..n).map(|j| self.eval(j, seed, None, &frozen, &shift, false, false, &mut sc)).collect::<Result<_, _>>()?;
        Ok(self.collect(outs, n, false))
    }

    /// Variance shares by group (growth, margin, sbc, charge, s2c, discount, rates, terminal, failure).
    pub fn variance_shares(&self, n: usize, seed: u64, base: &[f64]) -> Result<Vec<GroupShare>, DcfError> {
        let full = variance(base);
        let used: Vec<usize> = (0..self.groups.len())
            .filter(|g| self.params.iter().any(|p| p.group == *g && p.std > 0.0) || self.chan_group.contains(g))
            .collect();
        used.par_iter()
            .map(|&g| {
                let alone = self.simulate_serial(n, seed, &Mods { only: Some(g), ..Mods::default() })?;
                let without = self.simulate_serial(n, seed, &Mods { frozen: vec![g], ..Mods::default() })?;
                Ok(GroupShare { group: self.groups[g].clone(), alone: variance(&alone.per_share) / full, removed: 1.0 - variance(&without.per_share) / full })
            })
            .collect()
    }
}
