//! OLS, Newey-West variance, Clark-West test and expanding-window forecasts.

/// Newey-West lag of the design (rule 5.5).
pub const NW_LAG: usize = 8;

/// Upper tail of the standard normal.
pub fn norm_sf(t: f64) -> f64 {
    0.5 * libm::erfc(t / std::f64::consts::SQRT_2)
}

/// Inverse by Gauss-Jordan with partial pivoting. `None` when the matrix is singular.
pub fn invert(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = a
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut row = r.clone();
            row.extend((0..n).map(|j| f64::from(u8::from(i == j))));
            row
        })
        .collect();
    let scale = a.iter().map(|r| r.iter().fold(0.0_f64, |s, v| s.max(v.abs()))).fold(0.0_f64, f64::max);
    for c in 0..n {
        let p = (c..n).max_by(|&i, &j| m[i][c].abs().total_cmp(&m[j][c].abs()))?;
        if m[p][c].abs() <= 1e-12 * scale.max(1e-300) {
            return None;
        }
        m.swap(c, p);
        let d = m[c][c];
        for v in m[c].iter_mut() {
            *v /= d;
        }
        for r in 0..n {
            if r != c {
                let f = m[r][c];
                if f != 0.0 {
                    let pivot_row = m[c].clone();
                    for (v, pv) in m[r].iter_mut().zip(pivot_row) {
                        *v -= f * pv;
                    }
                }
            }
        }
    }
    Some(m.into_iter().map(|r| r[n..].to_vec()).collect())
}

