//! Defined-risk option structures. All legs have one expiry, so the maximum loss is exact.
//! A quantity is for ONE unit of the structure (a vertical is +1 / -1). Prices are for one
//! share; multiply by `MULTIPLIER` for dollars of one contract.

use super::calendar::{iso, Date};
use super::pricing::intrinsic;

pub const MULTIPLIER: f64 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leg {
    pub call: bool,
    pub strike: f64,
    /// Positive is long, negative is short, for one unit.
    pub qty: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Structure {
    pub name: &'static str,
    pub symbol: &'static str,
    pub expiry: Date,
    pub legs: Vec<Leg>,
}

impl Structure {
    /// Checks the rules of a defined-risk structure: at least one leg, valid legs, and no net
    /// short calls (unbounded loss) or net short puts (naked).
    pub fn new(name: &'static str, symbol: &'static str, expiry: Date, legs: Vec<Leg>) -> Result<Self, String> {
        if legs.is_empty() {
            return Err("structure needs at least one leg".into());
        }
        for leg in &legs {
            if leg.qty == 0 || !(leg.strike > 0.0) {
                return Err(format!("bad leg {leg:?}"));
            }
        }
        for call in [true, false] {
            let net: i32 = legs.iter().filter(|l| l.call == call).map(|l| l.qty).sum();
            if net < 0 {
                let side = if call { "C" } else { "P" };
                return Err(format!("{name}: net short {side} legs = not defined-risk"));
            }
        }
        Ok(Structure { name, symbol, expiry, legs })
    }

    pub fn contracts_per_unit(&self) -> i32 {
        self.legs.iter().map(|l| l.qty.abs()).sum()
    }

    /// Value at expiry for one unit, for one share.
    pub fn payoff(&self, spot: f64) -> f64 {
        self.legs.iter().fold(0.0, |acc, l| acc + l.qty as f64 * intrinsic(l.call, spot, l.strike))
    }

    /// The payoff is piecewise linear: check 0, every strike, and the far upside.
    fn kink_points(&self) -> Vec<f64> {
        let mut strikes: Vec<f64> = self.legs.iter().map(|l| l.strike).collect();
        strikes.sort_by(|a, b| a.partial_cmp(b).expect("strikes are numbers"));
        strikes.dedup();
        let last = *strikes.last().expect("one leg at least");
        let mut points = vec![0.0];
        points.extend(strikes);
        points.push(last * 10.0);
        points
    }

    pub fn min_payoff(&self) -> f64 {
        self.kink_points().into_iter().map(|s| self.payoff(s)).fold(f64::INFINITY, f64::min)
    }

    /// Worst-case loss for one unit and one share, for a net entry price (+debit, -credit).
    pub fn max_loss(&self, entry_net: f64) -> f64 {
        entry_net - self.min_payoff()
    }

    pub fn describe(&self) -> String {
        let legs: Vec<String> = self
            .legs
            .iter()
            .map(|l| format!("{}{}{}{}", if l.qty > 0 { "+" } else { "" }, l.qty, if l.call { "C" } else { "P" }, fmt_g(l.strike)))
            .collect();
        format!("{} {} {} [{}]", self.symbol, iso(self.expiry), self.name, legs.join(" "))
    }
}

/// Python `format(x, "g")`: 6 significant digits, no trailing zeros.
pub fn fmt_g(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    if !x.is_finite() {
        return format!("{x}");
    }
    let sci = format!("{:.5e}", x);
    let (mant, exp) = sci.split_once('e').expect("exponent");
    let exp: i32 = exp.parse().expect("exponent number");
    let trim = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if exp < -4 || exp >= 6 {
        format!("{}e{}{:02}", trim(mant), if exp < 0 { '-' } else { '+' }, exp.abs())
    } else {
        trim(&format!("{:.*}", (5 - exp).max(0) as usize, x))
    }
}

/// Python `round(round(x / increment) * increment, 2)`; Python rounds halves to even.
pub fn round_strike(x: f64, increment: f64) -> f64 {
    let r = (x / increment).round_ties_even() * increment;
    (r * 100.0).round_ties_even() / 100.0
}

fn call_leg(strike: f64, qty: i32) -> Leg {
    Leg { call: true, strike, qty }
}

fn put_leg(strike: f64, qty: i32) -> Leg {
    Leg { call: false, strike, qty }
}

/// Call debit: long < short. Put debit: long > short. Reverse for credit spreads.
pub fn vertical(symbol: &'static str, expiry: Date, call: bool, long_strike: f64, short_strike: f64) -> Result<Structure, String> {
    if long_strike == short_strike {
        return Err("vertical strikes must differ".into());
    }
    let debit = if call { long_strike < short_strike } else { long_strike > short_strike };
    let name = match (call, debit) {
        (true, true) => "call_debit_spread",
        (true, false) => "call_credit_spread",
        (false, true) => "put_debit_spread",
        (false, false) => "put_credit_spread",
    };
    let leg = |k: f64, q: i32| Leg { call, strike: k, qty: q };
    Structure::new(name, symbol, expiry, vec![leg(long_strike, 1), leg(short_strike, -1)])
}

pub fn iron_condor(symbol: &'static str, expiry: Date, put_long: f64, put_short: f64, call_short: f64, call_long: f64) -> Result<Structure, String> {
    if !(put_long < put_short && put_short <= call_short && call_short < call_long) {
        return Err("iron condor strikes must be ordered put_long < put_short <= call_short < call_long".into());
    }
    let name = if put_short == call_short { "iron_butterfly" } else { "iron_condor" };
    Structure::new(name, symbol, expiry, vec![put_leg(put_long, 1), put_leg(put_short, -1), call_leg(call_short, -1), call_leg(call_long, 1)])
}

pub fn butterfly(symbol: &'static str, expiry: Date, call: bool, lower: f64, middle: f64, upper: f64) -> Result<Structure, String> {
    if !(lower < middle && middle < upper) {
        return Err("butterfly strikes must be ordered".into());
    }
    let name = if call { "call_butterfly" } else { "put_butterfly" };
    let leg = |k: f64, q: i32| Leg { call, strike: k, qty: q };
    Structure::new(name, symbol, expiry, vec![leg(lower, 1), leg(middle, -2), leg(upper, 1)])
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXP: Date = 20_000;

    #[test]
    fn vertical_names_and_max_loss() {
        let cds = vertical("SPY", EXP, true, 100.0, 101.0).unwrap();
        assert_eq!(cds.name, "call_debit_spread");
        // Debit 0.40 -> loss 0.40; payoff is worth 0 below the long strike and 1 above the short.
        assert!((cds.max_loss(0.40) - 0.40).abs() < 1e-12);
        assert_eq!(cds.payoff(99.0), 0.0);
        assert_eq!(cds.payoff(105.0), 1.0);
        let pcs = vertical("SPY", EXP, false, 99.0, 100.0).unwrap();
        assert_eq!(pcs.name, "put_credit_spread");
        // Credit 0.30 (entry -0.30) on a $1 wide spread: worst case 0.70.
        assert!((pcs.max_loss(-0.30) - 0.70).abs() < 1e-12);
        assert_eq!(vertical("SPY", EXP, true, 100.0, 100.0).unwrap_err(), "vertical strikes must differ");
    }

    #[test]
    fn iron_condor_and_butterfly() {
        let fly = iron_condor("SPY", EXP, 98.0, 100.0, 100.0, 102.0).unwrap();
        assert_eq!(fly.name, "iron_butterfly");
        assert_eq!(fly.contracts_per_unit(), 4);
        assert!((fly.max_loss(-1.5) - 0.5).abs() < 1e-12);
        let ic = iron_condor("SPY", EXP, 97.0, 98.0, 102.0, 103.0).unwrap();
        assert_eq!(ic.name, "iron_condor");
        assert!(iron_condor("SPY", EXP, 98.0, 97.0, 102.0, 103.0).is_err());
        let b = butterfly("SPY", EXP, true, 99.0, 100.0, 101.0).unwrap();
        assert_eq!(b.payoff(100.0), 1.0);
        assert!((b.max_loss(0.3) - 0.3).abs() < 1e-12);
        assert!(butterfly("SPY", EXP, true, 99.0, 99.0, 101.0).is_err());
    }

    #[test]
    fn undefined_risk_is_refused() {
        let naked_call = Structure::new("x", "SPY", EXP, vec![call_leg(100.0, -1)]);
        assert!(naked_call.unwrap_err().contains("not defined-risk"));
        let naked_put = Structure::new("x", "SPY", EXP, vec![put_leg(100.0, -1), put_leg(95.0, 0)]);
        assert!(naked_put.is_err());
        assert!(Structure::new("x", "SPY", EXP, vec![]).is_err());
    }

    #[test]
    fn describe_and_rounding_match_python() {
        let v = vertical("SPY", days(2026, 11, 20), true, 730.0, 731.0).unwrap();
        assert_eq!(v.describe(), "SPY 2026-11-20 call_debit_spread [+1C730 -1C731]");
        let p = vertical("QQQ", days(2026, 11, 20), false, 650.5, 649.5).unwrap();
        assert_eq!(p.describe(), "QQQ 2026-11-20 put_debit_spread [+1P650.5 -1P649.5]");
        assert_eq!(fmt_g(1234567.0), "1.23457e+06");
        assert_eq!(fmt_g(0.00001234), "1.234e-05");
        assert_eq!(round_strike(730.4, 1.0), 730.0);
        assert_eq!(round_strike(730.5, 1.0), 730.0); // half to even
        assert_eq!(round_strike(731.5, 1.0), 732.0);
    }

    fn days(y: i32, m: u32, d: u32) -> Date {
        super::super::calendar::days_from_civil(y, m, d)
    }
}
