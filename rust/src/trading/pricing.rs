//! Option pricing. The pricer is a MODEL, not market quotes (same as the Python pricer).
//!
//! - ATM vol from the vol index (VIX for SPY, VXN for QQQ), VIX1D for one trading day or
//!   less, linear in time between them out to 30 trading days.
//! - Skew: IV multiplier quadratic in the standardized log-moneyness (puts richer).
//! - Spread: half-spread = clip(0.5% of mid, $0.01, $0.05) for each leg. Long legs pay the
//!   ask, short legs receive the bid. The engine charges the commission.
//! - Time to expiry is a variance clock (`year_fraction`): overnight decay and a U-shaped
//!   intraday variance profile, so one close-to-close day is 1/252 of a year.
//! - European Black-Scholes, settled at intrinsic on expiry.
//!
//! Every floating-point operation keeps the order of the Python code, so the results are
//! the same to the last digits (`erf` comes from the `libm` crate; Python uses the C library).

use super::calendar::{busday_count, Date};

pub const SESSION_MINUTES: f64 = 390.0;
pub const TRADING_DAYS: f64 = 252.0;
const MINUTES_PER_YEAR: f64 = 252.0 * 390.0; // floor for a tiny T only
const OVERNIGHT_WEIGHT: f64 = 0.25;
const OPEN_W: f64 = 2.0;
const CLOSE_W: f64 = 1.5;
const EDGE: f64 = 30.0;
const VARIANCE_UNITS_PER_YEAR: f64 = 252.0 * 390.0 * (1.0 + OVERNIGHT_WEIGHT);

/// Minute of the day of the session close (16:00).
pub const CLOSE_MINUTE: i32 = 16 * 60;

pub fn norm_cdf(x: f64) -> f64 {
    0.5 * (1.0 + libm::erf(x / std::f64::consts::SQRT_2))
}

pub fn intrinsic(call: bool, spot: f64, strike: f64) -> f64 {
    if call {
        (spot - strike).max(0.0)
    } else {
        (strike - spot).max(0.0)
    }
}

pub fn bs_price(call: bool, spot: f64, strike: f64, t: f64, rate: f64, vol: f64) -> f64 {
    if t <= 0.0 || vol <= 0.0 {
        return intrinsic(call, spot, strike);
    }
    let sq = vol * t.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * t) / sq;
    let d2 = d1 - sq;
    let disc = (-rate * t).exp();
    if call {
        spot * norm_cdf(d1) - strike * disc * norm_cdf(d2)
    } else {
        strike * disc * norm_cdf(-d2) - spot * norm_cdf(-d1)
    }
}

pub fn bs_delta(call: bool, spot: f64, strike: f64, t: f64, rate: f64, vol: f64) -> f64 {
    if t <= 0.0 || vol <= 0.0 {
        let itm = if call { spot > strike } else { spot < strike };
        return if itm {
            if call { 1.0 } else { -1.0 }
        } else {
            0.0
        };
    }
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * t) / (vol * t.sqrt());
    if call {
        norm_cdf(d1)
    } else {
        norm_cdf(d1) - 1.0
    }
}

/// Variance units from the open to `x` minutes into the session.
fn cum_weight(x: f64) -> f64 {
    let mid_w = (SESSION_MINUTES - EDGE * (OPEN_W + CLOSE_W)) / (SESSION_MINUTES - 2.0 * EDGE);
    let x = x.max(0.0).min(SESSION_MINUTES);
    let late = SESSION_MINUTES - EDGE;
    OPEN_W * x.min(EDGE) + mid_w * (x - EDGE).max(0.0).min(late - EDGE) + CLOSE_W * (x - late).max(0.0)
}

