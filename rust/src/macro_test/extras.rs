//! Descriptions of rules 6.3, 6.4, 6.7 and the sanity check of rule 7.2.

use super::data::{parse_date, Week};
use super::design::{test_rows, Rows, TRAIN};
use super::stats::{expanding_preds, ols};
use serde::Serialize;

/// Rule 7.2: the correlation must be below this.
pub const SANITY_MAX_CORR: f64 = -0.5;

/// Stress windows of rule 6.3 that touch 2019 or later. Source: `analysis/macro/2026-11-21-stress-windows.md`
/// (origin/main, last changed in commit 346092a), main table "Windows from the rule": Start and End columns.
/// The row 2018-Q4 ends 2019-01-09 and touches the sample (2019-01-04). The 2026-11 window is open: its
/// end is the last VIX date of the store (2026-11-19). Rows before 2019 are left out.
pub const STRESS_WINDOWS: [(&str, &str, &str); 11] = [
    ("2018-Q4", "2018-12-03", "2019-01-09"),
    ("2020-03", "2020-02-21", "2021-03-16"),
    ("2021-12", "2021-11-24", "2021-12-08"),
    ("2022-Q1", "2022-01-14", "2022-03-28"),
    ("2022-Q2/Q3", "2022-04-04", "2022-08-10"),
    ("2022-H2", "2022-08-18", "2023-01-12"),
    ("2023-03", "2023-03-08", "2023-03-28"),
    ("2024-08", "2024-08-01", "2024-08-13"),
    ("2025-04", "2025-03-27", "2025-05-12"),
    ("2026-03", "2026-02-27", "2026-04-09"),
    ("2026-11", "2026-11-04", "2026-11-19"),
];

