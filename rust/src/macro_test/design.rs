//! The pre-registered test: expanding-window OOS fit, Clark-West, placebo, verdict, descriptions.

use super::data::{fmt_date, Week};
use super::stats::{clark_west, expanding_preds, nw_se, ols, NW_LAG};
use serde::Serialize;

/// First fit uses this many kept weeks (rule 3).
pub const TRAIN: usize = 104;
pub const CW_P_MAX: f64 = 0.05;
pub const GAIN_MIN: f64 = 0.05;
pub const PLACEBO_MAX_YES: usize = 4;
pub const SHIFT_MIN: i64 = 13;
pub const SHIFT_MAX: i64 = 52;

/// Rows of one regression: y, the M0 columns (intercept first) and the block that M1 adds.
pub struct Rows {
    pub fridays: Vec<i64>,
    pub y: Vec<f64>,
    pub x0: Vec<Vec<f64>>,
    pub block: Vec<Vec<f64>>,
    pub x0_names: Vec<String>,
    pub block_names: Vec<String>,
}

fn m0_row(w: &Week) -> Vec<f64> {
    let r = w.ret.unwrap_or(f64::NAN);
    vec![1.0, r, r * r, w.dz.unwrap_or(f64::NAN)]
}

fn m0_names() -> Vec<String> {
    ["intercept", "ret_w", "ret_w^2", "dzero10"].map(String::from).to_vec()
}

/// Main test rows: M1 block is dVIX (record R2: no NFCI). `y` is the S1 change.
pub fn test_rows(weeks: &[Week]) -> Rows {
    let ks: Vec<&Week> = weeks.iter().filter(|w| w.test_ok()).collect();
    Rows {
        fridays: ks.iter().map(|w| w.friday).collect(),
        y: ks.iter().map(|w| w.y.unwrap_or(f64::NAN)).collect(),
        x0: ks.iter().map(|w| m0_row(w)).collect(),
        block: ks.iter().map(|w| vec![w.dvix.unwrap_or(f64::NAN)]).collect(),
        x0_names: m0_names(),
        block_names: ["dVIX"].map(String::from).to_vec(),
    }
}

/// Descriptive block: M0 plus dDFII10 and dT10YIE, on the weeks that have both.
pub fn real_bei_rows(weeks: &[Week]) -> Rows {
    let ks: Vec<&Week> = weeks.iter().filter(|w| w.core_ok() && w.ddfii.is_some() && w.dbei.is_some()).collect();
    Rows {
        fridays: ks.iter().map(|w| w.friday).collect(),
        y: ks.iter().map(|w| w.y.unwrap_or(f64::NAN)).collect(),
        x0: ks.iter().map(|w| m0_row(w)).collect(),
        block: ks.iter().map(|w| vec![w.ddfii.unwrap_or(f64::NAN), w.dbei.unwrap_or(f64::NAN)]).collect(),
        x0_names: m0_names(),
        block_names: ["dDFII10", "dT10YIE"].map(String::from).to_vec(),
    }
}

/// S2 (plug part, points) on the test weeks that have S2.
pub fn s2_rows(weeks: &[Week]) -> Rows {
    let ks: Vec<&Week> = weeks.iter().filter(|w| w.test_ok() && w.s2.is_some()).collect();
    let mut r = test_rows(&weeks.iter().filter(|w| w.test_ok() && w.s2.is_some()).cloned().collect::<Vec<_>>());
    r.y = ks.iter().map(|w| w.s2.unwrap_or(f64::NAN)).collect();
    r
}

#[derive(Debug, Clone, Serialize)]
pub struct Eval {
    pub sse0: f64,
    pub sse1: f64,
    pub gain: f64,
    pub cw_t: f64,
    pub cw_p: f64,
    /// The "yes" rule: p < 0.05 and gain >= 0.05.
    pub yes: bool,
}

/// `y`, `p0`, `p1` are for the OOS weeks only.
pub fn evaluate(y: &[f64], p0: &[f64], p1: &[f64]) -> Result<Eval, String> {
    let sse = |p: &[f64]| y.iter().zip(p).map(|(a, b)| (a - b).powi(2)).sum::<f64>();
    let (sse0, sse1) = (sse(p0), sse(p1));
    if sse0 <= 0.0 || !sse0.is_finite() {
        return Err(format!("M0 has an out-of-sample SSE of {sse0}: the gain is not defined; the test stops, no verdict"));
    }
    let gain = 1.0 - sse1 / sse0;
    if !gain.is_finite() {
        return Err(format!("the gain of M1 over M0 is not finite (SSE M0 {sse0}, SSE M1 {sse1}); the test stops, no verdict"));
    }
    let (cw_t, cw_p) = clark_west(y, p0, p1, NW_LAG);
    Ok(Eval { sse0, sse1, gain, cw_t, cw_p, yes: cw_p < CW_P_MAX && gain >= GAIN_MIN })
}

