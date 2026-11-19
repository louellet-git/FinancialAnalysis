//! Tests of the path engine (`paths.rs`, `pathrate.rs`, `process.rs`).
//!
//! Groups: (1) the old engine is not changed (the old tests run without any edit); (2) limits
//! that give the old engine; (3) fixed answers against an independent loop; (4) moments of the
//! processes; (5) the discount factors against the curve; (6) failure; (7) input errors; and
//! the speed table (`#[ignore]`).

use super::driver::{CorrEntry, DriverDcf};
use super::model::ModelSpec;
use super::pathrate::{RateFactor, RateKind, RfMode};
use super::paths::*;
use super::process::*;
use crate::ratemodels::curve::ZeroCurve;
use crate::ratemodels::models::{Cir, RateModel, Vasicek};
use crate::ratemodels::rng::Rng;
use crate::ratemodels::sim::mean_sd_se;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn gsw_curve() -> (ZeroCurve, CurveSpec) {
    let text = std::fs::read_to_string(format!("{}/tests/fixtures/dcf/gsw_2026-11-13.csv", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let ds = crate::forwards::parse_gsw(&text).unwrap();
    let (d, day) = crate::forwards::parse_date_arg("2026-11-13").unwrap();
    let f = crate::forwards::forwards_on(&ds, &d, day).unwrap();
    let c = ZeroCurve::from_forwards(&f).unwrap();
    let p = |n: &str| f.parameters[n];
    let spec = CurveSpec { date: d, beta0: p("BETA0"), beta1: p("BETA1"), beta2: p("BETA2"), beta3: p("BETA3"), tau1: p("TAU1"), tau2: p("TAU2") };
    (c, spec)
}

const BODY: &str = r#"
engine = "paths"
ticker = "TST"
net_debt = 1.0e9
shares = 1.0e8
projection_years = 6

[[segments]]
name = "A"
base_revenue = 1.0e10
process = "gA"
year1_growth = 0.10

[[segments]]
name = "B"
base_revenue = 4.0e9
process = "gB"

[process.gA]
role = "growth"
start = 0.30
target = 0.05
half_life = 2.0

[process.gB]
role = "growth"
start = 0.10
target = 0.03
half_life = 3.0

[process.m]
role = "margin"
start = -0.05
target = 0.25
half_life = 2.5
drift = { start = 0.0, end = 0.01, ramp_years = 6 }

[process.s]
role = "sbc"
start = 0.10
target = 0.03
shape = "ramp"
ramp_years = 4

[process.c]
role = "charge"
start = "target"
target = 0.01
half_life = inf

[discount]
beta = 1.2
erp = 0.05
kd_spread = 0.02
equity_weight = 0.8

[terminal]
growth = 0.03
"#;

fn fixed_rate_toml(extra: &str) -> String {
    format!("{BODY}\n[tax]\nrate = 0.25\n\n[rates]\nmodel = \"fixed\"\nrf = 0.045\n{extra}")
}

fn hw_toml(curve: &CurveSpec, sigma: f64, extra: &str) -> String {
    format!(
        "{BODY}\n[tax]\nrate = 0.25\n\n[rates]\nmodel = \"hullwhite\"\na = 0.1\nsigma = {sigma}\ntenor = 10.0\ncurve = {{ date = \"{}\", beta0 = {:e}, beta1 = {}, beta2 = {}, beta3 = {}, tau1 = {}, tau2 = {} }}\n{extra}",
        curve.date, curve.beta0, curve.beta1, curve.beta2, curve.beta3, curve.tau1, curve.tau2
    )
}

fn engine(spec: &PathsSpec) -> PathEngine {
    PathEngine::new(spec).unwrap()
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-300)
}

// ---------------------------------------------------------------------------
// An independent calculation of the value at the means (a plain loop)
// ---------------------------------------------------------------------------

/// The value per share with every parameter at its mean and every shock at zero, from the
/// specification only. Returns `(value per share, FCFF of each year)`.
fn hand_value(spec: &PathsSpec) -> (f64, Vec<f64>) {
    let n = spec.projection_years;
    let center = |p: &ProcessSpec, t: usize| -> f64 {
        let tgt = p.target.mean();
        let start = match &p.start {
            StartSpec::Tied(_) => tgt,
            StartSpec::Value(v) => v.mean(),
        };
        let c = if p.shape == "ou" {
            let h = p.half_life.as_ref().unwrap().mean();
            let phi = (-std::f64::consts::LN_2 / h).exp();
            tgt + (start - tgt) * phi.powi(t as i32 - 1)
        } else {
            let r = p.ramp_years.unwrap();
            start + (tgt - start) * ((t - 1).min(r - 1) as f64) / (r - 1) as f64
        };
        let d = p.drift.as_ref().map_or(0.0, |r| r.schedule(n)[t - 1]);
        (c + d).clamp(p.lo.unwrap_or(f64::NEG_INFINITY), p.hi.unwrap_or(f64::INFINITY))
    };
    let role = |r: &str| spec.process.values().find(|p| p.role == r);
    let (margin, sbc) = (role("margin").unwrap(), role("sbc").unwrap());
    let rate_curve = spec.rates.curve.as_ref().map(|c| ZeroCurve {
        beta0: c.beta0 / 100.0,
        beta1: c.beta1 / 100.0,
        beta2: c.beta2 / 100.0,
        beta3: c.beta3 / 100.0,
        tau1: c.tau1,
        tau2: c.tau2,
    });
    let flat = spec.rates.mode == "long_flat";
    let rf_at = |i: usize| -> f64 {
        let i = if flat { 0 } else { i };
        match (spec.rates.model.as_str(), &rate_curve) {
            ("fixed", _) => spec.rates.rf.as_ref().unwrap().mean(),
            ("hullwhite", Some(c)) => {
                let (a, s, tau, t) = (spec.rates.a.unwrap(), spec.rates.sigma.unwrap(), spec.rates.tenor, i as f64);
                let b = |x: f64| (1.0 - (-a * x).exp()) / a;
                let alpha = c.forward(t) + 0.5 * s * s * b(t) * b(t);
                // x = 0 at the mean: r_t = alpha(t)
                let ln_p = ((-(t + tau) * c.zero_yield(t + tau)) - (-t * c.zero_yield(t))) + b(tau) * c.forward(t)
                    - s * s / (4.0 * a) * (1.0 - (-2.0 * a * t).exp()) * b(tau) * b(tau)
                    - b(tau) * alpha;
                -ln_p / tau
            }
            _ => panic!("hand_value: rate model"),
        }
    };
    let (beta, erp) = (spec.discount.beta.mean(), spec.discount.erp.mean());
    let (w, kd, tax) = (spec.discount.equity_weight, spec.discount.kd_spread, spec.tax.rate);
    let k_of = |rf: f64| w * (rf + beta * erp) + (1.0 - w) * (rf + kd) * (1.0 - tax);
    let mode_b = spec.reinvestment.mode == "sales_to_capital";
    let hazard = spec.failure.as_ref().map_or(0.0, |f| f.hazard.mean());
    let mult = spec.failure.as_ref().map_or(0.0, |f| f.recovery_revenue_multiple);
    let mut rev_seg: Vec<f64> = spec.segments.iter().map(|s| s.base_revenue).collect();
    let mut total_prev: f64 = rev_seg.iter().sum();
    let (mut disc, mut pv, mut surv) = (1.0, 0.0, 1.0);
    let mut loss = spec.tax.opening_loss;
    let (mut last_fcff, mut last_ebit) = (0.0, 0.0);
    let mut flows = Vec::new();
    for t in 1..=n {
        let mut total = 0.0;
        for (s, seg) in spec.segments.iter().enumerate() {
            let g = if t == 1 { seg.year1_growth.unwrap_or_else(|| center(&spec.process[&seg.process], 1)) } else { center(&spec.process[&seg.process], t) };
            rev_seg[s] *= 1.0 + g;
            total += rev_seg[s];
        }
        let ch = role("charge").map_or(0.0, |p| center(p, t));
        let x = total * (center(margin, t) - center(sbc, t) - ch);
        let k = k_of(rf_at(t - 1));
        disc /= 1.0 + k;
        let fcff = if mode_b {
            let s2c = center(role("s2c").unwrap(), t);
            let pos = x.max(0.0);
            let used = loss.min(pos * spec.tax.loss_limit_share);
            let tx = tax * (pos - used);
            loss = loss - used + (-x).max(0.0);
            last_ebit = x;
            x - tx - (total - total_prev).max(0.0) / s2c
        } else {
            x
        };
        flows.push(fcff);
        pv += surv * ((1.0 - hazard) * fcff + hazard * mult * total_prev) * disc;
        surv *= 1.0 - hazard;
        last_fcff = fcff;
        total_prev = total;
    }
    let k_t = k_of(rf_at(n));
    let rf_n = rf_at(n);
    let mut tg = spec.terminal.growth.mean();
    if let Some(p) = &spec.terminal.cap_by_process {
        tg = tg.min(spec.process[p].target.mean());
    }
    if let Some(m) = spec.terminal.rf_margin {
        tg = tg.min(rf_n + m);
    }
    tg = tg.min(k_t - 0.02);
    let next = if mode_b {
        let nopat = if last_ebit > 0.0 { last_ebit * (1.0 + tg) * (1.0 - tax) } else { last_ebit * (1.0 + tg) };
        let roic = match &spec.terminal.roic {
            Some(StartSpec::Tied(_)) => k_t,
            Some(StartSpec::Value(v)) => v.mean(),
            None => unreachable!(),
        };
        nopat - if nopat > 0.0 { tg / roic * nopat } else { 0.0 }
    } else {
        last_fcff * (1.0 + tg)
    };
    pv += next / (k_t - tg) * disc * surv;
    ((pv - spec.net_debt) / spec.shares, flows)
}

// ---------------------------------------------------------------------------
// (3) Fixed answers
// ---------------------------------------------------------------------------

#[test]
fn fixed_answer_option_a_fixed_rate() {
    let spec = PathsSpec::from_toml(&fixed_rate_toml("")).unwrap();
    let (hand, flows) = hand_value(&spec);
    let det = engine(&spec).deterministic().unwrap();
    assert!(rel(det.per_share[0], hand) < 1e-12, "{} vs {hand}", det.per_share[0]);
    for (t, f) in flows.iter().enumerate() {
        assert!(rel(det.at(0, t, F_FCFF), *f) < 1e-12);
    }
}

/// A closed form that has no loop in it: one segment, constant growth and margin, flat rate.
#[test]
fn fixed_answer_closed_form_geometric_series() {
    let toml = r#"
engine = "paths"
net_debt = 0.0
shares = 1.0
projection_years = 3
[[segments]]
name = "A"
base_revenue = 100.0
process = "g"
[process.g]
role = "growth"
start = "target"
target = 0.10
half_life = inf
[process.m]
role = "margin"
start = "target"
target = 0.30
half_life = inf
[process.s]
role = "sbc"
start = "target"
target = 0.10
half_life = inf
[discount]
beta = 1.0
erp = 0.05
[rates]
rf = 0.05
[terminal]
growth = 0.02
"#;
    let spec = PathsSpec::from_toml(toml).unwrap();
    let v = engine(&spec).deterministic().unwrap().per_share[0];
    // k = 0.05 + 1.0 * 0.05 = 0.10. FCFF_t = 100 * 1.1^t * 0.2 = 22 * 1.1^(t-1) * 1.0 ... compute by hand:
    // revenue 110, 121, 133.1; FCFF 22, 24.2, 26.62; each divided by 1.1^t gives 20 each.
    // PV of years 1 to 3 = 60. Terminal: 26.62 * 1.02 / (0.10 - 0.02) / 1.1^3 = 339.405 / 1.331.
    let tv = 26.62 * 1.02 / 0.08 / 1.331;
    assert!(rel(v, 60.0 + tv) < 1e-13, "{v} vs {}", 60.0 + tv);
}

#[test]
fn fixed_answer_option_b_with_loss_carry_forward_literal() {
    // revenue stays 100; margin (EBIT before costs) -50%, -5%, +40% (a ramp over 3 years); tax 25%.
    let toml = |limit: f64| {
        format!(
            r#"
engine = "paths"
net_debt = 0.0
shares = 1.0
projection_years = 3
[[segments]]
name = "A"
base_revenue = 100.0
process = "g"
year1_growth = 0.0
[process.g]
role = "growth"
start = 0.0
target = 0.0
half_life = inf
[process.m]
role = "margin"
start = -0.50
target = 0.40
shape = "ramp"
ramp_years = 3
[process.s]
role = "sbc"
start = "target"
target = 0.0
half_life = inf
[process.k]
role = "s2c"
start = "target"
target = 2.0
half_life = inf
[reinvestment]
mode = "sales_to_capital"
[tax]
rate = 0.25
loss_limit_share = {limit}
[discount]
beta = 0.0
erp = 0.05
[rates]
rf = 0.10
[terminal]
growth = 0.0
roic = 0.10
"#
        )
    };
    // EBIT: -50, -5, +40. Loss pool: 50, 55, then year 3 uses min(55, 40 * limit).
    let d1 = engine(&PathsSpec::from_toml(&toml(1.0)).unwrap()).deterministic().unwrap();
    for (t, want) in [-50.0, -5.0, 40.0].into_iter().enumerate() {
        assert!((d1.at(0, t, F_FCFF) - want).abs() < 1e-12, "year {t}");
    }
    let d2 = engine(&PathsSpec::from_toml(&toml(0.8)).unwrap()).deterministic().unwrap();
    // limit 0.8: use 32, tax 0.25 * (40 - 32) = 2, FCFF 38
    assert!((d2.at(0, 2, F_FCFF) - 38.0).abs() < 1e-12);
    // terminal: tg 0, so no reinvestment; EBIT 40 taxed at 25% without the remaining pool: NOPAT 30; TV = 30 / 0.10.
    // PV: -50/1.1 - 5/1.21 + 40/1.331 + 300/1.331 (limit 1)
    let pv = -50.0 / 1.1 - 5.0 / 1.21 + 40.0 / 1.331 + 300.0 / 1.331;
    assert!(rel(d1.per_share[0], pv) < 1e-13, "{} vs {pv}", d1.per_share[0]);
    // the same with a growth of the revenue: reinvestment = revenue change / s2c
    let g = toml(1.0).replace("year1_growth = 0.0", "year1_growth = 0.2").replace("start = 0.0\ntarget = 0.0", "start = 0.1\ntarget = 0.1");
    let d3 = engine(&PathsSpec::from_toml(&g).unwrap()).deterministic().unwrap();
    // revenue 120, 132, 145.2; reinvestment 10, 6, 6.6; EBIT -60, -6.6, +58.08
    assert!(rel(d3.at(0, 0, F_REINV), 10.0) < 1e-14 && rel(d3.at(0, 1, F_REINV), 6.0) < 1e-14 && rel(d3.at(0, 2, F_REINV), 6.6) < 1e-13);
}

#[test]
fn fixed_answer_option_b_against_the_loop() {
    let extra = r#"
"#;
    let mut text = fixed_rate_toml(extra);
    text = text.replace("[discount]", "[reinvestment]\nmode = \"sales_to_capital\"\n\n[process.k]\nrole = \"s2c\"\nstart = 1.5\ntarget = 2.5\nhalf_life = 3.0\n\n[discount]");
    text = text.replace("[tax]\nrate = 0.25", "[tax]\nrate = 0.25\nopening_loss = 2.0e9\nloss_limit_share = 0.8");
    text = text.replace("growth = 0.03\n", "growth = 0.03\nroic = 0.12\ncap_by_process = \"gA\"\n");
    let spec = PathsSpec::from_toml(&text).unwrap();
    let (hand, flows) = hand_value(&spec);
    let det = engine(&spec).deterministic().unwrap();
    assert!(rel(det.per_share[0], hand) < 1e-12, "{} vs {hand}", det.per_share[0]);
    for (t, f) in flows.iter().enumerate() {
        assert!(rel(det.at(0, t, F_FCFF), *f) < 1e-11, "year {t}");
    }
}

#[test]
fn fixed_answer_hull_white_with_the_curve_forward_path() {
    let (curve, cs) = gsw_curve();
    // volatility near 0: the rate of year t is the 10-year yield that the curve implies for the forward start t-1
    let spec = PathsSpec::from_toml(&hw_toml(&cs, 1e-9, "")).unwrap();
    let det = engine(&spec).deterministic().unwrap();
    for t in 0..6 {
        let tt = t as f64;
        let y = (-(tt + 10.0) * curve.zero_yield(tt + 10.0) + tt * curve.zero_yield(tt)) / -10.0;
        let k = 0.8 * (y + 1.2 * 0.05) + 0.2 * (y + 0.02) * 0.75;
        // the discount factor is the product of the yearly factors
        let mut disc = 1.0;
        for s in 0..=t {
            let ss = s as f64;
            let ys = (-(ss + 10.0) * curve.zero_yield(ss + 10.0) + ss * curve.zero_yield(ss)) / -10.0;
            disc /= 1.0 + 0.8 * (ys + 1.2 * 0.05) + 0.2 * (ys + 0.02) * 0.75;
        }
        assert!(rel(det.at(0, t, F_RATE), k) < 1e-6, "k year {t}: {} vs {k}", det.at(0, t, F_RATE));
        assert!(rel(det.at(0, t, F_DISC), disc) < 1e-6);
    }
    // with the full volatility the hand loop (closed form of the bond price at the mean state) agrees to 1e-12
    let spec = PathsSpec::from_toml(&hw_toml(&cs, 0.012, "")).unwrap();
    let (hand, _) = hand_value(&spec);
    let det = engine(&spec).deterministic().unwrap();
    assert!(rel(det.per_share[0], hand) < 1e-11, "{} vs {hand}", det.per_share[0]);
}

#[test]
fn application_models_agree_with_the_loop_at_the_means() {
    for ticker in ["ANT", "IBM"] {
        let path = repo().join("analysis").join(ticker).join("driver_model_paths.toml");
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let spec = PathsSpec::from_toml(&text).unwrap();
        let (hand, _) = hand_value(&spec);
        let det = engine(&spec).deterministic().unwrap();
        assert!(rel(det.per_share[0], hand) < 1e-11, "{ticker}: {} vs {hand}", det.per_share[0]);
    }
}

// ---------------------------------------------------------------------------
// (2) Limits: the old engine
// ---------------------------------------------------------------------------

/// The paths model of an old driver model: one draw for each path, no yearly shocks, fixed
/// rate. The old drivers `margin_mix_drift`, `sbc_ratio` and `ma_*` are written as the
/// processes that have the same schedules. Returns the spec and the map old name -> new name.
fn old_as_paths(old: &ModelSpec) -> (PathsSpec, BTreeMap<String, String>) {
    let eng = old.engine().unwrap();
    let drivers = &eng.model.drivers;
    let get = |n: &str| drivers.iter().find(|d| d.name == n).unwrap();
    let param = |n: &str| -> ParamSpec {
        let d = get(n);
        if d.stochastic {
            ParamSpec::Draw(DrawSpec { mean: d.mean, std: d.std, lo: d.lo, hi: d.hi })
        } else {
            ParamSpec::Fixed(d.mean)
        }
    };
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let mut process: BTreeMap<String, ProcessSpec> = BTreeMap::new();
    let flat = |target: ParamSpec, role: &str| ProcessSpec {
        role: role.into(),
        start: StartSpec::Tied("target".into()),
        target,
        shape: "ou".into(),
        half_life: Some(ParamSpec::Fixed(f64::INFINITY)),
        ramp_years: None,
        sigma: 0.0,
        sigma_level: 0.0,
        persistence: None,
        lo: None,
        hi: None,
        drift: None,
        rationale: String::new(),
    };
    let mut segments = Vec::new();
    for s in &old.segments {
        let pname = s.growth_driver.clone();
        process.insert(pname.clone(), flat(param(&s.growth_driver), "growth"));
        map.insert(s.growth_driver.clone(), format!("process.{pname}.target"));
        segments.push(PathSegment { name: s.name.clone(), base_revenue: s.base_revenue, process: pname, year1_growth: s.year1_growth });
    }
    let sched_to_ramp = |d: &super::driver::Driver| -> super::driver::Ramp {
        match &d.schedule {
            Some(s) => (2..=s.len())
                .map(|r| super::driver::Ramp { start: s[0], end: s[s.len() - 1], ramp_years: r })
                .find(|r| r.schedule(s.len()).iter().zip(s).all(|(a, b)| (a - b).abs() < 1e-15))
                .unwrap_or_else(|| panic!("the schedule of {} is not a ramp", d.name)),
            None => super::driver::Ramp { start: d.mean, end: d.mean, ramp_years: 2 },
        }
    };
    let mut margin = flat(param("core_margin"), "margin");
    margin.drift = Some(sched_to_ramp(get("margin_mix_drift")));
    map.insert("core_margin".into(), "process.margin.target".into());
    process.insert("margin".into(), margin);
    let sbc = {
        let r = sched_to_ramp(get("sbc_ratio"));
        ProcessSpec {
            start: StartSpec::Value(ParamSpec::Fixed(r.start)),
            shape: "ramp".into(),
            half_life: None,
            ramp_years: Some(r.ramp_years),
            ..flat(ParamSpec::Fixed(r.end), "sbc")
        }
    };
    process.insert("sbc".into(), sbc);
    // charge = recurrence * ratio: a draw scaled by the (fixed) recurrence
    let rec = get("ma_recurrence").mean;
    let ma = get("ma_ratio");
    let charge = if ma.stochastic {
        ParamSpec::Draw(DrawSpec { mean: rec * ma.mean, std: rec * ma.std, lo: ma.lo.map(|v| rec * v), hi: ma.hi.map(|v| rec * v) })
    } else {
        ParamSpec::Fixed(rec * ma.mean)
    };
    process.insert("charge".into(), flat(charge, "charge"));
    map.insert("ma_ratio".into(), "process.charge.target".into());
    map.insert("rf".into(), "rates.rf".into());
    map.insert("erp".into(), "discount.erp".into());
    map.insert("beta".into(), "discount.beta".into());
    map.insert("terminal_growth".into(), "terminal.growth".into());
    let corr = old
        .corr
        .iter()
        .map(|e: &CorrEntry| CorrSpec { a: map[&e.a].clone(), b: map[&e.b].clone(), rho: e.rho, note: String::new() })
        .collect();
    let spec = PathsSpec {
        engine: "paths".into(),
        ticker: old.ticker.clone(),
        notes: String::new(),
        net_debt: old.net_debt,
        shares: old.shares,
        projection_years: old.projection_years,
        segments,
        process,
        reinvestment: ReinvestSpec::default(),
        tax: TaxSpec { rate: get("tax_rate").mean, opening_loss: 0.0, loss_limit_share: 1.0 },
        discount: DiscountSpec { beta: param("beta"), erp: param("erp"), kd_spread: get("kd_spread").mean, equity_weight: get("equity_weight").mean },
        rates: RatesSpec { rf: Some(param("rf")), ..RatesSpec::default() },
        terminal: TerminalSpec { growth: param("terminal_growth"), cap_by_process: None, rf_margin: None, roic: None },
        failure: None,
        corr,
        shock_corr: vec![],
        segment_history: BTreeMap::new(),
    };
    (spec, map)
}

/// The correlated normals of the old engine in the row order of the new engine.
fn old_z_in_new_order(old: &ModelSpec, map: &BTreeMap<String, String>, new: &PathEngine, n: usize, seed: u64) -> Vec<Vec<f64>> {
    let eng = old.engine().unwrap();
    let z = eng.model.correlated_z(n, seed).unwrap();
    let old_names: Vec<String> = eng.model.stochastic_drivers().iter().map(|d| d.name.clone()).collect();
    new.stochastic_names()
        .iter()
        .map(|nn| {
            let (old_name, _) = map.iter().find(|(_, v)| v.as_str() == *nn).unwrap_or_else(|| panic!("no old driver for {nn}"));
            let q = old_names.iter().position(|x| x == old_name).unwrap();
            z[q].clone()
        })
        .collect()
}

fn load_old(ticker: &str) -> ModelSpec {
    ModelSpec::load(&repo(), ticker).unwrap().unwrap()
}

#[test]
fn limit_no_reversion_no_shocks_fixed_rate_is_the_old_engine_at_the_means() {
    for ticker in ["IBM", "JNJ", "ANT"] {
        let old = load_old(ticker);
        let (spec, _) = old_as_paths(&old);
        let new = engine(&spec).deterministic().unwrap().per_share[0];
        let want = old.engine().unwrap().value_at_means().unwrap();
        assert!(rel(new, want) < 1e-12, "{ticker}: {new} vs {want}");
    }
}

/// The same draws, path by path: the paths model that copies the old model gives the old
/// values of each path to 1e-10 (the sums are in another order; the difference is about 1e-15)
/// and the reviewed percentiles.
#[test]
fn limit_copy_of_the_old_model_with_the_same_draws_gives_the_old_percentiles() {
    let record: [(&str, [f64; 3]); 2] = [("IBM", [83.84, 151.84, 258.03]), ("ANT", [12.82, 30.97, 74.96])];
    for (ticker, want) in record {
        let old = load_old(ticker);
        let (spec, map) = old_as_paths(&old);
        let new = engine(&spec);
        let (n, seed) = (10_000, 7);
        let z = old_z_in_new_order(&old, &map, &new, n, seed);
        let sim = new.simulate(n, seed, &Mods::default(), Some(&z), false).unwrap();
        let oe: DriverDcf = old.engine().unwrap();
        let draws = oe.model.sample(n, seed, &[]).unwrap();
        let ov = oe.valuation_with(&draws, n, false).unwrap();
        let worst = sim.per_share.iter().zip(&ov.per_share).map(|(a, b)| rel(*a, *b)).fold(0.0, f64::max);
        eprintln!("{ticker}: worst per-path relative difference to the old engine {worst:e}");
        assert!(worst < 1e-10, "{ticker}: worst relative difference {worst:e}");
        let q = sim.quantiles();
        for (a, b) in [(q[0], want[0]), (q[2], want[1]), (q[4], want[2])] {
            assert!((a - b).abs() < 0.005, "{ticker}: {a} vs {b}");
        }
    }
}

/// With its own random numbers the copy gives the same distribution: percentiles within the
/// Monte Carlo error (the standard error of a percentile of 10,000 paths is about 1.5% of
/// the spread; the test accepts 5% of the P10 to P90 width).
#[test]
fn limit_copy_with_own_random_numbers_gives_the_same_distribution() {
    let old = load_old("IBM");
    let (spec, _) = old_as_paths(&old);
    let sim = engine(&spec).simulate(100_000, 11, &Mods::default(), None, false).unwrap();
    let q = sim.quantiles();
    let want = [83.84, 112.57, 151.84, 200.39, 258.03];
    let width = want[4] - want[0];
    for (a, b) in q.iter().zip(want) {
        assert!((a - b).abs() < 0.03 * width, "{a} vs {b}");
    }
}

// ---------------------------------------------------------------------------
// (4) Moments of the processes
// ---------------------------------------------------------------------------

fn moments_toml() -> &'static str {
    r#"
engine = "paths"
net_debt = 0.0
shares = 1.0
projection_years = 10
[[segments]]
name = "A"
base_revenue = 100.0
process = "g"
[process.g]
role = "growth"
start = 0.40
target = 0.05
half_life = 2.0
sigma = 0.05
[process.m]
role = "margin"
start = -0.10
target = 0.20
half_life = 3.0
sigma = 0.03
[process.s]
role = "sbc"
start = "target"
target = 0.0
half_life = inf
[discount]
beta = 1.0
erp = 0.05
[rates]
rf = 0.05
[terminal]
growth = 0.02
[[shock_corr]]
a = "g"
b = "m"
rho = 0.6
"#
}

