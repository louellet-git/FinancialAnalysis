//! Curve mathematics for the bond reconciliation: dates, par curve to zero curve,
//! discount factors, and the prices and yields of bills and plain coupon bonds.
//!
//! This is the only place that has this mathematics. All rates are decimals.
//!
//! CONVENTIONS
//! - Par yields (U.S. Treasury daily par yield curve) are bond-equivalent yields:
//!   semi-annual compounding. A par yield `y` is the coupon rate of a bond that
//!   prices to 100 on the curve date. Its coupons are `y/2` each half year.
//! - Tenors up to 6 months are bills (zero-coupon). The Treasury inputs are the 4, 6,
//!   8, 13, 17 and 26 week bills, so 1M, 1.5M, 2M, 3M, 4M and 6M are 28, 42, 56, 91,
//!   119 and 182 days. A bill node has `D = 1 / (1 + y t)` with `t = days / 365`
//!   (the bill bond-equivalent yield is simple interest, actual/365).
//! - Tenors of 1 year and more are coupon bonds. The coupon dates are the curve date
//!   plus 6, 12, 18, ... calendar months.
//! - Time is `t = days / 365` (actual/365 fixed). A calendar half year is 181 to 184
//!   days, so `t` of a coupon date is not exactly 0.5, 1.0, ...
//! - The zero rate is continuously compounded: `D(t) = exp(-r t)`.
//! - Interpolation: the monotone convex method of Hagan and West (2006) on the
//!   forward rates. The Treasury says it builds its par curve in this way (bootstrap
//!   of the instantaneous forward rates at the inputs, then monotone convex
//!   interpolation). The Treasury does not publish its program, so our curve is a
//!   reasonable re-creation and not an exact copy. After the last node, the forward
//!   rate is flat.
//! - Settlement is T+1 business day (regular way for Treasury securities). Business
//!   days exclude Saturday, Sunday and the Federal Reserve holidays.
//! - Treasury notes and bonds: accrued interest is actual/actual (the days in the
//!   coupon period are the days between the two coupon dates). Yield is the
//!   street convention: semi-annual, with a fractional first period; one simple
//!   interest period when the last coupon is the only one left.
//! - Bills: yield is the bond-equivalent yield, actual/365; the quadratic formula
//!   is used for more than 182 days.

use std::fmt;

// ---------------------------------------------------------------------------
// Dates (days since 1970-01-01, proleptic Gregorian calendar)
// ---------------------------------------------------------------------------

pub type Day = i64;

pub fn days_from_civil(y: i64, m: i64, d: i64) -> Day {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: Day) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

pub fn days_in_month(y: i64, m: i64) -> i64 {
    let next = if m == 12 { days_from_civil(y + 1, 1, 1) } else { days_from_civil(y, m + 1, 1) };
    next - days_from_civil(y, m, 1)
}

/// "YYYY-MM-DD".
pub fn parse_iso(s: &str) -> Option<Day> {
    let mut p = s.trim().split('-');
    let (y, m, d) = (p.next()?.parse().ok()?, p.next()?.parse().ok()?, p.next()?.parse().ok()?);
    valid(y, m, d)
}

/// "MM/DD/YYYY" (the format of the FedInvest file).
pub fn parse_us(s: &str) -> Option<Day> {
    let mut p = s.trim().split('/');
    let (m, d, y) = (p.next()?.parse().ok()?, p.next()?.parse().ok()?, p.next()?.parse().ok()?);
    valid(y, m, d)
}

fn valid(y: i64, m: i64, d: i64) -> Option<Day> {
    if (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m) { Some(days_from_civil(y, m, d)) } else { None }
}

