//! Tests of the estimators on synthetic panels with known parameters (fixed seed). No network and no cache.

use super::calib_bands::{ar1_band, band_observations, evaluate, GroupParams, HORIZON};
use super::calibrate::*;
use super::nprandom::Pcg64;
use super::panel::{clean_series, CompanyPanel, RawYear, RuleCounts};

/// A panel of `units` companies with `t` observations each, `y_t = mu_i + phi (y_{t-1} - mu_i) + sigma e_t`,
/// with `mu_i ~ N(mu, mu_sd^2)`. A second series has shocks with the correlation `rho` to the first.
struct Synth {
    a: Vec<Pair>,
    b: Vec<Pair>,
}

#[allow(clippy::too_many_arguments)]
fn synth(units: usize, t: usize, phi: f64, mu: f64, mu_sd: f64, sigma: f64, rho: f64, seed: u64) -> Synth {
    let mut rng = Pcg64::new(seed);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for u in 0..units {
        let mu_i = mu + mu_sd * rng.standard_normal();
        let stationary = sigma / (1.0 - phi * phi).sqrt();
        let (mut ya, mut yb) = (mu_i + stationary * rng.standard_normal(), mu_i + stationary * rng.standard_normal());
        let mut hist_a = vec![ya];
        let mut hist_b = vec![yb];
        for k in 1..=t {
            let (z1, z2) = (rng.standard_normal(), rng.standard_normal());
            let e1 = z1;
            let e2 = rho * z1 + (1.0 - rho * rho).sqrt() * z2;
            let (xa, xb) = (ya, yb);
            ya = mu_i + phi * (ya - mu_i) + sigma * e1;
            yb = mu_i + phi * (yb - mu_i) + sigma * e2;
            hist_a.push(ya);
            hist_b.push(yb);
            let w = |h: &Vec<f64>| if k >= 2 { Some(h[k - 2]) } else { None };
            a.push(Pair { unit: u as u32, year: 2010 + k as i32, x: xa, y: ya, w: w(&hist_a), size: 1, template: 0 });
            b.push(Pair { unit: u as u32, year: 2010 + k as i32, x: xb, y: yb, w: w(&hist_b), size: 1, template: 0 });
        }
    }
    Synth { a, b }
}

#[test]
fn the_nickell_bias_has_the_known_size_and_the_correction_inverts_it() {
    let b = nickell_bias(0.5, 9.0);
    assert!((b + 0.1812).abs() < 2e-3, "{b}");
    // no bias for a long panel, and none for no persistence at large T
    assert!(nickell_bias(0.5, 200.0).abs() < 0.01);
    let phi_fe = 0.5 + b;
    let back = nickell_correct(phi_fe, &[9; 50]);
    assert!((back - 0.5).abs() < 1e-6, "{back}");
}

#[test]
fn a_short_panel_with_company_means_gives_the_known_values_with_the_bias_correction() {
    // 10 years of growth: 500 companies, 9 pairs each (the first value is the start)
    let (phi, mu, sigma) = (0.5, 0.08, 0.10);
    let s = synth(500, 9, phi, mu, 0.05, sigma, 0.0, 11);
    let f = fit_ar1(&s.a).unwrap();
    assert_eq!((f.n_obs, f.n_units), (4500, 500));
    // the within estimate is too low by about 0.17 (Nickell), the correction brings it back
    assert!(f.fe.phi < phi - 0.10, "fe {}", f.fe.phi);
    assert!((f.fe_corrected.phi - phi).abs() < 0.06, "corrected {} (se {})", f.fe_corrected.phi, f.fe_corrected.phi_se);
    // the pooled estimate is too high: the company means are in it
    assert!(f.pooled.phi > phi + 0.04 && f.pooled.phi < 0.62, "pooled {} (theory 0.57)", f.pooled.phi);
    // Anderson-Hsiao has no bias but a larger error
    let ah = f.ah.unwrap();
    assert!((ah.phi - phi).abs() < 0.12, "ah {} (se {})", ah.phi, ah.phi_se);
    assert!((f.fe_corrected.mean - mu).abs() < 0.01, "mean {}", f.fe_corrected.mean);
    assert!((f.fe_corrected.sigma - sigma).abs() < 0.12 * sigma, "sigma {}", f.fe_corrected.sigma);
    assert!(f.fe_corrected.phi_se > 0.0 && f.fe_corrected.phi_se < 0.06);
    // the spread of the company means (0.05) is found, to a rough value
    assert!((f.mean_sd_units - 0.05).abs() < 0.015, "tau {}", f.mean_sd_units);
}

