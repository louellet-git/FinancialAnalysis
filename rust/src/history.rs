//! `fa recon history`: the difference of each security from the fitted curve, for a
//! sequence of dates, and how long it lasts.
//!
//! Input: the cached FedInvest files (`data/raw/prices/`). Each date has its own fit
//! (`fit.rs`), so a residual is a difference from the curve of the same day.
//!
//! The panel has one residual (bp, observed minus fitted yield) for each security and date.
//! Notes and bonds with a life of 6 months or more are in the panel. Bills are not.
//!
//! Measures:
//! - the pooled autocorrelation of the residual at lags of 1, 5 and 10 business days;
//! - the same after the mean of each security is removed ("within": a security that is always
//!   cheap by 3 bp has a high raw autocorrelation and a within autocorrelation near 0);
//! - the half-life, from the lag-1 coefficient `phi` of a regression through the origin:
//!   `ln(0.5) / ln(phi)`, in business days;
//! - the change of the residual after a large value: the 10 percent most cheap and the 10 percent
//!   most rich securities of a date, and their mean residual k business days later.
//!
//! BIAS. A residual has the price noise of the source (price steps, bid and ask). The most
//! extreme values of a date have more noise than the others, so part of a measured reversal is
//! noise and not a convergence that a trade can get. The within version has a small downward
//! bias for a short sample (Nickell bias, order 1/T).

use crate::richcheap::{self, Analysis};
use crate::{curve, Fa};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// One security, one date.
#[derive(Debug, Clone, PartialEq)]
pub struct Obs {
    /// Business-day number of the date.
    pub t: i64,
    pub res: f64,
    pub cost: f64,
    pub group: &'static str,
}

/// The residual panel: for each CUSIP, its observations sorted by `t`.
#[derive(Debug, Default)]
pub struct Panel {
    pub series: BTreeMap<String, Vec<Obs>>,
}

impl Panel {
    pub fn push(&mut self, id: &str, o: Obs) {
        self.series.entry(id.to_string()).or_default().push(o);
    }

    pub fn finish(&mut self) {
        for v in self.series.values_mut() {
            v.sort_by_key(|o| o.t);
        }
    }

    pub fn len(&self) -> usize {
        self.series.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }

    /// (value at t, value at t + lag) for each security that has both. `within` removes the mean of the security.
    pub fn pairs(&self, lag: i64, within: bool, filter: &dyn Fn(&Obs) -> bool) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        for v in self.series.values() {
            let mean = if within { v.iter().map(|o| o.res).sum::<f64>() / v.len() as f64 } else { 0.0 };
            let by_t: BTreeMap<i64, &Obs> = v.iter().map(|o| (o.t, o)).collect();
            for o in v {
                if !filter(o) {
                    continue;
                }
                if let Some(next) = by_t.get(&(o.t + lag)) {
                    out.push((o.res - mean, next.res - mean));
                }
            }
        }
        out
    }
}

/// Pooled Pearson correlation of the pairs. None if fewer than 3 pairs or no variance.
pub fn correlation(p: &[(f64, f64)]) -> Option<f64> {
    if p.len() < 3 {
        return None;
    }
    let n = p.len() as f64;
    let (mx, my) = (p.iter().map(|a| a.0).sum::<f64>() / n, p.iter().map(|a| a.1).sum::<f64>() / n);
    let sxy: f64 = p.iter().map(|a| (a.0 - mx) * (a.1 - my)).sum();
    let sxx: f64 = p.iter().map(|a| (a.0 - mx).powi(2)).sum();
    let syy: f64 = p.iter().map(|a| (a.1 - my).powi(2)).sum();
    if sxx < 1e-12 || syy < 1e-12 {
        return None;
    }
    Some(sxy / (sxx * syy).sqrt())
}

/// Regression through the origin: the coefficient `phi` of `x(t+lag) = phi x(t)`.
pub fn coefficient(p: &[(f64, f64)]) -> Option<f64> {
    let sxx: f64 = p.iter().map(|a| a.0 * a.0).sum();
    if p.len() < 3 || sxx < 1e-12 {
        return None;
    }
    Some(p.iter().map(|a| a.0 * a.1).sum::<f64>() / sxx)
}

/// Half-life in steps of `lag` business days: `ln(0.5) / ln(phi)`; None if `phi` is not in (0, 1).
pub fn half_life(phi: f64, lag: f64) -> Option<f64> {
    (phi > 0.0 && phi < 1.0).then(|| lag * 0.5_f64.ln() / phi.ln())
}

/// The result for one side (cheap or rich) and one horizon.
#[derive(Debug, Clone, PartialEq)]
pub struct Reversal {
    pub side: &'static str,
    pub k: i64,
    pub n: usize,
    pub mean0: f64,
    pub mean_k: f64,
}

impl Reversal {
    /// The share of the first residual that is still there after k days.
    pub fn remaining(&self) -> f64 {
        self.mean_k / self.mean0
    }
}

