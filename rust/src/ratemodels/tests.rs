//! Tests: no network, fixed seeds. Tolerances are in standard errors (SE) and are stated at each test.

use super::calib::*;
use super::curve::ZeroCurve;
use super::data::{sample, Freq, Sample, Series, Span};
use super::models::*;
use super::rolling::{compare, frequencies, rolling, StudyInput};
use super::sim::*;
use mapcheck::dates;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn cfg(years: f64, spy: usize, paths: usize, seed: u64, r0: Option<f64>) -> SimConfig {
    SimConfig { years, steps_per_year: spy, paths, seed, r0 }
}

/// One long path (steps+1 values).
fn one_path<M: RateModel>(m: &M, years: f64, spy: usize, seed: u64, r0: Option<f64>) -> Vec<f64> {
    simulate_paths(m, &cfg(years, spy, 1, seed, r0)).unwrap().rates
}

fn mk_sample(r: Vec<f64>, dt: f64) -> Sample {
    let n = r.len() as i64;
    Sample { series: "sim".into(), freq: Freq::Weekly, days: (0..n).collect(), r, dt, warnings: vec![] }
}

fn business_series(values: Vec<f64>) -> Series {
    let mut days = Vec::new();
    let mut day = dates::parse("1990-01-01").unwrap(); // a Monday
    while days.len() < values.len() {
        if (day + 3).rem_euclid(7) < 5 {
            days.push(day);
        }
        day += 1;
    }
    Series::new("synthetic", days, values).unwrap()
}

/// Standard error of the sample variance, from the fourth moment.
fn var_se(x: &[f64]) -> f64 {
    let n = x.len() as f64;
    let m = x.iter().sum::<f64>() / n;
    let v = x.iter().map(|a| (a - m).powi(2)).sum::<f64>() / n;
    let m4 = x.iter().map(|a| (a - m).powi(4)).sum::<f64>() / n;
    ((m4 - v * v) / n).sqrt()
}

/// Moments and bond price against the closed forms. The tolerance is 4 SE; the bond price has
/// an extra 5e-5 for the trapezoid error of the integral (order dt^2).
fn check_closed_forms<M: RateModel>(m: &M, r0: f64, years: f64, spy: usize, paths: usize) {
    let c = cfg(years, spy, paths, 11, Some(r0));
    let steps = c.steps();
    let dt = c.dt();
    let out = simulate_map(m, &c, |_, p| (p[steps], (-trapezoid(p, dt, steps)).exp())).unwrap();
    let r: Vec<f64> = out.iter().map(|x| x.0).collect();
    let disc: Vec<f64> = out.iter().map(|x| x.1).collect();
    let (mean, _, se) = mean_sd_se(&r);
    let exact_mean = m.mean(r0, years).unwrap();
    assert!((mean - exact_mean).abs() < 4.0 * se, "{}: mean {mean} vs {exact_mean} (se {se})", m.name());
    if let Some(v) = m.variance(r0, years) {
        let mc_var = r.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / (r.len() - 1) as f64;
        let s = var_se(&r);
        assert!((mc_var - v).abs() < 4.0 * s, "{}: variance {mc_var} vs {v} (se {s})", m.name());
    }
    let (p, _, pse) = mean_sd_se(&disc);
    let cf = m.bond_price(r0, years).unwrap();
    assert!((p - cf).abs() < 4.0 * pse + 5e-5, "{}: bond {p} vs {cf} (se {pse})", m.name());
}

// ---------------------------------------------------------------------------
// models: closed forms
// ---------------------------------------------------------------------------

#[test]
fn vasicek_moments_and_bond_price() {
    let m = Vasicek { kappa: 0.4, theta: 0.05, sigma: 0.015 };
    check_closed_forms(&m, 0.02, 5.0, 52, 200_000);
    check_closed_forms(&m, 0.08, 1.0, 52, 200_000);
}

#[test]
fn cir_moments_and_bond_price_with_feller_satisfied() {
    let m = Cir { kappa: 0.5, theta: 0.05, sigma: 0.1 };
    assert!(m.feller_satisfied() && (m.feller_ratio() - 5.0).abs() < 1e-12);
    check_closed_forms(&m, 0.03, 5.0, 52, 200_000);
}

