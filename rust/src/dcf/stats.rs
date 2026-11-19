//! Numbers: `np.percentile` (linear), `np.linalg.cholesky`, mean and std.

use super::DcfError;

/// `np.percentile(values, q)` with the default linear interpolation, `q` in [0, 100].
///
/// Same steps as numpy 2.x (`_quantile`): `q / 100`, virtual index `(n - 1) * q`,
/// `gamma = virtual - floor`, and the two-sided `_lerp` (it counts from the nearer end).
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    debug_assert!(n > 0);
    let quant = q / 100.0;
    let virt = (n as f64 - 1.0) * quant;
    if virt >= n as f64 - 1.0 {
        return sorted[n - 1];
    }
    if virt < 0.0 {
        return sorted[0];
    }
    let prev = virt.floor();
    let gamma = virt - prev;
    let a = sorted[prev as usize];
    let b = sorted[prev as usize + 1];
    let diff = b - a;
    if gamma >= 0.5 {
        b - diff * (1.0 - gamma)
    } else {
        a + diff * gamma
    }
}

/// Sorted copy, ascending (NaN last).
pub fn sorted_copy(values: &[f64]) -> Vec<f64> {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    v
}

/// Several percentiles of `values` (one sort).
pub fn percentiles(values: &[f64], qs: &[f64]) -> Vec<f64> {
    let s = sorted_copy(values);
    qs.iter().map(|q| percentile(&s, *q)).collect()
}

pub fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Population standard deviation (`ndarray.std()`, ddof 0).
pub fn std_pop(values: &[f64]) -> f64 {
    let m = mean(values);
    let ss: f64 = values.iter().map(|v| (v - m) * (v - m)).sum();
    (ss / values.len() as f64).sqrt()
}

/// The matrix is not positive definite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NotPositiveDefinite;

/// Lower-triangular Cholesky factor `L` with `L * L^T = a` (`np.linalg.cholesky`).
// Index loops: they are the textbook algorithm and are easier to check than iterator chains.
#[allow(clippy::needless_range_loop)]
pub fn cholesky(a: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, NotPositiveDefinite> {
    let k = a.len();
    let mut l = vec![vec![0.0; k]; k];
    for i in 0..k {
        for j in 0..=i {
            let mut s = a[i][j];
            for p in 0..j {
                s -= l[i][p] * l[j][p];
            }
            if i == j {
                if s <= 0.0 || s.is_nan() {
                    return Err(NotPositiveDefinite);
                }
                l[i][i] = s.sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    Ok(l)
}

/// Smallest eigenvalue of a symmetric matrix (Jacobi rotations). For error text only.
#[allow(clippy::needless_range_loop)]
pub fn min_eigenvalue(a: &[Vec<f64>]) -> f64 {
    let k = a.len();
    let mut m: Vec<Vec<f64>> = a.to_vec();
    for _ in 0..100 {
        let mut off = 0.0;
        for i in 0..k {
            for j in 0..k {
                if i != j {
                    off += m[i][j] * m[i][j];
                }
            }
        }
        if off < 1e-24 {
            break;
        }
        for p in 0..k {
            for q in (p + 1)..k {
                if m[p][q].abs() < 1e-300 {
                    continue;
                }
                let theta = (m[q][q] - m[p][p]) / (2.0 * m[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for r in 0..k {
                    let (mrp, mrq) = (m[r][p], m[r][q]);
                    m[r][p] = c * mrp - s * mrq;
                    m[r][q] = s * mrp + c * mrq;
                }
                for r in 0..k {
                    let (mpr, mqr) = (m[p][r], m[q][r]);
                    m[p][r] = c * mpr - s * mqr;
                    m[q][r] = s * mpr + c * mqr;
                }
            }
        }
    }
    (0..k).map(|i| m[i][i]).fold(f64::INFINITY, f64::min)
}

/// Share of `values` that are below `limit` (`np.mean(values < limit)`).
pub fn share_below(values: &[f64], limit: f64) -> f64 {
    values.iter().filter(|v| **v < limit).count() as f64 / values.len() as f64
}

pub(crate) fn err<T>(msg: impl Into<String>) -> Result<T, DcfError> {
    Err(DcfError(msg.into()))
}
