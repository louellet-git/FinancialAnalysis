//! The score of the market-level premium against the later real excess return of the S&P composite.
//! Spec: `analysis/macro/2026-11-21-long-history-design.md` (design 2ffd392, records R1 and R2, 7b6c050). Each line of
//! the score text names the section that it follows. The price measures; no model input comes from a return.
//!
//! Choices that the design leaves open are listed in the score text, under "Interpretations".

use super::market::{compute, known_year, CpiSource, Fred, MarketRow, Obs};
use super::nprandom::Pcg64;
use super::stats::mean;
use super::DcfError;
use std::collections::HashMap;
use std::fmt::Write as _;

type R<T> = Result<T, DcfError>;

pub const H_MAIN: i32 = 120;
pub const H_SEC: i32 = 60;
/// Block length of the bootstrap and number of phases (section 4).
pub const BLOCK: usize = 120;
pub const REPS: usize = 10_000;
pub const SEED: u64 = 20_261_121;
pub const ALPHA: f64 = 0.05;
/// 20 years of complete pairs (section 4, "the first forecast comes when 20 years of complete pairs exist").
pub const MIN_PAIRS_OOS: usize = 240;
pub const MIN_WINDOWS: usize = 3;

pub struct Inputs<'a> {
    pub cpi: &'a Fred,
    pub tb3ms: &'a Fred,
    pub nber: &'a Fred,
    pub gs10: &'a Fred,
}

pub struct Report {
    pub score_text: String,
    pub months_csv: String,
}

/// Month number: year * 12 + month - 1.
pub fn mi(y: i32, m: u32) -> i32 {
    y * 12 + m as i32 - 1
}

pub fn ym(m: i32) -> (i32, u32) {
    (m.div_euclid(12), m.rem_euclid(12) as u32 + 1)
}

fn label(m: i32) -> String {
    let (y, mo) = ym(m);
    format!("{y}-{mo:02}")
}

/// Why a month has no outcome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Drop {
    NoCpi,
    NoDividend,
    NoBill,
    NoData,
}

pub struct Series<'a> {
    obs: &'a [Obs],
    inp: &'a Inputs<'a>,
    src: CpiSource<'a>,
    first: i32,
}

impl<'a> Series<'a> {
    pub fn new(obs: &'a [Obs], inp: &'a Inputs<'a>) -> Self {
        let first = mi(obs[0].year, obs[0].month);
        Series { obs, inp, src: CpiSource::new(obs, Some(inp.cpi)), first }
    }

    fn obs_at(&self, m: i32) -> Option<&Obs> {
        let i = m - self.first;
        if i < 0 {
            None
        } else {
            self.obs.get(i as usize)
        }
    }

    /// The bill rate (percent a year) of month m: TB3MS from 1934-01, the NBER commercial paper rate before (section 3).
    fn bill(&self, m: i32) -> Option<f64> {
        let k = ym(m);
        if m >= mi(1934, 1) {
            self.inp.tb3ms.get(&k).copied()
        } else {
            self.inp.nber.get(&k).copied()
        }
    }

    /// The outcome deflator c = CPI_a / CPI_b for a <= b (section 3). The monthly ratios CPI(s-1)/CPI(s) come from
    /// the Shiller CPI when s <= 1913-01 and from CPIAUCNS when s >= 1913-02. The product telescopes to the end values.
    pub fn cpi_ratio(&self, a: i32, b: i32) -> Result<f64, Drop> {
        let pivot = mi(1913, 1);
        let shiller = |m: i32| self.obs_at(m).map(|o| o.cpi).ok_or(Drop::NoData);
        let fred = |m: i32| self.inp.cpi.get(&ym(m)).copied().ok_or(Drop::NoCpi);
        let mut c = 1.0;
        if a < pivot {
            c *= shiller(a)? / shiller(b.min(pivot))?;
        }
        if b > pivot {
            c *= fred(a.max(pivot))? / fred(b)?;
        }
        Ok(c)
    }