#[test]
fn cir_moments_and_bond_price_with_feller_violated_and_nonnegative() {
    let m = Cir { kappa: 0.3, theta: 0.04, sigma: 0.3 };
    assert!(!m.feller_satisfied() && m.feller_ratio() < 0.3);
    check_closed_forms(&m, 0.01, 5.0, 52, 200_000);
    let paths = simulate_paths(&m, &cfg(5.0, 52, 2000, 3, Some(0.01))).unwrap();
    assert!(paths.rates.iter().all(|r| *r >= 0.0), "a CIR rate is negative");
    assert!(paths.rates.iter().filter(|r| **r == 0.0).count() > 0, "the QE scheme should reach 0 when Feller fails");
}

#[test]
fn ckls_gamma_zero_is_vasicek_exactly_and_half_is_cir() {
    let v = Vasicek { kappa: 0.4, theta: 0.05, sigma: 0.015 };
    let k0 = Ckls { kappa: 0.4, theta: 0.05, sigma: 0.015, gamma: 0.0 };
    let (a, b) = (simulate_paths(&v, &cfg(2.0, 52, 50, 5, None)).unwrap(), simulate_paths(&k0, &cfg(2.0, 52, 50, 5, None)).unwrap());
    assert_eq!(a.rates, b.rates);
    assert_eq!(v.bond_price(0.03, 4.0), k0.bond_price(0.03, 4.0));
    // gamma = 1/2: Milstein scheme with dt = 1/500 against the CIR formulas; weak order 1
    let c = Cir { kappa: 0.5, theta: 0.05, sigma: 0.1 };
    let k5 = Ckls { kappa: 0.5, theta: 0.05, sigma: 0.1, gamma: 0.5 };
    assert_eq!(c.bond_price(0.03, 3.0), k5.bond_price(0.03, 3.0));
    let n = 100_000;
    let r: Vec<f64> = simulate_map(&k5, &cfg(3.0, 500, n, 8, Some(0.03)), |_, p| p[p.len() - 1]).unwrap();
    let (mean, _, se) = mean_sd_se(&r);
    assert!((mean - c.mean(0.03, 3.0).unwrap()).abs() < 4.0 * se, "mean {mean}");
    let var = r.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let exact = c.variance(0.03, 3.0).unwrap();
    // Milstein bias: allow 4 SE plus 1% of the variance
    assert!((var - exact).abs() < 4.0 * var_se(&r) + 0.01 * exact, "var {var} vs {exact}");
}

#[test]
fn ckls_other_gamma_has_no_closed_form_but_the_mean() {
    let m = Ckls { kappa: 0.5, theta: 0.05, sigma: 0.3, gamma: 1.0 };
    assert!(m.bond_price(0.05, 1.0).is_none() && m.variance(0.05, 1.0).is_none());
    let r: Vec<f64> = simulate_map(&m, &cfg(2.0, 252, 100_000, 2, Some(0.08)), |_, p| p[p.len() - 1]).unwrap();
    let (mean, _, se) = mean_sd_se(&r);
    assert!((mean - m.mean(0.08, 2.0).unwrap()).abs() < 4.0 * se + 2e-5, "mean {mean}");
}

fn svensson() -> ZeroCurve {
    ZeroCurve { beta0: 0.045, beta1: -0.02, beta2: 0.01, beta3: -0.015, tau1: 2.0, tau2: 8.0 }
}

#[test]
fn hull_white_and_ho_lee_reprice_the_initial_curve() {
    let curve = svensson();
    let hw = HullWhite { a: 0.1, sigma: 0.01, curve };
    let hl = HoLee { sigma: 0.01, curve };
    for t in [1.0, 5.0, 10.0] {
        assert!((hw.bond_price(hw.default_r0(), t).unwrap() - curve.discount(t)).abs() < 1e-14);
        check_closed_forms(&hw, hw.default_r0(), t, 52, 200_000);
        check_closed_forms(&hl, hl.default_r0(), t, 52, 200_000);
        // the Monte Carlo price against the curve itself
        let c = cfg(t, 52, 200_000, 21, None);
        let (steps, dt) = (c.steps(), c.dt());
        let d: Vec<f64> = simulate_map(&hw, &c, |_, p| (-trapezoid(p, dt, steps)).exp()).unwrap();
        let (p, _, se) = mean_sd_se(&d);
        assert!((p - curve.discount(t)).abs() < 4.0 * se + 5e-5, "t={t}: {p} vs {} (se {se})", curve.discount(t));
    }
}