/// The 10 percent (`share`) most cheap and most rich of each date, and their residual k days later.
/// `within` removes the mean of each security first.
pub fn reversal(panel: &Panel, share: f64, k: i64, within: bool, filter: &dyn Fn(&Obs) -> bool) -> Vec<Reversal> {
    // value of each (cusip, t)
    let mut value: BTreeMap<(&str, i64), f64> = BTreeMap::new();
    let mut by_date: BTreeMap<i64, Vec<(&str, f64)>> = BTreeMap::new();
    for (id, v) in &panel.series {
        let mean = if within { v.iter().map(|o| o.res).sum::<f64>() / v.len() as f64 } else { 0.0 };
        for o in v {
            if !filter(o) {
                continue;
            }
            value.insert((id.as_str(), o.t), o.res - mean);
            by_date.entry(o.t).or_default().push((id.as_str(), o.res - mean));
        }
    }
    let mut acc = [(0usize, 0.0, 0.0), (0usize, 0.0, 0.0)]; // cheap, rich: (n, sum0, sumk)
    for (t, list) in &mut by_date {
        list.sort_by(|a, b| a.1.total_cmp(&b.1));
        let m = ((list.len() as f64 * share).floor() as usize).max(1);
        if list.len() < 10 {
            continue;
        }
        for (side, chunk) in [(0usize, &list[list.len() - m..]), (1usize, &list[..m])] {
            for (id, r0) in chunk {
                if let Some(rk) = value.get(&(*id, *t + k)) {
                    acc[side].0 += 1;
                    acc[side].1 += r0;
                    acc[side].2 += rk;
                }
            }
        }
    }
    ["cheap", "rich"]
        .iter()
        .zip(acc)
        .filter(|(_, a)| a.0 > 0)
        .map(|(side, a)| Reversal { side, k, n: a.0, mean0: a.1 / a.0 as f64, mean_k: a.2 / a.0 as f64 })
        .collect()
}

