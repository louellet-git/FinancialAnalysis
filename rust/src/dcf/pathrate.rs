//! The short-rate factor of the path engine.
//!
//! The factor is a one-factor Gaussian model (Hull-White with the drift from a curve, or
//! Vasicek) or CIR, stepped with a standard normal shock that the caller gives. The caller
//! gives the shock so that the rate shock can have a correlation with the shocks of the
//! growth and the margin. The models are the models of `crate::ratemodels` (same equations,
//! same exact steps, same QE scheme for CIR); only the source of the normal number is different.
//!
//! The risk-free rate of the CAPM is an affine function of the state: `rf(i, state) = d0[i] +
//! d1[i] * state`. With `mode = long` it is the zero-coupon yield of maturity `tenor` that the
//! model gives at time `i * dt` (closed form of the model); with `mode = short` it is the short
//! rate plus a spread.
//!
//! Hull-White: `r = alpha(t) + x`, `alpha(t) = f(0,t) + s^2 B(t)^2 / 2`, `B(t) = (1 - e^(-a t))/a`,
//! x is an Ornstein-Uhlenbeck process with mean 0 (exact step). The price of a zero-coupon bond
//! at time `t` for the maturity `t + tau` is
//! `P(t,t+tau) = P(0,t+tau)/P(0,t) * exp(B(tau) f(0,t) - s^2/(4a) (1 - e^(-2at)) B(tau)^2 - B(tau) r_t)`.

use crate::ratemodels::curve::ZeroCurve;
use crate::ratemodels::models::{Cir, RateModel, Vasicek};

