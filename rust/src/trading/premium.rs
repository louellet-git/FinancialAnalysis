//! `fa trade premium`: the implied-variance premium in isolation (research stage 1a).
//!
//! 30-day premium on day t: (VIX_t/100)^2 minus the realised variance of SPY over the next 21
//! trading days (sum of squared close-to-close log returns, times 252/21, no demeaning). The VIX
//! close of day t and the returns of the days t+1 to t+21 are the only inputs, so no signal
//! uses a later fact than its date. 1-day premium: (VIX1D/100)^2/252 minus the squared return of the same
//! day (close to close, or open to close). The VIX1D value is the open of that day.
//! These are index levels and real SPY prices, not option prices; the premium is a measure of
//! implied against realised variance, not a P&L.

use super::calendar::{iso, parse_iso, Date};
use super::data::{read_series, Store};
use super::stats;
use std::path::Path;

pub const USAGE: &str = "\
fa trade premium - implied against realised variance of SPY (VIX 21 days, VIX1D 1 day)

USAGE:
  fa trade premium [--horizon N] [--lag N]

  --horizon N   trading days of the realised variance after the VIX close (default 21)
  --lag N       Newey-West lags for the overlapping 30-day premium (default 20)

Data: _VIX_1d.csv, SPY_1d.csv, _VIX1D_1d.csv in the trading cache. No network.";

/// The stress windows of `analysis/macro/2026-11-21-stress-windows.md` (start, end, name).
pub const WINDOWS: [(&str, &str, &str); 13] = [
    ("2016-01-05", "2016-02-22", "2016-02"),
    ("2018-02-02", "2018-02-14", "2018-02"),
    ("2018-12-03", "2019-01-09", "2018-Q4"),
    ("2020-02-21", "2021-03-16", "2020-03"),
    ("2021-11-24", "2021-12-08", "2021-12"),
    ("2022-01-14", "2022-03-28", "2022-Q1"),
    ("2022-04-04", "2022-08-10", "2022-Q2/Q3"),
    ("2022-08-18", "2023-01-12", "2022-H2"),
    ("2023-03-08", "2023-03-28", "2023-03"),
    ("2024-08-01", "2024-08-13", "2024-08"),
    ("2025-03-27", "2025-05-12", "2025-04"),
    ("2026-02-27", "2026-04-09", "2026-03"),
    ("2026-11-04", "2026-11-19", "2026-11 open"),
];

/// Premium of each day t that has `horizon` later returns. `dates`, `spy` (close) and `vix`
/// (close, points) are aligned by index. Returns (index of t, premium in annual variance).
pub fn premium_30d(spy: &[f64], vix: &[f64], horizon: usize) -> Vec<(usize, f64)> {
    let r: Vec<f64> = std::iter::once(f64::NAN).chain(spy.windows(2).map(|w| (w[1] / w[0]).ln())).collect();
    let mut out = Vec::new();
    for t in 0..spy.len().saturating_sub(horizon) {
        let rv: f64 = r[t + 1..=t + horizon].iter().map(|x| x * x).sum::<f64>() * 252.0 / horizon as f64;
        let iv = (vix[t] / 100.0).powi(2);
        if rv.is_finite() && iv.is_finite() {
            out.push((t, iv - rv));
        }
    }
    out
}

struct Row {
    name: String,
    n: usize,
    mean: f64,
    median: f64,
    pos: f64,
    t_nw: f64,
    t_block: f64,
}

/// One row of statistics for the days where `mask` is true. `block` is the step of the
/// non-overlapping sample (the horizon for the 30-day premium, 1 for the 1-day premium).
fn row(name: &str, x: &[f64], mask: &[bool], lag: usize, block: usize) -> Row {
    let (mean, t_nw, n) = stats::nw_mean_t(x, mask, lag);
    let mut v: Vec<f64> = x.iter().zip(mask).filter(|(_, &m)| m).map(|(a, _)| *a).collect();
    let pos = v.iter().filter(|&&a| a > 0.0).count() as f64 / v.len().max(1) as f64;
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if v.is_empty() { f64::NAN } else { v[v.len() / 2] };
    let blk: Vec<f64> = (0..x.len()).step_by(block.max(1)).filter(|&i| mask[i]).map(|i| x[i]).collect();
    Row { name: name.to_string(), n, mean, median, pos, t_nw, t_block: stats::t_mean(&blk) }
}

