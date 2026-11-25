//! Estimation of the process parameters of revenue growth and of the margins from the company panel
//! (ticket `T-20261119-dcf-process-calibration`, steps 2 and 3). `fa dcf calibrate`.
//!
//! The model of each series is an AR(1) with a long-run mean: `y_t = mu + phi (y_{t-1} - mu) + sigma e_t`.
//! The half-life is `ln 2 / -ln phi`. Four estimates of `phi` are made for each group of observations:
//!
//! - `pooled`: least squares with one intercept for the group (the mean is the group mean);
//! - `fe`: least squares with one intercept for each company (the within estimator). A short panel biases it
//!   toward zero (Nickell bias);
//! - `fe_corrected`: `fe` with the Nickell bias removed. The bias formula (first order in `1/T`, exact in
//!   `phi` and `T`) is solved for `phi`, with the number of observations of each company;
//! - `ah`: the Anderson-Hsiao estimate: first differences, and the level two years back as the instrument.
//!
//! Standard errors of the three least squares estimates and of `ah` are robust to correlation inside a company
//! (a company is a cluster). The standard error of `fe_corrected` is the standard error of `fe` times the slope of
//! the correction. A group has no standard error from a bootstrap, except the correlations of the shocks (a
//! bootstrap over companies).

use super::panel::{bucket, CompanyPanel, PanelRow, BUCKET_LABELS};
use super::DcfError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const TEMPLATES: [&str; 5] = ["standard", "bank", "insurer", "reit", "fund"];
/// A series of a growth above this value, or below the lower limit, is not used for the estimates.
pub const GROWTH_LIMITS: (f64, f64) = (-0.8, 3.0);
/// A margin outside this range is not used.
pub const MARGIN_LIMIT: f64 = 1.0;
/// The least number of observations and companies for a group.
pub const MIN_OBS: usize = 150;
pub const MIN_UNITS: usize = 25;

// ---------------------------------------------------------------------------------------------
// Pairs
// ---------------------------------------------------------------------------------------------

/// One observation of the AR(1) regression: the value `y` of a year, the value `x` of the year before, and
/// the value `w` two years before (if the chain has it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    pub unit: u32,
    pub year: i32,
    pub x: f64,
    pub y: f64,
    pub w: Option<f64>,
    /// Size bucket of the revenue of the year before.
    pub size: u8,
    /// Index in `TEMPLATES` (`TEMPLATES.len()` for another template).
    pub template: u8,
}

fn template_index(t: &str) -> u8 {
    TEMPLATES.iter().position(|x| *x == t).unwrap_or(TEMPLATES.len()) as u8
}

/// The pairs of one series. `value(row)` gives the value of a row. Companies are in the sequence of the panel,
/// so the pairs of a company are together.
pub fn pairs(companies: &[CompanyPanel], value: &dyn Fn(&PanelRow) -> Option<f64>) -> Vec<Pair> {
    let mut out = Vec::new();
    for (u, c) in companies.iter().enumerate() {
        let template = template_index(&c.template);
        for k in 1..c.rows.len() {
            let r = &c.rows[k];
            if !r.prev_ok {
                continue;
            }
            let (Some(y), Some(x)) = (value(r), value(&c.rows[k - 1])) else { continue };
            let w = if k >= 2 && c.rows[k - 1].prev_ok { value(&c.rows[k - 2]) } else { None };
            out.push(Pair { unit: u as u32, year: r.year, x, y, w, size: bucket(c.rows[k - 1].revenue) as u8, template });
        }
    }
    out
}

/// The mean growth of each fiscal year over all companies (growth with the jump rule and the limits).
pub fn year_means(companies: &[CompanyPanel]) -> HashMap<i32, f64> {
    let mut acc: HashMap<i32, (f64, usize)> = HashMap::new();
    for c in companies {
        for r in &c.rows {
            if let Some(g) = limited_growth(r) {
                let e = acc.entry(r.year).or_insert((0.0, 0));
                e.0 += g;
                e.1 += 1;
            }
        }
    }
    acc.into_iter().map(|(y, (s, n))| (y, s / n as f64)).collect()
}

/// Growth with the jump rule (removed) and the limits (removed).
pub fn limited_growth(r: &PanelRow) -> Option<f64> {
    r.growth_clean().filter(|g| *g > GROWTH_LIMITS.0 && *g < GROWTH_LIMITS.1)
}

