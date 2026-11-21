//! Option prices from a characteristic function.
//!
//! - `lewis_j`: the Lewis (2001) integral with adaptive Gauss-Legendre panels. All strikes of one expiry share the
//!   values of the characteristic function (one grid of nodes).
//!   `C / df = F - sqrt(F K) J` and `P / df = K - sqrt(F K) J`, with
//!   `J(k) = (1/pi) Int_0^inf Re[exp(i u k) phi(u - i/2)] / (u^2 + 1/4) du` and `k = ln(F / K)`.
//! - `carr_madan_calls`: the Carr-Madan (1999) FFT, an independent check.

use super::base::CharModel;
use super::cx::Cx;
use std::f64::consts::PI;
use std::sync::OnceLock;

/// Gauss-Legendre nodes and weights on [-1, 1] (Newton on the Legendre polynomial).
pub fn gauss_legendre(n: usize) -> (Vec<f64>, Vec<f64>) {
    let (mut x, mut w) = (vec![0.0; n], vec![0.0; n]);
    for i in 0..n {
        let mut z = (PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
        let mut dp = 1.0;
        for _ in 0..100 {
            let (mut p0, mut p1) = (1.0, z);
            for k in 2..=n {
                let p2 = ((2 * k - 1) as f64 * z * p1 - (k - 1) as f64 * p0) / k as f64;
                p0 = p1;
                p1 = p2;
            }
            dp = n as f64 * (z * p1 - p0) / (z * z - 1.0);
            let dz = p1 / dp;
            z -= dz;
            if dz.abs() < 1e-16 {
                break;
            }
        }
        x[i] = z;
        w[i] = 2.0 / ((1.0 - z * z) * dp * dp);
    }
    (x, w)
}

fn gl16() -> &'static (Vec<f64>, Vec<f64>) {
    static GL: OnceLock<(Vec<f64>, Vec<f64>)> = OnceLock::new();
    GL.get_or_init(|| gauss_legendre(16))
}

/// Error limit of one panel (for J; the price error is at most `sqrt(F K)` times this) and of the tail.
#[derive(Debug, Clone, Copy)]
pub struct Accuracy {
    pub panel: f64,
    pub tail: f64,
}

impl Accuracy {
    /// For prices and the fixed-answer tests.
    pub const TIGHT: Accuracy = Accuracy { panel: 1e-13, tail: 1e-16 };
    /// For the residuals of a fit: a price error of 1e-9 of the forward is far below any spread.
    pub const FIT: Accuracy = Accuracy { panel: 1e-9, tail: 1e-12 };
}

const MAX_DEPTH: usize = 10;
const U_MAX: f64 = 20_000.0;

/// The values of the integrand without the oscillation: `(u, w phi(u - i/2) / (u^2 + 1/4))` for the nodes of a panel.
fn panel_nodes<M: CharModel>(m: &M, t: f64, a: f64, b: f64) -> Vec<(f64, Cx)> {
    let (x, w) = gl16();
    let (h, mid) = (0.5 * (b - a), 0.5 * (a + b));
    (0..x.len())
        .map(|i| {
            let u = mid + h * x[i];
            let g = m.log_cf(Cx::new(u, -0.5), t) * (w[i] * h / (u * u + 0.25));
            (u, g)
        })
        .collect()
}

fn node_sum(nodes: &[(f64, Cx)], k: f64) -> f64 {
    nodes
        .iter()
        .map(|(u, g)| {
            let (s, c) = (u * k).sin_cos();
            c * g.re - s * g.im
        })
        .sum()
}

/// `J(k)` for each `k = ln(F / K)`, by the Lewis integral with adaptive Gauss-Legendre.
pub fn lewis_j<M: CharModel>(m: &M, t: f64, ks: &[f64]) -> Vec<f64> {
    lewis_j_acc(m, t, ks, Accuracy::TIGHT)
}

/// `lewis_j` with a chosen accuracy.
pub fn lewis_j_acc<M: CharModel>(m: &M, t: f64, ks: &[f64], acc: Accuracy) -> Vec<f64> {
    let mut out = vec![0.0; ks.len()];
    if ks.is_empty() {
        return out;
    }
    // The check of a panel uses the strikes that oscillate most (the two extreme k) and the one nearest to 0.
    let mut sentinels: Vec<f64> = vec![ks.iter().cloned().fold(f64::INFINITY, f64::min), ks.iter().cloned().fold(f64::NEG_INFINITY, f64::max)];
    sentinels.push(ks.iter().cloned().min_by(|a, b| a.abs().total_cmp(&b.abs())).unwrap_or(0.0));
    let kmax = sentinels.iter().fold(0.0f64, |a, k| a.max(k.abs()));
    let (mut a, mut width): (f64, f64) = (0.0, 1.0);
    while a < U_MAX {
        // A panel resolves the oscillation: at most about 12 radians over a panel.
        let w = width.min(12.0 / kmax.max(1e-9));
        let b = a + w;
        let mag = adapt(m, t, a, b, ks, &sentinels, acc.panel, 0, &mut out);
        a = b;
        if mag.is_nan() {
            break;
        }
        if a > 2.0 && mag < acc.tail {
            break;
        }
        width = (width * 1.15).min(50.0);
    }
    out.iter().map(|v| v / PI).collect()
}