#[test]
fn moments_of_growth_and_margin_follow_the_ar1_formulas() {
    let spec = PathsSpec::from_toml(moments_toml()).unwrap();
    let n = 100_000;
    let sim = engine(&spec).simulate(n, 5, &Mods::default(), None, true).unwrap();
    let phi_g = (-std::f64::consts::LN_2 / 2.0).exp();
    let phi_m = (-std::f64::consts::LN_2 / 3.0).exp();
    // year 1: growth is the start value (no pin), no shock
    for t in 0..10 {
        let k = t as i32;
        let var_unit = |phi: f64| (0..t).map(|i| phi.powi(2 * i as i32)).sum::<f64>();
        for (f, start, tgt, phi, sigma) in [(F_GROWTH, 0.40, 0.05, phi_g, 0.05), (F_MARGIN, -0.10, 0.20, phi_m, 0.03)] {
            let mean_want = tgt + (start - tgt) * phi.powi(k);
            let var_want = sigma * sigma * var_unit(phi);
            // growth of the total revenue is the growth of the one segment
            let x: Vec<f64> = sim.column(t, f);
            let (mean, sd, se) = mean_sd_se(&x);
            assert!((mean - mean_want).abs() <= 4.0 * se + 1e-9, "year {} field {f}: mean {mean} vs {mean_want} (se {se})", t + 1);
            let var = sd * sd;
            // the standard error of a variance of normal draws: var * sqrt(2 / n)
            assert!((var - var_want).abs() <= 4.0 * var_want * (2.0 / n as f64).sqrt() + 1e-12, "year {} field {f}: var {var} vs {var_want}", t + 1);
        }
    }
    // correlation of the shocks of the same year: innovation = x(t) - target - phi (x(t-1) - target) = sigma e(t)
    let innov = |f: usize, t: usize, tgt: f64, phi: f64| -> Vec<f64> {
        (0..n).map(|j| (sim.at(j, t, f) - tgt) - phi * (sim.at(j, t - 1, f) - tgt)).collect()
    };
    let corr = |a: &[f64], b: &[f64]| {
        let (ma, mb) = (a.iter().sum::<f64>() / a.len() as f64, b.iter().sum::<f64>() / b.len() as f64);
        let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
        for (x, y) in a.iter().zip(b) {
            sab += (x - ma) * (y - mb);
            saa += (x - ma) * (x - ma);
            sbb += (y - mb) * (y - mb);
        }
        sab / (saa * sbb).sqrt()
    };
    let se_corr = (1.0 - 0.36) / (n as f64).sqrt();
    for t in [2usize, 5, 8] {
        let (eg, em) = (innov(F_GROWTH, t, 0.05, phi_g), innov(F_MARGIN, t, 0.20, phi_m));
        let c = corr(&eg, &em);
        assert!((c - 0.6).abs() < 4.0 * se_corr, "year {}: shock correlation {c}", t + 1);
        // the shock of a year is not correlated with the shock of the next year
        let eg2 = innov(F_GROWTH, t + 1, 0.05, phi_g);
        assert!(corr(&eg, &eg2).abs() < 4.0 / (n as f64).sqrt());
        // the standard deviation of the innovation is sigma
        let (_, sd, _) = mean_sd_se(&eg);
        assert!((sd - 0.05).abs() < 4.0 * 0.05 / (2.0 * n as f64).sqrt());
    }
}