pub fn iso(day: Day) -> String {
    let (y, m, d) = civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// 0 = Sunday ... 6 = Saturday.
pub fn weekday(day: Day) -> i64 {
    (day + 4).rem_euclid(7)
}

/// Add calendar months. When `month_end` is true and the date is the last day of its
/// month, the result is the last day of the target month (a note that matures on
/// 11/30 pays its coupons on 5/31 and 11/30). Else the day is cut to the month length.
pub fn add_months(day: Day, months: i64, month_end: bool) -> Day {
    let (y, m, d) = civil_from_days(day);
    let total = y * 12 + (m - 1) + months;
    let (ny, nm) = (total.div_euclid(12), total.rem_euclid(12) + 1);
    let last = days_in_month(ny, nm);
    let nd = if month_end && d == days_in_month(y, m) { last } else { d.min(last) };
    days_from_civil(ny, nm, nd)
}

fn nth_weekday(y: i64, m: i64, wd: i64, n: i64) -> Day {
    let first = days_from_civil(y, m, 1);
    first + (wd - weekday(first)).rem_euclid(7) + 7 * (n - 1)
}

fn last_weekday(y: i64, m: i64, wd: i64) -> Day {
    let last = days_from_civil(y, m, days_in_month(y, m));
    last - (weekday(last) - wd).rem_euclid(7)
}

/// Federal Reserve holidays of one year. A holiday on Sunday is observed on Monday.
/// A holiday on Saturday is not observed on Friday (the Federal Reserve rule).
fn holidays(y: i64) -> Vec<Day> {
    let fixed = |m, d| {
        let day = days_from_civil(y, m, d);
        if weekday(day) == 0 { day + 1 } else { day }
    };
    vec![
        fixed(1, 1),
        nth_weekday(y, 1, 1, 3),
        nth_weekday(y, 2, 1, 3),
        last_weekday(y, 5, 1),
        fixed(6, 19),
        fixed(7, 4),
        nth_weekday(y, 9, 1, 1),
        nth_weekday(y, 10, 1, 2),
        fixed(11, 11),
        nth_weekday(y, 11, 4, 4),
        fixed(12, 25),
    ]
}

pub fn is_business_day(day: Day) -> bool {
    let wd = weekday(day);
    if wd == 0 || wd == 6 {
        return false;
    }
    let (y, _, _) = civil_from_days(day);
    !holidays(y).contains(&day)
}

/// The business day `n` business days after `day`.
pub fn add_business_days(day: Day, n: u32) -> Day {
    let (mut d, mut left) = (day, n);
    while left > 0 {
        d += 1;
        if is_business_day(d) {
            left -= 1;
        }
    }
    d
}

// ---------------------------------------------------------------------------
// Tenors
// ---------------------------------------------------------------------------

/// How a tenor of the Treasury par curve is built: bill days or coupon-bond months.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tenor {
    Bill { days: i64 },
    Coupon { months: i64 },
}

/// The tenor labels of the Treasury par curve (the labels of `financial_analysis::TENORS`).
pub fn tenor_spec(label: &str) -> Option<Tenor> {
    Some(match label {
        "1M" => Tenor::Bill { days: 28 },
        "1.5M" => Tenor::Bill { days: 42 },
        "2M" => Tenor::Bill { days: 56 },
        "3M" => Tenor::Bill { days: 91 },
        "4M" => Tenor::Bill { days: 119 },
        "6M" => Tenor::Bill { days: 182 },
        other => {
            let years: i64 = other.strip_suffix('Y')?.parse().ok()?;
            Tenor::Coupon { months: 12 * years }
        }
    })
}

pub fn year_fraction(from: Day, to: Day) -> f64 {
    (to - from) as f64 / 365.0
}

// ---------------------------------------------------------------------------
// Monotone convex zero curve (Hagan and West 2006)
// ---------------------------------------------------------------------------

/// A zero curve. `t[0] = 0`. `l[i] = r_i * t_i` is the integral of the forward rate
/// to node `i`. `fd` are the discrete forward rates of the intervals, `f` the
/// instantaneous forward rates at the nodes.
#[derive(Debug, Clone)]
pub struct ZeroCurve {
    pub date: Day,
    pub t: Vec<f64>,
    l: Vec<f64>,
    fd: Vec<f64>,
    f: Vec<f64>,
}

