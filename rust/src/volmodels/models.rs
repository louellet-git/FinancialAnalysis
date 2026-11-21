//! Heston (1993) and Bates (1996): the characteristic function in the stable form of
//! Albrecher, Mayer, Schoutens and Tistaert (2007) ("the little Heston trap").
//!
//! `dF / F = sqrt(v) dW1 (+ jumps)`, `dv = kappa (theta - v) dt + xi sqrt(v) dW2`, `corr = rho`.
//! The form `D = -(q / (b + d)) (1 - e^{-dt}) / (1 - c e^{-dt})` and the log term in `ln1p` are
//! algebraic rewrites of the published form with no difference of near-equal numbers. So `xi = 0` works.

use super::base::CharModel;
use super::cx::Cx;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Heston {
    pub v0: f64,
    pub theta: f64,
    pub kappa: f64,
    pub xi: f64,
    pub rho: f64,
}

impl Heston {
    /// 2 kappa theta / xi^2. A value under 1 means that v can reach 0.
    pub fn feller(&self) -> f64 {
        2.0 * self.kappa * self.theta / (self.xi * self.xi)
    }
}

impl CharModel for Heston {
    fn log_cf(&self, z: Cx, t: f64) -> Cx {
        let iz = Cx::I * z;
        let q = z * z + iz; // u^2 + i u
        let xi2 = self.xi * self.xi;
        let b = Cx::real(self.kappa) - iz * (self.rho * self.xi);
        let d = (b * b + q * xi2).sqrt();
        let bd = b + d;
        let e = (-(d * t)).exp();
        let c_over_xi2 = -q / (bd * bd);
        let c = c_over_xi2 * xi2;
        let one_minus_ce = Cx::ONE - c * e;
        let big_d = -(q / bd) * (Cx::ONE - e) / one_minus_ce;
        // (1 - c e) / (1 - c) = 1 + w, with w = xi^2 w_t.
        let w_t = c_over_xi2 * (Cx::ONE - e) / (Cx::ONE - c);
        let w = w_t * xi2;
        let big_c = (-(q * t) / bd - w_t * w.ln1p_over() * 2.0) * (self.kappa * self.theta);
        (big_c + big_d * self.v0).exp()
    }
    fn names() -> &'static [&'static str] {
        &["v0", "theta", "kappa", "xi", "rho"]
    }
    fn bounds() -> Vec<(f64, f64)> {
        vec![(1e-4, 4.0), (1e-4, 4.0), (0.05, 20.0), (0.05, 3.0), (-0.99, 0.0)]
    }
    fn from_vec(p: &[f64]) -> Self {
        Heston { v0: p[0], theta: p[1], kappa: p[2], xi: p[3], rho: p[4] }
    }
    fn to_vec(&self) -> Vec<f64> {
        vec![self.v0, self.theta, self.kappa, self.xi, self.rho]
    }
    fn start(atm_vol: f64, skew: f64) -> Vec<f64> {
        let (v, xi) = (atm_vol * atm_vol, 0.6);
        vec![v, v, 2.0, xi, (4.0 * atm_vol * skew / xi).clamp(-0.95, -0.05)]
    }
}

/// Heston plus jumps of the log price: rate `lambda`, `ln(1 + J)` normal with mean `mu_j` and standard deviation `delta_j`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bates {
    pub heston: Heston,
    pub lambda: f64,
    pub mu_j: f64,
    pub delta_j: f64,
}

impl Bates {
    /// The mean jump `exp(mu_j + delta_j^2 / 2) - 1`. The drift has the compensator `- lambda * mean_jump`.
    pub fn mean_jump(&self) -> f64 {
        (self.mu_j + 0.5 * self.delta_j * self.delta_j).exp() - 1.0
    }
}

impl CharModel for Bates {
    fn log_cf(&self, z: Cx, t: f64) -> Cx {
        let iz = Cx::I * z;
        let jump = (iz * self.mu_j - (z * z) * (0.5 * self.delta_j * self.delta_j)).exp() - 1.0 - iz * self.mean_jump();
        self.heston.log_cf(z, t) * (jump * (self.lambda * t)).exp()
    }
    fn names() -> &'static [&'static str] {
        &["v0", "theta", "kappa", "xi", "rho", "lambda", "mu_j", "delta_j"]
    }
    fn bounds() -> Vec<(f64, f64)> {
        let mut b = Heston::bounds();
        b.extend([(0.0, 10.0), (-1.0, 0.5), (0.01, 1.0)]);
        b
    }
    fn from_vec(p: &[f64]) -> Self {
        Bates { heston: Heston::from_vec(&p[..5]), lambda: p[5], mu_j: p[6], delta_j: p[7] }
    }
    fn to_vec(&self) -> Vec<f64> {
        let mut v = self.heston.to_vec();
        v.extend([self.lambda, self.mu_j, self.delta_j]);
        v
    }
    fn start(atm_vol: f64, skew: f64) -> Vec<f64> {
        let mut v = Heston::start(atm_vol, skew);
        v.extend([0.2, -0.1, 0.1]);
        v
    }
}