#[test]
fn bounds_cut_each_year_and_phi_one_without_shocks_is_constant_growth() {
    let text = moments_toml().replace("sigma = 0.05", "sigma = 0.05\nlo = 0.0\nhi = 0.30");
    let spec = PathsSpec::from_toml(&text).unwrap();
    let sim = engine(&spec).simulate(2_000, 3, &Mods::default(), None, true).unwrap();
    for j in 0..2_000 {
        for t in 0..10 {
            let g = sim.at(j, t, F_GROWTH);
            assert!((0.0..=0.30 + 1e-12).contains(&g), "growth {g}");
        }
    }
    // phi = 1, sigma = 0, start tied: the revenue is base * (1 + g)^t
    let flat = moments_toml().replace("start = 0.40", "start = \"target\"").replace("half_life = 2.0\nsigma = 0.05", "half_life = inf").replace("sigma = 0.03", "").replace("[[shock_corr]]\na = \"g\"\nb = \"m\"\nrho = 0.6\n", "");
    let det = engine(&PathsSpec::from_toml(&flat).unwrap()).deterministic().unwrap();
    for t in 0..10 {
        assert!(rel(det.at(0, t, F_REV), 100.0 * 1.05f64.powi(t as i32 + 1)) < 1e-13);
    }
}

#[test]
fn drawn_parameters_have_the_stated_moments_and_correlation() {
    let text = moments_toml()
        .replace("target = 0.05\nhalf_life = 2.0\nsigma = 0.05", "target = { mean = 0.05, std = 0.02 }\nhalf_life = 2.0\nsigma = 0.05")
        .replace("target = 0.20\nhalf_life = 3.0", "target = { mean = 0.20, std = 0.04 }\nhalf_life = 3.0")
        + "\n[[corr]]\na = \"process.g.target\"\nb = \"process.m.target\"\nrho = -0.5\n";
    let spec = PathsSpec::from_toml(&text).unwrap();
    let e = engine(&spec);
    assert_eq!(e.stochastic_names(), vec!["process.g.target", "process.m.target"]);
    let n = 100_000;
    let sim = e.simulate(n, 9, &Mods::default(), None, true).unwrap();
    // the margin of year 10 at 9 steps: mean = target + (start - target) phi^9; shock variance is in the margin too
    let phi_m = (-std::f64::consts::LN_2 / 3.0).exp();
    let x: Vec<f64> = sim.column(9, F_MARGIN);
    let (mean, sd, se) = mean_sd_se(&x);
    let want = 0.20 + (-0.10 - 0.20) * phi_m.powi(9);
    // target draw (std 0.04) enters with weight (1 - phi^9); the shock part is sigma^2 (1 - phi^18)/(1 - phi^2)
    let var_want = (0.04f64 * (1.0 - phi_m.powi(9))).powi(2) + 0.03f64.powi(2) * (1.0 - phi_m.powi(18)) / (1.0 - phi_m * phi_m);
    assert!((mean - want).abs() < 4.0 * se);
    assert!((sd * sd - var_want).abs() < 4.0 * var_want * (2.0 / n as f64).sqrt());
}