    /// X(t,H): the later real excess return (section 3). Total return from the monthly P and D, bills from the rate of
    /// the month before, CPI deflation, annualized over H months.
    pub fn outcome(&self, t: i32, h: i32) -> Result<f64, Drop> {
        let end = t + h;
        if self.obs_at(t).is_none() || self.obs_at(end).is_none() {
            return Err(Drop::NoData);
        }
        let c = self.cpi_ratio(t, end)?;
        let (mut g_tr, mut g_b) = (1.0f64, 1.0f64);
        for s in t + 1..=end {
            let (Some(o), Some(prev)) = (self.obs_at(s), self.obs_at(s - 1)) else {
                return Err(Drop::NoData);
            };
            let d = o.dividend.ok_or(Drop::NoDividend)?;
            let y = self.bill(s - 1).ok_or(Drop::NoBill)?;
            g_tr *= (o.price + d / 12.0) / prev.price;
            g_b *= (1.0 + y / 100.0).powf(1.0 / 12.0);
        }
        let k = 12.0 / h as f64;
        Ok((g_tr * c).powf(k) - (g_b * c).powf(k))
    }

    /// P1(t) = DY(t) + g(t) - r10(t) (section 3). None when an input is missing.
    pub fn p1(&self, t: i32) -> Option<f64> {
        let (y, m) = ym(t);
        let o = self.obs_at(t)?;
        let yk = known_year(y, m);
        if yk - 1871 < 20 {
            return None;
        }
        let d_k = self.src.december_dividend(yk)?;
        let d_0 = self.src.december_dividend(1871)?;
        let cpi_k = self.src.december(yk).ok()?;
        let cpi_0 = self.src.december(1871).ok()?;
        let g = ((d_k / cpi_k) / (d_0 / cpi_0)).powf(1.0 / (yk - 1871) as f64) - 1.0;
        let dy = d_k / o.price;
        Some(dy + g - self.r10(t)?)
    }

    /// r10(t) = GS10(t)/100 - ((CPIK(t) / CPIK(t-120))^(1/10) - 1) (section 3). GS10: the Shiller long rate before
    /// 1953-04, FRED GS10 from 1953-04.
    pub fn r10(&self, t: i32) -> Option<f64> {
        let (y, m) = ym(t);
        let o = self.obs_at(t)?;
        let cpik_now = self.src.known(y, m).ok()?;
        let cpik_back = self.src.known(y - 10, m).ok()?;
        let gs = if t >= mi(1953, 4) { self.inp.gs10.get(&(y, m)).copied()? } else { o.gs10 };
        Some(gs / 100.0 - ((cpik_now / cpik_back).powf(0.1) - 1.0))
    }

    /// CAPE10(t) = the real P(t) / the mean of the real annual E of the last 10 known years (record R3). The known years
    /// follow the dividend rule (section 5). Each December E is deflated with its own December CPI; P(t) with CPIK(t).
    pub fn cape10(&self, t: i32) -> Option<f64> {
        let (y, m) = ym(t);
        let o = self.obs_at(t)?;
        let yk = known_year(y, m);
        let mut sum = 0.0;
        for yr in yk - 9..=yk {
            sum += self.src.december_earnings(yr)? / self.src.december(yr).ok()?;
        }
        let mean_e = sum / 10.0;
        if mean_e <= 0.0 {
            return None;
        }
        Some((o.price / self.src.known(y, m).ok()?) / mean_e)
    }

    /// P2(t) = 1 / CAPE10(t) - r10(t) (section 3, record R3).
    pub fn p2(&self, t: i32) -> Option<f64> {
        Some(1.0 / self.cape10(t)? - self.r10(t)?)
    }
}

pub type Pair = Option<(f64, f64)>;

fn slope_from(n: f64, sx: f64, sy: f64, sxx: f64, sxy: f64) -> Option<f64> {
    if n < 2.0 {
        return None;
    }
    let den = n * sxx - sx * sx;
    if den <= 1e-12 * (n * sxx).abs().max(1e-300) {
        return None;
    }
    Some((n * sxy - sx * sy) / den)
}

