//! A smooth discount curve fitted to the market prices of Treasury notes and bonds.
//!
//! WHY THIS METHOD. The Treasury par curve is a published fit of on-the-run securities. To
//! find a rich or cheap security, we need a curve that is fitted to the securities
//! themselves and that has few parameters, so that one security cannot bend it. The
//! Nelson-Siegel-Svensson (NSS) curve has six parameters. The Federal Reserve Board
//! publishes the same family of curve (Gurkaynak, Sack and Wright, GSW; `fa forwards`),
//! so the fitted curve can be compared with an independent fit of the same kind. A
//! spline has more freedom. It would fit the local errors that we want to see, and it
//! would hide them.
//!
//! CURVE. The continuous zero rate at time `t` (years from settlement, actual/365):
//! `r(t) = b0 + b1 g1(t) + b2 (g1(t) - e^(-t/tau1)) + b3 (g2(t) - e^(-t/tau2))`, with
//! `g(x) = (1 - e^(-x))/x`. The discount factor is `exp(-r(t) t)`.
//!
//! FIT. Weighted least squares on clean prices. The weight of a security is
//! `1 / (dirty price x modified duration)`: a price error, divided by this, is a yield
//! error. So each security counts as one yield observation, and a long bond does not
//! dominate only because its price moves more (the weight 1/duration of Bliss). The
//! method is Levenberg-Marquardt with a numeric Jacobian and several starts.
//! The fit repeats with the securities that are far from the curve left out (3 passes,
//! limit = 4 robust sigma, from the median absolute deviation), so one bad price does not move the curve.

use crate::curve::{Bond, Day};
use crate::prices::Quote;
use crate::ratemodels::math::invert;

/// The Nelson-Siegel-Svensson curve. `tau1` and `tau2` are in years.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nss {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub b3: f64,
    pub tau1: f64,
    pub tau2: f64,
}

fn g(x: f64) -> f64 {
    if x < 1e-8 { 1.0 - x / 2.0 } else { (1.0 - (-x).exp()) / x }
}

impl Nss {
    /// Continuous zero rate.
    pub fn zero(&self, t: f64) -> f64 {
        let (x1, x2) = (t / self.tau1, t / self.tau2);
        self.b0 + self.b1 * g(x1) + self.b2 * (g(x1) - (-x1).exp()) + self.b3 * (g(x2) - (-x2).exp())
    }

    pub fn df(&self, t: f64) -> f64 {
        (-self.zero(t) * t).exp()
    }

    fn params(&self) -> [f64; 6] {
        [self.b0, self.b1, self.b2, self.b3, self.tau1.ln(), self.tau2.ln()]
    }

    fn from_params(p: &[f64]) -> Nss {
        Nss { b0: p[0], b1: p[1], b2: p[2], b3: p[3], tau1: p[4].clamp(-2.5, 3.7).exp(), tau2: p[5].clamp(-1.0, 3.7).exp() }
    }
}

/// One security as the fit needs it: cash flows after settlement, accrued interest, the
/// observed clean price and the weight.
#[derive(Debug, Clone)]
pub struct Inst {
    /// (time in years, amount per 100) for each cash flow after settlement.
    pub flows: Vec<(f64, f64)>,
    pub accrued: f64,
    pub obs_clean: f64,
    /// 1 / (dirty price x modified duration): a price error times this is a yield error (decimal).
    pub weight: f64,
}

impl Inst {
    pub fn model_clean(&self, c: &Nss) -> f64 {
        self.flows.iter().map(|(t, cf)| cf * c.df(*t)).sum::<f64>() - self.accrued
    }

    /// Price error as a yield error in bp (first order).
    pub fn residual_bp(&self, c: &Nss) -> f64 {
        (self.model_clean(c) - self.obs_clean) * self.weight * 1e4
    }
}

