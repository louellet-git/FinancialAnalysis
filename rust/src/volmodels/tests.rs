//! Tests with fixed answers (ticket T-20261120-vol-1-heston-bates).

use super::base::*;
use super::calib::{at_bound, fit, levenberg_marquardt, lm_core, FitOptions, LmStatus};
use super::chain::{build, Filters};
use super::cx::Cx;
use super::fourier::*;
use super::models::*;
use crate::market::option_iv::{black76, implied_vol};
use crate::market::options::{OptionChainSnapshot, OptionQuote};
use crate::ratemodels::rng::Rng;
use crate::ratemodels::sim::mean_sd_se;
use rayon::prelude::*;
use std::f64::consts::PI;

/// Chan (2017), arXiv:1706.06709v3, appendix B.
const CHAN: Heston = Heston { v0: 0.0175, theta: 0.0398, kappa: 1.5768, xi: 0.5751, rho: -0.5711 };

fn call(m: &impl CharModel, f: f64, k: f64, t: f64) -> f64 {
    call_over_df(m, f, k, t)
}

#[test]
fn t1_published_values_of_chan() {
    let c1 = call(&CHAN, 100.0, 100.0, 1.0);
    let c10 = call(&CHAN, 100.0, 100.0, 10.0);
    println!("T=1: {c1:.9} (error {:.2e}); T=10: {c10:.9} (error {:.2e})", c1 - 5.785155450, c10 - 22.318945791);
    // The published T=1 value differs from the converged integral by 1.6e-8 (see the model document, "Checks of the price").
    assert!((c1 - 5.785155450).abs() < 2e-8, "T=1 {c1}");
    // The converged value (an independent Gil-Pelaez price of the validator gives the same 12 digits).
    assert!((c1 - 5.785155434376).abs() < 1e-10, "T=1 {c1}");
    assert!((c10 - 22.318945791).abs() < 1e-8, "T=10 {c10}");
}

#[test]
fn t2_xi_near_zero_is_black76() {
    for &(v, kappa, t) in &[(0.04, 1.5, 1.0), (0.09, 3.0, 0.25), (0.0225, 0.5, 2.0)] {
        let m = Heston { v0: v, theta: v, kappa, xi: 1e-12, rho: -0.5 };
        for &k in &[70.0, 90.0, 100.0, 110.0, 140.0] {
            let got = call(&m, 100.0, k, t);
            let want = black76(true, 100.0, k, t, 1.0, v.sqrt());
            assert!((got - want).abs() < 1e-10, "v={v} t={t} k={k}: {got} against {want}");
        }
    }
}

/// Gil-Pelaez put price (without df): a different formula and a fixed grid, as an independent check of the Lewis integral.
fn gil_pelaez_put<M: CharModel>(m: &M, f: f64, k: f64, t: f64) -> f64 {
    let s = (k / f).ln();
    let (x, w) = gauss_legendre(16);
    let (mut i1, mut i2) = (0.0, 0.0);
    for p in 0..3000 {
        let (a, b) = (p as f64 * 0.5, (p + 1) as f64 * 0.5);
        for j in 0..16 {
            let u = 0.5 * (a + b) + 0.5 * (b - a) * x[j];
            let wt = 0.5 * (b - a) * w[j];
            let e = Cx::new((u * s).cos(), -(u * s).sin());
            let iu = Cx::new(0.0, u);
            i2 += wt * (e * m.log_cf(Cx::real(u), t) / iu).re;
            i1 += wt * (e * m.log_cf(Cx::new(u, -1.0), t) / iu).re;
        }
    }
    let (p1, p2) = (0.5 + i1 / PI, 0.5 + i2 / PI);
    k * (1.0 - p2) - f * (1.0 - p1)
}