pub fn limited_margin(m: Option<f64>) -> Option<f64> {
    m.filter(|v| v.is_finite() && v.abs() <= MARGIN_LIMIT)
}

// ---------------------------------------------------------------------------------------------
// The estimators
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Est {
    pub phi: f64,
    pub phi_se: f64,
    pub mean: f64,
    pub sigma: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ar1Fit {
    pub n_obs: usize,
    pub n_units: usize,
    pub pooled: Est,
    pub fe: Est,
    pub fe_corrected: Est,
    pub ah: Option<Est>,
    /// Number of observations of the Anderson-Hsiao estimate.
    pub ah_obs: usize,
    /// Standard error of the mean (cluster robust; the mean of the values `y`).
    pub mean_se: f64,
    /// Standard deviation of the long-run means of the companies (the noise of the company means removed).
    pub mean_sd_units: f64,
    /// The same spread from the quartiles of the company means (interquartile range / 1.349, the median noise removed).
    pub mean_sd_robust: f64,
    /// (P90 - P10) / (2.563 x the standard deviation) of the within residuals: 1 for a normal sample, below 1 for fat tails.
    pub shape_sigma: f64,
    /// The same ratio of the company means.
    pub shape_means: f64,
}

/// Half-life in years: `ln 2 / -ln phi` for `0 < phi < 1`; 0 for `phi <= 0`; 99 for `phi >= 1`.
pub fn half_life(phi: f64) -> f64 {
    if phi <= 0.0 {
        0.0
    } else if phi >= 1.0 {
        99.0
    } else {
        (std::f64::consts::LN_2 / -phi.ln()).min(99.0)
    }
}

/// The first order Nickell bias of the within estimator for a company with `t` observations.
pub fn nickell_bias(phi: f64, t: f64) -> f64 {
    if t < 2.0 || phi.abs() >= 0.9999 {
        return 0.0;
    }
    let pt = phi.powf(t);
    let a = 1.0 - (1.0 - pt) / (t * (1.0 - phi));
    let b = 1.0 - 2.0 * phi / ((1.0 - phi) * (t - 1.0)) * a;
    -(1.0 + phi) / (t - 1.0) * a / b
}

/// The `phi` for which the mean bias (weights: the numbers of observations) brings the within estimate.
pub fn nickell_correct(phi_fe: f64, sizes: &[usize]) -> f64 {
    let total: f64 = sizes.iter().map(|n| *n as f64).sum();
    let bias = |phi: f64| sizes.iter().map(|n| *n as f64 * nickell_bias(phi, *n as f64)).sum::<f64>() / total;
    let f = |phi: f64| phi + bias(phi) - phi_fe;
    let (mut lo, mut hi) = (-0.95, 0.98);
    if f(lo) > 0.0 {
        return lo;
    }
    if f(hi) < 0.0 {
        return hi;
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// The variance of the mean of `n` values of an AR(1) in steady state, in units of `sigma^2`.
pub(crate) fn mean_variance_factor(phi: f64, n: f64) -> f64 {
    let n_int = n.round().max(1.0) as usize;
    let mut s = 1.0;
    for k in 1..n_int {
        s += 2.0 * (1.0 - k as f64 / n) * phi.powi(k as i32);
    }
    s / ((1.0 - phi * phi) * n)
}

/// The runs of pairs with the same unit.
fn runs(p: &[Pair]) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for i in 1..=p.len() {
        if i == p.len() || p[i].unit != p[start].unit {
            out.push(start..i);
            start = i;
        }
    }
    out
}

/// The slope of `y` on `x` with a cluster robust standard error. `xs`, `ys`: the values (demeaned already).
fn slope(p: &[Pair], units: &[std::ops::Range<usize>], x: &[f64], y: &[f64], n_regressors_df: usize) -> (f64, f64, f64) {
    let sxx: f64 = x.iter().map(|v| v * v).sum();
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| a * b).sum();
    let phi = sxy / sxx;
    let mut rss = 0.0;
    let mut meat = 0.0;
    for r in units {
        let mut score = 0.0;
        for i in r.clone() {
            let e = y[i] - phi * x[i];
            rss += e * e;
            score += x[i] * e;
        }
        meat += score * score;
    }
    let g = units.len() as f64;
    let n = p.len() as f64;
    let se = (meat / (sxx * sxx) * g / (g - 1.0).max(1.0)).sqrt();
    let sigma = (rss / (n - n_regressors_df as f64).max(1.0)).sqrt();
    (phi, se, sigma)
}

fn mean_of(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// The long-run mean of the group from the slope: `(mean y - phi mean x) / (1 - phi)`.
fn group_mean(my: f64, mx: f64, phi: f64) -> f64 {
    if phi < 0.999 {
        (my - phi * mx) / (1.0 - phi)
    } else {
        my
    }
}

/// The estimates of one group of pairs (sorted by unit). `None` if the group is too small.
pub fn fit_ar1(p: &[Pair]) -> Option<Ar1Fit> {
    let units = runs(p);
    if p.len() < 10 || units.len() < 3 {
        return None;
    }
    let n = p.len();
    let xs: Vec<f64> = p.iter().map(|q| q.x).collect();
    let ys: Vec<f64> = p.iter().map(|q| q.y).collect();
    let (mx, my) = (mean_of(&xs), mean_of(&ys));

    // pooled, one intercept
    let xd: Vec<f64> = xs.iter().map(|v| v - mx).collect();
    let yd: Vec<f64> = ys.iter().map(|v| v - my).collect();
    let (phi_p, se_p, sigma_p) = slope(p, &units, &xd, &yd, 2);
    let pooled = Est { phi: phi_p, phi_se: se_p, mean: group_mean(my, mx, phi_p), sigma: sigma_p };

    // within: demean each company (companies with one observation have no information)
    let mut xw = vec![0.0; n];
    let mut yw = vec![0.0; n];
    let mut ybar_units: Vec<(f64, usize)> = Vec::new();
    let mut sizes: Vec<usize> = Vec::new();
    for r in &units {
        let k = r.len();
        let (ax, ay) = (mean_of(&xs[r.clone()]), mean_of(&ys[r.clone()]));
        for i in r.clone() {
            xw[i] = xs[i] - ax;
            yw[i] = ys[i] - ay;
        }
        ybar_units.push((ay, k));
        if k >= 2 {
            sizes.push(k);
        }
    }
    let g_used = sizes.len();
    let df_fe = g_used + 1;
    let (phi_f, se_f, sigma_f) = slope(p, &units, &xw, &yw, df_fe);
    // slope() uses n minus df; the companies with one observation are not in g_used, so correct n
    let single = units.iter().filter(|r| r.len() < 2).count();
    let sigma_f = sigma_f * (((n as f64 - df_fe as f64).max(1.0)) / ((n - single) as f64 - df_fe as f64).max(1.0)).sqrt();
    let fe = Est { phi: phi_f, phi_se: se_f, mean: group_mean(my, mx, phi_f), sigma: sigma_f };

    // Nickell correction
    let phi_c = nickell_correct(phi_f, &sizes);
    let h = 1e-3;
    let slope_c = (nickell_correct(phi_f + h, &sizes) - nickell_correct(phi_f - h, &sizes)) / (2.0 * h);
    let mut rss = 0.0;
    for i in 0..n {
        let e = yw[i] - phi_c * xw[i];
        rss += e * e;
    }
    let sigma_c = (rss / ((n - single) as f64 - df_fe as f64).max(1.0)).sqrt();
    let fe_corrected = Est { phi: phi_c, phi_se: se_f * slope_c.abs(), mean: group_mean(my, mx, phi_c), sigma: sigma_c };

    // Anderson-Hsiao: dy_t = phi dy_{t-1} + de_t, instrument w = y_{t-2}
    let sel: Vec<usize> = (0..n).filter(|i| p[*i].w.is_some()).collect();
    let mut ah = None;
    let ah_obs = sel.len();
    if sel.len() >= 30 {
        let mw = mean_of(&sel.iter().map(|i| p[*i].w.unwrap()).collect::<Vec<_>>());
        let (mut szy, mut szx) = (0.0, 0.0);
        for i in &sel {
            let z = p[*i].w.unwrap() - mw;
            szy += z * (p[*i].y - p[*i].x);
            szx += z * (p[*i].x - p[*i].w.unwrap());
        }
        if szx.abs() > 1e-12 {
            let phi_a = szy / szx;
            let mut meat = 0.0;
            let mut rss = 0.0;
            let mut cnt = 0usize;
            for r in &units {
                let mut score = 0.0;
                let mut used = false;
                for i in r.clone() {
                    if let Some(w) = p[i].w {
                        let e = (p[i].y - p[i].x) - phi_a * (p[i].x - w);
                        score += (w - mw) * e;
                        rss += e * e;
                        used = true;
                    }
                }
                if used {
                    cnt += 1;
                }
                meat += score * score;
            }
            let g = cnt as f64;
            let se = (meat / (szx * szx) * g / (g - 1.0).max(1.0)).sqrt();
            // innovations: Var(de) = 2 sigma^2
            let sigma_a = (rss / sel.len() as f64 / 2.0).sqrt();
            ah = Some(Est { phi: phi_a, phi_se: se, mean: group_mean(my, mx, phi_a), sigma: sigma_a });
        }
    }

    // standard error of the mean and the spread of the company means
    let g = units.len() as f64;
    let mean_se = (units.iter().map(|r| ys[r.clone()].iter().map(|v| v - my).sum::<f64>().powi(2)).sum::<f64>() * g / (g - 1.0)).sqrt() / n as f64;
    let mean_sd_units = {
        let multi: Vec<&(f64, usize)> = ybar_units.iter().filter(|(_, k)| *k >= 2).collect();
        if multi.len() < 3 {
            0.0
        } else {
            let m = mean_of(&multi.iter().map(|(a, _)| *a).collect::<Vec<_>>());
            let var = multi.iter().map(|(a, _)| (a - m) * (a - m)).sum::<f64>() / (multi.len() as f64 - 1.0);
            let noise = multi.iter().map(|(_, k)| fe_corrected.sigma.powi(2) * mean_variance_factor(phi_c.clamp(-0.9, 0.99), *k as f64)).sum::<f64>() / multi.len() as f64;
            (var - noise).max(0.0).sqrt()
        }
    };
    // the same spread from the quartiles of the company means (a few extreme company means do not move it): the
    // interquartile range over 1.349 (the range of a normal distribution in standard deviations), less the median noise
    let mean_sd_robust = {
        let multi: Vec<&(f64, usize)> = ybar_units.iter().filter(|(_, k)| *k >= 2).collect();
        if multi.len() < 3 {
            0.0
        } else {
            let mut a: Vec<f64> = multi.iter().map(|(a, _)| *a).collect();
            a.sort_by(f64::total_cmp);
            let sd = (quantile_sorted(&a, 0.75) - quantile_sorted(&a, 0.25)) / 1.349;
            let mut noise: Vec<f64> = multi.iter().map(|(_, k)| fe_corrected.sigma.powi(2) * mean_variance_factor(phi_c.clamp(-0.9, 0.99), *k as f64)).collect();
            noise.sort_by(f64::total_cmp);
            (sd * sd - quantile_sorted(&noise, 0.5)).max(0.0).sqrt()
        }
    };
    // the shape ratios: (P90 - P10) / (2.563 x the standard deviation) of the same sample. 1 for a normal sample,
    // below 1 for fat tails. The within residuals of the companies with 2 or more observations; the company means.
    let shape = |v: &mut Vec<f64>| -> f64 {
        if v.len() < 20 {
            return 1.0;
        }
        v.sort_by(f64::total_cmp);
        let m = mean_of(v);
        let sd = (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len() as f64 - 1.0)).sqrt();
        if sd > 0.0 { (quantile_sorted(v, 0.9) - quantile_sorted(v, 0.1)) / (2.0 * 1.2815515655446004 * sd) } else { 1.0 }
    };
    let mut res: Vec<f64> = units.iter().filter(|r| r.len() >= 2).flat_map(|r| r.clone().map(|i| yw[i] - phi_c * xw[i])).collect();
    let shape_sigma = shape(&mut res);
    let mut means: Vec<f64> = ybar_units.iter().filter(|(_, k)| *k >= 2).map(|(a, _)| *a).collect();
    let shape_means = shape(&mut means);
    Some(Ar1Fit { n_obs: n, n_units: units.len(), pooled, fe, fe_corrected, ah, ah_obs, mean_se, mean_sd_units, mean_sd_robust, shape_sigma, shape_means })
}

/// The quantile `q` of a sorted slice with at least one value (linear between the two nearest values).
fn quantile_sorted(v: &[f64], q: f64) -> f64 {
    let pos = q.clamp(0.0, 1.0) * (v.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    v[lo] + (v[hi] - v[lo]) * (pos - lo as f64)
}

/// The residuals of the within model for a given `phi`: `(unit, year, residual)`.
pub fn fe_residuals(p: &[Pair], phi: f64) -> Vec<(u32, i32, f64)> {
    let mut out = Vec::with_capacity(p.len());
    for r in runs(p) {
        if r.len() < 2 {
            continue;
        }
        let ax = mean_of(&p[r.clone()].iter().map(|q| q.x).collect::<Vec<_>>());
        let ay = mean_of(&p[r.clone()].iter().map(|q| q.y).collect::<Vec<_>>());
        for q in &p[r] {
            out.push((q.unit, q.year, (q.y - ay) - phi * (q.x - ax)));
        }
    }
    out
}

/// The correlation of the residuals of two series for the same company and year, and its standard error from
/// a bootstrap over companies (`reps` resamples, seed fixed).
pub fn shock_correlation(a: &[(u32, i32, f64)], b: &[(u32, i32, f64)], reps: usize) -> Option<(f64, f64, usize)> {
    let index: HashMap<(u32, i32), f64> = b.iter().map(|(u, y, e)| ((*u, *y), *e)).collect();
    let mut matched: Vec<(u32, f64, f64)> = a.iter().filter_map(|(u, y, e)| index.get(&(*u, *y)).map(|f| (*u, *e, *f))).collect();
    matched.sort_by_key(|m| m.0);
    if matched.len() < 30 {
        return None;
    }
    let corr = |rows: &[(f64, f64)]| -> f64 {
        let n = rows.len() as f64;
        let (ma, mb) = (rows.iter().map(|r| r.0).sum::<f64>() / n, rows.iter().map(|r| r.1).sum::<f64>() / n);
        let (mut saa, mut sbb, mut sab) = (0.0, 0.0, 0.0);
        for (x, y) in rows {
            saa += (x - ma) * (x - ma);
            sbb += (y - mb) * (y - mb);
            sab += (x - ma) * (y - mb);
        }
        sab / (saa * sbb).sqrt()
    };
    let all: Vec<(f64, f64)> = matched.iter().map(|m| (m.1, m.2)).collect();
    let rho = corr(&all);
    // clusters
    let mut clusters: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut last = u32::MAX;
    for m in &matched {
        if m.0 != last {
            clusters.push(Vec::new());
            last = m.0;
        }
        clusters.last_mut().expect("a cluster").push((m.1, m.2));
    }
    let mut rng = super::nprandom::Pcg64::new(12345);
    let mut draws = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut sample: Vec<(f64, f64)> = Vec::with_capacity(all.len());
        for _ in 0..clusters.len() {
            let k = (rng.next_u64() % clusters.len() as u64) as usize;
            sample.extend_from_slice(&clusters[k]);
        }
        let c = corr(&sample);
        if c.is_finite() {
            draws.push(c);
        }
    }
    let m = mean_of(&draws);
    let se = (draws.iter().map(|d| (d - m) * (d - m)).sum::<f64>() / (draws.len() as f64 - 1.0).max(1.0)).sqrt();
    Some((rho, se, matched.len()))
}

// ---------------------------------------------------------------------------------------------
// The table of parameters
// ---------------------------------------------------------------------------------------------

/// The parameters of one AR(1) series for one group. `phi` is the recommended value: the within estimate with
/// the Nickell correction. The other estimates stay in the table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ar1Params {
    pub phi: f64,
    pub phi_se: f64,
    pub half_life: f64,
    /// Long-run mean of the group.
    pub mean: f64,
    pub mean_se: f64,
    /// Standard deviation of the long-run means of the companies of the group.
    pub mean_sd_units: f64,
    /// The same spread from the quartiles of the company means (`Ar1Fit::mean_sd_robust`). A measurement for the rule
    /// `tau_robust`: it is not in the table file, and a table that was read from a file has `None`.
    #[serde(skip)]
    pub mean_sd_robust: Option<f64>,
    /// The shape ratios of the shock and of the company means (`Ar1Fit::shape_sigma`, `shape_means`) for the rule
    /// `band_q`. Not in the table file; a table that was read from a file has `None`.
    #[serde(skip)]
    pub shape: Option<(f64, f64)>,
    /// Standard deviation of the shock.
    pub sigma: f64,
    pub n_obs: usize,
    pub n_units: usize,
    pub phi_pooled: f64,
    pub phi_pooled_se: f64,
    pub phi_fe: f64,
    pub phi_fe_se: f64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub phi_ah: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub phi_ah_se: Option<f64>,
    pub sigma_pooled: f64,
    pub mean_pooled: f64,
}