/// Mean residual of each security, with the standard deviation over dates (for the list of persistent ones).
pub fn security_means(panel: &Panel, min_obs: usize) -> Vec<(String, usize, f64, f64, f64)> {
    let mut out = Vec::new();
    for (id, v) in &panel.series {
        if v.len() < min_obs {
            continue;
        }
        let n = v.len() as f64;
        let mean = v.iter().map(|o| o.res).sum::<f64>() / n;
        let sd = (v.iter().map(|o| (o.res - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        let cost = v.iter().map(|o| o.cost).sum::<f64>() / n;
        out.push((id.clone(), v.len(), mean, sd, cost));
    }
    out
}

/// Business-day number of a day (days since 1970 counted only on business days).
pub fn business_ordinal(day: curve::Day, origin: curve::Day) -> i64 {
    (origin..day).filter(|d| curve::is_business_day(*d)).count() as i64
}

/// The panel from the analyses of several dates.
pub fn build_panel(analyses: &[Analysis]) -> Panel {
    let origin = analyses.iter().map(|a| a.settle).min().unwrap_or(0) - 1;
    let mut panel = Panel::default();
    for a in analyses {
        let t = business_ordinal(a.settle, origin);
        for r in a.rows.iter().filter(|r| r.kind != "bill" && r.years >= 0.5) {
            panel.push(&r.id, Obs { t, res: r.res_bp, cost: r.cost_bp, group: r.group });
        }
    }
    panel.finish();
    panel
}

/// The dates (YYYY-MM-DD) that have a cached file, in the range.
pub fn cached_dates(cache: &std::path::Path, from: Option<&str>, to: Option<&str>) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(cache.join("prices"))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|n| n.strip_prefix("fedinvest_")?.strip_suffix(".csv").map(str::to_string))
        .filter(|d| curve::parse_iso(d).is_some())
        .filter(|d| from.is_none_or(|f| d.as_str() >= f) && to.is_none_or(|t| d.as_str() <= t))
        .collect();
    v.sort();
    v
}

pub const HELP: &str = "\
fa recon history [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--out FILE]
fa recon history --lasting [--from D] [--to D] [--min-dates 38] [--same-sign 0.9] [--min-abs-bp 1.9]

Fit the curve for each cached price date and keep the difference of each note and bond from
its curve. Measure how long the difference lasts: autocorrelation at 1, 5 and 10 business
days, half-life, and the change after a large difference.

  --from, --to  The range of dates (default: all cached files in data/raw/prices).
  --out         The panel CSV (cusip, date, res_bp, cost_bp, group).
                Default: data/recon/richcheap_history.csv in the repository.

  --lasting     List the securities with a lasting residual (A-117) and count them: rich, cheap, group.
                It reads the stored price files only. It makes no request, reads no GSW file and writes no file.
                A lasting security has at least --min-dates dates, the same sign as its mean on at least
                --same-sign of its days, and |mean| above its mean cost and above --min-abs-bp (the fit RMSE).
                --to D is an as-of date: no later day is read, so a later file does not change the result.
                A candidate here is a security to study, not a trade.";

fn fmt_opt(v: Option<f64>) -> String {
    v.map_or("     -".to_string(), |x| format!("{x:6.3}"))
}

pub fn report(analyses: &[Analysis], panel: &Panel) -> String {
    let mut o = String::new();
    let (first, last) = (&analyses[0].date, &analyses[analyses.len() - 1].date);
    let rmse: Vec<f64> = analyses.iter().map(|a| a.rmse_all).collect();
    let _ = writeln!(o, "Dates: {} business days, {first} to {last}. Observations: {} (security-days), {} securities.", analyses.len(), panel.len(), panel.series.len());
    let _ = writeln!(
        o,
        "Fit error of each day (RMSE, bp): mean {:.2}, min {:.2}, max {:.2}. Notes and bonds of 6 months or more are in the panel.\n",
        rmse.iter().sum::<f64>() / rmse.len() as f64,
        rmse.iter().cloned().fold(f64::INFINITY, f64::min),
        rmse.iter().cloned().fold(0.0, f64::max)
    );
    let all = |_: &Obs| true;
    let _ = writeln!(o, "PERSISTENCE (pooled over securities; lag in business days)");
    let _ = writeln!(o, "{:<28} {:>6} {:>6} {:>6} {:>8} {:>8}", "measure", "lag 1", "lag 5", "lag 10", "pairs 1", "pairs 10");
    for (label, within) in [("autocorrelation, raw", false), ("autocorrelation, within", true)] {
        let c: Vec<Option<f64>> = [1, 5, 10].iter().map(|l| correlation(&panel.pairs(*l, within, &all))).collect();
        let _ = writeln!(
            o,
            "{label:<28} {} {} {} {:>8} {:>8}",
            fmt_opt(c[0]),
            fmt_opt(c[1]),
            fmt_opt(c[2]),
            panel.pairs(1, within, &all).len(),
            panel.pairs(10, within, &all).len()
        );
    }
    for (label, within) in [("coefficient phi, raw", false), ("coefficient phi, within", true)] {
        let c: Vec<Option<f64>> = [1, 5, 10].iter().map(|l| coefficient(&panel.pairs(*l, within, &all))).collect();
        let _ = writeln!(o, "{label:<28} {} {} {}", fmt_opt(c[0]), fmt_opt(c[1]), fmt_opt(c[2]));
    }
    let _ = writeln!(o, "\nHALF-LIFE (business days, from phi at lag k: k ln(0.5) / ln(phi); '-' = phi not in (0,1))");
    for (label, within) in [("raw", false), ("within", true)] {
        let hl: Vec<String> = [1i64, 5, 10]
            .iter()
            .map(|l| {
                let phi = coefficient(&panel.pairs(*l, within, &all));
                phi.and_then(|p| half_life(p, *l as f64)).map_or("-".to_string(), |h| format!("{h:.1}"))
            })
            .collect();
        let _ = writeln!(o, "{label:<8} lag 1: {:>6}   lag 5: {:>6}   lag 10: {:>6}", hl[0], hl[1], hl[2]);
    }
    let _ = writeln!(o, "\nAFTER A LARGE DIFFERENCE: the 10% most cheap and 10% most rich securities of a date, residual k days later");
    let _ = writeln!(o, "{:<8} {:<6} {:>3} {:>6} {:>9} {:>9} {:>10}", "values", "side", "k", "n", "mean bp 0", "mean bp k", "remaining");
    for (label, within) in [("raw", false), ("within", true)] {
        for k in [1i64, 5, 10] {
            for r in reversal(panel, 0.10, k, within, &all) {
                let _ = writeln!(o, "{label:<8} {:<6} {:>3} {:>6} {:>+9.2} {:>+9.2} {:>9.0}%", r.side, r.k, r.n, r.mean0, r.mean_k, r.remaining() * 100.0);
            }
        }
    }
    let _ = writeln!(o, "\nBY GROUP (lag 1 and lag 10 pooled autocorrelation, raw; group at date t)");
    let _ = writeln!(o, "{:<22} {:>6} {:>6} {:>7}", "group", "lag 1", "lag 10", "pairs 1");
    for grp in richcheap::GROUPS {
        let f = |ob: &Obs| ob.group == grp;
        let p1 = panel.pairs(1, false, &f);
        if p1.len() < 30 {
            continue;
        }
        let _ = writeln!(o, "{grp:<22} {} {} {:>7}", fmt_opt(correlation(&p1)), fmt_opt(correlation(&panel.pairs(10, false, &f))), p1.len());
    }
    let mut means = security_means(panel, (analyses.len() * 2 / 3).max(10));
    let _ = writeln!(o, "\nSECURITIES WITH A LONG RECORD ({} with at least {} dates): mean residual over the dates", means.len(), (analyses.len() * 2 / 3).max(10));
    let above = means.iter().filter(|m| m.2.abs() > m.4).count();
    let _ = writeln!(o, "{above} of them have |mean residual| larger than their mean cost.");
    means.sort_by(|a, b| b.2.total_cmp(&a.2));
    let _ = writeln!(o, "{:<10} {:>4} {:>9} {:>7} {:>6}", "cusip", "n", "mean bp", "sd bp", "cost");
    for (label, list) in [("most cheap", means.iter().take(5).collect::<Vec<_>>()), ("most rich", means.iter().rev().take(5).collect::<Vec<_>>())] {
        let _ = writeln!(o, "  {label}");
        for m in list {
            let _ = writeln!(o, "{:<10} {:>4} {:>+9.2} {:>7.2} {:>6.1}", m.0, m.1, m.2, m.3, m.4);
        }
    }
    o
}

/// Our curve against the GSW curve on each date that has a GSW row: for each tenor the mean, the
/// smallest and the largest difference (bp), and the number of dates.
pub fn gsw_summary(analyses: &[Analysis], ds: &crate::forwards::Dataset) -> Vec<(u32, f64, f64, f64, usize)> {
    let mut by_tenor: BTreeMap<u32, Vec<f64>> = BTreeMap::new();
    for a in analyses {
        let Some(day) = curve::parse_iso(&a.date) else { continue };
        let Some((table, gdate)) = richcheap::gsw_table(&a.nss, ds, &a.date, day, a.settle) else { continue };
        if gdate != a.date {
            continue; // a GSW row of an earlier day is not the same day
        }
        for (y, g, f) in table {
            by_tenor.entry(y).or_default().push((f - g) * 1e4);
        }
    }
    by_tenor
        .into_iter()
        .map(|(y, v)| {
            let n = v.len();
            (y, v.iter().sum::<f64>() / n as f64, v.iter().cloned().fold(f64::INFINITY, f64::min), v.iter().cloned().fold(f64::NEG_INFINITY, f64::max), n)
        })
        .collect()
}

/// The rule of a lasting residual (A-117). The values are an author choice, not a study.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LastingRule {
    /// Smallest number of dates of a security.
    pub min_dates: usize,
    /// Smallest share of the days that have the sign of the mean (0 to 1).
    pub same_sign: f64,
    /// |mean residual| must be above this value (bp). The default is the mean fit RMSE of the 58 days.
    pub min_abs_bp: f64,
}