#[test]
fn t3_parity_martingale_and_fft() {
    let bates = Bates { heston: CHAN, lambda: 0.6, mu_j: -0.12, delta_j: 0.15 };
    // E[F_T] = F: the characteristic function is 1 at z = -i.
    for t in [0.01, 0.5, 1.0, 10.0] {
        for cf in [CHAN.log_cf(Cx::new(0.0, -1.0), t), bates.log_cf(Cx::new(0.0, -1.0), t)] {
            assert!((cf - Cx::ONE).abs() < 1e-12, "t={t} cf(-i) = {cf:?}");
        }
    }
    for (name, t) in [("heston", 1.0), ("heston", 0.5), ("heston", 5.0)] {
        for &k in &[80.0, 100.0, 125.0] {
            let (c, p) = (call_over_df(&CHAN, 100.0, k, t), put_over_df(&CHAN, 100.0, k, t));
            assert!((c - p - (100.0 - k)).abs() < 1e-10, "{name} parity t={t} k={k}");
            let pg = gil_pelaez_put(&CHAN, 100.0, k, t);
            assert!((pg - p).abs() < 1e-9, "{name} t={t} k={k}: Gil-Pelaez put {pg}, Lewis put {p}");
        }
    }
    let strikes = [70.0, 85.0, 95.0, 100.0, 105.0, 120.0, 150.0];
    for t in [0.25, 1.0] {
        let fft = carr_madan_calls(&CHAN, 100.0, &strikes, t, 1.5, 1 << 14, 0.1);
        for (k, c) in strikes.iter().zip(&fft) {
            let want = call(&CHAN, 100.0, *k, t);
            assert!((c - want).abs() < 1e-6, "FFT t={t} k={k}: {c} against {want}");
        }
    }
}

#[test]
fn t4_bates_special_cases() {
    let none = Bates { heston: CHAN, lambda: 0.0, mu_j: -0.2, delta_j: 0.3 };
    for &k in &[80.0, 100.0, 130.0] {
        let (a, b) = (call(&none, 100.0, k, 0.7), call(&CHAN, 100.0, k, 0.7));
        assert!((a - b).abs() < 1e-12, "lambda = 0: {a} against {b}");
    }
    // xi near 0: Merton jump-diffusion, a Poisson sum of Black-76 prices.
    let (v, lambda, mu, delta, t) = (0.04, 0.8, -0.1, 0.15, 0.5);
    let m = Bates { heston: Heston { v0: v, theta: v, kappa: 2.0, xi: 1e-12, rho: -0.5 }, lambda, mu_j: mu, delta_j: delta };
    let mean_jump = m.mean_jump();
    for &k in &[70.0, 90.0, 100.0, 115.0, 150.0] {
        let mut want = 0.0;
        let mut weight = (-lambda * t).exp();
        for n in 0..80 {
            let nf = n as f64;
            let fwd = 100.0 * (-lambda * mean_jump * t + nf * (mu + 0.5 * delta * delta)).exp();
            want += weight * black76(true, fwd, k, t, 1.0, ((v * t + nf * delta * delta) / t).sqrt());
            weight *= lambda * t / (nf + 1.0);
        }
        let got = call(&m, 100.0, k, t);
        assert!((got - want).abs() < 1e-8, "Merton k={k}: {got} against {want}");
    }
}

/// One path of the quadratic exponential scheme of Andersen (2008), with jumps of Bates (`lambda = 0` is Heston). Gives ln(F_T / F_0).
fn qe_path(b: &Bates, t: f64, steps: usize, rng: &mut Rng) -> f64 {
    let h = &b.heston;
    let dt = t / steps as f64;
    let (e, rho_xi) = ((-h.kappa * dt).exp(), h.rho / h.xi);
    let (k1, k2) = (0.5 * dt * (h.kappa * rho_xi - 0.5) - rho_xi, 0.5 * dt * (h.kappa * rho_xi - 0.5) + rho_xi);
    let (k3, k4) = (0.5 * dt * (1.0 - h.rho * h.rho), 0.5 * dt * (1.0 - h.rho * h.rho));
    let a_coef = k2 + 0.5 * k4;
    let (mut x, mut v) = (0.0, h.v0);
    for _ in 0..steps {
        let m = h.theta + (v - h.theta) * e;
        let s2 = v * h.xi * h.xi * e / h.kappa * (1.0 - e) + h.theta * h.xi * h.xi / (2.0 * h.kappa) * (1.0 - e) * (1.0 - e);
        let psi = s2 / (m * m);
        // M = E[exp(A v')] makes each step a martingale (Andersen 2008, section 4.2).
        let (vn, ln_m) = if psi <= 1.5 {
            let inv = 2.0 / psi;
            let b2 = inv - 1.0 + (inv * (inv - 1.0)).sqrt();
            let a = m / (1.0 + b2);
            let z = rng.normal() + b2.sqrt();
            (a * z * z, a_coef * b2 * a / (1.0 - 2.0 * a_coef * a) - 0.5 * (1.0 - 2.0 * a_coef * a).ln())
        } else {
            let p = (psi - 1.0) / (psi + 1.0);
            let beta = (1.0 - p) / m;
            let u = rng.uniform();
            let vn = if u <= p { 0.0 } else { ((1.0 - p) / (1.0 - u)).ln() / beta };
            (vn, (p + beta * (1.0 - p) / (beta - a_coef)).ln())
        };
        let k0 = -ln_m - (k1 + 0.5 * k3) * v;
        x += k0 + k1 * v + k2 * vn + (k3 * v + k4 * vn).sqrt() * rng.normal();
        if b.lambda > 0.0 {
            // Poisson by inversion; the mean of a step is small.
            let (mut n, mut p, u) = (0u32, (-b.lambda * dt).exp(), rng.uniform());
            let mut cum = p;
            while u > cum && n < 50 {
                n += 1;
                p *= b.lambda * dt / n as f64;
                cum += p;
            }
            if n > 0 {
                x += n as f64 * b.mu_j + b.delta_j * (n as f64).sqrt() * rng.normal();
            }
            x -= b.lambda * b.mean_jump() * dt;
        }
        v = vn;
    }
    x
}

