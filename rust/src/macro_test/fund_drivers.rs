//! `fa macro fund-drivers`: the frozen layer-3 study of URA and GLD against outside drivers.
//! Design: `analysis/macro/2026-11-25-fund-drivers-design.md` (rules 1 to 10, records R1, R2, R3). Model document:
//! `docs/models/fund-drivers.md`.
//!
//! The command reads files only. It starts no download and sends no order. A fit is not a cause.
//!
//! Choices where the design is not explicit:
//! 1. The group return of a month is a simple return; the outcome is its log, ln(1 + r).
//! 2. URA-F quarterly return = sum of the layer-2 parts of the foreign rows / sum of their start weights
//!    (the same group-return rule as URA-M and URA-R), as a log.
//! 3. The quarterly U is the sum of the three monthly log changes of the IMF series.
//! 4. R is the change of the month-end DFII-type 10Y par real yield of the Treasury file, in percentage points.
//! 5. The two sums of rule 3.6(b) are in percentage points of log returns.
//! 6. Without a U reader (none exists yet) the three URA models are not run; the GLD model and the identity run.
//! 7. URA-F uses the quarters for which both N-PORT reports are in the cache (a prefix of the 12 quarters); the count is printed.

use super::stats::{expanding_preds, nw_se, ols};
use super::CmdError;
use crate::market::fund_attribution::{attribute, close_on, parse_map, positions, Group, Quarter};
use crate::market::prices::PricePoint;
use crate::market::fund_holdings::{read_cached_reports, select_reports, series_of, Report};
use crate::market::{commodity, price_cache};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa macro fund-drivers [--cache DIR] [--out FILE]

The frozen layer-3 study of URA and GLD (design analysis/macro/2026-11-25-fund-drivers-design.md).
  --cache DIR   default <repo>/data/raw: N-PORT reports and Nasdaq closes (URA, GLD, SPY, SMH, UUP and the holdings),
                the Treasury real curve (DIR/rates/treasury_real_<year>.csv), the World Bank file (DIR/commodity/worldbank)
  --out FILE    JSON of the results only (no series values of U)
U (IMF uranium) is read at run time through a reader and is not stored. No reader exists yet: the URA models then print
'U not available' and are not run. No request is sent. A fit is not a cause.";

/// Record R3: the group of each US-listed holding of URA, fixed for all quarters.
/// Source: design section 6, record R3 (2026-11-26).
pub const URA_M: [&str; 9] = ["CCJ", "NXE", "DNN", "UEC", "UUUU", "URG", "UROY", "ISOU", "EU"];
/// Record R3: reactors, fuel and equipment.
pub const URA_R: [&str; 7] = ["LEU", "OKLO", "SMR", "NNE", "LTBR", "IMSR", "NKLR"];
/// Record R1: Daewoo E&C, left out of the sensitivity line of URA-F.
pub const DAEWOO_ISIN: &str = "KR7047040001";
const FIRST_MONTH: (i64, i64) = (2023, 11);
const N_MONTHS: usize = 36;
const FIT_MONTHS: usize = 24;
const MONTHLY_LAG: usize = 3;
const QUARTERLY_LAG: usize = 1;
/// Smallest circular shift: monthly 6 (25 shifts 6..=30), quarterly 2 (9 shifts 2..=10).
const MONTHLY_MIN_SHIFT: usize = 6;
const QUARTERLY_MIN_SHIFT: usize = 2;
const P_LIMIT: f64 = 0.10;
const MAX_STALE: i64 = 5;
const MAP_CSV: &str = include_str!("../market/fund_ticker_map.csv");

// ----------------------------------------------------------------------------------------------
// Pure rules
// ----------------------------------------------------------------------------------------------

/// The month-end dates of the sample, with the base month-end before the first month: 37 dates.
pub fn month_ends() -> Vec<String> {
    month_ends_from(FIRST_MONTH, N_MONTHS)
}

/// `n` month-ends from the month `first`, with the base month-end before it: n + 1 dates.
pub fn month_ends_from(first: (i64, i64), n: usize) -> Vec<String> {
    let (mut y, mut m) = first;
    // base: the last day of the month before the first month
    let mut out = vec![mapcheck::dates::format(mapcheck::dates::days_from_civil(y, m, 1) - 1)];
    for _ in 0..n {
        out.push(mapcheck::dates::format(mapcheck::dates::days_from_civil(y, m, mapcheck::dates::days_in_month(y, m))));
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
    out
}

/// Group month returns inside one quarter. A member is (start weight, [P0, P1, P2, P3]): the closes at the quarter start and
/// at the three month ends. Buy and hold: month 1 uses the start weights; in month j each weight has drifted with its own
/// price, w x P(j-1) / P0 (record R3, the same rule as layer 2). Return = sum(w x r) / sum(w).
pub fn group_month_returns(members: &[(f64, [f64; 4])]) -> [f64; 3] {
    let mut out = [0.0; 3];
    for (j, o) in out.iter_mut().enumerate() {
        let (mut num, mut den) = (0.0, 0.0);
        for (w, p) in members {
            let wj = w * p[j] / p[0];
            num += wj * (p[j + 1] / p[j] - 1.0);
            den += wj;
        }
        *o = num / den;
    }
    out
}

/// The shifts that count: `min..=n-min` (a shift of -k on a circle is +(n-k)).
pub fn shifts(n: usize, min: usize) -> Vec<usize> {
    (min..=n - min).collect()
}

/// p = (1 + count) / (1 + number of shifts).
pub fn placebo_p(count: usize, n_shifts: usize) -> f64 {
    (1.0 + count as f64) / (1.0 + n_shifts as f64)
}

/// The words of rule 3.5. Monthly: out-of-sample R2 above 0 AND p below 0.10. Quarterly (URA-F): p at or below 0.10, no OOS check.
pub fn words(quarterly: bool, oos_r2: Option<f64>, p: f64) -> &'static str {
    let ok = if quarterly { p <= P_LIMIT + 1e-12 } else { oos_r2.is_some_and(|r| r > 0.0) && p < P_LIMIT };
    if ok {
        "consistent with the drivers"
    } else {
        "not shown with these drivers"
    }
}