/// Variance-time years from `now` (date and minute of the day) to the 16:00 close on
/// `expiry`. It counts the U-shaped rest of today's session, one overnight for each session
/// boundary, and full later sessions. A full close-to-close day is exactly 1/252.
pub fn year_fraction(today: Date, minute: i32, expiry: Date) -> f64 {
    if expiry < today {
        return 0.0;
    }
    let mins_left = ((CLOSE_MINUTE - minute) as f64).max(0.0).min(SESSION_MINUTES);
    let mut units = SESSION_MINUTES - cum_weight(SESSION_MINUTES - mins_left);
    if expiry > today {
        let sessions_after = busday_count(today + 1, expiry + 1);
        units += (sessions_after as f64) * SESSION_MINUTES * (1.0 + OVERNIGHT_WEIGHT);
    }
    units / VARIANCE_UNITS_PER_YEAR
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolSnapshot {
    /// 30-day ATM IV (decimal).
    pub atm: f64,
    /// About 1-day ATM IV (decimal).
    pub short: f64,
    pub rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quote {
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub iv: f64,
    pub delta: f64,
}

/// The model pricer with its default parameters.
pub struct SyntheticPricer;

const SKEW_SLOPE: f64 = -0.12; // IV change for each unit of standardized moneyness (puts richer)
const SKEW_CURVE: f64 = 0.03; // smile curvature
const MULT_FLOOR: f64 = 0.6;
const MULT_CAP: f64 = 2.5;
const SPREAD_REL: f64 = 0.005;
const SPREAD_MIN: f64 = 0.01;
const SPREAD_MAX: f64 = 0.05;

impl SyntheticPricer {
    pub fn atm_vol(t: f64, vol: &VolSnapshot) -> f64 {
        let t1 = 1.0 / TRADING_DAYS;
        let t30 = 30.0 / TRADING_DAYS;
        if t <= t1 {
            return vol.short;
        }
        if t >= t30 {
            return vol.atm;
        }
        let w = (t - t1) / (t30 - t1);
        (1.0 - w) * vol.short + w * vol.atm
    }

    pub fn implied_vol(spot: f64, strike: f64, t: f64, vol: &VolSnapshot) -> f64 {
        let atm = Self::atm_vol(t, vol).max(0.01);
        let tt = t.max(1.0 / MINUTES_PER_YEAR);
        let z = (strike / spot).ln() / (atm * tt.sqrt());
        let z = z.max(-6.0).min(6.0);
        let mult = 1.0 + SKEW_SLOPE * z + SKEW_CURVE * z * z;
        atm * mult.max(MULT_FLOOR).min(MULT_CAP)
    }

    /// Quote of one option at `now` (date, minute of the day).
    pub fn quote(call: bool, strike: f64, expiry: Date, today: Date, minute: i32, spot: f64, vol: &VolSnapshot) -> Quote {
        let t = year_fraction(today, minute, expiry);
        if t <= 0.0 {
            let px = intrinsic(call, spot, strike);
            return Quote { bid: px, ask: px, mid: px, iv: 0.0, delta: bs_delta(call, spot, strike, 0.0, 0.0, 0.0) };
        }
        let iv = Self::implied_vol(spot, strike, t, vol);
        let mid = bs_price(call, spot, strike, t, vol.rate, iv);
        let half = (SPREAD_REL * mid).max(SPREAD_MIN).min(SPREAD_MAX);
        let bid = (mid - half).max(0.0);
        Quote { bid, ask: mid + half, mid, iv, delta: bs_delta(call, spot, strike, t, vol.rate, iv) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::calendar::days_from_civil;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} vs {b}");
    }

    #[test]
    fn normal_cdf_hand_values() {
        close(norm_cdf(0.0), 0.5, 1e-15);
        close(norm_cdf(1.0), 0.841_344_746_068_542_9, 1e-12);
        close(norm_cdf(-1.96), 0.024_997_895_148_220_435, 1e-12);
    }

    #[test]
    fn black_scholes_hand_values() {
        // S=100 K=100 T=1 r=5% vol=20%: call 10.4506, put 5.5735 (textbook values).
        close(bs_price(true, 100.0, 100.0, 1.0, 0.05, 0.2), 10.450_583_572_185_565, 1e-9);
        close(bs_price(false, 100.0, 100.0, 1.0, 0.05, 0.2), 5.573_526_022_256_971, 1e-9);
        // Put-call parity: C - P = S - K exp(-rT).
        let c = bs_price(true, 123.0, 117.0, 0.3, 0.04, 0.27);
        let p = bs_price(false, 123.0, 117.0, 0.3, 0.04, 0.27);
        close(c - p, 123.0 - 117.0 * (-0.04f64 * 0.3).exp(), 1e-9);
        // Expired option: intrinsic value; delta is 0 or 1.
        close(bs_price(true, 105.0, 100.0, 0.0, 0.05, 0.2), 5.0, 0.0);
        close(bs_delta(false, 95.0, 100.0, 0.0, 0.0, 0.0), -1.0, 0.0);
        close(bs_delta(true, 95.0, 100.0, 0.0, 0.0, 0.0), 0.0, 0.0);
        close(bs_delta(true, 100.0, 100.0, 1.0, 0.05, 0.2), 0.636_830_651_175_619_4, 1e-9);
    }

    #[test]
    fn variance_clock() {
        let mon = days_from_civil(2026, 11, 16);
        let tue = mon + 1;
        let fri = mon + 4;
        let next_mon = mon + 7;
        let day = 1.0 / 252.0;
        // One full close-to-close day is 1/252.
        close(year_fraction(mon, CLOSE_MINUTE, tue), day, 1e-12);
        // Friday close to Monday close is also one day (the weekend counts as one overnight).
        close(year_fraction(fri, CLOSE_MINUTE, next_mon), day, 1e-12);
        // From the open of the expiry day: the whole session without the overnight.
        close(year_fraction(mon, 9 * 60 + 30, mon), 1.0 / 252.0 / 1.25, 1e-12);
        // Expired: zero. At the close of the expiry day nothing is left.
        assert_eq!(year_fraction(tue, 600, mon), 0.0);
        assert!(year_fraction(mon, CLOSE_MINUTE, mon) <= 0.0);
        // The first half hour carries more variance than a midday half hour.
        let first = year_fraction(mon, 9 * 60 + 30, mon) - year_fraction(mon, 10 * 60, mon);
        let mid = year_fraction(mon, 12 * 60, mon) - year_fraction(mon, 12 * 60 + 30, mon);
        assert!(first > mid);
    }

    #[test]
    fn skew_and_spread_rules() {
        let vol = VolSnapshot { atm: 0.20, short: 0.16, rate: 0.04 };
        let today = days_from_civil(2026, 11, 16);
        let exp = today + 7;
        let put = SyntheticPricer::quote(false, 95.0, exp, today, CLOSE_MINUTE, 100.0, &vol);
        let call = SyntheticPricer::quote(true, 105.0, exp, today, CLOSE_MINUTE, 100.0, &vol);
        assert!(put.iv > call.iv, "puts are richer than calls");
        for q in [put, call] {
            assert!(q.bid <= q.mid && q.mid <= q.ask);
            let half = q.ask - q.mid;
            assert!((0.01 - 1e-12..=0.05 + 1e-12).contains(&half));
        }
        // ATM vol: the short vol at one day or less, the 30-day vol at 30 days or more.
        close(SyntheticPricer::atm_vol(1.0 / 252.0, &vol), 0.16, 0.0);
        close(SyntheticPricer::atm_vol(30.0 / 252.0, &vol), 0.20, 0.0);
        close(SyntheticPricer::atm_vol(15.5 / 252.0, &vol), 0.18, 1e-12);
        // Expired quote: intrinsic value with no spread.
        let q = SyntheticPricer::quote(true, 90.0, today, today, CLOSE_MINUTE, 100.0, &vol);
        assert_eq!((q.bid, q.ask, q.mid), (10.0, 10.0, 10.0));
    }
}