impl ZeroCurve {
    /// `times` increasing and positive, `zeros` the continuous zero rates at the times.
    pub fn from_zeros(date: Day, times: &[f64], zeros: &[f64]) -> ZeroCurve {
        let n = times.len();
        let mut t = vec![0.0];
        t.extend_from_slice(times);
        let mut l = vec![0.0];
        l.extend(times.iter().zip(zeros).map(|(t, r)| t * r));
        let mut fd = vec![0.0; n + 1];
        for i in 1..=n {
            fd[i] = (l[i] - l[i - 1]) / (t[i] - t[i - 1]);
        }
        let mut f = vec![0.0; n + 1];
        for i in 1..n {
            let (a, b) = (t[i] - t[i - 1], t[i + 1] - t[i]);
            f[i] = (a * fd[i + 1] + b * fd[i]) / (a + b);
        }
        if n == 1 {
            f[0] = fd[1];
            f[1] = fd[1];
        } else {
            f[0] = fd[1] - 0.5 * (f[1] - fd[1]);
            f[n] = fd[n] - 0.5 * (f[n - 1] - fd[n]);
            // Positivity (Hagan and West, section 4).
            f[0] = f[0].min(2.0 * fd[1]).max(0.0);
            f[n] = f[n].min(2.0 * fd[n]).max(0.0);
            for i in 1..n {
                f[i] = f[i].min(2.0 * fd[i].min(fd[i + 1])).max(0.0);
            }
        }
        ZeroCurve { date, t, l, fd, f }
    }

    /// `-ln D(t)`.
    pub fn log_df(&self, t: f64) -> f64 {
        let n = self.t.len() - 1;
        if t <= 0.0 {
            return 0.0;
        }
        if t >= self.t[n] {
            return self.l[n] + (t - self.t[n]) * self.f[n];
        }
        let i = self.t.partition_point(|x| *x < t).max(1); // t[i-1] < t <= t[i]
        let width = self.t[i] - self.t[i - 1];
        let x = (t - self.t[i - 1]) / width;
        let g0 = self.f[i - 1] - self.fd[i];
        let g1 = self.f[i] - self.fd[i];
        self.l[i - 1] + (t - self.t[i - 1]) * self.fd[i] + width * g_integral(x, g0, g1)
    }

    pub fn df(&self, t: f64) -> f64 {
        (-self.log_df(t)).exp()
    }

    pub fn df_on(&self, day: Day) -> f64 {
        self.df(year_fraction(self.date, day))
    }

    /// Continuous zero rate at time `t`.
    pub fn zero(&self, t: f64) -> f64 {
        if t <= 0.0 { self.f[0] } else { self.log_df(t) / t }
    }
}

/// Integral from 0 to `x` of the monotone convex forward deviation `g`, with `g(0) = g0`,
/// `g(1) = g1` and the integral over [0, 1] equal to 0 (Hagan and West, 2006).
fn g_integral(x: f64, g0: f64, g1: f64) -> f64 {
    const EPS: f64 = 1e-15;
    if g0.abs() < EPS && g1.abs() < EPS {
        return 0.0;
    }
    let cube = |v: f64| v * v * v;
    // Zone (i): one quadratic.
    if (g0 >= 0.0 && -0.5 * g0 >= g1 && g1 >= -2.0 * g0) || (g0 <= 0.0 && -0.5 * g0 <= g1 && g1 <= -2.0 * g0) {
        return g0 * (x - 2.0 * x * x + cube(x)) + g1 * (cube(x) - x * x);
    }
    // Zone (ii): flat, then a quadratic rise.
    if (g0 < 0.0 && g1 > -2.0 * g0) || (g0 > 0.0 && g1 < -2.0 * g0) {
        let eta = (g1 + 2.0 * g0) / (g1 - g0);
        return if x <= eta { g0 * x } else { g0 * x + (g1 - g0) * cube(x - eta) / (3.0 * (1.0 - eta) * (1.0 - eta)) };
    }
    // Zone (iii): a quadratic, then flat.
    if (g0 < 0.0 && g1 >= 0.0 && g1 < -0.5 * g0) || (g0 > 0.0 && g1 <= 0.0 && g1 > -0.5 * g0) {
        let eta = 3.0 * g1 / (g1 - g0);
        return if x < eta {
            g1 * x + (g0 - g1) * (cube(eta) - cube(eta - x)) / (3.0 * eta * eta)
        } else {
            g1 * x + (g0 - g1) * eta / 3.0
        };
    }
    // Zone (iv): g0 and g1 have the same sign.
    let eta = g1 / (g0 + g1);
    let a = -g0 * g1 / (g0 + g1);
    if x < eta {
        a * x + (g0 - a) * (cube(eta) - cube(eta - x)) / (3.0 * eta * eta)
    } else {
        let tail = if 1.0 - eta < 1e-12 { 0.0 } else { (g1 - a) * cube(x - eta) / (3.0 * (1.0 - eta) * (1.0 - eta)) };
        a * x + (g0 - a) * eta / 3.0 + tail
    }
}