pub fn corr(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma).powi(2);
        sbb += (y - mb).powi(2);
    }
    if saa > 0.0 && sbb > 0.0 {
        sab / (saa * sbb).sqrt()
    } else {
        f64::NAN
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Sanity {
    pub n: usize,
    pub corr_s2_rates_dzero10: f64,
    pub passed: bool,
}

/// Rule 7.2 on the kept weeks that have `s2_rates`. No value or fewer than 3 weeks fails.
pub fn sanity(weeks: &[Week]) -> Sanity {
    let ks: Vec<(f64, f64)> = weeks.iter().filter(|w| w.test_ok()).filter_map(|w| Some((w.s2_rates?, w.dz?))).collect();
    let n = ks.len();
    let c = if n >= 3 { corr(&ks.iter().map(|k| k.0).collect::<Vec<_>>(), &ks.iter().map(|k| k.1).collect::<Vec<_>>()) } else { f64::NAN };
    Sanity { n, corr_s2_rates_dzero10: c, passed: c < SANITY_MAX_CORR }
}

#[derive(Debug, Clone, Serialize)]
pub struct StressRow {
    pub model: String,
    pub r2_stress: f64,
    pub r2_other: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stress {
    pub n_stress: usize,
    pub n_other: usize,
    pub rows: Vec<StressRow>,
}

fn in_window(friday: i64) -> bool {
    STRESS_WINDOWS.iter().any(|(_, a, b)| friday >= parse_date(a).unwrap_or(i64::MAX) && friday <= parse_date(b).unwrap_or(i64::MIN))
}

fn r2_subset(y: &[f64], resid: &[f64], idx: &[usize]) -> f64 {
    let m = idx.iter().map(|&i| y[i]).sum::<f64>() / idx.len() as f64;
    let sst: f64 = idx.iter().map(|&i| (y[i] - m).powi(2)).sum();
    let sse: f64 = idx.iter().map(|&i| resid[i].powi(2)).sum();
    1.0 - sse / sst
}

/// Rule 6.3 (description). M0 and M1 are fitted once on all kept weeks; the R2 of each subset is
/// 1 - SSE/SST of the subset (SST about the subset mean), from the residuals of that one fit.
pub fn stress(rows: &Rows) -> Result<Stress, String> {
    let (s, o): (Vec<usize>, Vec<usize>) = (0..rows.y.len()).partition(|&i| in_window(rows.fridays[i]));
    if s.len() < 3 || o.len() < 3 {
        return Err(format!("stress split needs 3 weeks on each side (stress {}, other {})", s.len(), o.len()));
    }
    let x1: Vec<Vec<f64>> = rows.x0.iter().zip(&rows.block).map(|(a, b)| a.iter().chain(b).copied().collect()).collect();
    let mut out = Vec::new();
    for (name, x) in [("M0", &rows.x0), ("M1", &x1)] {
        let f = ols(x, &rows.y).ok_or("stress fit failed")?;
        out.push(StressRow { model: name.into(), r2_stress: r2_subset(&rows.y, &f.resid, &s), r2_other: r2_subset(&rows.y, &f.resid, &o) });
    }
    Ok(Stress { n_stress: s.len(), n_other: o.len(), rows: out })
}

pub const PART_NAMES: [&str; 8] = ["facts", "time", "rates", "premium", "growth", "residual", "nomodel", "change"];

#[derive(Debug, Clone, Serialize)]
pub struct VarSplit {
    pub n: usize,
    pub var_change: f64,
    /// var(part) / var(change), for the 7 parts in file order.
    pub var_share: Vec<(String, f64)>,
    /// 2 cov(i, j) / var(change), for each pair i < j (21 pairs).
    pub cov_share: Vec<(String, f64)>,
    pub sum_var_share: f64,
    pub sum_cov_share: f64,
    /// 1 - sum of the shares above: the part of var(change) that the 7 parts do not hold.
    pub other_share: f64,
}

fn cov(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum::<f64>() / (n - 1.0)
}

/// Rule 6.4 (description): the variance of s2_change over the kept weeks that have all 8 columns.
/// Identity: var(sum of the parts) = sum var + 2 sum cov. The "other" share is what is left of var(change).
pub fn var_split(weeks: &[Week]) -> Result<VarSplit, String> {
    let ks: Vec<[f64; 8]> = weeks
        .iter()
        .filter(|w| w.test_ok())
        .filter_map(|w| {
            let mut v = [0.0; 8];
            for (i, p) in w.s2_parts.iter().enumerate() {
                v[i] = (*p)?;
            }
            Some(v)
        })
        .collect();
    if ks.len() < 3 {
        return Err(format!("only {} weeks with all S2 parts", ks.len()));
    }
    let col = |i: usize| -> Vec<f64> { ks.iter().map(|r| r[i]).collect() };
    let cols: Vec<Vec<f64>> = (0..8).map(col).collect();
    let vc = cov(&cols[7], &cols[7]);
    let var_share: Vec<(String, f64)> = (0..7).map(|i| (PART_NAMES[i].to_string(), cov(&cols[i], &cols[i]) / vc)).collect();
    let mut cov_share = Vec::new();
    for i in 0..7 {
        for j in i + 1..7 {
            cov_share.push((format!("{}-{}", PART_NAMES[i], PART_NAMES[j]), 2.0 * cov(&cols[i], &cols[j]) / vc));
        }
    }
    let (sv, sc): (f64, f64) = (var_share.iter().map(|p| p.1).sum(), cov_share.iter().map(|p| p.1).sum());
    Ok(VarSplit { n: ks.len(), var_change: vc, var_share, cov_share, sum_var_share: sv, sum_cov_share: sc, other_share: 1.0 - sv - sc })
}

#[derive(Debug, Clone, Serialize)]
pub struct SpyCheck {
    pub n: usize,
    pub corr_ret_spy: f64,
    pub m0_r2_in_sample_ret_w: f64,
    pub m0_r2_in_sample_spy: f64,
    pub m0_r2_oos_vs_zero_ret_w: Option<f64>,
    pub m0_r2_oos_vs_zero_spy: Option<f64>,
}

fn oos_r2(x: &[Vec<f64>], y: &[f64]) -> Option<f64> {
    let p = expanding_preds(x, y, TRAIN)?;
    let (mut a, mut b) = (0.0, 0.0);
    for (t, pr) in p.iter().enumerate() {
        a += (y[TRAIN + t] - pr).powi(2);
        b += y[TRAIN + t].powi(2);
    }
    Some(1.0 - a / b)
}

/// Rule 6.7 (description): the SPY weekly return beside ret_w, on the kept weeks that have SPY.
pub fn spy_check(weeks: &[Week]) -> Result<SpyCheck, String> {
    let sel: Vec<Week> = weeks.iter().filter(|w| w.test_ok() && w.spy_ret.is_some()).cloned().collect();
    if sel.len() < 10 {
        return Err(format!("only {} kept weeks with a SPY return", sel.len()));
    }
    let a = test_rows(&sel);
    let spy: Vec<f64> = sel.iter().map(|w| w.spy_ret.unwrap_or(f64::NAN)).collect();
    let x_spy: Vec<Vec<f64>> = a.x0.iter().zip(&spy).map(|(r, s)| vec![1.0, *s, s * s, r[3]]).collect();
    let r2 = |x: &[Vec<f64>]| ols(x, &a.y).map(|f| f.r2).ok_or("SPY fit failed");
    let ret: Vec<f64> = a.x0.iter().map(|r| r[1]).collect();
    Ok(SpyCheck {
        n: sel.len(),
        corr_ret_spy: corr(&ret, &spy),
        m0_r2_in_sample_ret_w: r2(&a.x0)?,
        m0_r2_in_sample_spy: r2(&x_spy)?,
        m0_r2_oos_vs_zero_ret_w: if sel.len() > TRAIN + 1 { oos_r2(&a.x0, &a.y) } else { None },
        m0_r2_oos_vs_zero_spy: if sel.len() > TRAIN + 1 { oos_r2(&x_spy, &a.y) } else { None },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wk(friday: &str, dz: f64, rates: Option<f64>, parts: [Option<f64>; 8]) -> Week {
        Week {
            friday: parse_date(friday).unwrap(),
            in_sample: true,
            switch: false,
            excluded: false,
            y: Some(0.0),
            ret: Some(0.0),
            dz: Some(dz),
            dvix: Some(0.0),
            dbaa: Some(0.0),
            dnfci: Some(0.0),
            ddfii: None,
            dbei: None,
            s2: None,
            s2_rates: rates,
            s2_parts: parts,
            spy_ret: None,
        }
    }

    /// dz = 1, 2, 3 and rates = -2, -4, -6: exactly linear with a negative slope, corr = -1 (< -0.5, passes).
    /// rates = 2, 4, 6 gives +1 (fails). rates = 1, 1, 1 has no variance (NaN, fails). No rates at all: n = 0 (fails).
    #[test]
    fn sanity_fixed_answers() {
        let mk = |r: [Option<f64>; 3]| -> Vec<Week> { (0..3).map(|i| wk(["2019-01-04", "2019-01-11", "2019-01-18"][i], (i + 1) as f64, r[i], [None; 8])).collect() };
        let s = sanity(&mk([Some(-2.0), Some(-4.0), Some(-6.0)]));
        assert!(s.passed && s.n == 3 && (s.corr_s2_rates_dzero10 + 1.0).abs() < 1e-12);
        assert!(!sanity(&mk([Some(2.0), Some(4.0), Some(6.0)])).passed);
        assert!(!sanity(&mk([Some(1.0), Some(1.0), Some(1.0)])).passed);
        let none = sanity(&mk([None, None, None]));
        assert!(!none.passed && none.n == 0);
    }

    /// Stress membership: 2020-03-20 is inside 2020-03 (2020-02-21 to 2021-03-16); 2019-06-14 is outside every window;
    /// 2019-01-04 is inside 2018-Q4 (ends 2019-01-09); 2026-11-20 is outside (the open window ends 2026-11-19).
    /// Hand R2: y = 1,2,3,4 on x = 1,2,3,4 is a perfect fit, so a subset R2 is 1 only if its SST > 0.
    #[test]
    fn stress_membership_and_windows() {
        for (d, want) in [("2020-03-20", true), ("2019-06-14", false), ("2019-01-04", true), ("2026-11-20", false), ("2021-12-10", false), ("2021-12-08", true)] {
            assert_eq!(in_window(parse_date(d).unwrap()), want, "{d}");
        }
        for (_, a, b) in STRESS_WINDOWS {
            assert!(parse_date(a).unwrap() < parse_date(b).unwrap());
        }
    }

    #[test]
    fn subset_r2_hand() {
        // y = 1,3,2 ; residuals -0.5, 1, -0.5 (the line 1.5 + 0.5 x). Subset {0,1}: mean 2, SST 2, SSE 1.25: R2 = 0.375.
        let r = r2_subset(&[1.0, 3.0, 2.0], &[-0.5, 1.0, -0.5], &[0, 1]);
        assert!((r - 0.375).abs() < 1e-12);
    }

    /// Three weeks; parts (facts, time, rates, premium, growth, residual, nomodel, change):
    /// w1: 1,0,1,0,0,0,0,2  w2: 2,0,2,0,0,0,0,4  w3: 3,0,3,0,0,0,0,6. Change = facts + rates exactly.
    /// Variances (n-1): facts 1, rates 1, change 4. Cov(facts, rates) = 1. Shares: facts 0.25, rates 0.25;
    /// covariance facts-rates 2*1/4 = 0.5; others 0. Sum = 1.0, other = 0.
    #[test]
    fn variance_split_fixed() {
        let p = |a: f64| [Some(a), Some(0.0), Some(a), Some(0.0), Some(0.0), Some(0.0), Some(0.0), Some(2.0 * a)];
        let ws = vec![wk("2019-01-04", 0.0, None, p(1.0)), wk("2019-01-11", 0.0, None, p(2.0)), wk("2019-01-18", 0.0, None, p(3.0))];
        let v = var_split(&ws).unwrap();
        let g = |n: &str| v.var_share.iter().find(|p| p.0 == n).unwrap().1;
        assert!((v.var_change - 4.0).abs() < 1e-12 && (g("facts") - 0.25).abs() < 1e-12 && (g("rates") - 0.25).abs() < 1e-12);
        assert!((v.cov_share.iter().find(|p| p.0 == "facts-rates").unwrap().1 - 0.5).abs() < 1e-12);
        assert!((v.sum_var_share + v.sum_cov_share - 1.0).abs() < 1e-12 && v.other_share.abs() < 1e-12);
    }

    /// corr of (1,2,3,4) with (2,4,6,9) by hand: means 2.5 and 5.25; deviations (-1.5,-.5,.5,1.5), (-3.25,-1.25,.75,3.75);
    /// sab = 4.875+0.625+0.375+5.625 = 11.5; saa = 5; sbb = 10.5625+1.5625+0.5625+14.0625 = 26.75; r = 11.5/sqrt(133.75) = 0.99438.
    #[test]
    fn corr_hand() {
        let r = corr(&[1.0, 2.0, 3.0, 4.0], &[2.0, 4.0, 6.0, 9.0]);
        assert!((r - 11.5 / 133.75_f64.sqrt()).abs() < 1e-12);
    }
}