/// OOS R2 against the mean of the fit window, and the sum of the errors. `preds[i]` predicts `y[start + i]`; the benchmark
/// of row t is the mean of `y[..t]`. Returns (R2, sum of y - pred).
pub fn oos_stats(y: &[f64], preds: &[f64], start: usize) -> (f64, f64) {
    let (mut sse, mut sst, mut sum) = (0.0, 0.0, 0.0);
    for (i, p) in preds.iter().enumerate() {
        let t = start + i;
        let mean = y[..t].iter().sum::<f64>() / t as f64;
        sse += (y[t] - p).powi(2);
        sst += (y[t] - mean).powi(2);
        sum += y[t] - p;
    }
    (1.0 - sse / sst, sum)
}

fn rows_of(cols: &[&[f64]], n: usize) -> Vec<Vec<f64>> {
    (0..n).map(|t| std::iter::once(1.0).chain(cols.iter().map(|c| c[t])).collect()).collect()
}

fn r2_of(y: &[f64], cols: &[&[f64]]) -> f64 {
    ols(&rows_of(cols, y.len()), y).map_or(f64::NAN, |f| f.r2)
}

// ----------------------------------------------------------------------------------------------
// One model
// ----------------------------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
pub struct Coef {
    pub name: String,
    pub beta: f64,
    pub se_nw: f64,
    pub t: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct Oos {
    pub n: usize,
    pub r2: f64,
    /// Sum of the prediction errors of the out-of-sample months, percentage points of log return.
    pub error_sum_pp: f64,
    /// Sum of the log returns of the same months, percentage points.
    pub return_sum_pp: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct Yearly {
    pub year: String,
    pub n: usize,
    pub coefs: Vec<(String, f64)>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Joint {
    pub r2_full: f64,
    pub r2_without_s_c: f64,
    pub r2_s_c_only: f64,
    pub joint_contribution: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct Model {
    pub name: String,
    pub n: usize,
    pub df: usize,
    pub lag: usize,
    pub r2: f64,
    pub unexplained_in_sample: f64,
    pub coefs: Vec<Coef>,
    /// Intercept x n, percentage points of log return (not an explanation).
    pub intercept_total_pp: f64,
    pub oos: Option<Oos>,
    pub shifts: usize,
    pub shifts_at_or_above: usize,
    pub placebo_p: f64,
    pub words: String,
    pub joint_s_c: Option<Joint>,
    pub yearly: Vec<Yearly>,
}

/// Fit one model. `labels` are the period end dates (for the yearly lines). Quarterly: lag 1, shifts 2..=10, no OOS.
pub fn analyse(name: &str, y: &[f64], names: &[&str], cols: &[&[f64]], labels: &[String], quarterly: bool) -> Result<Model, String> {
    let n = y.len();
    let x = rows_of(cols, n);
    let lag = if quarterly { QUARTERLY_LAG } else { MONTHLY_LAG };
    let fit = ols(&x, y).ok_or_else(|| format!("{name}: the fit failed (too few rows or a singular matrix)"))?;
    let se = nw_se(&x, &fit, lag);
    let mut cn = vec!["intercept".to_string()];
    cn.extend(names.iter().map(|s| s.to_string()));
    let coefs: Vec<Coef> = cn.iter().enumerate().map(|(i, c)| Coef { name: c.clone(), beta: fit.beta[i], se_nw: se[i], t: fit.beta[i] / se[i] }).collect();
    // OOS.
    let oos = if quarterly {
        None
    } else {
        let preds = expanding_preds(&x, y, FIT_MONTHS).ok_or_else(|| format!("{name}: the expanding fit failed"))?;
        let (r2, err) = oos_stats(y, &preds, FIT_MONTHS);
        Some(Oos { n: preds.len(), r2, error_sum_pp: err * 100.0, return_sum_pp: y[FIT_MONTHS..].iter().sum::<f64>() * 100.0 })
    };
    // Placebo: all drivers shifted together.
    let sh = shifts(n, if quarterly { QUARTERLY_MIN_SHIFT } else { MONTHLY_MIN_SHIFT });
    let mut count = 0;
    for k in &sh {
        let shifted: Vec<Vec<f64>> = cols.iter().map(|c| (0..n).map(|t| c[(t + n - k) % n]).collect()).collect();
        let refs: Vec<&[f64]> = shifted.iter().map(|v| v.as_slice()).collect();
        if r2_of(y, &refs) >= fit.r2 {
            count += 1;
        }
    }
    let p = placebo_p(count, sh.len());
    // S and C together.
    let joint_s_c = if names.contains(&"S") && names.contains(&"C") {
        let rest: Vec<&[f64]> = names.iter().zip(cols).filter(|(nm, _)| **nm != "S" && **nm != "C").map(|(_, c)| *c).collect();
        let sc: Vec<&[f64]> = names.iter().zip(cols).filter(|(nm, _)| **nm == "S" || **nm == "C").map(|(_, c)| *c).collect();
        let without = r2_of(y, &rest);
        Some(Joint { r2_full: fit.r2, r2_without_s_c: without, r2_s_c_only: r2_of(y, &sc), joint_contribution: fit.r2 - without })
    } else {
        None
    };
    // Yearly coefficients, description only.
    let mut yearly = Vec::new();
    for year in ["2024", "2025", "2026"] {
        let idx: Vec<usize> = (0..n).filter(|t| labels[*t].starts_with(year)).collect();
        let yy: Vec<f64> = idx.iter().map(|t| y[*t]).collect();
        let xs: Vec<Vec<f64>> = idx.iter().map(|t| x[*t].clone()).collect();
        let coefs = ols(&xs, &yy).map(|f| cn.iter().cloned().zip(f.beta).collect()).unwrap_or_default();
        yearly.push(Yearly { year: year.into(), n: idx.len(), coefs });
    }
    Ok(Model {
        name: name.into(),
        n,
        df: n - cols.len() - 1,
        lag,
        r2: fit.r2,
        unexplained_in_sample: 1.0 - fit.r2,
        intercept_total_pp: fit.beta[0] * n as f64 * 100.0,
        coefs,
        words: words(quarterly, oos.as_ref().map(|o| o.r2), p).into(),
        oos,
        shifts: sh.len(),
        shifts_at_or_above: count,
        placebo_p: p,
        joint_s_c,
        yearly,
    })
}

// ----------------------------------------------------------------------------------------------
// Data
// ----------------------------------------------------------------------------------------------

/// What a U reader returns: the monthly series ("YYYY-MM" to price), the read date and the SHA-256 of the series bytes.
pub struct URead {
    pub months: BTreeMap<String, f64>,
    pub read_date: String,
    pub sha256: String,
}

/// The reader of U. No IMF reader exists yet (Technology owns readers): this one says so.
pub fn no_u_reader() -> Result<URead, String> {
    Err("U not available (no IMF uranium reader in the program)".into())
}

fn ln_ratio(rows: &[crate::market::prices::PricePoint], d0: &str, d1: &str, t: &str) -> Result<f64, String> {
    match (close_on(rows, d0), close_on(rows, d1)) {
        (Some(a), Some(b)) if a > 0.0 && b > 0.0 => Ok((b / a).ln()),
        _ => Err(format!("{t}: no close on or before {d0} or {d1}")),
    }
}

fn price_log_returns(cache: &Path, t: &str, dates: &[String]) -> Result<Vec<f64>, String> {
    let rows = price_cache::history(cache, t).ok_or_else(|| format!("{t}: no price file in the cache (fa prices get {t} --source nasdaq)"))?;
    (1..dates.len()).map(|i| ln_ratio(&rows, &dates[i - 1], &dates[i], t)).collect()
}

/// The Treasury real 10Y par yield (percentage points) on or before each date, from DIR/rates/treasury_real_<year>.csv.
fn real_yield_changes(cache: &Path, dates: &[String]) -> Result<Vec<f64>, String> {
    let mut curves = crate::rates::CurveByDay::new();
    for year in 2023..=2026 {
        let p = cache.join("rates").join(format!("treasury_real_{year}.csv"));
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        curves.extend(crate::rates::parse_treasury_csv(&text)?);
    }
    let level = |d: &str| -> Result<f64, String> {
        let day = mapcheck::dates::parse(d).ok_or("bad date")?;
        let (dd, c) = curves.range(..=day).next_back().ok_or_else(|| format!("no real curve on or before {d}"))?;
        if day - dd > MAX_STALE {
            return Err(format!("the newest real curve before {d} is {} days old", day - dd));
        }
        c.get("10Y").map(|v| v * 100.0).ok_or_else(|| format!("no 10Y real yield on {d}"))
    };
    let lv: Vec<f64> = dates.iter().map(|d| level(d)).collect::<Result<_, _>>()?;
    Ok(lv.windows(2).map(|w| w[1] - w[0]).collect())
}

fn log_changes_of_months(months: &BTreeMap<String, f64>, dates: &[String], what: &str) -> Result<Vec<f64>, String> {
    let v: Vec<f64> = dates.iter().map(|d| months.get(&d[..7]).copied().ok_or_else(|| format!("{what}: no value for {}", &d[..7]))).collect::<Result<_, _>>()?;
    Ok(v.windows(2).map(|w| (w[1] / w[0]).ln()).collect())
}

/// The URA outcomes. M and R: 36 monthly log returns. F: quarterly log returns (all holdings, and without Daewoo E&C).
#[derive(Debug, Default)]
pub struct UraOutcomes {
    pub m: Vec<f64>,
    pub r: Vec<f64>,
    pub f: Vec<f64>,
    pub f_ex_daewoo: Vec<f64>,
    pub notes: Vec<String>,
}

type Prices = HashMap<String, Option<Vec<PricePoint>>>;

/// The report that gives the weights of a quarter: the one dated at the quarter start, no other (no look-ahead).
pub fn weight_report<'a>(reports: &'a [Report], quarter_start: &str) -> Option<&'a Report> {
    reports.iter().find(|r| r.report_date == quarter_start)
}

/// Record R4 (proposed): the 12 quarters of URA-F are the quarters with public N-PORT reports, 2023-07-31 to 2026-07-31.
/// The 37 month-end dates 2023-07-31 to 2026-07-31; quarter k runs from date 3k to date 3k+3.
pub fn f_dates() -> Vec<String> {
    month_ends_from((2023, 8), N_MONTHS)
}

/// Group month returns of URA-M and URA-R from the reports and the Nasdaq closes. `dates` has 37 month-ends.
pub fn group_outcomes(reports: &[Report], prices: &Prices, map: &HashMap<String, String>, dates: &[String]) -> Result<UraOutcomes, String> {
    let mut out = UraOutcomes::default();
    for qi in 0..(dates.len() - 1) / 3 {
        let q_dates = &dates[3 * qi..=3 * qi + 3];
        let r0 = weight_report(reports, &q_dates[0]).ok_or_else(|| format!("URA: no N-PORT report of {} (the weights of the quarter)", q_dates[0]))?;
        let net0 = r0.net_assets.filter(|n| *n > 0.0).ok_or_else(|| format!("URA {}: no net assets", r0.report_date))?;
        let (pos, _) = positions(r0);
        for (group, list, sink) in [("URA-M", URA_M.as_slice(), &mut out.m), ("URA-R", URA_R.as_slice(), &mut out.r)] {
            let mut members: Vec<(f64, [f64; 4])> = Vec::new();
            let mut dropped: Vec<(String, f64)> = Vec::new();
            for p in &pos {
                let Some(t) = map.get(&p.isin).or_else(|| map.get(&p.cusip)) else { continue };
                if !list.contains(&t.as_str()) {
                    continue;
                }
                let w = p.val_usd / net0;
                let rows = prices.get(t).and_then(|o| o.as_ref());
                let px: Option<Vec<f64>> = rows.and_then(|rows| q_dates.iter().map(|d| close_on(rows, d)).collect());
                match px {
                    Some(px) if px.iter().all(|v| *v > 0.0) => members.push((w, [px[0], px[1], px[2], px[3]])),
                    _ => dropped.push((t.clone(), w)),
                }
            }
            if members.is_empty() {
                return Err(format!("{group}: no member with prices in the quarter to {}", q_dates[3]));
            }
            if !dropped.is_empty() {
                let kept: f64 = members.iter().map(|m| m.0).sum();
                let lost: f64 = dropped.iter().map(|m| m.1).sum();
                let names: Vec<String> = dropped.iter().map(|(t, w)| format!("{t} (start weight {:.4})", w)).collect();
                out.notes.push(format!(
                    "{group} quarter to {}: left out for a missing close: {}. Weights of the members kept sum to {kept:.4} of the fund; the left-out weights {lost:.4}; the group return divides by the kept sum (renormalised to 1)",
                    q_dates[3],
                    names.join(", ")
                ));
            }
            sink.extend(group_month_returns(&members).iter().map(|r| (1.0 + r).ln()));
        }
    }
    Ok(out)
}

/// The foreign-group return of one quarter as a log: sum of the layer-2 parts / sum of the start weights of the foreign rows.
/// `exclude_daewoo` drops the row with the ISIN of record R1.
pub fn foreign_return(q: &Quarter, exclude_daewoo: bool) -> Result<f64, String> {
    let rows: Vec<_> = q.rows.iter().filter(|r| r.group == Group::Foreign && !(exclude_daewoo && r.isin == DAEWOO_ISIN)).collect();
    let w: f64 = rows.iter().map(|r| r.w0).sum();
    let c: f64 = rows.iter().map(|r| r.contrib).sum();
    if w <= 0.0 {
        return Err(format!("URA-F: no foreign weight in the quarter to {}", q.d1));
    }
    Ok((1.0 + c / w).ln())
}

/// URA-F: exactly 12 quarters, 2023-07-31 to 2026-07-31 (record R4). A missing report is an error, not a shorter sample.
pub fn foreign_outcomes(reports: &[Report], class_id: &str, map: &HashMap<String, String>, dates: &[String]) -> Result<(Vec<f64>, Vec<f64>), String> {
    let (mut f, mut fx) = (vec![], vec![]);
    for qi in 0..N_MONTHS / 3 {
        let (d0, d1) = (&dates[3 * qi], &dates[3 * qi + 3]);
        let (Some(r0), Some(r1)) = (weight_report(reports, d0), weight_report(reports, d1)) else {
            return Err(format!("URA-F needs the 12 quarters 2023-07-31 to 2026-07-31; the N-PORT report of {d0} or {d1} is missing (quarters found: {qi} of 12)"));
        };
        let q = attribute(r0, r1, class_id, map)?;
        f.push(foreign_return(&q, false)?);
        fx.push(foreign_return(&q, true)?);
    }
    Ok((f, fx))
}

fn ura_outcomes(cache: &Path, dates: &[String]) -> Result<UraOutcomes, String> {
    let map = parse_map(MAP_CSV);
    let path = cache.join("_sec").join("company_tickers_mf.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e} (the share class of URA is needed)", path.display()))?;
    let s = series_of(&text, "URA").map_err(|e| format!("URA in {}: {e}", path.display()))?;
    let (loaded, _) = select_reports(read_cached_reports(cache, "URA", Some(&s.series_id)));
    let reports: Vec<Report> = loaded.into_iter().map(|l| l.report).collect();
    let mut prices: Prices = HashMap::new();
    for t in URA_M.iter().chain(URA_R.iter()) {
        prices.insert((*t).to_string(), price_cache::history(cache, t));
    }
    let mut out = group_outcomes(&reports, &prices, &map, dates)?;
    let (f, fx) = foreign_outcomes(&reports, &s.class_id, &map, &f_dates())?;
    out.f = f;
    out.f_ex_daewoo = fx;
    Ok(out)
}

// ----------------------------------------------------------------------------------------------
// Run
// ----------------------------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct Identity {
    pub n: usize,
    pub correlation: f64,
    pub slope: f64,
    pub r2: f64,
    pub mean_abs_difference_pp: f64,
}