// ---------------------------------------------------------------------------
// Par curve to zero curve
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum CurveError {
    NoNodes,
    UnknownTenor(String),
    NoRoot(String),
}

impl fmt::Display for CurveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CurveError::NoNodes => write!(f, "the par curve has no usable tenor"),
            CurveError::UnknownTenor(t) => write!(f, "unknown tenor {t}"),
            CurveError::NoRoot(t) => write!(f, "the bootstrap did not converge at tenor {t}"),
        }
    }
}

impl std::error::Error for CurveError {}

/// One node of a bootstrapped curve.
#[derive(Debug, Clone)]
pub struct Node {
    pub label: String,
    pub spec: Tenor,
    pub par: f64,
    pub t: f64,
    pub zero: f64,
}

/// A zero curve with the par yields it was built from.
#[derive(Debug, Clone)]
pub struct Bootstrapped {
    pub curve: ZeroCurve,
    pub nodes: Vec<Node>,
}

fn node_time(date: Day, spec: Tenor) -> f64 {
    match spec {
        Tenor::Bill { days } => days as f64 / 365.0,
        Tenor::Coupon { months } => year_fraction(date, add_months(date, months, false)),
    }
}

/// Price of the par bond of a coupon node on a candidate curve, minus 1.
fn par_error(curve: &ZeroCurve, par: f64, months: i64) -> f64 {
    let mut pv = 0.0;
    let periods = months / 6;
    for k in 1..=periods {
        let t = year_fraction(curve.date, add_months(curve.date, 6 * k, false));
        let cf = if k == periods { 1.0 + par / 2.0 } else { par / 2.0 };
        pv += cf * curve.df(t);
    }
    pv - 1.0
}

/// Bootstrap par yields to zero rates. `par` has (label, decimal par yield) in
/// increasing tenor order. Bill nodes are exact. Coupon nodes are solved one at a
/// time (bisection) and the sweep repeats until all par bonds price to 100: the
/// interpolation of a coupon date depends on its neighbour nodes.
pub fn bootstrap(date: Day, par: &[(String, f64)]) -> Result<Bootstrapped, CurveError> {
    if par.is_empty() {
        return Err(CurveError::NoNodes);
    }
    let mut nodes = Vec::new();
    for (label, y) in par {
        let spec = tenor_spec(label).ok_or_else(|| CurveError::UnknownTenor(label.clone()))?;
        let t = node_time(date, spec);
        let zero = match spec {
            Tenor::Bill { .. } => (1.0 + y * t).ln() / t,
            Tenor::Coupon { .. } => (1.0 + y / 2.0).ln() * 2.0, // first guess
        };
        nodes.push(Node { label: label.clone(), spec, par: *y, t, zero });
    }
    let times: Vec<f64> = nodes.iter().map(|n| n.t).collect();
    let build = |nodes: &[Node]| ZeroCurve::from_zeros(date, &times, &nodes.iter().map(|n| n.zero).collect::<Vec<_>>());
    let worst_error = |nodes: &[Node]| {
        let curve = build(nodes);
        nodes
            .iter()
            .filter_map(|n| match n.spec {
                Tenor::Coupon { months } => Some(par_error(&curve, n.par, months).abs()),
                Tenor::Bill { .. } => None,
            })
            .fold(0.0, f64::max)
    };
    for _ in 0..200 {
        for j in 0..nodes.len() {
            let Tenor::Coupon { months } = nodes[j].spec else { continue };
            let par_y = nodes[j].par;
            let err = |r: f64, nodes: &mut Vec<Node>| {
                nodes[j].zero = r;
                par_error(&build(nodes), par_y, months)
            };
            let (mut lo, mut hi) = (-0.2, 0.6);
            if err(lo, &mut nodes) * err(hi, &mut nodes) > 0.0 {
                return Err(CurveError::NoRoot(nodes[j].label.clone()));
            }
            for _ in 0..100 {
                let mid = 0.5 * (lo + hi);
                if err(mid, &mut nodes) > 0.0 { lo = mid } else { hi = mid }
            }
            nodes[j].zero = 0.5 * (lo + hi);
        }
        if worst_error(&nodes) < 1e-13 {
            return Ok(Bootstrapped { curve: build(&nodes), nodes });
        }
    }
    Err(CurveError::NoRoot("sweep".into()))
}

