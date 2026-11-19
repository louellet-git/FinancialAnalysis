//! Tests of the DCF module. The expected values come from the Python engines (numpy 2.5.3),
//! recorded into `rust/tests/fixtures/dcf/*.json` before the Rust code was written, and from
//! exact closed-form calculations. No test needs Python or the network.

use super::classic::{derive_fcf_margin, run_classic, Company, DcfAssumptions};
use super::driver::{CorrEntry, Driver, DriverModel, Ramp, SegmentSpec};
use super::model::ModelSpec;
use super::nprandom::Pcg64;
use super::scenarios::{capital_structure_for, default_scenarios, scenarios_for, SCENARIO_SEED};
use super::stats::{cholesky, min_eigenvalue, percentile};
use super::verdict::{chance_price_too_high, verdict_band};
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/dcf/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap()
}

fn hex(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}

/// Relative difference of two numbers (absolute below 1).
fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1.0)
}

#[track_caller]
fn close(a: f64, b: f64, tol: f64, what: &str) {
    assert!(rel(a, b) <= tol, "{what}: rust {a:?} python {b:?} rel {:e}", rel(a, b));
}

// ---------------------------------------------------------------------------
// Random numbers: bit for bit
// ---------------------------------------------------------------------------

#[test]
fn raw_stream_matches_numpy() {
    let f = fixture("random.json");
    for (seed, e) in f["streams"].as_object().unwrap() {
        let mut rng = Pcg64::new(seed.parse().unwrap());
        for (i, want) in e["raw_u64"].as_array().unwrap().iter().enumerate() {
            let want: u64 = want.as_str().unwrap().parse().unwrap();
            assert_eq!(rng.next_u64(), want, "seed {seed} raw {i}");
        }
    }
}

#[test]
fn standard_normal_matches_numpy_bit_for_bit() {
    let f = fixture("random.json");
    for (seed, e) in f["streams"].as_object().unwrap() {
        let mut rng = Pcg64::new(seed.parse().unwrap());
        let got = rng.standard_normal_vec(1000);
        for (i, want) in e["normal_1000"].as_array().unwrap().iter().enumerate() {
            assert_eq!(got[i].to_bits(), hex(want).to_bits(), "seed {seed} value {i}");
        }
    }
}

#[test]
fn matrix_fill_is_row_major_and_matches() {
    let f = fixture("random.json");
    for (seed, e) in f["streams"].as_object().unwrap() {
        let m = Pcg64::new(seed.parse().unwrap()).standard_normal_matrix(3, 500);
        let want = e["matrix_3x500"].as_array().unwrap();
        for r in 0..3 {
            for c in 0..500 {
                assert_eq!(m[r][c].to_bits(), hex(&want[r * 500 + c]).to_bits(), "seed {seed} [{r}][{c}]");
            }
        }
    }
}

#[test]
fn normal_is_mean_plus_std_times_standard_normal() {
    let f = fixture("random.json");
    for (seed, e) in f["streams"].as_object().unwrap() {
        let got = Pcg64::new(seed.parse().unwrap()).normal_vec(0.1, 0.02, 300);
        for (i, want) in e["normal_mean_std_300"].as_array().unwrap().iter().enumerate() {
            assert_eq!(got[i].to_bits(), hex(want).to_bits(), "seed {seed} value {i}");
        }
    }
}

#[test]
fn long_stream_covers_the_wedge_and_the_tail() {
    let f = fixture("random.json");
    for (seed, e) in f["streams"].as_object().unwrap() {
        let big = &e["big_200k"];
        let v = Pcg64::new(seed.parse().unwrap()).standard_normal_vec(200_000);
        let (mut xor, mut sum, mut tails) = (0u64, 0u64, 0);
        for x in &v {
            xor ^= x.to_bits();
            sum = sum.wrapping_add(x.to_bits());
            if x.abs() > super::nprandom::ZIGGURAT_NOR_R {
                tails += 1;
            }
        }
        assert_eq!(xor.to_string(), big["xor"].as_str().unwrap(), "seed {seed} xor");
        assert_eq!(sum.to_string(), big["wrapping_sum"].as_str().unwrap(), "seed {seed} sum");
        assert_eq!(tails, big["tail_count"].as_i64().unwrap(), "seed {seed} tail count");
        assert!(tails > 0, "the test must reach the tail branch");
        let max = v.iter().cloned().fold(f64::MIN, f64::max);
        assert_eq!(max.to_bits(), hex(&big["max_hex"]).to_bits());
    }
}

#[test]
fn same_seed_same_stream_and_seeds_differ() {
    let a = Pcg64::new(7).standard_normal_vec(50);
    let b = Pcg64::new(7).standard_normal_vec(50);
    let c = Pcg64::new(8).standard_normal_vec(50);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn percentile_matches_numpy_linear() {
    let f = fixture("random.json");
    for (name, c) in f["percentile"].as_object().unwrap() {
        let values: Vec<f64> = c["values"].as_array().unwrap().iter().map(hex).collect();
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        for (q, want) in c["q"].as_array().unwrap().iter().zip(c["result"].as_array().unwrap()) {
            let q = q.as_f64().unwrap();
            assert_eq!(percentile(&sorted, q).to_bits(), hex(want).to_bits(), "{name} q={q}");
        }
    }
}

#[test]
fn cholesky_matches_numpy_and_rejects_non_pd() {
    let f = fixture("random.json");
    for name in ["c3", "ibm8"] {
        let c = &f["cholesky"][name];
        let m: Vec<Vec<f64>> = c["matrix"].as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(num).collect()).collect();
        let want: Vec<Vec<f64>> = c["lower"].as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(num).collect()).collect();
        let got = cholesky(&m).unwrap();
        for i in 0..m.len() {
            for j in 0..m.len() {
                assert!((got[i][j] - want[i][j]).abs() < 1e-14, "{name}[{i}][{j}]");
            }
        }
    }
    let bad: Vec<Vec<f64>> = f["cholesky"]["not_pd"]["matrix"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_array().unwrap().iter().map(num).collect())
        .collect();
    assert!(cholesky(&bad).is_err());
    // eigenvalues of [[1,.9,-.9],[.9,1,.9],[-.9,.9,1]]: 1+0.9*(-1)... the minimum is 1 - 1.8 = -0.8
    assert!((min_eigenvalue(&bad) - (-0.8)).abs() < 1e-9, "{}", min_eigenvalue(&bad));
}

// ---------------------------------------------------------------------------
// Driver engine
// ---------------------------------------------------------------------------

fn spec_from(v: &Value) -> ModelSpec {
    serde_json::from_value(v.clone()).unwrap()
}