fn print_rows(unit: &str, rows: &[Row]) {
    println!("{:<34} {:>6} {:>10} {:>10} {:>7} {:>7} {:>7}", "subset", "n", format!("mean[{unit}]"), format!("med[{unit}]"), "pos%", "t_NW", "t_blk");
    for r in rows {
        println!("{:<34} {:>6} {:>10.5} {:>10.5} {:>7.1} {:>7.2} {:>7.2}", r.name, r.n, r.mean, r.median, 100.0 * r.pos, r.t_nw, r.t_block);
    }
}

/// True where the date is inside a stress window (inclusive).
fn in_window(d: Date) -> bool {
    WINDOWS.iter().any(|(s, e, _)| parse_iso(s).is_some_and(|a| d >= a) && parse_iso(e).is_some_and(|b| d <= b))
}

pub fn command(repo: &Path, args: &[String]) -> Result<i32, String> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let (mut horizon, mut lag) = (21usize, 20usize);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let v = |x: Option<&String>| x.and_then(|s| s.parse::<usize>().ok()).ok_or_else(|| format!("{a} needs a whole number"));
        match a.as_str() {
            "--horizon" => horizon = v(it.next())?,
            "--lag" => lag = v(it.next())?,
            o => return Err(format!("unknown option '{o}'")),
        }
    }
    if horizon == 0 {
        return Err("--horizon needs a whole number of at least 1".to_string());
    }
    let dir = Store::from_env(repo).dir().to_path_buf();
    let spy = read_series(&dir.join("SPY_1d.csv"))?;
    let vix = read_series(&dir.join("_VIX_1d.csv"))?;
    let vix1d = read_series(&dir.join("_VIX1D_1d.csv"))?;
    report(&spy, &vix, &vix1d, horizon, lag)?;
    Ok(0)
}