// ---------------------------------------------------------------------------
// Bills
// ---------------------------------------------------------------------------

/// Bill price per 100 from the curve: `100 D(maturity) / D(settle)`.
pub fn bill_price(curve: &ZeroCurve, settle: Day, maturity: Day) -> f64 {
    100.0 * curve.df_on(maturity) / curve.df_on(settle)
}

/// Bond-equivalent yield of a bill, actual/365.
pub fn bill_yield(price: f64, days: i64) -> f64 {
    let (d, y) = (days as f64, 365.0);
    if days <= 182 {
        return (100.0 - price) / price * y / d;
    }
    let a = d / (2.0 * y) - 0.25;
    let b = d / y;
    let c = (price - 100.0) / price;
    (-b + (b * b - 4.0 * a * c).sqrt()) / (2.0 * a)
}

// ---------------------------------------------------------------------------
// Coupon bonds
// ---------------------------------------------------------------------------

/// A plain fixed-coupon bond. `coupon` is the decimal annual rate.
#[derive(Debug, Clone, Copy)]
pub struct Bond {
    pub coupon: f64,
    pub maturity: Day,
}

/// The coupon schedule round a settlement date.
#[derive(Debug, Clone)]
pub struct Schedule {
    /// The last coupon date on or before settlement.
    pub prev: Day,
    /// The coupon dates after settlement, the first one first. The last one is the maturity.
    pub next: Vec<Day>,
}

impl Bond {
    pub fn schedule(&self, settle: Day) -> Schedule {
        let mut dates = Vec::new();
        let mut k = 0;
        loop {
            let d = add_months(self.maturity, -6 * k, true);
            if d <= settle {
                dates.reverse();
                return Schedule { prev: d, next: dates };
            }
            dates.push(d);
            k += 1;
        }
    }

    /// Accrued interest per 100, actual/actual.
    pub fn accrued(&self, settle: Day) -> f64 {
        let s = self.schedule(settle);
        let (prev, next) = (s.prev, s.next[0]);
        100.0 * self.coupon / 2.0 * (settle - prev) as f64 / (next - prev) as f64
    }

    /// Dirty price per 100 at settlement, from the curve: sum of the cash flows after
    /// settlement, discounted to the settlement date.
    pub fn dirty_from_curve(&self, curve: &ZeroCurve, settle: Day) -> f64 {
        let s = self.schedule(settle);
        let last = s.next.len() - 1;
        let pv: f64 = s
            .next
            .iter()
            .enumerate()
            .map(|(i, d)| (100.0 * self.coupon / 2.0 + if i == last { 100.0 } else { 0.0 }) * curve.df_on(*d))
            .sum();
        pv / curve.df_on(settle)
    }

    pub fn clean_from_curve(&self, curve: &ZeroCurve, settle: Day) -> f64 {
        self.dirty_from_curve(curve, settle) - self.accrued(settle)
    }

    /// Dirty price per 100 for a yield (semi-annual, street convention).
    pub fn dirty_from_yield(&self, y: f64, settle: Day) -> f64 {
        let s = self.schedule(settle);
        let n = s.next.len();
        let cpn = 100.0 * self.coupon / 2.0;
        let w = (s.next[0] - settle) as f64 / (s.next[0] - s.prev) as f64;
        if n == 1 {
            return (100.0 + cpn) / (1.0 + w * y / 2.0);
        }
        let v = 1.0 / (1.0 + y / 2.0);
        (0..n).map(|k| (cpn + if k == n - 1 { 100.0 } else { 0.0 }) * v.powf(w + k as f64)).sum()
    }

    pub fn clean_from_yield(&self, y: f64, settle: Day) -> f64 {
        self.dirty_from_yield(y, settle) - self.accrued(settle)
    }

    /// The yield that gives a clean price (bisection; the price falls when the yield rises).
    pub fn yield_from_clean(&self, clean: f64, settle: Day) -> Option<f64> {
        let (mut lo, mut hi) = (-0.5, 1.5);
        if self.clean_from_yield(lo, settle) < clean || self.clean_from_yield(hi, settle) > clean {
            return None;
        }
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if self.clean_from_yield(mid, settle) > clean { lo = mid } else { hi = mid }
        }
        Some(0.5 * (lo + hi))
    }
}