fn join(x0: &[Vec<f64>], block: &[Vec<f64>]) -> Vec<Vec<f64>> {
    x0.iter().zip(block).map(|(a, b)| a.iter().chain(b).copied().collect()).collect()
}

/// Circular shift of the block by `s` kept weeks: row t gets the block of row (t + s) mod n.
fn shifted(block: &[Vec<f64>], s: i64) -> Vec<Vec<f64>> {
    let n = block.len() as i64;
    (0..n).map(|t| block[(t + s).rem_euclid(n) as usize].clone()).collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct Placebo {
    pub runs: usize,
    pub yes: usize,
    pub yes_shifts: Vec<i64>,
    pub gain_min: f64,
    pub gain_median: f64,
    pub gain_max: f64,
}

/// Rule 5.6: every shift 13..=52 weeks in both directions (80 runs); y, ret, ret^2, dzero10 stay in place.
pub fn placebo(rows: &Rows, start: usize, p0: &[f64]) -> Result<Placebo, String> {
    let yo = &rows.y[start..];
    let mut gains = Vec::new();
    let mut yes_shifts = Vec::new();
    for s in (SHIFT_MIN..=SHIFT_MAX).flat_map(|s| [s, -s]) {
        let x1 = join(&rows.x0, &shifted(&rows.block, s));
        let p1 = expanding_preds(&x1, &rows.y, start).ok_or("placebo fit failed (singular)")?;
        let e = evaluate(yo, p0, &p1)?;
        gains.push(e.gain);
        if e.yes {
            yes_shifts.push(s);
        }
    }
    gains.sort_by(f64::total_cmp);
    Ok(Placebo { runs: gains.len(), yes: yes_shifts.len(), yes_shifts, gain_min: gains[0], gain_median: gains[gains.len() / 2], gain_max: gains[gains.len() - 1] })
}

pub fn verdict(real_yes: bool, placebo_yes: usize) -> &'static str {
    if real_yes && placebo_yes <= PLACEBO_MAX_YES {
        "yes"
    } else {
        "no share shown"
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Coef {
    pub name: String,
    pub beta: f64,
    /// Newey-West t (lag 8). A description, not a test.
    pub t_nw: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InSample {
    pub r2: f64,
    pub coefs: Vec<Coef>,
}

fn insample(x: &[Vec<f64>], y: &[f64], names: &[String]) -> Result<InSample, String> {
    let f = ols(x, y).ok_or("in-sample fit failed (singular or too few rows)")?;
    let se = nw_se(x, &f, NW_LAG);
    Ok(InSample { r2: f.r2, coefs: names.iter().enumerate().map(|(i, n)| Coef { name: n.clone(), beta: f.beta[i], t_nw: f.beta[i] / se[i] }).collect() })
}

fn top5_share(e: &[f64]) -> Result<f64, String> {
    let mut s: Vec<f64> = e.iter().map(|v| v * v).collect();
    let tot: f64 = s.iter().sum();
    if tot <= 0.0 || !tot.is_finite() {
        return Err(format!("the out-of-sample SSE is {tot}: the share of the largest weeks is not defined; the test stops, no verdict"));
    }
    s.sort_by(|a, b| b.total_cmp(a));
    Ok(s.iter().take(5).sum::<f64>() / tot)
}

#[derive(Debug, Clone, Serialize)]
pub struct Oos {
    pub n_kept: usize,
    pub n_train: usize,
    pub n_oos: usize,
    pub first_oos_friday: String,
    pub last_oos_friday: String,
    /// "Mechanical": M0 against the zero change.
    pub m0_r2_vs_zero: f64,
    pub m0_r2_vs_expanding_mean: f64,
    pub m1_r2_vs_zero: f64,
    pub m1_r2_vs_expanding_mean: f64,
    pub gain_m1_over_m0: f64,
    pub cw_t: f64,
    pub cw_p_one_sided: f64,
    pub top5_sse_share_m0: f64,
    pub top5_sse_share_m1: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Block {
    pub names: Vec<String>,
    pub m0_insample: InSample,
    pub m1_insample: InSample,
    pub oos: Oos,
}

/// M0 and M1 in sample (description) and out of sample; no placebo, no verdict.
pub fn run_block(rows: &Rows) -> Result<Block, String> {
    let n = rows.y.len();
    if n <= TRAIN + 1 {
        return Err(format!("only {n} kept weeks; the design needs more than {TRAIN}"));
    }
    let x1 = join(&rows.x0, &rows.block);
    let p0 = expanding_preds(&rows.x0, &rows.y, TRAIN).ok_or("M0 expanding fit failed (singular)")?;
    let p1 = expanding_preds(&x1, &rows.y, TRAIN).ok_or("M1 expanding fit failed (singular)")?;
    let yo = &rows.y[TRAIN..];
    let ev = evaluate(yo, &p0, &p1)?;
    let sse_zero: f64 = yo.iter().map(|v| v * v).sum();
    let mut csum: f64 = rows.y[..TRAIN].iter().sum();
    let mut sse_mean = 0.0;
    for (t, yy) in yo.iter().enumerate() {
        sse_mean += (yy - csum / (TRAIN + t) as f64).powi(2);
        csum += yy;
    }
    let e0: Vec<f64> = yo.iter().zip(&p0).map(|(a, b)| a - b).collect();
    let e1: Vec<f64> = yo.iter().zip(&p1).map(|(a, b)| a - b).collect();
    let mut names1 = rows.x0_names.clone();
    names1.extend(rows.block_names.iter().cloned());
    Ok(Block {
        names: rows.block_names.clone(),
        m0_insample: insample(&rows.x0, &rows.y, &rows.x0_names)?,
        m1_insample: insample(&x1, &rows.y, &names1)?,
        oos: Oos {
            n_kept: n,
            n_train: TRAIN,
            n_oos: n - TRAIN,
            first_oos_friday: fmt_date(rows.fridays[TRAIN]),
            last_oos_friday: fmt_date(rows.fridays[n - 1]),
            m0_r2_vs_zero: 1.0 - ev.sse0 / sse_zero,
            m0_r2_vs_expanding_mean: 1.0 - ev.sse0 / sse_mean,
            m1_r2_vs_zero: 1.0 - ev.sse1 / sse_zero,
            m1_r2_vs_expanding_mean: 1.0 - ev.sse1 / sse_mean,
            gain_m1_over_m0: ev.gain,
            cw_t: ev.cw_t,
            cw_p_one_sided: ev.cw_p,
            top5_sse_share_m0: top5_share(&e0)?,
            top5_sse_share_m1: top5_share(&e1)?,
        },
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct Primary {
    pub block: Block,
    pub real_yes_rule: bool,
    pub placebo: Placebo,
    pub verdict: String,
}

/// The inferential test of rule 5 with the placebo and the verdict.
pub fn run_primary(rows: &Rows) -> Result<Primary, String> {
    let block = run_block(rows)?;
    let p0 = expanding_preds(&rows.x0, &rows.y, TRAIN).ok_or("M0 expanding fit failed")?;
    let pl = placebo(rows, TRAIN, &p0)?;
    let real = block.oos.cw_p_one_sided < CW_P_MAX && block.oos.gain_m1_over_m0 >= GAIN_MIN;
    let v = verdict(real, pl.yes).to_string();
    Ok(Primary { block, real_yes_rule: real, placebo: pl, verdict: v })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dcf::nprandom::Pcg64;

    fn make(seed: u64, n: usize, link: f64, shuffle: bool) -> Rows {
        let mut g = Pcg64::new(seed);
        let mut z = |k: usize| g.standard_normal_vec(k);
        let ret = z(n);
        let dz = z(n);
        let mut dvix = z(n);
        let noise = z(n);
        let y: Vec<f64> = (0..n).map(|t| 0.3 * ret[t] + 0.2 * dz[t] + link * dvix[t] + 0.5 * noise[t]).collect();
        if shuffle {
            // Deterministic scramble of dVIX in blocks of 7 so that it is unrelated to y.
            dvix = (0..n).map(|t| dvix[(t * 37 + 11) % n]).collect();
        }
        Rows {
            fridays: (0..n as i64).map(|t| 17_900 + 7 * t).collect(),
            y,
            x0: (0..n).map(|t| vec![1.0, ret[t], ret[t] * ret[t], dz[t]]).collect(),
            block: (0..n).map(|t| vec![dvix[t]]).collect(),
            x0_names: m0_names(),
            block_names: vec!["dVIX".into()],
        }
    }

    /// The verdict rule: yes needs the real yes and at most 4 placebo yes.
    #[test]
    fn verdict_rule() {
        assert_eq!(verdict(true, 4), "yes");
        assert_eq!(verdict(true, 5), "no share shown");
        assert_eq!(verdict(false, 0), "no share shown");
    }

    #[test]
    fn shift_is_circular_and_has_80_runs() {
        let b: Vec<Vec<f64>> = (0..5).map(|i| vec![i as f64]).collect();
        // Shift 2: row 0 gets row 2, row 4 gets row (4+2) mod 5 = 1. Shift -1: row 0 gets row 4.
        assert_eq!(shifted(&b, 2)[0][0], 2.0);
        assert_eq!(shifted(&b, 2)[4][0], 1.0);
        assert_eq!(shifted(&b, -1)[0][0], 4.0);
        assert_eq!((SHIFT_MIN..=SHIFT_MAX).flat_map(|s| [s, -s]).count(), 80);
    }

    /// A planted link (y holds 0.8 dVIX, noise 0.5) gives yes: CW p is tiny, the gain is large,
    /// and a shifted block has no link, so the placebo count is far below 5.
    /// A series where dVIX is scrambled (no link) gives "no share shown". Seeds fixed.
    #[test]
    fn placebo_planted_yes_shuffled_no() {
        let planted = run_primary(&make(20_261_121, 320, 0.8, false)).unwrap();
        assert!(planted.block.oos.cw_p_one_sided < 1e-6, "{:?}", planted.block.oos);
        assert!(planted.block.oos.gain_m1_over_m0 > 0.3);
        assert_eq!(planted.placebo.runs, 80);
        assert!(planted.placebo.yes <= 4, "placebo yes {}", planted.placebo.yes);
        assert_eq!(planted.verdict, "yes");
        let shuffled = run_primary(&make(20_261_121, 320, 0.8, true)).unwrap();
        assert_eq!(shuffled.verdict, "no share shown");
        assert!(!shuffled.real_yes_rule, "{:?}", shuffled.block.oos);
    }

    /// If the placebo also says yes (a block that is the same under every shift: a constant-like
    /// regressor is not possible), the rule 5.7 still decides by count: tested by `verdict_rule`.
    /// Here the OOS count is exact: 320 rows, 104 train, so 216 OOS weeks; M0 R2 is positive when y holds ret.
    #[test]
    fn oos_counts_and_mechanical_r2() {
        let b = run_block(&make(7, 320, 0.0, false)).unwrap();
        assert_eq!((b.oos.n_kept, b.oos.n_train, b.oos.n_oos), (320, 104, 216));
        assert!(b.oos.m0_r2_vs_zero > 0.1);
        assert!(b.oos.top5_sse_share_m0 > 5.0 / 216.0);
    }

    /// If M0 fits the OOS weeks exactly (SSE 0), the gain is not defined: an error, no verdict.
    #[test]
    fn zero_sse_stops_without_verdict() {
        let y = [1.0, 2.0, 3.0];
        let e = evaluate(&y, &y, &[1.5, 2.5, 3.5]).unwrap_err();
        assert!(e.contains("no verdict"), "{e}");
        assert!(top5_share(&[0.0, 0.0]).unwrap_err().contains("no verdict"));
        // A normal case still works: SSE0 = 3 x 0.25 = 0.75, SSE1 = 0, gain 1.
        let ok = evaluate(&y, &[1.5, 2.5, 3.5], &y).unwrap();
        assert!((ok.gain - 1.0).abs() < 1e-12 && (ok.sse0 - 0.75).abs() < 1e-12);
    }

    /// A dVIX that is the same in every week is collinear with the intercept: a clear error, no verdict.
    #[test]
    fn constant_dvix_is_singular() {
        let mut r = make(5, 320, 0.0, false);
        r.block = vec![vec![0.7]; 320];
        let e = run_primary(&r).unwrap_err();
        assert!(e.contains("singular"), "{e}");
    }

    /// The placebo block has one column (dVIX): the shifted M1 has the 4 columns of M0 plus 1,
    /// and the 80 shifts all run.
    #[test]
    fn placebo_block_has_one_column() {
        let r = make(9, 320, 0.0, false);
        assert_eq!(r.block[0].len(), 1);
        assert_eq!(r.block_names, ["dVIX"]);
        let s = shifted(&r.block, 13);
        assert!(s.iter().all(|row| row.len() == 1));
        assert_eq!(join(&r.x0, &s)[0].len(), 5);
        // Row 0 gets the block of row 13.
        assert_eq!(s[0][0], r.block[13][0]);
        let p0 = expanding_preds(&r.x0, &r.y, TRAIN).unwrap();
        assert_eq!(placebo(&r, TRAIN, &p0).unwrap().runs, 80);
    }
}