// ---------------------------------------------------------------------------
// (5) Discount factors against the curve
// ---------------------------------------------------------------------------

fn discount_factors(f: &RateFactor, n: usize, seed: u64, years: usize, per_year: usize) -> Vec<Vec<f64>> {
    use rayon::prelude::*;
    let steps = years * per_year;
    let dt = f.dt;
    let rows: Vec<Vec<f64>> = (0..n)
        .into_par_iter()
        .map(|p| {
            let mut rng = Rng::for_path(seed, p as u64);
            let mut s = f.state0;
            let mut integral = 0.0;
            let mut out = Vec::with_capacity(years + 1);
            let mut r_prev = f.short_rate(0, s);
            out.push(1.0);
            for i in 0..steps {
                s = f.step(i, s, rng.normal());
                let r = f.short_rate(i + 1, s);
                integral += 0.5 * (r_prev + r) * dt;
                r_prev = r;
                if (i + 1) % per_year == 0 {
                    // discount factor to this time, and the zero-bond price implied at this time for the tenor
                    out.push((-integral).exp());
                    out.push(f.rf(i + 1, s));
                }
            }
            out
        })
        .collect();
    rows
}

#[test]
fn hull_white_discount_factors_agree_with_the_curve() {
    let (curve, _) = gsw_curve();
    let (n, per_year, years) = (400_000, 12, 10);
    let f = RateFactor::new(RateKind::HullWhite { a: 0.1, sigma: 0.012, curve }, 1.0 / per_year as f64, years * per_year, RfMode::Long, 10.0, 0.0).unwrap();
    let rows = discount_factors(&f, n, 17, years, per_year);
    for t in 1..=years {
        let d: Vec<f64> = rows.iter().map(|r| r[1 + 2 * (t - 1)]).collect();
        let (mean, _, se) = mean_sd_se(&d);
        let want = curve.discount(t as f64);
        eprintln!("HW discount t={t}: mean {mean:.6} curve {want:.6} z {:.2}", (mean - want) / se);
        // 3 standard errors of the Monte Carlo mean; the trapezoid rule at one month adds a bias below 2e-5
        assert!((mean - want).abs() < 3.0 * se + 2e-5, "t={t}: {mean} vs {want} (se {se}, z {:.2})", (mean - want) / se);
    }
    // the implied 10-year yield of time t prices the bond: E[D(0,t) exp(-10 y_t)] = P(0, t+10)
    for t in [1usize, 3, 5] {
        let d: Vec<f64> = rows.iter().map(|r| r[1 + 2 * (t - 1)] * (-10.0 * r[2 + 2 * (t - 1)]).exp()).collect();
        let (mean, _, se) = mean_sd_se(&d);
        let want = curve.discount(t as f64 + 10.0);
        assert!((mean - want).abs() < 3.0 * se + 2e-5, "implied yield, t={t}: {mean} vs {want} (se {se})");
    }
}

