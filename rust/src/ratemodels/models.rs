//! Short-rate models. All rates are decimals per year. Time is in years.
//!
//! Each model has a parameter struct, a one-step transition (`step`), and the closed forms
//! that exist: mean and variance of r(t), stationary mean and variance, and the zero-coupon
//! bond price P(0,t) = E[exp(-integral of r)] under the model (the model is its own
//! pricing measure; there is no market price of risk).

use super::curve::ZeroCurve;
use super::rng::Rng;

pub type Result<T> = std::result::Result<T, String>;

pub trait RateModel: Sync {
    /// Constants for one step size. The caller makes them once for each simulation, not for each step.
    type Prep: Sync;
    fn name(&self) -> &'static str;
    fn parameters(&self) -> Vec<(&'static str, f64)>;
    /// An error if the parameters are outside the domain of the model.
    fn validate(&self) -> Result<()>;
    fn prepare(&self, dt: f64, steps: usize) -> Self::Prep;
    /// The rate at step i+1 from the rate `r` at step i (time i*dt). No allocation.
    fn step(&self, prep: &Self::Prep, i: usize, r: f64, rng: &mut Rng) -> f64;
    /// The rate at time 0 that the model uses when the caller gives none.
    fn default_r0(&self) -> f64;
    fn mean(&self, _r0: f64, _t: f64) -> Option<f64> {
        None
    }
    fn variance(&self, _r0: f64, _t: f64) -> Option<f64> {
        None
    }
    fn stationary_mean(&self) -> Option<f64> {
        None
    }
    fn stationary_variance(&self) -> Option<f64> {
        None
    }
    /// P(0, t) given r(0) = r0, where a closed form exists.
    fn bond_price(&self, _r0: f64, _t: f64) -> Option<f64> {
        None
    }
}

fn check_positive(model: &str, name: &str, v: f64) -> Result<()> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(format!("{model}: {name} must be positive and finite; got {v}"))
    }
}

/// B(a, tau) = (1 - exp(-a tau)) / a. The limit for a -> 0 is tau.
fn b_fn(a: f64, tau: f64) -> f64 {
    if a.abs() < 1e-9 {
        tau
    } else {
        -(-a * tau).exp_m1() / a
    }
}

/// Variance of the OU process with mean reversion `a` and volatility `sigma` after time `t`.
fn ou_variance(a: f64, sigma: f64, t: f64) -> f64 {
    if a.abs() < 1e-9 {
        sigma * sigma * t
    } else {
        sigma * sigma * (-(-2.0 * a * t).exp_m1()) / (2.0 * a)
    }
}

// ---------------------------------------------------------------------------
// Vasicek
// ---------------------------------------------------------------------------

/// dr = kappa (theta - r) dt + sigma dW. The transition is exactly Gaussian, so the step is exact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vasicek {
    pub kappa: f64,
    pub theta: f64,
    pub sigma: f64,
}

pub struct OuPrep {
    e: f64,
    sd: f64,
}