/// Semi-annual equivalent of a continuous rate.
pub fn continuous_to_semi(r: f64) -> f64 {
    2.0 * ((r / 2.0).exp() - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Day {
        parse_iso(s).unwrap()
    }

    fn flat_par(y: f64) -> Vec<(String, f64)> {
        ["1M", "1.5M", "2M", "3M", "4M", "6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "20Y", "30Y"]
            .iter()
            .map(|t| (t.to_string(), y))
            .collect()
    }

    #[test]
    fn civil_round_trip_and_weekday() {
        for day in [-1000, 0, 11_016, 20_000, 24_000] {
            let (y, m, dd) = civil_from_days(day);
            assert_eq!(days_from_civil(y, m, dd), day);
        }
        assert_eq!(iso(d("2026-11-18")), "2026-11-18");
        assert_eq!(weekday(d("2026-11-18")), 3); // Wednesday
        assert_eq!(weekday(0), 4); // 1970-01-01 was a Thursday
        assert_eq!(d("2027-02-15") - d("2026-11-15"), 92);
        assert!(parse_iso("2026-02-30").is_none());
        assert_eq!(parse_us("11/30/2026"), Some(d("2026-11-30")));
    }

    #[test]
    fn month_end_coupon_dates() {
        assert_eq!(iso(add_months(d("2026-11-30"), -6, true)), "2026-05-31");
        assert_eq!(iso(add_months(d("2027-02-28"), -6, true)), "2026-08-31");
        assert_eq!(iso(add_months(d("2026-11-15"), -6, true)), "2026-05-15");
        assert_eq!(iso(add_months(d("2026-08-31"), 6, false)), "2027-02-28");
    }

    #[test]
    fn business_days_skip_weekends_and_holidays() {
        assert_eq!(iso(add_business_days(d("2026-11-17"), 1)), "2026-11-18");
        assert_eq!(iso(add_business_days(d("2026-11-10"), 1)), "2026-11-12"); // Veterans Day, Wednesday
        assert_eq!(iso(add_business_days(d("2026-11-25"), 1)), "2026-11-27"); // Thanksgiving, Thursday
        assert_eq!(iso(add_business_days(d("2026-11-27"), 1)), "2026-11-30"); // weekend
        // 4 July 2026 is a Saturday: the Federal Reserve does not close on Friday 3 July.
        assert_eq!(iso(add_business_days(d("2026-07-02"), 1)), "2026-07-03");
        assert_eq!(iso(add_business_days(d("2026-07-03"), 1)), "2026-07-06");
    }

    #[test]
    fn bill_yield_by_hand() {
        // (100 - 98.9) / 98.9 * 365 / 91
        assert!((bill_yield(98.9, 91) - 0.044_611_606_795_630_84).abs() < 1e-12);
        // 364 days: quadratic formula, computed apart.
        assert!((bill_yield(95.5, 364) - 0.046_706_006_047_388_63).abs() < 1e-12);
        // At the day limit both formulas are near: no jump.
        assert!((bill_yield(97.8, 182) - bill_yield(97.8, 183)).abs() < 1e-3);
    }

    #[test]
    fn bond_between_coupon_dates_by_hand() {
        let bond = Bond { coupon: 0.06, maturity: d("2027-05-15") };
        // On the coupon date 2026-11-15 one coupon is left: 103 / (1 + y/2) = 100 at y = 6%.
        assert!((bond.clean_from_yield(0.06, d("2026-11-15")) - 100.0).abs() < 1e-12);
        // 2027-02-15: accrued = 3 * 92 / 181, w = 89 / 181, simple interest in the last period.
        let settle = d("2027-02-15");
        assert!((bond.accrued(settle) - 1.524_861_878_453_878_4).abs() < 1e-12);
        assert!((bond.dirty_from_yield(0.06, settle) - 101.502_695_050_906_5).abs() < 1e-9);
        assert!((bond.clean_from_yield(0.06, settle) - 99.977_833_172_453_5).abs() < 1e-9);
        let y = bond.yield_from_clean(99.977_833_172_453_5, settle).unwrap();
        assert!((y - 0.06).abs() < 1e-10);
    }

    #[test]
    fn par_bond_prices_to_100_on_a_coupon_date() {
        let bond = Bond { coupon: 0.045, maturity: d("2031-08-15") };
        for settle in ["2026-08-15", "2027-02-15"] {
            assert!((bond.clean_from_yield(0.045, d(settle)) - 100.0).abs() < 1e-10, "{settle}");
            assert!(bond.accrued(d(settle)).abs() < 1e-12);
        }
    }

    #[test]
    fn bootstrap_reprices_every_par_bond_and_bill() {
        let date = d("2026-11-17");
        let par: Vec<(String, f64)> = [
            ("1M", 0.0409), ("1.5M", 0.0424), ("2M", 0.0426), ("3M", 0.0433), ("4M", 0.0432), ("6M", 0.0441),
            ("1Y", 0.046), ("2Y", 0.048), ("3Y", 0.0484), ("5Y", 0.049), ("7Y", 0.0499), ("10Y", 0.051),
            ("20Y", 0.055), ("30Y", 0.0546),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), *b))
        .collect();
        let b = bootstrap(date, &par).unwrap();
        for n in &b.nodes {
            match n.spec {
                Tenor::Bill { .. } => {
                    let bill = 1.0 / b.curve.df(n.t);
                    assert!((bill - (1.0 + n.par * n.t)).abs() < 1e-12, "{}", n.label);
                }
                Tenor::Coupon { months } => {
                    assert!(par_error(&b.curve, n.par, months).abs() < 1e-12, "{}", n.label);
                    // The same test as a bond: coupon = par yield, settlement = curve date.
                    let bond = Bond { coupon: n.par, maturity: add_months(date, months, false) };
                    let theo = bond.clean_from_curve(&b.curve, date);
                    // The curve date is not a coupon date of the bond: accrued is not 0.
                    assert!((bond.dirty_from_curve(&b.curve, date) - 100.0).abs() < 1e-9, "{}", n.label);
                    let _ = theo;
                }
            }
        }
        // Discount factors fall and forward rates are positive.
        let mut last = 1.0;
        for k in 1..=360 {
            let df = b.curve.df(k as f64 / 12.0);
            assert!(df < last);
            last = df;
        }
    }

    #[test]
    fn flat_curve_prices_a_bill_and_a_bond() {
        let date = d("2026-11-17");
        let y = 0.05;
        let b = bootstrap(date, &flat_par(y)).unwrap();
        // The zero curve of a flat par curve is near flat: the bill price is close to the exponential.
        let settle = d("2026-11-18");
        let mat = d("2027-02-18");
        let price = bill_price(&b.curve, settle, mat);
        let z = b.curve.zero(0.25);
        let approx = 100.0 * (-z * 92.0 / 365.0).exp();
        assert!((price - approx).abs() < 0.01, "{price} {approx}");
        assert!(price < 100.0 && price > 98.5);
        // A bond on the curve date prices near 100 when coupon = flat par yield.
        let bond = Bond { coupon: y, maturity: d("2031-11-17") };
        assert!((bond.dirty_from_curve(&b.curve, date) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn monotone_convex_keeps_node_values_and_is_continuous() {
        let times = [0.25, 0.5, 1.0, 2.0, 5.0];
        let zeros = [0.04, 0.042, 0.047, 0.045, 0.05];
        let c = ZeroCurve::from_zeros(0, &times, &zeros);
        for (t, r) in times.iter().zip(zeros) {
            assert!((c.zero(*t) - r).abs() < 1e-12);
        }
        let mut last = c.log_df(0.001);
        for k in 2..=5000 {
            let v = c.log_df(k as f64 * 0.001);
            assert!(v >= last - 1e-12, "log df must grow: forwards >= 0");
            assert!(v - last < 0.001, "no jump");
            last = v;
        }
        // Flat forward after the last node.
        let f = c.log_df(8.0) - c.log_df(7.0);
        assert!((f - (c.log_df(7.0) - c.log_df(6.0))).abs() < 1e-12);
    }

    #[test]
    fn curve_conversions() {
        assert!((continuous_to_semi(0.05) - 0.050_630_2).abs() < 1e-6);
    }
}