#[test]
fn the_pooled_estimate_is_right_when_the_companies_have_one_mean() {
    let (phi, mu, sigma) = (0.3, 0.06, 0.12);
    let s = synth(500, 9, phi, mu, 0.0, sigma, 0.0, 5);
    let f = fit_ar1(&s.a).unwrap();
    assert!((f.pooled.phi - phi).abs() < 0.04, "{}", f.pooled.phi);
    assert!((f.pooled.mean - mu).abs() < 0.01, "{}", f.pooled.mean);
    assert!((f.pooled.sigma - sigma).abs() < 0.1 * sigma, "{}", f.pooled.sigma);
    // the standard error has the right order: the error of 0.04 is within 3 standard errors
    assert!(f.pooled.phi_se > 0.005 && f.pooled.phi_se < 0.03);
    assert!((f.mean_se - 0.002).abs() < 0.002 || f.mean_se < 0.01);
}

#[test]
fn the_shock_correlation_is_recovered() {
    let s = synth(500, 9, 0.4, 0.05, 0.03, 0.1, 0.4, 21);
    let (fa, fb) = (fit_ar1(&s.a).unwrap(), fit_ar1(&s.b).unwrap());
    let ra = fe_residuals(&s.a, fa.fe_corrected.phi);
    let rb = fe_residuals(&s.b, fb.fe_corrected.phi);
    let (rho, se, n) = shock_correlation(&ra, &rb, 100).unwrap();
    assert_eq!(n, 4500);
    assert!((rho - 0.4).abs() < 0.07, "rho {rho} se {se}");
    assert!(se > 0.005 && se < 0.05);
    // no correlation in the data: near zero
    let z = synth(500, 9, 0.4, 0.05, 0.03, 0.1, 0.0, 22);
    let (ra, rb) = (fe_residuals(&z.a, 0.4), fe_residuals(&z.b, 0.4));
    let (rho0, _, _) = shock_correlation(&ra, &rb, 50).unwrap();
    assert!(rho0.abs() < 0.07, "{rho0}");
}

#[test]
fn half_life_is_ln2_over_minus_ln_phi() {
    assert!((half_life(0.5) - 1.0).abs() < 1e-12);
    assert!((half_life(0.9) - 6.5788).abs() < 1e-3);
    assert_eq!(half_life(-0.1), 0.0);
    assert_eq!(half_life(1.2), 99.0);
}

/// A panel from synthetic revenue (a growth AR(1) with one mean), through the panel rules to the table.
fn synthetic_companies(n: usize, years: usize, phi: f64, mu: f64, sigma: f64, seed: u64) -> Vec<CompanyPanel> {
    let mut rng = Pcg64::new(seed);
    let mut out = Vec::new();
    for i in 0..n {
        let mut g = mu + sigma / (1.0 - phi * phi).sqrt() * rng.standard_normal();
        let mut revenue = 2e9;
        let mut raws = Vec::new();
        for y in 0..years {
            raws.push(RawYear {
                end: format!("{}-12-31", 2005 + y),
                filed: format!("{}-02-15", 2006 + y),
                revenue: Some(revenue),
                operating_income: Some(0.15 * revenue),
                operating_cash_flow: Some(0.2 * revenue),
                capex: Some(-0.05 * revenue),
                ..Default::default()
            });
            g = mu + phi * (g - mu) + sigma * rng.standard_normal();
            g = g.clamp(-0.5, 0.55);
            revenue *= 1.0 + g;
        }
        let mut counts = RuleCounts::default();
        let rows = clean_series(raws, &mut counts);
        out.push(CompanyPanel { ticker: format!("S{i}"), name: String::new(), template: "standard".into(), rows });
    }
    out
}