impl RateModel for Vasicek {
    type Prep = OuPrep;
    fn name(&self) -> &'static str {
        "vasicek"
    }
    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![("kappa", self.kappa), ("theta", self.theta), ("sigma", self.sigma)]
    }
    fn validate(&self) -> Result<()> {
        check_positive("vasicek", "kappa", self.kappa)?;
        check_positive("vasicek", "sigma", self.sigma)?;
        if !self.theta.is_finite() {
            return Err("vasicek: theta must be finite".into());
        }
        Ok(())
    }
    fn prepare(&self, dt: f64, _steps: usize) -> OuPrep {
        OuPrep { e: (-self.kappa * dt).exp(), sd: ou_variance(self.kappa, self.sigma, dt).sqrt() }
    }
    #[inline]
    fn step(&self, p: &OuPrep, _i: usize, r: f64, rng: &mut Rng) -> f64 {
        self.theta + (r - self.theta) * p.e + p.sd * rng.normal()
    }
    fn default_r0(&self) -> f64 {
        self.theta
    }
    fn mean(&self, r0: f64, t: f64) -> Option<f64> {
        Some(self.theta + (r0 - self.theta) * (-self.kappa * t).exp())
    }
    fn variance(&self, _r0: f64, t: f64) -> Option<f64> {
        Some(ou_variance(self.kappa, self.sigma, t))
    }
    fn stationary_mean(&self) -> Option<f64> {
        Some(self.theta)
    }
    fn stationary_variance(&self) -> Option<f64> {
        Some(self.sigma * self.sigma / (2.0 * self.kappa))
    }
    fn bond_price(&self, r0: f64, t: f64) -> Option<f64> {
        let (k, th, s) = (self.kappa, self.theta, self.sigma);
        let b = b_fn(k, t);
        let ln_a = (th - s * s / (2.0 * k * k)) * (b - t) - s * s * b * b / (4.0 * k);
        Some((ln_a - b * r0).exp())
    }
}

// ---------------------------------------------------------------------------
// Cox-Ingersoll-Ross
// ---------------------------------------------------------------------------

/// dr = kappa (theta - r) dt + sigma sqrt(r) dW.
///
/// Scheme: the quadratic-exponential (QE) scheme of Andersen (2008), switch psi_c = 1.5.
/// Each step matches the exact conditional mean and variance, and r stays at 0 or above,
/// also when the Feller condition 2 kappa theta >= sigma^2 does not hold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cir {
    pub kappa: f64,
    pub theta: f64,
    pub sigma: f64,
}

pub struct CirPrep {
    e: f64,
    c1: f64,
    c2: f64,
}

const QE_PSI_C: f64 = 1.5;

impl Cir {
    /// 2 kappa theta / sigma^2. The value 1 or more means that r cannot reach 0.
    pub fn feller_ratio(&self) -> f64 {
        2.0 * self.kappa * self.theta / (self.sigma * self.sigma)
    }
    pub fn feller_satisfied(&self) -> bool {
        self.feller_ratio() >= 1.0
    }
}

pub(crate) fn cir_prep(kappa: f64, theta: f64, sigma: f64, dt: f64) -> CirPrep {
    let e = (-kappa * dt).exp();
    let one_minus = -(-kappa * dt).exp_m1();
    CirPrep { e, c1: sigma * sigma * e * one_minus / kappa, c2: theta * sigma * sigma * one_minus * one_minus / (2.0 * kappa) }
}

#[inline]
pub(crate) fn cir_step(p: &CirPrep, theta: f64, r: f64, rng: &mut Rng) -> f64 {
    let r = r.max(0.0);
    let m = theta + (r - theta) * p.e;
    let s2 = r * p.c1 + p.c2;
    let psi = s2 / (m * m);
    if psi <= QE_PSI_C {
        let inv = 2.0 / psi;
        let b2 = inv - 1.0 + (inv * (inv - 1.0)).sqrt();
        let a = m / (1.0 + b2);
        let x = b2.sqrt() + rng.normal();
        a * x * x
    } else {
        let p0 = (psi - 1.0) / (psi + 1.0);
        let u = rng.uniform();
        if u <= p0 {
            0.0
        } else {
            let beta = (1.0 - p0) / m;
            ((1.0 - p0) / (1.0 - u)).ln() / beta
        }
    }
}

fn cir_variance(k: f64, th: f64, s: f64, r0: f64, t: f64) -> f64 {
    let (e1, e2) = ((-k * t).exp(), (-2.0 * k * t).exp());
    r0 * s * s / k * (e1 - e2) + th * s * s / (2.0 * k) * (1.0 - e1) * (1.0 - e1)
}