/// Cash flows, accrued interest, duration and the weight of one coupon security.
/// `None` if the market yield has no solution.
pub fn make_inst(bond: &Bond, settle: Day, obs_clean: f64) -> Option<(Inst, f64, f64)> {
    let y = bond.yield_from_clean(obs_clean, settle)?;
    let dirty = obs_clean + bond.accrued(settle);
    let h = 1e-4;
    let d_mod = (bond.dirty_from_yield(y - h, settle) - bond.dirty_from_yield(y + h, settle)) / (2.0 * h) / dirty;
    if !(d_mod.is_finite() && d_mod > 1e-4) {
        return None;
    }
    let s = bond.schedule(settle);
    let last = s.next.len() - 1;
    let half = 100.0 * bond.coupon / 2.0;
    let flows = s
        .next
        .iter()
        .enumerate()
        .map(|(i, d)| ((*d - settle) as f64 / 365.0, half + if i == last { 100.0 } else { 0.0 }))
        .collect();
    Some((Inst { flows, accrued: bond.accrued(settle), obs_clean, weight: 1.0 / (dirty * d_mod) }, y, d_mod))
}

fn sse(items: &[Inst], c: &Nss) -> f64 {
    items.iter().map(|i| i.residual_bp(c).powi(2)).sum()
}

/// Levenberg-Marquardt from one start. Gives the curve and its sum of squares.
fn lm(items: &[Inst], start: Nss, max_iter: usize) -> (Nss, f64) {
    let mut p = start.params();
    let mut cur = sse(items, &Nss::from_params(&p));
    let mut lambda = 1e-2;
    for _ in 0..max_iter {
        let base = Nss::from_params(&p);
        let e0: Vec<f64> = items.iter().map(|i| i.residual_bp(&base)).collect();
        // forward-difference Jacobian
        let mut jac = vec![[0.0; 6]; items.len()];
        for k in 0..6 {
            let h = 1e-6 * (1.0 + p[k].abs());
            let mut q = p;
            q[k] += h;
            let c = Nss::from_params(&q);
            for (r, item) in items.iter().enumerate() {
                jac[r][k] = (item.residual_bp(&c) - e0[r]) / h;
            }
        }
        let mut jtj = vec![vec![0.0; 6]; 6];
        let mut jte = [0.0; 6];
        for (row, e) in jac.iter().zip(&e0) {
            for a in 0..6 {
                jte[a] += row[a] * e;
                for b in 0..6 {
                    jtj[a][b] += row[a] * row[b];
                }
            }
        }
        let mut improved = false;
        for _ in 0..12 {
            let mut m = jtj.clone();
            for a in 0..6 {
                m[a][a] += lambda * (jtj[a][a] + 1e-9);
            }
            let Some(inv) = invert(&m) else {
                lambda *= 10.0;
                continue;
            };
            let mut q = p;
            for a in 0..6 {
                q[a] -= (0..6).map(|b| inv[a][b] * jte[b]).sum::<f64>();
            }
            let s = sse(items, &Nss::from_params(&q));
            if s.is_finite() && s < cur {
                let gain = cur - s;
                p = q;
                cur = s;
                lambda = (lambda / 3.0).max(1e-12);
                improved = true;
                if gain < 1e-10 * (1.0 + cur) {
                    return (Nss::from_params(&p), cur);
                }
                break;
            }
            lambda *= 4.0;
        }
        if !improved {
            break;
        }
    }
    (Nss::from_params(&p), cur)
}

/// Betas by least squares of the market yields on the four loadings, for given taus.
/// This is a start only: it treats the yield of a security as the zero rate at its maturity.
fn start_betas(points: &[(f64, f64)], tau1: f64, tau2: f64) -> Option<Nss> {
    let mut ata = vec![vec![0.0; 4]; 4];
    let mut aty = [0.0; 4];
    for (t, y) in points {
        let (x1, x2) = (t / tau1, t / tau2);
        let l = [1.0, g(x1), g(x1) - (-x1).exp(), g(x2) - (-x2).exp()];
        for a in 0..4 {
            aty[a] += l[a] * y;
            for b in 0..4 {
                ata[a][b] += l[a] * l[b];
            }
        }
    }
    for (a, row) in ata.iter_mut().enumerate() {
        row[a] += 1e-9;
    }
    let inv = invert(&ata)?;
    let b: Vec<f64> = (0..4).map(|a| (0..4).map(|k| inv[a][k] * aty[k]).sum()).collect();
    Some(Nss { b0: b[0], b1: b[1], b2: b[2], b3: b[3], tau1, tau2 })
}