/// The statistic of section 4: the mean of the OLS slopes of the phases. Phase = position modulo 120 (each phase takes
/// every 120th month). A phase with fewer than 2 pairs or no variance in the premium has no slope and is not counted.
pub fn phase_mean_slope(pairs: &[Pair]) -> Option<f64> {
    let mut acc = [[0.0f64; 5]; BLOCK];
    for (i, p) in pairs.iter().enumerate() {
        if let Some((x, y)) = p {
            let a = &mut acc[i % BLOCK];
            a[0] += 1.0;
            a[1] += x;
            a[2] += y;
            a[3] += x * x;
            a[4] += x * y;
        }
    }
    let slopes: Vec<f64> = acc.iter().filter_map(|a| slope_from(a[0], a[1], a[2], a[3], a[4])).collect();
    if slopes.is_empty() {
        None
    } else {
        Some(mean(&slopes))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Boot {
    pub b_hat: f64,
    pub p: f64,
}

/// The circular block bootstrap of section 4: blocks of 120 consecutive start months (the pair kept together), joined
/// to a pseudo-sample of the same length, the same statistic on each; one-sided p = (1 + #{b* - b_hat >= b_hat}) / (K + 1).
pub fn bootstrap(pairs: &[Pair], reps: usize, seed: u64) -> Option<Boot> {
    let b_hat = phase_mean_slope(pairs)?;
    let n = pairs.len();
    let mut rng = Pcg64::new(seed);
    let nb = n.div_ceil(BLOCK);
    let mut pseudo: Vec<Pair> = vec![None; n];
    let mut count = 0usize;
    for _ in 0..reps {
        for blk in 0..nb {
            let u = rng.bounded((n - 1) as u64)? as usize;
            for j in 0..BLOCK {
                let pos = blk * BLOCK + j;
                if pos >= n {
                    break;
                }
                pseudo[pos] = pairs[(u + j) % n];
            }
        }
        if let Some(b) = phase_mean_slope(&pseudo) {
            if b - b_hat >= b_hat {
                count += 1;
            }
        }
    }
    Some(Boot { b_hat, p: (1.0 + count as f64) / (reps as f64 + 1.0) })
}

#[derive(Debug, Clone, Copy)]
pub struct Oos {
    pub r2: f64,
    pub forecasts: usize,
}

/// The out-of-sample R2 of section 4: at month i the fit uses the pairs of s with s + H <= i (expanding; `lag` = H),
/// the benchmark is the mean of the same X values, the first forecast comes with `MIN_PAIRS_OOS` pairs.
pub fn oos_r2(pairs: &[Pair], lag: usize) -> Option<Oos> {
    let (mut n, mut sx, mut sy, mut sxx, mut sxy) = (0.0f64, 0.0, 0.0, 0.0, 0.0);
    let (mut sse_m, mut sse_b, mut cnt) = (0.0, 0.0, 0usize);
    for i in 0..pairs.len() {
        if i >= lag {
            if let Some((x, y)) = pairs[i - lag] {
                n += 1.0;
                sx += x;
                sy += y;
                sxx += x * x;
                sxy += x * y;
            }
        }
        let Some((x, y)) = pairs[i] else { continue };
        if (n as usize) < MIN_PAIRS_OOS {
            continue;
        }
        let Some(b) = slope_from(n, sx, sy, sxx, sxy) else { continue };
        let a = (sy - b * sx) / n;
        sse_m += (y - (a + b * x)).powi(2);
        sse_b += (y - sy / n).powi(2);
        cnt += 1;
    }
    if cnt == 0 || sse_b <= 0.0 {
        return None;
    }
    Some(Oos { r2: 1.0 - sse_m / sse_b, forecasts: cnt })
}

/// The OLS slope of one phase alone: the pairs at positions `phase`, `phase + 120`, ... (record R3: "phase January" is
/// the phase that starts at 1893-01, `phase` 0). Returns the slope and the number of windows.
pub fn single_phase_slope(pairs: &[Pair], phase: usize) -> Option<(f64, usize)> {
    let (mut n, mut sx, mut sy, mut sxx, mut sxy) = (0.0f64, 0.0, 0.0, 0.0, 0.0);
    for p in pairs.iter().skip(phase).step_by(BLOCK).flatten() {
        n += 1.0;
        sx += p.0;
        sy += p.1;
        sxx += p.0 * p.0;
        sxy += p.0 * p.1;
    }
    slope_from(n, sx, sy, sxx, sxy).map(|b| (b, n as usize))
}

/// Holm step-down adjusted p-values (one-sided, same order as the input).
pub fn holm(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|a, b| p[*a].total_cmp(&p[*b]));
    let mut adj = vec![0.0; m];
    let mut run = 0.0f64;
    for (rank, i) in order.iter().enumerate() {
        run = run.max(((m - rank) as f64 * p[*i]).min(1.0));
        adj[*i] = run;
    }
    adj
}

/// The monetary regime of a start month (section 6).
pub fn regime(m: i32) -> &'static str {
    let cuts = [(1914, 1, "P-1"), (1934, 1, "P-2"), (1951, 3, "P-3"), (1971, 8, "P-4"), (1979, 10, "P-5"), (2008, 12, "P-6"), (2022, 3, "P-7")];
    for (y, mo, name) in cuts {
        if m < mi(y, mo) {
            return name;
        }
    }
    "P-8"
}