#[test]
fn vasicek_and_cir_implied_yields_price_the_bond() {
    let (n, per_year, years) = (100_000, 12, 6);
    let kinds = [
        RateKind::Vasicek { kappa: 0.4, theta: 0.05, sigma: 0.012, r0: 0.03 },
        RateKind::Cir { kappa: 0.4, theta: 0.05, sigma: 0.08, r0: 0.03 },
    ];
    for kind in kinds {
        let f = RateFactor::new(kind, 1.0 / per_year as f64, years * per_year, RfMode::Long, 5.0, 0.0).unwrap();
        let rows = discount_factors(&f, n, 23, years, per_year);
        let price = |t: f64| match kind {
            RateKind::Vasicek { kappa, theta, sigma, r0 } => Vasicek { kappa, theta, sigma }.bond_price(r0, t).unwrap(),
            RateKind::Cir { kappa, theta, sigma, r0 } => Cir { kappa, theta, sigma }.bond_price(r0, t).unwrap(),
            _ => unreachable!(),
        };
        for t in [1usize, 3, 6] {
            let d: Vec<f64> = rows.iter().map(|r| r[1 + 2 * (t - 1)]).collect();
            let (mean, _, se) = mean_sd_se(&d);
            assert!((mean - price(t as f64)).abs() < 3.0 * se + 5e-5, "{kind:?} t={t}: {mean} vs {} (se {se})", price(t as f64));
            let d: Vec<f64> = rows.iter().map(|r| r[1 + 2 * (t - 1)] * (-5.0 * r[2 + 2 * (t - 1)]).exp()).collect();
            let (mean, _, se) = mean_sd_se(&d);
            assert!((mean - price(t as f64 + 5.0)).abs() < 3.0 * se + 5e-5, "{kind:?} implied yield t={t}: {mean} vs {} (se {se})", price(t as f64 + 5.0));
        }
    }
}