#[test]
fn hull_white_mean_follows_alpha() {
    let hw = HullWhite { a: 0.3, sigma: 0.02, curve: svensson() };
    let r: Vec<f64> = simulate_map(&hw, &cfg(7.0, 52, 200_000, 4, None), |_, p| p[p.len() - 1]).unwrap();
    let (mean, _, se) = mean_sd_se(&r);
    assert!((mean - hw.mean(hw.default_r0(), 7.0).unwrap()).abs() < 4.0 * se);
    let mc_var = r.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / (r.len() - 1) as f64;
    assert!((mc_var - hw.variance(0.0, 7.0).unwrap()).abs() < 4.0 * var_se(&r));
}

#[test]
fn same_seed_gives_same_paths_with_one_and_four_threads() {
    let models = (Cir { kappa: 0.5, theta: 0.05, sigma: 0.1 }, HullWhite { a: 0.1, sigma: 0.01, curve: svensson() });
    let run = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        pool.install(|| (simulate_paths(&models.0, &cfg(3.0, 52, 500, 99, None)).unwrap().rates, simulate_paths(&models.1, &cfg(3.0, 52, 500, 99, None)).unwrap().rates))
    };
    let (a, b) = (run(1), run(4));
    assert_eq!(a.0, b.0);
    assert_eq!(a.1, b.1);
    let c = simulate_paths(&models.0, &cfg(3.0, 52, 500, 100, None)).unwrap();
    assert_ne!(a.0, c.rates);
}

#[test]
fn invalid_parameters_are_errors() {
    assert!(simulate_paths(&Vasicek { kappa: -1.0, theta: 0.05, sigma: 0.01 }, &cfg(1.0, 12, 10, 1, None)).is_err());
    assert!(simulate_paths(&Cir { kappa: 1.0, theta: 0.05, sigma: 0.0 }, &cfg(1.0, 12, 10, 1, None)).is_err());
    assert!(simulate_paths(&Ckls { kappa: 1.0, theta: 0.05, sigma: 0.1, gamma: 4.0 }, &cfg(1.0, 12, 10, 1, None)).is_err());
    assert!(Ckls::from_ab(0.01, 0.2, 0.1, 0.5).is_err());
    let k = Ckls::from_ab(0.025, -0.5, 0.1, 0.5).unwrap();
    assert!((k.theta - 0.05).abs() < 1e-15 && (k.kappa - 0.5).abs() < 1e-15);
    assert!(simulate_paths(&Vasicek { kappa: 1.0, theta: 0.05, sigma: 0.01 }, &cfg(1.0, 12, 0, 1, None)).is_err());
}

// ---------------------------------------------------------------------------
// CIR density
// ---------------------------------------------------------------------------

#[test]
fn cir_exact_density_integrates_to_one_and_has_the_right_mean() {
    for (kappa, theta, sigma, dt, r0) in [(0.5, 0.05, 0.1, 1.0 / 12.0, 0.04), (0.3, 0.04, 0.3, 1.0 / 252.0, 0.02), (2.0, 0.06, 0.2, 0.25, 0.1)] {
        // Use cir_loglik on two-point samples to read the density: r = [r0, x]
        let (mut total, mut mean) = (0.0, 0.0);
        let upper = theta + (r0 - theta) * f64::exp(-kappa * dt) + 12.0 * (cir_var(kappa, theta, sigma, r0, dt)).sqrt() + 0.05;
        let n = 200_000;
        let h = upper / n as f64;
        for i in 1..=n {
            let x = (i as f64 - 0.5) * h;
            let f = cir_loglik(&[r0, x], dt, kappa, theta, sigma).exp();
            total += f * h;
            mean += x * f * h;
        }
        let exact_mean = theta + (r0 - theta) * f64::exp(-kappa * dt);
        assert!((total - 1.0).abs() < 2e-4, "integral {total} for kappa {kappa} dt {dt}");
        assert!((mean - exact_mean).abs() < 2e-4 * exact_mean.max(0.01) + 1e-6, "mean {mean} vs {exact_mean}");
    }
}