/// Fit the curve. `points` are (years, market yield) of the securities. Start: a grid of
/// (tau1, tau2) with betas from least squares; the three best starts are improved with
/// Levenberg-Marquardt. The curve with the smallest sum of squares is the result.
pub fn fit_curve(items: &[Inst], points: &[(f64, f64)]) -> (Nss, f64) {
    try_fit_curve(items, points).expect("a start")
}

/// `fit_curve` with no panic: `None` when no start has a finite sum of squares.
pub fn try_fit_curve(items: &[Inst], points: &[(f64, f64)]) -> Option<(Nss, f64)> {
    let mut starts: Vec<(f64, Nss)> = Vec::new();
    for tau1 in [0.4, 0.8, 1.5, 2.5, 4.0] {
        for tau2 in [3.0, 6.0, 10.0, 16.0, 25.0] {
            if tau2 < 1.5 * tau1 {
                continue;
            }
            if let Some(c) = start_betas(points, tau1, tau2) {
                let v = sse(items, &c);
                if v.is_finite() {
                    starts.push((v, c));
                }
            }
        }
    }
    starts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut best: Option<(Nss, f64)> = None;
    for (_, c) in starts.into_iter().take(3) {
        let r = lm(items, c, 80);
        if best.as_ref().is_none_or(|b| r.1 < b.1) {
            best = Some(r);
        }
    }
    best
}

/// Why a security was not in the fit.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Excluded {
    pub bills: usize,
    pub tips_frn: usize,
    pub callable: usize,
    pub no_price: usize,
    pub short_life: usize,
    pub zero_coupon: usize,
    pub no_yield: usize,
    pub outliers: usize,
}

/// One coupon security with its market yield and duration.
#[derive(Debug, Clone)]
pub struct Sec {
    pub quote: Quote,
    pub bond: Bond,
    pub inst: Inst,
    pub y_obs: f64,
    pub d_mod: f64,
    pub years: f64,
    pub in_fit: bool,
}

/// The securities that can enter a fit, and the counts of the ones that cannot.
/// The minimum life is `min_years` (default 0.25 = 3 months) from settlement.
pub fn select(settle: Day, quotes: &[Quote], min_years: f64) -> (Vec<Sec>, Excluded) {
    let mut ex = Excluded::default();
    let mut out = Vec::new();
    for q in quotes {
        match q.security_type.as_str() {
            "MARKET BASED BILL" => {
                ex.bills += 1;
                continue;
            }
            "MARKET BASED NOTE" | "MARKET BASED BOND" => {}
            _ => {
                ex.tips_frn += 1;
                continue;
            }
        }
        if q.call_date.is_some() {
            ex.callable += 1;
            continue;
        }
        if q.end_of_day <= 0.0 {
            ex.no_price += 1;
            continue;
        }
        if q.coupon <= 0.0 {
            ex.zero_coupon += 1;
            continue;
        }
        let years = (q.maturity - settle) as f64 / 365.0;
        if years < min_years {
            ex.short_life += 1;
            continue;
        }
        let bond = Bond { coupon: q.coupon, maturity: q.maturity };
        let Some((inst, y_obs, d_mod)) = make_inst(&bond, settle, q.end_of_day) else {
            ex.no_yield += 1;
            continue;
        };
        out.push(Sec { quote: q.clone(), bond, inst, y_obs, d_mod, years, in_fit: true });
    }
    (out, ex)
}

pub fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { 0.5 * (v[n / 2 - 1] + v[n / 2]) }
}