fn check_driver_result(spec: &ModelSpec, want: &Value, tag: &str) {
    let eng = spec.engine().unwrap();
    let n = want["n"].as_u64().unwrap() as usize;
    let seed = want["seed"].as_u64().unwrap();
    let r = eng.run(n, Some(seed)).unwrap();
    let tol = 1e-9;
    close(r.per_share_p10, num(&want["per_share_p10"]), tol, &format!("{tag} p10"));
    close(r.per_share_p50, num(&want["per_share_p50"]), tol, &format!("{tag} p50"));
    close(r.per_share_p90, num(&want["per_share_p90"]), tol, &format!("{tag} p90"));
    for (g, w) in r.quantiles().iter().zip(want["quantiles_10_25_50_75_90"].as_array().unwrap()) {
        close(*g, num(w), tol, &format!("{tag} quantile"));
    }
    close(r.tv_ev_share_at_means, num(&want["tv_ev_share_at_means"]), tol, &format!("{tag} tv/ev"));
    close(r.tg_pinned_fraction, num(&want["tg_pinned_fraction"]), 1e-12, &format!("{tag} pinned"));
    close(r.value_at_means, num(&want["value_at_means"]), tol, &format!("{tag} at means"));
    let order: Vec<&str> = want["attribution_order"].as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect();
    assert_eq!(r.attribution.iter().map(|a| a.driver.as_str()).collect::<Vec<_>>(), order);
    for a in &r.attribution {
        let w = want["attribution"][&a.driver].as_array().unwrap();
        close(a.p50_minus_1sd, num(&w[0]), tol, &format!("{tag} {} -1sd", a.driver));
        close(a.p50_plus_1sd, num(&w[1]), tol, &format!("{tag} {} +1sd", a.driver));
    }
    for (i, w) in want["first20"].as_array().unwrap().iter().enumerate() {
        close(r.per_share_paths[i], num(w), tol, &format!("{tag} path {i}"));
    }
    for (i, w) in want["every_500th"].as_array().unwrap().iter().enumerate() {
        close(r.per_share_paths[i * 500], num(w), tol, &format!("{tag} path {}", i * 500));
    }
    let ps = &want["per_share_stats"];
    close(r.per_share_paths.iter().sum::<f64>(), num(&ps["sum"]), tol, &format!("{tag} sum"));
    close(r.per_share_paths.iter().cloned().fold(f64::MAX, f64::min), num(&ps["min"]), tol, &format!("{tag} min"));
    close(r.per_share_paths.iter().cloned().fold(f64::MIN, f64::max), num(&ps["max"]), tol, &format!("{tag} max"));
    let w = &want["wacc"];
    close(r.valuation.wacc.iter().cloned().fold(f64::MAX, f64::min), num(&w["min"]), tol, &format!("{tag} wacc min"));
    close(r.valuation.wacc.iter().cloned().fold(f64::MIN, f64::max), num(&w["max"]), tol, &format!("{tag} wacc max"));
    close(r.valuation.wacc.iter().sum::<f64>() / n as f64, num(&w["mean"]), tol, &format!("{tag} wacc mean"));
    let v = &r.valuation;
    for (t, w) in want["fcff_year_sums"].as_array().unwrap().iter().enumerate() {
        close(v.fcff[t * n..(t + 1) * n].iter().sum::<f64>(), num(w), tol, &format!("{tag} fcff year {t}"));
    }
    for (t, w) in want["fcff_first_path"].as_array().unwrap().iter().enumerate() {
        close(v.fcff[t * n], num(w), tol, &format!("{tag} fcff path0 year {t}"));
    }
    for (t, w) in want["pv_fcff_year_sums"].as_array().unwrap().iter().enumerate() {
        close(v.pv_fcff[t * n..(t + 1) * n].iter().sum::<f64>(), num(w), tol, &format!("{tag} pv year {t}"));
    }
    let pt = &want["pv_terminal"];
    close(v.pv_terminal.iter().sum::<f64>(), num(&pt["sum"]), tol, &format!("{tag} pv_terminal sum"));
    for (i, w) in pt["first5"].as_array().unwrap().iter().enumerate() {
        close(v.pv_terminal[i], num(w), tol, &format!("{tag} pv_terminal {i}"));
    }
    for (name, dd) in want["driver_draws"].as_object().unwrap() {
        let got = &r.driver_draws.iter().find(|(k, _)| k == name).unwrap().1;
        close(got.iter().sum::<f64>(), num(&dd["sum"]), tol, &format!("{tag} draws {name} sum"));
        close(got.iter().cloned().fold(f64::MAX, f64::min), num(&dd["min"]), tol, &format!("{tag} draws {name} min"));
        close(got.iter().cloned().fold(f64::MIN, f64::max), num(&dd["max"]), tol, &format!("{tag} draws {name} max"));
        for (i, w) in dd["first5"].as_array().unwrap().iter().enumerate() {
            close(got[i], num(w), tol, &format!("{tag} draws {name} {i}"));
        }
    }
    for (p, w) in want["chance_below"].as_object().unwrap() {
        close(r.share_below(p.parse().unwrap()), num(w), 1e-12, &format!("{tag} below {p}"));
    }
}

#[test]
fn driver_engine_matches_python_for_ibm_jnj_ant() {
    for t in ["IBM", "JNJ", "ANT"] {
        for file in [format!("driver_{t}.json"), format!("driver_{t}_s123.json")] {
            let f = fixture(&file);
            check_driver_result(&spec_from(&f["model"]), &f["result"], &file);
        }
    }
}

#[test]
fn ibm_headline_is_the_published_number() {
    let f = fixture("driver_IBM.json");
    let r = spec_from(&f["model"]).engine().unwrap().run(10_000, Some(7)).unwrap();
    assert_eq!(format!("{:.2}", r.per_share_p10), "83.84");
    assert_eq!(format!("{:.2}", r.per_share_p50), "151.84");
    assert_eq!(format!("{:.2}", r.per_share_p90), "258.03");
}

#[test]
fn driver_engine_matches_exact_hand_calculation() {
    for case in fixture("driver_deterministic.json").as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let eng = spec_from(&case["model"]).engine().unwrap();
        let hand = num(&case["hand_per_share"]);
        let r = eng.run(50, Some(1)).unwrap();
        // every path is the same: the model has no random driver
        close(r.per_share_p10, hand, 1e-12, &format!("{name} p10"));
        close(r.per_share_p50, hand, 1e-12, &format!("{name} p50"));
        close(r.per_share_p90, hand, 1e-12, &format!("{name} p90"));
        close(r.value_at_means, hand, 1e-12, &format!("{name} at means"));
        close(r.per_share_p50, num(&case["engine_p50"]), 1e-12, &format!("{name} python"));
        // EV and terminal value at the means
        let at = eng.valuation(&eng.mean_draws(), 1).unwrap();
        let ev = at.pv_fcff.iter().sum::<f64>() + at.pv_terminal[0];
        close(ev, num(&case["hand_ev"]), 1e-12, &format!("{name} ev"));
        close(at.pv_terminal[0], num(&case["hand_pv_terminal"]), 1e-12, &format!("{name} tv"));
        assert!(r.attribution.is_empty());
        assert_eq!(r.tg_pinned_fraction, 0.0);
    }
}

fn small_model() -> ModelSpec {
    spec_from(&fixture("driver_deterministic.json")[0]["model"])
}

#[test]
fn terminal_growth_guard_binds_when_tg_is_close_to_wacc() {
    let mut spec = small_model();
    for d in spec.drivers.iter_mut() {
        if d.name == "terminal_growth" {
            d.mean = 0.20;
        }
    }
    let eng = spec.engine().unwrap();
    let r = eng.run(10, Some(1)).unwrap();
    assert_eq!(r.tg_pinned_fraction, 1.0);
    let at = eng.valuation(&eng.mean_draws(), 1).unwrap();
    // wacc at means: 0.8*(0.04+1.1*0.05) + 0.2*(0.05)*(0.75) = 0.0795; tg_eff = wacc - 0.02
    let wacc = 0.8 * (0.04 + 1.1 * 0.05) + 0.2 * (0.04 + 0.01) * 0.75;
    assert!((at.wacc[0] - wacc).abs() < 1e-15);
}

fn d(name: &str, stochastic: bool, mean: f64) -> Driver {
    Driver::new(name, stochastic, mean)
}