/// Six decimals (the table is a file that people read).
fn r6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

impl Ar1Params {
    fn from_fit(f: &Ar1Fit) -> Ar1Params {
        Ar1Params::from_fit_exact(f).rounded()
    }

    fn rounded(mut self) -> Ar1Params {
        for v in [&mut self.phi, &mut self.phi_se, &mut self.half_life, &mut self.mean, &mut self.mean_se, &mut self.mean_sd_units, &mut self.sigma, &mut self.phi_pooled, &mut self.phi_pooled_se, &mut self.phi_fe, &mut self.phi_fe_se, &mut self.sigma_pooled, &mut self.mean_pooled] {
            *v = r6(*v);
        }
        self.phi_ah = self.phi_ah.map(r6);
        self.phi_ah_se = self.phi_ah_se.map(r6);
        self
    }

    fn from_fit_exact(f: &Ar1Fit) -> Ar1Params {
        Ar1Params {
            phi: f.fe_corrected.phi,
            phi_se: f.fe_corrected.phi_se,
            half_life: half_life(f.fe_corrected.phi),
            mean: f.fe_corrected.mean,
            mean_se: f.mean_se,
            mean_sd_units: f.mean_sd_units,
            mean_sd_robust: Some(f.mean_sd_robust),
            shape: Some((f.shape_sigma, f.shape_means)),
            sigma: f.fe_corrected.sigma,
            n_obs: f.n_obs,
            n_units: f.n_units,
            phi_pooled: f.pooled.phi,
            phi_pooled_se: f.pooled.phi_se,
            phi_fe: f.fe.phi,
            phi_fe_se: f.fe.phi_se,
            phi_ah: f.ah.map(|a| a.phi),
            phi_ah_se: f.ah.map(|a| a.phi_se),
            sigma_pooled: f.pooled.sigma,
            mean_pooled: f.pooled.mean,
        }
    }
}

