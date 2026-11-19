//! Small numeric tools: log-gamma, Nelder-Mead, numeric Hessian, matrix inverse, OLS.

use rayon::prelude::*;

/// ln Gamma(x) for x > 0. Lanczos approximation (g = 7, 9 terms), error near 1e-15.
pub fn lgamma(x: f64) -> f64 {
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - lgamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = G[0];
    let t = x + 7.5;
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Sum of `f(i)` for i in 0..n. The chunks are fixed, so the result does not depend on the thread count.
pub fn chunked_sum(n: usize, f: impl Fn(usize) -> f64 + Sync) -> f64 {
    const CHUNK: usize = 1024;
    let chunks = n.div_ceil(CHUNK);
    let parts: Vec<f64> = (0..chunks)
        .into_par_iter()
        .map(|c| (c * CHUNK..((c + 1) * CHUNK).min(n)).map(&f).sum::<f64>())
        .collect();
    parts.iter().sum()
}

/// Minimise `f` with the Nelder-Mead simplex method. Restarts from the best point
/// until a restart gives no gain. Gives (x, f(x), converged).
pub fn nelder_mead(f: &dyn Fn(&[f64]) -> f64, x0: &[f64], step: f64, ftol: f64, max_iter: usize) -> (Vec<f64>, f64, bool) {
    let n = x0.len();
    let mut best = x0.to_vec();
    let mut best_f = f(&best);
    let mut converged = false;
    for _ in 0..4 {
        let mut simplex: Vec<(Vec<f64>, f64)> = vec![(best.clone(), best_f)];
        for i in 0..n {
            let mut p = best.clone();
            p[i] += if p[i].abs() > 1e-8 { step * p[i].abs().max(0.1) } else { step };
            let v = f(&p);
            simplex.push((p, v));
        }
        let mut iter = 0;
        loop {
            simplex.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let (lo, hi) = (simplex[0].1, simplex[n].1);
            let size = simplex[1..]
                .iter()
                .map(|s| s.0.iter().zip(&simplex[0].0).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max))
                .fold(0.0, f64::max);
            if (hi - lo).abs() <= ftol * (1.0 + lo.abs()) && size < 1e-7 {
                converged = true;
                break;
            }
            iter += 1;
            if iter > max_iter {
                converged = false;
                break;
            }
            let mut centroid = vec![0.0; n];
            for s in &simplex[..n] {
                for (c, x) in centroid.iter_mut().zip(&s.0) {
                    *c += x / n as f64;
                }
            }
            let point = |t: f64| -> Vec<f64> { centroid.iter().zip(&simplex[n].0).map(|(c, w)| c + t * (c - w)).collect() };
            let xr = point(1.0);
            let fr = f(&xr);
            if fr < simplex[0].1 {
                let xe = point(2.0);
                let fe = f(&xe);
                simplex[n] = if fe < fr { (xe, fe) } else { (xr, fr) };
            } else if fr < simplex[n - 1].1 {
                simplex[n] = (xr, fr);
            } else {
                let (xc, fc) = if fr < simplex[n].1 {
                    let x = point(0.5);
                    let v = f(&x);
                    (x, v)
                } else {
                    let x = point(-0.5);
                    let v = f(&x);
                    (x, v)
                };
                if fc < fr.min(simplex[n].1) {
                    simplex[n] = (xc, fc);
                } else {
                    let b = simplex[0].0.clone();
                    for s in simplex.iter_mut().skip(1) {
                        for (x, bb) in s.0.iter_mut().zip(&b) {
                            *x = bb + 0.5 * (*x - bb);
                        }
                        s.1 = f(&s.0);
                    }
                }
            }
        }
        simplex.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let gain = best_f - simplex[0].1;
        if simplex[0].1 <= best_f {
            best = simplex[0].0.clone();
            best_f = simplex[0].1;
        }
        if gain.abs() <= ftol * (1.0 + best_f.abs()) && converged {
            break;
        }
    }
    (best, best_f, converged)
}

