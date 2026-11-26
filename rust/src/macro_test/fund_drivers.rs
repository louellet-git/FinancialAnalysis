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
use crate::market::fund_attribution::{attribute, close_on, parse_map, positions, Group};
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
    let (mut y, mut m) = FIRST_MONTH;
    // base: the last day of the month before the first month
    let mut out = vec![mapcheck::dates::format(mapcheck::dates::days_from_civil(y, m, 1) - 1)];
    for _ in 0..N_MONTHS {
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
pub struct UraOutcomes {
    pub m: Vec<f64>,
    pub r: Vec<f64>,
    pub f: Vec<f64>,
    pub f_ex_daewoo: Vec<f64>,
    pub notes: Vec<String>,
}

fn find<'a>(reports: &'a [Report], date: &str) -> Option<&'a Report> {
    reports.iter().find(|r| r.report_date == date)
}

fn ura_outcomes(cache: &Path, dates: &[String]) -> Result<UraOutcomes, String> {
    let map = parse_map(MAP_CSV);
    let (series_id, class_id) = std::fs::read_to_string(cache.join("_sec").join("company_tickers_mf.json"))
        .ok()
        .and_then(|t| series_of(&t, "URA").ok())
        .map_or((None, String::new()), |s| (Some(s.series_id), s.class_id));
    let (loaded, _) = select_reports(read_cached_reports(cache, "URA", series_id.as_deref()));
    let reports: Vec<Report> = loaded.into_iter().map(|l| l.report).collect();
    let mut prices: HashMap<String, Option<Vec<crate::market::prices::PricePoint>>> = HashMap::new();
    for t in URA_M.iter().chain(URA_R.iter()) {
        prices.insert((*t).to_string(), price_cache::history(cache, t));
    }
    let mut out = UraOutcomes { m: vec![], r: vec![], f: vec![], f_ex_daewoo: vec![], notes: vec![] };
    let mut f_open = true;
    for qi in 0..N_MONTHS / 3 {
        let q_dates = &dates[3 * qi..=3 * qi + 3];
        let r0 = find(&reports, &q_dates[0]).ok_or_else(|| format!("URA: no N-PORT report of {} (the weights of the quarter)", q_dates[0]))?;
        let net0 = r0.net_assets.filter(|n| *n > 0.0).ok_or_else(|| format!("URA {}: no net assets", r0.report_date))?;
        let (pos, _) = positions(r0);
        for (group, list, sink) in [("URA-M", URA_M.as_slice(), &mut out.m), ("URA-R", URA_R.as_slice(), &mut out.r)] {
            let mut members: Vec<(f64, [f64; 4])> = Vec::new();
            for p in &pos {
                let Some(t) = map.get(&p.isin).or_else(|| map.get(&p.cusip)) else { continue };
                if !list.contains(&t.as_str()) {
                    continue;
                }
                let rows = prices.get(t).and_then(|o| o.as_ref());
                let px: Option<Vec<f64>> = rows.and_then(|rows| q_dates.iter().map(|d| close_on(rows, d)).collect());
                match px {
                    Some(px) if px.iter().all(|v| *v > 0.0) => members.push((p.val_usd / net0, [px[0], px[1], px[2], px[3]])),
                    _ => out.notes.push(format!("{group} {}: member {t} has no close at a date of the quarter and is left out", q_dates[3])),
                }
            }
            if members.is_empty() {
                return Err(format!("{group}: no member with prices in the quarter to {}", q_dates[3]));
            }
            sink.extend(group_month_returns(&members).iter().map(|r| (1.0 + r).ln()));
        }
        // URA-F needs the report at the end of the quarter.
        match (f_open, find(&reports, &q_dates[3])) {
            (true, Some(r1)) => {
                let q = attribute(r0, r1, &class_id, &map)?;
                for (excl, sink) in [(false, &mut out.f), (true, &mut out.f_ex_daewoo)] {
                    let rows: Vec<_> = q.rows.iter().filter(|r| r.group == Group::Foreign && !(excl && r.isin == DAEWOO_ISIN)).collect();
                    let w: f64 = rows.iter().map(|r| r.w0).sum();
                    let c: f64 = rows.iter().map(|r| r.contrib).sum();
                    if w <= 0.0 {
                        return Err(format!("URA-F: no foreign weight in the quarter to {}", q_dates[3]));
                    }
                    sink.push((1.0 + c / w).ln());
                }
            }
            _ => {
                f_open = false;
                out.notes.push(format!("URA-F: no N-PORT report of {}: the quarter and the later ones are not used", q_dates[3]));
            }
        }
    }
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

/// The whole study. `u_reader` is called once.
pub fn run_study(cache: &Path, u_reader: &dyn Fn() -> Result<URead, String>) -> Result<Results, String> {
    let dates = month_ends();
    let labels: Vec<String> = dates[1..].to_vec();
    let q_labels: Vec<String> = (0..N_MONTHS / 3).map(|i| dates[3 * i + 3].clone()).collect();
    let mut res = Results { design: "analysis/macro/2026-11-25-fund-drivers-design.md (R1, R2, R3)", u_read_date: None, u_sha256: None, models: vec![], sensitivity_f_without_daewoo: None, not_run: vec![], notes: vec![], gld_identity: None };
    let s = price_log_returns(cache, "SPY", &dates)?;
    let c = price_log_returns(cache, "SMH", &dates)?;
    let d = price_log_returns(cache, "UUP", &dates)?;
    // U.
    let u = match u_reader() {
        Ok(u) => {
            res.u_read_date = Some(u.read_date.clone());
            res.u_sha256 = Some(u.sha256.clone());
            Some(log_changes_of_months(&u.months, &dates, "U")?)
        }
        Err(e) => {
            res.not_run.push(format!("URA-M, URA-R, URA-F: {e}"));
            None
        }
    };
    if let Some(u) = &u {
        let ura = ura_outcomes(cache, &dates)?;
        res.notes.extend(ura.notes.clone());
        for (nm, y) in [("URA-M", &ura.m), ("URA-R", &ura.r)] {
            res.models.push(analyse(nm, y, &["U", "S", "C"], &[u, &s, &c], &labels, false)?);
        }
        let nf = ura.f.len();
        res.notes.push(format!("URA-F uses {nf} of 12 quarters"));
        let uq: Vec<f64> = (0..nf).map(|i| u[3 * i..3 * i + 3].iter().sum()).collect();
        let ql = &q_labels[..nf];
        res.models.push(analyse("URA-F", &ura.f, &["U"], &[&uq], ql, true)?);
        res.sensitivity_f_without_daewoo = Some(analyse("URA-F without Daewoo E&C (R1, sensitivity only)", &ura.f_ex_daewoo, &["U"], &[&uq], ql, true)?);
    }
    // GLD.
    let gld = price_log_returns(cache, "GLD", &dates)?;
    match real_yield_changes(cache, &dates) {
        Ok(r) => res.models.push(analyse("GLD", &gld, &["R", "D"], &[&r, &d], &labels, false)?),
        Err(e) => res.not_run.push(format!("GLD: R not available ({e})")),
    }
    // Identity check against the World Bank gold price (not a driver).
    let g = commodity::newest(&commodity::series_dir(cache))
        .ok_or_else(|| "no World Bank file in the cache (fa commodity get gold)".to_string())
        .and_then(|p| std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display())))
        .and_then(|b| commodity::read_series(&b, &commodity::SERIES[0]))
        .and_then(|p| log_changes_of_months(&p.months, &dates, "G"));
    match g {
        Ok(g) => res.gld_identity = identity(&gld, &g),
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
    let _ = writeln!(s, "Models in the table: {}. With four models at 10%, one false 'consistent' is possible; the out-of-sample condition lowers that chance.", r.models.len());
    let mut all: Vec<&Model> = r.models.iter().collect();
    all.extend(r.sensitivity_f_without_daewoo.iter());
    for m in all {
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
        let _ = writeln!(s, "  words: {}", m.words);
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
}