/// The parameter set of one group: a size bucket (`all` or a label of `BUCKET_LABELS`) and a template (`all` or
/// a template).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessPriors {
    pub size: String,
    pub template: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub growth: Option<Ar1Params>,
    /// Growth less the mean growth of the fiscal year over all companies.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub growth_excess: Option<Ar1Params>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub operating_margin: Option<Ar1Params>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fcf_margin: Option<Ar1Params>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub corr_growth_operating_margin: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub corr_growth_operating_margin_se: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub corr_growth_fcf_margin: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub corr_growth_fcf_margin_se: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableMeta {
    pub date: String,
    pub command: String,
    pub companies: usize,
    pub company_years: usize,
    pub growth_observations: usize,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessTable {
    pub meta: TableMeta,
    pub cell: Vec<ProcessPriors>,
}

impl ProcessTable {
    pub fn from_toml(text: &str) -> Result<ProcessTable, DcfError> {
        toml::from_str(text).map_err(|x| DcfError(format!("process table: {x}")))
    }

    pub fn load(path: &std::path::Path) -> Result<ProcessTable, DcfError> {
        let text = std::fs::read_to_string(path).map_err(|x| DcfError(format!("{}: {x}", path.display())))?;
        ProcessTable::from_toml(&text)
    }