fn cir_var(k: f64, th: f64, s: f64, r0: f64, t: f64) -> f64 {
    Cir { kappa: k, theta: th, sigma: s }.variance(r0, t).unwrap()
}

// ---------------------------------------------------------------------------
// calibration: recover known parameters
// ---------------------------------------------------------------------------

/// |estimate - truth| must be below 4 SE of the fit (SE from the fit itself) and below a fixed
/// relative cap, so that a wrong SE cannot hide a wrong estimate.
fn close(fit: &Fit, name: &str, truth: f64, rel_cap: f64) {
    let v = fit.get(name).unwrap();
    let se = fit.se(name).expect("a standard error");
    assert!((v - truth).abs() < 4.0 * se, "{} {} {name}: {v} vs {truth} (se {se})", fit.model, fit.method);
    assert!((v - truth).abs() < rel_cap * truth.abs(), "{} {} {name}: {v} vs {truth} (cap {rel_cap})", fit.model, fit.method);
}

#[test]
fn vasicek_methods_recover_parameters_at_two_sampling_steps() {
    // kappa = 3 makes the Euler map wrong by 12% at the monthly step: the exact map must remove it.
    let truth = Vasicek { kappa: 3.0, theta: 0.05, sigma: 0.02 };
    for (spy, years) in [(252usize, 400.0), (12, 3000.0)] {
        let r = one_path(&truth, years, spy, 17, None);
        let dt = 1.0 / spy as f64;
        let s = mk_sample(r.clone(), dt);
        for method in [Method::Ml, Method::Ols, Method::RealVol(VolEstimator::CloseToClose)] {
            let fit = calibrate(ModelKind::Vasicek, method, &s, 0.5).unwrap();
            close(&fit, "kappa", 3.0, 0.15);
            close(&fit, "theta", 0.05, 0.05);
            close(&fit, "sigma", 0.02, 0.03);
            assert_eq!(fit.mean_reversion_significant, Some(true));
        }
        if spy == 12 {
            // the first-order (Euler) estimate of kappa from the same data is clearly wrong
            let x = &r[..r.len() - 1];
            let y = &r[1..];
            let o = super::math::ols(x, y).unwrap();
            let euler_kappa = (1.0 - o.beta) / dt;
            assert!((euler_kappa - 3.0).abs() > 0.25, "Euler kappa {euler_kappa}");
        }
    }
}

#[test]
fn vasicek_ml_standard_errors_are_close_to_least_squares() {
    let truth = Vasicek { kappa: 1.0, theta: 0.05, sigma: 0.02 };
    let s = mk_sample(one_path(&truth, 200.0, 52, 5, None), 1.0 / 52.0);
    let ml = calibrate(ModelKind::Vasicek, Method::Ml, &s, 0.5).unwrap();
    let ols = calibrate(ModelKind::Vasicek, Method::Ols, &s, 0.5).unwrap();
    for p in ["kappa", "theta"] {
        let (a, b) = (ml.se(p).unwrap(), ols.se(p).unwrap());
        assert!((a / b - 1.0).abs() < 0.2, "{p}: se {a} vs {b}");
        assert!((ml.get(p).unwrap() - ols.get(p).unwrap()).abs() < 1e-9);
    }
    assert!(ml.loglik.unwrap().is_finite() && (ml.aic.unwrap() - (6.0 - 2.0 * ml.loglik.unwrap())).abs() < 1e-9);
}

#[test]
fn cir_methods_recover_parameters_at_two_sampling_steps() {
    let truth = Cir { kappa: 1.5, theta: 0.05, sigma: 0.12 };
    for (spy, years) in [(52usize, 250.0), (12, 600.0)] {
        let s = mk_sample(one_path(&truth, years, spy, 29, None), 1.0 / spy as f64);
        for method in [Method::Ml, Method::Ols, Method::RealVol(VolEstimator::CloseToClose)] {
            let fit = calibrate(ModelKind::Cir, method, &s, 0.5).unwrap();
            close(&fit, "kappa", 1.5, 0.25);
            close(&fit, "theta", 0.05, 0.08);
            close(&fit, "sigma", 0.12, 0.05);
            assert!(fit.feller_ratio.unwrap() > 1.0);
        }
    }
}