#[test]
fn t5_monte_carlo_within_three_standard_errors() {
    let paths = 1usize << 18;
    let bates = Bates { heston: CHAN, lambda: 0.5, mu_j: -0.1, delta_j: 0.12 };
    let heston = Bates { heston: CHAN, lambda: 0.0, mu_j: 0.0, delta_j: 0.0 };
    let started = std::time::Instant::now();
    for (name, model, seed) in [("heston", heston, 21u64), ("bates", bates, 22u64)] {
        let xs: Vec<f64> = (0..paths).into_par_iter().map(|p| qe_path(&model, 1.0, 128, &mut Rng::for_path(seed, p as u64))).collect();
        let mean_f = xs.iter().map(|x| x.exp()).sum::<f64>() / paths as f64;
        assert!((mean_f - 1.0).abs() < 0.01, "{name}: mean of F_T / F_0 = {mean_f}");
        for &k in &[90.0, 100.0, 115.0] {
            let pay: Vec<f64> = xs.iter().map(|x| (100.0 * x.exp() - k).max(0.0)).collect();
            let (mc, _, se) = mean_sd_se(&pay);
            let fourier = if name == "heston" { call(&CHAN, 100.0, k, 1.0) } else { call(&model, 100.0, k, 1.0) };
            println!("MC {name} K={k}: {mc:.4} +- {se:.4} (se), Fourier {fourier:.4}, {:.2} se", (mc - fourier) / se);
            assert!((mc - fourier).abs() < 3.0 * se, "{name} K={k}: MC {mc} (se {se}) against Fourier {fourier}");
        }
    }
    println!("MC run time {:.2} s", started.elapsed().as_secs_f64());
}

/// A series of OTM quotes priced by the model `m`, with a spread of 0.2 vol points around the model volatility.
fn synthetic_series<M: CharModel>(m: &M, f: f64, days: i64, half: i32) -> Series {
    let (t, df) = (days as f64 / 365.0, 1.0);
    let ks: Vec<f64> = (-half..=half).map(|i| f * (0.2 * t.sqrt() * i as f64 * 2.0 / half as f64).exp()).collect();
    let mut s = Series { root: "SYN".into(), expiry: format!("+{days}d"), days, t, forward: f, df, quotes: vec![] };
    let prototype = Series { quotes: ks.iter().map(|&k| Quote { strike: k, call: k >= f, mid: 0.0, iv_mid: 0.2, iv_bid: 0.199, iv_ask: 0.201 }).collect(), ..s.clone() };
    let prices = prototype.model_prices(m);
    for (&k, p) in ks.iter().zip(&prices) {
        let call = k >= f;
        if let Some(iv) = implied_vol(call, *p, f, k, t, df) {
            s.quotes.push(Quote { strike: k, call, mid: *p, iv_mid: iv, iv_bid: iv - 0.001, iv_ask: iv + 0.001 });
        }
    }
    s
}

#[test]
fn t6_synthetic_surface_is_recovered() {
    let truth = Heston { v0: 0.04, theta: 0.05, kappa: 2.0, xi: 0.5, rho: -0.7 };
    let set = QuoteSet { series: [7, 14, 30, 60, 90, 180, 270, 365, 540, 730].iter().map(|&d| synthetic_series(&truth, 100.0, d, 8)).collect() };
    let started = std::time::Instant::now();
    let r = fit::<Heston>(&set, &FitOptions::default()).unwrap();
    let secs = started.elapsed().as_secs_f64();
    let errs = iv_errors(&r.model, &set);
    let worst = errs.iter().map(|e| e.max_points).fold(0.0, f64::max);
    println!("fit of {} quotes in {secs:.2} s: {:?}, worst error {worst:.5} vol points, loss {:.3e}", set.n_quotes(), r.model, r.loss);
    for (name, got, want) in [("v0", r.model.v0, truth.v0), ("theta", r.model.theta, truth.theta), ("kappa", r.model.kappa, truth.kappa), ("xi", r.model.xi, truth.xi), ("rho", r.model.rho, truth.rho)] {
        assert!(((got - want) / want).abs() < 0.01, "{name}: {got} against {want}");
    }
    assert!(worst < 0.01, "worst error {worst} vol points");
}