/// Add the integral over [a, b] to `out`. Returns the largest size of the integrand times the panel width.
#[allow(clippy::too_many_arguments)]
fn adapt<M: CharModel>(m: &M, t: f64, a: f64, b: f64, ks: &[f64], sentinels: &[f64], tol: f64, depth: usize, out: &mut [f64]) -> f64 {
    let whole = panel_nodes(m, t, a, b);
    if whole.iter().any(|(_, g)| !(g.re.is_finite() && g.im.is_finite())) {
        // A non-finite characteristic function: the result is NaN, and the caller stops on it.
        out.iter_mut().for_each(|o| *o = f64::NAN);
        return f64::NAN;
    }
    let mag = whole.iter().map(|(_, g)| g.abs()).fold(0.0, f64::max);
    let mid = 0.5 * (a + b);
    let (left, right) = (panel_nodes(m, t, a, mid), panel_nodes(m, t, mid, b));
    let err = sentinels.iter().map(|&k| (node_sum(&whole, k) - node_sum(&left, k) - node_sum(&right, k)).abs()).fold(0.0, f64::max);
    if err <= tol || depth >= MAX_DEPTH {
        // The two halves are the better estimate and are already computed.
        for (o, &k) in out.iter_mut().zip(ks) {
            *o += node_sum(&left, k) + node_sum(&right, k);
        }
        return mag;
    }
    let m1 = adapt(m, t, a, mid, ks, sentinels, tol, depth + 1, out);
    let m2 = adapt(m, t, mid, b, ks, sentinels, tol, depth + 1, out);
    if m1.is_nan() || m2.is_nan() {
        f64::NAN
    } else {
        m1.max(m2)
    }
}

/// Call price (without `df`) of strike `K`: `F - sqrt(F K) J`.
pub fn call_over_df<M: CharModel>(m: &M, forward: f64, strike: f64, t: f64) -> f64 {
    let j = lewis_j(m, t, &[(forward / strike).ln()])[0];
    forward - (forward * strike).sqrt() * j
}

/// Put price (without `df`) of strike `K`: `K - sqrt(F K) J`.
pub fn put_over_df<M: CharModel>(m: &M, forward: f64, strike: f64, t: f64) -> f64 {
    let j = lewis_j(m, t, &[(forward / strike).ln()])[0];
    strike - (forward * strike).sqrt() * j
}

/// In-place radix-2 FFT (forward, `exp(-2 pi i j m / n)`). `x.len()` is a power of 2.
pub fn fft(x: &mut [Cx]) {
    let n = x.len();
    assert!(n.is_power_of_two());
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            x.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f64;
        let wl = Cx::new(ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let mut w = Cx::ONE;
            for k in 0..len / 2 {
                let (u, v) = (x[start + k], x[start + k + len / 2] * w);
                x[start + k] = u + v;
                x[start + k + len / 2] = u - v;
                w = w * wl;
            }
        }
        len <<= 1;
    }
}

/// Call prices (without `df`) by the Carr-Madan FFT, Simpson weights, damping `alpha`, cubic interpolation in the log strike.
/// Needs the moment of order `alpha + 1` of `F_T / F` to be finite.
pub fn carr_madan_calls<M: CharModel>(m: &M, forward: f64, strikes: &[f64], t: f64, alpha: f64, n: usize, eta: f64) -> Vec<f64> {
    let lambda = 2.0 * PI / (n as f64 * eta);
    let b = 0.5 * n as f64 * lambda;
    let mut x: Vec<Cx> = (0..n)
        .map(|j| {
            let v = j as f64 * eta;
            let psi = m.log_cf(Cx::new(v, -(alpha + 1.0)), t) / Cx::new(alpha * alpha + alpha - v * v, (2.0 * alpha + 1.0) * v);
            let simpson = (3.0 + if j % 2 == 0 { -1.0 } else { 1.0 } - if j == 0 { 1.0 } else { 0.0 }) / 3.0;
            psi * Cx::new((v * b).cos(), (v * b).sin()) * (eta * simpson)
        })
        .collect();
    fft(&mut x);
    // c(s_m) for s_m = -b + m lambda, s = ln(K / F).
    let c: Vec<f64> = (0..n).map(|mm| (-alpha * (-b + mm as f64 * lambda)).exp() / PI * x[mm].re).collect();
    strikes
        .iter()
        .map(|&k| {
            let s = (k / forward).ln();
            let pos = (s + b) / lambda;
            let i0 = (pos.floor() as isize - 1).clamp(0, n as isize - 4) as usize;
            let f = pos - i0 as f64;
            // Lagrange on the four nodes i0 .. i0 + 3 at 0, 1, 2, 3.
            let l = [-(f - 1.0) * (f - 2.0) * (f - 3.0) / 6.0, f * (f - 2.0) * (f - 3.0) / 2.0, -f * (f - 1.0) * (f - 3.0) / 2.0, f * (f - 1.0) * (f - 2.0) / 6.0];
            forward * (0..4).map(|i| l[i] * c[i0 + i]).sum::<f64>()
        })
        .collect()
}
