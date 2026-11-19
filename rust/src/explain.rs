//! `fa recon explain`: what explains the difference of a Treasury note or bond from the
//! fitted curve (`richcheap.rs`)?
//!
//! A cross-section regression (ordinary least squares) of the residual in bp on features
//! from public data (`refdata.rs`): coupon against the par rate, age, on-the-run flag,
//! SOMA share, stripped share, size, original term, futures deliverability. The part that
//! the features do not explain is the residual after the regression. A security with a
//! large residual after the regression is the one to study. Nothing here is arbitrage.
//!
//! SIGN. `res_bp = observed yield - fitted yield`. Positive = cheap, negative = rich.

use crate::curve::{self, Bond, Day};
use crate::history;
use crate::prices::Quote;
use crate::refdata::{self, Auction, Fetcher, Issue, Soma, Stripped};
use crate::richcheap::{self, Analysis, Row};
use crate::ratemodels::math::invert;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// Futures delivery baskets
// ---------------------------------------------------------------------------

/// The CME Treasury futures. Rules (see `docs/ARCHITECTURE.md` for the sources):
/// - ZT 2-Year: notes, original term at most 5 years 3 months; remaining term at least 1 year
///   9 months from the first day of the delivery month and at most 2 years from the last day.
/// - ZF 5-Year: notes, original term at most 5 years 3 months; remaining term at least 4 years
///   2 months from the first day of the delivery month.
/// - ZN 10-Year: notes, remaining term at least 6 years 6 months and less than 8 years.
/// - TN Ultra 10-Year: original-issue 10-year notes, remaining term 9 years 5 months to 10 years.
/// - ZB Treasury bond: bonds, remaining term at least 15 years and less than 25 years.
/// - UB Ultra Treasury bond: bonds, remaining term at least 25 years.
///
/// Remaining term is counted from the first day of the delivery month and rounded down: to
/// one month for ZT and ZF, to three months for ZN, ZB and UB (TN: months).
pub const CONTRACTS: [&str; 6] = ["ZT", "ZF", "ZN", "TN", "ZB", "UB"];

/// Whole months from `from` to `to` (rounded down).
pub fn months_between(from: Day, to: Day) -> i64 {
    let (y1, m1, d1) = curve::civil_from_days(from);
    let (y2, m2, d2) = curve::civil_from_days(to);
    (y2 - y1) * 12 + (m2 - m1) - if d2 < d1 { 1 } else { 0 }
}

/// The first day of the next quarterly delivery month (Mar, Jun, Sep, Dec) after `date`.
pub fn front_delivery(date: Day) -> Day {
    let (y, m, _) = curve::civil_from_days(date);
    for k in 0..12 {
        let mm = m + k;
        let (yy, mo) = (y + (mm - 1) / 12, (mm - 1) % 12 + 1);
        if mo % 3 == 0 {
            let first = curve::days_from_civil(yy, mo, 1);
            if first > date {
                return first;
            }
        }
    }
    date
}