fn snapshot_quote(sym: &str, expiry: &str, k: f64, call: bool, bid: Option<f64>, ask: Option<f64>) -> OptionQuote {
    OptionQuote { contract_symbol: format!("{sym}261218{}{:08}", if call { "C" } else { "P" }, (k * 1000.0) as i64), option_type: if call { "call" } else { "put" }.into(), expiry: expiry.into(), strike: k, last_price: None, bid, ask, volume: None, open_interest: None, implied_volatility: None, in_the_money: None }
}

#[test]
fn chain_filters_count_every_dropped_quote() {
    // 2026-11-20 to 2026-12-18: 28 days. Prices from Black-76, forward 100, vol 0.2, spread 0.4 vol points.
    let (f, t) = (100.0, 28.0 / 365.0);
    let df = (-0.04f64 * t).exp();
    let mut quotes = Vec::new();
    for i in -6..=6 {
        let k = 101.0 + 2.0 * i as f64;
        for call in [true, false] {
            let (b, a) = (black76(call, f, k, t, df, 0.198), black76(call, f, k, t, df, 0.202));
            quotes.push(snapshot_quote("ABC", "2026-12-18", k, call, Some(b), Some(a)));
        }
    }
    // One of each bad kind.
    quotes.push(snapshot_quote("ABC", "2026-12-18", 104.5, true, None, Some(1.0)));
    quotes.push(snapshot_quote("ABC", "2026-12-18", 105.5, true, Some(2.0), Some(1.9)));
    quotes.push(snapshot_quote("ABC", "2026-12-18", 106.5, true, Some(0.5), Some(1.5)));
    quotes.push(snapshot_quote("ABC", "2026-11-20", 100.0, true, Some(1.0), Some(1.1)));
    // A series with too few quotes left (3 out-of-the-money quotes), and a series with no pair for a forward (2 quotes).
    let t2 = 56.0 / 365.0;
    let df2 = (-0.04f64 * t2).exp();
    for k in [99.0, 101.0, 103.0] {
        for call in [true, false] {
            let (b, a) = (black76(call, f, k, t2, df2, 0.198), black76(call, f, k, t2, df2, 0.202));
            quotes.push(snapshot_quote("ABC", "2027-01-15", k, call, Some(b), Some(a)));
        }
    }
    for k in [100.0, 101.0] {
        quotes.push(snapshot_quote("ABC", "2027-02-19", k, true, Some(1.0), Some(1.1)));
    }
    let snap = OptionChainSnapshot { ticker: "ABC".into(), snapshot_date: "2026-11-20".into(), spot_price: Some(100.0), quotes, source: None, feed: None, quote_kind: None, note: None, snapshot_time: None };
    let mut rate = |_: &str, _: f64| -> Result<f64, String> { Ok(0.04) };
    let (set, c) = build(&snap, &mut rate, &Filters::for_symbol("ABC", None)).unwrap();
    assert_eq!(c.total, 26 + 4 + 6 + 2);
    assert_eq!(c.no_forward, 2);
    assert_eq!(c.expired_or_today, 1);
    assert_eq!(c.no_bid_or_ask, 1);
    assert_eq!(c.ask_not_above_bid, 1);
    assert_eq!(c.spread_too_wide, 1);
    assert_eq!(c.in_the_money, 13 + 3, "one side of each strike is in the money");
    assert_eq!(c.short_series, 3);
    assert_eq!(c.kept, 13);
    let sum = c.expired_or_today + c.beyond_expiry_max + c.no_forward + c.no_bid_or_ask + c.ask_not_above_bid + c.in_the_money + c.no_iv + c.spread_too_wide + c.short_series + c.kept;
    assert_eq!(sum, c.total, "each quote has one fate");
    let s = &set.series[0];
    assert!((s.forward - 100.0).abs() < 0.05, "forward {}", s.forward);
    assert!(s.quotes.iter().all(|q| (q.iv_mid - 0.2).abs() < 1e-3));
}