fn cir_bond(k: f64, th: f64, s: f64, r0: f64, t: f64) -> f64 {
    let h = (k * k + 2.0 * s * s).sqrt();
    let eh = (h * t).exp_m1();
    let den = 2.0 * h + (k + h) * eh;
    let ln_a = 2.0 * k * th / (s * s) * ((2.0 * h).ln() + (k + h) * t / 2.0 - den.ln());
    let b = 2.0 * eh / den;
    (ln_a - b * r0).exp()
}

impl RateModel for Cir {
    type Prep = CirPrep;
    fn name(&self) -> &'static str {
        "cir"
    }
    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![("kappa", self.kappa), ("theta", self.theta), ("sigma", self.sigma)]
    }
    fn validate(&self) -> Result<()> {
        check_positive("cir", "kappa", self.kappa)?;
        check_positive("cir", "theta", self.theta)?;
        check_positive("cir", "sigma", self.sigma)
    }
    fn prepare(&self, dt: f64, _steps: usize) -> CirPrep {
        cir_prep(self.kappa, self.theta, self.sigma, dt)
    }
    #[inline]
    fn step(&self, p: &CirPrep, _i: usize, r: f64, rng: &mut Rng) -> f64 {
        cir_step(p, self.theta, r, rng)
    }
    fn default_r0(&self) -> f64 {
        self.theta
    }
    fn mean(&self, r0: f64, t: f64) -> Option<f64> {
        Some(self.theta + (r0 - self.theta) * (-self.kappa * t).exp())
    }
    fn variance(&self, r0: f64, t: f64) -> Option<f64> {
        Some(cir_variance(self.kappa, self.theta, self.sigma, r0, t))
    }
    fn stationary_mean(&self) -> Option<f64> {
        Some(self.theta)
    }
    fn stationary_variance(&self) -> Option<f64> {
        Some(self.theta * self.sigma * self.sigma / (2.0 * self.kappa))
    }
    fn bond_price(&self, r0: f64, t: f64) -> Option<f64> {
        Some(cir_bond(self.kappa, self.theta, self.sigma, r0, t))
    }
}

// ---------------------------------------------------------------------------
// CKLS
// ---------------------------------------------------------------------------

/// Chan-Karolyi-Longstaff-Sanders: dr = kappa (theta - r) dt + sigma r^gamma dW.
/// This is dr = (a + b r) dt + sigma r^gamma dW with a = kappa theta and b = -kappa.
///
/// gamma = 0 is Vasicek. gamma = 1/2 is CIR. gamma = 1 is the Brennan-Schwartz-type model.
///
/// Scheme: for gamma = 0 the exact Gaussian step. For gamma > 0 the Milstein scheme with
/// full truncation (the coefficients use max(r, 0)); weak order 1. The scheme is biased
/// near 0 when the Feller-type condition fails (for gamma = 1/2 use `Cir`, which has the QE scheme).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ckls {
    pub kappa: f64,
    pub theta: f64,
    pub sigma: f64,
    pub gamma: f64,
}

pub struct CklsPrep {
    e: f64,
    sd0: f64,
    dt: f64,
    sqrt_dt: f64,
}

impl Ckls {
    /// From the form dr = (a + b r) dt + sigma r^gamma dW. Needs b < 0.
    pub fn from_ab(a: f64, b: f64, sigma: f64, gamma: f64) -> Result<Ckls> {
        if b >= 0.0 {
            return Err(format!("ckls: b = {b} is not negative, so there is no mean reversion"));
        }
        Ok(Ckls { kappa: -b, theta: a / -b, sigma, gamma })
    }
    fn is_vasicek(&self) -> bool {
        self.gamma.abs() < 1e-12
    }
    fn is_cir(&self) -> bool {
        (self.gamma - 0.5).abs() < 1e-12
    }
}