#[derive(Debug, Serialize)]
pub struct Results {
    pub design: &'static str,
    pub u_read_date: Option<String>,
    pub u_sha256: Option<String>,
    pub models: Vec<Model>,
    pub sensitivity_f_without_daewoo: Option<Model>,
    pub not_run: Vec<String>,
    pub notes: Vec<String>,
    pub gld_identity: Option<Identity>,
}

pub fn identity(gld: &[f64], g: &[f64]) -> Option<Identity> {
    let n = gld.len();
    let f = ols(&rows_of(&[g], n), gld)?;
    let (mg, mx) = (gld.iter().sum::<f64>() / n as f64, g.iter().sum::<f64>() / n as f64);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in gld.iter().zip(g) {
        sxy += (a - mg) * (b - mx);
        sxx += (b - mx).powi(2);
        syy += (a - mg).powi(2);
    }
    Some(Identity { n, correlation: sxy / (sxx * syy).sqrt(), slope: f.beta[1], r2: f.r2, mean_abs_difference_pp: gld.iter().zip(g).map(|(a, b)| (a - b).abs()).sum::<f64>() / n as f64 * 100.0 })
}

/// The inputs that come from files (the cache), each as a result so that a missing file is a note and not a stop.
pub struct Inputs<'a> {
    pub s: Vec<f64>,
    pub c: Vec<f64>,
    pub d: Vec<f64>,
    pub gld: Vec<f64>,
    pub r: Result<Vec<f64>, String>,
    pub g: Result<Vec<f64>, String>,
    /// The URA outcomes (called only when U is read).
    pub ura: &'a dyn Fn() -> Result<UraOutcomes, String>,
}