#[test]
fn cf_matches_the_riccati_equations() {
    // An independent route to the characteristic function: RK4 on D' = -q/2 - b D + xi^2 D^2 / 2, C' = kappa theta D.
    let m = CHAN;
    for &(u, t) in &[(0.0, 1.0), (1.0, 1.0), (10.0, 1.0), (50.0, 1.0), (3.0, 10.0)] {
        let z = Cx::new(u, -0.5);
        let iz = Cx::I * z;
        let q = z * z + iz;
        let b = Cx::real(m.kappa) - iz * (m.rho * m.xi);
        let (mut d, mut c) = (Cx::ZERO, Cx::ZERO);
        let n = 100_000;
        let h = t / n as f64;
        let f = |d: Cx| -(q * 0.5) - b * d + d * d * (0.5 * m.xi * m.xi);
        for _ in 0..n {
            let k1 = f(d);
            let k2 = f(d + k1 * (h / 2.0));
            let k3 = f(d + k2 * (h / 2.0));
            let k4 = f(d + k3 * h);
            let dn = d + (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (h / 6.0);
            let dm = d + k1 * (h / 4.0) + k2 * (h / 4.0);
            c = c + (d + dm * 4.0 + dn) * (h / 6.0 * m.kappa * m.theta);
            d = dn;
        }
        let (ode, got) = ((c + d * m.v0).exp(), m.log_cf(z, t));
        assert!((ode - got).abs() < 1e-12, "u={u} t={t}: {ode:?} against {got:?}");
    }
}

/// Speed of the pricer and of a full fit (run: `cargo test --manifest-path rust/Cargo.toml --lib volmodels::tests::speed -- --ignored --nocapture`).
/// It prints times and asserts nothing: it is a report, not a check.
#[test]
#[ignore = "timing report, not a check"]
fn speed_report() {
    let truth = Heston { v0: 0.04, theta: 0.05, kappa: 2.0, xi: 0.5, rho: -0.7 };
    for days in [7, 30, 90, 365, 730] {
        let s = synthetic_series(&truth, 100.0, days, 50);
        let started = std::time::Instant::now();
        let reps = 200;
        for _ in 0..reps {
            std::hint::black_box(s.model_prices(&truth));
        }
        println!("one expiry, {days} days, {} strikes: {:.3} ms (one thread)", s.quotes.len(), 1e3 * started.elapsed().as_secs_f64() / reps as f64);
    }
    let days: Vec<i64> = (0..40).map(|i| 5 + 18 * i).collect();
    let set = QuoteSet { series: days.iter().map(|&d| synthetic_series(&truth, 100.0, d, 137)).collect() };
    let started = std::time::Instant::now();
    let r = fit::<Heston>(&set, &FitOptions::default()).unwrap();
    println!("heston fit, {} series, {} quotes: {:.2} s, {} threads, LM iterations {}, rho {:.4}", set.series.len(), set.n_quotes(), started.elapsed().as_secs_f64(), rayon::current_num_threads(), r.iterations, r.model.rho);
    let bates = Bates { heston: truth, lambda: 0.3, mu_j: -0.1, delta_j: 0.1 };
    let set = QuoteSet { series: days.iter().map(|&d| synthetic_series(&bates, 100.0, d, 137)).collect() };
    let started = std::time::Instant::now();
    let r = fit::<Bates>(&set, &FitOptions::default()).unwrap();
    println!("bates fit, {} series, {} quotes: {:.2} s, LM iterations {}, {:?}", set.series.len(), set.n_quotes(), started.elapsed().as_secs_f64(), r.iterations, r.model);
}

#[test]
fn lm_reports_a_run_that_did_not_converge() {
    let truth = Heston { v0: 0.04, theta: 0.05, kappa: 2.0, xi: 0.5, rho: -0.7 };
    let set = QuoteSet { series: [30, 90, 365].iter().map(|&d| synthetic_series(&truth, 100.0, d, 8)).collect() };
    let far = Heston { v0: 0.01, theta: 0.2, kappa: 8.0, xi: 1.5, rho: -0.2 }.to_vec();
    let (_, _, iters, status) = levenberg_marquardt::<Heston>(&set, &far, 2);
    assert_eq!((iters, status), (2, LmStatus::IterationCap), "two iterations do not converge from a far start");
    assert_ne!(status, LmStatus::Converged);
    let (_, f, _, status) = levenberg_marquardt::<Heston>(&set, &far, 60);
    assert_eq!(status, LmStatus::Converged, "loss {f}");
}

#[test]
fn bound_marks_and_non_finite_cf() {
    let b = Heston::bounds();
    assert_eq!(at_bound(&[0.04, 0.05, 20.0, 1.0, -0.5], &b), vec![false, false, true, false, false]);
    assert_eq!(at_bound(&[0.04, 0.05, 2.0, 3.0, 0.0], &b), vec![false, false, false, true, true]);
    let bad = Heston { v0: 0.04, theta: 0.05, kappa: f64::NAN, xi: 0.5, rho: -0.5 };
    assert!(lewis_j(&bad, 1.0, &[0.0, 0.1]).iter().all(|v| v.is_nan()), "a NaN characteristic function gives NaN, not a number");
    let set = QuoteSet { series: vec![synthetic_series(&CHAN, 100.0, 90, 8)] };
    assert!(fit::<Heston>(&set, &FitOptions::default()).is_ok());
    assert!(loss(&bad, &set).is_nan());
}

#[test]
fn bates_surface_is_recovered_with_a_loose_tolerance() {
    let truth = Bates { heston: Heston { v0: 0.04, theta: 0.05, kappa: 2.0, xi: 0.5, rho: -0.7 }, lambda: 0.3, mu_j: -0.1, delta_j: 0.1 };
    let set = QuoteSet { series: [7, 14, 30, 60, 90, 180, 270, 365, 540, 730].iter().map(|&d| synthetic_series(&truth, 100.0, d, 8)).collect() };
    let r = fit::<Bates>(&set, &FitOptions::default()).unwrap();
    let worst = iv_errors(&r.model, &set).iter().map(|e| e.max_points).fold(0.0, f64::max);
    println!("bates recovery: {:?}, worst {worst:.5} vol points, {:?}", r.model, r.status);
    assert!(worst < 0.05, "worst error {worst} vol points");
    for (name, got, want) in [("v0", r.model.heston.v0, 0.04), ("theta", r.model.heston.theta, 0.05), ("kappa", r.model.heston.kappa, 2.0), ("xi", r.model.heston.xi, 0.5), ("rho", r.model.heston.rho, -0.7), ("lambda", r.model.lambda, 0.3), ("mu_j", r.model.mu_j, -0.1), ("delta_j", r.model.delta_j, 0.1)] {
        assert!(((got - want) / want).abs() < 0.05, "{name}: {got} against {want}");
    }
}

// ---- SABR (ticket T-20261120-vol-2-sabr-localvol, part 1) ----

use super::sabr::{bounds as sabr_bounds, fit_series as sabr_fit_series, residuals as sabr_residuals, Sabr};

/// A series whose mid vols are the SABR vols of `m` (an exact surface, no noise). The spread is 0.2 vol points.
fn sabr_series(m: &Sabr, f: f64, days: i64, n: usize) -> Series {
    let t = days as f64 / 365.0;
    let quotes = (0..n)
        .map(|i| {
            let k = f * (-0.25 + 0.45 * i as f64 / (n - 1) as f64).exp();
            let v = m.implied_vol(f, k, t);
            Quote { strike: k, call: k >= f, mid: 1.0, iv_mid: v, iv_bid: v - 0.001, iv_ask: v + 0.001 }
        })
        .collect();
    Series { root: "TST".into(), expiry: "2027-01-01".into(), days, t, forward: f, df: 1.0, quotes }
}

#[test]
fn sabr_1_nu_zero_beta_one_is_flat_at_alpha() {
    for &(rho, t) in &[(-0.7, 0.1), (0.0, 1.0), (0.4, 5.0)] {
        let m = Sabr { alpha: 0.23, beta: 1.0, rho, nu: 0.0 };
        for &k in &[40.0, 80.0, 99.999, 100.0, 100.001, 120.0, 300.0] {
            let v = m.implied_vol(100.0, k, t);
            assert!((v - 0.23).abs() < 1e-12, "rho {rho} k {k}: {v}");
        }
    }
}

#[test]
fn sabr_2_atm_vol_is_the_published_atm_expansion() {
    // Hagan et al. (2002) eq. (2.18) by hand: beta = 1, F = 100, alpha = 0.2, rho = -0.5, nu = 0.6, T = 1:
    // 0.2 * (1 + (0.25 * (-0.5) * 0.2 * 0.6 + (2 - 3 * 0.25) / 24 * 0.36)) = 0.2 * (1 - 0.015 + 0.01875) = 0.20075.
    let m = Sabr { alpha: 0.2, beta: 1.0, rho: -0.5, nu: 0.6 };
    assert!((m.atm_vol(100.0, 1.0) - 0.20075).abs() < 1e-14);
    assert!((m.implied_vol(100.0, 100.0, 1.0) - 0.20075).abs() < 1e-12, "{}", m.implied_vol(100.0, 100.0, 1.0));
    // The general formula tends to the ATM value from both sides, for several beta (the branch x -> 0 and the near branch agree).
    for &beta in &[1.0, 0.7, 0.5, 0.0] {
        let m = Sabr { alpha: 0.2 * 100f64.powf(1.0 - beta), beta, rho: -0.3, nu: 0.5 };
        let atm = m.atm_vol(100.0, 0.5);
        // A strike 1e-5 away moves the vol by (skew about 0.1) x 1e-5 = 1e-6 at most, so the tolerance there is 1e-5.
        for &(k, tol) in &[(100.0, 1e-12), (100.0 * (1.0 + 1e-9), 1e-8), (100.0 * (1.0 - 1e-9), 1e-8), (100.0 * (1.0 + 1e-5), 1e-5)] {
            assert!((m.implied_vol(100.0, k, 0.5) - atm).abs() < tol, "beta {beta} k {k}");
        }
        assert!((m.implied_vol(100.0, 100.0, 0.5) - atm).abs() < 1e-12, "beta {beta}");
    }
}

#[test]
fn sabr_2b_limits_of_the_obloj_zero_order_term() {
    // nu = 0, beta < 1: I0 = alpha (1 - beta) x / (F^(1-beta) - K^(1-beta)) (Obloj 2008, Table 1), T -> 0 (beta = 0.5, no time term when T = 0).
    let m = Sabr { alpha: 2.0, beta: 0.5, rho: -0.4, nu: 0.0 };
    let (f, k) = (100.0f64, 80.0f64);
    let want = 2.0 * 0.5 * (f / k).ln() / (f.sqrt() - k.sqrt());
    assert!((m.implied_vol(f, k, 0.0) - want).abs() < 1e-14);
    // beta -> 1 is continuous: beta = 1 - 1e-9 gives the beta = 1 value.
    let a = Sabr { alpha: 0.2, beta: 1.0, rho: -0.5, nu: 0.6 }.implied_vol(100.0, 90.0, 1.0);
    let b = Sabr { alpha: 0.2 * 100f64.powf(1e-9), beta: 1.0 - 1e-9, rho: -0.5, nu: 0.6 }.implied_vol(100.0, 90.0, 1.0);
    assert!((a - b).abs() < 1e-8, "{a} {b}");
}

#[test]
fn sabr_3_round_trip_gives_the_parameters_back() {
    for &(beta, alpha) in &[(1.0, 0.18), (0.5, 0.18 * 10.0)] {
        for &days in &[30, 180] {
            let truth = Sabr { alpha, beta, rho: -0.65, nu: 0.8 };
            let s = sabr_series(&truth, 100.0, days, 25);
            let r = sabr_fit_series(&s, beta, 100).unwrap();
            println!("beta {beta} days {days}: {:?} {:?} rms {:.2e}", r.model, r.status, r.rms_points);
            assert_eq!(r.status, LmStatus::Converged);
            assert!(r.rms_points < 1e-6 && r.max_points < 1e-6, "{} {}", r.rms_points, r.max_points);
            assert!((r.model.alpha - alpha).abs() / alpha < 1e-5 && (r.model.rho - truth.rho).abs() < 1e-4 && (r.model.nu - truth.nu).abs() / truth.nu < 1e-4, "{:?}", r.model);
            assert_eq!(r.at_bound, [false, false, false]);
        }
    }
}

#[test]
fn sabr_4_a_fit_that_cannot_converge_is_reported() {
    let truth = Sabr { alpha: 0.18, beta: 1.0, rho: -0.65, nu: 0.8 };
    let s = sabr_series(&truth, 100.0, 90, 25);
    let b = sabr_bounds(100.0, 1.0);
    let far = [0.05, 0.5, 3.0];
    let resid = |p: &[f64]| sabr_residuals(&s, 1.0, p);
    let (_, _, iters, status) = lm_core(&resid, &b, &far, 1);
    assert_eq!((iters, status), (1, LmStatus::IterationCap), "one iteration does not converge from a far start");
    let (_, f, _, status) = lm_core(&resid, &b, &far, 100);
    assert_eq!(status, LmStatus::Converged, "loss {f}");
    // The fit of the series with one iteration is printed as not converged and not as a result.
    let r = sabr_fit_series(&s, 1.0, 1).unwrap();
    assert_ne!(r.status, LmStatus::Converged);
}

#[test]
fn sabr_5_fixed_answers_off_the_money() {
    // Independent implementation (Obloj I0 with the time term), Quality and Risk, 2026-11-21. Not a published value.
    // (alpha, beta, rho, nu, F, K, T, vol)
    let cases = [
        (0.2, 1.0, -0.5, 0.6, 100.0, 90.0, 1.0, 0.218281601470249),
        (0.2, 1.0, -0.5, 0.6, 100.0, 100.0, 1.0, 0.200750000000000),
        (2.0, 0.5, -0.3, 0.4, 100.0, 80.0, 0.5, 0.230258294758247),
        (2.0, 0.5, -0.3, 0.4, 100.0, 130.0, 2.0, 0.184383689251854),
        (20.0, 0.0, 0.2, 0.5, 100.0, 95.0, 0.25, 0.204208331704431),
        (0.2, 1.0, 0.3, 0.6, 4500.0, 5200.0, 3.0, 0.240274639653655),
    ];
    for (alpha, beta, rho, nu, f, k, t, want) in cases {
        let got = Sabr { alpha, beta, rho, nu }.implied_vol(f, k, t);
        println!("beta {beta} K {k}: {got:.15} against {want:.15}");
        assert!((got - want).abs() < 1e-12, "beta {beta} K {k}: {got} against {want}");
    }
}

#[test]
fn sabr_6_alpha_bounds_scale_with_the_forward() {
    // beta 0.5, F 7000: alpha = 0.2 * 7000^0.5 = 16.7, above an unscaled upper bound of 10.
    let f: f64 = 7000.0;
    let alpha = 0.2 * f.sqrt();
    assert!(alpha > 10.0);
    let b = sabr_bounds(f, 0.5);
    assert!((b[0].1 - 10.0 * f.sqrt()).abs() < 1e-9 && (b[0].0 - 1e-4 * f.sqrt()).abs() < 1e-12);
    let truth = Sabr { alpha, beta: 0.5, rho: -0.8, nu: 1.0 };
    let r = sabr_fit_series(&sabr_series(&truth, f, 60, 25), 0.5, 100).unwrap();
    assert_eq!((r.status, r.at_bound), (LmStatus::Converged, [false, false, false]), "{:?}", r.model);
    assert!((r.model.alpha - alpha).abs() / alpha < 1e-5, "{:?}", r.model);
}

#[test]
fn sabr_7_beta_zero_nu_zero_is_bachelier() {
    // beta = 0, nu = 0: alpha is a normal (Bachelier) volatility. Bachelier price, inverted with the Black inversion, against the SABR vol.
    // At T = 1e-4 the two agree to the order T^2 (the time term of the formula is the O(T) conversion).
    // This test cannot see the O(T) time term (tolerance 1e-7 at T = 1e-4): the time term is tested by sabr_2 and sabr_5.
    let (f, t, sn): (f64, f64, f64) = (100.0, 1e-4, 20.0);
    for &k in &[99.6, 99.8, 100.0, 100.3] {
        let sd: f64 = sn * t.sqrt();
        let d = (f - k) / sd;
        let ncdf = |x: f64| crate::trading::pricing::norm_cdf(x);
        let price = (f - k) * ncdf(d) + sd * (-0.5 * d * d).exp() / (2.0 * PI).sqrt();
        let black = implied_vol(true, price, f, k, t, 1.0).unwrap();
        let sabr = Sabr { alpha: sn, beta: 0.0, rho: 0.3, nu: 0.0 }.implied_vol(f, k, t);
        println!("K {k}: Bachelier->Black {black:.12}, SABR {sabr:.12}");
        assert!((black - sabr).abs() < 1e-7, "K {k}: {black} against {sabr}");
    }
}

#[test]
fn sabr_8_poor_fit_mark_and_empty_series() {
    // Quotes with a 10 vol point bump on one strike and a 0.2 point spread: the largest error is above 5 spreads.
    let truth = Sabr { alpha: 0.18, beta: 1.0, rho: -0.65, nu: 0.8 };
    let mut s = sabr_series(&truth, 100.0, 90, 25);
    s.quotes[12].iv_mid += 0.10;
    let r = sabr_fit_series(&s, 1.0, 100).unwrap();
    assert!(r.poor, "max {} points", r.max_points);
    assert!(!sabr_fit_series(&sabr_series(&truth, 100.0, 90, 25), 1.0, 100).unwrap().poor);
    s.quotes.clear();
    assert!(sabr_fit_series(&s, 1.0, 100).unwrap_err().contains("no quotes"));
}