/// Invert a square matrix (Gauss-Jordan, partial pivoting). None if singular.
pub fn invert(m: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = m.len();
    let mut a: Vec<Vec<f64>> = m
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut r = row.clone();
            r.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
            r
        })
        .collect();
    for c in 0..n {
        let p = (c..n).max_by(|&i, &j| a[i][c].abs().partial_cmp(&a[j][c].abs()).unwrap_or(std::cmp::Ordering::Equal))?;
        if a[p][c].abs() < 1e-300 || !a[p][c].is_finite() {
            return None;
        }
        a.swap(c, p);
        let d = a[c][c];
        for v in a[c].iter_mut() {
            *v /= d;
        }
        for r in 0..n {
            if r != c {
                let f = a[r][c];
                if f != 0.0 {
                    let pivot_row = a[c].clone();
                    for (v, pv) in a[r].iter_mut().zip(&pivot_row) {
                        *v -= f * pv;
                    }
                }
            }
        }
    }
    Some(a.into_iter().map(|r| r[n..].to_vec()).collect())
}

/// Central-difference Hessian of `f` at `x`. The step of each coordinate is `rel` times its size.
pub fn hessian(f: &dyn Fn(&[f64]) -> f64, x: &[f64], rel: f64) -> Vec<Vec<f64>> {
    let n = x.len();
    let h: Vec<f64> = x.iter().map(|v| rel * v.abs().max(1e-6)).collect();
    let f0 = f(x);
    let at = |di: &[(usize, f64)]| -> f64 {
        let mut p = x.to_vec();
        for &(i, s) in di {
            p[i] += s * h[i];
        }
        f(&p)
    };
    let mut hs = vec![vec![0.0; n]; n];
    for i in 0..n {
        hs[i][i] = (at(&[(i, 1.0)]) - 2.0 * f0 + at(&[(i, -1.0)])) / (h[i] * h[i]);
        for j in i + 1..n {
            let v = (at(&[(i, 1.0), (j, 1.0)]) - at(&[(i, 1.0), (j, -1.0)]) - at(&[(i, -1.0), (j, 1.0)])
                + at(&[(i, -1.0), (j, -1.0)]))
                / (4.0 * h[i] * h[j]);
            hs[i][j] = v;
            hs[j][i] = v;
        }
    }
    hs
}

/// Covariance matrix = inverse of the negative Hessian of a log-likelihood.
/// None if the matrix is not positive definite (a flat or boundary maximum).
pub fn covariance_from_loglik(f: &dyn Fn(&[f64]) -> f64, x: &[f64]) -> Option<Vec<Vec<f64>>> {
    let h = hessian(f, x, 1e-3);
    let neg: Vec<Vec<f64>> = h.iter().map(|r| r.iter().map(|v| -v).collect()).collect();
    let inv = invert(&neg)?;
    if inv.iter().enumerate().all(|(i, r)| r[i].is_finite() && r[i] > 0.0) {
        Some(inv)
    } else {
        None
    }
}

/// Standard errors of g(phi) from the covariance of phi (delta method, numeric Jacobian).
pub fn delta_se(g: &dyn Fn(&[f64]) -> Vec<f64>, phi: &[f64], cov: &[Vec<f64>]) -> Vec<f64> {
    let base = g(phi);
    let (m, n) = (base.len(), phi.len());
    let mut jac = vec![vec![0.0; n]; m];
    for j in 0..n {
        let h = 1e-6 * phi[j].abs().max(1e-6);
        let (mut p, mut q) = (phi.to_vec(), phi.to_vec());
        p[j] += h;
        q[j] -= h;
        let (gp, gq) = (g(&p), g(&q));
        for i in 0..m {
            jac[i][j] = (gp[i] - gq[i]) / (2.0 * h);
        }
    }
    (0..m)
        .map(|i| {
            let mut v = 0.0;
            for a in 0..n {
                for b in 0..n {
                    v += jac[i][a] * cov[a][b] * jac[i][b];
                }
            }
            v.max(0.0).sqrt()
        })
        .collect()
}