#[test]
fn driver_validation_rejects_the_same_inputs_as_python() {
    let bad = |f: &dyn Fn(&mut Driver)| {
        let mut x = d("x", true, 0.1);
        x.std = 0.1;
        f(&mut x);
        x.validate().is_err()
    };
    assert!(bad(&|x| x.std = 0.0), "stochastic with std 0");
    assert!(bad(&|x| x.schedule = Some(vec![0.1])), "stochastic with schedule");
    assert!(bad(&|x| x.dist = "lognormal".into()));
    assert!(bad(&|x| x.lo = Some(0.0)), "bounds with dist normal");
    assert!(bad(&|x| x.dist = "truncnorm".into()), "truncnorm without bounds");
    assert!(bad(&|x| {
        x.dist = "truncnorm".into();
        x.lo = Some(0.5);
        x.hi = Some(0.4);
    }));
    assert!(bad(&|x| {
        x.dist = "truncnorm".into();
        x.lo = Some(0.2);
    }), "mean below lo");
    assert!(bad(&|x| {
        x.dist = "truncnorm".into();
        x.hi = Some(0.05);
    }), "mean above hi");
    let mut y = d("y", false, 0.1);
    y.std = 0.1;
    assert!(y.validate().is_err(), "deterministic with std");
    let mut y = d("y", false, 0.1);
    y.schedule = Some(vec![0.2, 0.3]);
    assert!(y.validate().is_err(), "mean must equal schedule[0]");
    let mut y = d("y", false, 0.1);
    y.schedule = Some(vec![]);
    assert!(y.validate().is_err(), "empty schedule");
    assert!(d("", false, 0.0).validate().is_err());
    assert!(d("ok", false, 0.0).validate().is_ok());
}

#[test]
fn model_rejects_duplicates_bad_correlations_and_non_pd_matrices() {
    let mk = |corr: Vec<(&str, &str, f64)>| {
        let mut a = d("a", true, 0.0);
        a.std = 1.0;
        let mut b = d("b", true, 0.0);
        b.std = 1.0;
        let mut c = d("c", true, 0.0);
        c.std = 1.0;
        let k = d("k", false, 1.0);
        DriverModel::new(
            vec![a, b, c, k],
            corr.into_iter().map(|(a, b, rho)| CorrEntry { a: a.into(), b: b.into(), rho, note: String::new() }).collect(),
        )
        .unwrap()
    };
    assert!(DriverModel::new(vec![d("a", false, 0.0), d("a", false, 0.0)], vec![]).is_err());
    assert!(mk(vec![("a", "b", 0.5)]).correlation_matrix().is_ok());
    assert!(mk(vec![("a", "k", 0.5)]).correlation_matrix().is_err(), "deterministic driver");
    assert!(mk(vec![("a", "zz", 0.5)]).correlation_matrix().is_err(), "unknown driver");
    assert!(mk(vec![("a", "a", 0.5)]).correlation_matrix().is_err(), "self correlation");
    assert!(mk(vec![("a", "b", 1.5)]).correlation_matrix().is_err(), "out of range");
    assert!(mk(vec![("a", "b", 0.5), ("b", "a", 0.4)]).correlation_matrix().is_err(), "conflict");
    assert!(mk(vec![("a", "b", 0.5), ("b", "a", 0.5)]).correlation_matrix().is_ok(), "same value twice");
    let e = mk(vec![("a", "b", 0.9), ("b", "c", 0.9), ("a", "c", -0.9)]).correlation_matrix().unwrap_err();
    assert!(e.0.contains("not positive definite"), "{e}");
}

#[test]
fn engine_rejects_missing_drivers_and_bad_structure() {
    let spec = small_model();
    let mut s = spec.clone();
    s.drivers.retain(|x| x.name != "beta");
    assert!(s.engine().unwrap_err().0.contains("missing required driver"));
    let mut s = spec.clone();
    s.shares = 0.0;
    assert!(s.engine().is_err());
    let mut s = spec.clone();
    s.segments.clear();
    assert!(s.engine().is_err());
    let mut s = spec.clone();
    s.segments.push(s.segments[0].clone());
    assert!(s.engine().unwrap_err().0.contains("Duplicate segment"));
    let mut s = spec.clone();
    s.segments[0].base_revenue = 0.0;
    assert!(s.engine().is_err());
    let mut s = spec.clone();
    s.projection_years = 0;
    assert!(s.engine().is_err());
    // schedule length must match the horizon
    let mut s = spec.clone();
    for x in s.drivers.iter_mut() {
        if x.name == "sbc_ratio" {
            x.schedule = Some(vec![x.mean, 0.04]);
        }
    }
    assert!(s.engine().unwrap_err().0.contains("schedule has 2 entries"));
    // a schedule on a WACC driver is not allowed
    let mut s = spec.clone();
    for x in s.drivers.iter_mut() {
        if x.name == "rf" {
            x.schedule = Some(vec![0.04; 10]);
        }
    }
    assert!(s.engine().is_err());
}

#[test]
fn non_positive_wacc_is_an_error() {
    let mut s = small_model();
    for x in s.drivers.iter_mut() {
        if x.name == "rf" {
            x.mean = -0.5;
        }
    }
    let eng = s.engine().unwrap();
    assert!(eng.run(5, Some(1)).unwrap_err().0.contains("WACC is non-positive"));
}

#[test]
fn sample_rejects_unknown_overrides_and_zero_paths() {
    let eng = small_model().engine().unwrap();
    assert!(eng.model.sample(0, 1, &[]).is_err());
    assert!(eng.model.sample(5, 1, &[("nope".into(), 1.0)]).is_err());
}

#[test]
fn mean_override_shifts_a_schedule_and_keeps_the_z_draws() {
    let f = fixture("driver_IBM.json");
    let eng = spec_from(&f["model"]).engine().unwrap();
    let a = eng.model.sample(100, 7, &[]).unwrap();
    let b = eng.model.sample(100, 7, &[("g_software".into(), 0.06)]).unwrap();
    let (super::driver::Draw::PerPath(x), super::driver::Draw::PerPath(y)) = (&a["g_software"], &b["g_software"]) else {
        panic!("per-path draws")
    };
    // same z: the difference is the shift of the mean, except where the bound clips
    let mean = 0.05;
    let moved = x.iter().zip(y).filter(|(p, q)| **p < 0.11 && **q < 0.11).all(|(p, q)| ((q - p) - (0.06 - mean)).abs() < 1e-12);
    assert!(moved);
    let c = eng.model.sample(10, 7, &[("sbc_ratio".into(), 0.03)]).unwrap();
    let super::driver::Draw::PerYear(s) = &c["sbc_ratio"] else { panic!("schedule") };
    assert!((s[0] - 0.03).abs() < 1e-15 && (s[9] - (0.035 + 0.03 - 0.0254)).abs() < 1e-15);
}

#[test]
fn ramp_builds_the_python_schedule() {
    let r = Ramp { start: 0.25, end: 0.06, ramp_years: 8 };
    let s = r.schedule(10);
    assert_eq!(s.len(), 10);
    assert_eq!(s[0], 0.25);
    assert!((s[7] - 0.06).abs() < 1e-15 && (s[9] - 0.06).abs() < 1e-15);
    assert!((s[1] - (0.25 + (0.06 - 0.25) / 7.0)).abs() < 1e-15);
}

// ---------------------------------------------------------------------------
// Classic engine, scenarios, capital structure
// ---------------------------------------------------------------------------

fn company_from(v: &Value) -> Company {
    let mut c: Company = serde_json::from_value(v.clone()).unwrap();
    c.ticker = v["ticker"].as_str().unwrap_or("").to_string();
    c
}

fn assumptions_from(v: &Value) -> DcfAssumptions {
    serde_json::from_value(v.clone()).unwrap()
}