/// The whole study from the cache. `u_reader` is called once.
pub fn run_study(cache: &Path, u_reader: &dyn Fn() -> Result<URead, String>) -> Result<Results, String> {
    let dates = month_ends();
    let g = commodity::newest(&commodity::series_dir(cache))
        .ok_or_else(|| "no World Bank file in the cache (fa commodity get gold)".to_string())
        .and_then(|p| std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display())))
        .and_then(|b| commodity::read_series(&b, &commodity::SERIES[0]))
        .and_then(|p| log_changes_of_months(&p.months, &dates, "G"));
    let ura = || ura_outcomes(cache, &dates);
    let inp = Inputs {
        s: price_log_returns(cache, "SPY", &dates)?,
        c: price_log_returns(cache, "SMH", &dates)?,
        d: price_log_returns(cache, "UUP", &dates)?,
        gld: price_log_returns(cache, "GLD", &dates)?,
        r: real_yield_changes(cache, &dates),
        g,
        ura: &ura,
    };
    run_core(inp, u_reader)
}

pub fn run_core(inp: Inputs, u_reader: &dyn Fn() -> Result<URead, String>) -> Result<Results, String> {
    let dates = month_ends();
    let fd = f_dates();
    let labels: Vec<String> = dates[1..].to_vec();
    let q_labels: Vec<String> = (0..N_MONTHS / 3).map(|i| fd[3 * i + 3].clone()).collect();
    let mut res = Results { design: "analysis/macro/2026-11-25-fund-drivers-design.md (R1, R2, R3, R4)", u_read_date: None, u_sha256: None, models: vec![], sensitivity_f_without_daewoo: None, not_run: vec![], notes: vec![], gld_identity: None };
    let (s, c, d) = (&inp.s, &inp.c, &inp.d);
    match u_reader() {
        Ok(u) => {
            res.u_read_date = Some(u.read_date.clone());
            res.u_sha256 = Some(u.sha256.clone());
            let um = log_changes_of_months(&u.months, &dates, "U")?;
            let uf = log_changes_of_months(&u.months, &fd, "U (URA-F window)")?;
            let ura = (inp.ura)()?;
            res.notes.extend(ura.notes.clone());
            for (nm, y) in [("URA-M", &ura.m), ("URA-R", &ura.r)] {
                res.models.push(analyse(nm, y, &["U", "S", "C"], &[&um, s, c], &labels, false)?);
            }
            // Quarterly U: the sum of the three monthly log changes of the quarter (IMF months 2023-08 to 2026-07).
            let uq: Vec<f64> = (0..N_MONTHS / 3).map(|i| uf[3 * i..3 * i + 3].iter().sum()).collect();
            res.models.push(analyse("URA-F", &ura.f, &["U"], &[&uq], &q_labels, true)?);
            res.sensitivity_f_without_daewoo = Some(analyse("URA-F without Daewoo E&C (R1, sensitivity only)", &ura.f_ex_daewoo, &["U"], &[&uq], &q_labels, true)?);
        }
        Err(e) => res.not_run.push(format!("URA-M, URA-R, URA-F: {e}")),
    }
    match &inp.r {
        Ok(r) => res.models.push(analyse("GLD", &inp.gld, &["R", "D"], &[r, d], &labels, false)?),
        Err(e) => res.not_run.push(format!("GLD: R not available ({e})")),
    }
    match &inp.g {
        Ok(g) => res.gld_identity = identity(&inp.gld, g),
        Err(e) => res.not_run.push(format!("GLD identity: {e}")),
    }
    Ok(res)
}