#[test]
fn ckls_gmm_and_quasi_ml_recover_gamma() {
    // simulate CIR (gamma = 1/2) and also gamma = 1 (fine Milstein path, then take every 5th point)
    let cir = Cir { kappa: 1.5, theta: 0.05, sigma: 0.12 };
    let s = mk_sample(one_path(&cir, 1500.0, 52, 31, None), 1.0 / 52.0);
    for method in [Method::Gmm, Method::Ml] {
        let fit = calibrate(ModelKind::Ckls, method, &s, 0.5).unwrap();
        close(&fit, "gamma", 0.5, 0.3);
        close(&fit, "kappa", 1.5, 0.15);
        close(&fit, "theta", 0.05, 0.05);
        assert!(fit.get("sigma").unwrap() > 0.0);
    }
    let bs = Ckls { kappa: 1.0, theta: 0.05, sigma: 0.6, gamma: 1.0 };
    let fine = one_path(&bs, 1000.0, 260, 33, None);
    let coarse: Vec<f64> = fine.iter().step_by(5).copied().collect();
    let s = mk_sample(coarse, 5.0 / 260.0);
    let fit = calibrate(ModelKind::Ckls, Method::Gmm, &s, 0.5).unwrap();
    close(&fit, "gamma", 1.0, 0.25);
    close(&fit, "kappa", 1.0, 0.3);
}

#[test]
fn realized_volatility_estimators_follow_a_change_of_regime() {
    let (dt, spy) = (1.0 / 252.0, 252);
    let first = Vasicek { kappa: 0.5, theta: 0.05, sigma: 0.01 };
    let second = Vasicek { kappa: 0.5, theta: 0.05, sigma: 0.02 };
    let mut r = one_path(&first, 20.0, spy, 41, None);
    let last = *r.last().unwrap();
    r.extend(one_path(&second, 20.0, spy, 42, Some(last)).into_iter().skip(1));
    let s = mk_sample(r, dt);
    let sig = |m| calibrate(ModelKind::Vasicek, Method::RealVol(m), &s, 0.5).unwrap().get("sigma").unwrap();
    let all = sig(VolEstimator::CloseToClose);
    let expected = ((0.01f64.powi(2) + 0.02f64.powi(2)) / 2.0).sqrt();
    assert!((all / expected - 1.0).abs() < 0.04, "close-to-close {all} vs {expected}");
    let rolling = sig(VolEstimator::Rolling(500));
    assert!((rolling / 0.02 - 1.0).abs() < 0.15, "rolling {rolling}"); // 4 SE of 500 changes: 4/sqrt(1000)
    let ewma = sig(VolEstimator::Ewma(0.97));
    assert!((ewma / 0.02 - 1.0).abs() < 0.35, "ewma {ewma}"); // n_eff = 65: 4 SE = 35%
    // the plain estimate of the realized volatility of the changes is close to the sigma of the model
    let fit = calibrate(ModelKind::Vasicek, Method::RealVol(VolEstimator::Rolling(500)), &s, 0.5).unwrap();
    assert!((fit.realized_vol_estimator.unwrap() / 0.02 - 1.0).abs() < 0.15);
    assert!((fit.realized_vol / expected - 1.0).abs() < 0.05);
    // estimator settings that are not possible
    assert!(calibrate(ModelKind::Vasicek, Method::RealVol(VolEstimator::Rolling(10_000_000)), &s, 0.5).is_err());
    assert!(VolEstimator::parse("ewma:1.5").is_err() && VolEstimator::parse("garch").is_err());
}