    pub fn to_toml(&self) -> Result<String, DcfError> {
        toml::to_string_pretty(self).map_err(|x| DcfError(format!("process table: {x}")))
    }

    fn find(&self, size: &str, template: &str) -> Option<&ProcessPriors> {
        self.cell.iter().find(|c| c.size == size && c.template == template && c.growth.is_some())
    }

    /// The parameters for a company with this revenue (dollars) and template. The sector is not used: the
    /// cache has no SIC code, so no cell has a sector (the argument is for later use). The cell of the size
    /// bucket and the template is used if it has `MIN_OBS` growth observations and `MIN_UNITS` companies;
    /// if not, the cell of the size bucket for all templates, then the cell of the template for all sizes,
    /// then all. The first value of the result is the cell.
    pub fn priors_for(&self, revenue: f64, template: &str, sector: Option<&str>) -> Option<ProcessPriors> {
        let _ = sector;
        let size = BUCKET_LABELS[bucket(revenue)];
        let enough = |c: &ProcessPriors| c.growth.as_ref().is_some_and(|g| g.n_obs >= MIN_OBS && g.n_units >= MIN_UNITS);
        [(size, template), (size, "all"), ("all", template), ("all", "all")]
            .iter()
            .filter_map(|(s, t)| self.find(s, t))
            .find(|c| enough(c))
            .cloned()
    }
}