/// The contracts that accept the security, for the delivery month that starts on `first`.
pub fn deliverable(kind: &str, original_term: &str, maturity: Day, first: Day) -> Vec<&'static str> {
    let (y, m, _) = curve::civil_from_days(first);
    let last = curve::days_from_civil(y, m, curve::days_in_month(y, m));
    let rem = months_between(first, maturity);
    let rem3 = rem - rem % 3;
    let orig = refdata::term_months(original_term);
    let mut out = Vec::new();
    if kind == "note" {
        if orig.is_some_and(|o| o <= 63) && rem >= 21 && months_between(last, maturity) <= 24 {
            out.push("ZT");
        }
        if orig.is_some_and(|o| o <= 63) && rem >= 50 {
            out.push("ZF");
        }
        if rem3 >= 78 && rem3 < 96 {
            out.push("ZN");
        }
        if orig == Some(120) && (113..=120).contains(&rem) {
            out.push("TN");
        }
    }
    if kind == "bond" {
        if (180..300).contains(&rem3) {
            out.push("ZB");
        }
        if rem3 >= 300 {
            out.push("UB");
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Ordinary least squares with several variables
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Reg {
    /// Names; the first is the intercept.
    pub names: Vec<String>,
    pub beta: Vec<f64>,
    /// Heteroskedasticity-robust standard errors (HC1).
    pub se: Vec<f64>,
    /// Classical standard errors.
    pub se_ols: Vec<f64>,
    pub r2: f64,
    pub adj_r2: f64,
    pub n: usize,
    pub sigma: f64,
    pub fitted: Vec<f64>,
    pub resid: Vec<f64>,
}

/// Regress `y` on the columns `x` (one `Vec` for each row of data, without the intercept).
/// A column that is zero in every row is dropped and reported in `dropped`.
pub fn ols_multi(names: &[&str], x: &[Vec<f64>], y: &[f64]) -> Result<(Reg, Vec<String>), String> {
    let n = y.len();
    if n != x.len() {
        return Err("regression: x and y have different lengths".into());
    }
    if n == 0 {
        return Err("regression: no security has all the features".into());
    }
    let p = names.len();
    let mut keep: Vec<usize> = Vec::new();
    let mut dropped = Vec::new();
    for j in 0..p {
        if x.iter().all(|r| r[j] == 0.0) {
            dropped.push(names[j].to_string());
        } else {
            keep.push(j);
        }
    }
    let k = keep.len() + 1;
    if n <= k {
        return Err(format!("regression: {n} rows for {k} parameters"));
    }
    let row = |i: usize| -> Vec<f64> {
        let mut v = vec![1.0];
        v.extend(keep.iter().map(|&j| x[i][j]));
        v
    };
    let rows: Vec<Vec<f64>> = (0..n).map(row).collect();
    let mut xtx = vec![vec![0.0; k]; k];
    let mut xty = vec![0.0; k];
    for (r, yi) in rows.iter().zip(y) {
        for a in 0..k {
            xty[a] += r[a] * yi;
            for b in 0..k {
                xtx[a][b] += r[a] * r[b];
            }
        }
    }
    let inv = invert(&xtx).ok_or("regression: the features are collinear (singular matrix)")?;
    let beta: Vec<f64> = (0..k).map(|a| (0..k).map(|b| inv[a][b] * xty[b]).sum()).collect();
    let fitted: Vec<f64> = rows.iter().map(|r| r.iter().zip(&beta).map(|(a, b)| a * b).sum()).collect();
    let resid: Vec<f64> = y.iter().zip(&fitted).map(|(a, b)| a - b).collect();
    let rss: f64 = resid.iter().map(|e| e * e).sum();
    let my = y.iter().sum::<f64>() / n as f64;
    let tss: f64 = y.iter().map(|v| (v - my) * (v - my)).sum();
    let r2 = if tss > 0.0 { 1.0 - rss / tss } else { f64::NAN };
    let adj_r2 = 1.0 - (1.0 - r2) * (n as f64 - 1.0) / (n as f64 - k as f64);
    let sigma2 = rss / (n - k) as f64;
    // HC1: (X'X)^-1 (sum e^2 x x') (X'X)^-1 * n/(n-k)
    let mut meat = vec![vec![0.0; k]; k];
    for (r, e) in rows.iter().zip(&resid) {
        for a in 0..k {
            for b in 0..k {
                meat[a][b] += e * e * r[a] * r[b];
            }
        }
    }
    let scale = n as f64 / (n - k) as f64;
    let mut se = vec![0.0; k];
    for a in 0..k {
        let mut v = 0.0;
        for c in 0..k {
            for d in 0..k {
                v += inv[a][c] * meat[c][d] * inv[d][a];
            }
        }
        se[a] = (v * scale).sqrt();
    }
    let se_ols: Vec<f64> = (0..k).map(|a| (inv[a][a] * sigma2).sqrt()).collect();
    let mut out_names = vec!["intercept".to_string()];
    out_names.extend(keep.iter().map(|&j| names[j].to_string()));
    Ok((Reg { names: out_names, beta, se, se_ols, r2, adj_r2, n, sigma: sigma2.sqrt(), fitted, resid }, dropped))
}

// ---------------------------------------------------------------------------
// The data of each security
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Rec {
    pub id: String,
    pub kind: &'static str,
    pub coupon: f64,
    pub maturity: Day,
    pub years: f64,
    pub group: &'static str,
    pub in_fit: bool,
    pub candidate: bool,
    pub cost_bp: f64,
    pub z: f64,
    /// Residual of the date (bp, + = cheap).
    pub res_bp: f64,
    /// Mean residual over the cached dates and the number of dates.
    pub res_mean: Option<(f64, usize)>,
    pub y_obs: f64,
    /// Coupon minus the par rate of the same maturity on the fitted curve (percentage points).
    pub coupon_gap_pp: f64,
    pub issue: Option<Issue>,
    pub age: Option<f64>,
    pub on_the_run: bool,
    pub soma_pct: Option<f64>,
    pub strip_pct: Option<f64>,
    pub outstanding_bn: Option<f64>,
    /// Original 30-year bond with 5 to 15 years left.
    pub old30: bool,
    /// Original 30-year bond with less than 5 years left.
    pub old30_short: bool,
    pub orig20: bool,
    pub deliverable: Vec<&'static str>,
}

/// The coupon rate at which a bond of this maturity prices at par on the curve (first
/// coupon period taken as a full period).
pub fn par_rate(nss: &crate::fit::Nss, settle: Day, maturity: Day) -> f64 {
    let b = Bond { coupon: 0.0, maturity };
    let s = b.schedule(settle);
    let dfs: Vec<f64> = s.next.iter().map(|d| nss.df((*d - settle) as f64 / 365.0)).collect();
    let last = dfs.last().copied().unwrap_or(1.0);
    2.0 * (1.0 - last) / dfs.iter().sum::<f64>().max(1e-12)
}

pub struct Inputs<'a> {
    pub soma: &'a BTreeMap<String, Soma>,
    pub mspd: &'a BTreeMap<String, Stripped>,
    pub issues: &'a BTreeMap<String, Issue>,
    pub on_the_run: &'a BTreeMap<String, String>,
    pub means: &'a BTreeMap<String, (f64, usize)>,
}

/// One record for each note and bond of 6 months or more.
pub fn build(a: &Analysis, inp: &Inputs, date: Day) -> Vec<Rec> {
    let first = front_delivery(date);
    a.rows
        .iter()
        .filter(|r| r.kind != "bill" && r.years >= 0.5 && r.coupon.is_some())
        .map(|r| rec(a, r, inp, date, first))
        .collect()
}

fn rec(a: &Analysis, r: &Row, inp: &Inputs, date: Day, first: Day) -> Rec {
    let coupon = r.coupon.unwrap_or(0.0);
    let issue = inp.issues.get(&r.id).cloned();
    let age = issue.as_ref().and_then(|i| curve::parse_iso(&i.first_issue)).map(|d| (date - d) as f64 / 365.25);
    let on_the_run = issue.as_ref().is_some_and(|i| inp.on_the_run.get(&i.original_term) == Some(&r.id));
    let m = inp.mspd.get(&r.id);
    let orig = issue.as_ref().map(|i| i.original_term.clone()).unwrap_or_default();
    Rec {
        id: r.id.clone(),
        kind: r.kind,
        coupon,
        maturity: r.maturity,
        years: r.years,
        group: r.group,
        in_fit: r.in_fit,
        candidate: r.survives(),
        cost_bp: r.cost_bp,
        z: r.z,
        res_bp: r.res_bp,
        res_mean: inp.means.get(&r.id).copied(),
        y_obs: r.y_obs,
        coupon_gap_pp: (coupon - par_rate(&a.nss, a.settle, r.maturity)) * 100.0,
        age,
        on_the_run,
        soma_pct: m.map(|_| inp.soma.get(&r.id).map_or(0.0, |s| s.share * 100.0)),
        strip_pct: m.filter(|s| s.outstanding > 0.0).map(|s| s.stripped / s.outstanding * 100.0),
        outstanding_bn: m.map(|s| s.outstanding / 1e9),
        old30: orig == "30-Year" && r.kind == "bond" && (5.0..15.0).contains(&r.years),
        old30_short: orig == "30-Year" && r.kind == "bond" && r.years < 5.0,
        orig20: orig == "20-Year",
        deliverable: if orig.is_empty() { Vec::new() } else { deliverable(r.kind, &orig, r.maturity, first) },
        issue,
    }
}

/// The feature row of a record, or `None` if a feature is missing.
fn features(r: &Rec, with_years: bool) -> Option<Vec<f64>> {
    let mut v = vec![
        r.coupon_gap_pp,
        r.age?,
        r.on_the_run as u8 as f64,
        r.soma_pct?,
        r.strip_pct?,
        r.outstanding_bn?.max(1e-6).ln(),
        r.old30 as u8 as f64,
        r.old30_short as u8 as f64,
        r.orig20 as u8 as f64,
        (!r.deliverable.is_empty()) as u8 as f64,
    ];
    if with_years {
        v.push(r.years);
    }
    Some(v)
}

pub const FEATURES: [&str; 10] = [
    "coupon minus par rate (pp)",
    "age since first issue (y)",
    "on-the-run (0/1)",
    "SOMA share (%)",
    "stripped share (%)",
    "ln amount outstanding ($bn)",
    "original 30-year, 5-15y left",
    "original 30-year, <5y left",
    "original 20-year",
    "futures deliverable (0/1)",
];

pub struct Spec {
    pub label: String,
    pub reg: Reg,
    pub dropped: Vec<String>,
    /// Record index (into the list given) of each row of the regression.
    pub index: Vec<usize>,
}

/// Regress the residual on the features. `mean`: use the 50-day mean residual and keep only
/// securities that have one.
/// `zero`: indexes of features that are left out (set to 0, so the regression drops them).
pub fn run_spec(recs: &[Rec], mean: bool, with_years: bool, label: &str, zero: &[usize]) -> Result<Spec, String> {
    let mut names: Vec<&str> = FEATURES.to_vec();
    if with_years {
        names.push("years to maturity");
    }
    let (mut x, mut y, mut index) = (Vec::new(), Vec::new(), Vec::new());
    for (i, r) in recs.iter().enumerate() {
        let dep = if mean { r.res_mean.map(|m| m.0) } else { Some(r.res_bp) };
        if let (Some(mut f), Some(d)) = (features(r, with_years), dep) {
            for &z in zero {
                f[z] = 0.0;
            }
            if d.is_finite() {
                x.push(f);
                y.push(d);
                index.push(i);
            }
        }
    }
    let (reg, dropped) = ols_multi(&names, &x, &y)?;
    Ok(Spec { label: label.to_string(), reg, dropped, index })
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

fn print_spec(out: &mut String, s: &Spec) {
    let r = &s.reg;
    let _ = writeln!(out, "\n{}  (n = {}, R2 = {:.3}, adjusted R2 = {:.3}, residual sd = {:.2} bp)", s.label, r.n, r.r2, r.adj_r2, r.sigma);
    let _ = writeln!(out, "{:<30} {:>9} {:>8} {:>7} {:>7}", "feature", "coef bp", "se(HC1)", "t", "t(OLS)");
    for j in 0..r.beta.len() {
        let _ = writeln!(
            out,
            "{:<30} {:>+9.3} {:>8.3} {:>+7.2} {:>+7.2}",
            r.names[j],
            r.beta[j],
            r.se[j],
            r.beta[j] / r.se[j],
            r.beta[j] / r.se_ols[j]
        );
    }
    if !s.dropped.is_empty() {
        let _ = writeln!(out, "(dropped, all zero: {})", s.dropped.join(", "));
    }
}

fn opt(v: Option<f64>, prec: usize) -> String {
    v.map_or("-".to_string(), |x| format!("{x:.prec$}"))
}

/// The candidates of the date with the residual before and after the regression.
fn print_candidates(out: &mut String, recs: &[Rec], spec: &Spec) {
    let mut after: BTreeMap<usize, f64> = BTreeMap::new();
    let mut fitted: BTreeMap<usize, f64> = BTreeMap::new();
    for (k, &i) in spec.index.iter().enumerate() {
        after.insert(i, spec.reg.resid[k]);
        fitted.insert(i, spec.reg.fitted[k]);
    }
    let mut c: Vec<usize> = (0..recs.len()).filter(|&i| recs[i].candidate).collect();
    c.sort_by(|&a, &b| recs[b].res_bp.abs().total_cmp(&recs[a].res_bp.abs()));
    let _ = writeln!(out, "\nCANDIDATES OF THE DATE BEFORE AND AFTER THE REGRESSION (bp; + cheap, - rich; cost = one leg)");
    let _ = writeln!(out, "{:<10} {:<5} {:>6} {:>10} {:>6} {:>8} {:>8} {:>8} {:>5} {:<18} {}", "cusip", "type", "cpn %", "maturity", "cost", "before", "explained", "after", "flag", "group", "contracts");
    for i in c {
        let r = &recs[i];
        let (f, a) = (fitted.get(&i).copied(), after.get(&i).copied());
        let flag = match a {
            Some(a) if a.abs() > r.cost_bp.max(2.0) => "STAYS",
            Some(_) => "gone",
            None => "n/a",
        };
        let _ = writeln!(
            out,
            "{:<10} {:<5} {:>6.3} {:>10} {:>6.1} {:>+8.1} {:>8} {:>8} {:>5} {:<18} {}",
            r.id,
            r.kind,
            r.coupon * 100.0,
            curve::iso(r.maturity),
            r.cost_bp,
            r.res_bp,
            f.map_or("-".to_string(), |v| format!("{v:+.1}")),
            a.map_or("-".to_string(), |v| format!("{v:+.1}")),
            flag,
            r.group,
            if r.deliverable.is_empty() { "-".to_string() } else { r.deliverable.join(",") }
        );
    }
    let _ = writeln!(out, "STAYS = the residual after the regression is larger than the cost and than 2 bp. It is not explained by the features; it is not arbitrage.");
}

/// The securities with the largest residual after the regression (not only the candidates).
fn print_largest_after(out: &mut String, recs: &[Rec], spec: &Spec, n: usize) {
    let mut v: Vec<(usize, f64)> = spec.index.iter().zip(&spec.reg.resid).map(|(&i, &e)| (i, e)).collect();
    v.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    let _ = writeln!(out, "\nLARGEST RESIDUALS AFTER THE REGRESSION: {}", spec.label);
    let _ = writeln!(out, "{:<10} {:<5} {:>6} {:>10} {:>8} {:>8} {:>6} {:<18}", "cusip", "type", "cpn %", "maturity", "before", "after", "cost", "group");
    for (i, e) in v.into_iter().take(n) {
        let r = &recs[i];
        let _ = writeln!(out, "{:<10} {:<5} {:>6.3} {:>10} {:>+8.1} {:>+8.1} {:>6.1} {:<18}", r.id, r.kind, r.coupon * 100.0, curve::iso(r.maturity), r.res_bp, e, r.cost_bp, r.group);
    }
}

/// Securities with a SOMA share of 60 % or more, by maturity: is the share alone the cause?
fn print_soma_heavy(out: &mut String, recs: &[Rec], spec: &Spec) {
    let mut after: BTreeMap<usize, f64> = BTreeMap::new();
    for (k, &i) in spec.index.iter().enumerate() {
        after.insert(i, spec.reg.resid[k]);
    }
    let mut v: Vec<usize> = (0..recs.len()).filter(|&i| recs[i].soma_pct.is_some_and(|s| s >= 60.0)).collect();
    v.sort_by_key(|&i| recs[i].maturity);
    let _ = writeln!(out, "\nSECURITIES WITH A SOMA SHARE OF 60 % OR MORE ({}), BY MATURITY: the share is about the same (60 to 70 %), the residual is not", v.len());
    let _ = writeln!(out, "{:<10} {:<5} {:>6} {:>10} {:>5} {:>6} {:>8} {:>8} {:>8} {:<18}", "cusip", "type", "cpn %", "maturity", "yrs", "SOMA%", "res bp", "mean bp", "after", "group");
    for i in v {
        let r = &recs[i];
        let _ = writeln!(out, "{:<10} {:<5} {:>6.3} {:>10} {:>5.1} {:>6.1} {:>+8.1} {:>8} {:>8} {:<18}", r.id, r.kind, r.coupon * 100.0, curve::iso(r.maturity), r.years, r.soma_pct.unwrap_or(0.0), r.res_bp, r.res_mean.map_or("-".to_string(), |m| format!("{:+.1}", m.0)), after.get(&i).map_or("-".to_string(), |x| format!("{x:+.1}")), r.group);
    }
}

/// The original 30-year bonds that mature in 2036 to 2039 and the note of the nearest maturity.
fn print_old_bonds(out: &mut String, a: &Analysis, recs: &[Rec], spec: &Spec, rates: Option<[f64; 3]>) {
    let lo = curve::days_from_civil(2036, 1, 1);
    let hi = curve::days_from_civil(2039, 12, 31);
    let mut after: BTreeMap<usize, f64> = BTreeMap::new();
    for (k, &i) in spec.index.iter().enumerate() {
        after.insert(i, spec.reg.resid[k]);
    }
    let bonds: Vec<usize> = (0..recs.len()).filter(|&i| recs[i].kind == "bond" && recs[i].old30 && recs[i].maturity >= lo && recs[i].maturity <= hi).collect();
    let _ = writeln!(out, "\nTHE OLD 30-YEAR BONDS THAT MATURE IN 2036 TO 2039 ({})", bonds.len());
    let _ = writeln!(out, "{:<10} {:>6} {:>10} {:>5} {:>6} {:>7} {:>8} {:>8} {:>7} {:>8} {:>8} {:>7} {:>9}", "cusip", "cpn %", "maturity", "yrs", "SOMA%", "strip%", "amt $bn", "price", "res bp", "mean bp", "after", "orig", "carry bp");
    for &i in &bonds {
        let r = &recs[i];
        let row = a.rows.iter().find(|x| x.id == r.id);
        let carry = rates.map(|q| (r.y_obs * 100.0 - q[2]) * 100.0);
        let _ = writeln!(
            out,
            "{:<10} {:>6.3} {:>10} {:>5.1} {:>6} {:>7} {:>8} {:>8.3} {:>+7.1} {:>8} {:>8} {:>7} {:>9}",
            r.id,
            r.coupon * 100.0,
            curve::iso(r.maturity),
            r.years,
            opt(r.soma_pct, 1),
            opt(r.strip_pct, 1),
            opt(r.outstanding_bn, 1),
            row.map_or(f64::NAN, |x| x.obs_clean),
            r.res_bp,
            r.res_mean.map_or("-".to_string(), |m| format!("{:+.1}", m.0)),
            after.get(&i).map_or("-".to_string(), |v| format!("{v:+.1}")),
            r.issue.as_ref().map_or("-", |i| i.original_term.as_str()),
            carry.map_or("-".to_string(), |c| format!("{c:+.0}")),
        );
    }
    let _ = writeln!(out, "carry bp = observed yield minus TGCR (general collateral repo) of the date: the gain of a long position that is financed in repo, before any special rate.");
    // Comparison with the note of the nearest maturity.
    let _ = writeln!(out, "\nEACH BOND AGAINST THE NOTE OF THE NEAREST MATURITY (notes that are not on-the-run; gap m = months between the two maturities; there is no note after 2036-11)");
    let _ = writeln!(
        out,
        "{:<10} {:<10} {:>10} {:>5} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "bond", "note", "note mat", "gap m", "cpn diff", "dP obs", "dP curve", "dP extra", "dY obs bp", "dY crv bp", "extra bp"
    );
    for &i in &bonds {
        let r = &recs[i];
        let note = recs
            .iter()
            .filter(|n| n.kind == "note" && !n.on_the_run && n.issue.is_some())
            .min_by_key(|n| n.maturity.abs_diff(r.maturity));
        let (Some(note), Some(rb)) = (note, a.rows.iter().find(|x| x.id == r.id)) else {
            let _ = writeln!(out, "{:<10} no note", r.id);
            continue;
        };
        let Some(rn) = a.rows.iter().find(|x| x.id == note.id) else { continue };
        let d_obs = rb.obs_clean - rn.obs_clean;
        let d_crv = rb.model_clean - rn.model_clean;
        let dy_obs = (rb.y_obs - rn.y_obs) * 1e4;
        let dy_crv = (rb.y_model - rn.y_model) * 1e4;
        let _ = writeln!(
            out,
            "{:<10} {:<10} {:>10} {:>5} {:>+9.3} {:>+9.3} {:>+9.3} {:>+9.3} {:>+9.1} {:>+9.1} {:>+9.1}",
            r.id,
            note.id,
            curve::iso(note.maturity),
            months_between(note.maturity.min(r.maturity), note.maturity.max(r.maturity)),
            (r.coupon - note.coupon) * 100.0,
            d_obs,
            d_crv,
            d_obs - d_crv,
            dy_obs,
            dy_crv,
            dy_obs - dy_crv
        );
    }
    let _ = writeln!(out, "dP = bond price minus note price (clean, per 100). dP curve = the same difference from the fitted curve (it contains the coupon difference). extra = observed minus curve.");
}

/// Does the curve itself carry the effect? The nine bonds are in or near the fit set, and the
/// notes end in 2036. Fit the curve again without them and without all bonds of a SOMA share
/// of 60 % or more, and measure the same residual.
fn print_sensitivity(out: &mut String, a: &Analysis, quotes: &[Quote], recs: &[Rec], date: &str) {
    let lo = curve::days_from_civil(2036, 1, 1);
    let hi = curve::days_from_civil(2039, 12, 31);
    let nine: Vec<&Rec> = recs.iter().filter(|r| r.kind == "bond" && r.old30 && r.maturity >= lo && r.maturity <= hi).collect();
    let old: std::collections::BTreeSet<&str> = recs.iter().filter(|r| r.old30 || r.old30_short).map(|r| r.id.as_str()).collect();
    let heavy: std::collections::BTreeSet<&str> = recs.iter().filter(|r| r.old30 || r.old30_short || r.soma_pct.is_some_and(|s| s >= 60.0)).map(|r| r.id.as_str()).collect();
    let _ = writeln!(out, "\nDOES THE CURVE CARRY THE EFFECT? The curve is fitted again without a group; residual of the nine bonds on that curve (bp)");
    let mut cols: Vec<(String, Option<(Analysis, Vec<Option<f64>>)>)> = Vec::new();
    for (name, set) in [("without all original 30-year bonds", &old), ("and without every SOMA share >= 60 %", &heavy)] {
        let kept: Vec<Quote> = quotes.iter().filter(|q| !set.contains(q.cusip.as_str())).cloned().collect();
        let res = richcheap::analyse(date, a.settle, &kept).ok().map(|b| {
            let r: Vec<Option<f64>> = nine.iter().map(|x| quotes.iter().find(|q| q.cusip == x.id).and_then(|q| richcheap::residual_bp(&b.nss, a.settle, q))).collect();
            (b, r)
        });
        cols.push((format!("{name} ({} securities left out)", set.len()), res));
    }
    let _ = writeln!(out, "{:<10} {:>10} {:>10} {:>10}", "cusip", "base", "fit A", "fit B");
    for (k, r) in nine.iter().enumerate() {
        let f = |c: usize| cols[c].1.as_ref().and_then(|x| x.1[k]).map_or("-".to_string(), |v| format!("{v:+.1}"));
        let _ = writeln!(out, "{:<10} {:>+10.1} {:>10} {:>10}", r.id, r.res_bp, f(0), f(1));
    }
    for (c, (name, res)) in cols.iter().enumerate() {
        let _ = writeln!(out, "fit {}: {name}; fit RMSE {}", ["A", "B"][c], res.as_ref().map_or("-".to_string(), |x| format!("{:.2} bp", x.0.rmse_all)));
    }
}

pub const HELP: &str = "\
fa recon explain [--date YYYY-MM-DD] [--out FILE]

Explain the difference of each note and bond from the fitted curve. A cross-section
regression (ordinary least squares) of the residual in bp on: coupon minus the par rate,
age since first issue, on-the-run flag, SOMA share, stripped share, log of the amount
outstanding, original term group and a futures-deliverable flag. It is made for the
residual of the date and for the mean residual over all cached dates up to the date.
It gives the residual of each candidate before and after the regression, and a table of
the old 30-year bonds that mature in 2036 to 2039.

Data (cached in data/raw; a missing file is downloaded, 2 seconds between requests):
  New York Fed: SOFR, BGCR, TGCR and SOMA holdings by CUSIP.
  Fiscal Data: Monthly Statement of the Public Debt table 5 and the auction records.

  --date  A cached price date. Default: the latest.
  --out   The CSV file of the securities. Default: data/recon/explain_<date>.csv.";

pub fn csv_header() -> String {
    "id,type,coupon,maturity,years,group,in_fit,candidate,cost_bp,z,res_bp,res_mean_bp,n_dates,coupon_gap_pp,first_issue,original_term,auctions,age_y,on_the_run,soma_pct,stripped_pct,outstanding_bn,old30,old30_short,orig20,deliverable,res_after_bp,res_mean_after_bp".to_string()
}

fn csv_rows(recs: &[Rec], latest: &Spec, mean: &Spec) -> String {
    let mut after: BTreeMap<usize, f64> = BTreeMap::new();
    for (k, &i) in latest.index.iter().enumerate() {
        after.insert(i, latest.reg.resid[k]);
    }
    let mut after_mean: BTreeMap<usize, f64> = BTreeMap::new();
    for (k, &i) in mean.index.iter().enumerate() {
        after_mean.insert(i, mean.reg.resid[k]);
    }
    let mut s = csv_header();
    s.push('\n');
    let o = |v: Option<f64>| v.map_or(String::new(), |x| format!("{x:.4}"));
    for (i, r) in recs.iter().enumerate() {
        let _ = writeln!(
            s,
            "{},{},{:.6},{},{:.3},{},{},{},{:.3},{:.3},{:.3},{},{},{:.4},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            r.id,
            r.kind,
            r.coupon,
            curve::iso(r.maturity),
            r.years,
            r.group,
            r.in_fit,
            r.candidate,
            r.cost_bp,
            r.z,
            r.res_bp,
            o(r.res_mean.map(|m| m.0)),
            r.res_mean.map_or(String::new(), |m| m.1.to_string()),
            r.coupon_gap_pp,
            r.issue.as_ref().map_or("", |x| x.first_issue.as_str()),
            r.issue.as_ref().map_or("", |x| x.original_term.as_str()),
            r.issue.as_ref().map_or(String::new(), |x| x.auctions.to_string()),
            o(r.age),
            r.on_the_run,
            o(r.soma_pct),
            o(r.strip_pct),
            o(r.outstanding_bn),
            r.old30,
            r.old30_short,
            r.orig20,
            r.deliverable.join("|"),
            o(after.get(&i).copied()),
            o(after_mean.get(&i).copied()),
        );
    }
    s
}

pub fn run_explain(fa: &crate::Fa, args: &[String]) -> Result<(), String> {
    let t0 = std::time::Instant::now();
    let (mut date, mut out) = (None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--date" => date = Some(it.next().ok_or("--date needs a value")?.clone()),
            "--out" => out = Some(std::path::PathBuf::from(it.next().ok_or("--out needs a value")?)),
            "--help" | "-h" => {
                println!("{HELP}");
                return Ok(());
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    let cache = fa.repo.join("data/raw");
    let date = match date {
        Some(d) => d,
        None => richcheap::latest_cached(&cache).ok_or("no cached price file in data/raw/prices")?,
    };
    let day = curve::parse_iso(&date).ok_or_else(|| format!("'{date}' is not a date (YYYY-MM-DD)"))?;
    let settle = curve::add_business_days(day, 1);
    let quotes = richcheap::cached_quotes(&cache, &date)?;
    let a = richcheap::analyse(&date, settle, &quotes)?;

    // The residual of each security over the cached dates up to the date.
    let dates = history::cached_dates(&cache, None, Some(&date));
    let analyses: Vec<Analysis> = dates
        .par_iter()
        .filter_map(|d| {
            let q = richcheap::cached_quotes(&cache, d).ok()?;
            richcheap::analyse(d, curve::add_business_days(curve::parse_iso(d)?, 1), &q).ok()
        })
        .collect();
    let panel = history::build_panel(&analyses);
    let min_obs = (analyses.len() * 4 / 5).max(10);
    let means: BTreeMap<String, (f64, usize)> = history::security_means(&panel, min_obs).into_iter().map(|(id, n, m, _, _)| (id, (m, n))).collect();

    // Public data.
    let mut f = Fetcher::new(&cache);
    let mut notes: Vec<String> = Vec::new();
    let first_date = dates.first().cloned().unwrap_or_else(|| date.clone());
    let rates = match refdata::secured_rates(&mut f, &first_date, &date) {
        Ok(r) => Some(r),
        Err(e) => {
            notes.push(format!("secured rates: {e}"));
            None
        }
    };
    let (soma_asof, soma) = match refdata::soma_asof(&mut f, &date).and_then(|d| refdata::soma_holdings(&mut f, &d).map(|h| (d, h))) {
        Ok(x) => x,
        Err(e) => return Err(format!("SOMA holdings: {e}")),
    };
    let (mspd_date, mspd) = refdata::mspd5(&mut f, &date).map_err(|e| format!("MSPD table 5: {e}"))?;
    let auctions: Vec<Auction> = refdata::auctions(&mut f, &date).map_err(|e| format!("auction records: {e}"))?;
    let (issues, otr) = refdata::issues(&auctions, &date);

    let inputs = Inputs { soma: &soma, mspd: &mspd, issues: &issues, on_the_run: &otr, means: &means };
    let recs = build(&a, &inputs, day);

    let mut text = String::new();
    let _ = writeln!(text, "Explain the difference from the fitted curve | price date {date} | settlement {settle_iso} | {n} notes and bonds of 6 months or more", settle_iso = curve::iso(settle), n = recs.len());
    let _ = writeln!(text, "Curve fit error: RMSE {:.2} bp. Sign: + = cheap (yield above the curve), - = rich.", a.rmse_all);
    let _ = writeln!(text, "Sources: SOMA as of {soma_asof} (weekly; later purchases and maturities are not in it); MSPD table 5 of {mspd_date} (month end; securities issued after it have no row); auction records to {date}; NY Fed secured rates.");
    let matched = |f: &dyn Fn(&Rec) -> bool| recs.iter().filter(|r| f(r)).count();
    let _ = writeln!(
        text,
        "Coverage of the {} securities: first issue and original term {}, MSPD amount and stripped {}, SOMA join {} (a CUSIP that is not in the SOMA file has a share of 0), mean residual {}.",
        recs.len(),
        matched(&|r| r.issue.is_some()),
        matched(&|r| r.outstanding_bn.is_some()),
        matched(&|r| r.soma_pct.is_some()),
        matched(&|r| r.res_mean.is_some())
    );
    let soma_held = recs.iter().filter(|r| r.soma_pct.is_some_and(|s| s > 0.0)).count();
    let _ = writeln!(text, "SOMA holds {soma_held} of them. Not matched: {}", recs.iter().filter(|r| r.issue.is_none() || r.outstanding_bn.is_none()).map(|r| r.id.as_str()).collect::<Vec<_>>().join(" "));
    for n in &notes {
        let _ = writeln!(text, "NOTE: {n}");
    }
    let q = rates.as_ref().and_then(|r| r.get(&date).copied());
    if let Some(r) = &rates {
        let _ = writeln!(text, "\nFINANCING (general collateral, percent): the rate for one CUSIP can differ (special); that is not free data.");
        let _ = writeln!(text, "{:<11} {:>6} {:>6} {:>6}", "date", "SOFR", "BGCR", "TGCR");
        for (d, v) in r.iter().rev().take(5).collect::<Vec<_>>().into_iter().rev() {
            let _ = writeln!(text, "{d:<11} {:>6.2} {:>6.2} {:>6.2}", v[0], v[1], v[2]);
        }
        let n = r.len() as f64;
        let m: Vec<f64> = (0..3).map(|i| r.values().map(|v| v[i]).sum::<f64>() / n).collect();
        let _ = writeln!(text, "mean of {} dates {:>6.2} {:>6.2} {:>6.2}   SOFR - TGCR on the date: {}", r.len(), m[0], m[1], m[2], q.map_or("-".to_string(), |v| format!("{:+.0} bp", (v[0] - v[2]) * 100.0)));
    }
    let _ = writeln!(text, "\nFUTURES (front delivery month {}): deliverable into {}", curve::iso(front_delivery(day)), {
        let c: Vec<String> = CONTRACTS.iter().map(|k| format!("{k} {}", recs.iter().filter(|r| r.deliverable.contains(k)).count())).collect();
        c.join(", ")
    });

    let latest = run_spec(&recs, false, false, "REGRESSION 1: residual of the date on the features", &[])?;
    let latest_y = run_spec(&recs, false, true, "REGRESSION 1b: the same, with years to maturity added", &[])?;
    let mean = run_spec(&recs, true, false, &format!("REGRESSION 2: mean residual of {} dates on the features", analyses.len()), &[])?;
    let no_group = run_spec(&recs, false, false, "REGRESSION 3: as 1, without the two original-30-year groups (SOMA share, size and age must carry the effect)", &[6, 7])?;
    print_spec(&mut text, &latest);
    print_spec(&mut text, &latest_y);
    print_spec(&mut text, &mean);
    print_spec(&mut text, &no_group);
    let _ = writeln!(text, "The residuals of one security are persistent over the dates, so the t values of regression 2 are too large (the dates are not independent); read its size, not its t.");
    print_candidates(&mut text, &recs, &latest);
    print_largest_after(&mut text, &recs, &latest, 12);
    print_old_bonds(&mut text, &a, &recs, &latest, q);
    print_soma_heavy(&mut text, &recs, &latest);
    print_sensitivity(&mut text, &a, &quotes, &recs, &date);
    let _ = writeln!(text, "\nNot in the data: a special repo rate or short availability of one CUSIP, dealer positions, tax by investor, and the cost of balance sheet. The regression gives association, not cause. Run time {:.1} s.", t0.elapsed().as_secs_f64());
    print!("{text}");

    let path = out.unwrap_or_else(|| fa.repo.join("data/recon").join(format!("explain_{date}.csv")));
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(&path, csv_rows(&recs, &latest, &mean)).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("Wrote {} rows to {}", recs.len(), path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fit::Nss;

    fn d(s: &str) -> Day {
        curve::parse_iso(s).unwrap()
    }

    #[test]
    fn months_and_front_month() {
        assert_eq!(months_between(d("2026-12-01"), d("2036-02-15")), 110);
        assert_eq!(months_between(d("2026-12-31"), d("2028-11-30")), 22);
        assert_eq!(months_between(d("2026-12-01"), d("2027-12-01")), 12);
        assert_eq!(front_delivery(d("2026-11-18")), d("2026-12-01"));
        assert_eq!(front_delivery(d("2026-12-05")), d("2027-03-01"));
        assert_eq!(front_delivery(d("2026-03-01")), d("2026-06-01"));
        assert_eq!(front_delivery(d("2026-01-10")), d("2026-03-01"));
    }

    #[test]
    fn delivery_baskets_follow_the_rules() {
        let first = d("2026-12-01");
        // 2036-02 bond (9y2m left, an original 30-year bond): in no basket.
        assert!(deliverable("bond", "30-Year", d("2036-02-15"), first).is_empty());
        // The 10-year note of 2036-02 (9y2m left): below the 9y5m limit of TN, above the 8y limit of ZN.
        assert!(deliverable("note", "10-Year", d("2036-02-15"), first).is_empty());
        assert_eq!(deliverable("note", "10-Year", d("2036-10-15"), first), vec!["TN"]);
        // 6y8m left: 6y6m after rounding down to 3 months, in ZN. 7y11m left: in ZN. 8y left: out.
        assert_eq!(deliverable("note", "10-Year", d("2033-08-15"), first), vec!["ZN"]);
        assert!(deliverable("note", "10-Year", d("2034-12-01"), first).is_empty());
        assert_eq!(deliverable("note", "7-Year", d("2033-08-15"), first), vec!["ZN"]);
        // 2-year and 5-year notes.
        assert_eq!(deliverable("note", "2-Year", d("2028-11-30"), first), vec!["ZT"]);
        assert_eq!(deliverable("note", "5-Year", d("2031-10-31"), first), vec!["ZF"]);
        assert!(deliverable("note", "7-Year", d("2031-10-31"), first).is_empty());
        // Bonds: ZB from 15 to less than 25 years, UB from 25 years.
        assert_eq!(deliverable("bond", "30-Year", d("2046-11-15"), first), vec!["ZB"]);
        assert_eq!(deliverable("bond", "30-Year", d("2056-11-15"), first), vec!["UB"]);
        assert!(deliverable("bond", "30-Year", d("2041-11-15"), first).is_empty()); // 14y11m
        assert_eq!(deliverable("bond", "30-Year", d("2041-12-15"), first), vec!["ZB"]);
    }

    #[test]
    fn par_rate_of_a_flat_curve() {
        let flat = Nss { b0: 0.04, b1: 0.0, b2: 0.0, b3: 0.0, tau1: 2.0, tau2: 8.0 };
        let p = par_rate(&flat, d("2026-11-15"), d("2031-11-15"));
        assert!((p - 2.0 * ((0.02_f64).exp() - 1.0)).abs() < 2e-4, "{p}");
    }

    #[test]
    fn regression_finds_a_planted_line() {
        // y = 1 + 2 a - 3 b + 0.5 c (c is a 0/1 flag), with a tiny pattern in the noise.
        let mut x = Vec::new();
        let mut y = Vec::new();
        for i in 0..60 {
            let a = (i % 7) as f64;
            let b = ((i * 3) % 11) as f64;
            let c = (i % 2) as f64;
            let e = if i % 3 == 0 { 0.01 } else { -0.005 };
            x.push(vec![a, b, c, 0.0]);
            y.push(1.0 + 2.0 * a - 3.0 * b + 0.5 * c + e);
        }
        let (r, dropped) = ols_multi(&["a", "b", "c", "zero"], &x, &y).unwrap();
        assert_eq!(dropped, vec!["zero".to_string()]);
        assert!((r.beta[1] - 2.0).abs() < 0.01 && (r.beta[2] + 3.0).abs() < 0.01 && (r.beta[3] - 0.5).abs() < 0.02, "{:?}", r.beta);
        assert!(r.r2 > 0.9999 && r.n == 60 && r.se.iter().all(|s| s.is_finite() && *s >= 0.0));
        let sse: f64 = r.resid.iter().map(|e| e * e).sum();
        assert!(sse < 0.01);
        // Two equal columns: singular.
        let x2: Vec<Vec<f64>> = (0..10).map(|i| vec![i as f64, i as f64]).collect();
        let y2: Vec<f64> = (0..10).map(|i| (i * i) as f64).collect();
        assert!(ols_multi(&["a", "b"], &x2, &y2).is_err());
    }

    #[test]
    fn regression_standard_errors_match_the_textbook_case() {
        // One variable: classical se(beta) = sigma / sqrt(Sxx). x = 0..9, y alternates around the line.
        let x: Vec<Vec<f64>> = (0..10).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = (0..10).map(|i| 1.0 + 0.5 * i as f64 + if i % 2 == 0 { 0.3 } else { -0.3 }).collect();
        let (r, _) = ols_multi(&["x"], &x, &y).unwrap();
        let sxx: f64 = (0..10).map(|i| (i as f64 - 4.5).powi(2)).sum();
        assert!((r.se_ols[1] - r.sigma / sxx.sqrt()).abs() < 1e-12);
        assert!(r.se[1] > 0.0);
    }
}