#[test]
fn ho_lee_and_hull_white_calibration() {
    let truth = Vasicek { kappa: 0.5, theta: 0.05, sigma: 0.01 };
    let s = mk_sample(one_path(&truth, 100.0, 252, 6, None), 1.0 / 252.0);
    let hw = calibrate(ModelKind::HullWhite, Method::Ml, &s, 0.5).unwrap();
    close(&hw, "a", 0.5, 0.5);
    close(&hw, "sigma", 0.01, 0.03);
    assert!(hw.get("theta").is_none() && hw.warnings.iter().any(|w| w.contains("zero curve")));
    // a random walk has the realized volatility as sigma
    let mut r = vec![0.05];
    let mut g = super::rng::Rng::for_path(5, 5);
    for _ in 0..20_000 {
        let l = *r.last().unwrap();
        r.push(l + 0.008 * (1.0 / 252.0f64).sqrt() * g.normal());
    }
    let fit = calibrate(ModelKind::HoLee, Method::Ml, &mk_sample(r, 1.0 / 252.0), 0.5).unwrap();
    close(&fit, "sigma", 0.008, 0.03);
    assert!(fit.mean_reversion_t.is_none() && fit.n_params_aic == 2);
    assert!(calibrate(ModelKind::HoLee, Method::Ols, &mk_sample(vec![0.01; 40], 0.1), 0.5).is_err());
}

// ---------------------------------------------------------------------------
// edge cases
// ---------------------------------------------------------------------------

#[test]
fn edge_cases_give_errors_or_flags() {
    // too short
    let s = business_series((0..20).map(|i| 0.05 + 0.0001 * i as f64).collect());
    assert!(sample(&s, None, None, Freq::Daily).unwrap_err().contains("at least 30"));
    // constant
    let flat = mk_sample(vec![0.05; 200], 1.0 / 52.0);
    for kind in [ModelKind::Vasicek, ModelKind::Cir, ModelKind::Ckls, ModelKind::HoLee] {
        let e = calibrate(kind, if kind == ModelKind::Ckls { Method::Gmm } else { Method::Ml }, &flat, 0.5).unwrap_err();
        assert!(e.contains("constant") || e.contains("zero"), "{kind:?}: {e}");
    }
    // negative rates with CIR and CKLS
    let neg = Vasicek { kappa: 1.0, theta: 0.0, sigma: 0.01 };
    let s = mk_sample(one_path(&neg, 20.0, 52, 3, None), 1.0 / 52.0);
    assert!(s.r.iter().any(|r| *r <= 0.0));
    for kind in [ModelKind::Cir, ModelKind::Ckls] {
        let e = calibrate(kind, Method::Ml, &s, 0.5).unwrap_err();
        assert!(e.contains("positive rates"), "{e}");
    }
    assert!(calibrate(ModelKind::Vasicek, Method::Ml, &s, 0.5).is_ok());
    // unit root: an explosive series has beta above 1
    let mut g = super::rng::Rng::for_path(8, 8);
    let mut r = vec![0.02];
    for _ in 0..500 {
        let l = *r.last().unwrap();
        r.push(l * 1.002 + 1e-4 * g.normal());
    }
    let e = calibrate(ModelKind::Vasicek, Method::Ml, &mk_sample(r.clone(), 1.0 / 252.0), 0.5).unwrap_err();
    assert!(e.contains("no mean reversion") && e.contains("unit root"), "{e}");
    let e = calibrate(ModelKind::Cir, Method::Ml, &mk_sample(r, 1.0 / 252.0), 0.5);
    assert!(e.is_ok() || e.unwrap_err().contains("mean reversion")); // ML may find a weak kappa; never a silent crash
    // weak mean reversion is flagged, not hidden
    let weak = Vasicek { kappa: 0.05, theta: 0.05, sigma: 0.01 };
    let fit = calibrate(ModelKind::Vasicek, Method::Ml, &mk_sample(one_path(&weak, 15.0, 252, 12, None), 1.0 / 252.0), 0.5);
    if let Ok(f) = fit {
        assert_eq!(f.mean_reversion_significant, Some(false), "t = {:?}", f.mean_reversion_t);
        assert!(f.warnings.iter().any(|w| w.contains("not different from zero")));
        assert!(f.se("kappa").unwrap() > 0.5 * f.get("kappa").unwrap().abs());
    }
    // the method gmm is only for ckls
    assert!(calibrate(ModelKind::Vasicek, Method::Gmm, &mk_sample(vec![0.01, 0.02, 0.015, 0.012, 0.02, 0.01, 0.015, 0.02, 0.011, 0.018, 0.02, 0.01, 0.012, 0.018, 0.02, 0.014, 0.011, 0.019, 0.02, 0.012], 0.1), 0.5).is_err());
}