/// Which pairs belong to a group: a size bucket (`None`: all) and a template index (`None`: all).
fn select(p: &[Pair], size: Option<usize>, template: Option<usize>) -> Vec<Pair> {
    p.iter().filter(|q| size.is_none_or(|s| q.size as usize == s) && template.is_none_or(|t| q.template as usize == t)).copied().collect()
}

/// All series of a panel.
pub struct Series {
    pub growth: Vec<Pair>,
    pub growth_excess: Vec<Pair>,
    pub operating_margin: Vec<Pair>,
    pub fcf_margin: Vec<Pair>,
}

pub fn series_of(companies: &[CompanyPanel]) -> Series {
    let ym = year_means(companies);
    Series {
        growth: pairs(companies, &|r| limited_growth(r)),
        growth_excess: pairs(companies, &|r| limited_growth(r).map(|g| g - ym.get(&r.year).copied().unwrap_or(0.0))),
        operating_margin: pairs(companies, &|r| limited_margin(r.operating_margin())),
        fcf_margin: pairs(companies, &|r| limited_margin(r.fcf_margin())),
    }
}

fn params(p: &[Pair]) -> Option<Ar1Params> {
    fit_ar1(p).map(|f| Ar1Params::from_fit(&f))
}

/// The cell of one group: all series and the shock correlations.
pub fn estimate_cell(s: &Series, size: Option<usize>, template: Option<usize>, reps: usize) -> ProcessPriors {
    let g = select(&s.growth, size, template);
    let ge = select(&s.growth_excess, size, template);
    let om = select(&s.operating_margin, size, template);
    let fm = select(&s.fcf_margin, size, template);
    let big = |p: &[Pair]| p.len() >= MIN_OBS && runs(p).len() >= MIN_UNITS;
    let growth = if big(&g) { params(&g) } else { None };
    let correlation = |margin: &[Pair], growth_params: &Option<Ar1Params>| -> Option<(f64, f64)> {
        let gp = growth_params.as_ref()?;
        let mp = if big(margin) { params(margin) } else { None }?;
        let a = fe_residuals(&g, gp.phi);
        let b = fe_residuals(margin, mp.phi);
        shock_correlation(&a, &b, reps).map(|(r, se, _)| (r, se))
    };
    let co = correlation(&om, &growth);
    let cf = correlation(&fm, &growth);
    ProcessPriors {
        size: size.map_or("all".to_string(), |i| BUCKET_LABELS[i].to_string()),
        template: template.map_or("all".to_string(), |i| TEMPLATES[i].to_string()),
        growth,
        growth_excess: if big(&ge) { params(&ge) } else { None },
        operating_margin: if big(&om) { params(&om) } else { None },
        fcf_margin: if big(&fm) { params(&fm) } else { None },
        corr_growth_operating_margin: co.map(|c| r6(c.0)),
        corr_growth_operating_margin_se: co.map(|c| r6(c.1)),
        corr_growth_fcf_margin: cf.map(|c| r6(c.0)),
        corr_growth_fcf_margin_se: cf.map(|c| r6(c.1)),
    }
}