impl RateModel for Ckls {
    type Prep = CklsPrep;
    fn name(&self) -> &'static str {
        "ckls"
    }
    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![("kappa", self.kappa), ("theta", self.theta), ("sigma", self.sigma), ("gamma", self.gamma)]
    }
    fn validate(&self) -> Result<()> {
        check_positive("ckls", "kappa", self.kappa)?;
        check_positive("ckls", "sigma", self.sigma)?;
        if !self.theta.is_finite() || (self.gamma != 0.0 && self.theta <= 0.0) {
            return Err(format!("ckls: theta must be positive when gamma is not 0; got {}", self.theta));
        }
        if !self.gamma.is_finite() || !(0.0..=3.0).contains(&self.gamma) {
            return Err(format!("ckls: gamma must be in [0, 3]; got {}", self.gamma));
        }
        Ok(())
    }
    fn prepare(&self, dt: f64, _steps: usize) -> CklsPrep {
        CklsPrep { e: (-self.kappa * dt).exp(), sd0: ou_variance(self.kappa, self.sigma, dt).sqrt(), dt, sqrt_dt: dt.sqrt() }
    }
    #[inline]
    fn step(&self, p: &CklsPrep, _i: usize, r: f64, rng: &mut Rng) -> f64 {
        let z = rng.normal();
        if self.is_vasicek() {
            return self.theta + (r - self.theta) * p.e + p.sd0 * z;
        }
        let rp = r.max(0.0);
        let (vol, milstein) = if self.is_cir() {
            let s = rp.sqrt();
            (self.sigma * s, 0.25 * self.sigma * self.sigma * p.dt * (z * z - 1.0))
        } else {
            let v = self.sigma * rp.powf(self.gamma);
            let m = if rp > 0.0 { 0.5 * self.gamma * self.sigma * self.sigma * rp.powf(2.0 * self.gamma - 1.0) * p.dt * (z * z - 1.0) } else { 0.0 };
            (v, m)
        };
        r + self.kappa * (self.theta - rp) * p.dt + vol * p.sqrt_dt * z + milstein
    }
    fn default_r0(&self) -> f64 {
        self.theta
    }
    fn mean(&self, r0: f64, t: f64) -> Option<f64> {
        // The drift is linear, so the mean is the same for each gamma (before truncation).
        Some(self.theta + (r0 - self.theta) * (-self.kappa * t).exp())
    }
    fn variance(&self, r0: f64, t: f64) -> Option<f64> {
        if self.is_vasicek() {
            Some(ou_variance(self.kappa, self.sigma, t))
        } else if self.is_cir() {
            Some(cir_variance(self.kappa, self.theta, self.sigma, r0, t))
        } else {
            None
        }
    }
    fn stationary_mean(&self) -> Option<f64> {
        Some(self.theta)
    }
    fn stationary_variance(&self) -> Option<f64> {
        if self.is_vasicek() {
            Some(self.sigma * self.sigma / (2.0 * self.kappa))
        } else if self.is_cir() {
            Some(self.theta * self.sigma * self.sigma / (2.0 * self.kappa))
        } else {
            None
        }
    }
    fn bond_price(&self, r0: f64, t: f64) -> Option<f64> {
        if self.is_vasicek() {
            Vasicek { kappa: self.kappa, theta: self.theta, sigma: self.sigma }.bond_price(r0, t)
        } else if self.is_cir() {
            Some(cir_bond(self.kappa, self.theta, self.sigma, r0, t))
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Ho-Lee and Hull-White (one factor), fitted to an initial zero curve
// ---------------------------------------------------------------------------

/// Hull-White: dr = (theta(t) - a r) dt + sigma dW, with theta(t) set so that the model
/// prices the zero curve at time 0 exactly. With x = r - alpha(t), where
/// `alpha(t) = f(0,t) + sigma^2 B(t)^2 / 2` and B(t) = (1 - exp(-a t))/a, the process x is
/// an OU process with mean 0, and the step of x is exact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HullWhite {
    pub a: f64,
    pub sigma: f64,
    pub curve: ZeroCurve,
}

/// Ho-Lee: dr = theta(t) dt + sigma dW. This is Hull-White with a = 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoLee {
    pub sigma: f64,
    pub curve: ZeroCurve,
}