impl Default for LastingRule {
    fn default() -> Self {
        LastingRule { min_dates: 38, same_sign: 0.9, min_abs_bp: 1.9 }
    }
}

/// One security that meets the rule.
#[derive(Debug, Clone, PartialEq)]
pub struct LastingRow {
    pub id: String,
    pub n: usize,
    /// Share of the days with the sign of the mean.
    pub share: f64,
    pub mean: f64,
    pub cost: f64,
    pub sd: f64,
    /// Half-life (business days) of the deviation from the own mean, from the lag-1 coefficient.
    pub half_life: Option<f64>,
    /// Group at the last date of the security (a label).
    pub group: &'static str,
}

/// The securities of the panel that meet the rule, sorted by |mean| (largest first), and the counts.
#[derive(Debug, Default, PartialEq)]
pub struct Lasting {
    pub n_securities: usize,
    pub rows: Vec<LastingRow>,
}

impl Lasting {
    pub fn rich(&self) -> usize {
        self.rows.iter().filter(|r| r.mean < 0.0).count()
    }

    pub fn cheap(&self) -> usize {
        self.rows.iter().filter(|r| r.mean > 0.0).count()
    }

    /// Count of lasting securities for each group, in the order of `richcheap::GROUPS`; groups with 0 are left out.
    pub fn by_group(&self) -> Vec<(&'static str, usize)> {
        richcheap::GROUPS
            .iter()
            .map(|g| (*g, self.rows.iter().filter(|r| r.group == *g).count()))
            .filter(|x| x.1 > 0)
            .collect()
    }
}

/// Apply the rule to the panel. All values are exact (no rounding before a test).
pub fn lasting(panel: &Panel, rule: &LastingRule) -> Lasting {
    let mut rows = Vec::new();
    for (id, v) in &panel.series {
        let n = v.len();
        if n < rule.min_dates || n < 2 {
            continue;
        }
        let nf = n as f64;
        let mean = v.iter().map(|o| o.res).sum::<f64>() / nf;
        let same = v.iter().filter(|o| o.res * mean > 0.0).count() as f64 / nf;
        let cost = v.iter().map(|o| o.cost).sum::<f64>() / nf;
        let a = mean.abs();
        // 1e-12: a share that is exactly the limit (36 of 40 days at 0.9) passes in floating point too.
        if same + 1e-12 < rule.same_sign || a <= cost || a <= rule.min_abs_bp {
            continue;
        }
        let sd = (v.iter().map(|o| (o.res - mean).powi(2)).sum::<f64>() / (nf - 1.0)).sqrt();
        // Lag-1 pairs of the deviation from the own mean; a gap in the dates is not a lag.
        let by_t: BTreeMap<i64, f64> = v.iter().map(|o| (o.t, o.res - mean)).collect();
        let pairs: Vec<(f64, f64)> = by_t.iter().filter_map(|(t, x)| by_t.get(&(t + 1)).map(|y| (*x, *y))).collect();
        let half_life = coefficient(&pairs).and_then(|p| half_life(p, 1.0));
        let group = v.last().map_or("other", |o| o.group);
        rows.push(LastingRow { id: id.clone(), n, share: same, mean, cost, sd, half_life, group });
    }
    rows.sort_by(|a, b| b.mean.abs().total_cmp(&a.mean.abs()).then_with(|| a.id.cmp(&b.id)));
    Lasting { n_securities: panel.series.len(), rows }
}

/// Maturity and coupon of each security, from the last analysis that has it.
pub fn security_terms(analyses: &[Analysis]) -> BTreeMap<String, (curve::Day, Option<f64>)> {
    let mut m = BTreeMap::new();
    for a in analyses {
        for r in &a.rows {
            m.insert(r.id.clone(), (r.maturity, r.coupon));
        }
    }
    m
}