/// All cells: size (6 values) by template (6 values), those that have no series are not in the table.
pub fn estimate_table(companies: &[CompanyPanel], reps: usize) -> Vec<ProcessPriors> {
    let s = series_of(companies);
    let mut jobs: Vec<(Option<usize>, Option<usize>)> = Vec::new();
    for size in std::iter::once(None).chain((0..BUCKET_LABELS.len()).map(Some)) {
        for template in std::iter::once(None).chain((0..TEMPLATES.len()).map(Some)) {
            jobs.push((size, template));
        }
    }
    use rayon::prelude::*;
    let cells: Vec<ProcessPriors> = jobs.par_iter().map(|(sz, t)| estimate_cell(&s, *sz, *t, reps)).collect();
    cells.into_iter().filter(|c| c.growth.is_some() || c.operating_margin.is_some() || c.fcf_margin.is_some()).collect()
}

// ---------------------------------------------------------------------------------------------
// The fade table (ticket `T-20261119-dcf-auto-model`): growth k years after a year of high growth
// ---------------------------------------------------------------------------------------------

/// The growth `k` years after the start years of one group: P10, P50 and P90.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FadeStep {
    pub k: usize,
    pub n: usize,
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
}

/// One group of start years: a size bucket (`all` or a label of `BUCKET_LABELS`) and a growth threshold.
/// `group_mean` is the pooled mean growth of the size bucket (the mean that `fa dcf calibrate` prints
/// in the fade text).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FadeCell {
    pub size: String,
    pub threshold: f64,
    pub n_start: usize,
    pub start_mean: f64,
    pub start_median: f64,
    pub group_mean: f64,
    pub step: Vec<FadeStep>,
}