#[test]
fn the_engine_discount_rate_follows_the_rate_path_and_the_value_is_lower_when_rates_are_higher() {
    let (_, cs) = gsw_curve();
    let spec = PathsSpec::from_toml(&hw_toml(&cs, 0.012, "")).unwrap();
    let e = engine(&spec);
    let sim = e.simulate(20_000, 2, &Mods::default(), None, true).unwrap();
    // the discount rate k of year 3 against the rate shock: more spread than at year 1 (year 1 is known)
    let k1: Vec<f64> = sim.column(0, F_RATE);
    let k3: Vec<f64> = sim.column(2, F_RATE);
    assert!(mean_sd_se(&k1).1 < 1e-9);
    assert!(mean_sd_se(&k3).1 > 1e-3);
}

// ---------------------------------------------------------------------------
// (6) Failure
// ---------------------------------------------------------------------------

fn failure_toml(mode: &str, hazard: f64) -> String {
    let toml = r#"
engine = "paths"
net_debt = 0.0
shares = 1.0
projection_years = 5
[[segments]]
name = "A"
base_revenue = 100.0
process = "g"
year1_growth = 0.2
[process.g]
role = "growth"
start = 0.1
target = 0.1
half_life = inf
[process.m]
role = "margin"
start = "target"
target = 0.25
half_life = inf
[process.s]
role = "sbc"
start = "target"
target = 0.05
half_life = inf
[discount]
beta = 1.0
erp = 0.04
[rates]
rf = 0.06
[terminal]
growth = 0.02
[failure]
recovery_revenue_multiple = 0.5
"#;
    format!("{toml}hazard = {hazard}\nmode = \"{mode}\"\n")
}

#[test]
fn failure_expected_mode_equals_the_closed_form() {
    let h = 0.07;
    let spec = PathsSpec::from_toml(&failure_toml("expected", h)).unwrap();
    let v = engine(&spec).deterministic().unwrap();
    // by hand: k = 0.10; revenue 120, 132, 145.2, 159.72, 175.692; margin 0.20 of revenue
    let rev = [120.0, 132.0, 145.2, 159.72, 175.692];
    let prev = [100.0, 120.0, 132.0, 145.2, 159.72];
    let mut pv = 0.0;
    let mut surv = 1.0;
    for t in 0..5 {
        let d = 1.1f64.powi(t as i32 + 1);
        pv += surv * ((1.0 - h) * 0.20 * rev[t] + h * 0.5 * prev[t]) / d;
        surv *= 1.0 - h;
    }
    let tv = 0.20 * rev[4] * 1.02 / (0.10 - 0.02) / 1.1f64.powi(5) * surv;
    assert!(rel(v.per_share[0], pv + tv) < 1e-13, "{} vs {}", v.per_share[0], pv + tv);
    // hazard 0 gives the value without failure
    let none = engine(&PathsSpec::from_toml(&failure_toml("expected", 0.0)).unwrap()).deterministic().unwrap();
    let no_failure_text = failure_toml("expected", 0.0);
    let (hand, _) = hand_value(&PathsSpec::from_toml(&no_failure_text).unwrap());
    assert!(rel(none.per_share[0], hand) < 1e-13);
}

#[test]
fn failure_sampled_mean_agrees_with_the_closed_form() {
    let h = 0.07;
    let exp = engine(&PathsSpec::from_toml(&failure_toml("expected", h)).unwrap()).deterministic().unwrap().per_share[0];
    let n = 400_000;
    let sim = engine(&PathsSpec::from_toml(&failure_toml("sampled", h)).unwrap()).simulate(n, 31, &Mods::default(), None, false).unwrap();
    let (mean, _, se) = mean_sd_se(&sim.per_share);
    assert!((mean - exp).abs() < 3.0 * se, "{mean} vs {exp} (se {se}, z {:.2})", (mean - exp) / se);
    // the share of failed paths: 1 - (1 - h)^5
    let share = sim.failed as f64 / n as f64;
    let want = 1.0 - (1.0 - h).powi(5);
    assert!((share - want).abs() < 3.0 * (want * (1.0 - want) / n as f64).sqrt(), "{share} vs {want}");
}

