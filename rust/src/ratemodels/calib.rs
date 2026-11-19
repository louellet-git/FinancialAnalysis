//! Calibration of a rate model to a sampled rate history.
//!
//! One function, `calibrate(kind, method, sample, gamma)`, for each model and method.
//! All parameters are annualized and in continuous time: the mapping from the regression
//! coefficients uses the exact AR(1) solution for the sampling step `dt`, not the first-order
//! Euler approximation. With `beta = exp(-kappa dt)` and `alpha = theta (1 - beta)`:
//! `kappa = -ln(beta)/dt`, `theta = alpha/(1 - beta)`.
//!
//! What "realized volatility" means here. The change of the rate over one sampling step is
//! `d_i = r(i+1) - r(i)`. The realized volatility is
//! `sqrt( sum w_i (d_i - dbar)^2 / sum w_i / dt )`: the standard deviation of the changes in
//! rate units per square root of a year (a decimal: 0.0100 = 100 basis points per sqrt(year)).
//! The weights `w_i` come from the estimator: all changes (close-to-close), the last N changes
//! (rolling), or `lambda^(age)` (exponentially weighted; the newest change has weight 1).
//! To set the model volatility, the code does not use `d_i` but the residual `e_i` of the
//! fitted conditional mean (`e_i = r(i+1) - E[r(i+1) | r(i)]`). It divides `e_i^2` by the
//! conditional variance of the model with `sigma = 1`: for Vasicek `(1 - beta^2)/(2 kappa)`,
//! for CIR `r_i e (1-e)/kappa + theta (1-e)^2/(2 kappa)` (e = beta), for CKLS
//! `r_i^(2 gamma) (1 - beta^2)/(2 kappa)`. Then `sigma^2 = sum w_i e_i^2 / v_i / sum w_i`.
//! So for Gaussian models sigma is in rate units per sqrt(year), and for CIR and CKLS the
//! volatility of the rate is `sigma r^gamma`: the realized volatility at the level `r`.

use super::data::Sample;
use super::math::{chunked_sum, covariance_from_loglik, delta_se, invert, lgamma, nelder_mead, ols, Ols};
use mapcheck::dates;
use serde::Serialize;

pub type Result<T> = std::result::Result<T, String>;

/// The DF 5% critical value (constant, no trend), as t-statistic: 2.86. The normal value
/// 1.96 is too small near a unit root, so the code uses this one for the test of mean reversion.
pub const MR_T_THRESHOLD: f64 = 2.86;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    Vasicek,
    Cir,
    Ckls,
    HoLee,
    HullWhite,
}

