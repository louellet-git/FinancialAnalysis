//! Tests with fixed answers (ticket T-20261120-vol-1-heston-bates).

use super::base::*;
use super::calib::{at_bound, fit, levenberg_marquardt, FitOptions, LmStatus};
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