#[test]
fn the_table_of_a_synthetic_panel_has_the_known_growth_parameters() {
    let companies = synthetic_companies(600, 12, 0.3, 0.07, 0.10, 3);
    let table = estimate_table(&companies, 20);
    let all = table.iter().find(|c| c.size == "all" && c.template == "all").unwrap();
    let g = all.growth.as_ref().unwrap();
    assert!((g.phi_pooled - 0.3).abs() < 0.05, "pooled {}", g.phi_pooled);
    assert!((g.phi - 0.3).abs() < 0.06, "corrected {}", g.phi);
    assert!((g.mean - 0.07).abs() < 0.01 && (g.sigma - 0.10).abs() < 0.015, "{} {}", g.mean, g.sigma);
    // constant margins: the margin cells exist and the margin has no shock
    let om = all.operating_margin.as_ref().unwrap();
    assert!(om.sigma < 1e-6 && (om.mean - 0.15).abs() < 1e-9);
    // priors_for: the size bucket of 3B is 1-10B; an unknown template falls back to the size cell
    let table = ProcessTable {
        meta: TableMeta { date: "2026-11-19".into(), command: "test".into(), companies: 600, company_years: 7200, growth_observations: 6600, note: String::new() },
        cell: table,
    };
    let p = table.priors_for(3e9, "standard", None).unwrap();
    assert_eq!(p.size, "1-10B");
    let p2 = table.priors_for(3e9, "no such template", Some("energy")).unwrap();
    assert_eq!((p2.size.as_str(), p2.template.as_str()), ("1-10B", "all"));
    // the table is TOML and reads back
    let text = table.to_toml().unwrap();
    let back = ProcessTable::from_toml(&text).unwrap();
    assert_eq!(back.cell.len(), table.cell.len());
    assert!((back.cell[0].growth.as_ref().unwrap().phi - table.cell[0].growth.as_ref().unwrap().phi).abs() < 1e-12);
}

#[test]
fn bands_from_the_true_ar1_have_80_percent_coverage() {
    let (phi, mu, sigma) = (0.3, 0.07, 0.10);
    let companies = synthetic_companies(500, 14, phi, mu, sigma, 9);
    let obs = band_observations(&companies);
    assert!(obs.len() > 2500, "{}", obs.len());
    let truth = GroupParams { phi_pooled: phi, mean_pooled: mu, sigma_pooled: sigma, phi_fe: phi, mean_fe: mu, sigma_fe: sigma, tau: 0.0 };
    let mut params = [None; 5];
    params[1] = Some(truth);
    let cov = evaluate("test", &obs, &params, 1500);
    // method 1 (pooled AR(1) with the true values)
    let c = cov.get(0, 1, 5, 0);
    let n: usize = c.iter().sum();
    let inside = c[1] as f64 / n as f64;
    assert!((inside - 0.80).abs() < 0.04, "inside {inside} of {n}");
    assert!(c[0] as f64 / n as f64 > 0.05 && c[2] as f64 / n as f64 > 0.05);
    // the 3-year band keeps the coverage
    let c3 = cov.get(0, 1, 5, 2);
    let inside3 = c3[1] as f64 / c3.iter().sum::<usize>() as f64;
    assert!((inside3 - 0.80).abs() < 0.04, "{inside3}");
    // the legacy rule (one growth draw for all years, std of 4 values) is too narrow here
    let l = cov.get(0, 0, 5, 0);
    assert!((l[1] as f64 / l.iter().sum::<usize>() as f64) < 0.76);
}

#[test]
fn an_ar1_band_widens_with_the_horizon_and_the_start_moves_the_first_year() {
    let mut rng = Pcg64::new(1);
    let z: Vec<[f64; HORIZON]> = (0..4000).map(|_| [rng.standard_normal(), rng.standard_normal(), rng.standard_normal()]).collect();
    let low = ar1_band(100.0, 0.0, 0.05, 0.5, 0.1, &z);
    let high = ar1_band(100.0, 0.6, 0.05, 0.5, 0.1, &z);
    assert!(high[0].0 > low[0].0 && high[0].1 > low[0].1);
    assert!((low[2].1 - low[2].0) > (low[0].1 - low[0].0));
}
