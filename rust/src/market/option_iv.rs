//! Implied volatility from option closes, and the forward from put-call parity.
//!
//! A first version, with known errors (`docs/HIDDEN-ASSUMPTIONS.md` #236):
//!
//! - The model is Black-76 (Black-Scholes with the forward `F`): `C = df * (F N(d1) - K N(d2))`,
//!   `P = df * (K N(-d2) - F N(-d1))`, `d1 = (ln(F/K) + v^2 T / 2) / (v sqrt(T))`, `df = exp(-r T)`.
//!   It is a European model. SPY and single-stock options are American: the early-exercise
//!   premium and the dividend make an error in the implied volatility of an in-the-money option, so
//!   the builder gives a volatility for the out-of-the-money options and the options near the
//!   money only.
//! - The forward comes from put-call parity, `F = K + (C - P) / df`, at the strikes nearest to the spot where
//!   a call and a put both have a close. The parity of American options is only an inequality: the
//!   forward is an estimate. Without a pair the forward is `spot / df` (no dividend), and the
//!   caller marks it.
//! - The prices are closes of the day: the last trade of the day, not a mid of a bid and an ask. A
//!   contract that did not trade that day has no close. A close is not a price at 16:00: a thin contract
//!   traded at any time of the day while the underlying moved.

use crate::trading::pricing::{intrinsic, norm_cdf};

pub const MIN_VOL: f64 = 1e-4;
pub const MAX_VOL: f64 = 5.0;

/// The Black-76 price. `df` is the discount factor to the expiry.
pub fn black76(call: bool, forward: f64, strike: f64, t: f64, df: f64, vol: f64) -> f64 {
    if t <= 0.0 || vol <= 0.0 {
        return df * intrinsic(call, forward, strike);
    }
    let sq = vol * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * sq * sq) / sq;
    let d2 = d1 - sq;
    if call {
        df * (forward * norm_cdf(d1) - strike * norm_cdf(d2))
    } else {
        df * (strike * norm_cdf(-d2) - forward * norm_cdf(-d1))
    }
}

/// The volatility that gives `price`, or `None` when no volatility does: a price at or below the
/// discounted intrinsic value, or above the upper bound of the option.
pub fn implied_vol(call: bool, price: f64, forward: f64, strike: f64, t: f64, df: f64) -> Option<f64> {
    if !(price.is_finite() && forward > 0.0 && strike > 0.0 && t > 0.0 && df > 0.0) {
        return None;
    }
    let lower = df * intrinsic(call, forward, strike);
    let upper = if call { df * forward } else { df * strike };
    if price <= lower + 1e-12 || price >= upper {
        return None;
    }
    let (mut lo, mut hi) = (MIN_VOL, MAX_VOL);
    if black76(call, forward, strike, t, df, hi) < price || black76(call, forward, strike, t, df, lo) > price {
        return None;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        let p = black76(call, forward, strike, t, df, mid);
        if p < price {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-12 {
            break;
        }
    }
    Some(0.5 * (lo + hi))
}

/// How many strikes nearest to the spot give a forward (the median of them is the forward).
pub const PARITY_STRIKES: usize = 3;

/// The forward of one expiry: the median of `K + (C - P) / df` over the (at most `PARITY_STRIKES`) strikes
/// nearest to `spot` that have both closes. `pairs` are `(strike, call close, put close)`. The result must
/// be within a factor 2 of the spot, else it is not used (`None`).
pub fn forward_from_parity(pairs: &[(f64, f64, f64)], spot: f64, df: f64) -> Option<f64> {
    if !(spot > 0.0 && df > 0.0) {
        return None;
    }
    let mut near: Vec<&(f64, f64, f64)> = pairs.iter().filter(|p| p.0 > 0.0 && p.1 > 0.0 && p.2 > 0.0).collect();
    near.sort_by(|a, b| (a.0 - spot).abs().total_cmp(&(b.0 - spot).abs()));
    near.truncate(PARITY_STRIKES);
    let mut f: Vec<f64> = near.iter().map(|(k, c, p)| k + (c - p) / df).collect();
    if f.is_empty() {
        return None;
    }
    f.sort_by(f64::total_cmp);
    let median = if f.len() % 2 == 1 { f[f.len() / 2] } else { 0.5 * (f[f.len() / 2 - 1] + f[f.len() / 2]) };
    (median > 0.5 * spot && median < 2.0 * spot).then_some(median)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_a_known_volatility() {
        let (f, t, df) = (100.0, 0.5, (-0.04f64 * 0.5).exp());
        for &vol in &[0.08, 0.2, 0.55, 1.5] {
            for &k in &[70.0, 95.0, 100.0, 108.0, 140.0] {
                for call in [true, false] {
                    let p = black76(call, f, k, t, df, vol);
                    if p > df * intrinsic(call, f, k) + 1e-9 {
                        let iv = implied_vol(call, p, f, k, t, df).expect("solvable");
                        assert!((iv - vol).abs() < 1e-6, "call={call} k={k} vol={vol} got {iv}");
                    }
                }
            }
        }
    }

    #[test]
    fn prices_with_no_volatility_give_none() {
        let (f, t, df) = (100.0, 0.25, 0.99);
        assert_eq!(implied_vol(true, 0.5 * df * (f - 80.0), f, 80.0, t, df), None, "below intrinsic");
        assert_eq!(implied_vol(true, 150.0, f, 100.0, t, df), None, "above the bound");
        assert_eq!(implied_vol(true, 5.0, f, 100.0, 0.0, df), None, "no time");
        assert_eq!(implied_vol(true, f64::NAN, f, 100.0, t, df), None);
    }

    #[test]
    fn put_call_parity_forward() {
        // A forward of 101.3 with r = 3%, T = 0.25, a flat 22% volatility.
        let (spot, f, t, df) = (100.0, 101.3, 0.25, (-0.03f64 * 0.25).exp());
        let pairs: Vec<(f64, f64, f64)> = [90.0, 95.0, 99.0, 100.0, 101.0, 105.0, 120.0].iter().map(|&k| (k, black76(true, f, k, t, df, 0.22), black76(false, f, k, t, df, 0.22))).collect();
        let got = forward_from_parity(&pairs, spot, df).unwrap();
        assert!((got - f).abs() < 1e-9, "{got}");
        assert_eq!(forward_from_parity(&[(100.0, 3.0, 0.0)], spot, df), None, "a pair with no put close");
        assert_eq!(forward_from_parity(&[(100.0, 150.0, 1.0)], spot, df), None, "an absurd forward is not used");
        // The median ignores one bad pair among three.
        let mut bad = pairs.clone();
        bad.sort_by(|a, b| (a.0 - spot).abs().total_cmp(&(b.0 - spot).abs()));
        bad[0].1 += 0.8;
        let got = forward_from_parity(&bad, spot, df).unwrap();
        assert!((got - f).abs() < 1e-9, "median of three: {got}");
    }
}