/// Result of the regression y = alpha + beta x + e, with heteroskedasticity-robust (HC0) covariance.
#[derive(Debug, Clone)]
pub struct Ols {
    pub alpha: f64,
    pub beta: f64,
    pub residuals: Vec<f64>,
    pub rss: f64,
    /// Covariance of (alpha, beta).
    pub cov: [[f64; 2]; 2],
}

/// None if x is constant.
pub fn ols(x: &[f64], y: &[f64]) -> Option<Ols> {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let sxx: f64 = x.iter().map(|v| (v - mx) * (v - mx)).sum();
    if sxx <= 1e-300 || !sxx.is_finite() {
        return None;
    }
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let beta = sxy / sxx;
    let alpha = my - beta * mx;
    let residuals: Vec<f64> = x.iter().zip(y).map(|(a, b)| b - alpha - beta * a).collect();
    let rss = residuals.iter().map(|e| e * e).sum();
    // (X'X)^-1 with X = [1, x]
    let sx: f64 = x.iter().sum();
    let sx2: f64 = x.iter().map(|v| v * v).sum();
    let det = n * sx2 - sx * sx;
    let inv = [[sx2 / det, -sx / det], [-sx / det, n / det]];
    let mut meat = [[0.0; 2]; 2];
    for (xi, e) in x.iter().zip(&residuals) {
        let (e2, v) = (e * e, [1.0, *xi]);
        for a in 0..2 {
            for b in 0..2 {
                meat[a][b] += e2 * v[a] * v[b];
            }
        }
    }
    let mut cov = [[0.0; 2]; 2];
    for a in 0..2 {
        for b in 0..2 {
            for c in 0..2 {
                for d in 0..2 {
                    cov[a][b] += inv[a][c] * meat[c][d] * inv[d][b];
                }
            }
        }
    }
    Some(Ols { alpha, beta, residuals, rss, cov })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lgamma_known_values() {
        assert!((lgamma(1.0)).abs() < 1e-13);
        assert!((lgamma(0.5) - 0.5 * std::f64::consts::PI.ln()).abs() < 1e-13);
        assert!((lgamma(10.0) - 362_880f64.ln()).abs() < 1e-12);
        assert!((lgamma(0.1) - 2.252_712_651_734_206).abs() < 1e-12);
        assert!((lgamma(100.0) - 359.134_205_369_575_4).abs() < 1e-10);
    }

    #[test]
    fn nelder_mead_finds_rosenbrock_minimum() {
        let f = |x: &[f64]| (1.0 - x[0]).powi(2) + 100.0 * (x[1] - x[0] * x[0]).powi(2);
        let (x, fx, ok) = nelder_mead(&f, &[-1.2, 1.0], 0.1, 1e-14, 5000);
        assert!(ok && fx < 1e-9 && (x[0] - 1.0).abs() < 1e-3 && (x[1] - 1.0).abs() < 1e-3, "{x:?} {fx}");
    }

    #[test]
    fn invert_and_hessian() {
        let m = vec![vec![4.0, 1.0], vec![1.0, 3.0]];
        let inv = invert(&m).unwrap();
        assert!((inv[0][0] - 3.0 / 11.0).abs() < 1e-14 && (inv[0][1] + 1.0 / 11.0).abs() < 1e-14);
        assert!(invert(&[vec![1.0, 2.0], vec![2.0, 4.0]]).is_none());
        let f = |x: &[f64]| -(2.0 * x[0] * x[0] + x[0] * x[1] + 3.0 * x[1] * x[1]);
        let h = hessian(&f, &[0.7, 0.3], 1e-3);
        assert!((h[0][0] + 4.0).abs() < 1e-5 && (h[0][1] + 1.0).abs() < 1e-5 && (h[1][1] + 6.0).abs() < 1e-5);
    }

    #[test]
    fn ols_exact_line_and_constant_x() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| 2.0 + 0.5 * v).collect();
        let o = ols(&x, &y).unwrap();
        assert!((o.alpha - 2.0).abs() < 1e-12 && (o.beta - 0.5).abs() < 1e-12 && o.rss < 1e-20);
        assert!(ols(&[1.0; 5], &[1.0, 2.0, 3.0, 4.0, 5.0]).is_none());
    }
}