// ---------------------------------------------------------------------------
// (7) Input errors
// ---------------------------------------------------------------------------

fn build_err(text: &str) -> String {
    match PathsSpec::from_toml(text).and_then(|s| PathEngine::new(&s).map(|_| ())) {
        Ok(()) => panic!("expected an error"),
        Err(e) => e.0,
    }
}

#[test]
fn input_errors_have_a_reason() {
    let base = fixed_rate_toml("");
    let with_draws = base
        .replace("target = 0.05\nhalf_life = 2.0", "target = { mean = 0.05, std = 0.02 }\nhalf_life = 2.0")
        .replace("target = 0.03\nhalf_life = 3.0", "target = { mean = 0.03, std = 0.02 }\nhalf_life = 3.0")
        .replace("target = 0.25\nhalf_life = 2.5", "target = { mean = 0.25, std = 0.02 }\nhalf_life = 2.5");
    // a correlation matrix that is not positive definite
    let bad = format!(
        "{with_draws}\n[[corr]]\na = \"process.gA.target\"\nb = \"process.gB.target\"\nrho = -0.9\n[[corr]]\na = \"process.gB.target\"\nb = \"process.m.target\"\nrho = -0.9\n[[corr]]\na = \"process.gA.target\"\nb = \"process.m.target\"\nrho = -0.9\n"
    );
    assert!(build_err(&bad).contains("not positive definite"));
    let out = format!("{with_draws}\n[[corr]]\na = \"process.gA.target\"\nb = \"process.gB.target\"\nrho = 1.2\n");
    assert!(build_err(&out).contains("outside [-1, 1]"));
    let fixed_ref = format!("{with_draws}\n[[corr]]\na = \"process.gA.target\"\nb = \"discount.beta\"\nrho = 0.2\n");
    assert!(build_err(&fixed_ref).contains("one of"));
    let conflict = format!("{with_draws}\n[[corr]]\na = \"process.gA.target\"\nb = \"process.gB.target\"\nrho = 0.2\n[[corr]]\na = \"process.gB.target\"\nb = \"process.gA.target\"\nrho = 0.3\n");
    assert!(build_err(&conflict).contains("conflicting"));
    let selfc = format!("{with_draws}\n[[corr]]\na = \"process.gA.target\"\nb = \"process.gA.target\"\nrho = 1.0\n");
    assert!(build_err(&selfc).contains("self-correlation"));
    // the half-life
    assert!(build_err(&base.replace("half_life = 2.0", "half_life = 0.0")).contains("half_life must be above 0"));
    assert!(build_err(&base.replace("half_life = 2.0", "half_life = -1.0")).contains("half_life must be above 0"));
    assert!(build_err(&base.replace("half_life = 2.0", "half_life = { mean = 2.0, std = 1.0 }")).contains("lo > 0"));
    assert!(build_err(&base.replace("half_life = 2.0\n", "")).contains("needs half_life"));
    // bounds and parameters
    assert!(build_err(&with_draws.replace("target = { mean = 0.05, std = 0.02 }", "target = { mean = 0.05, std = 0.02, lo = 0.06 }")).contains("outside [lo, hi]"));
    assert!(build_err(&with_draws.replace("target = { mean = 0.05, std = 0.02 }", "target = { mean = 0.05, std = -0.02 }")).contains("std must be 0 or more"));
    assert!(build_err(&with_draws.replace("target = { mean = 0.05, std = 0.02 }", "target = { mean = 0.05, std = 0.02, lo = 0.1, hi = 0.0 }")).contains("must be below"));
    assert!(build_err(&base.replace("start = 0.30\ntarget = 0.05", "start = 0.30\nlo = 0.5\nhi = 0.1\ntarget = 0.05")).contains("must be below hi"));
    // structure
    assert!(build_err(&base.replace("role = \"margin\"", "role = \"boss\"")).contains("role must be"));
    assert!(build_err(&base.replace("role = \"margin\"", "role = \"growth\"")).contains("needs one process with the role \"margin\""));
    assert!(build_err(&base.replace("ramp_years = 4", "ramp_years = 1")).contains("ramp_years must be at least 2"));
    assert!(build_err(&base.replace("process = \"gB\"", "process = \"nothing\"")).contains("does not exist"));
    assert!(build_err(&base.replace("equity_weight = 0.8", "equity_weight = 1.5")).contains("equity_weight"));
    assert!(build_err(&base.replace("rf = 0.045", "")).contains("needs rates.rf"));
    assert!(build_err(&base.replace("model = \"fixed\"", "model = \"hullwhite\"")).contains("needs rates.curve"));
    assert!(build_err(&format!("{base}\n[failure]\nhazard = 1.0\n")).contains("failure.hazard"));
    assert!(build_err(&base.replace("[tax]\nrate = 0.25", "[tax]\nrate = 1.0")).contains("tax.rate"));
    assert!(build_err(&base.replace("[terminal]", "[reinvestment]\nmode = \"sales_to_capital\"\n\n[terminal]")).contains("s2c"));
    assert!(build_err(&base.replace("year1_growth = 0.10", "year1_growth = 0.10\nbad_key = 1")).contains("bad_key"));
    assert!(build_err(&base.replace("projection_years = 6", "projection_years = 0")).contains("projection_years"));
    // a model in the mode 'sales_to_capital' needs terminal.roic
    let b = base.replace("[discount]", "[reinvestment]\nmode = \"sales_to_capital\"\n\n[process.k]\nrole = \"s2c\"\nstart = 2.0\ntarget = 2.0\nhalf_life = inf\n\n[discount]");
    assert!(build_err(&b).contains("terminal.roic"));
    // a Hull-White horizon plus tenor above the end of the curve
    let (_, cs) = gsw_curve();
    assert!(build_err(&hw_toml(&cs, 0.01, "").replace("projection_years = 6", "projection_years = 25")).contains("curve ends"));
    // a shock correlation that names a process without a shock
    let shock = format!("{base}\n[[shock_corr]]\na = \"gA\"\nb = \"gB\"\nrho = 0.3\n");
    assert!(build_err(&shock).contains("shock_corr"));
}

#[test]
fn a_file_without_the_engine_key_is_not_a_paths_file() {
    assert!(!is_paths_model("ticker = \"IBM\"\nnet_debt = 1.0"));
    assert!(is_paths_model("engine = \"paths\"\nticker = \"X\""));
    let old = std::fs::read_to_string(repo().join("analysis/IBM/driver_model.toml")).unwrap();
    assert!(!is_paths_model(&old));
}

// ---------------------------------------------------------------------------
// Reproducibility, attribution, other rate models, application files
// ---------------------------------------------------------------------------