pub fn report_text(r: &Results) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "FUND DRIVERS (frozen design {}): a fit is not a cause; log returns; rates in percentage points", r.design);
    match (&r.u_read_date, &r.u_sha256) {
        (Some(d), Some(h)) => {
            let _ = writeln!(s, "U read on {d}; SHA-256 of the series bytes {h}. The series is not stored. A new run can change if the IMF revises a value.");
        }
        _ => {
            let _ = writeln!(s, "U not available");
        }
    }
    let n = r.models.len();
    let _ = writeln!(s, "Models that ran: {n}. With {n} models at 10%, a false 'consistent' is possible; the out-of-sample condition lowers that chance.");
    let mut all: Vec<(&Model, bool)> = r.models.iter().map(|m| (m, true)).collect();
    all.extend(r.sensitivity_f_without_daewoo.iter().map(|m| (m, false)));
    for (m, with_words) in all {
        let _ = writeln!(s, "\n{}: n {}, df {}, Newey-West lag {}", m.name, m.n, m.df, m.lag);
        let _ = writeln!(s, "  R2 {:.3}; unexplained in sample (a) {:.3}", m.r2, m.unexplained_in_sample);
        for c in &m.coefs {
            let _ = writeln!(s, "  {:<10} {:>9.4}  se {:>8.4}  t {:>6.2}", c.name, c.beta, c.se_nw, c.t);
        }
        let _ = writeln!(s, "  intercept x n (separate line, not an explanation): {:+.2} pp", m.intercept_total_pp);
        match &m.oos {
            Some(o) => {
                let _ = writeln!(s, "  out of sample ({} months, expanding window, against the fit-window mean): R2 {:.3}; (b) sum of errors {:+.2} pp next to the return of those months {:+.2} pp", o.n, o.r2, o.error_sum_pp, o.return_sum_pp);
            }
            None => {
                let _ = writeln!(s, "  no out-of-sample check (too few quarters); with {} shifts the smallest p is {:.2}, so the rule is p <= 0.10", m.shifts, placebo_p(0, m.shifts));
            }
        }
        let _ = writeln!(s, "  placebo: {} shifts, {} at or above the true R2, p {:.3}", m.shifts, m.shifts_at_or_above, m.placebo_p);
        if with_words {
            let _ = writeln!(s, "  words: {}", m.words);
        } else {
            let _ = writeln!(s, "  (sensitivity line only: no words; the words use the fit with all holdings, record R1)");
        }
        if let Some(j) = &m.joint_s_c {
            let _ = writeln!(s, "  S and C together: R2 full {:.3}, without S and C {:.3}, S and C only {:.3}, joint contribution {:.3}", j.r2_full, j.r2_without_s_c, j.r2_s_c_only, j.joint_contribution);
        }
        for y in &m.yearly {
            let c: Vec<String> = y.coefs.iter().map(|(n, b)| format!("{n} {b:.3}")).collect();
            let _ = writeln!(s, "  year {} (n {}, description only): {}", y.year, y.n, if c.is_empty() { "too few rows".into() } else { c.join(", ") });
        }
    }
    if let Some(i) = &r.gld_identity {
        let _ = writeln!(s, "\nGLD identity against the World Bank gold change (not a driver; monthly average, lags about half a month): n {}, correlation {:.3}, slope {:.3}, R2 {:.3}, mean absolute difference {:.2} pp",
            i.n, i.correlation, i.slope, i.r2, i.mean_abs_difference_pp);
    }
    for n in r.not_run.iter().chain(&r.notes) {
        let _ = writeln!(s, "note: {n}");
    }
    let _ = writeln!(s, "limits: 36 months is a small sample; weights change at quarter ends only; URA-F closes quarterly; the IMF series is a monthly average with its own method; a fit is not a cause.");
    s
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