/// The `[[fade]]` part of a table file. A table file that has no such part gives an empty list.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FadeFile {
    #[serde(default)]
    pub fade: Vec<FadeCell>,
}

impl FadeFile {
    pub fn from_toml(text: &str) -> Result<FadeFile, DcfError> {
        toml::from_str(text).map_err(|x| DcfError(format!("fade table: {x}")))
    }

    pub fn to_toml(&self) -> Result<String, DcfError> {
        toml::to_string_pretty(self).map_err(|x| DcfError(format!("fade table: {x}")))
    }
}

/// The half-life, in years, of the slow fade of the growth of a fast grower toward the group mean. The rule:
/// for each threshold of the cells of all sizes, the median growth `k = 3` years after the start and `k = 10`
/// years after it give the share of the gap to the group mean that is left, `r = (p50_10 - m) / (p50_3 - m)`;
/// the yearly factor is `r^(1/7)` and the half-life is `ln 2 / -ln(factor)`. The result is the median over
/// the thresholds that have both steps, a gap above 2 points and `r` in (0, 1). Years 1 and 2 are not used:
/// the fast part of the process (the AR(1) shock) gives them.
pub fn slow_half_life(cells: &[FadeCell]) -> Option<f64> {
    let mut hl: Vec<f64> = Vec::new();
    for c in cells.iter().filter(|c| c.size == "all") {
        let at = |k: usize| c.step.iter().find(|s| s.k == k).map(|s| s.p50);
        let (Some(g3), Some(g10)) = (at(3), at(10)) else { continue };
        let gap = g3 - c.group_mean;
        if gap < 0.02 {
            continue;
        }
        let r = (g10 - c.group_mean) / gap;
        if r > 0.0 && r < 1.0 {
            hl.push(std::f64::consts::LN_2 / -(r.powf(1.0 / 7.0)).ln());
        }
    }
    if hl.is_empty() {
        return None;
    }
    hl.sort_by(|a, b| a.total_cmp(b));
    Some(hl[hl.len() / 2])
}

/// The spread of the growth one year after a year of high growth, over the growth of the start year: for each
/// threshold of the cells of all sizes, `(P90 - P10) / (2 * 1.2816) / start_mean` at `k = 1`; the result is the median
/// over the thresholds. A company in a high-growth state has a next-year spread of this share of its growth.
pub fn level_spread_ratio(cells: &[FadeCell]) -> Option<f64> {
    let mut v: Vec<f64> = cells
        .iter()
        .filter(|c| c.size == "all" && c.start_mean > 0.0)
        .filter_map(|c| c.step.iter().find(|s| s.k == 1).map(|s| (s.p90 - s.p10) / (2.0 * 1.2815515655446004) / c.start_mean))
        .collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    Some(v[v.len() / 2])
}