pub struct CurvePrep {
    alpha: Vec<f64>,
    e: f64,
    sd: f64,
}

fn alpha_at(a: f64, sigma: f64, curve: &ZeroCurve, t: f64) -> f64 {
    let b = b_fn(a, t);
    curve.forward(t) + 0.5 * sigma * sigma * b * b
}

fn gaussian_curve_prepare(a: f64, sigma: f64, curve: &ZeroCurve, dt: f64, steps: usize) -> CurvePrep {
    CurvePrep {
        alpha: (0..=steps).map(|i| alpha_at(a, sigma, curve, i as f64 * dt)).collect(),
        e: (-a * dt).exp(),
        sd: ou_variance(a, sigma, dt).sqrt(),
    }
}

#[inline]
fn gaussian_curve_step(p: &CurvePrep, i: usize, r: f64, rng: &mut Rng) -> f64 {
    p.alpha[i + 1] + (r - p.alpha[i]) * p.e + p.sd * rng.normal()
}

fn gaussian_curve_bond(a: f64, curve: &ZeroCurve, r0: f64, t: f64) -> f64 {
    curve.discount(t) * (-b_fn(a, t) * (r0 - curve.forward(0.0))).exp()
}

impl RateModel for HullWhite {
    type Prep = CurvePrep;
    fn name(&self) -> &'static str {
        "hullwhite"
    }
    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![("a", self.a), ("sigma", self.sigma)]
    }
    fn validate(&self) -> Result<()> {
        check_positive("hullwhite", "a", self.a)?;
        check_positive("hullwhite", "sigma", self.sigma)
    }
    fn prepare(&self, dt: f64, steps: usize) -> CurvePrep {
        gaussian_curve_prepare(self.a, self.sigma, &self.curve, dt, steps)
    }
    #[inline]
    fn step(&self, p: &CurvePrep, i: usize, r: f64, rng: &mut Rng) -> f64 {
        gaussian_curve_step(p, i, r, rng)
    }
    fn default_r0(&self) -> f64 {
        self.curve.forward(0.0)
    }
    fn mean(&self, r0: f64, t: f64) -> Option<f64> {
        Some(alpha_at(self.a, self.sigma, &self.curve, t) + (r0 - alpha_at(self.a, self.sigma, &self.curve, 0.0)) * (-self.a * t).exp())
    }
    fn variance(&self, _r0: f64, t: f64) -> Option<f64> {
        Some(ou_variance(self.a, self.sigma, t))
    }
    fn bond_price(&self, r0: f64, t: f64) -> Option<f64> {
        Some(gaussian_curve_bond(self.a, &self.curve, r0, t))
    }
}

impl RateModel for HoLee {
    type Prep = CurvePrep;
    fn name(&self) -> &'static str {
        "holee"
    }
    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![("sigma", self.sigma)]
    }
    fn validate(&self) -> Result<()> {
        check_positive("holee", "sigma", self.sigma)
    }
    fn prepare(&self, dt: f64, steps: usize) -> CurvePrep {
        gaussian_curve_prepare(0.0, self.sigma, &self.curve, dt, steps)
    }
    #[inline]
    fn step(&self, p: &CurvePrep, i: usize, r: f64, rng: &mut Rng) -> f64 {
        gaussian_curve_step(p, i, r, rng)
    }
    fn default_r0(&self) -> f64 {
        self.curve.forward(0.0)
    }
    fn mean(&self, r0: f64, t: f64) -> Option<f64> {
        Some(alpha_at(0.0, self.sigma, &self.curve, t) + r0 - alpha_at(0.0, self.sigma, &self.curve, 0.0))
    }
    fn variance(&self, _r0: f64, t: f64) -> Option<f64> {
        Some(ou_variance(0.0, self.sigma, t))
    }
    fn bond_price(&self, r0: f64, t: f64) -> Option<f64> {
        Some(gaussian_curve_bond(0.0, &self.curve, r0, t))
    }
}