pub fn command(repo: &Path, args: &[String]) -> Result<i32, CmdError> {
    use CmdError::{Data, Usage};
    if args.iter().any(|x| x == "-h" || x == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let mut i = 0;
    while i < args.len() {
        if args[i] != "--cache" && args[i] != "--out" {
            return Err(Usage(format!("unknown argument '{}'", args[i])));
        }
        if args.get(i + 1).is_none_or(|v| v.starts_with("--")) {
            return Err(Usage(format!("{} needs a value", args[i])));
        }
        i += 2;
    }
    let cache = flag(args, "--cache").map_or_else(|| repo.join("data").join("raw"), |c| if PathBuf::from(&c).is_absolute() { PathBuf::from(c) } else { repo.join(c) });
    let res = run_study(&cache, &no_u_reader).map_err(Data)?;
    print!("{}", report_text(&res));
    if let Some(out) = flag(args, "--out") {
        let json = serde_json::to_string_pretty(&res).map_err(|e| Data(e.to_string()))?;
        std::fs::write(&out, json + "\n").map_err(|e| Data(format!("cannot write {out}: {e}")))?;
        println!("\nwrote {out}");
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn groups_are_the_record_r3_lists() {
        assert_eq!(URA_M.len() + URA_R.len(), 16);
        assert!(URA_M.iter().all(|t| !URA_R.contains(t)));
        let map = parse_map(MAP_CSV);
        for t in URA_M.iter().chain(URA_R.iter()) {
            assert!(map.values().any(|v| v == t), "{t} not in the ticker map");
        }
    }

    #[test]
    fn month_end_list() {
        let d = month_ends();
        assert_eq!(d.len(), 37);
        assert_eq!(d[0], "2023-10-31");
        assert_eq!(d[1], "2023-11-30");
        assert_eq!(d[3], "2024-01-31");
        assert_eq!(d[4], "2024-02-29");
        assert_eq!(d[36], "2026-10-31");
    }

    /// OLS with intercept: x = 0,1,2 and y = 1,3,2 give slope 0.5, intercept 1.5, R2 0.25; intercept x n x 100 = 450 pp.
    #[test]
    fn ols_fixed_answer() {
        let x = rows_of(&[&[0.0, 1.0, 2.0]], 3);
        let f = ols(&x, &[1.0, 3.0, 2.0]).unwrap();
        assert!(close(f.beta[0], 1.5) && close(f.beta[1], 0.5) && close(f.r2, 0.25));
    }

    /// Newey-West lag 1 for x = 0,1,2, y = 1,3,2 (hand calculation in the test text of the module note):
    /// S = [[0.5, 0.5], [0.5, 1]], slope row of (X'X)^-1 = (-0.5, 0.5), variance 0.125.
    #[test]
    fn newey_west_lag_one_fixed_answer() {
        let x = rows_of(&[&[0.0, 1.0, 2.0]], 3);
        let f = ols(&x, &[1.0, 3.0, 2.0]).unwrap();
        let se = nw_se(&x, &f, 1);
        assert!(close(se[1], 0.125_f64.sqrt()), "{}", se[1]);
    }

    /// y = 1,3,2,4; start 2; predictions 2 and 3 for rows 2 and 3.
    /// Benchmarks: mean(1,3) = 2 and mean(1,3,2) = 2. SSE = 0 + 1 = 1; SST = 0 + 4 = 4; R2 = 0.75; sum of errors = 1.
    #[test]
    fn oos_r2_fixed_answer() {
        let (r2, e) = oos_stats(&[1.0, 3.0, 2.0, 4.0], &[2.0, 3.0], 2);
        assert!(close(r2, 0.75) && close(e, 1.0), "{r2} {e}");
        // A perfect line gives OOS R2 = 1 through the expanding fit.
        let xv: Vec<f64> = (0..8).map(|i| (i * i % 5) as f64 + i as f64).collect();
        let y: Vec<f64> = xv.iter().map(|v| 1.0 + 2.0 * v).collect();
        let p = expanding_preds(&rows_of(&[&xv], 8), &y, 4).unwrap();
        assert!((oos_stats(&y, &p, 4).0 - 1.0).abs() < 1e-8);
    }

    #[test]
    fn placebo_count_and_p() {
        assert_eq!(shifts(36, MONTHLY_MIN_SHIFT).len(), 25);
        assert_eq!(shifts(36, MONTHLY_MIN_SHIFT).first(), Some(&6));
        assert_eq!(shifts(36, MONTHLY_MIN_SHIFT).last(), Some(&30));
        assert_eq!(shifts(12, QUARTERLY_MIN_SHIFT).len(), 9);
        assert_eq!(shifts(12, QUARTERLY_MIN_SHIFT).last(), Some(&10));
        assert!(close(placebo_p(0, 25), 1.0 / 26.0));
        assert!(close(placebo_p(0, 9), 0.1));
        assert!(close(placebo_p(1, 9), 0.2));
        assert!(close(placebo_p(2, 25), 3.0 / 26.0));
    }

    /// A driver that equals the outcome: no shift reaches R2 = 1, so count is 0 and p is 1/26 (monthly).
    #[test]
    fn placebo_in_a_model() {
        let y: Vec<f64> = (0..36).map(|i| ((i * 7 + 3) % 11) as f64 - 5.0).collect();
        let labels: Vec<String> = month_ends()[1..].to_vec();
        let m = analyse("T", &y, &["A"], &[&y], &labels, false).unwrap();
        assert!(close(m.r2, 1.0));
        assert_eq!((m.shifts, m.shifts_at_or_above), (25, 0));
        assert!(close(m.placebo_p, 1.0 / 26.0));
        assert_eq!(m.df, 34);
        assert!(m.oos.is_some());
    }

    #[test]
    fn word_rule() {
        assert_eq!(words(false, Some(0.1), 0.05), "consistent with the drivers");
        assert_eq!(words(false, Some(-0.1), 0.05), "not shown with these drivers");
        assert_eq!(words(false, Some(0.1), 0.10), "not shown with these drivers");
        assert_eq!(words(false, None, 0.01), "not shown with these drivers");
        // URA-F: p at or below 0.10 and no OOS check.
        assert_eq!(words(true, None, 0.1), "consistent with the drivers");
        assert_eq!(words(true, None, 0.2), "not shown with these drivers");
    }

    /// Group return with drift. A: weight 1, closes 100, 110, 121, 121. B: weight 3, closes 100, 100, 100, 200.
    /// Month 1: weights 1 and 3: (1 x 0.10 + 3 x 0) / 4 = 0.025.
    /// Month 2: A has drifted to 1.1, B stays 3: (1.1 x 0.10 + 0) / 4.1.
    /// Month 3: A 1.21, B 3: (0 + 3 x 1.0) / 4.21.
    #[test]
    fn group_return_with_drift() {
        let r = group_month_returns(&[(1.0, [100.0, 110.0, 121.0, 121.0]), (3.0, [100.0, 100.0, 100.0, 200.0])]);
        assert!(close(r[0], 0.025));
        assert!((r[1] - 0.11 / 4.1).abs() < 1e-12, "{}", r[1]);
        assert!((r[2] - 3.0 / 4.21).abs() < 1e-12, "{}", r[2]);
        // Without drift month 3 would be (1 x 0 + 3 x 1) / 4 = 0.75: the drift test is not the same number.
        assert!((r[2] - 0.75).abs() > 0.03);
        // One member: its own return.
        let one = group_month_returns(&[(0.07, [10.0, 11.0, 9.9, 9.9])]);
        assert!(close(one[0], 0.1) && (one[1] + 0.1).abs() < 1e-12 && one[2].abs() < 1e-12);
    }

    #[test]
    fn identity_of_equal_series() {
        let a = [0.01, -0.02, 0.03, 0.0];
        let i = identity(&a, &a).unwrap();
        assert!(close(i.correlation, 1.0) && close(i.slope, 1.0) && i.mean_abs_difference_pp.abs() < 1e-12);
    }

    #[test]
    fn without_a_u_reader_the_uranium_models_are_not_run() {
        let e = no_u_reader().err().unwrap();
        assert!(e.contains("U not available"));
    }

    // ---- record R4 and review tests ----

    #[test]
    fn f_window_is_the_twelve_public_quarters() {
        let d = f_dates();
        assert_eq!(d.len(), 37);
        assert_eq!(d[0], "2023-07-31");
        assert_eq!(d[3], "2023-10-31");
        assert_eq!(d[33], "2026-04-30");
        assert_eq!(d[36], "2026-07-31");
        assert_eq!(shifts(N_MONTHS / 3, QUARTERLY_MIN_SHIFT).len(), 9);
    }

    #[test]
    fn fewer_than_twelve_quarters_is_an_error() {
        let map = parse_map(MAP_CSV);
        let e = foreign_outcomes(&[], "C000", &map, &f_dates()).unwrap_err();
        assert!(e.contains("12 quarters") && e.contains("2023-07-31") && e.contains("0 of 12"), "{e}");
    }

    fn frow(name: &str, isin: &str, group: Group, w0: f64, contrib: f64) -> crate::market::fund_attribution::Row {
        crate::market::fund_attribution::Row {
            name: name.into(),
            isin: isin.into(),
            group,
            ticker: None,
            method: crate::market::fund_attribution::Method::Isin,
            w0,
            p0: 1.0,
            p1: 1.0,
            ret: 0.0,
            contrib,
            bal_ratio: 1.0,
            split: None,
            trade_est: 0.0,
            flag: None,
        }
    }

    /// Foreign rows: A weight 0.2 part 0.02 (return 10%); Daewoo weight 0.1 part 0.03 (30%). A US row is ignored.
    /// All: 0.05 / 0.3 = 1/6. Without Daewoo: 0.02 / 0.2 = 0.1.
    #[test]
    fn daewoo_is_excluded_from_the_r1_outcome() {
        let q = Quarter {
            d1: "2024-01-31".into(),
            rows: vec![frow("A", "KR0000000001", Group::Foreign, 0.2, 0.02), frow("Daewoo E&C", DAEWOO_ISIN, Group::Foreign, 0.1, 0.03), frow("US", "US0000000001", Group::Us, 0.5, 0.5)],
            ..Quarter::default()
        };
        assert!(close(foreign_return(&q, false).unwrap(), (1.0 + 1.0 / 6.0_f64).ln()));
        assert!(close(foreign_return(&q, true).unwrap(), 1.1_f64.ln()));
        let only = Quarter { rows: vec![frow("Daewoo E&C", DAEWOO_ISIN, Group::Foreign, 0.1, 0.03)], ..Quarter::default() };
        assert!(foreign_return(&only, true).is_err());
    }

    /// x = 0,1,2,3 and y = 0,2,1,3: slope 0.8, intercept 0.3, residuals -0.3, 0.9, -0.9, 0.3.
    /// Lag 3, weights 3/4, 1/2, 1/4. S0 = [[1.8, 2.7], [2.7, 4.86]]; Gamma1 = [[-2.7, -4.05], [-4.05, -6.48]];
    /// Gamma2 = [[1.08, 1.62], [1.62, 1.62]]; Gamma3 = [[-0.18, -0.27], [-0.27, 0]].
    /// S = [[0.27, 0.405], [0.405, 0.81]]. (X'X)^-1 = [[0.7, -0.3], [-0.3, 0.2]].
    /// Slope variance = 0.09 x 0.27 - 2 x 0.06 x 0.405 + 0.04 x 0.81 = 0.0081; se 0.09.
    /// Intercept variance = 0.49 x 0.27 - 2 x 0.21 x 0.405 + 0.09 x 0.81 = 0.0351.
    #[test]
    fn newey_west_lag_three_by_hand() {
        let x = rows_of(&[&[0.0, 1.0, 2.0, 3.0]], 4);
        let f = ols(&x, &[0.0, 2.0, 1.0, 3.0]).unwrap();
        assert!(close(f.beta[0], 0.3) || (f.beta[0] - 0.3).abs() < 1e-12);
        assert!((f.beta[1] - 0.8).abs() < 1e-12);
        let se = nw_se(&x, &f, 3);
        assert!((se[1] - 0.09).abs() < 1e-12, "{}", se[1]);
        assert!((se[0] - 0.0351_f64.sqrt()).abs() < 1e-12, "{}", se[0]);
    }

    fn rep(date: &str, w: [f64; 3]) -> Report {
        let h = |name: &str, isin: &str, v: f64| crate::market::fund_holdings::Holding {
            name: name.into(),
            cusip: String::new(),
            isin: isin.into(),
            val_usd: v,
            pct_val: 0.0,
            balance: 1.0,
            units: "NS".into(),
            currency: "USD".into(),
            exchange_rt: None,
            ticker: String::new(),
            asset_cat: "EC".into(),
            country: "CA".into(),
            payoff: "Long".into(),
        };
        Report {
            series_id: "S".into(),
            report_date: date.into(),
            form: "NPORT-P".into(),
            rep_pd_end: date.into(),
            tot_assets: Some(100.0),
            tot_liabs: Some(0.0),
            net_assets: Some(100.0),
            monthly: vec![],
            flows: [None, None, None],
            holdings: vec![h("Cameco", "CA13321L1085", w[0]), h("NexGen", "CA65340P1062", w[1]), h("Centrus", "US15643U1043", w[2])],
            bad_numbers: 0,
        }
    }

    fn pp_of(closes: [f64; 4]) -> Option<Vec<PricePoint>> {
        let d = ["2023-07-31", "2023-08-31", "2023-09-30", "2023-10-31"];
        Some(d.iter().zip(closes).map(|(d, c)| PricePoint { date: (*d).into(), open: c, high: c, low: c, close: c, volume: 1 }).collect())
    }

    fn fixture_prices() -> Prices {
        let mut p = Prices::new();
        p.insert("CCJ".into(), pp_of([100.0, 110.0, 121.0, 121.0]));
        p.insert("NXE".into(), pp_of([100.0, 100.0, 100.0, 200.0]));
        p.insert("LEU".into(), pp_of([10.0, 10.0, 10.0, 10.0]));
        p
    }

    fn q_dates() -> Vec<String> {
        vec!["2023-07-31".into(), "2023-08-31".into(), "2023-09-30".into(), "2023-10-31".into()]
    }

    /// The weights of the quarter come from the report dated at its start. The report at the end has other weights (CCJ 30,
    /// NXE 10): using it would give month 1 = (0.3 x 0.1 + 0)/0.4 = 0.075, not 0.025.
    #[test]
    fn weights_come_from_the_start_report_only() {
        let reports = vec![rep("2023-07-31", [10.0, 30.0, 10.0]), rep("2023-10-31", [30.0, 10.0, 10.0])];
        assert_eq!(weight_report(&reports, "2023-07-31").unwrap().report_date, "2023-07-31");
        assert!(weight_report(&reports, "2023-08-31").is_none());
        assert!(weight_report(&reports[1..], "2023-07-31").is_none());
        let map = parse_map(MAP_CSV);
        let o = group_outcomes(&reports, &fixture_prices(), &map, &q_dates()).unwrap();
        assert!(close(o.m[0], 1.025_f64.ln()), "{}", o.m[0]);
        assert!((o.m[1] - (1.0 + 0.11 / 4.1_f64).ln()).abs() < 1e-12);
        assert!((o.m[2] - (1.0 + 3.0 / 4.21_f64).ln()).abs() < 1e-12);
        assert!(close(o.r[0], 0.0));
        assert!(o.notes.is_empty());
        // Without the report of the quarter start: an error, not the later report.
        assert!(group_outcomes(&reports[1..], &fixture_prices(), &map, &q_dates()).is_err());
    }

    /// NexGen has no close: it is left out, the note names it and the kept weight sum (0.1 of the fund; left out 0.3).
    #[test]
    fn a_member_without_a_close_is_named_in_a_note() {
        let reports = vec![rep("2023-07-31", [10.0, 30.0, 10.0])];
        let mut p = fixture_prices();
        p.insert("NXE".into(), None);
        let o = group_outcomes(&reports, &p, &parse_map(MAP_CSV), &q_dates()).unwrap();
        assert_eq!(o.notes.len(), 1);
        assert!(o.notes[0].contains("NXE") && o.notes[0].contains("0.1000") && o.notes[0].contains("0.3000") && o.notes[0].contains("renormalised"), "{}", o.notes[0]);
        // CCJ alone: its own return.
        assert!(close(o.m[0], 1.1_f64.ln()));
    }

    /// 10-year real yield 1.80, 1.95, 1.70 percent on 2024-01-31, 2024-02-29, 2024-03-29 (the month end 03-31 is a Sunday).
    /// Changes in percentage points: +0.15 and -0.25.
    #[test]
    fn real_yield_change_in_pp_from_a_fixture() {
        let dir = std::env::temp_dir().join(format!("fa_fund_drivers_real_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("rates")).unwrap();
        for y in 2023..=2026 {
            let body = if y == 2024 { "01/31/2024,1.80\n02/29/2024,1.95\n03/29/2024,1.70\n" } else { "06/30/2023,1.0\n" };
            std::fs::write(dir.join("rates").join(format!("treasury_real_{y}.csv")), format!("Date,\"10 YR\"\n{body}")).unwrap();
        }
        let d: Vec<String> = vec!["2024-01-31".into(), "2024-02-29".into(), "2024-03-31".into()];
        let r = real_yield_changes(&dir, &d).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!((r[0] - 0.15).abs() < 1e-9 && (r[1] + 0.25).abs() < 1e-9, "{r:?}");
    }

    fn wave(i: usize, k: usize) -> f64 {
        (((i * 37 + k * 11 + i * i * 3) % 17) as f64 - 8.0) / 100.0
    }

    /// A fake U reader and fake outcomes run the URA path through `run_core`: three URA models, the sensitivity line,
    /// GLD, the read date and the SHA-256 line, the real count of models, and no words line for the sensitivity.
    #[test]
    fn fake_u_reader_runs_the_ura_path_end_to_end() {
        let col = |k: usize, n: usize| -> Vec<f64> { (0..n).map(|i| wave(i, k)).collect() };
        let ura = || {
            Ok(UraOutcomes { m: col(1, 36), r: col(2, 36), f: col(3, 12), f_ex_daewoo: col(4, 12), notes: vec!["a note".into()] })
        };
        let inp = Inputs { s: col(5, 36), c: col(6, 36), d: col(7, 36), gld: col(8, 36), r: Ok(col(9, 36)), g: Ok(col(8, 36)), ura: &ura };
        let u = || {
            let mut months = BTreeMap::new();
            for (i, d) in month_ends_from((2023, 7), 40).iter().enumerate() {
                months.insert(d[..7].to_string(), 100.0 * (1.0 + wave(i, 12) / 2.0));
            }
            Ok(URead { months, read_date: "2026-11-27".into(), sha256: "ab12".repeat(16) })
        };
        let res = run_core(inp, &u).unwrap();
        assert_eq!(res.models.len(), 4);
        assert_eq!(res.models[2].name, "URA-F");
        assert_eq!(res.models[2].n, 12);
        assert_eq!(res.models[2].shifts, 9);
        assert!(res.sensitivity_f_without_daewoo.is_some());
        assert!(res.gld_identity.is_some());
        let t = report_text(&res);
        assert!(t.contains("U read on 2026-11-27; SHA-256 of the series bytes abab") || t.contains(&format!("SHA-256 of the series bytes {}", "ab12".repeat(16))), "{t}");
        assert!(t.contains("Models that ran: 4"));
        assert_eq!(t.matches("  words:").count(), 4);
        assert!(t.contains("sensitivity line only"));
        assert!(t.contains("note: a note"));
        // Without the reader: no URA models, GLD stays.
        let ura2 = || -> Result<UraOutcomes, String> { panic!("not called") };
        let inp2 = Inputs { s: col(5, 36), c: col(6, 36), d: col(7, 36), gld: col(8, 36), r: Ok(col(9, 36)), g: Err("x".into()), ura: &ura2 };
        let res2 = run_core(inp2, &no_u_reader).unwrap();
        assert_eq!(res2.models.len(), 1);
        assert!(report_text(&res2).contains("U not available"));
    }
}