fn col_scales(x: &[Vec<f64>]) -> Vec<f64> {
    let k = x[0].len();
    (0..k)
        .map(|j| {
            let s = (x.iter().map(|r| r[j] * r[j]).sum::<f64>() / x.len() as f64).sqrt();
            if s > 0.0 {
                s
            } else {
                1.0
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct Fit {
    pub beta: Vec<f64>,
    pub resid: Vec<f64>,
    pub r2: f64,
    /// (X'X)^-1 in the original units.
    pub xtx_inv: Vec<Vec<f64>>,
}

/// OLS. The first column of `x` must be the intercept (R2 is centred). Columns are scaled inside
/// for numerics only; the result is in original units.
pub fn ols(x: &[Vec<f64>], y: &[f64]) -> Option<Fit> {
    let (n, k) = (x.len(), x.first()?.len());
    if n <= k || y.len() != n {
        return None;
    }
    let s = col_scales(x);
    let mut a = vec![vec![0.0; k]; k];
    let mut b = vec![0.0; k];
    for (r, yy) in x.iter().zip(y) {
        for i in 0..k {
            let ri = r[i] / s[i];
            b[i] += ri * yy;
            for j in 0..k {
                a[i][j] += ri * r[j] / s[j];
            }
        }
    }
    let inv = invert(&a)?;
    let beta_s: Vec<f64> = (0..k).map(|i| (0..k).map(|j| inv[i][j] * b[j]).sum()).collect();
    let beta: Vec<f64> = (0..k).map(|i| beta_s[i] / s[i]).collect();
    let resid: Vec<f64> = x.iter().zip(y).map(|(r, yy)| yy - r.iter().zip(&beta).map(|(a, b)| a * b).sum::<f64>()).collect();
    let ybar = y.iter().sum::<f64>() / n as f64;
    let sst: f64 = y.iter().map(|v| (v - ybar).powi(2)).sum();
    let sse: f64 = resid.iter().map(|e| e * e).sum();
    let xtx_inv = (0..k).map(|i| (0..k).map(|j| inv[i][j] / (s[i] * s[j])).collect()).collect();
    Some(Fit { beta, resid, r2: if sst > 0.0 { 1.0 - sse / sst } else { f64::NAN }, xtx_inv })
}

/// Newey-West (Bartlett) standard errors of the coefficients.
pub fn nw_se(x: &[Vec<f64>], fit: &Fit, lag: usize) -> Vec<f64> {
    let (n, k) = (x.len(), x[0].len());
    let mut s = vec![vec![0.0; k]; k];
    for (xt, e) in x.iter().zip(&fit.resid) {
        let e2 = e * e;
        for i in 0..k {
            for j in 0..k {
                s[i][j] += e2 * xt[i] * xt[j];
            }
        }
    }
    for l in 1..=lag.min(n - 1) {
        let w = 1.0 - l as f64 / (lag as f64 + 1.0);
        for t in l..n {
            let e = fit.resid[t] * fit.resid[t - l];
            for i in 0..k {
                for j in 0..k {
                    s[i][j] += w * e * (x[t][i] * x[t - l][j] + x[t - l][i] * x[t][j]);
                }
            }
        }
    }
    let v = &fit.xtx_inv;
    (0..k)
        .map(|i| {
            let mut var = 0.0;
            for a in 0..k {
                for b in 0..k {
                    var += v[i][a] * s[a][b] * v[b][i];
                }
            }
            var.max(0.0).sqrt()
        })
        .collect()
}

/// Mean of `f`, and its t-value with a Newey-West variance (Bartlett weights 1 - j/(lag+1)).
/// Variance of the mean is gamma(lag-weighted) / T. Returns (mean, t); t is NaN when the variance is not positive.
pub fn nw_mean_t(f: &[f64], lag: usize) -> (f64, f64) {
    let n = f.len();
    let mean = f.iter().sum::<f64>() / n as f64;
    let d: Vec<f64> = f.iter().map(|v| v - mean).collect();
    let mut var = d.iter().map(|v| v * v).sum::<f64>() / n as f64;
    for l in 1..=lag.min(n - 1) {
        let g: f64 = (l..n).map(|t| d[t] * d[t - l]).sum::<f64>() / n as f64;
        var += 2.0 * (1.0 - l as f64 / (lag as f64 + 1.0)) * g;
    }
    let t = if var > 0.0 { mean / (var / n as f64).sqrt() } else { f64::NAN };
    (mean, t)
}

/// Clark-West statistic for the nested pair: M0 (small) against M1 (large). One-sided p-value
/// from the normal. f_t = e0^2 - (e1^2 - (p0 - p1)^2). A variance that is not positive gives p = 1.
pub fn clark_west(y: &[f64], p0: &[f64], p1: &[f64], lag: usize) -> (f64, f64) {
    let f: Vec<f64> = (0..y.len()).map(|t| (y[t] - p0[t]).powi(2) - ((y[t] - p1[t]).powi(2) - (p0[t] - p1[t]).powi(2))).collect();
    let (_, t) = nw_mean_t(&f, lag);
    if t.is_nan() {
        (f64::NAN, 1.0)
    } else {
        (t, norm_sf(t))
    }
}

/// Expanding-window predictions for rows `start..n`: the fit for row t uses rows `0..t` only.
pub fn expanding_preds(x: &[Vec<f64>], y: &[f64], start: usize) -> Option<Vec<f64>> {
    let (n, k) = (x.len(), x.first()?.len());
    if start <= k || start >= n {
        return None;
    }
    let s = col_scales(x); // scaling for numerics only; OLS predictions do not depend on it
    let xs: Vec<Vec<f64>> = x.iter().map(|r| r.iter().zip(&s).map(|(v, c)| v / c).collect()).collect();
    let mut a = vec![vec![0.0; k]; k];
    let mut b = vec![0.0; k];
    let add = |t: usize, a: &mut Vec<Vec<f64>>, b: &mut Vec<f64>| {
        for i in 0..k {
            b[i] += xs[t][i] * y[t];
            for j in 0..k {
                a[i][j] += xs[t][i] * xs[t][j];
            }
        }
    };
    for t in 0..start {
        add(t, &mut a, &mut b);
    }
    let mut out = Vec::with_capacity(n - start);
    #[allow(clippy::needless_range_loop)]
    for t in start..n {
        let inv = invert(&a)?;
        let beta: Vec<f64> = (0..k).map(|i| (0..k).map(|j| inv[i][j] * b[j]).sum()).collect();
        out.push(xs[t].iter().zip(&beta).map(|(a, b)| a * b).sum());
        add(t, &mut a, &mut b);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(x: &[f64]) -> Vec<Vec<f64>> {
        x.iter().map(|v| vec![1.0, *v]).collect()
    }

    /// x = 0,1,2 and y = 1,3,2. Means 1 and 2. Sxy = 1, Sxx = 2, slope 0.5, intercept 1.5.
    /// Fitted 1.5, 2, 2.5; residuals -0.5, 1, -0.5; SSE 1.5; SST 2; R2 = 0.25.
    #[test]
    fn ols_tiny_exact() {
        let f = ols(&rows(&[0.0, 1.0, 2.0]), &[1.0, 3.0, 2.0]).unwrap();
        assert!((f.beta[0] - 1.5).abs() < 1e-12 && (f.beta[1] - 0.5).abs() < 1e-12);
        assert!((f.r2 - 0.25).abs() < 1e-12);
        // (X'X)^-1 of x = 0,1,2: X'X = [[3,3],[3,5]], det 6, inverse [[5,-3],[-3,3]]/6.
        assert!((f.xtx_inv[0][0] - 5.0 / 6.0).abs() < 1e-12 && (f.xtx_inv[1][1] - 0.5).abs() < 1e-12);
        // A perfect line gives R2 = 1.
        let g = ols(&rows(&[0.0, 1.0, 2.0, 3.0]), &[1.0, 3.0, 5.0, 7.0]).unwrap();
        assert!((g.beta[1] - 2.0).abs() < 1e-12 && (g.r2 - 1.0).abs() < 1e-12);
        // Two equal columns: singular.
        assert!(ols(&[vec![1.0, 2.0, 2.0], vec![1.0, 3.0, 3.0], vec![1.0, 5.0, 5.0], vec![1.0, 7.0, 7.0]], &[1.0, 2.0, 3.0, 5.0]).is_none());
    }

    /// OLS slope standard error with no lags: V = (X'X)^-1 (sum e^2 x x') (X'X)^-1, lag 0.
    /// For x = 0,1,2 and y = 1,3,2: S = [[sum e2, sum e2 x],[., sum e2 x2]] with e2 = .25,1,.25:
    /// sum e2 = 1.5, sum e2 x = 0+1+0.5 = 1.5, sum e2 x2 = 0+1+1 = 2.
    /// Slope row of the inverse = (-3, 3)/6 = (-0.5, 0.5). Var = 0.25*1.5 - 2*0.25*1.5 + 0.25*2 = 0.125. se = sqrt(0.125).
    #[test]
    fn nw_se_lag_zero_is_white() {
        let x = rows(&[0.0, 1.0, 2.0]);
        let f = ols(&x, &[1.0, 3.0, 2.0]).unwrap();
        let se = nw_se(&x, &f, 0);
        assert!((se[1] - 0.125_f64.sqrt()).abs() < 1e-12, "{}", se[1]);
    }

    /// f = 1, 3. Mean 2, deviations -1, 1. gamma0 = 1, gamma1 = -1/2.
    /// Weight of lag 1 for lag 8 is 1 - 1/9 = 8/9. Var = 1 + 2 (8/9)(-1/2) = 1/9. t = 2 / sqrt((1/9)/2) = 2 sqrt(18).
    #[test]
    fn newey_west_mean_known() {
        let (m, t) = nw_mean_t(&[1.0, 3.0], 8);
        assert!((m - 2.0).abs() < 1e-12 && (t - 2.0 * 18.0_f64.sqrt()).abs() < 1e-9, "{t}");
    }

    /// y = 1, 2; M0 predicts 0, 0; M1 predicts 1, 2 (exact). e0^2 = 1, 4; e1^2 = 0, 0; (p0-p1)^2 = 1, 4.
    /// f = e0^2 - (e1^2 - (p0-p1)^2) = 2, 8. Mean 5, deviations -3, 3. gamma0 = 9, gamma1 = -4.5.
    /// Var = 9 + 2 (8/9)(-4.5) = 1. t = 5 / sqrt(1/2) = 5 sqrt 2 = 7.0711. p = normal upper tail of 7.07 (about 8e-13).
    #[test]
    fn clark_west_known_case() {
        let (t, p) = clark_west(&[1.0, 2.0], &[0.0, 0.0], &[1.0, 2.0], 8);
        assert!((t - 5.0 * 2.0_f64.sqrt()).abs() < 1e-9);
        assert!(p < 1e-10 && p > 0.0);
        // The normal tail: P(Z > 1.6448536) = 0.05.
        assert!((norm_sf(1.644_853_626_951_472_7) - 0.05).abs() < 1e-9);
    }

    /// With y = 1 + 2x exactly, each expanding fit is exact, so the predictions equal y after the start.
    /// Uses only earlier rows: poison row 5 (y changes) and predictions of rows 0..=5 must not change.
    #[test]
    fn expanding_uses_only_earlier_rows() {
        let xv: Vec<f64> = (0..8).map(|i| (i * i % 5) as f64 + i as f64).collect();
        let x = rows(&xv);
        let y: Vec<f64> = xv.iter().map(|v| 1.0 + 2.0 * v).collect();
        let p = expanding_preds(&x, &y, 3).unwrap();
        for (t, pr) in p.iter().enumerate() {
            assert!((pr - y[t + 3]).abs() < 1e-8);
        }
        let mut y2 = y.clone();
        y2[5] = 1.0e6;
        let p2 = expanding_preds(&x, &y2, 3).unwrap();
        for t in 3..=5 {
            assert!((p[t - 3] - p2[t - 3]).abs() < 1e-12);
        }
        assert!((p[3] - p2[3]).abs() > 1.0);
    }

    /// The design value of the Newey-West lag is 8 (rule 5; record A-161).
    #[test]
    fn nw_lag_is_the_design_value() {
        assert_eq!(NW_LAG, 8);
    }
}