/// The two tables for the three series. An error when the series have no common day or no VIX1D day.
fn report(spy: &super::data::Series, vix: &super::data::Series, vix1d: &super::data::Series, horizon: usize, lag: usize) -> Result<(), String> {
    if horizon == 0 {
        return Err("horizon must be at least 1".to_string());
    }
    let at = |s: &super::data::Series, d: Date| s.date.binary_search(&d).ok();
    // Common dates of SPY and VIX.
    let days: Vec<usize> = (0..spy.len()).filter(|&i| at(vix, spy.date[i]).is_some()).collect();
    let dates: Vec<Date> = days.iter().map(|&i| spy.date[i]).collect();
    if dates.len() <= horizon {
        return Err(format!("SPY and VIX have {} common days, not more than the horizon {horizon}: no premium can be computed", dates.len()));
    }
    let spy_c: Vec<f64> = days.iter().map(|&i| spy.px.close[i]).collect();
    let vix_c: Vec<f64> = dates.iter().map(|&d| vix.px.close[at(vix, d).unwrap_or(0)]).collect();
    println!("SPY and VIX common days: {} ({} to {}). Model prices: none; this is implied against realised variance.", dates.len(), iso(dates[0]), iso(dates[dates.len() - 1]));

    let p = premium_30d(&spy_c, &vix_c, horizon);
    let pd: Vec<Date> = p.iter().map(|&(t, _)| dates[t]).collect();
    let x: Vec<f64> = p.iter().map(|&(_, v)| v).collect();
    // Clean outside: no window day among the signal day and the next `horizon` days.
    let win: Vec<bool> = dates.iter().map(|&d| in_window(d)).collect();
    let clean: Vec<bool> = p.iter().map(|&(t, _)| !win[t..=(t + horizon).min(dates.len() - 1)].iter().any(|&w| w)).collect();
    let (y2016, y2016e) = (parse_iso("2016-01-05").unwrap_or(0), parse_iso("2026-11-19").unwrap_or(0));
    let w16: Vec<bool> = pd.iter().map(|&d| d >= y2016 && d <= y2016e).collect();
    let inw: Vec<bool> = pd.iter().map(|&d| in_window(d)).collect();
    let all = vec![true; x.len()];
    let and = |a: &[bool], b: &[bool]| a.iter().zip(b).map(|(p, q)| *p && *q).collect::<Vec<bool>>();
    let not = |a: &[bool]| a.iter().map(|p| !p).collect::<Vec<bool>>();
    println!("\n== 30-day premium: VIX^2 - realised variance of the next {horizon} days (annual variance units; 0.01 = 10 vol points squared) ==");
    println!("convention: realised = sum of squared close-to-close log returns x 252/{horizon}, no demeaning; t_NW = Newey-West, {lag} lags; t_blk = every {horizon}th day, plain t");
    let mut rows = vec![
        row("all days 1990s-2026", &x, &all, lag, horizon),
        row("1993 to 2015 (no windows defined)", &x, &not(&w16), lag, horizon),
        row("2016-2026 all", &x, &w16, lag, horizon),
        row("2016-2026 INSIDE windows", &x, &and(&w16, &inw), lag, horizon),
        row("2016-2026 OUTSIDE windows (gate)", &x, &and(&w16, &not(&inw)), lag, horizon),
        row("clean outside: USES THE FUTURE", &x, &and(&w16, &clean), lag, horizon),
    ];
    for (s, e, name) in WINDOWS {
        let (a, b) = (parse_iso(s).unwrap_or(0), parse_iso(e).unwrap_or(0));
        let m: Vec<bool> = pd.iter().map(|&d| d >= a && d <= b).collect();
        if m.iter().any(|&k| k) {
            rows.push(row(&format!("window {name}"), &x, &m, lag, horizon));
        }
    }
    print_rows("var", &rows);
    // The same in volatility points: VIX minus the realised vol (annual, %).
    let xv: Vec<f64> = p.iter().map(|&(t, v)| vix_c[t] - 100.0 * ((vix_c[t] / 100.0).powi(2) - v).max(0.0).sqrt()).collect();
    let rv = row("vol points, 2016-2026 OUTSIDE", &xv, &and(&w16, &not(&inw)), lag, horizon);
    let ra = row("vol points, all days", &xv, &all, lag, horizon);
    println!("in vol points (VIX - realised vol): all days mean {:.2} median {:.2}; outside windows 2016-26 mean {:.2} median {:.2}", ra.mean, ra.median, rv.mean, rv.median);

    // 1-day premium with VIX1D (open of the day).
    println!("\n== 1-day premium: (VIX1D open/100)^2/252 - squared return of the same day (daily variance units) ==");
    println!("close-to-close: previous close to close of the day (overnight and session, like the horizon of VIX1D); open-to-close: session only (the session has about 80% of the day variance in the pricer, so this one understates the realised side)");
    let mut cc = Vec::new();
    let mut oc = Vec::new();
    let mut d1 = Vec::new();
    for j in 1..vix1d.len() {
        let d = vix1d.date[j];
        if let (Some(k), true) = (at(spy, d), vix1d.px.open[j].is_finite()) {
            if k == 0 || spy.date[k - 1] >= d {
                continue;
            }
            let iv = (vix1d.px.open[j] / 100.0).powi(2) / 252.0;
            cc.push(iv - (spy.px.close[k] / spy.px.close[k - 1]).ln().powi(2));
            oc.push(iv - (spy.px.close[k] / spy.px.open[k]).ln().powi(2));
            d1.push(d);
        }
    }
    if d1.is_empty() {
        return Err("no day where VIX1D and SPY overlap (check _VIX1D_1d.csv and SPY_1d.csv)".to_string());
    }
    let m_all = vec![true; cc.len()];
    let m_w: Vec<bool> = d1.iter().map(|&d| in_window(d)).collect();
    let mut r1 = Vec::new();
    for (lab, v) in [("close-to-close", &cc), ("open-to-close", &oc)] {
        r1.push(row(&format!("{lab} all days"), v, &m_all, 5, 1));
        r1.push(row(&format!("{lab} INSIDE windows"), v, &m_w, 5, 1));
        r1.push(row(&format!("{lab} OUTSIDE windows"), v, &not(&m_w), 5, 1));
    }
    println!("days: {} ({} to {}); t_NW with 5 lags, t_blk is the plain t", cc.len(), iso(d1[0]), iso(d1[d1.len() - 1]));
    print_rows("var", &r1);
    for (s, e, name) in WINDOWS {
        let (a, b) = (parse_iso(s).unwrap_or(0), parse_iso(e).unwrap_or(0));
        let m: Vec<bool> = d1.iter().map(|&d| d >= a && d <= b).collect();
        if m.iter().any(|&k| k) {
            let r = row(&format!("cc window {name}"), &cc, &m, 5, 1);
            println!("{:<34} {:>6} {:>10.6} {:>10.6} {:>7.1}", r.name, r.n, r.mean, r.median, 100.0 * r.pos);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premium_of_a_synthetic_series_has_a_fixed_answer() {
        // SPY log returns r1..r4 = +1%, -2%, +3%, -1% (all different); VIX 20, 25, 30, 10, 15; horizon 2.
        // Realised variance of day t = (r(t+1)^2 + r(t+2)^2) x 252/2; implied = (VIX/100)^2.
        // t=0: 0.04   - (0.0001+0.0004) x 126 = 0.04   - 0.063  = -0.0230
        // t=1: 0.0625 - (0.0004+0.0009) x 126 = 0.0625 - 0.1638 = -0.1013
        // t=2: 0.09   - (0.0009+0.0001) x 126 = 0.09   - 0.126  = -0.0360
        let mut spy = vec![100.0_f64];
        for r in [0.01_f64, -0.02, 0.03, -0.01] {
            let last = *spy.last().unwrap();
            spy.push(last * r.exp());
        }
        let vix = [20.0, 25.0, 30.0, 10.0, 15.0];
        let p = premium_30d(&spy, &vix, 2);
        let want = [-0.0230, -0.1013, -0.0360];
        assert_eq!(p.len(), 3);
        for (k, (t, v)) in p.iter().enumerate() {
            assert_eq!(*t, k);
            assert!((v - want[k]).abs() < 1e-12, "t {t}: {v}");
        }
        // The last `horizon` days have no full future window, so the signal never uses a missing day.
        assert_eq!(p.last().map(|x| x.0), Some(2));
        // The statistics of the three values: mean -0.0534333, no positive day.
        let x: Vec<f64> = p.iter().map(|x| x.1).collect();
        let r = row("t", &x, &[true; 3], 1, 2);
        assert!((r.mean - (-0.160_3 / 3.0)).abs() < 1e-12 && r.pos == 0.0 && r.n == 3);
        assert!((r.median - (-0.036)).abs() < 1e-12, "median is the middle of the three sorted values");
    }

    fn series(days: &[&str], close: &[f64]) -> super::super::data::Series {
        let mut s = super::super::data::Series::default();
        for (d, &c) in days.iter().zip(close) {
            s.date.push(parse_iso(d).unwrap());
            s.minute.push(0);
            s.utc.push(0);
            s.px.open.push(c);
            s.px.high.push(c);
            s.px.low.push(c);
            s.px.close.push(c);
            s.px.volume.push(0.0);
        }
        s
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        let a = series(&["2024-01-02", "2024-01-03", "2024-01-04"], &[100.0, 101.0, 102.0]);
        let b = series(&["2024-02-02", "2024-02-03", "2024-02-04"], &[20.0, 20.0, 20.0]);
        // Horizon 0 (the command and the report).
        let e = report(&a, &a, &a, 0, 1).unwrap_err();
        assert!(e.contains("horizon"), "{e}");
        let tmp = std::env::temp_dir().join("fa-no-such-cache-premium");
        let e = command(&tmp, &["--horizon".into(), "0".into()]).unwrap_err();
        assert!(e.contains("--horizon"), "{e}");
        // No common day of SPY and VIX.
        let e = report(&a, &b, &b, 1, 1).unwrap_err();
        assert!(e.contains("common days"), "{e}");
        // Common days, but no VIX1D day that overlaps with SPY.
        let e = report(&a, &a, &b, 1, 1).unwrap_err();
        assert!(e.contains("VIX1D"), "{e}");
    }

    #[test]
    fn window_membership_uses_inclusive_dates() {
        assert!(in_window(parse_iso("2025-04-08").unwrap()));
        assert!(in_window(parse_iso("2025-03-27").unwrap()) && in_window(parse_iso("2025-05-12").unwrap()));
        assert!(!in_window(parse_iso("2025-05-13").unwrap()));
    }
}