/// Fit the curve to `secs` and leave out the far securities (3 passes). Sets `in_fit`.
/// Gives the curve and the weighted sum of squares of the last fit.
pub fn fit_robust(secs: &mut [Sec], ex: &mut Excluded) -> Option<(Nss, f64)> {
    if secs.len() < 12 {
        return None;
    }
    for s in secs.iter_mut() {
        s.in_fit = true;
    }
    let mut result: Option<(Nss, f64)> = None;
    for pass in 0..4 {
        let items: Vec<Inst> = secs.iter().filter(|s| s.in_fit).map(|s| s.inst.clone()).collect();
        let (curve, sse_value) = match result {
            // The first pass searches the start grid; the next passes start from the last curve.
            Some((c, _)) => lm(&items, c, 80),
            None => {
                let points: Vec<(f64, f64)> = secs.iter().filter(|s| s.in_fit).map(|s| (s.years, s.y_obs)).collect();
                fit_curve(&items, &points)
            }
        };
        result = Some((curve, sse_value));
        if pass == 3 {
            break;
        }
        let mut all: Vec<f64> = secs.iter().map(|s| s.inst.residual_bp(&curve)).collect();
        let med = median(&mut all.clone());
        let mut dev: Vec<f64> = all.iter().map(|e| (e - med).abs()).collect();
        let sigma = (1.4826 * median(&mut dev)).max(0.5);
        for (s, e) in secs.iter_mut().zip(all.drain(..)) {
            s.in_fit = (e - med).abs() <= 4.0 * sigma;
        }
    }
    ex.outliers = secs.iter().filter(|s| !s.in_fit).count();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::{add_business_days, parse_iso};

    const TRUE: Nss = Nss { b0: 0.052, b1: -0.012, b2: 0.015, b3: -0.01, tau1: 1.6, tau2: 7.5 };

    #[test]
    fn nss_by_hand() {
        // t = 0: r = b0 + b1 (the short rate).
        assert!((TRUE.zero(1e-9) - (0.052 - 0.012)).abs() < 1e-9);
        // Long end: r tends to b0.
        assert!((TRUE.zero(1e4) - 0.052).abs() < 2e-4);
        // One value by hand at t = 2: x1 = 1.25, x2 = 0.26667.
        let (x1, x2) = (1.25_f64, 2.0_f64 / 7.5);
        let g1 = (1.0 - (-x1).exp()) / x1;
        let g2 = (1.0 - (-x2).exp()) / x2;
        let r = 0.052 - 0.012 * g1 + 0.015 * (g1 - (-x1).exp()) - 0.01 * (g2 - (-x2).exp());
        assert!((TRUE.zero(2.0) - r).abs() < 1e-14);
        assert!((TRUE.df(2.0) - (-r * 2.0).exp()).abs() < 1e-14);
    }

    /// Synthetic market: 90 securities on the true curve, one coupon for each, 0.5 to 30 years.
    fn synthetic(settle: Day, noise: impl Fn(usize) -> f64) -> Vec<Quote> {
        let mut quotes = Vec::new();
        for i in 0..90_i64 {
            let months = 7 + i * 4; // 7 months to 30 years
            let maturity = crate::curve::add_months(settle, months, false);
            let coupon = 0.03 + 0.0005 * (i % 9) as f64;
            let bond = Bond { coupon, maturity };
            // Price on the true curve.
            let s = bond.schedule(settle);
            let last = s.next.len() - 1;
            let dirty: f64 = s
                .next
                .iter()
                .enumerate()
                .map(|(k, d)| (100.0 * coupon / 2.0 + if k == last { 100.0 } else { 0.0 }) * TRUE.df((*d - settle) as f64 / 365.0))
                .sum();
            quotes.push(Quote {
                cusip: format!("S{i:04}"),
                security_type: if months > 120 { "MARKET BASED BOND" } else { "MARKET BASED NOTE" }.into(),
                coupon,
                maturity,
                call_date: None,
                buy: 0.0,
                sell: 0.0,
                end_of_day: dirty - bond.accrued(settle) + noise(i as usize),
            });
        }
        quotes
    }

    #[test]
    fn recovers_known_parameters_from_synthetic_prices() {
        let settle = add_business_days(parse_iso("2026-11-17").unwrap(), 1);
        let quotes = synthetic(settle, |_| 0.0);
        let (mut secs, mut ex) = select(settle, &quotes, 0.25);
        assert_eq!(secs.len(), 90);
        let (curve, _) = fit_robust(&mut secs, &mut ex).unwrap();
        // The curve (not each parameter: tau1 and tau2 can trade off) must match to 0.01 bp.
        for t in [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0] {
            assert!((curve.zero(t) - TRUE.zero(t)).abs() < 1e-6, "t={t}: {} vs {}", curve.zero(t), TRUE.zero(t));
        }
        let worst = secs.iter().map(|s| s.inst.residual_bp(&curve).abs()).fold(0.0, f64::max);
        assert!(worst < 0.01, "worst residual {worst} bp");
        assert_eq!(ex.outliers, 0);
    }

    #[test]
    fn one_bad_price_is_left_out_and_does_not_move_the_curve() {
        let settle = add_business_days(parse_iso("2026-11-17").unwrap(), 1);
        // Security 40 is 1.2 points too high; the others have a small deterministic noise (about 1 bp).
        let quotes = synthetic(settle, |i| if i == 40 { 1.2 } else { 0.002 * (((i * 7919) % 11) as f64 - 5.0) / 5.0 });
        let (mut secs, mut ex) = select(settle, &quotes, 0.25);
        let (curve, _) = fit_robust(&mut secs, &mut ex).unwrap();
        assert!(!secs[40].in_fit);
        assert_eq!(ex.outliers, secs.iter().filter(|s| !s.in_fit).count());
        // The bad price shows as a large residual: price error 1.2 -> a yield error of more than 10 bp.
        let r = secs[40].inst.residual_bp(&curve);
        assert!(r.abs() > 10.0, "residual {r}");
        for t in [1.0, 5.0, 10.0, 30.0] {
            assert!((curve.zero(t) - TRUE.zero(t)).abs() < 5e-4, "t={t}");
        }
    }

    #[test]
    fn weight_turns_price_error_into_yield_error() {
        let settle = add_business_days(parse_iso("2026-11-17").unwrap(), 1);
        let bond = Bond { coupon: 0.04, maturity: parse_iso("2036-11-15").unwrap() };
        let p = 97.0;
        let (inst, y, d) = make_inst(&bond, settle, p).unwrap();
        // A price 0.1 higher is a yield lower by about 0.1 / (dirty x D). Check against the yield solver.
        let y2 = bond.yield_from_clean(p + 0.1, settle).unwrap();
        let expect_bp = (y - y2) * 1e4;
        let got = (0.1 * inst.weight * 1e4).abs();
        assert!((got - expect_bp).abs() / expect_bp < 0.01, "{got} vs {expect_bp}");
        assert!(d > 7.0 && d < 9.0);
    }

    #[test]
    fn select_counts_each_exclusion() {
        let settle = add_business_days(parse_iso("2026-11-17").unwrap(), 1);
        let text = "\
912797AA1,MARKET BASED BILL,0.0,02/18/2027,,0.0,98.9,98.9
912828BB2,MARKET BASED NOTE,0.04,11/15/2030,,0.0,100.0,100.0
912828CC3,MARKET BASED BOND,0.05,11/15/2040,11/15/2035,0.0,101.0,101.0
912828DD4,MARKET BASED NOTE,0.03,02/15/2027,,0.0,100.0,100.0
912828EE5,TIPS,0.01,11/15/2030,,0.0,100.0,100.0
912828FF6,MARKET BASED NOTE,0.03,11/15/2029,,0.0,0.0,0.0
";
        let (secs, ex) = select(settle, &crate::prices::parse_csv(text), 0.25);
        assert_eq!(secs.len(), 1);
        assert_eq!(ex, Excluded { bills: 1, tips_frn: 1, callable: 1, no_price: 1, short_life: 1, ..Default::default() });
    }
}
