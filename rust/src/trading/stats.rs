//! Sums, means and standard deviations that add in the same order as numpy (pairwise
//! summation), so a mean of the Rust code is the same as the mean of the Python code to the
//! last digits.

/// numpy `add.reduce` for a contiguous float array.
pub fn sum(a: &[f64]) -> f64 {
    0.0 + pairwise(a)
}

fn pairwise(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0;
        for &x in a {
            res += x;
        }
        res
    } else if n <= 128 {
        let mut r = [a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]];
        let mut i = 8;
        while i < n - n % 8 {
            for (j, slot) in r.iter_mut().enumerate() {
                *slot += a[i + j];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < n {
            res += a[i];
            i += 1;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        pairwise(&a[..n2]) + pairwise(&a[n2..])
    }
}

/// `a.mean()` of numpy and pandas.
pub fn mean(a: &[f64]) -> f64 {
    sum(a) / a.len() as f64
}

/// `a.std(ddof=ddof)` of numpy and pandas (NaN for an empty slice).
pub fn std(a: &[f64], ddof: usize) -> f64 {
    let n = a.len();
    let avg = sum(a) / n as f64;
    let sq: Vec<f64> = a.iter().map(|&x| (avg - x) * (avg - x)).collect();
    (sum(&sq) / (n as f64 - ddof as f64)).sqrt()
}

/// t of the mean: mean / (sd / sqrt n), sd with ddof 1 (NaN for fewer than 2 values).
pub fn t_mean(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 2 {
        return f64::NAN;
    }
    mean(a) / (std(a, 1) / (n as f64).sqrt())
}

/// Mean of `x` over the days where `mask` is true, and its t with a Newey-West (Bartlett) long-run
/// variance of the masked deviations over the whole series (it handles overlapping windows and
/// gaps). `lag` is the number of lags. Returns (mean, t, count).
pub fn nw_mean_t(x: &[f64], mask: &[bool], lag: usize) -> (f64, f64, usize) {
    let n_s = mask.iter().filter(|&&m| m).count();
    if n_s < 2 {
        return (f64::NAN, f64::NAN, n_s);
    }
    let t = x.len();
    let m = x.iter().zip(mask).filter(|(_, &k)| k).map(|(v, _)| v).sum::<f64>() / n_s as f64;
    let g: Vec<f64> = x.iter().zip(mask).map(|(v, &k)| if k { v - m } else { 0.0 }).collect();
    let gamma = |l: usize| (l..t).map(|i| g[i] * g[i - l]).sum::<f64>() / t as f64;
    let mut omega = gamma(0);
    for l in 1..=lag.min(t - 1) {
        omega += 2.0 * (1.0 - l as f64 / (lag as f64 + 1.0)) * gamma(l);
    }
    let var = omega * t as f64 / (n_s as f64 * n_s as f64);
    (m, if var > 0.0 { m / var.sqrt() } else { f64::NAN }, n_s)
}

/// Inverse of the normal CDF by bisection (p in (0,1)).
pub fn norm_inv(p: f64) -> f64 {
    let (mut lo, mut hi) = (-12.0_f64, 12.0_f64);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if super::pricing::norm_cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Skewness and kurtosis (not excess) of `a`, population moments.
pub fn skew_kurt(a: &[f64]) -> (f64, f64) {
    let n = a.len() as f64;
    let m = mean(a);
    let m2 = a.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n;
    let m3 = a.iter().map(|x| (x - m).powi(3)).sum::<f64>() / n;
    let m4 = a.iter().map(|x| (x - m).powi(4)).sum::<f64>() / n;
    (m3 / m2.powf(1.5), m4 / (m2 * m2))
}

/// Deflated Sharpe ratio (Bailey and Lopez de Prado, 2014). `sr` is the Sharpe ratio per period
/// (not annualised) over `t_obs` periods with the given skewness and kurtosis; `n_trials` is the
/// number of trials and `sr_var` the variance of the per-period Sharpe ratios of the trials.
/// Returns (SR0, the probability that the true Sharpe ratio is above SR0).
pub fn deflated_sharpe(sr: f64, t_obs: usize, skew: f64, kurt: f64, n_trials: usize, sr_var: f64) -> (f64, f64) {
    const EULER: f64 = 0.577_215_664_901_532_9;
    let sr0 = if n_trials <= 1 {
        0.0
    } else {
        let n = n_trials as f64;
        sr_var.sqrt() * ((1.0 - EULER) * norm_inv(1.0 - 1.0 / n) + EULER * norm_inv(1.0 - 1.0 / (n * std::f64::consts::E)))
    };
    let denom = (1.0 - skew * sr + (kurt - 1.0) / 4.0 * sr * sr).max(1e-12).sqrt();
    (sr0, super::pricing::norm_cdf((sr - sr0) * ((t_obs as f64 - 1.0).sqrt()) / denom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_statistics() {
        let a = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert_eq!(sum(&a), 40.0);
        assert_eq!(mean(&a), 5.0);
        assert_eq!(std(&a, 0), 2.0);
        assert!((std(&a, 1) - 2.138_089_935_299_395).abs() < 1e-12);
        let long: Vec<f64> = (0..1000).map(|x| x as f64).collect();
        assert_eq!(sum(&long), 499_500.0);
        assert!(std(&[], 0).is_nan());
    }
}

#[cfg(test)]
mod tests_alpha {
    use super::*;

    #[test]
    fn t_nw_and_deflated_sharpe() {
        let a = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((t_mean(&a) - 5.0 / (2.138_089_935_299_395 / 8f64.sqrt())).abs() < 1e-9);
        // With lag 0 and a full mask the NW t is the plain t with the population sd.
        let (m, t, n) = nw_mean_t(&a, &[true; 8], 0);
        assert!((m, n) == (5.0, 8) && (t - 5.0 / (2.0 / 8f64.sqrt())).abs() < 1e-9);
        assert!((norm_inv(0.975) - 1.959_963_985).abs() < 1e-6);
        // One trial: SR0 = 0. SR 0.1, T 101, normal: Phi(0.1 * 10 / sqrt(1.005)) = 0.8407.
        let (sr0, p) = deflated_sharpe(0.1, 101, 0.0, 3.0, 1, 0.01);
        assert_eq!(sr0, 0.0);
        assert!((p - 0.840_74).abs() < 1e-4, "{p}");
        // More trials raise SR0 and lower the probability.
        let (sr0b, pb) = deflated_sharpe(0.1, 101, 0.0, 3.0, 20, 0.01);
        assert!(sr0b > 0.1 && pb < p);
    }
}