const QE_PSI_C: f64 = 1.5;
/// The GSW curve is fitted up to 30 years.
pub const CURVE_MAX_YEARS: f64 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateKind {
    HullWhite { a: f64, sigma: f64, curve: ZeroCurve },
    Vasicek { kappa: f64, theta: f64, sigma: f64, r0: f64 },
    Cir { kappa: f64, theta: f64, sigma: f64, r0: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RfMode {
    /// Zero-coupon yield of the maturity `tenor`; its expected value follows the forward curve.
    Long,
    /// The same yield with the expected value fixed at the level of time 0 (only the deviations of the model).
    LongFlat,
    /// Short rate plus a spread.
    Short,
}

#[derive(Debug, Clone)]
pub struct RateFactor {
    pub kind: RateKind,
    pub dt: f64,
    pub steps: usize,
    alpha: Vec<f64>,
    e: f64,
    sd: f64,
    qe_c1: f64,
    qe_c2: f64,
    d0: Vec<f64>,
    d1: Vec<f64>,
    pub state0: f64,
}

fn b_fn(a: f64, tau: f64) -> f64 {
    -(-a * tau).exp_m1() / a
}

/// Standard normal distribution function.
fn norm_cdf(z: f64) -> f64 {
    0.5 * libm::erfc(-z / std::f64::consts::SQRT_2)
}

impl RateFactor {
    /// A factor with `steps` steps of `dt` years; `rf` is defined at the times `0, dt, .., steps*dt`.
    pub fn new(kind: RateKind, dt: f64, steps: usize, mode: RfMode, tenor: f64, spread: f64) -> Result<RateFactor, String> {
        if !(dt.is_finite() && dt > 0.0) {
            return Err(format!("rate factor: dt must be positive, got {dt}"));
        }
        if mode != RfMode::Short && !(tenor.is_finite() && tenor > 0.0) {
            return Err(format!("rate factor: tenor must be positive, got {tenor}"));
        }
        let n = steps + 1;
        let (mut alpha, mut d0, mut d1) = (Vec::new(), vec![0.0; n], vec![0.0; n]);
        let (e, sd, qe_c1, qe_c2, state0);
        match kind {
            RateKind::HullWhite { a, sigma, curve } => {
                check_pos("hullwhite a", a)?;
                check_pos("hullwhite sigma", sigma)?;
                let horizon = steps as f64 * dt + if mode != RfMode::Short { tenor } else { 0.0 };
                if horizon > CURVE_MAX_YEARS + 1e-9 {
                    return Err(format!(
                        "rate factor: the horizon plus the tenor is {horizon:.1} years; the curve ends at {CURVE_MAX_YEARS} years. Use a shorter tenor or horizon."
                    ));
                }
                e = (-a * dt).exp();
                sd = sigma * ((1.0 - (-2.0 * a * dt).exp()) / (2.0 * a)).sqrt();
                alpha = (0..n)
                    .map(|i| {
                        let t = i as f64 * dt;
                        let b = b_fn(a, t);
                        curve.forward(t) + 0.5 * sigma * sigma * b * b
                    })
                    .collect();
                for i in 0..n {
                    let t = i as f64 * dt;
                    match mode {
                        RfMode::Long | RfMode::LongFlat => {
                            let b = b_fn(a, tenor);
                            let ln_p = |x: f64| -x * curve.zero_yield(x);
                            let num = ln_p(t) - ln_p(t + tenor) - b * curve.forward(t)
                                + sigma * sigma / (4.0 * a) * (1.0 - (-2.0 * a * t).exp()) * b * b
                                + b * alpha[i];
                            d0[i] = num / tenor;
                            d1[i] = b / tenor;
                        }
                        RfMode::Short => {
                            d0[i] = alpha[i] + spread;
                            d1[i] = 1.0;
                        }
                    }
                }
                state0 = 0.0;
                qe_c1 = 0.0;
                qe_c2 = 0.0;
            }
            RateKind::Vasicek { kappa, theta, sigma, r0 } => {
                let m = Vasicek { kappa, theta, sigma };
                m.validate()?;
                e = (-kappa * dt).exp();
                sd = sigma * ((1.0 - (-2.0 * kappa * dt).exp()) / (2.0 * kappa)).sqrt();
                qe_c1 = 0.0;
                qe_c2 = 0.0;
                let (a0, a1) = affine(mode, tenor, spread, |r| m.bond_price(r, tenor).expect("closed form"));
                d0.iter_mut().for_each(|v| *v = a0);
                d1.iter_mut().for_each(|v| *v = a1);
                state0 = r0;
            }
            RateKind::Cir { kappa, theta, sigma, r0 } => {
                let m = Cir { kappa, theta, sigma };
                m.validate()?;
                e = (-kappa * dt).exp();
                sd = 0.0;
                let one_minus = -(-kappa * dt).exp_m1();
                qe_c1 = sigma * sigma * e * one_minus / kappa;
                qe_c2 = theta * sigma * sigma * one_minus * one_minus / (2.0 * kappa);
                let (a0, a1) = affine(mode, tenor, spread, |r| m.bond_price(r, tenor).expect("closed form"));
                d0.iter_mut().for_each(|v| *v = a0);
                d1.iter_mut().for_each(|v| *v = a1);
                state0 = r0;
            }
        }
        if mode == RfMode::LongFlat {
            let target = d0[0] + d1[0] * state0;
            for i in 0..n {
                let t = i as f64 * dt;
                let mean_state = match kind {
                    RateKind::HullWhite { .. } => 0.0,
                    RateKind::Vasicek { kappa, theta, r0, .. } | RateKind::Cir { kappa, theta, r0, .. } => theta + (r0 - theta) * (-kappa * t).exp(),
                };
                d0[i] = target - d1[i] * mean_state;
            }
        }
        Ok(RateFactor { kind, dt, steps, alpha, e, sd, qe_c1, qe_c2, d0, d1, state0 })
    }

    /// The state at step `i + 1` from the state at step `i`, for the standard normal shock `z`.
    #[inline]
    pub fn step(&self, i: usize, s: f64, z: f64) -> f64 {
        match self.kind {
            RateKind::HullWhite { .. } => self.e * s + self.sd * z,
            RateKind::Vasicek { theta, .. } => theta + (s - theta) * self.e + self.sd * z,
            RateKind::Cir { theta, .. } => {
                let _ = i;
                let r = s.max(0.0);
                let m = theta + (r - theta) * self.e;
                let s2 = r * self.qe_c1 + self.qe_c2;
                let psi = s2 / (m * m);
                if psi <= QE_PSI_C {
                    let inv = 2.0 / psi;
                    let b2 = inv - 1.0 + (inv * (inv - 1.0)).sqrt();
                    let a = m / (1.0 + b2);
                    let x = b2.sqrt() + z;
                    a * x * x
                } else {
                    let p0 = (psi - 1.0) / (psi + 1.0);
                    let u = norm_cdf(z).clamp(1e-16, 1.0 - 1e-16);
                    if u <= p0 {
                        0.0
                    } else {
                        let beta = (1.0 - p0) / m;
                        ((1.0 - p0) / (1.0 - u)).ln() / beta
                    }
                }
            }
        }
    }

    /// The short rate at step `i` for the state `s`.
    #[inline]
    pub fn short_rate(&self, i: usize, s: f64) -> f64 {
        match self.kind {
            RateKind::HullWhite { .. } => self.alpha[i] + s,
            _ => s,
        }
    }

    /// The risk-free rate of the CAPM at step `i` for the state `s`.
    #[inline]
    pub fn rf(&self, i: usize, s: f64) -> f64 {
        self.d0[i] + self.d1[i] * s
    }
}

fn check_pos(name: &str, v: f64) -> Result<(), String> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(format!("rate factor: {name} must be positive and finite, got {v}"))
    }
}

/// `y(r) = d0 + d1 r` for the models with an affine bond price.
fn affine(mode: RfMode, tenor: f64, spread: f64, price: impl Fn(f64) -> f64) -> (f64, f64) {
    match mode {
        RfMode::Long | RfMode::LongFlat => {
            let y = |r: f64| -price(r).ln() / tenor;
            let (y0, y1) = (y(0.0), y(1.0));
            (y0, y1 - y0)
        }
        RfMode::Short => (spread, 1.0),
    }
}