fn check_classic(r: &super::classic::DcfResult, want: &Value, tag: &str) {
    let tol = 1e-9;
    close(r.median_value, num(&want["median_value"]), tol, &format!("{tag} median"));
    close(r.p10_value, num(&want["p10_value"]), tol, &format!("{tag} p10"));
    close(r.p90_value, num(&want["p90_value"]), tol, &format!("{tag} p90"));
    close(r.mean_value, num(&want["mean_value"]), tol, &format!("{tag} mean"));
    close(r.std_value, num(&want["std_value"]), tol, &format!("{tag} std"));
    match (r.median_per_share, want["median_per_share"].as_f64()) {
        (Some(a), Some(b)) => close(a, b, tol, &format!("{tag} per share")),
        (None, None) => {}
        other => panic!("{tag} per share {other:?}"),
    }
    for (i, w) in want["first20"].as_array().unwrap().iter().enumerate() {
        close(r.simulations[i], num(w), tol, &format!("{tag} sim {i}"));
    }
    close(r.simulations.iter().sum::<f64>(), num(&want["sim_sum"]), tol, &format!("{tag} sim sum"));
    let n = r.n;
    for (t, w) in want["fcf_year_sums"].as_array().unwrap().iter().enumerate() {
        close(r.fcfs[t * n..(t + 1) * n].iter().sum::<f64>(), num(w), tol, &format!("{tag} fcf year {t}"));
    }
    for (t, w) in want["pv_fcf_year_sums"].as_array().unwrap().iter().enumerate() {
        close(r.pv_fcfs[t * n..(t + 1) * n].iter().sum::<f64>(), num(w), tol, &format!("{tag} pv year {t}"));
    }
    close(r.pv_terminal.iter().sum::<f64>(), num(&want["pv_terminal_sum"]), tol, &format!("{tag} pv terminal"));
}

#[test]
fn classic_engine_and_scenarios_match_python() {
    let mut checked = 0;
    for case in fixture("classic.json").as_array().unwrap() {
        let t = case["ticker"].as_str().unwrap();
        let company = company_from(&case["company"]);
        // derived margin
        match (&case["derived_margin"], derive_fcf_margin(&company)) {
            (Value::Array(w), Ok((m, s))) => {
                close(m, num(&w[0]), 1e-12, &format!("{t} margin mean"));
                close(s, num(&w[1]), 1e-12, &format!("{t} margin std"));
            }
            (Value::Object(_), Err(_)) => {}
            (w, g) => panic!("{t} derived margin {w:?} vs {g:?}"),
        }
        // capital structure
        match (&case["capital_structure"], capital_structure_for(&company)) {
            (w, Ok(c)) if w.get("error").is_none() => {
                close(c.net_debt, num(&w["net_debt"]), 1e-12, &format!("{t} net debt"));
                close(c.shares, num(&w["shares"]), 1e-12, &format!("{t} shares"));
            }
            (w, Err(_)) if w.get("error").is_some() => {}
            (w, g) => panic!("{t} capital {w:?} vs {g:?}"),
        }
        // default run
        let a = assumptions_from(&case["default_run"]["assumptions"]);
        let want = &case["default_run"]["result"];
        match run_classic(&company, &a) {
            Ok(r) => check_classic(&r, want, &format!("{t} default")),
            Err(e) => assert!(want.get("error").is_some(), "{t}: unexpected error {e}"),
        }
        // scenarios of this ticker: assumptions are the registry values
        let sc = scenarios_for(&company.ticker);
        assert_eq!(sc.len(), case["scenarios"].as_array().unwrap().len());
        for (s, w) in sc.iter().zip(case["scenarios"].as_array().unwrap()) {
            assert_eq!(s.key, w["key"].as_str().unwrap());
            assert_eq!(s.label, w["label"].as_str().unwrap());
            assert_eq!(s.assumptions, assumptions_from(&w["assumptions"]), "{t} {}", s.key);
            let res = run_classic(&company, &s.assumptions);
            match res {
                Ok(r) => {
                    check_classic(&r, &w["result"], &format!("{t} {}", s.key));
                    if let Some(ps) = w.get("per_share") {
                        let cap = capital_structure_for(&company).unwrap();
                        let paths: Vec<f64> = r.simulations.iter().map(|v| (v - cap.net_debt) / cap.shares).collect();
                        let sorted = super::stats::sorted_copy(&paths);
                        for (k, q) in [("p10", 10.0), ("p25", 25.0), ("p50", 50.0), ("p75", 75.0), ("p90", 90.0)] {
                            close(percentile(&sorted, q), num(&ps[k]), 1e-9, &format!("{t} {} per-share {k}", s.key));
                        }
                    }
                    checked += 1;
                }
                Err(e) => assert!(w["result"].get("error").is_some(), "{t} {}: {e}", s.key),
            }
        }
    }
    assert!(checked >= 12, "checked {checked}");
}

#[test]
fn error_messages_name_the_cause() {
    let cases = fixture("classic.json");
    let by = |t: &str| company_from(&cases.as_array().unwrap().iter().find(|c| c["ticker"] == t).unwrap()["company"]);
    let a = DcfAssumptions { seed: Some(7), ..DcfAssumptions::default() };
    let e = run_classic(&by("SHORT"), &a).unwrap_err();
    assert!(e.0.contains("only 3 annual period(s) with positive FCF"), "{e}");
    let e = run_classic(&by("NOREV"), &a).unwrap_err();
    assert!(e.0.contains("FCF margin cannot be derived"), "{e}");
    let pinned = DcfAssumptions { fcf_margin_mean: Some(0.1), fcf_margin_std: Some(0.01), seed: Some(7), ..DcfAssumptions::default() };
    let e = run_classic(&by("NOREV"), &pinned).unwrap_err();
    assert!(e.0.contains("no annual revenue data"), "{e}");
    assert!(capital_structure_for(&by("NODEBT")).unwrap_err().0.contains("cannot derive net debt"));
    assert!(capital_structure_for(&by("NOSHARES")).unwrap_err().0.contains("no shares_outstanding or shares_diluted"));
    assert!(capital_structure_for(&Company { ticker: "X".into(), ..Company::default() }).is_err());
    let zero = DcfAssumptions { n_simulations: 0, ..a.clone() };
    assert!(run_classic(&by("SYN"), &zero).is_err());
    let neg = DcfAssumptions { wacc_std: -0.1, ..a };
    assert!(run_classic(&by("SYN"), &neg).is_err());
}

#[test]
fn classic_engine_matches_exact_closed_form_with_zero_std() {
    for case in fixture("classic_deterministic.json").as_array().unwrap() {
        let company = Company {
            ticker: "DET".into(),
            company_name: String::new(),
            periods: serde_json::from_value(case["company"].clone()).unwrap(),
        };
        let a = assumptions_from(&case["assumptions"]);
        let r = run_classic(&company, &a).unwrap();
        let hand = num(&case["hand_ev"]);
        close(r.median_value, hand, 1e-12, "median");
        close(r.p10_value, hand, 1e-12, "p10");
        close(r.p90_value, hand, 1e-12, "p90");
        close(r.mean_value, hand, 1e-12, "mean");
        assert!(r.std_value < 1e-9 * hand);
    }
}

#[test]
fn scenario_registry_matches_python() {
    let f = fixture("scenarios.json");
    assert_eq!(SCENARIO_SEED, f["seed"].as_u64().unwrap());
    for (s, w) in default_scenarios().iter().zip(f["default"].as_array().unwrap()) {
        assert_eq!(s.assumptions, assumptions_from(&w["assumptions"]));
        assert_eq!((s.key, s.label), (w["key"].as_str().unwrap(), w["label"].as_str().unwrap()));
    }
    for (t, list) in f["overrides"].as_object().unwrap() {
        let got = scenarios_for(t);
        assert_eq!(got.len(), list.as_array().unwrap().len());
        for (s, w) in got.iter().zip(list.as_array().unwrap()) {
            assert_eq!(s.assumptions, assumptions_from(&w["assumptions"]), "{t} {}", s.key);
            assert_eq!(s.label, w["label"].as_str().unwrap());
        }
    }
    for (t, w) in f["capital"].as_object().unwrap() {
        let c = super::scenarios::capital_override(t).unwrap();
        assert_eq!((c.net_debt, c.shares), (num(&w["net_debt"]), num(&w["shares"])));
    }
    assert!(scenarios_for("ibm")[0].label.starts_with("Bear  (disruption"));
}

// ---------------------------------------------------------------------------
// Verdict
// ---------------------------------------------------------------------------