// ---------------------------------------------------------------------------
// intervals, frequencies, rolling, comparison
// ---------------------------------------------------------------------------

/// An exact Vasicek series on business days with `n` steps. The step is 7/(5*365.25) years (one
/// business day on average), so that the calendar step of the sample equals the step of the model.
fn calendar_vasicek(n: usize, seed: u64) -> Series {
    let dt = 7.0 / (5.0 * 365.25);
    let m = Vasicek { kappa: 1.5, theta: 0.05, sigma: 0.015 };
    let e = (-m.kappa * dt).exp();
    let sd = (m.sigma * m.sigma * (1.0 - (-2.0 * m.kappa * dt).exp()) / (2.0 * m.kappa)).sqrt();
    let mut g = super::rng::Rng::for_path(seed, 0);
    let mut r = vec![0.05];
    for _ in 0..n {
        let l = *r.last().unwrap();
        r.push(m.theta + (l - m.theta) * e + sd * g.normal());
    }
    business_series(r)
}

#[test]
fn frequencies_give_comparable_annualized_parameters() {
    let series = calendar_vasicek(125_000, 77); // about 480 years of business days
    let inp = StudyInput { series: &series, kind: ModelKind::Vasicek, method: Method::Ml, gamma: 0.5, start: None, end: None, freq: Freq::Daily };
    let t = frequencies(&inp);
    assert_eq!(t.rows.len(), 3);
    let col = |name: &str| t.columns.iter().position(|c| c == name).unwrap();
    for row in &t.rows {
        let k = row[col("kappa")].as_f64().unwrap();
        let sg = row[col("sigma")].as_f64().unwrap();
        let th = row[col("theta")].as_f64().unwrap();
        // 480 years: SE(kappa) is about sqrt(2 * 1.5 / 480) = 0.08; 4 SE
        assert!((k - 1.5).abs() < 0.32 && (sg / 0.015 - 1.0).abs() < 0.03 && (th - 0.05).abs() < 0.01, "{row:?}");
    }
    assert!(t.to_csv().lines().count() == 4);
}

#[test]
fn rolling_windows_and_csv() {
    let series = calendar_vasicek(2_700, 78); // about 10.3 years
    let inp = StudyInput { series: &series, kind: ModelKind::Vasicek, method: Method::Ml, gamma: 0.5, start: None, end: None, freq: Freq::Weekly };
    let t = rolling(&inp, Span::parse("5y").unwrap(), Span::parse("1y").unwrap()).unwrap();
    assert!(t.rows.len() == 6 || t.rows.len() == 5, "{}", t.rows.len());
    assert_eq!(t.rows[0][0], "1990-01-01");
    assert_eq!(t.rows[1][0], "1991-01-01");
    let csv = t.to_csv();
    assert!(csv.starts_with("window_start,window_end,n_transitions,kappa,kappa_se,theta,theta_se,sigma,sigma_se,"));
    assert_eq!(csv.lines().count(), t.rows.len() + 1);
    // a window longer than the data gives an error, not an empty table
    assert!(rolling(&inp, Span::parse("50y").unwrap(), Span::parse("1y").unwrap()).is_err());
    // a window too short for the frequency gives a row with an error text
    let short = rolling(&inp, Span::parse("3m").unwrap(), Span::parse("2y").unwrap()).unwrap();
    let ecol = short.columns.iter().position(|c| c == "error").unwrap();
    assert!(short.rows.iter().all(|r| r[ecol].as_str().is_some_and(|e| e.contains("at least 30"))));
}

#[test]
fn compare_ranks_models_by_aic() {
    let series = calendar_vasicek(30_000, 79);
    let inp = StudyInput { series: &series, kind: ModelKind::Vasicek, method: Method::Ml, gamma: 0.5, start: None, end: None, freq: Freq::Weekly };
    let rows = compare(&inp);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].delta_aic, Some(0.0));
    assert!(rows.windows(2).all(|w| w[0].delta_aic.unwrap_or(f64::INFINITY) <= w[1].delta_aic.unwrap_or(f64::INFINITY)));
    // the data come from a Vasicek model: the random walk (holee) must be worse
    let hl = rows.iter().find(|r| r.model == "holee").unwrap();
    assert!(hl.delta_aic.unwrap() > 10.0);
}