pub fn lasting_report(l: &Lasting, rule: &LastingRule, dates: &[String], terms: &BTreeMap<String, (curve::Day, Option<f64>)>) -> String {
    let mut o = String::new();
    let (first, last) = (&dates[0], &dates[dates.len() - 1]);
    let _ = writeln!(o, "LASTING RESIDUALS. Dates: {} business days, {first} to {last} (as-of date {last}). Securities in the panel: {}.", dates.len(), l.n_securities);
    let _ = writeln!(
        o,
        "Rule (A-117): at least {} dates; the sign of the mean on at least {:.0}% of the days; |mean residual| above the mean cost and above {} bp.",
        rule.min_dates,
        rule.same_sign * 100.0,
        rule.min_abs_bp
    );
    let _ = writeln!(o, "Residual = observed minus fitted yield (bp); plus is cheap, minus is rich. Each fit is of its own day.\n");
    let _ = writeln!(o, "{:<10} {:<10} {:>6} {:>7} {:>10} {:>8} {:>7} {:>9}  group", "cusip", "maturity", "days", "same %", "mean bp", "cost bp", "sd bp", "half-life");
    for r in &l.rows {
        let (mat, _) = terms.get(&r.id).copied().unwrap_or((0, None));
        let _ = writeln!(
            o,
            "{:<10} {:<10} {:>6} {:>7.1} {:>+10.2} {:>8.2} {:>7.2} {:>9}  {}",
            r.id,
            if mat == 0 { "-".to_string() } else { curve::iso(mat) },
            r.n,
            r.share * 100.0,
            r.mean,
            r.cost,
            r.sd,
            r.half_life.map_or("-".to_string(), |h| format!("{h:.1}")),
            r.group
        );
    }
    let _ = writeln!(o, "\nLasting: {} of {} securities. Rich {} (price above the curve), cheap {}.", l.rows.len(), l.n_securities, l.rich(), l.cheap());
    let _ = writeln!(o, "By group (group at the last date of the security):");
    for (g, n) in l.by_group() {
        let _ = writeln!(o, "  {g:<20} {n}");
    }
    let _ = writeln!(
        o,
        "\nHalf-life: of the deviation from the own mean, in business days, from the lag-1 coefficient; '-' = not in (0,1).\n\
         A lasting residual is a candidate after cost, not a trade: the data has no repo rate, borrow cost, tax or executable quote.\n\
         The residual is a price gap plus model error. The sample has {} days: no unit-root test was run, and the level may be a group effect.",
        dates.len()
    );
    o
}