type Span = (&'static str, (i32, u32), (i32, u32));

const REGIMES: [Span; 8] = [
    ("P-1", (1871, 1), (1913, 12)),
    ("P-2", (1914, 1), (1933, 12)),
    ("P-3", (1934, 1), (1951, 2)),
    ("P-4", (1951, 3), (1971, 7)),
    ("P-5", (1971, 8), (1979, 9)),
    ("P-6", (1979, 10), (2008, 11)),
    ("P-7", (2008, 12), (2022, 2)),
    ("P-8", (2022, 3), (2100, 12)),
];

/// Independent windows of a period (section 6): floor(start months / 120). A period with fewer than 3 has no regression.
pub fn independent_windows(start_months: usize) -> usize {
    start_months / BLOCK
}

pub fn has_regression(start_months: usize) -> bool {
    independent_windows(start_months) >= MIN_WINDOWS
}

struct Test {
    name: String,
    pairs: Vec<Pair>,
    boot: Option<Boot>,
    oos: Option<Oos>,
}

fn run_test(name: &str, pairs: Vec<Pair>, lag: usize) -> Test {
    let boot = bootstrap(&pairs, REPS, SEED);
    let oos = oos_r2(&pairs, lag);
    Test { name: name.to_string(), pairs, boot, oos }
}

fn count_pairs(p: &[Pair]) -> usize {
    p.iter().filter(|x| x.is_some()).count()
}

fn f4(x: Option<f64>) -> String {
    x.map_or("n/a".to_string(), |v| format!("{v:.4}"))
}

fn last_month(obs: &[Obs], has: impl Fn(&Obs) -> bool) -> i32 {
    obs.iter().filter(|o| has(o)).map(|o| mi(o.year, o.month)).max().unwrap_or(0)
}

/// The largest absolute difference between the Shiller gs10 column and FRED GS10 from 1953-04, with its month and the
/// number of months compared (record R3, point 7).
pub fn gs10_difference(obs: &[Obs], gs10: &Fred) -> (f64, i32, usize) {
    let (mut mx, mut at, mut n) = (0.0f64, 0, 0usize);
    for o in obs {
        let m = mi(o.year, o.month);
        if m < mi(1953, 4) {
            continue;
        }
        if let Some(f) = gs10.get(&(o.year, o.month)) {
            n += 1;
            let d = (o.gs10 - f).abs();
            if d > mx {
                mx = d;
                at = m;
            }
        }
    }
    (mx, at, n)
}

pub fn run(obs: &[Obs], inp: &Inputs) -> R<Report> {
    let s = Series::new(obs, inp);
    let first = mi(obs[0].year, obs[0].month);
    let last_obs = mi(obs[obs.len() - 1].year, obs[obs.len() - 1].month);
    let last_of = |f: &Fred| f.keys().map(|k| mi(k.0, k.1)).max().unwrap_or(0);
    // section 4 (line 89, record R3): the last month for which every series of the test has a value, minus H.
    // The series: CPIAUCNS, GS10, TB3MS and the Shiller dividend.
    let latest = last_of(inp.cpi).min(last_of(inp.gs10)).min(last_of(inp.tb3ms)).min(last_month(obs, |o| o.dividend.is_some()));
    let start = mi(1893, 1);
    let end120 = latest - H_MAIN;
    let end60 = latest - H_SEC;
    if end120 < start + 2 * H_MAIN {
        return Err(DcfError(format!("the data end at {}: the sample (from {}, H = 120) is too short", label(latest), label(start))));
    }

    // the engine: the primary premium and the four variants (record R1)
    let fred = Some(inp.cpi);
    let variants: [(f64, usize); 5] = [(0.02, 10), (0.01, 10), (0.03, 10), (0.02, 5), (0.02, 20)];
    let mut erp: Vec<HashMap<i32, f64>> = Vec::new();
    let mut base_rows: HashMap<i32, MarketRow> = HashMap::new();
    let mut gordon: HashMap<i32, f64> = HashMap::new();
    for (k, (l, n)) in variants.iter().enumerate() {
        let rows = compute(obs, fred, *l, *n)?;
        let mut mp = HashMap::new();
        for r in rows {
            let m = mi(r.year, r.month);
            if let Some(x) = r.erp_implied {
                mp.insert(m, x);
            }
            if k == 0 {
                if r.erp_gordon.is_finite() && r.status != "no_cpi" {
                    gordon.insert(m, r.erp_gordon);
                }
                base_rows.insert(m, r);
            }
        }
        erp.push(mp);
    }
    // point 7 of R3: the difference between the Shiller gs10 and FRED GS10; if above 0.0001, the engine runs once more
    // with FRED GS10 from 1953-04 for rr
    let (gdiff, gdiff_at, gdiff_n) = gs10_difference(obs, inp.gs10);
    let mut erp_alt: HashMap<i32, f64> = HashMap::new();
    if gdiff > 1e-4 {
        let alt: Vec<Obs> = obs
            .iter()
            .map(|o| {
                let mut c = o.clone();
                if mi(o.year, o.month) >= mi(1953, 4) {
                    if let Some(f) = inp.gs10.get(&(o.year, o.month)) {
                        c.gs10 = *f;
                    }
                }
                c
            })
            .collect();
        for r in compute(&alt, fred, 0.02, 10)? {
            if let Some(x) = r.erp_implied {
                erp_alt.insert(mi(r.year, r.month), x);
            }
        }
    }

    // the month records
    let p1: HashMap<i32, f64> = (first..=last_obs).filter_map(|t| s.p1(t).map(|v| (t, v))).collect();
    let p2: HashMap<i32, f64> = (first..=last_obs).filter_map(|t| s.p2(t).map(|v| (t, v))).collect();
    let mut x120: HashMap<i32, f64> = HashMap::new();
    let mut x60: HashMap<i32, f64> = HashMap::new();
    let mut drop120: HashMap<i32, Drop> = HashMap::new();
    let mut drop60: HashMap<i32, Drop> = HashMap::new();
    for t in first..=last_obs {
        match s.outcome(t, H_MAIN) {
            Ok(v) => {
                x120.insert(t, v);
            }
            Err(d) => {
                drop120.insert(t, d);
            }
        }
        match s.outcome(t, H_SEC) {
            Ok(v) => {
                x60.insert(t, v);
            }
            Err(d) => {
                drop60.insert(t, d);
            }
        }
    }

    let sample: Vec<i32> = (start..=end120).collect();
    let sample60: Vec<i32> = (start..=end60).collect();
    let pairs_of = |months: &[i32], x: &HashMap<i32, f64>, pred: &dyn Fn(i32) -> Option<f64>| -> Vec<Pair> { months.iter().map(|t| Some((pred(*t)?, *x.get(t)?))).collect() };
    let p1_pairs = pairs_of(&sample, &x120, &|t| p1.get(&t).copied());
    let p2_pairs = pairs_of(&sample, &x120, &|t| p2.get(&t).copied());
    let p1_pairs60 = pairs_of(&sample60, &x60, &|t| p1.get(&t).copied());
    let eng_pairs: Vec<Vec<Pair>> = erp.iter().map(|mp| pairs_of(&sample, &x120, &|t| mp.get(&t).copied())).collect();
    let gor_pairs = pairs_of(&sample, &x120, &|t| gordon.get(&t).copied());
    let alt_pairs = pairs_of(&sample, &x120, &|t| erp_alt.get(&t).copied());

    let primary_engine = run_test("engine erp_implied (L 2%, N 10)", eng_pairs[0].clone(), 120);
    let primary_p1 = run_test("P1", p1_pairs.clone(), 120);
    let fam_names = ["engine L 1%", "engine L 3%", "engine N 5", "engine N 20", "engine erp_gordon (L 2%)"];
    let mut family: Vec<Test> = Vec::new();
    for (k, nm) in fam_names.iter().enumerate() {
        let pairs = if k < 4 { eng_pairs[k + 1].clone() } else { gor_pairs.clone() };
        family.push(run_test(nm, pairs, 120));
    }
    // the P1 family, m = 6 (section 6): S1, S2, B1 to B4
    let blocks = [("B1", (1893, 1), (1922, 12)), ("B2", (1923, 1), (1952, 12)), ("B3", (1953, 1), (1982, 12)), ("B4", (1983, 1), (2016, 10))];
    let block_range = |lo: (i32, u32), hi: (i32, u32)| -> (i32, i32) { (mi(lo.0, lo.1), mi(hi.0, hi.1).min(end120)) };
    let slice = |pairs: &[Pair], a: i32, b: i32| pairs[(a - start) as usize..=(b - start) as usize].to_vec();
    let s1 = run_test("S1  P1, H = 60, 1893-01 to end", p1_pairs60.clone(), 60);
    let s2 = run_test("S2  P2, H = 120", p2_pairs.clone(), 120);
    let mut fam6: Vec<Test> = vec![s1, s2];
    for (name, lo, hi) in blocks {
        let (a, b) = block_range(lo, hi);
        let pairs = slice(&p1_pairs, a, b);
        let boot = if has_regression((b - a + 1) as usize) { bootstrap(&pairs, REPS, SEED) } else { None };
        fam6.push(Test { name: format!("{name} P1, H = 120, {} to {}", label(a), label(b)), pairs, boot, oos: None });
    }

    // the score text
    let mut t = String::new();
    let w = &mut t;
    let _ = writeln!(w, "SCORE of the market-level premium, complete run. Design analysis/macro/2026-11-21-long-history-design.md (2ffd392, R1, R2: 7b6c050, R3 after the first run)");
    let _ = writeln!(w, "A measurement. The price is the input of the search; no model input comes from a return. One run on the frozen sample.");
    let _ = writeln!(w, "Latest data month with every series (CPIAUCNS, GS10, TB3MS, Shiller dividend; section 4, line 89): {}. Last start month H=120: {}. H=60: {}.", label(latest), label(end120), label(end60));
    let _ = writeln!(w, "Bootstrap (section 4): circular blocks of {BLOCK} start months, K = {REPS}, seed {SEED}. Generator: PCG64 (numpy default_rng) from the repo nprandom, block starts by `bounded`. Phases: {BLOCK}.");
    let _ = writeln!(w, "Sample (section 4): start months {} to {} = {} calendar months; independent windows floor(n/120) = {}.", label(start), label(end120), sample.len(), independent_windows(sample.len()));

    let _ = writeln!(w, "\nCOUNTS (sections 4, 9 R2)");
    let in_sample = |m: &i32| *m >= start && *m <= end120;
    let n_x120 = sample.iter().filter(|t| x120.contains_key(t)).count();
    let _ = writeln!(w, "X(t,120) present in the sample: {n_x120} of {}", sample.len());
    for (d, nm) in [(Drop::NoCpi, "CPIAUCNS value missing at the end month (R2: end month 2025-10, start 2015-10)"), (Drop::NoDividend, "Shiller dividend missing in a month of the window"), (Drop::NoBill, "bill rate missing"), (Drop::NoData, "data end")] {
        let mut ms: Vec<i32> = drop120.iter().filter(|(m, x)| **x == d && in_sample(m)).map(|(m, _)| *m).collect();
        ms.sort();
        let list: Vec<String> = ms.iter().take(6).map(|m| label(*m)).collect();
        let _ = writeln!(w, "  X120 dropped, {nm}: {} {}", ms.len(), list.join(" "));
    }
    let n_x60 = sample60.iter().filter(|t| x60.contains_key(t)).count();
    let mut d60: Vec<String> = drop60.iter().filter(|(m, _)| **m >= start && **m <= end60).map(|(m, d)| format!("{} {:?}", label(*m), d)).collect();
    d60.sort();
    let _ = writeln!(w, "X(t,60) present in {} to {}: {n_x60} of {}; dropped: {}", label(start), label(end60), sample60.len(), d60.join(", "));
    let miss = |f: &dyn Fn(i32) -> bool| sample.iter().filter(|t| !f(**t)).count();
    let _ = writeln!(w, "Predictor missing in the sample: P1 {}, P2 {}, engine erp_implied {}", miss(&|t| p1.contains_key(&t)), miss(&|t| p2.contains_key(&t)), miss(&|t| erp[0].contains_key(&t)));
    let no_cpi_rows = base_rows.values().filter(|r| r.status == "no_cpi").count();
    let no_root = base_rows.values().filter(|r| r.status.starts_with("no_root")).count();
    let _ = writeln!(w, "Engine rows with status no_cpi (R2): {no_cpi_rows}; with no root: {no_root}. No value is filled.");
    let _ = writeln!(w, "Pairs in the sample: P1 {}, P2 {}, engine {}, L1 {}, L3 {}, N5 {}, N20 {}, gordon {}", count_pairs(&p1_pairs), count_pairs(&p2_pairs), count_pairs(&eng_pairs[0]), count_pairs(&eng_pairs[1]), count_pairs(&eng_pairs[2]), count_pairs(&eng_pairs[3]), count_pairs(&eng_pairs[4]), count_pairs(&gor_pairs));

    let line = |w: &mut String, tt: &Test| {
        let _ = writeln!(w, "  {:<34} pairs {:>5}  b_hat {:>9}  p {:>8}  OOS R2 {:>9} ({} forecasts)", tt.name, count_pairs(&tt.pairs), f4(tt.boot.map(|b| b.b_hat)), f4(tt.boot.map(|b| b.p)), f4(tt.oos.map(|o| o.r2)), tt.oos.map_or(0, |o| o.forecasts));
    };
    let pass = |tt: &Test| -> bool { tt.boot.is_some_and(|b| b.b_hat > 0.0 && b.p < ALPHA) && tt.oos.is_some_and(|o| o.r2 > 0.0) };

    let _ = writeln!(w, "\nPRIMARY TESTS (section 4: the mean phase slope, one-sided p < {ALPHA} AND OOS R2 > 0)");
    line(w, &primary_p1);
    let _ = writeln!(w, "  P1: {}", if pass(&primary_p1) { "PASS" } else { "FAIL" });
    line(w, &primary_engine);
    let eng_pass = pass(&primary_engine);
    let _ = writeln!(w, "  engine: rejection rule {}", if eng_pass { "PASS" } else { "FAIL" });
    let below = match (primary_engine.oos, primary_p1.oos) {
        (Some(e), Some(p)) => e.r2 < p.r2,
        _ => true,
    };
    let _ = writeln!(w, "ENGINE FAILURE RULE (section 4, confirmed 2026-11-21): fails if the rejection rule fails or its OOS R2 is below that of P1.");
    let _ = writeln!(w, "  engine OOS R2 {} < P1 OOS R2 {}: {}", f4(primary_engine.oos.map(|o| o.r2)), f4(primary_p1.oos.map(|o| o.r2)), below);
    let _ = writeln!(w, "  engine result: {}", if eng_pass && !below { "PASS (does not fail)" } else { "FAIL" });

    let _ = writeln!(w, "\nP1 FAMILY (section 6: S1, S2, B1 to B4; m = 6, Holm on the bootstrap p, family-wise {ALPHA}, one-sided; the OOS R2 is descriptive)");
    let _ = writeln!(w, "  P2 = 1 / CAPE10 - r10 (section 3, R3); CAPE10 = real P(t) / mean real annual E of the last 10 known years.");
    let ps6: Vec<f64> = fam6.iter().map(|f| f.boot.map_or(1.0, |b| b.p)).collect();
    let adj6 = holm(&ps6);
    for (f, a) in fam6.iter().zip(&adj6) {
        line(w, f);
        let _ = writeln!(w, "      Holm-adjusted p {:.4}: {}", a, if *a <= ALPHA { "reject H0" } else { "no rejection" });
    }

    let _ = writeln!(w, "\nENGINE SECONDARY FAMILY (section 9 R1: m = 5, Holm on the bootstrap p, family-wise {ALPHA}, one-sided; separate from the P1 family; the OOS R2 is descriptive)");
    let ps: Vec<f64> = family.iter().map(|f| f.boot.map_or(1.0, |b| b.p)).collect();
    let adj = holm(&ps);
    for (f, a) in family.iter().zip(&adj) {
        line(w, f);
        let _ = writeln!(w, "      Holm-adjusted p {:.4}: {}", a, if *a <= ALPHA { "reject H0" } else { "no rejection" });
    }

    let _ = writeln!(w, "\nGS10 (R3 point 7): Shiller gs10 against FRED GS10 from 1953-04: maximum absolute difference {gdiff:.4} percentage points at {} ({gdiff_n} months compared).", label(gdiff_at));
    if gdiff > 1e-4 {
        let alt = run_test("engine erp_implied, FRED GS10 in rr", alt_pairs.clone(), 120);
        let _ = writeln!(w, "  The difference is above 0.0001: the engine ran once more with FRED GS10 from 1953-04 for rr. Both results (the first is the primary, R1):");
        line(w, &primary_engine);
        line(w, &alt);
        let alt_below = match (alt.oos, primary_p1.oos) {
            (Some(e), Some(p)) => e.r2 < p.r2,
            _ => true,
        };
        let _ = writeln!(w, "  with FRED GS10: rejection rule {}; OOS R2 below P1: {}", if pass(&alt) { "PASS" } else { "FAIL" }, alt_below);
    }

    let _ = writeln!(w, "\nPERIODS (section 6: a window belongs to the period of its start month; independent windows = floor(start months / 120))");
    let _ = writeln!(w, "Monetary regimes (descriptive only, no p-value, not in a Holm family):");
    for (name, lo, hi) in REGIMES {
        let (a, b) = (mi(lo.0, lo.1).max(start), mi(hi.0, hi.1).min(end120));
        let n = if b >= a { (b - a + 1) as usize } else { 0 };
        let note = if has_regression(n) {
            let sl = |pairs: &[Pair]| phase_mean_slope(&pairs[(a - start) as usize..=(b - start) as usize]);
            format!("mean slope P1 {}, engine {}", f4(sl(&p1_pairs)), f4(sl(&eng_pairs[0])))
        } else {
            "no regression (fewer than 3 independent windows)".to_string()
        };
        let _ = writeln!(w, "  {name}: start months in the sample {n}, independent windows {}: {note}", independent_windows(n));
    }
    let _ = writeln!(w, "30-year blocks (section 6). P1 blocks are in the P1 family above. Engine blocks: not in a Holm family, uncorrected:");
    for (name, lo, hi) in blocks {
        let (a, b) = block_range(lo, hi);
        let n = (b - a + 1) as usize;
        if has_regression(n) {
            for (who, pairs) in [("P1", slice(&p1_pairs, a, b)), ("engine", slice(&eng_pairs[0], a, b))] {
                let bt = bootstrap(&pairs, REPS, SEED);
                let tag = if who == "P1" { "in the P1 family" } else { "not in a Holm family, uncorrected" };
                let _ = writeln!(w, "  {name} {who}: start months {n}, independent windows {}, b_hat {}, p {} ({tag})", independent_windows(n), f4(bt.map(|x| x.b_hat)), f4(bt.map(|x| x.p)));
            }
        } else {
            let _ = writeln!(w, "  {name}: start months {n}, independent windows {}: no regression", independent_windows(n));
        }
    }

    let _ = writeln!(w, "\nJANUARY PHASE (section 4, R3 point 4; descriptive): the single phase that starts at {}, windows at every 120th month.", label(start));
    for (nm, pairs) in [("P1", &p1_pairs), ("P2", &p2_pairs), ("engine erp_implied", &eng_pairs[0])] {
        match single_phase_slope(pairs, 0) {
            Some((b, n)) => {
                let _ = writeln!(w, "  {nm}: slope {b:.4} over {n} non-overlapping windows");
            }
            None => {
                let _ = writeln!(w, "  {nm}: no slope");
            }
        }
    }

    let _ = writeln!(w, "\nINTERPRETATIONS that remain (the design is not exact here; each can change a number)");
    for l in [
        "1. S1 (H = 60) uses the statistic and bootstrap of section 4 as written: 120 phases, blocks of 120 start months, OOS lag 60. The design gives no change for H = 60.",
        "2. 20 years of complete pairs = 240 monthly pairs.",
        "3. The crossing ratio of the deflator (1912-12 to 1913-01) uses the Shiller CPI; Dr(Y) of g(t) and the annual E of CAPE10 deflate with the December CPI of the engine rule (Shiller before 1913, CPIAUCNS from 1913); P(t) in CAPE10 deflates with CPIK(t).",
        "4. The phase slope uses the calendar months of the sample; a dropped month leaves a gap. Windows per period count calendar start months.",
        "5. The bootstrap phase of a pseudo-sample is its position modulo 120.",
    ] {
        let _ = writeln!(w, "  {l}");
    }

    // the CSV by month
    let mut csv = String::from("month,period,erp_implied,P1,P2,X60,X120,erp_L1,erp_L3,erp_N5,erp_N20,erp_gordon,erp_fredgs10,status\n");
    let f6 = |x: Option<&f64>| x.map_or(String::new(), |v| format!("{v:.6}"));
    let mut months: Vec<i32> = base_rows.keys().copied().collect();
    months.sort();
    for m in months {
        let st = base_rows[&m].status;
        let _ = writeln!(csv, "{},{},{},{},{},{},{},{},{},{},{},{},{},{}", label(m), regime(m), f6(erp[0].get(&m)), f6(p1.get(&m)), f6(p2.get(&m)), f6(x60.get(&m)), f6(x120.get(&m)), f6(erp[1].get(&m)), f6(erp[2].get(&m)), f6(erp[3].get(&m)), f6(erp[4].get(&m)), f6(gordon.get(&m)), f6(erp_alt.get(&m)), st);
    }
    Ok(Report { score_text: t, months_csv: csv })
}

#[cfg(test)]
#[path = "market_score_tests.rs"]
mod tests;