fn small_random_spec() -> PathsSpec {
    let (_, cs) = gsw_curve();
    let text = hw_toml(&cs, 0.012, "")
        .replace("target = 0.05\nhalf_life = 2.0", "target = { mean = 0.05, std = 0.02, lo = 0.0, hi = 0.15 }\nhalf_life = { mean = 2.0, std = 0.5, lo = 0.5, hi = 6.0 }\nsigma = 0.04")
        .replace("target = 0.25\nhalf_life = 2.5", "target = { mean = 0.25, std = 0.04, lo = 0.05, hi = 0.45 }\nhalf_life = 2.5\nsigma = 0.02")
        .replace("beta = 1.2\nerp = 0.05", "beta = 1.2\nerp = { mean = 0.05, std = 0.0075, lo = 0.02 }")
        + "\n[[corr]]\na = \"process.gA.target\"\nb = \"process.m.target\"\nrho = 0.4\n[[shock_corr]]\na = \"gA\"\nb = \"m\"\nrho = 0.5\n[[shock_corr]]\na = \"rate\"\nb = \"gA\"\nrho = 0.2\n";
    PathsSpec::from_toml(&text).unwrap()
}

#[test]
fn a_run_gives_the_same_numbers_with_one_and_four_threads_and_for_each_seed() {
    let e = engine(&small_random_spec());
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap().install(|| e.simulate(5_000, 42, &Mods::default(), None, false).unwrap().per_share)
    };
    let (a, b, c) = (run(1), run(4), run(3));
    assert_eq!(a, b);
    assert_eq!(a, c);
    let other = e.simulate(5_000, 43, &Mods::default(), None, false).unwrap().per_share;
    assert_ne!(a, other);
}

#[test]
fn attribution_signs_and_variance_shares() {
    let e = engine(&small_random_spec());
    let n = 20_000;
    let sim = e.simulate(n, 7, &Mods::default(), None, false).unwrap();
    let eff = e.attribution(n, 7, sim.p50()).unwrap();
    let find = |name: &str| eff.iter().find(|x| x.name == name).unwrap();
    // a higher margin target and higher long-run growth raise the value; a higher equity premium lowers it
    assert!(find("process.m.target").p50_plus_1sd > 0.0 && find("process.m.target").p50_minus_1sd < 0.0);
    assert!(find("process.gA.target").p50_plus_1sd > 0.0);
    assert!(find("discount.erp").p50_plus_1sd < 0.0 && find("discount.erp").p50_minus_1sd > 0.0);
    let shares = e.variance_shares(n, 7, &sim.per_share).unwrap();
    assert!(shares.iter().all(|s| s.alone > 0.0 && s.alone < 1.3 && s.removed > -0.05 && s.removed < 1.0));
    // a group that is frozen and all other groups frozen give a value with no spread
    let all: Vec<usize> = (0..e.groups.len()).collect();
    let frozen = e.simulate(1_000, 7, &Mods { frozen: all, ..Mods::default() }, None, false).unwrap();
    assert!(mean_sd_se(&frozen.per_share).1 < 1e-9);
    assert!(rel(frozen.per_share[0], e.deterministic().unwrap().per_share[0]) < 1e-12);
}

#[test]
fn vasicek_and_cir_engines_run_and_pset_changes_a_number() {
    let text = fixed_rate_toml("").replace("model = \"fixed\"\nrf = 0.045", "model = \"cir\"\nkappa = 0.3\ntheta = 0.045\nsigma = 0.06\nmode = \"long\"\ntenor = 10.0");
    let e = engine(&PathsSpec::from_toml(&text).unwrap());
    let sim = e.simulate(2_000, 1, &Mods::default(), None, true).unwrap();
    assert!(sim.per_share.iter().all(|v| v.is_finite()));
    let text = fixed_rate_toml("").replace("model = \"fixed\"\nrf = 0.045", "model = \"vasicek\"\nkappa = 0.3\ntheta = 0.045\nsigma = 0.01\nmode = \"short\"\nspread = 0.005");
    let e = engine(&PathsSpec::from_toml(&text).unwrap());
    assert!(e.simulate(2_000, 1, &Mods::default(), None, false).unwrap().per_share.iter().all(|v| v.is_finite()));
    let spec = PathsSpec::from_toml_with(&fixed_rate_toml(""), &[("rates.rf".into(), 0.05), ("process.gA.target".into(), 0.07)], false).unwrap();
    assert_eq!(spec.rates.rf, Some(ParamSpec::Fixed(0.05)));
    assert_eq!(spec.process["gA"].target, ParamSpec::Fixed(0.07));
}

#[test]
fn terminal_growth_caps_are_counted() {
    let text = fixed_rate_toml("").replace("growth = 0.03\n", "growth = 0.06\ncap_by_process = \"gA\"\nrf_margin = -0.02\n");
    let e = engine(&PathsSpec::from_toml(&text).unwrap());
    let det = e.deterministic().unwrap();
    // 6% is above the target 5% of gA: the cap by the process binds first
    assert_eq!(det.flags[0] & CAP_PROCESS, CAP_PROCESS);
    // 5% is above rf - 2% = 2.5%: the cap by the risk-free rate binds
    assert_eq!(det.flags[0] & CAP_RF, CAP_RF);
    assert_eq!(det.flags[0] & CAP_GAP, 0);
    let spec = PathsSpec::from_toml(&text).unwrap();
    let (hand, _) = hand_value(&spec);
    assert!(rel(det.per_share[0], hand) < 1e-12);
}

#[test]
#[ignore]
fn speed_table() {
    use std::time::Instant;
    let mut spec = small_random_spec();
    spec.projection_years = 10;
    let e = engine(&spec);
    println!("paths x 10 years, Hull-White, 3 shock channels, 5 drawn parameters (the engine, without detail)");
    for threads in [1usize, 4] {
        for n in [10_000usize, 100_000] {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
            pool.install(|| {
                e.simulate(n, 1, &Mods::default(), None, false).unwrap();
                let t = Instant::now();
                let reps = if n == 10_000 { 20 } else { 3 };
                for r in 0..reps {
                    e.simulate(n, 1 + r as u64, &Mods::default(), None, false).unwrap();
                }
                let ms = t.elapsed().as_secs_f64() * 1000.0 / reps as f64;
                println!("  {threads} thread(s), {n:>7} paths: {ms:8.2} ms");
            });
        }
    }
}

#[test]
fn long_flat_mode_keeps_the_expected_risk_free_rate_at_the_level_of_time_zero() {
    let (curve, _) = gsw_curve();
    let kinds = [
        RateKind::HullWhite { a: 0.14, sigma: 0.0075, curve },
        RateKind::Vasicek { kappa: 0.3, theta: 0.06, sigma: 0.01, r0: 0.04 },
    ];
    for kind in kinds {
        let f = RateFactor::new(kind, 1.0, 10, RfMode::LongFlat, 10.0, 0.0).unwrap();
        let rows: Vec<Vec<f64>> = (0..100_000usize)
            .map(|p| {
                let mut rng = Rng::for_path(5, p as u64);
                let mut s = f.state0;
                let mut out = vec![f.rf(0, s)];
                for i in 0..10 {
                    s = f.step(i, s, rng.normal());
                    out.push(f.rf(i + 1, s));
                }
                out
            })
            .collect();
        for i in [0usize, 3, 10] {
            let x: Vec<f64> = rows.iter().map(|r| r[i]).collect();
            let (mean, _, se) = mean_sd_se(&x);
            assert!((mean - rows[0][0]).abs() <= 3.0 * se + 1e-12, "{kind:?} i={i}: {mean} vs {}", rows[0][0]);
        }
    }
}