pub fn write_panel(path: &std::path::Path, analyses: &[Analysis]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut text = String::from("cusip,date,res_bp,cost_bp,group\n");
    for a in analyses {
        for r in a.rows.iter().filter(|r| r.kind != "bill" && r.years >= 0.5) {
            let _ = writeln!(text, "{},{},{:.3},{:.3},{}", r.id, a.date, r.res_bp, r.cost_bp, r.group.replace(',', ";"));
        }
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Fit each date from the stored price files. Reads files only.
fn fit_dates(cache: &std::path::Path, dates: &[String]) -> Result<Vec<Analysis>, String> {
    let results: Vec<Result<Analysis, String>> = dates
        .par_iter()
        .map(|d| {
            let quotes = richcheap::cached_quotes(cache, d)?;
            let day = curve::parse_iso(d).ok_or("bad date")?;
            richcheap::analyse(d, curve::add_business_days(day, 1), &quotes).map_err(|e| format!("{d}: {e}"))
        })
        .collect();
    let mut analyses = Vec::new();
    for (d, r) in dates.iter().zip(results) {
        match r {
            Ok(a) => analyses.push(a),
            Err(e) => eprintln!("fa recon: skipped {d}: {e}"),
        }
    }
    Ok(analyses)
}

fn run_lasting(cache: &std::path::Path, from: Option<&str>, to: Option<&str>, rule: &LastingRule) -> Result<(), String> {
    let dates = cached_dates(cache, from, to);
    if dates.len() < rule.min_dates {
        return Err(format!(
            "only {} stored price files in range in {} (the rule needs {} or more dates). No request was made; the store copy must have the days (docs/DATA-SOURCES.md, the data store).",
            dates.len(),
            cache.join("prices").display(),
            rule.min_dates
        ));
    }
    let analyses = fit_dates(cache, &dates)?;
    if analyses.len() < rule.min_dates {
        return Err(format!("only {} dates could be fitted (the rule needs {})", analyses.len(), rule.min_dates));
    }
    let used: Vec<String> = analyses.iter().map(|a| a.date.clone()).collect();
    let panel = build_panel(&analyses);
    let l = lasting(&panel, rule);
    print!("{}", lasting_report(&l, rule, &used, &security_terms(&analyses)));
    Ok(())
}

pub fn run_history(fa: &Fa, args: &[String]) -> Result<(), String> {
    let (mut from, mut to, mut out) = (None, None, None);
    let (mut lasting_mode, mut rule, mut rule_flag) = (false, LastingRule::default(), false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--from" => from = Some(it.next().ok_or("--from needs a value")?.clone()),
            "--to" => to = Some(it.next().ok_or("--to needs a value")?.clone()),
            "--out" => out = Some(std::path::PathBuf::from(it.next().ok_or("--out needs a value")?)),
            "--lasting" => lasting_mode = true,
            "--min-dates" => {
                rule_flag = true;
                rule.min_dates = it.next().ok_or("--min-dates needs a value")?.parse().map_err(|_| "--min-dates needs a whole number")?;
            }
            "--same-sign" => {
                rule_flag = true;
                rule.same_sign = it.next().ok_or("--same-sign needs a value")?.parse().map_err(|_| "--same-sign needs a number (0.9 is 90%)")?;
            }
            "--min-abs-bp" => {
                rule_flag = true;
                rule.min_abs_bp = it.next().ok_or("--min-abs-bp needs a value")?.parse().map_err(|_| "--min-abs-bp needs a number (bp)")?;
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    if rule_flag && !lasting_mode {
        return Err("--min-dates, --same-sign and --min-abs-bp need --lasting".into());
    }
    if lasting_mode && out.is_some() {
        return Err("--lasting writes no file; do not use --out".into());
    }
    if rule.min_dates < 2 || !(rule.same_sign > 0.0 && rule.same_sign <= 1.0) || !rule.min_abs_bp.is_finite() || rule.min_abs_bp < 0.0 {
        return Err("rule values: --min-dates 2 or more, --same-sign in (0, 1] (0.9 is 90%), --min-abs-bp 0 or more".into());
    }
    for d in [&from, &to].into_iter().flatten() {
        if curve::parse_iso(d).is_none() {
            return Err(format!("'{d}' is not a date (YYYY-MM-DD)"));
        }
    }
    let cache = fa.repo.join("data/raw");
    if lasting_mode {
        return run_lasting(&cache, from.as_deref(), to.as_deref(), &rule);
    }
    let dates = cached_dates(&cache, from.as_deref(), to.as_deref());
    if dates.len() < 12 {
        return Err(format!("only {} cached price files in range (need 12 or more; see hidden assumption 69)", dates.len()));
    }
    let analyses = fit_dates(&cache, &dates)?;
    if analyses.len() < 12 {
        return Err("fewer than 12 dates could be fitted".into());
    }
    let panel = build_panel(&analyses);
    print!("{}", report(&analyses, &panel));
    if let Ok(ds) = crate::forwards::load(&fa.repo) {
        let g = gsw_summary(&analyses, &ds);
        if !g.is_empty() {
            println!("\nOUR CURVE MINUS THE GSW CURVE (zero-coupon yield, bp), over the {} dates that have a GSW row", g[0].4);
            println!("{:<6} {:>7} {:>7} {:>7}", "years", "mean", "min", "max");
            for (y, m, lo, hi, _) in g {
                println!("{y:<6} {m:>+7.1} {lo:>+7.1} {hi:>+7.1}");
            }
        }
    }
    let path = out.unwrap_or_else(|| fa.repo.join("data/recon/richcheap_history.csv"));
    write_panel(&path, &analyses)?;
    println!("\nWrote the panel ({} rows) to {}", panel.len(), path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 12 securities, 40 dates: x(t) = a_i * phi^t.
    fn decay_panel(phi: f64) -> Panel {
        let mut p = Panel::default();
        for i in 0..12 {
            let a = if i % 2 == 0 { 1.0 + i as f64 } else { -1.0 - i as f64 };
            for t in 0..40 {
                p.push(&format!("S{i}"), Obs { t, res: a * phi.powi(t as i32), cost: 1.0, group: "other" });
            }
        }
        p.finish();
        p
    }

    #[test]
    fn coefficient_and_half_life_of_a_known_decay() {
        let p = decay_panel(0.9);
        let all = |_: &Obs| true;
        let phi1 = coefficient(&p.pairs(1, false, &all)).unwrap();
        assert!((phi1 - 0.9).abs() < 1e-12);
        let phi5 = coefficient(&p.pairs(5, false, &all)).unwrap();
        assert!((phi5 - 0.9_f64.powi(5)).abs() < 1e-12);
        let hl = half_life(phi1, 1.0).unwrap();
        assert!((hl - 0.5_f64.ln() / 0.9_f64.ln()).abs() < 1e-12); // 6.58 days
        assert!((half_life(phi5, 5.0).unwrap() - hl).abs() < 1e-9);
        assert_eq!(half_life(1.0, 1.0), None);
        assert_eq!(half_life(-0.2, 1.0), None);
        // Pairs at lag 10: 30 per security.
        assert_eq!(p.pairs(10, false, &all).len(), 12 * 30);
    }

    #[test]
    fn a_fixed_difference_is_persistent_raw_and_has_no_within_information() {
        let mut p = Panel::default();
        for i in 0..12 {
            for t in 0..30 {
                p.push(&format!("S{i}"), Obs { t, res: (i as f64 - 5.5) * 2.0, cost: 1.0, group: "other" });
            }
        }
        p.finish();
        let all = |_: &Obs| true;
        assert!((correlation(&p.pairs(1, false, &all)).unwrap() - 1.0).abs() < 1e-12);
        assert_eq!(correlation(&p.pairs(1, true, &all)), None); // no variance after the mean is removed
    }

    #[test]
    fn alternating_residual_has_negative_within_autocorrelation() {
        let mut p = Panel::default();
        for i in 0..12 {
            for t in 0..30 {
                let s = if (t + i) % 2 == 0 { 1.0 } else { -1.0 };
                p.push(&format!("S{i}"), Obs { t, res: s, cost: 1.0, group: "other" });
            }
        }
        p.finish();
        let all = |_: &Obs| true;
        assert!((correlation(&p.pairs(1, false, &all)).unwrap() + 1.0).abs() < 1e-12);
        assert!((correlation(&p.pairs(2, false, &all)).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_gap_in_the_dates_is_not_a_lag() {
        let mut p = Panel::default();
        for t in [0, 1, 3, 4] {
            p.push("A", Obs { t, res: t as f64, cost: 1.0, group: "other" });
        }
        p.finish();
        let all = |_: &Obs| true;
        // Lag 1 pairs: (0,1) and (3,4). Not (1,3).
        assert_eq!(p.pairs(1, false, &all), vec![(0.0, 1.0), (3.0, 4.0)]);
        assert_eq!(p.pairs(3, false, &all), vec![(0.0, 3.0), (1.0, 4.0)]);
    }

    #[test]
    fn reversal_after_a_large_value_follows_the_decay() {
        let p = decay_panel(0.8);
        let all = |_: &Obs| true;
        // 10 percent of 12 securities = 1 security each side. Cheap: the one with the largest positive value.
        let r = reversal(&p, 0.10, 5, false, &all);
        let cheap = r.iter().find(|x| x.side == "cheap").unwrap();
        let rich = r.iter().find(|x| x.side == "rich").unwrap();
        assert!((cheap.remaining() - 0.8_f64.powi(5)).abs() < 1e-9, "{}", cheap.remaining());
        assert!((rich.remaining() - 0.8_f64.powi(5)).abs() < 1e-9);
        assert!(cheap.mean0 > 0.0 && rich.mean0 < 0.0);
        // The extremes of the first date are S10 (11) and S11 (-12); later dates keep them.
        assert!(cheap.n == 35 && rich.n == 35);
    }

    #[test]
    fn business_ordinal_skips_weekends_and_holidays() {
        let fri = curve::parse_iso("2026-11-06").unwrap();
        let mon = curve::parse_iso("2026-11-09").unwrap();
        assert_eq!(business_ordinal(mon, fri), 1);
        // 2026-11-11 (Wednesday) is a holiday: from Tue 11-10 to Thu 11-12 is 1 business day.
        let (tue, thu) = (curve::parse_iso("2026-11-10").unwrap(), curve::parse_iso("2026-11-12").unwrap());
        assert_eq!(business_ordinal(thu, tue), 1);
    }

    #[test]
    fn security_means_need_enough_dates() {
        let p = decay_panel(0.9);
        assert_eq!(security_means(&p, 41).len(), 0);
        assert_eq!(security_means(&p, 40).len(), 12);
    }

    // ---- lasting rule ----

    fn series(p: &mut Panel, id: &str, vals: &[f64], cost: f64, group: &'static str) {
        for (t, v) in vals.iter().enumerate() {
            p.push(id, Obs { t: t as i64, res: *v, cost, group });
        }
    }

    /// n values: `neg` days of -(mean_abs + 1) first, then the rest at a value so the mean is exact.
    fn mixed(n: usize, wrong: usize, big: f64, small: f64) -> Vec<f64> {
        // (n - wrong) days at `big`, `wrong` days at `small` (opposite sign).
        (0..n).map(|i| if i < wrong { small } else { big }).collect()
    }

    #[test]
    fn counts_of_a_synthetic_panel_by_hand() {
        let mut p = Panel::default();
        // A, B: rich, 40 days at -5 (mean -5, cost 1): lasting. Groups: old 30-year bond, other.
        series(&mut p, "A", &[-5.0; 40], 1.0, "old 30-year bond");
        series(&mut p, "B", &[-5.0; 40], 1.0, "other");
        // C: cheap, 40 days at +3 (cost 1): lasting.
        series(&mut p, "C", &[3.0; 40], 1.0, "other");
        // D: mean -1.5, below the 1.9 bp floor: not lasting.
        series(&mut p, "D", &[-1.5; 40], 0.5, "other");
        // E: mean -5 but cost 6: not lasting.
        series(&mut p, "E", &[-5.0; 40], 6.0, "other");
        // F: 37 dates only: not lasting.
        series(&mut p, "F", &[-5.0; 37], 1.0, "other");
        // G: alternates -9 and +1 (mean -4): same sign on 50% of days: not lasting.
        let g: Vec<f64> = (0..40).map(|i| if i % 2 == 0 { -9.0 } else { 1.0 }).collect();
        series(&mut p, "G", &g, 1.0, "other");
        p.finish();
        let l = lasting(&p, &LastingRule::default());
        assert_eq!(l.n_securities, 7);
        let ids: Vec<&str> = l.rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["A", "B", "C"]); // |mean| 5, 5 (id order), 3
        assert_eq!((l.rich(), l.cheap()), (2, 1));
        assert_eq!(l.by_group(), vec![("old 30-year bond", 1), ("other", 2)]);
        assert!((l.rows[0].mean + 5.0).abs() < 1e-12 && (l.rows[0].share - 1.0).abs() < 1e-12);
        assert!((l.rows[2].cost - 1.0).abs() < 1e-12);
    }

    #[test]
    fn exactly_38_dates_passes_and_37_fails() {
        let mut p = Panel::default();
        series(&mut p, "X38", &[-5.0; 38], 1.0, "other");
        series(&mut p, "X37", &[-5.0; 37], 1.0, "other");
        p.finish();
        let l = lasting(&p, &LastingRule::default());
        assert_eq!(l.rows.len(), 1);
        assert_eq!(l.rows[0].id, "X38");
        // --min-dates moves the edge.
        let l = lasting(&p, &LastingRule { min_dates: 37, ..LastingRule::default() });
        assert_eq!(l.rows.len(), 2);
    }

    #[test]
    fn exactly_90_percent_same_sign_passes_and_less_fails() {
        let mut p = Panel::default();
        // 40 days, 4 of them +1 (wrong sign), 36 at -10: share 36/40 = 0.9 exactly. Mean = (36*-10 + 4)/40 = -8.9.
        series(&mut p, "E90", &mixed(40, 4, -10.0, 1.0), 1.0, "other");
        // 5 wrong days: 35/40 = 0.875.
        series(&mut p, "E875", &mixed(40, 5, -10.0, 1.0), 1.0, "other");
        // 10 days, scaled: 50 days, 5 wrong (45/50 = 0.9).
        series(&mut p, "F90", &mixed(50, 5, 10.0, -1.0), 1.0, "other");
        // A zero day is not the sign of the mean: 36 days at -10, 4 at 0 -> share 0.9 passes; 5 zero days -> fails.
        series(&mut p, "Z90", &mixed(40, 4, -10.0, 0.0), 1.0, "other");
        series(&mut p, "Z875", &mixed(40, 5, -10.0, 0.0), 1.0, "other");
        p.finish();
        let l = lasting(&p, &LastingRule::default());
        let ids: Vec<&str> = l.rows.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"E90") && ids.contains(&"F90") && ids.contains(&"Z90"), "{ids:?}");
        assert!(!ids.contains(&"E875") && !ids.contains(&"Z875"), "{ids:?}");
        let e90 = l.rows.iter().find(|r| r.id == "E90").unwrap();
        assert!((e90.share - 0.9).abs() < 1e-12 && (e90.mean + 8.9).abs() < 1e-12);
        // A higher limit removes the 90% securities.
        let l = lasting(&p, &LastingRule { same_sign: 0.95, ..LastingRule::default() });
        assert!(l.rows.is_empty());
    }

    #[test]
    fn mean_must_be_above_cost_and_floor_strictly() {
        let mut p = Panel::default();
        series(&mut p, "EQCOST", &[-5.0; 40], 5.0, "other"); // |mean| = cost: fails
        series(&mut p, "EQFLOOR", &[-1.9; 40], 0.5, "other"); // |mean| = 1.9: fails
        series(&mut p, "OK", &[-1.95; 40], 0.5, "other");
        p.finish();
        let l = lasting(&p, &LastingRule::default());
        assert_eq!(l.rows.len(), 1);
        assert_eq!(l.rows[0].id, "OK");
    }

    #[test]
    fn half_life_of_the_deviation_from_the_mean() {
        let mut p = Panel::default();
        let v: Vec<f64> = (0..40).map(|t| -6.0 + if t % 2 == 0 { 0.5 } else { -0.5 }).collect();
        series(&mut p, "ALT", &v, 1.0, "other");
        p.finish();
        let l = lasting(&p, &LastingRule::default());
        // Alternating deviation: lag-1 coefficient is negative, so there is no half-life.
        assert_eq!(l.rows.len(), 1);
        assert_eq!(l.rows[0].half_life, None);
        let mut p = Panel::default();
        series(&mut p, "FLAT", &[-6.0; 40], 1.0, "other");
        p.finish();
        assert_eq!(lasting(&p, &LastingRule::default()).rows[0].half_life, None); // no variance
    }

    /// A panel of `days` dates; the securities of the first 38 dates are rich for ever. A later day is poisoned.
    fn panel_to(days: usize, poison_after: usize) -> Panel {
        let mut p = Panel::default();
        for i in 0..5 {
            let id = format!("S{i}");
            let v: Vec<f64> = (0..days).map(|t| if t >= poison_after { 1000.0 } else { -5.0 - i as f64 }).collect();
            series(&mut p, &id, &v, 1.0, "other");
        }
        p.finish();
        p
    }

    #[test]
    fn a_later_day_does_not_change_an_earlier_as_of_result() {
        // Point in time: the panel to day 40 (as of day 40) is the same, whatever follows on days 41 to 60.
        let clean = lasting(&panel_to(40, 1000), &LastingRule::default());
        let rows_40: Vec<Obs> = panel_to(60, 40).series["S0"][..40].to_vec();
        let mut cut = Panel::default();
        for (id, v) in &panel_to(60, 40).series {
            for o in v.iter().filter(|o| o.t < 40) {
                cut.push(id, o.clone());
            }
        }
        cut.finish();
        assert_eq!(rows_40.len(), 40);
        assert_eq!(lasting(&cut, &LastingRule::default()), clean);
        assert_eq!(clean.rows.len(), 5);
        // The poisoned days, when they are read, do change the result: the test has power.
        let full = lasting(&panel_to(60, 40), &LastingRule::default());
        assert_ne!(full, clean);
    }

    #[test]
    fn to_date_ignores_later_files() {
        let dir = std::env::temp_dir().join(format!("fa_lasting_{}", std::process::id()));
        let prices = dir.join("prices");
        std::fs::create_dir_all(&prices).unwrap();
        for d in ["2026-03-02", "2026-03-03", "2026-03-04", "2026-03-05"] {
            std::fs::write(prices.join(format!("fedinvest_{d}.csv")), "x").unwrap();
        }
        let upto = cached_dates(&dir, None, Some("2026-03-03"));
        assert_eq!(upto, vec!["2026-03-02", "2026-03-03"]);
        std::fs::write(prices.join("fedinvest_2026-03-06.csv"), "x").unwrap(); // a later day arrives
        assert_eq!(cached_dates(&dir, None, Some("2026-03-03")), upto);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lasting_refuses_when_the_files_are_missing() {
        let dir = std::env::temp_dir().join(format!("fa_lasting_none_{}", std::process::id()));
        let e = run_lasting(&dir, None, None, &LastingRule::default()).unwrap_err();
        assert!(e.contains("only 0 stored price files") && e.contains("No request was made"), "{e}");
    }
}