#[test]
fn verdict_bands_and_cases_match_python() {
    let f = fixture("verdict.json");
    for b in f["bands"].as_array().unwrap() {
        assert_eq!(verdict_band(num(&b["p"])), b["verdict"].as_str().unwrap(), "p {}", b["p"]);
    }
    for c in f["cases"].as_array().unwrap() {
        let t = c["ticker"].as_str().unwrap();
        let spec = spec_from(&fixture(&format!("driver_{t}.json"))["model"]);
        let r = spec.engine().unwrap().run(10_000, Some(7)).unwrap();
        let price = num(&c["price"]);
        for (g, w) in r.quantiles().iter().zip(c["p"].as_array().unwrap()) {
            close(*g, num(w), 1e-9, &format!("{t} verdict quantile"));
        }
        let th = chance_price_too_high(&r.per_share_paths, price);
        close(th, num(&c["chance_price_too_high"]), 1e-12, &format!("{t} chance {price}"));
        assert_eq!(verdict_band(th), c["verdict"].as_str().unwrap(), "{t} {price}");
    }
}

#[test]
fn segment_spec_roundtrip_keeps_year1_pin() {
    let s: SegmentSpec = serde_json::from_str(r#"{"name":"A","base_revenue":1.0,"growth_driver":"g","year1_growth":0.05}"#).unwrap();
    assert_eq!(s.year1_growth, Some(0.05));
    let s: SegmentSpec = serde_json::from_str(r#"{"name":"A","base_revenue":1.0,"growth_driver":"g"}"#).unwrap();
    assert_eq!(s.year1_growth, None);
}

/// Prints the largest relative difference to Python. Run: `cargo test parity_report -- --ignored --nocapture`.
#[test]
#[ignore]
fn parity_report() {
    for t in ["IBM", "JNJ", "ANT"] {
        let f = fixture(&format!("driver_{t}.json"));
        let w = &f["result"];
        let r = spec_from(&f["model"]).engine().unwrap().run(10_000, Some(7)).unwrap();
        let mut worst = 0.0f64;
        for a in &r.attribution {
            let x = w["attribution"][&a.driver].as_array().unwrap();
            worst = worst.max(rel(a.p50_minus_1sd, num(&x[0]))).max(rel(a.p50_plus_1sd, num(&x[1])));
        }
        let paths: f64 = w["first20"].as_array().unwrap().iter().enumerate().map(|(i, v)| rel(r.per_share_paths[i], num(v))).fold(0.0, f64::max);
        eprintln!(
            "{t}: P10 py {:.12} rs {:.12} rel {:.1e} | P50 py {:.12} rs {:.12} rel {:.1e} | P90 rel {:.1e} | at-means py {:.10} rs {:.10} rel {:.1e} | tv/ev rel {:.1e} | attribution worst {:.1e} | first20 worst {:.1e}",
            num(&w["per_share_p10"]), r.per_share_p10, rel(r.per_share_p10, num(&w["per_share_p10"])),
            num(&w["per_share_p50"]), r.per_share_p50, rel(r.per_share_p50, num(&w["per_share_p50"])),
            rel(r.per_share_p90, num(&w["per_share_p90"])),
            num(&w["value_at_means"]), r.value_at_means, rel(r.value_at_means, num(&w["value_at_means"])),
            rel(r.tv_ev_share_at_means, num(&w["tv_ev_share_at_means"])), worst, paths
        );
    }
    for case in fixture("classic.json").as_array().unwrap() {
        let t = case["ticker"].as_str().unwrap();
        if !["IBM", "JNJ", "SYN"].contains(&t) {
            continue;
        }
        let company = company_from(&case["company"]);
        for (s, w) in scenarios_for(&company.ticker).iter().zip(case["scenarios"].as_array().unwrap()) {
            if let Ok(r) = run_classic(&company, &s.assumptions) {
                let x = &w["result"];
                eprintln!(
                    "classic {t} {}: median py {:.6e} rs {:.6e} rel {:.1e} | p10 {:.1e} | p90 {:.1e} | mean {:.1e} | std {:.1e}",
                    s.key, num(&x["median_value"]), r.median_value, rel(r.median_value, num(&x["median_value"])),
                    rel(r.p10_value, num(&x["p10_value"])), rel(r.p90_value, num(&x["p90_value"])),
                    rel(r.mean_value, num(&x["mean_value"])), rel(r.std_value, num(&x["std_value"]))
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Driver models as data (stage 2)
// ---------------------------------------------------------------------------

fn repo() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn toml_model(t: &str) -> ModelSpec {
    ModelSpec::load(&repo(), t).unwrap().unwrap_or_else(|| panic!("no model file for {t}"))
}

#[test]
fn toml_models_give_the_python_models() {
    for t in ["IBM", "JNJ", "ANT"] {
        let fix = fixture(&format!("driver_{t}.json"));
        let want = spec_from(&fix["model"]);
        let got = toml_model(t);
        let eng = got.engine().unwrap();
        assert_eq!(got.ticker, t);
        assert_eq!(got.net_debt, want.net_debt, "{t} net debt");
        assert_eq!(got.shares, want.shares, "{t} shares");
        assert_eq!(got.projection_years, want.projection_years);
        assert_eq!(got.segments, want.segments, "{t} segments");
        assert_eq!(got.segment_history, want.segment_history, "{t} segment history");
        assert_eq!(got.drivers.len(), want.drivers.len());
        for (g, w) in eng.model.drivers.iter().zip(&want.drivers) {
            assert_eq!(g.name, w.name);
            assert_eq!((g.stochastic, g.mean, g.std, &g.dist, g.lo, g.hi), (w.stochastic, w.mean, w.std, &w.dist, w.lo, w.hi), "{t} {}", g.name);
            assert_eq!(g.schedule, w.schedule, "{t} {} schedule", g.name);
            assert_eq!(g.rationale, w.rationale, "{t} {} rationale text", g.name);
            assert!(!g.rationale.is_empty(), "{t} {}", g.name);
        }
        let key = |e: &CorrEntry| (e.a.clone(), e.b.clone(), e.rho);
        assert_eq!(got.corr.iter().map(key).collect::<Vec<_>>(), want.corr.iter().map(key).collect::<Vec<_>>(), "{t} corr");
        // the result of the file is the recorded result of the Python model
        check_driver_result(&got, &fix["result"], &format!("{t} toml"));
        let fix2 = fixture(&format!("driver_{t}_s123.json"));
        check_driver_result(&got, &fix2["result"], &format!("{t} toml s123"));
    }
}

#[test]
fn toml_models_state_their_status() {
    assert!(toml_model("ANT").notes.contains("Not reviewed"));
    assert!(toml_model("JNJ").notes.contains("FIRST PASS"));
    assert!(!toml_model("IBM").notes.contains("Not reviewed"));
    assert_eq!(super::model::available_models(&repo()), vec!["ANT", "IBM", "JNJ"]);
    assert!(ModelSpec::load(&repo(), "NOPE").unwrap().is_none());
}

#[test]
fn bad_model_text_is_an_error() {
    assert!(ModelSpec::from_toml("net_debt = 1.0").is_err(), "shares missing");
    assert!(ModelSpec::from_toml("net_debt = 1.0\nshares = 2.0\n[[drivers]]\nname = 'x'").is_err(), "stochastic missing");
    let mut s = small_model();
    for x in s.drivers.iter_mut() {
        if x.name == "sbc_ratio" {
            x.ramp = Some(Ramp { start: x.mean + 0.1, end: 0.0, ramp_years: 4 });
        }
    }
    assert!(s.engine().unwrap_err().0.contains("must equal the ramp start"));
}

// ---------------------------------------------------------------------------
// Overrides, bisection, commands, verdict through `Fa`
// ---------------------------------------------------------------------------

use super::model::{bisect, Overrides};

#[test]
fn overrides_change_means_bounds_schedules_correlations_and_segments() {
    let m = toml_model("IBM");
    // plain mean: bounds stay
    let o = m.with_overrides(&Overrides::mean("g_software", 0.06)).unwrap();
    let d = o.driver("g_software").unwrap();
    assert_eq!((d.mean, d.hi), (0.06, Some(0.11)));
    // with shift: the bounds move by the same amount
    let o = m.with_overrides(&Overrides { means: vec![("g_software".into(), 0.06)], shift_bounds: true, ..Overrides::default() }).unwrap();
    assert!((o.driver("g_software").unwrap().hi.unwrap() - 0.12).abs() < 1e-15);
    // a mean on a scheduled driver is refused; flat and ramp work
    assert!(m.with_overrides(&Overrides::mean("sbc_ratio", 0.03)).is_err());
    let o = m.with_overrides(&Overrides { flat: vec![("sbc_ratio".into(), 0.0254)], ..Overrides::default() }).unwrap();
    assert!(o.driver("sbc_ratio").unwrap().schedule.is_none());
    let o = m.with_overrides(&Overrides { ramps: vec![("sbc_ratio".into(), Ramp { start: 0.03, end: 0.05, ramp_years: 5 })], ..Overrides::default() }).unwrap();
    let eng = o.engine().unwrap();
    let s = eng.model.driver("sbc_ratio").unwrap().schedule.clone().unwrap();
    assert_eq!((s.len(), s[0], s[9]), (10, 0.03, 0.05));
    // a pair that is in the model changes (also when the names are in the other order); a new pair is added
    let o = m.with_overrides(&Overrides { corr: vec![("g_consulting".into(), "g_software".into(), 0.1), ("rf".into(), "core_margin".into(), 0.2)], ..Overrides::default() }).unwrap();
    assert_eq!(o.corr.len(), m.corr.len() + 1);
    assert_eq!(o.corr.iter().find(|e| e.a == "g_software" && e.b == "g_consulting").unwrap().rho, 0.1);
    // year-1 pin from revenue, net debt, shares, horizon
    let a = toml_model("ANT");
    let o = a.with_overrides(&Overrides { year1_revenue: vec![("Total".into(), 80.6e9)], net_debt: Some(1.0), shares: Some(2.0), years: Some(15), ..Overrides::default() }).unwrap();
    assert_eq!((o.net_debt, o.shares, o.projection_years), (1.0, 2.0, 15));
    assert!((o.segments[0].year1_growth.unwrap() - (80.6e9 / 35_282e6 - 1.0)).abs() < 1e-15);
    assert_eq!(o.engine().unwrap().model.driver("sbc_ratio").unwrap().schedule.as_ref().unwrap().len(), 15);
    // unknown names
    assert!(m.with_overrides(&Overrides::mean("nope", 1.0)).is_err());
    assert!(m.with_overrides(&Overrides { year1_revenue: vec![("Nope".into(), 1.0)], ..Overrides::default() }).is_err());
    // an explicit schedule cannot follow a new horizon
    let o = m.with_overrides(&Overrides { years: Some(12), ..Overrides::default() }).unwrap();
    assert!(o.engine().is_err());
}

#[test]
fn bisect_finds_the_root_in_both_directions_and_reports_no_solution() {
    let up = bisect(|x| Ok(2.0 * x + 1.0), 0.0, 10.0, 7.0).unwrap().unwrap();
    assert!((up - 3.0).abs() < 1e-9);
    let down = bisect(|x| Ok(10.0 - x), 0.0, 10.0, 7.0).unwrap().unwrap();
    assert!((down - 3.0).abs() < 1e-9);
    assert!(bisect(Ok, 0.0, 1.0, 5.0).unwrap().is_none());
    assert!(bisect(|_| Err(super::DcfError("x".into())), 0.0, 1.0, 5.0).is_err());
}

#[test]
fn implied_value_gives_the_price_back() {
    use super::analysis::{implied_mean, p50_with_mean};
    let m = toml_model("JNJ");
    let base = Overrides::default();
    let target = 150.0;
    let beta = implied_mean(&m, &base, "beta", 0.0, 3.0, target, 10_000, 7).unwrap().unwrap();
    let back = p50_with_mean(&m, &base, "beta", beta, 10_000, 7).unwrap();
    assert!((back - target).abs() < 1e-6, "{back}");
    assert!(implied_mean(&m, &base, "beta", 0.0, 3.0, 1e9, 10_000, 7).unwrap().is_none());
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

#[test]
fn commands_check_their_arguments() {
    let r = repo();
    use super::cli::run;
    assert_eq!(run(&r, &args(&["help"])), 0);
    assert_eq!(run(&r, &args(&["models"])), 0);
    assert_eq!(run(&r, &args(&["nope"])), 64);
    assert_eq!(run(&r, &args(&["drivers"])), 2);
    assert_eq!(run(&r, &args(&["drivers", "IBM", "--bogus"])), 2);
    assert_eq!(run(&r, &args(&["drivers", "IBM", "--sims"])), 2);
    assert_eq!(run(&r, &args(&["drivers", "NOPE"])), 2);
    assert_eq!(run(&r, &args(&["drivers", "IBM", "--price", "-3"])), 2);
    assert_eq!(run(&r, &args(&["drivers", "IBM", "--set", "beta"])), 2);
    assert_eq!(run(&r, &args(&["drivers", "IBM", "--ramp", "sbc_ratio=1:2"])), 2);
    assert_eq!(run(&r, &args(&["drivers", "IBM", "--implied-price", "100"])), 2);
    assert_eq!(run(&r, &args(&["drivers", "IBM", "--sims", "1000", "--price", "150", "--set", "beta=0.9", "--json"])), 0);
    assert_eq!(
        run(&r, &args(&["drivers", "ANT", "--sims", "1000", "--ramp", "sbc_ratio=0.2:0.06:8", "--sensitivity", "beta=1.1,1.7", "--implied-price", "50", "--implied-driver", "beta", "--range", "0,3"])),
        0
    );
}

#[test]
fn classic_and_scenarios_commands_read_the_company_file() {
    let dir = std::env::temp_dir().join(format!("fa-dcf-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cases = fixture("classic.json");
    let syn = cases.as_array().unwrap().iter().find(|c| c["ticker"] == "SYN").unwrap();
    std::fs::write(dir.join("SYN.json"), serde_json::to_string(&syn["company"]).unwrap()).unwrap();
    let d = dir.to_str().unwrap();
    use super::cli::run;
    let r = repo();
    assert_eq!(run(&r, &args(&["classic", "SYN", "--seed", "7", "--sims", "500", "--data-dir", d])), 0);
    assert_eq!(run(&r, &args(&["classic", "SYN", "--seed", "7", "--json", "--data-dir", d])), 0);
    assert_eq!(run(&r, &args(&["scenarios", "SYN", "--data-dir", d])), 0);
    assert_eq!(run(&r, &args(&["scenarios", "SYN", "--json", "--data-dir", d])), 0);
    assert_eq!(run(&r, &args(&["classic", "MISSING", "--data-dir", d])), 2);
    assert_eq!(run(&r, &args(&["classic", "SYN", "--growth", "x", "--data-dir", d])), 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fa_verdict_runs_in_rust_for_a_model_file() {
    let fa = crate::Fa::new("python-that-does-not-exist", repo());
    let v = fa.verdict("ibm", 228.53, &crate::VerdictOptions::default()).unwrap();
    let want = &fixture("verdict.json")["cases"].as_array().unwrap().iter().find(|c| c["ticker"] == "IBM" && c["price"] == 228.53).unwrap().clone();
    assert_eq!((v.ticker.as_str(), v.simulations, v.seed, v.price), ("IBM", 10_000, 7, 228.53));
    close(v.p10, num(&want["p"][0]), 1e-12, "p10");
    close(v.p50, num(&want["p"][2]), 1e-12, "p50");
    close(v.p90, num(&want["p"][4]), 1e-12, "p90");
    assert_eq!(v.chance_price_too_high, num(&want["chance_price_too_high"]));
    assert_eq!(v.verdict, want["verdict"].as_str().unwrap());
    assert!(v.rates.is_none() && v.credit.is_none());
    // the JSON has the fields that the viewer reads
    let j = serde_json::to_value(&v).unwrap();
    for k in ["ticker", "simulations", "seed", "price", "p10", "p25", "p50", "p75", "p90", "chance_price_too_high", "verdict", "rates", "credit"] {
        assert!(j.get(k).is_some(), "{k}");
    }
    // a rating or a base date without --rf-date is an error before any request; so is an unknown ticker
    let rating_only = crate::VerdictOptions { rating: Some("A-".into()), ..Default::default() };
    assert!(matches!(fa.verdict("IBM", 228.53, &rating_only), Err(crate::FaError::Data(m)) if m.contains("--rf-date")));
    assert!(matches!(fa.verdict("ZZZ", 228.53, &crate::VerdictOptions::default()), Err(crate::FaError::Data(m)) if m.contains("No driver model")));
    // a bad price or seed is an error, not a panic
    assert!(fa.verdict("IBM", -1.0, &crate::VerdictOptions::default()).is_err());
    assert!(fa.verdict("IBM", 10.0, &crate::VerdictOptions { seed: Some(-1), ..Default::default() }).is_err());
}

#[test]
fn money_format_is_the_format_of_run_dcf() {
    use super::cli::money as m;
    assert_eq!(m(1_288.8e9), "$1,288.8B");
    assert_eq!(m(443.1e9), "$443.1B");
    assert_eq!(m(-230.677e9), "$-230.7B");
    assert_eq!(m(0.0), "$0.0B");
}

// ---------------------------------------------------------------------------
// Backtest: the expected values come from `financial_analysis/backtest.py` (recorded
// before the Rust code was written) on three synthetic companies.
// ---------------------------------------------------------------------------

mod backtest_parity {
    use super::super::backtest::*;
    use super::super::classic::Company;
    use super::{close, hex};
    use serde_json::Value;

    fn bt_fixture(name: &str) -> String {
        let path = format!("{}/tests/fixtures/backtest/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    fn company(t: &str) -> Company {
        serde_json::from_str(&bt_fixture(&format!("{t}.json"))).unwrap()
    }

    fn hexo(v: &Value) -> Option<f64> {
        v.as_str().map(|_| hex(v))
    }

    #[test]
    fn every_recorded_run_matches_python() {
        let exp: Value = serde_json::from_str(&bt_fixture("expected.json")).unwrap();
        let mut checked = 0;
        for run in exp["runs"].as_array().unwrap() {
            let t = run["ticker"].as_str().unwrap();
            let (h, n, seed) = (run["horizon"].as_u64().unwrap() as usize, run["n"].as_u64().unwrap() as usize, run["seed"].as_u64().unwrap());
            let c = company(t);
            let r = run_backtest(&c, h, n, seed);
            let what = format!("{t} h{h} n{n} s{seed}");
            let vs = run["vintages"].as_array().unwrap();
            assert_eq!(r.vintages.len(), vs.len(), "{what}: vintages");
            for (v, e) in r.vintages.iter().zip(vs) {
                let w = format!("{what} {}", e["as_of"]);
                assert_eq!(v.params.as_of.as_deref(), e["as_of"].as_str(), "{w}");
                assert_eq!(v.params.base_fiscal_year as i64, e["base_fy"].as_i64().unwrap(), "{w}");
                close(v.params.base_revenue, hex(&e["base_revenue"]), 1e-9, &format!("{w} base"));
                close(v.params.growth, hex(&e["growth"]), 1e-9, &format!("{w} growth"));
                close(v.params.growth_std, hex(&e["growth_std"]), 1e-9, &format!("{w} growth_std"));
                close(v.params.fcf_margin_mean, hex(&e["m_mean"]), 1e-9, &format!("{w} m_mean"));
                close(v.params.fcf_margin_std, hex(&e["m_std"]), 1e-9, &format!("{w} m_std"));
                let window: Vec<i32> = e["window"].as_array().unwrap().iter().map(|y| y.as_i64().unwrap() as i32).collect();
                assert_eq!(v.params.window_years, window, "{w}");
                let flags: Vec<&str> = e["flags"].as_array().unwrap().iter().map(|f| f.as_str().unwrap()).collect();
                assert_eq!(v.params.data_flags, flags, "{w}");
                for band in 0..3 {
                    for (y, (a, b)) in v.projection.rev[band].iter().zip(e["rev"][band].as_array().unwrap()).enumerate() {
                        close(*a, hex(b), 1e-9, &format!("{w} rev band {band} year {y}"));
                    }
                    for (y, (a, b)) in v.projection.fcf[band].iter().zip(e["fcf"][band].as_array().unwrap()).enumerate() {
                        close(*a, hex(b), 1e-9, &format!("{w} fcf band {band} year {y}"));
                    }
                }
                let outs = e["outcomes"].as_array().unwrap();
                assert_eq!(v.outcomes.len(), outs.len(), "{w}");
                for (o, eo) in v.outcomes.iter().zip(outs) {
                    assert_eq!(o.target_fiscal_year as i64, eo["fy"].as_i64().unwrap());
                    assert_eq!(o.metric, eo["metric"].as_str().unwrap());
                    assert_eq!(o.in_band, eo["in_band"].as_bool(), "{w} in_band");
                    match (o.actual, hexo(&eo["actual"])) {
                        (Some(a), Some(b)) => close(a, b, 1e-12, &format!("{w} actual")),
                        (None, None) => {}
                        other => panic!("{w}: actual {other:?}"),
                    }
                    match (o.error_pct, hexo(&eo["error_pct"])) {
                        (Some(a), Some(b)) => close(a, b, 1e-9, &format!("{w} error")),
                        (None, None) => {}
                        other => panic!("{w}: error {other:?}"),
                    }
                    checked += 1;
                }
            }
            let skipped: Vec<(String, String)> =
                run["skipped"].as_array().unwrap().iter().map(|s| (s[0].as_str().unwrap().to_string(), s[1].as_str().unwrap().to_string())).collect();
            assert_eq!(r.skipped, skipped, "{what}: skipped");
            let cov = run["coverage"].as_array().unwrap();
            assert_eq!(r.coverage(), (cov[0].as_u64().unwrap() as usize, cov[1].as_u64().unwrap() as usize), "{what}: coverage");
            let bias = r.bias_by_horizon();
            for (h_, d) in run["bias"].as_object().unwrap() {
                for (m, pair) in d.as_object().unwrap() {
                    let (mean, k) = bias[&h_.parse::<i32>().unwrap()][m.as_str()];
                    close(mean, hex(&pair[0]), 1e-9, &format!("{what} bias {h_} {m}"));
                    assert_eq!(k as u64, pair[1].as_u64().unwrap());
                }
            }
            // the text output is the same as the output of scripts/backtest.py, byte for byte
            let text = if r.vintages.is_empty() { render_no_vintages(t, &r) } else { render(&c.company_name, &c.ticker, &r, n, seed) };
            assert_eq!(text, run["cli_stdout"].as_str().unwrap_or(""), "{what}: text");
        }
        assert!(checked > 200, "too few rows compared: {checked}");
    }
}

mod backtest_behavior {
    //! The tests of `tests/test_backtest.py`, moved to Rust: a synthetic company with exact 5%
    //! growth and an exact 20% FCF margin.
    use super::super::backtest::*;
    use super::super::classic::{Company, Period};

    const GROWTH: f64 = 0.05;
    const MARGIN: f64 = 0.20;
    const BASE: f64 = 100e9;

    fn annual(year: i32, revenue: Option<f64>, ocf: Option<f64>, capex: Option<f64>, filed: Option<&str>, form: &str) -> Period {
        Period {
            form: form.into(),
            period_end: format!("{year}-12-31"),
            filed_date: filed.map_or(format!("{}-02-15", year + 1), str::to_string),
            revenue,
            operating_cash_flow: ocf,
            capex,
            ..Period::default()
        }
    }

    fn clean(year: i32) -> Period {
        let r = BASE * (1.0 + GROWTH).powi(year - 2018);
        annual(year, Some(r), Some(r * (MARGIN + 0.02)), Some(-r * 0.02), None, "10-K")
    }

    fn company(n_years: i32, ticker: &str) -> Company {
        Company { ticker: ticker.into(), company_name: "Synthetic Co".into(), periods: (0..n_years).map(|i| clean(2018 + i)).collect() }
    }

    fn as_of(fy: i32) -> String {
        add_days(&format!("{fy}-12-31"), FILING_LAG_DAYS)
    }

    fn years(v: &[&Period]) -> Vec<i32> {
        v.iter().map(|p| p.period_end[..4].parse().unwrap()).collect()
    }

    #[test]
    fn add_days_crosses_month_and_year_ends() {
        assert_eq!(add_days("2020-12-31", 90), "2021-03-31");
        assert_eq!(add_days("2023-12-31", 90), "2024-03-30");
        assert_eq!(add_days("2019-01-31", 1), "2019-02-01");
    }

    #[test]
    fn vintage_uses_filed_date_not_period_end() {
        let mut c = company(4, "SYN");
        c.periods[3].filed_date = "2022-09-01".into();
        assert_eq!(years(&vintage_periods(&c, "2022-03-31")), vec![2018, 2019, 2020]);
        assert_eq!(years(&vintage_periods(&c, "2022-09-01")), vec![2018, 2019, 2020, 2021]);
    }

    #[test]
    fn phantom_rows_and_quarterly_forms_are_not_used() {
        let mut c = company(4, "SYN");
        c.periods.push(annual(2016, Some(1e9), None, None, Some("2016-06-30"), "10-K"));
        c.periods.push(annual(2015, None, Some(1e9), None, Some("2015-06-30"), "10-K"));
        c.periods.push(annual(2014, None, None, None, Some("2014-06-30"), "10-K"));
        c.periods.push(annual(2021, Some(30e9), Some(8e9), Some(-1e9), Some("2021-05-01"), "10-Q"));
        assert_eq!(years(&vintage_periods(&c, "2030-01-01")), vec![2018, 2019, 2020, 2021]);
    }

    #[test]
    fn a_10ka_replaces_the_10k_of_the_same_year() {
        let mut c = company(4, "SYN");
        c.periods.push(annual(2020, Some(1.0e9), Some(1.0e8), Some(-1.0e7), Some("2021-09-01"), "10-K/A"));
        let v = vintage_periods(&c, "2030-01-01");
        assert_eq!(v.len(), 4);
        assert_eq!(v[2].revenue, Some(1.0e9));
    }

    #[test]
    fn params_recover_growth_and_margin_and_floor_the_stds() {
        let c = company(8, "SYN");
        let p = mechanical_params(&vintage_periods(&c, "2030-01-01"), "SYN", Some("2030-01-01")).unwrap();
        assert!((p.growth - GROWTH).abs() < 1e-9);
        assert!((p.fcf_margin_mean - MARGIN).abs() < 1e-9);
        assert_eq!(p.growth_std, 0.01);
        assert_eq!(p.fcf_margin_std, 0.01);
        assert_eq!(p.base_fiscal_year, 2025);
        assert_eq!(p.window_years, vec![2021, 2022, 2023, 2024, 2025]);
    }

    #[test]
    fn params_need_three_periods() {
        let c = company(2, "SYN");
        let e = mechanical_params(&vintage_periods(&c, "2030-01-01"), "SYN", None).unwrap_err();
        assert!(e.0.contains("usable annual period"), "{e}");
    }

    #[test]
    fn params_have_no_forward_information() {
        let (full, cut) = (company(8, "SYN"), company(4, "SYN"));
        let d = as_of(2021);
        let a = mechanical_params(&vintage_periods(&full, &d), "SYN", Some(&d)).unwrap();
        let b = mechanical_params(&vintage_periods(&cut, &d), "SYN", Some(&d)).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn the_ibm_flag_shows_only_when_the_window_has_a_flagged_year() {
        let c = company(8, "IBM");
        let flagged = mechanical_params(&vintage_periods(&c, &as_of(2020)), "IBM", None).unwrap();
        assert_eq!(flagged.data_flags.len(), 1);
        let clear = mechanical_params(&vintage_periods(&c, "2030-01-01"), "IBM", None).unwrap();
        assert!(clear.data_flags.is_empty());
    }

    #[test]
    fn projection_median_is_near_the_deterministic_path() {
        let c = company(8, "SYN");
        let p = mechanical_params(&vintage_periods(&c, "2030-01-01"), "SYN", None).unwrap();
        let pr = project(&p, 3, 20_000, 7);
        assert_eq!(pr.target_fiscal_years, vec![2026, 2027, 2028]);
        for t in 1..=3usize {
            let det = p.base_revenue * (1.0 + GROWTH).powi(t as i32);
            assert!((pr.rev[1][t - 1] / det - 1.0).abs() < 0.01);
            assert!((pr.fcf[1][t - 1] / (det * MARGIN) - 1.0).abs() < 0.01);
            assert!(pr.rev[0][t - 1] < pr.rev[1][t - 1] && pr.rev[1][t - 1] < pr.rev[2][t - 1]);
        }
        assert_eq!(pr, project(&p, 3, 20_000, 7), "same seed, same result");
    }

    #[test]
    fn backtest_joins_actuals_and_scores_them() {
        let r = run_backtest(&company(8, "SYN"), 3, 20_000, 7);
        let v = r.vintages.iter().find(|v| v.params.as_of.as_deref() == Some(as_of(2020).as_str())).unwrap();
        let o = v.outcomes.iter().find(|o| o.target_fiscal_year == 2021 && o.metric == "revenue").unwrap();
        assert!((o.actual.unwrap() / (BASE * 1.05f64.powi(3)) - 1.0).abs() < 1e-12);
        assert!((o.error_pct.unwrap() - (o.actual.unwrap() / o.p50 - 1.0) * 100.0).abs() < 1e-12);
        assert!(o.error_pct.unwrap().abs() < 2.0);
        assert_eq!(o.in_band, Some(true));
        // the last vintage has nothing reported
        let last = r.vintages.last().unwrap();
        assert!(last.outcomes.iter().all(|o| o.actual.is_none() && o.error_pct.is_none() && o.in_band.is_none()));
        // skipped vintages have a reason, and the others are in order
        assert_eq!(r.skipped.iter().map(|s| s.0.clone()).collect::<Vec<_>>(), vec![as_of(2018), as_of(2019)]);
        assert_eq!(r.vintages.iter().map(|v| v.params.as_of.clone().unwrap()).collect::<Vec<_>>(), (2020..=2025).map(as_of).collect::<Vec<_>>());
        // a deterministic history is covered everywhere
        let (inside, total) = r.coverage();
        assert!(total > 0 && inside == total);
        let bias = r.bias_by_horizon();
        assert_eq!(bias.keys().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(r.vintages.len() * 3 * 2, r.vintages.iter().map(|v| v.outcomes.len()).sum::<usize>());
    }

    #[test]
    fn a_shock_outside_the_band_is_found() {
        let mut c = company(8, "SYN");
        c.periods[6].revenue = c.periods[6].revenue.map(|r| r * 1.6);
        let r = run_backtest(&c, 3, 5_000, 7);
        let v = r.vintages.iter().find(|v| v.params.as_of.as_deref() == Some(as_of(2021).as_str())).unwrap();
        let o = v.outcomes.iter().find(|o| o.target_fiscal_year == 2024 && o.metric == "revenue").unwrap();
        assert_eq!(o.in_band, Some(false));
        assert!(o.error_pct.unwrap() > 0.0);
    }
}