impl ModelKind {
    pub fn parse(text: &str) -> Result<ModelKind> {
        match text {
            "vasicek" => Ok(ModelKind::Vasicek),
            "cir" => Ok(ModelKind::Cir),
            "ckls" => Ok(ModelKind::Ckls),
            "holee" | "ho-lee" => Ok(ModelKind::HoLee),
            "hullwhite" | "hull-white" | "hw" => Ok(ModelKind::HullWhite),
            other => Err(format!("unknown model '{other}': use vasicek, cir, ckls, holee or hullwhite")),
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            ModelKind::Vasicek => "vasicek",
            ModelKind::Cir => "cir",
            ModelKind::Ckls => "ckls",
            ModelKind::HoLee => "holee",
            ModelKind::HullWhite => "hullwhite",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolEstimator {
    CloseToClose,
    /// The last N changes.
    Rolling(usize),
    /// Weight lambda^age, 0 < lambda < 1.
    Ewma(f64),
}

impl VolEstimator {
    /// "close", "rolling:N" (N changes), "ewma:LAMBDA".
    pub fn parse(text: &str) -> Result<VolEstimator> {
        let bad = || format!("bad volatility estimator '{text}': use close, rolling:N or ewma:LAMBDA");
        match text.split_once(':') {
            None if text == "close" => Ok(VolEstimator::CloseToClose),
            Some(("rolling", n)) => n.parse().map(VolEstimator::Rolling).map_err(|_| bad()),
            Some(("ewma", l)) => {
                let l: f64 = l.parse().map_err(|_| bad())?;
                if l > 0.0 && l < 1.0 {
                    Ok(VolEstimator::Ewma(l))
                } else {
                    Err(format!("ewma lambda must be between 0 and 1; got {l}"))
                }
            }
            _ => Err(bad()),
        }
    }
    pub fn label(&self) -> String {
        match self {
            VolEstimator::CloseToClose => "close".into(),
            VolEstimator::Rolling(n) => format!("rolling:{n}"),
            VolEstimator::Ewma(l) => format!("ewma:{l}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    /// Maximum likelihood: exact density for Vasicek and CIR; quasi-likelihood for CKLS.
    Ml,
    /// Least squares on the AR(1) form with the exact mapping.
    Ols,
    /// CKLS (1992) moment conditions.
    Gmm,
    /// Drift from least squares, volatility from the realized-volatility estimator.
    RealVol(VolEstimator),
}

impl Method {
    pub fn parse(text: &str, vol: &str) -> Result<Method> {
        match text {
            "ml" | "mle" => Ok(Method::Ml),
            "ols" | "ls" => Ok(Method::Ols),
            "gmm" => Ok(Method::Gmm),
            "realvol" => Ok(Method::RealVol(VolEstimator::parse(vol)?)),
            other => Err(format!("unknown method '{other}': use ml, ols, gmm or realvol")),
        }
    }
    pub fn label(&self) -> String {
        match self {
            Method::Ml => "ml".into(),
            Method::Ols => "ols".into(),
            Method::Gmm => "gmm".into(),
            Method::RealVol(v) => format!("realvol({})", v.label()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    pub value: f64,
    /// Standard error. None if it cannot be computed (a flat likelihood or a boundary).
    pub se: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Fit {
    pub model: String,
    pub method: String,
    pub series: String,
    pub freq: String,
    pub start: String,
    pub end: String,
    /// The number of transitions (observations minus one).
    pub n_transitions: usize,
    /// The sampling step in years.
    pub dt_years: f64,
    /// Annualized, continuous-time parameters.
    pub params: Vec<Param>,
    /// Close-to-close realized volatility of the rate changes: rate units per sqrt(year).
    pub realized_vol: f64,
    /// For the method `realvol`: the value of the chosen estimator, in the same unit.
    pub realized_vol_estimator: Option<f64>,
    pub loglik: Option<f64>,
    pub loglik_kind: String,
    pub n_params_aic: usize,
    pub aic: Option<f64>,
    /// The t-statistic of mean reversion: (1 - beta)/se(beta) for least squares, kappa/se(kappa) for ML.
    pub mean_reversion_t: Option<f64>,
    /// None for a model without mean reversion. False: not different from zero (t < 2.86).
    pub mean_reversion_significant: Option<bool>,
    /// 2 kappa theta / sigma^2 for CIR (below 1: the Feller condition fails).
    pub feller_ratio: Option<f64>,
    pub warnings: Vec<String>,
}

impl Fit {
    pub fn get(&self, name: &str) -> Option<f64> {
        self.params.iter().find(|p| p.name == name).map(|p| p.value)
    }
    pub fn se(&self, name: &str) -> Option<f64> {
        self.params.iter().find(|p| p.name == name).and_then(|p| p.se)
    }
}

// ---------------------------------------------------------------------------
// Likelihoods
// ---------------------------------------------------------------------------

/// phi(kappa, dt) = (1 - exp(-2 kappa dt))/(2 kappa): the conditional variance of the OU process with sigma = 1.
fn phi(kappa: f64, dt: f64) -> f64 {
    -(-2.0 * kappa * dt).exp_m1() / (2.0 * kappa)
}

/// Log-likelihood of the transitions of the CKLS model: the conditional mean is exact (the drift
/// is linear), the conditional variance is `sigma^2 r^(2 gamma) phi(kappa, dt)`. For gamma = 0 this is the
/// exact Vasicek likelihood. For other gamma it is a quasi-likelihood (an Euler-type variance).
pub fn ckls_loglik(r: &[f64], dt: f64, kappa: f64, theta: f64, sigma: f64, gamma: f64) -> f64 {
    if !(kappa > 0.0 && sigma > 0.0 && kappa.is_finite() && theta.is_finite() && sigma.is_finite() && gamma.is_finite()) {
        return f64::NEG_INFINITY;
    }
    let e = (-kappa * dt).exp();
    let base = sigma * sigma * phi(kappa, dt);
    let two_pi = 2.0 * std::f64::consts::PI;
    chunked_sum(r.len() - 1, |i| {
        let m = theta + (r[i] - theta) * e;
        let var = if gamma == 0.0 { base } else { base * (2.0 * gamma * r[i].ln()).exp() };
        let d = r[i + 1] - m;
        -0.5 * ((two_pi * var).ln() + d * d / var)
    })
}

/// ln of the exact CIR transition density f(r1 | r0) over dt.
///
/// With c = 2 kappa/(sigma^2 (1 - e)), e = exp(-kappa dt), u = c r0 e, v = c r1 and a = 2 kappa theta/sigma^2,
/// the density of r1 is `c * sum_j Poisson(j; u) Gamma(v; a + j, 1)`. This is the noncentral
/// chi-square density as a Poisson mixture. The code sums the terms around the largest one, with
/// the ratio of two terms `T(j+1)/T(j) = u v / ((j+1)(a+j))`, until the terms are below 1e-18 of the sum.
fn cir_ln_density(r0: f64, r1: f64, c: f64, e: f64, a: f64) -> f64 {
    let (u, v) = (c * r0 * e, c * r1);
    let uv = u * v;
    let jstar = (-(a + 1.0) + ((a - 1.0) * (a - 1.0) + 4.0 * uv).sqrt()) / 2.0;
    let jm = jstar.round().max(0.0);
    let ln_t = -u + jm * u.ln() - lgamma(jm + 1.0) + (a + jm - 1.0) * v.ln() - v - lgamma(a + jm);
    let mut sum = 1.0;
    let mut term = 1.0;
    let mut j = jm;
    for _ in 0..50_000 {
        term *= uv / ((j + 1.0) * (a + j));
        j += 1.0;
        sum += term;
        if term < 1e-18 * sum {
            break;
        }
    }
    term = 1.0;
    j = jm;
    while j > 0.0 {
        term *= j * (a + j - 1.0) / uv;
        j -= 1.0;
        sum += term;
        if term < 1e-18 * sum {
            break;
        }
    }
    c.ln() + ln_t + sum.ln()
}

/// Exact log-likelihood of the CIR transitions of the sample (rates must be positive).
pub fn cir_loglik(r: &[f64], dt: f64, kappa: f64, theta: f64, sigma: f64) -> f64 {
    if !(kappa > 0.0 && theta > 0.0 && sigma > 0.0 && kappa.is_finite() && theta.is_finite() && sigma.is_finite()) {
        return f64::NEG_INFINITY;
    }
    let e = (-kappa * dt).exp();
    let c = 2.0 * kappa / (sigma * sigma * -(-kappa * dt).exp_m1());
    let a = 2.0 * kappa * theta / (sigma * sigma);
    chunked_sum(r.len() - 1, |i| cir_ln_density(r[i], r[i + 1], c, e, a))
}

// ---------------------------------------------------------------------------
// Building blocks
// ---------------------------------------------------------------------------

struct Ar1 {
    ols: Ols,
    kappa: f64,
    theta: f64,
    se_kappa: f64,
    se_theta: f64,
    /// (1 - beta)/se(beta)
    df_t: f64,
}

fn xy(r: &[f64]) -> (&[f64], &[f64]) {
    (&r[..r.len() - 1], &r[1..])
}

/// AR(1) regression of r(i+1) on r(i) and the exact map to (kappa, theta).
fn ar1(s: &Sample) -> Result<Ar1> {
    let (x, y) = xy(&s.r);
    let o = ols(x, y).ok_or("the rate series is constant (or has no variation): no model can be calibrated")?;
    if o.rss <= 1e-30 * (1.0 + s.r.iter().map(|v| v * v).sum::<f64>()) {
        return Err("the rate series is deterministic (the AR(1) residuals are zero)".into());
    }
    let b = o.beta;
    if b >= 1.0 {
        return Err(format!(
            "no mean reversion in this sample: the AR(1) coefficient {b:.6} is not below 1 (unit root); the implied kappa is {:.4} (not positive)",
            -b.ln() / s.dt
        ));
    }
    if b <= 0.0 {
        return Err(format!(
            "the AR(1) coefficient {b:.6} is not positive, so the exact map to a speed of mean reversion has no real solution (the sampling step is too long for this series or the series is noise)"
        ));
    }
    let kappa = -b.ln() / s.dt;
    let theta = o.alpha / (1.0 - b);
    let dt = s.dt;
    let g = move |p: &[f64]| vec![-p[1].ln() / dt, p[0] / (1.0 - p[1])];
    let cov = vec![vec![o.cov[0][0], o.cov[0][1]], vec![o.cov[1][0], o.cov[1][1]]];
    let se = delta_se(&g, &[o.alpha, b], &cov);
    let df_t = (1.0 - b) / o.cov[1][1].max(1e-300).sqrt();
    Ok(Ar1 { kappa, theta, se_kappa: se[0], se_theta: se[1], df_t, ols: o })
}

fn changes(r: &[f64]) -> Vec<f64> {
    r.windows(2).map(|w| w[1] - w[0]).collect()
}

/// The weights of the estimator for m changes, oldest first.
fn weights(est: VolEstimator, m: usize) -> Result<Vec<f64>> {
    match est {
        VolEstimator::CloseToClose => Ok(vec![1.0; m]),
        VolEstimator::Rolling(n) => {
            if n < 5 || n > m {
                return Err(format!("rolling:{n} needs 5 <= N <= {m} (the number of changes in the sample)"));
            }
            Ok((0..m).map(|i| if i + n >= m { 1.0 } else { 0.0 }).collect())
        }
        VolEstimator::Ewma(l) => Ok((0..m).map(|i| l.powi((m - 1 - i) as i32)).collect()),
    }
}

fn n_eff(w: &[f64]) -> f64 {
    let s: f64 = w.iter().sum();
    s * s / w.iter().map(|v| v * v).sum::<f64>()
}

/// Weighted realized volatility of the changes d (rate units per sqrt(year)).
fn weighted_realized_vol(d: &[f64], w: &[f64], dt: f64) -> f64 {
    let sw: f64 = w.iter().sum();
    let mean = d.iter().zip(w).map(|(x, w)| x * w).sum::<f64>() / sw;
    (d.iter().zip(w).map(|(x, w)| w * (x - mean) * (x - mean)).sum::<f64>() / sw / dt).sqrt()
}

/// sigma from the weighted mean of e_i^2 / v_i (v_i: conditional variance with sigma = 1).
fn sigma_from_weights(e: &[f64], v: &[f64], w: &[f64]) -> f64 {
    let sw: f64 = w.iter().sum();
    (e.iter().zip(v).zip(w).map(|((e, v), w)| w * e * e / v).sum::<f64>() / sw).sqrt()
}

struct Draft {
    params: Vec<Param>,
    loglik: Option<f64>,
    loglik_kind: &'static str,
    k: usize,
    mr_t: Option<f64>,
    feller: Option<f64>,
    realized_est: Option<f64>,
    warnings: Vec<String>,
}

fn param(name: &str, value: f64, se: Option<f64>) -> Param {
    Param { name: name.into(), value, se: se.filter(|s| s.is_finite()) }
}

fn require_positive(s: &Sample, model: &str) -> Result<()> {
    let bad = s.r.iter().filter(|v| **v <= 0.0).count();
    if bad > 0 {
        let min = s.r.iter().cloned().fold(f64::INFINITY, f64::min);
        return Err(format!(
            "{model} needs positive rates, and {bad} of {} observations are 0 or negative (lowest {min:.5}); use vasicek or holee",
            s.r.len()
        ));
    }
    Ok(())
}

/// Standard errors of ML parameters from the numeric Hessian of the log-likelihood at the optimum.
fn ml_se(f: &dyn Fn(&[f64]) -> f64, x: &[f64], warnings: &mut Vec<String>) -> Vec<Option<f64>> {
    match covariance_from_loglik(f, x) {
        Some(cov) => (0..x.len()).map(|i| Some(cov[i][i].sqrt())).collect(),
        None => {
            warnings.push("the standard errors could not be computed: the Hessian of the log-likelihood is not positive definite (flat likelihood or a boundary)".into());
            vec![None; x.len()]
        }
    }
}

fn mr_warning(t: f64, warnings: &mut Vec<String>) {
    if t < MR_T_THRESHOLD {
        warnings.push(format!(
            "mean reversion is not different from zero (t = {t:.2} < {MR_T_THRESHOLD}, the Dickey-Fuller 5% value): kappa and theta are weakly identified in this sample, and the long-run mean is not reliable"
        ));
    }
}

// ---------------------------------------------------------------------------
// Vasicek
// ---------------------------------------------------------------------------

fn vasicek(method: Method, s: &Sample) -> Result<Draft> {
    let a = ar1(s)?;
    let m = (s.r.len() - 1) as f64;
    let ph = phi(a.kappa, s.dt);
    let mut warnings = Vec::new();
    let (sigma, sigma_se, mr_t, realized_est, ml_params) = match method {
        Method::Ml => {
            let sigma = (a.ols.rss / m / ph).sqrt();
            let r = &s.r;
            let dt = s.dt;
            let f = move |p: &[f64]| ckls_loglik(r, dt, p[0], p[1], p[2], 0.0);
            let x = [a.kappa, a.theta, sigma];
            let se = ml_se(&f, &x, &mut warnings);
            let t = se[0].map(|se| a.kappa / se);
            (sigma, se[2], t.or(Some(a.df_t)), None, Some((se[0], se[1])))
        }
        Method::Ols => {
            let sigma = (a.ols.rss / (m - 2.0) / ph).sqrt();
            (sigma, Some(sigma / (2.0 * m).sqrt()), Some(a.df_t), None, None)
        }
        Method::RealVol(est) => {
            let w = weights(est, s.r.len() - 1)?;
            let v = vec![ph; s.r.len() - 1];
            let sigma = sigma_from_weights(&a.ols.residuals, &v, &w);
            let ne = n_eff(&w);
            if ne < 30.0 {
                warnings.push(format!("the volatility estimator has an effective sample of only {ne:.0} changes; the volatility is noisy"));
            }
            let rv = weighted_realized_vol(&changes(&s.r), &w, s.dt);
            (sigma, Some(sigma / (2.0 * ne).sqrt()), Some(a.df_t), Some(rv), None)
        }
        Method::Gmm => return Err("the method gmm is for the ckls model; use ml, ols or realvol".into()),
    };
    let (se_k, se_th) = match ml_params {
        Some((k, t)) => (k, t),
        None => (Some(a.se_kappa), Some(a.se_theta)),
    };
    if let Some(t) = mr_t {
        mr_warning(t, &mut warnings);
    }
    let ll = ckls_loglik(&s.r, s.dt, a.kappa, a.theta, sigma, 0.0);
    Ok(Draft {
        params: vec![param("kappa", a.kappa, se_k), param("theta", a.theta, se_th), param("sigma", sigma, sigma_se)],
        loglik: Some(ll),
        loglik_kind: "exact conditional (Gaussian)",
        k: 3,
        mr_t,
        feller: None,
        realized_est: realized_est,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Ho-Lee
// ---------------------------------------------------------------------------

fn holee(method: Method, s: &Sample) -> Result<Draft> {
    let (r, dt) = (&s.r, s.dt);
    let d = changes(r);
    let m = d.len();
    let mut warnings = Vec::new();
    let (est_vol, w): (Option<f64>, Vec<f64>) = match method {
        Method::Ml => (None, vec![1.0; m]),
        Method::RealVol(e) => {
            let w = weights(e, m)?;
            (Some(weighted_realized_vol(&d, &w, dt)), w)
        }
        Method::Ols | Method::Gmm => return Err("holee has no mean reversion: use the methods ml or realvol".into()),
    };
    let sigma = weighted_realized_vol(&d, &w, dt);
    if sigma <= 0.0 {
        return Err("the rate series is constant: the volatility is zero".into());
    }
    let ne = n_eff(&w);
    if ne < 30.0 {
        warnings.push(format!("the volatility estimator has an effective sample of only {ne:.0} changes; the volatility is noisy"));
    }
    // Likelihood of the random walk with a fitted drift (so 2 parameters: the drift and sigma).
    let dbar = d.iter().sum::<f64>() / m as f64;
    let ss: f64 = d.iter().map(|x| (x - dbar) * (x - dbar)).sum();
    let sig_ml = (ss / m as f64 / dt).sqrt();
    let ll = -0.5 * m as f64 * ((2.0 * std::f64::consts::PI * sig_ml * sig_ml * dt).ln() + 1.0);
    Ok(Draft {
        params: vec![param("sigma", sigma, Some(sigma / (2.0 * ne).sqrt()))],
        loglik: Some(ll),
        loglik_kind: "Gaussian random walk with fitted drift (2 parameters)",
        k: 2,
        mr_t: None,
        feller: None,
        realized_est: est_vol,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// CIR
// ---------------------------------------------------------------------------

fn cir_v1(r: f64, kappa: f64, theta: f64, beta: f64) -> f64 {
    r * beta * (1.0 - beta) / kappa + theta * (1.0 - beta) * (1.0 - beta) / (2.0 * kappa)
}

fn cir(method: Method, s: &Sample) -> Result<Draft> {
    require_positive(s, "cir")?;
    let m = s.r.len() - 1;
    let mut warnings = Vec::new();
    let a = ar1(s);
    // sigma from the ratio estimator, used by ols and realvol and as the start of ml
    let sigma_ols = |a: &Ar1, w: &[f64]| {
        let b = a.ols.beta;
        let v: Vec<f64> = s.r[..m].iter().map(|r| cir_v1(*r, a.kappa, a.theta, b)).collect();
        sigma_from_weights(&a.ols.residuals, &v, w)
    };
    let (kappa, theta, sigma, se, mr_t, realized_est);
    match method {
        Method::Ols | Method::RealVol(_) => {
            let a = a?;
            let (w, est) = match method {
                Method::RealVol(e) => (weights(e, m)?, Some(e)),
                _ => (vec![1.0; m], None),
            };
            let sg = sigma_ols(&a, &w);
            let ne = n_eff(&w);
            if est.is_some() && ne < 30.0 {
                warnings.push(format!("the volatility estimator has an effective sample of only {ne:.0} changes; the volatility is noisy"));
            }
            realized_est = est.map(|_| weighted_realized_vol(&changes(&s.r), &w, s.dt));
            (kappa, theta, sigma) = (a.kappa, a.theta, sg);
            se = [Some(a.se_kappa), Some(a.se_theta), Some(sg / (2.0 * ne).sqrt())];
            mr_t = Some(a.df_t);
        }
        Method::Ml => {
            // start: least squares if it works, else a generic start
            let mean = s.r.iter().sum::<f64>() / s.r.len() as f64;
            let (k0, t0, s0) = match &a {
                Ok(a) => (a.kappa, a.theta.max(1e-4), sigma_ols(a, &vec![1.0; m]).clamp(0.01, 2.0)),
                Err(_) => {
                    let rv = weighted_realized_vol(&changes(&s.r), &vec![1.0; m], s.dt);
                    (0.2, mean, (rv / mean.sqrt()).clamp(0.01, 2.0))
                }
            };
            let (r, dt) = (&s.r[..], s.dt);
            let nll = |x: &[f64]| {
                if x[0] < 1e-4f64.ln() || x[0] > 50f64.ln() || x[2] < 5e-3f64.ln() || x[2] > 5f64.ln() {
                    return 1e300;
                }
                let v = -cir_loglik(r, dt, x[0].exp(), x[1].exp(), x[2].exp());
                if v.is_finite() {
                    v
                } else {
                    1e300
                }
            };
            let (x, fx, ok) = nelder_mead(&nll, &[k0.max(1e-3).ln(), t0.ln(), s0.max(1e-6).ln()], 0.2, 1e-9, 1500);
            if fx >= 1e299 {
                return Err("the CIR likelihood has no finite value near the start point".into());
            }
            if !ok {
                warnings.push("the optimizer did not meet its tolerance; treat the parameters with care".into());
            }
            let (k, th, sg) = (x[0].exp(), x[1].exp(), x[2].exp());
            if sg < 5.05e-3 || sg > 4.95 || k > 49.5 {
                return Err(format!("the CIR maximum likelihood is at the edge of the allowed range (kappa {k:.3}, sigma {sg:.4}); the data do not fit the model"));
            }
            if k < 1.5e-4 {
                return Err(format!(
                    "the CIR maximum likelihood drives kappa to its lower bound (kappa = {k:.2e}): the sample shows no mean reversion"
                ));
            }
            let f = move |p: &[f64]| cir_loglik(r, dt, p[0], p[1], p[2]);
            let ses = ml_se(&f, &[k, th, sg], &mut warnings);
            (kappa, theta, sigma) = (k, th, sg);
            mr_t = ses[0].map(|se| k / se).or(a.as_ref().ok().map(|a| a.df_t));
            se = [ses[0], ses[1], ses[2]];
            realized_est = None;
        }
        Method::Gmm => return Err("the method gmm is for the ckls model; use ml, ols or realvol".into()),
    }
    if let Some(t) = mr_t {
        mr_warning(t, &mut warnings);
    }
    let ratio = 2.0 * kappa * theta / (sigma * sigma);
    if ratio < 1.0 {
        warnings.push(format!("the Feller condition fails: 2 kappa theta / sigma^2 = {ratio:.3} < 1, so the model rate can reach 0"));
    }
    let ll = cir_loglik(&s.r, s.dt, kappa, theta, sigma);
    Ok(Draft {
        params: vec![param("kappa", kappa, se[0]), param("theta", theta, se[1]), param("sigma", sigma, se[2])],
        loglik: Some(ll),
        loglik_kind: "exact conditional (noncentral chi-square)",
        k: 3,
        mr_t,
        feller: Some(ratio),
        realized_est: realized_est,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// CKLS
// ---------------------------------------------------------------------------

struct Gmm {
    kappa: f64,
    theta: f64,
    sigma: f64,
    gamma: f64,
    se: Option<[f64; 4]>,
    warnings: Vec<String>,
}

/// CKLS (1992) moment conditions. With e_i = r(i+1) - alpha - beta r(i):
/// E[e] = 0, E[e r] = 0, E[e^2 - s r^(2 gamma)] = 0, E[(e^2 - s r^(2 gamma)) r] = 0.
/// Four equations, four unknowns (alpha, beta, s, gamma): the system is exactly identified.
/// The first two are the normal equations of least squares, so (alpha, beta) come in closed form
/// and the exact map gives (kappa, theta). The last two give gamma by a one-dimensional root search
/// (s is eliminated), then s. The model volatility is sigma = sqrt(s / phi(kappa, dt)).
/// The standard errors come from the sandwich formula G^-1 S G^-T / n (no lag correction in S).
fn ckls_gmm(sample: &Sample) -> Result<Gmm> {
    require_positive(sample, "ckls")?;
    let a = ar1(sample)?;
    let (x, _) = xy(&sample.r);
    let n = x.len();
    let e = &a.ols.residuals;
    let e2: Vec<f64> = e.iter().map(|v| v * v).collect();
    let lnr: Vec<f64> = x.iter().map(|v| v.ln()).collect();
    let s_of = |g: f64| {
        let q: f64 = lnr.iter().map(|l| (2.0 * g * l).exp()).sum();
        e2.iter().sum::<f64>() / q
    };
    let h = |g: f64| {
        let s = s_of(g);
        e2.iter().zip(&lnr).zip(x).map(|((e2, l), r)| (e2 - s * (2.0 * g * l).exp()) * r).sum::<f64>()
    };
    let (lo, hi, step) = (-1.0, 3.0, 0.02);
    let mut roots = Vec::new();
    let mut g0 = lo;
    let mut h0 = h(g0);
    while g0 < hi - 1e-12 {
        let g1 = g0 + step;
        let h1 = h(g1);
        if h0 == 0.0 || h0 * h1 < 0.0 {
            let (mut l, mut r, mut hl) = (g0, g1, h0);
            for _ in 0..60 {
                let mid = 0.5 * (l + r);
                let hm = h(mid);
                if hl * hm <= 0.0 {
                    r = mid;
                } else {
                    l = mid;
                    hl = hm;
                }
            }
            roots.push(0.5 * (l + r));
        }
        g0 = g1;
        h0 = h1;
    }
    if roots.is_empty() {
        return Err(format!("the CKLS moment equations have no root for gamma in [{lo}, {hi}]; gamma is at the boundary or the data do not identify it"));
    }
    let mut warnings = Vec::new();
    roots.sort_by(|p, q| (p - 0.5).abs().partial_cmp(&(q - 0.5).abs()).unwrap());
    let gamma = roots[0];
    if roots.len() > 1 {
        warnings.push(format!("the moment equations have {} roots for gamma in [{lo}, {hi}]; the code took the one nearest 0.5 ({gamma:.3})", roots.len()));
    }
    let s = s_of(gamma);
    let ph = phi(a.kappa, sample.dt);
    let sigma = (s / ph).sqrt();
    // Standard errors
    let o = &a.ols;
    let phi_v = [o.alpha, o.beta, s, gamma];
    let mut g_mat = vec![vec![0.0; 4]; 4];
    let mut s_mat = vec![vec![0.0; 4]; 4];
    for i in 0..n {
        let (r, ei) = (x[i], e[i]);
        let q = (2.0 * gamma * lnr[i]).exp();
        let u = e2[i] - s * q;
        let g = [ei, ei * r, u, u * r];
        let d3 = [-2.0 * ei, -2.0 * ei * r, -q, -2.0 * s * q * lnr[i]];
        let rows = [[-1.0, -r, 0.0, 0.0], [-r, -r * r, 0.0, 0.0], d3, [d3[0] * r, d3[1] * r, d3[2] * r, d3[3] * r]];
        for p in 0..4 {
            for c in 0..4 {
                g_mat[p][c] += rows[p][c] / n as f64;
                s_mat[p][c] += g[p] * g[c] / n as f64;
            }
        }
    }
    let se = invert(&g_mat).and_then(|gi| {
        let mut gs = vec![vec![0.0; 4]; 4];
        for p in 0..4 {
            for c in 0..4 {
                for k in 0..4 {
                    gs[p][c] += gi[p][k] * s_mat[k][c];
                }
            }
        }
        let mut cov = vec![vec![0.0; 4]; 4];
        for p in 0..4 {
            for c in 0..4 {
                for k in 0..4 {
                    cov[p][c] += gs[p][k] * gi[c][k] / n as f64;
                }
            }
        }
        let dt = sample.dt;
        let map = move |v: &[f64]| {
            let k = -v[1].ln() / dt;
            vec![k, v[0] / (1.0 - v[1]), (v[2] / phi(k, dt)).sqrt(), v[3]]
        };
        let ses = delta_se(&map, &phi_v, &cov);
        if ses.iter().all(|v| v.is_finite()) {
            Some([ses[0], ses[1], ses[2], ses[3]])
        } else {
            None
        }
    });
    if se.is_none() {
        warnings.push("the GMM standard errors could not be computed (singular moment Jacobian)".into());
    }
    Ok(Gmm { kappa: a.kappa, theta: a.theta, sigma, gamma, se, warnings })
}

fn ckls(method: Method, s: &Sample, gamma_fixed: f64) -> Result<Draft> {
    require_positive(s, "ckls")?;
    let m = s.r.len() - 1;
    let mut warnings = Vec::new();
    let draft_from = |kappa: f64, theta: f64, sigma: f64, gamma: f64, se: [Option<f64>; 4], mr_t: Option<f64>, est: Option<f64>, mut warnings: Vec<String>| {
        if let Some(t) = mr_t {
            mr_warning(t, &mut warnings);
        }
        if gamma < 0.0 {
            warnings.push(format!("gamma = {gamma:.3} is negative: the model cannot be simulated by this library (gamma must be in [0, 3])"));
        }
        Draft {
            params: vec![param("kappa", kappa, se[0]), param("theta", theta, se[1]), param("sigma", sigma, se[2]), param("gamma", gamma, se[3])],
            loglik: Some(ckls_loglik(&s.r, s.dt, kappa, theta, sigma, gamma)),
            loglik_kind: "quasi (exact mean, Euler-type variance)",
            k: 4,
            mr_t,
            feller: None,
            realized_est: est,
            warnings,
        }
    };
    match method {
        Method::Gmm => {
            let g = ckls_gmm(s)?;
            let a = ar1(s)?;
            let se = g.se.map_or([None; 4], |v| [Some(v[0]), Some(v[1]), Some(v[2]), Some(v[3])]);
            Ok(draft_from(g.kappa, g.theta, g.sigma, g.gamma, se, Some(a.df_t), None, g.warnings))
        }
        Method::Ols => Err("the method ols is not defined for ckls (the variance is not constant); use gmm, ml or realvol".into()),
        Method::RealVol(est) => {
            if !(0.0..=3.0).contains(&gamma_fixed) {
                return Err(format!("gamma must be in [0, 3]; got {gamma_fixed}"));
            }
            let a = ar1(s)?;
            let w = weights(est, m)?;
            let ph = phi(a.kappa, s.dt);
            let v: Vec<f64> = s.r[..m].iter().map(|r| ph * r.powf(2.0 * gamma_fixed)).collect();
            let sg = sigma_from_weights(&a.ols.residuals, &v, &w);
            let ne = n_eff(&w);
            if ne < 30.0 {
                warnings.push(format!("the volatility estimator has an effective sample of only {ne:.0} changes; the volatility is noisy"));
            }
            let rv = weighted_realized_vol(&changes(&s.r), &w, s.dt);
            Ok(draft_from(a.kappa, a.theta, sg, gamma_fixed, [Some(a.se_kappa), Some(a.se_theta), Some(sg / (2.0 * ne).sqrt()), None], Some(a.df_t), Some(rv), warnings))
        }
        Method::Ml => {
            // profile out sigma: sigma^2 = mean(e^2 / (r^(2 gamma) phi)); optimise (ln kappa, theta, gamma)
            let (r, dt) = (&s.r[..], s.dt);
            let start = ckls_gmm(s).ok().map(|g| (g.kappa, g.theta, g.gamma)).or_else(|| ar1(s).ok().map(|a| (a.kappa, a.theta, 0.5))).unwrap_or((0.2, r.iter().sum::<f64>() / r.len() as f64, 0.5));
            let sigma_of = |k: f64, th: f64, g: f64| {
                let e = (-k * dt).exp();
                let ph = phi(k, dt);
                let tot = chunked_sum(m, |i| {
                    let d = r[i + 1] - (th + (r[i] - th) * e);
                    d * d / (ph * (2.0 * g * r[i].ln()).exp())
                });
                (tot / m as f64).sqrt()
            };
            let nll = |x: &[f64]| {
                if x[0] < 1e-4f64.ln() || x[0] > 50f64.ln() || x[2] < -1.0 || x[2] > 3.0 {
                    return 1e300;
                }
                let (k, th, g) = (x[0].exp(), x[1], x[2]);
                let v = -ckls_loglik(r, dt, k, th, sigma_of(k, th, g), g);
                if v.is_finite() {
                    v
                } else {
                    1e300
                }
            };
            let (x, fx, ok) = nelder_mead(&nll, &[start.0.max(1e-3).ln(), start.1, start.2.clamp(-0.9, 2.9)], 0.2, 1e-9, 2000);
            if fx >= 1e299 {
                return Err("the CKLS likelihood has no finite value near the start point".into());
            }
            if !ok {
                warnings.push("the optimizer did not meet its tolerance; treat the parameters with care".into());
            }
            let (k, th, g) = (x[0].exp(), x[1], x[2]);
            if k < 1.5e-4 {
                return Err(format!("the CKLS quasi-likelihood drives kappa to its lower bound (kappa = {k:.2e}): the sample shows no mean reversion"));
            }
            let sg = sigma_of(k, th, g);
            let f = move |p: &[f64]| ckls_loglik(r, dt, p[0], p[1], p[2], p[3]);
            let ses = ml_se(&f, &[k, th, sg, g], &mut warnings);
            let mr_t = ses[0].map(|se| k / se);
            Ok(draft_from(k, th, sg, g, [ses[0], ses[1], ses[2], ses[3]], mr_t, None, warnings))
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Calibrate one model to one sample with one method. `gamma` is used by the CKLS method `realvol` only.
pub fn calibrate(kind: ModelKind, method: Method, s: &Sample, gamma: f64) -> Result<Fit> {
    if s.r.iter().all(|v| *v == s.r[0]) {
        return Err("the rate series is constant: no model can be calibrated".into());
    }
    let draft = match kind {
        ModelKind::Vasicek => vasicek(method, s)?,
        ModelKind::Cir => cir(method, s)?,
        ModelKind::Ckls => ckls(method, s, gamma)?,
        ModelKind::HoLee => holee(method, s)?,
        ModelKind::HullWhite => {
            let mut d = vasicek(method, s)?;
            d.params.retain(|p| p.name != "theta");
            for p in d.params.iter_mut() {
                if p.name == "kappa" {
                    p.name = "a".into();
                }
            }
            d.warnings.push(
                "a and sigma come from the history (the Vasicek fit); theta(t) is not calibrated to history: the model fits it to a zero curve".into(),
            );
            d
        }
    };
    let mut warnings = s.warnings.clone();
    warnings.extend(draft.warnings);
    let d = changes(&s.r);
    let mean = d.iter().sum::<f64>() / d.len() as f64;
    let ss: f64 = d.iter().map(|x| (x - mean) * (x - mean)).sum();
    let realized_vol = (ss / (d.len() - 1) as f64 / s.dt).sqrt();
    Ok(Fit {
        model: kind.name().into(),
        method: method.label(),
        series: s.series.clone(),
        freq: s.freq.name().into(),
        start: dates::format(s.days[0]),
        end: dates::format(s.days[s.days.len() - 1]),
        n_transitions: s.r.len() - 1,
        dt_years: s.dt,
        params: draft.params,
        realized_vol,
        realized_vol_estimator: draft.realized_est,
        loglik: draft.loglik,
        loglik_kind: draft.loglik_kind.into(),
        n_params_aic: draft.k,
        aic: draft.loglik.map(|l| 2.0 * draft.k as f64 - 2.0 * l),
        mean_reversion_t: draft.mr_t,
        mean_reversion_significant: draft.mr_t.map(|t| t >= MR_T_THRESHOLD),
        feller_ratio: draft.feller,
        warnings,
    })
}
