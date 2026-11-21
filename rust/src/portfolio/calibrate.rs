//! `fa portfolio calibrate`: the calibration of the weekly rule (A-009) against what happened. Tests T2 to T4 of
//! `PLANNING/board/T-20261121-blind-projection-test.md` (Revision 1).
//!
//! For each rebalance week the scorer gives the score of each ticker from a [`View`] of that day (the prices before the
//! day, the SEC facts filed before it). The rank inside the sector is made from these scores. Only after that, the
//! outcome is read: the close of the rebalance day and the close 13 weeks (91 days) later. The outcome is never an input
//! of a score. The entries are those of `weekly_step`, the function of the strategy `value-rank-weekly` (one
//! implementation).
//!
//! Limits: the outcome is a price return (no dividends); the list is today's list (survivorship); a ticker that stops
//! trading keeps its last close (no failure loss), as in the backtest; a week with fewer than 91 days of data after it
//! has no outcome and is not in the statistics.

use super::strategies::{rank_in_sector, weekly_step, WEEKLY_HOLD_DAYS};
use super::{Prices, View, STALE_DAYS};
use crate::dcf::auto_pit::ticker_half;
use crate::ratemodels::rng::Rng;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

/// The horizon of the outcome in days: 13 weeks, the hold of the rule.
pub const HORIZON_DAYS: i64 = WEEKLY_HOLD_DAYS;

/// One scored ticker in one week.
#[derive(Debug, Clone, PartialEq)]
pub struct RankRow {
    pub date: i64,
    pub ticker: String,
    pub sector: String,
    /// The rank inside the sector (A-009 item 3).
    pub rank: f64,
    /// The P50 / price.
    pub ratio: f64,
    /// The outcome: the return from the close of `date` to the close 91 days later. `None` if the data end before it.
    pub fwd: Option<f64>,
    /// The outcome less the mean outcome of the scored list of that week.
    pub excess: Option<f64>,
}

/// One entry of the weekly rule.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryRow {
    pub row: RankRow,
    /// The ticker was in the book and was dropped by the 91-day exit of the same call, then chosen again.
    pub reentry: bool,
}

/// The return of a ticker from the close of `day` (a fresh close) to the last close on or before `day + 91`. `None` if
/// there is no fresh close on `day`, or if `day + 91` is after `last_day` (the data are not there yet).
pub fn forward_return(prices: &Prices, ticker: &str, day: i64, last_day: i64) -> Option<f64> {
    if day + HORIZON_DAYS > last_day {
        return None;
    }
    let (pd, p0) = prices.asof(ticker, day)?;
    if day - pd > STALE_DAYS || p0 <= 0.0 {
        return None;
    }
    let (_, p1) = prices.asof(ticker, day + HORIZON_DAYS)?;
    Some(p1 / p0 - 1.0)
}

/// The score of each ticker for the view of one rebalance day.
pub type Scorer<'a> = dyn Fn(&View, &[String]) -> Vec<(String, f64)> + 'a;

/// The rows of each rebalance day in `rdays` (date order) and the entries of the weekly rule. The score of a day is made
/// first, from the view of that day; the outcomes are read after it.
pub fn collect(prices: &Prices, universe: &[String], score: &Scorer, sector_of: &BTreeMap<String, String>, rdays: &[i64], last_day: i64) -> (Vec<RankRow>, Vec<EntryRow>) {
    let mut rows = Vec::new();
    let mut entries = Vec::new();
    let mut open: BTreeMap<String, i64> = BTreeMap::new();
    for &d in rdays {
        let scored = score(&View::new(d, prices), universe); // the score is fixed here
        let ranked = rank_in_sector(&scored, sector_of);
        let before = open.clone();
        let new = weekly_step(&mut open, d, &scored, sector_of);
        // the outcomes, read after the scores
        let fwd: Vec<Option<f64>> = ranked.iter().map(|r| forward_return(prices, &r.ticker, d, last_day)).collect();
        let known: Vec<f64> = fwd.iter().flatten().copied().collect();
        let ew = (!known.is_empty()).then(|| known.iter().sum::<f64>() / known.len() as f64);
        let day_rows: Vec<RankRow> = ranked
            .iter()
            .zip(&fwd)
            .map(|(r, f)| RankRow { date: d, ticker: r.ticker.clone(), sector: r.sector.clone(), rank: r.rank, ratio: r.score, fwd: *f, excess: f.zip(ew).map(|(a, b)| a - b) })
            .collect();
        for t in new {
            if let Some(row) = day_rows.iter().find(|r| r.ticker == t) {
                entries.push(EntryRow { row: row.clone(), reentry: before.contains_key(&t) });
            }
        }
        rows.extend(day_rows);
    }
    (rows, entries)
}

fn opt_num(x: Option<f64>) -> String {
    x.map_or(String::new(), |v| format!("{v:.6}"))
}

/// The CSV of the rank rows: date, ticker, sector, rank, ratio, fwd_13w, excess_13w.
pub fn ranks_csv(rows: &[RankRow]) -> String {
    let mut s = String::from("date,ticker,sector,rank,ratio,fwd_13w,excess_13w\n");
    for r in rows {
        let _ = writeln!(s, "{},{},{},{:.6},{:.6},{},{}", mapcheck::dates::format(r.date), r.ticker, r.sector, r.rank, r.ratio, opt_num(r.fwd), opt_num(r.excess));
    }
    s
}

/// The CSV of the entries: date, ticker, sector, rank, ratio, reentry, fwd_13w, excess_13w.
pub fn entries_csv(entries: &[EntryRow]) -> String {
    let mut s = String::from("date,ticker,sector,rank,ratio,reentry,fwd_13w,excess_13w\n");
    for e in entries {
        let r = &e.row;
        let _ = writeln!(s, "{},{},{},{:.6},{:.6},{},{},{}", mapcheck::dates::format(r.date), r.ticker, r.sector, r.rank, r.ratio, e.reentry, opt_num(r.fwd), opt_num(r.excess));
    }
    s
}

/// A stress window (descriptive only): a name and the first and last day.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub name: String,
    pub start: i64,
    pub end: i64,
}

/// The windows of a CSV text `name,start,end` (YYYY-MM-DD); a line that starts with `#` and the header `name,...` are skipped.
pub fn parse_windows(text: &str) -> Result<Vec<Window>, String> {
    let mut out = Vec::new();
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("name,")) {
        let f: Vec<&str> = l.split(',').map(str::trim).collect();
        let day = |i: usize| f.get(i).and_then(|t| mapcheck::dates::parse(t)).ok_or_else(|| format!("window line '{l}': name,start,end with dates YYYY-MM-DD"));
        out.push(Window { name: f[0].to_string(), start: day(1)?, end: day(2)? });
    }
    Ok(out)
}

/// The parameters of the statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatParams {
    /// The length of a block of the primary circular bootstrap, in weeks (26 by Revision 1). The blocks 13 and 52 are
    /// reported as checks.
    pub block_weeks: usize,
    /// The seed of the bootstrap.
    pub seed: u64,
    /// The number of resamples.
    pub reps: usize,
}

/// The blocks of the checks of T2, in weeks.
pub const CHECK_BLOCKS: [usize; 2] = [13, 52];
/// The first week of the expanding-window tail test.
pub const TAIL_FROM: &str = "2019-01-01";
/// The expanding window needs at least this many closed entries to state a P10 and a P5.
pub const MIN_TAIL: usize = 30;
/// The periods of T4 (first year, last year).
pub const PERIODS: [(i64, i64); 3] = [(2018, 2020), (2021, 2023), (2024, 2026)];

/// The result of a bootstrap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Boot {
    /// The statistic of the data: the sum of the numerators over the sum of the counts.
    pub stat: f64,
    /// The standard error: the standard deviation of the statistic over the resamples.
    pub se: f64,
    /// One-sided p: the share of resamples whose centered statistic is at least `stat`.
    pub p: f64,
    /// The 2.5% and the 97.5% points of the statistic over the resamples.
    pub lo: f64,
    pub hi: f64,
}

/// A circular block bootstrap of a ratio statistic. `series` has one `(sum, count)` for each week, in date order; the
/// statistic is the total sum over the total count (a mean of weekly values: `(x, 1)`; a share: `(hits, entries)`).
/// A resample takes blocks of `block` consecutive weeks (wrapping at the end) from random starts, until it has as many weeks
/// as the series. The null value is `null`: the resample distribution is moved so that its centre is at `null`, and p is the
/// share of resamples at or above the observed statistic. The stream of random numbers depends on `seed` only.
pub fn block_bootstrap(series: &[(f64, f64)], block: usize, reps: usize, seed: u64, null: f64) -> Option<Boot> {
    let n = series.len();
    let (s0, c0) = series.iter().fold((0.0, 0.0), |a, x| (a.0 + x.0, a.1 + x.1));
    if n == 0 || c0 <= 0.0 || reps == 0 {
        return None;
    }
    let block = block.clamp(1, n);
    let stat = s0 / c0;
    let mut rng = Rng::for_path(seed, 0);
    let mut stats = Vec::with_capacity(reps);
    for _ in 0..reps {
        let (mut s, mut c, mut taken) = (0.0, 0.0, 0usize);
        while taken < n {
            let start = (rng.next_u64() % n as u64) as usize;
            for j in 0..block.min(n - taken) {
                let x = series[(start + j) % n];
                s += x.0;
                c += x.1;
                taken += 1;
            }
        }
        if c > 0.0 {
            stats.push(s / c);
        }
    }
    if stats.is_empty() {
        return None;
    }
    let m = stats.len() as f64;
    let mean = stats.iter().sum::<f64>() / m;
    let se = (stats.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (m - 1.0).max(1.0)).sqrt();
    let p = stats.iter().filter(|x| **x - mean + null >= stat - 1e-12).count() as f64 / m;
    stats.sort_by(|a, b| a.total_cmp(b));
    Some(Boot { stat, se, p, lo: percentile(&stats, 0.025), hi: percentile(&stats, 0.975) })
}

/// The decile of a rank: 0 for the rank below 0.1, 9 for the rank of 0.9 or more.
pub fn decile(rank: f64) -> usize {
    ((rank * 10.0).floor().max(0.0) as usize).min(9)
}

/// The weekly spread d_w: for each date, the mean outcome (excess) of the top decile less that of the bottom decile (rows
/// with no outcome are left out; a date that lacks one of the two deciles gives no spread). In date order.
pub fn weekly_spreads(rows: &[&RankRow]) -> Vec<(i64, f64)> {
    let mut by_day: BTreeMap<i64, ([f64; 2], [usize; 2])> = BTreeMap::new();
    for r in rows {
        let Some(x) = r.excess else { continue };
        let k = match decile(r.rank) {
            9 => 0,
            0 => 1,
            _ => continue,
        };
        let e = by_day.entry(r.date).or_default();
        e.0[k] += x;
        e.1[k] += 1;
    }
    by_day.into_iter().filter(|(_, (_, n))| n[0] > 0 && n[1] > 0).map(|(d, (s, n))| (d, s[0] / n[0] as f64 - s[1] / n[1] as f64)).collect()
}

/// The mean outcome and the count of each decile over the rows with an outcome.
pub fn decile_means(rows: &[&RankRow]) -> [(f64, usize); 10] {
    let mut out = [(0.0, 0usize); 10];
    for r in rows {
        if let Some(x) = r.excess {
            let k = decile(r.rank);
            out[k].0 += x;
            out[k].1 += 1;
        }
    }
    out.map(|(s, n)| (if n > 0 { s / n as f64 } else { f64::NAN }, n))
}

/// The means of the 13 phases of a weekly series: phase `k` has the weeks with the index `k`, `k + 13`, `k + 26`, ... in the
/// list (the outcomes of one phase do not overlap). A phase with no week is left out.
pub fn phase_means(series: &[(i64, f64)]) -> Vec<f64> {
    (0..13)
        .filter_map(|k| {
            let v: Vec<f64> = series.iter().skip(k).step_by(13).map(|x| x.1).collect();
            (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
        })
        .collect()
}

/// The percentile `q` (0 to 1) of a sorted list, with linear steps between the points (the "type 7" rule).
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    match sorted.len() {
        0 => f64::NAN,
        1 => sorted[0],
        n => {
            let h = q * (n as f64 - 1.0);
            let (lo, hi) = (h.floor() as usize, h.ceil() as usize);
            sorted[lo] + (sorted[hi] - sorted[lo]) * (h - lo as f64)
        }
    }
}

/// The count, the hit rate (the share above 0), the mean, P10 and P5 of a list of outcomes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dist {
    pub n: usize,
    pub hit: f64,
    pub mean: f64,
    pub p10: f64,
    pub p5: f64,
}

pub fn dist(xs: &[f64]) -> Dist {
    let mut s = xs.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let n = s.len();
    let nn = n.max(1) as f64;
    Dist { n, hit: s.iter().filter(|x| **x > 0.0).count() as f64 / nn, mean: if n > 0 { s.iter().sum::<f64>() / nn } else { f64::NAN }, p10: percentile(&s, 0.10), p5: percentile(&s, 0.05) }
}

/// The calendar year of a day number.
pub fn year_of(day: i64) -> i64 {
    mapcheck::dates::format(day)[..4].parse().unwrap_or(0)
}

/// T3, part 1: for each week with at least one entry with an outcome, the hit rate of the entries (the share with an
/// excess above 0) less the share of all scored names with an excess above 0 in that week (the base rate). In date order.
pub fn base_rate_differences(rows: &[RankRow], entries: &[EntryRow]) -> Vec<(i64, f64)> {
    let mut base: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
    for r in rows {
        if let Some(x) = r.excess {
            let e = base.entry(r.date).or_default();
            e.0 += usize::from(x > 0.0);
            e.1 += 1;
        }
    }
    let mut hit: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
    for e in entries {
        if let Some(x) = e.row.excess {
            let h = hit.entry(e.row.date).or_default();
            h.0 += usize::from(x > 0.0);
            h.1 += 1;
        }
    }
    hit.into_iter().filter_map(|(d, (h, n))| base.get(&d).map(|(b, m)| (d, h as f64 / n as f64 - *b as f64 / *m as f64))).collect()
}

/// One week of the expanding-window tail test: the stated P10 and P5 (from the entries that closed before the week) and
/// how many of the entries of the week fell below each.
#[derive(Debug, Clone, PartialEq)]
pub struct TailWeek {
    pub date: i64,
    /// The number of closed entries behind the stated figures.
    pub closed: usize,
    pub p10: f64,
    pub p5: f64,
    /// The entries of the week with an outcome.
    pub n: usize,
    pub below10: usize,
    pub below5: usize,
}

/// The tail test. For each week `W` from `from` on with entries that have an outcome: the stated P10 and P5 are the
/// percentiles of the excess of all entries that closed before `W` (`date + 91 < W`, with an outcome); a week with fewer
/// than `MIN_TAIL` closed entries is left out. Then the entries of `W` are counted below the stated figures. In date order.
pub fn tail_weeks(entries: &[EntryRow], from: i64) -> Vec<TailWeek> {
    let known: Vec<(i64, f64)> = entries.iter().filter_map(|e| e.row.excess.map(|x| (e.row.date, x))).collect();
    let weeks: BTreeSet<i64> = known.iter().map(|x| x.0).filter(|d| *d >= from).collect();
    let mut out = Vec::new();
    for w in weeks {
        let mut closed: Vec<f64> = known.iter().filter(|(d, _)| d + HORIZON_DAYS < w).map(|x| x.1).collect();
        if closed.len() < MIN_TAIL {
            continue;
        }
        closed.sort_by(|a, b| a.total_cmp(b));
        let (p10, p5) = (percentile(&closed, 0.10), percentile(&closed, 0.05));
        let now: Vec<f64> = known.iter().filter(|(d, _)| *d == w).map(|x| x.1).collect();
        out.push(TailWeek { date: w, closed: closed.len(), p10, p5, n: now.len(), below10: now.iter().filter(|x| **x < p10).count(), below5: now.iter().filter(|x| **x < p5).count() });
    }
    out
}

/// The mean of a list of numbers (NaN if it is empty).
fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn pct(x: f64) -> String {
    format!("{:+.2}%", 100.0 * x)
}

fn verdict(b: bool) -> &'static str {
    if b {
        "PASS"
    } else {
        "FAIL"
    }
}

fn boot_line(label: &str, b: Option<Boot>) -> String {
    match b {
        Some(b) => format!("{label}: mean {}, bootstrap se {}, one-sided p {:.4}\n", pct(b.stat), pct(b.se), b.p),
        None => format!("{label}: no data\n"),
    }
}

/// The share of weeks `(below, n)` as one ratio, with the interval of the bootstrap.
fn share(weeks: &[(f64, f64)], p: &StatParams) -> Option<Boot> {
    block_bootstrap(weeks, p.block_weeks, p.reps, p.seed, 0.0)
}

/// The tail coverage in a set of weeks: the shares below the stated P10 and P5 and the count of entries.
fn coverage(weeks: &[&TailWeek]) -> (f64, f64, usize) {
    let n: usize = weeks.iter().map(|w| w.n).sum();
    let nn = n.max(1) as f64;
    (weeks.iter().map(|w| w.below10).sum::<usize>() as f64 / nn, weeks.iter().map(|w| w.below5).sum::<usize>() as f64 / nn, n)
}

/// A group of the statistics of T4: the mean entry excess and the mean d_w (NaN if empty).
fn group_means(entries: &[&EntryRow], spreads: &[(i64, f64)]) -> (f64, usize, f64, usize) {
    let xs: Vec<f64> = entries.iter().filter_map(|e| e.row.excess).collect();
    (mean(&xs), xs.len(), mean(&spreads.iter().map(|x| x.1).collect::<Vec<_>>()), spreads.len())
}

/// The whole text of the calibration: each test with its pass rule, its value, and pass or fail, then the headline.
/// `rows` and `entries` come from [`collect`]; the rows and entries with no outcome are left out of the statistics. T1 is
/// another command (`fa dcf pit --cut 2015`): the headline here covers T2 to T4.
pub fn summary(rows: &[RankRow], entries: &[EntryRow], windows: &[Window], p: &StatParams) -> String {
    let mut t = String::new();
    let sel: Vec<&RankRow> = rows.iter().filter(|r| r.excess.is_some()).collect();
    let ent: Vec<&EntryRow> = entries.iter().filter(|e| e.row.excess.is_some()).collect();
    let _ = writeln!(
        t,
        "CALIBRATION of the weekly rule (A-009). Block {} weeks (checks {:?}), seed {}, {} resamples. Outcome = 13-week (91 days) return less the equal weight of the scored list of that week. Price returns, today's list (survivorship), today's SIC sector.",
        p.block_weeks, CHECK_BLOCKS, p.seed, p.reps
    );
    let _ = writeln!(t, "{} observations and {} entries with an outcome (of {} and {}).", sel.len(), ent.len(), rows.len(), entries.len());

    // ---- T2 ----
    let dw = weekly_spreads(&sel);
    let dw_series: Vec<(f64, f64)> = dw.iter().map(|x| (x.1, 1.0)).collect();
    let primary = block_bootstrap(&dw_series, p.block_weeks, p.reps, p.seed, 0.0);
    let t2 = primary.is_some_and(|b| b.stat > 0.0 && b.p < 0.05);
    let _ = writeln!(t, "\nT2. THE RANK AGAINST THE OUTCOME. Rule: mean(d_w) > 0 with one-sided p < 0.05 (block {} weeks). d_w = mean excess of the top decile less that of the bottom decile of the week.", p.block_weeks);
    let _ = writeln!(t, "  decile of the rank (0 = lowest): mean excess, count");
    for (k, (m, n)) in decile_means(&sel).iter().enumerate() {
        let _ = writeln!(t, "    {k}: {} {n}", pct(*m));
    }
    let _ = write!(t, "  {}", boot_line(&format!("d_w over {} weeks, block {}", dw.len(), p.block_weeks), primary));
    for b in CHECK_BLOCKS.iter().filter(|b| **b != p.block_weeks) {
        let _ = write!(t, "  check {}", boot_line(&format!("block {b}"), block_bootstrap(&dw_series, *b, p.reps, p.seed, 0.0)));
    }
    let ph = phase_means(&dw);
    let _ = writeln!(t, "  13 phases (every 13th week): mean of the phase means {}; phases above 0: {} of {}", pct(mean(&ph)), ph.iter().filter(|x| **x > 0.0).count(), ph.len());
    match dw.iter().min_by(|a, b| a.1.total_cmp(&b.1)) {
        Some((d, x)) => {
            let _ = writeln!(t, "  worst 13-week d_w: {} on {}", pct(*x), mapcheck::dates::format(*d));
        }
        None => t.push_str("  worst 13-week d_w: none\n"),
    }
    let _ = writeln!(t, "  T2 {}", verdict(t2));

    // ---- T3 ----
    let diffs = base_rate_differences(rows, entries);
    let diff_series: Vec<(f64, f64)> = diffs.iter().map(|x| (x.1, 1.0)).collect();
    let t3a = block_bootstrap(&diff_series, p.block_weeks, p.reps, p.seed, 0.0);
    let t3a_pass = t3a.is_some_and(|b| b.stat > 0.0 && b.p < 0.05);
    let all: Vec<f64> = ent.iter().filter_map(|e| e.row.excess).collect();
    let _ = writeln!(t, "\nT3. THE ENTRIES OF THE RULE. Rule A: mean over the weeks of (hit rate of the entries less the share of all scored names with an excess above 0) > 0, one-sided p < 0.05.");
    let _ = writeln!(t, "  {}", dist_line("all entries", &all));
    let _ = writeln!(t, "  {}", dist_line("new tickers", &ent.iter().filter(|e| !e.reentry).filter_map(|e| e.row.excess).collect::<Vec<_>>()));
    let _ = writeln!(t, "  {}", dist_line("re-entries", &ent.iter().filter(|e| e.reentry).filter_map(|e| e.row.excess).collect::<Vec<_>>()));
    let _ = write!(t, "  {}", boot_line(&format!("hit rate less base rate over {} weeks", diffs.len()), t3a));
    let _ = writeln!(t, "  Rule A {}", verdict(t3a_pass));
    let from = mapcheck::dates::parse(TAIL_FROM).unwrap_or(0);
    let tw = tail_weeks(entries, from);
    let refs: Vec<&TailWeek> = tw.iter().collect();
    let (c10, c5, n_tail) = coverage(&refs);
    let s10: Vec<(f64, f64)> = tw.iter().map(|w| (w.below10 as f64, w.n as f64)).collect();
    let s5: Vec<(f64, f64)> = tw.iter().map(|w| (w.below5 as f64, w.n as f64)).collect();
    let t3b_pass = n_tail > 0 && (0.05..=0.15).contains(&c10) && (0.02..=0.08).contains(&c5);
    let _ = writeln!(t, "  Rule B (priced in): from {TAIL_FROM}, the stated P10 and P5 are those of all entries that closed before the week (expanding window, at least {MIN_TAIL}). The later entries below the stated P10 must be 5% to 15%, below the stated P5 2% to 8%.");
    let ival = |s: &[(f64, f64)]| share(s, p).map_or("no interval".to_string(), |b| format!("95% interval {:.1}% to {:.1}%", 100.0 * b.lo, 100.0 * b.hi));
    let _ = writeln!(t, "  {} entries in {} weeks: below the stated P10 {:.1}% ({}); below the stated P5 {:.1}% ({})", n_tail, tw.len(), 100.0 * c10, ival(&s10), 100.0 * c5, ival(&s5));
    let _ = writeln!(t, "  Rule B {}", verdict(t3b_pass));
    let t3 = t3a_pass && t3b_pass;
    let _ = writeln!(t, "  T3 {}", verdict(t3));

    // ---- T4 ----
    let _ = writeln!(t, "\nT4. AT ANY TIME. Rule: the mean entry excess and the mean d_w are above 0 in at least 7 of the calendar years, in both halves (ticker_half), and in at least 2 of the 3 periods 2018-2020, 2021-2023, 2024-2026.");
    let years: BTreeSet<i64> = sel.iter().map(|r| year_of(r.date)).chain(ent.iter().map(|e| year_of(e.row.date))).collect();
    let (mut ye, mut yd) = (0, 0);
    let _ = writeln!(t, "  by calendar year: entry mean (n), d_w mean (weeks)");
    for y in &years {
        let es: Vec<&EntryRow> = ent.iter().filter(|e| year_of(e.row.date) == *y).copied().collect();
        let ds: Vec<(i64, f64)> = dw.iter().filter(|x| year_of(x.0) == *y).copied().collect();
        let (em, en, dm, dn) = group_means(&es, &ds);
        ye += usize::from(em > 0.0);
        yd += usize::from(dm > 0.0);
        let _ = writeln!(t, "    {y}: {} ({en}), {} ({dn})", pct(em), pct(dm));
    }
    let _ = writeln!(t, "  years with entry mean above 0: {ye} of {}; years with d_w above 0: {yd} of {}", years.len(), years.len());
    let (mut he, mut hd) = (0, 0);
    for h in 0..2u64 {
        let es: Vec<&EntryRow> = ent.iter().filter(|e| ticker_half(&e.row.ticker) == h).copied().collect();
        let part: Vec<&RankRow> = sel.iter().filter(|r| ticker_half(&r.ticker) == h).copied().collect();
        let ds = weekly_spreads(&part);
        let (em, en, dm, dn) = group_means(&es, &ds);
        he += usize::from(em > 0.0);
        hd += usize::from(dm > 0.0);
        let _ = writeln!(t, "    half {h}: entry mean {} ({en}), d_w mean {} ({dn})", pct(em), pct(dm));
    }
    let (mut pe, mut pd) = (0, 0);
    for (a, b) in PERIODS {
        let es: Vec<&EntryRow> = ent.iter().filter(|e| (a..=b).contains(&year_of(e.row.date))).copied().collect();
        let ds: Vec<(i64, f64)> = dw.iter().filter(|x| (a..=b).contains(&year_of(x.0))).copied().collect();
        let (em, en, dm, dn) = group_means(&es, &ds);
        pe += usize::from(em > 0.0);
        pd += usize::from(dm > 0.0);
        let _ = writeln!(t, "    {a}-{b}: entry mean {} ({en}), d_w mean {} ({dn})", pct(em), pct(dm));
    }
    let t4 = ye >= 7 && yd >= 7 && he == 2 && hd == 2 && pe >= 2 && pd >= 2;
    let _ = writeln!(t, "  years {ye}/{yd} (need 7 and 7), halves {he}/{hd} (need 2 and 2), periods {pe}/{pd} (need 2 and 2)");
    let _ = writeln!(t, "  T4 {}", verdict(t4));
    if !windows.is_empty() {
        let _ = writeln!(t, "  Stress windows (descriptive only, no pass rule): d_w mean (weeks), entry mean (n), below stated P10 / P5 (entries)");
        for w in windows {
            let ds: Vec<f64> = dw.iter().filter(|x| x.0 >= w.start && x.0 <= w.end).map(|x| x.1).collect();
            let es: Vec<f64> = ent.iter().filter(|e| e.row.date >= w.start && e.row.date <= w.end).filter_map(|e| e.row.excess).collect();
            let tws: Vec<&TailWeek> = tw.iter().filter(|x| x.date >= w.start && x.date <= w.end).collect();
            let (a, b, n) = coverage(&tws);
            let _ = writeln!(
                t,
                "    {} {} to {}: d_w {} ({}), entries {} ({}), tail {:.1}% / {:.1}% ({n})",
                w.name,
                mapcheck::dates::format(w.start),
                mapcheck::dates::format(w.end),
                pct(mean(&ds)),
                ds.len(),
                pct(mean(&es)),
                es.len(),
                100.0 * a,
                100.0 * b
            );
        }
    }

    let _ = writeln!(t, "\nHEADLINE (T2 to T4; T1 is `fa dcf pit --cut 2015`): T2 {}, T3 {}, T4 {}: {}", verdict(t2), verdict(t3), verdict(t4), if t2 && t3 && t4 { "ALL PASS" } else { "NOT ALL PASS: the weekly trades do not start on this basis" });
    t
}

fn dist_line(label: &str, xs: &[f64]) -> String {
    let d = dist(xs);
    format!("{label}: n {}, hit rate {:.1}%, mean {}, P10 {}, P5 {}", d.n, 100.0 * d.hit, pct(d.mean), pct(d.p10), pct(d.p5))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> i64 {
        mapcheck::dates::parse(s).unwrap()
    }

    fn prices(items: Vec<(&str, Vec<(i64, f64)>)>) -> Prices {
        Prices(items.into_iter().map(|(t, s)| (t.to_string(), s)).collect())
    }

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    fn row(date: i64, ticker: &str, rank: f64, excess: Option<f64>) -> RankRow {
        RankRow { date, ticker: ticker.into(), sector: "S".into(), rank, ratio: 1.0, fwd: excess, excess }
    }

    fn entry(date: i64, ticker: &str, excess: Option<f64>, reentry: bool) -> EntryRow {
        EntryRow { row: row(date, ticker, 0.95, excess), reentry }
    }

    #[test]
    fn the_forward_return_is_from_the_close_of_the_day_to_the_last_close_on_or_before_day_plus_91() {
        let d0 = d("2020-01-31");
        // closes on the days 0, 1, 90, 92 only; the close of day 91 is missing: the last close on or before it is day 90
        let s = vec![(d0, 100.0), (d0 + 1, 101.0), (d0 + 90, 120.0), (d0 + 92, 150.0)];
        let p = prices(vec![("A", s)]);
        near(forward_return(&p, "A", d0, d0 + 200).unwrap(), 0.20);
        // no close on the day itself (d0 + 2): the close of the day before is the start (101); the end day is d0 + 93: close of day 92
        near(forward_return(&p, "A", d0 + 2, d0 + 200).unwrap(), 150.0 / 101.0 - 1.0);
        // the data end before the day + 91: no outcome
        assert_eq!(forward_return(&p, "A", d0, d0 + 90), None);
        near(forward_return(&p, "A", d0, d0 + 91).unwrap(), 0.20);
        // a close older than STALE_DAYS on the day: no outcome
        assert_eq!(forward_return(&p, "A", d0 + 20, d0 + 200), None);
        assert_eq!(forward_return(&p, "Z", d0, d0 + 200), None);
    }

    #[test]
    fn deciles_and_the_weekly_spread_have_fixed_answers() {
        assert_eq!([0.0, 0.0999, 0.1, 0.55, 0.8999, 0.9, 0.95, 1.0].map(decile), [0, 0, 1, 5, 8, 9, 9, 9]);
        let rows = [
            row(1, "A", 0.95, Some(0.10)),
            row(1, "B", 0.92, Some(0.20)), // top decile mean 0.15
            row(1, "C", 0.50, Some(9.0)),  // the middle is not in the spread
            row(1, "D", 0.05, Some(-0.05)),
            row(1, "E", 0.02, Some(0.05)), // bottom decile mean 0.00
            row(2, "A", 0.95, Some(0.30)),
            row(2, "D", 0.05, Some(0.10)), // spread 0.20
            row(3, "A", 0.95, None),       // no outcome: no spread on day 3
            row(3, "D", 0.05, Some(0.10)),
            row(4, "A", 0.95, Some(0.10)), // no bottom decile on day 4
        ];
        let r: Vec<&RankRow> = rows.iter().collect();
        let sp = weekly_spreads(&r);
        assert_eq!(sp.len(), 2);
        assert_eq!(sp[0].0, 1);
        near(sp[0].1, 0.15);
        near(sp[1].1, 0.20);
        let m = decile_means(&r);
        near(m[9].0, (0.10 + 0.20 + 0.30 + 0.10) / 4.0);
        assert_eq!((m[9].1, m[0].1, m[5].1, m[3].1), (4, 4, 1, 0));
        assert!(m[3].0.is_nan());
    }

    #[test]
    fn the_13_phases_take_every_13th_week() {
        let s: Vec<(i64, f64)> = (0..30).map(|i| (i, i as f64)).collect();
        let m = phase_means(&s);
        assert_eq!(m.len(), 13);
        near(m[0], (0.0 + 13.0 + 26.0) / 3.0); // weeks 0, 13, 26
        near(m[4], (4.0 + 17.0) / 2.0); // weeks 4 and 17 (the week 30 is not in the series)
        near(m[12], (12.0 + 25.0) / 2.0);
        assert_eq!(phase_means(&s[..5]).len(), 5); // a short series: the phases with no week are left out
        assert!(phase_means(&[]).is_empty());
    }

    #[test]
    fn percentiles_and_the_distribution_have_fixed_answers() {
        let xs = [-0.2, -0.1, 0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]; // 11 points
        let s = dist(&xs);
        assert_eq!(s.n, 11);
        near(s.hit, 8.0 / 11.0); // 0.0 is not above 0
        near(s.mean, 0.3);
        near(s.p10, -0.1); // h = 0.1 * 10 = 1
        near(s.p5, -0.15); // h = 0.5: halfway between -0.2 and -0.1
        assert!(dist(&[]).mean.is_nan());
    }

    #[test]
    fn the_bootstrap_is_deterministic_with_a_seed_and_the_seed_matters() {
        let series: Vec<(f64, f64)> = (0..60).map(|i| (((i * 37) % 11) as f64 - 4.0, 1.0)).collect();
        let a = block_bootstrap(&series, 4, 2000, 7, 0.0).unwrap();
        let b = block_bootstrap(&series, 4, 2000, 7, 0.0).unwrap();
        let c = block_bootstrap(&series, 4, 2000, 8, 0.0).unwrap();
        assert_eq!(a, b);
        assert_ne!(a.se, c.se);
    }

    #[test]
    fn the_bootstrap_is_right_on_known_cases() {
        // A constant series above the null: every resample equals the data, p = 0, se = 0, the interval is the point.
        let b = block_bootstrap(&[(2.0, 1.0); 10], 3, 500, 7, 0.0).unwrap();
        near(b.stat, 2.0);
        near(b.se, 0.0);
        near(b.p, 0.0);
        near(b.lo, 2.0);
        near(b.hi, 2.0);
        // The same series, null equal to the data: p = 1.
        near(block_bootstrap(&[(2.0, 1.0); 10], 3, 500, 7, 2.0).unwrap().p, 1.0);
        // +1, -1, +1, -1 with blocks of 2: each block sums to 0, so every resample has the mean 0 (the data mean).
        let alt = [(1.0, 1.0), (-1.0, 1.0), (1.0, 1.0), (-1.0, 1.0)];
        let b = block_bootstrap(&alt, 2, 500, 7, 0.0).unwrap();
        near(b.stat, 0.0);
        near(b.se, 0.0);
        // A block as long as the series gives a rotation: the mean does not move.
        let s: Vec<(f64, f64)> = (0..8).map(|i| (i as f64, 1.0)).collect();
        near(block_bootstrap(&s, 8, 300, 7, 0.0).unwrap().se, 0.0);
        // The standard error of a mean of 100 values, half 0 and half 1, with blocks of 1: sd 0.5 / sqrt(100) = 0.05.
        let iid: Vec<(f64, f64)> = (0..100).map(|i| ((i % 2) as f64, 1.0)).collect();
        let b = block_bootstrap(&iid, 1, 20000, 7, 0.5).unwrap();
        near(b.stat, 0.5);
        assert!((b.se - 0.05).abs() < 0.003, "{}", b.se);
        assert!((b.p - 0.5).abs() < 0.05, "{}", b.p); // the data are at the null: p near one half
        assert!((b.lo - (0.5 - 1.96 * 0.05)).abs() < 0.02 && (b.hi - (0.5 + 1.96 * 0.05)).abs() < 0.02, "{} {}", b.lo, b.hi);
        // A ratio statistic: 2 hits of 3 entries in week 1 and 0 of 1 in week 2 is 2 of 4.
        near(block_bootstrap(&[(2.0, 3.0), (0.0, 1.0)], 1, 100, 7, 0.5).unwrap().stat, 0.5);
        assert_eq!(block_bootstrap(&[], 1, 100, 7, 0.0), None);
        assert_eq!(block_bootstrap(&[(1.0, 0.0)], 1, 100, 7, 0.0), None);
    }

    #[test]
    fn the_base_rate_difference_is_the_hit_of_the_entries_less_the_share_of_all_scored_names() {
        // Week 1: 4 scored names with excess +, +, -, - (base 0.5); 2 entries: +, + (hit 1.0): difference +0.5.
        // Week 2: 2 scored names +, - (base 0.5); 1 entry: - (hit 0): difference -0.5. Week 3: an entry with no outcome: no week.
        let rows = [row(1, "A", 0.9, Some(0.1)), row(1, "B", 0.8, Some(0.2)), row(1, "C", 0.3, Some(-0.1)), row(1, "D", 0.2, Some(-0.2)), row(2, "A", 0.9, Some(0.1)), row(2, "B", 0.8, Some(-0.1)), row(3, "A", 0.9, None)];
        let ent = [entry(1, "A", Some(0.1), false), entry(1, "B", Some(0.2), false), entry(2, "B", Some(-0.1), false), entry(3, "A", None, false)];
        let r = base_rate_differences(&rows, &ent);
        assert_eq!(r.len(), 2);
        near(r[0].1, 0.5);
        near(r[1].1, -0.5);
    }

    #[test]
    fn the_tail_test_uses_only_entries_that_closed_before_the_week() {
        // 40 entries on day 0, with the excess 1..40 (in units of 0.01). They close on day 91: a week W sees them only if 0 + 91 < W.
        let mut ent: Vec<EntryRow> = (1..=40).map(|i| entry(0, &format!("T{i}"), Some(i as f64 / 100.0), false)).collect();
        // week 91: the entries of day 0 closed on day 91, not before it: no stated figure (nothing closed before day 91)
        // week 98: 40 closed entries. Three entries of that week: 0.045 (below the P10), 0.003 (below the P5 and the P10), 0.5.
        ent.extend([entry(98, "N1", Some(0.045), false), entry(98, "N2", Some(0.003), false), entry(98, "N3", Some(0.5), false)]);
        ent.push(entry(91, "N4", Some(-5.0), false)); // the week 91: no figure
        ent.push(entry(105, "N5", None, false)); // no outcome: not a week
        let tw = tail_weeks(&ent, 0);
        assert_eq!(tw.len(), 1);
        let w = &tw[0];
        assert_eq!((w.date, w.closed, w.n), (98, 40, 3));
        // sorted 0.01 .. 0.40: P10: h = 0.1 * 39 = 3.9: 0.04 + 0.9 * 0.01 = 0.049; P5: h = 1.95: 0.02 + 0.95 * 0.01 = 0.0295
        near(w.p10, 0.049);
        near(w.p5, 0.0295);
        assert_eq!((w.below10, w.below5), (2, 1));
        // a window with fewer than MIN_TAIL closed entries gives no week; `from` cuts the early weeks
        assert!(tail_weeks(&ent[..20], 0).is_empty());
        assert!(tail_weeks(&ent, 99).is_empty());
    }

    #[test]
    fn the_windows_file_is_parsed() {
        let w = parse_windows("# a comment\nname,start,end\n2020-03,2020-02-21,2021-03-16\n2026-11 (open),2026-11-04,2026-11-19\n").unwrap();
        assert_eq!(w.len(), 2);
        assert_eq!((w[0].name.as_str(), w[0].start, w[0].end), ("2020-03", d("2020-02-21"), d("2021-03-16")));
        assert!(parse_windows("x,2020-01-01,notadate").is_err());
    }

    /// 3 years of weekly data (2018 to 2020), 12 weeks in each, with a top decile (A) that beats the bottom (B) by `gap`.
    fn weekly_fixture(gap: f64) -> (Vec<RankRow>, Vec<EntryRow>) {
        let mut rows = Vec::new();
        let mut ent = Vec::new();
        for (yi, y) in [2018, 2019, 2020].iter().enumerate() {
            for w in 0..12 {
                let day = d(&format!("{y}-03-01")) + 7 * w;
                let g = gap + 0.01 * ((w % 3) as f64 - 1.0) + 0.001 * yi as f64;
                rows.push(row(day, "A", 0.95, Some(g / 2.0)));
                rows.push(row(day, "B", 0.05, Some(-g / 2.0)));
                ent.push(entry(day, "A", Some(g / 2.0), w % 2 == 1));
            }
        }
        (rows, ent)
    }

    #[test]
    fn the_summary_gives_each_test_its_rule_its_value_and_a_verdict() {
        let p = StatParams { block_weeks: 6, seed: 7, reps: 300 };
        let (rows, ent) = weekly_fixture(0.04);
        let win = [Window { name: "W".into(), start: d("2019-03-01"), end: d("2019-05-01") }];
        let t = summary(&rows, &ent, &win, &p);
        // d_w = gap + 0.01 * (w % 3 - 1) + 0.001 * year index: the mean over 36 weeks is 0.04 + 0.001
        assert!(t.contains("d_w over 36 weeks, block 6: mean +4.10%"), "{t}");
        assert!(t.contains("T2 PASS"), "{t}");
        assert!(t.contains("check block 13") && t.contains("check block 52"), "{t}");
        assert!(t.contains("13 phases"), "{t}");
        assert!(t.contains("worst 13-week d_w: +3.00% on 2018-03-01"), "{t}"); // the week with w % 3 == 0 gives gap - 0.01 = 0.03; the first is 2018-03-01
        // by year: each year has d_w mean gap + 0.001 * yi; entries are half of it (g / 2)
        assert!(t.contains("2018: +2.00% (12), +4.00% (12)"), "{t}");
        assert!(t.contains("2020: +2.10% (12), +4.20% (12)"), "{t}");
        // 3 years: fewer than 7: T4 fails by the rule even if every year is above 0
        assert!(t.contains("years 3/3 (need 7 and 7)"), "{t}");
        assert!(t.contains("T4 FAIL"), "{t}");
        // the entries have a hit rate of 100% and the base rate is 50%: difference +50%
        assert!(t.contains("hit rate less base rate over 36 weeks: mean +50.00%"), "{t}");
        assert!(t.contains("Rule A PASS"), "{t}");
        // fewer than 30 entries closed before any week from 2019-01: no stated figure, Rule B fails
        assert!(t.contains("0 entries in 0 weeks") && t.contains("Rule B FAIL"), "{t}");
        assert!(t.contains("W 2019-03-01 to 2019-05-01: d_w +4.10% (9), entries +2.05% (9), tail 0.0% / 0.0% (0)"), "{t}");
        assert!(t.contains("NOT ALL PASS"), "{t}");
        // a negative gap fails T2
        let (rows, ent) = weekly_fixture(-0.04);
        assert!(summary(&rows, &ent, &[], &p).contains("T2 FAIL"));
    }

    // ---- no look-ahead: the score does not read the outcome ----

    /// A score of the view only: the close before the date over the close 30 days before it (it reads closes before the day).
    fn momentum_score(v: &View, universe: &[String]) -> Vec<(String, f64)> {
        universe
            .iter()
            .filter_map(|t| {
                let (_, last) = v.last_close(t)?;
                let (_, old) = v.close_asof(t, v.date() - 30).ok().flatten()?;
                Some((t.clone(), last / old - 1.0))
            })
            .collect()
    }

    fn fixture(poison_from: i64) -> (Prices, Vec<String>, BTreeMap<String, String>) {
        let names: Vec<String> = (0..10).map(|k| format!("T{k}")).collect();
        let start = d("2020-01-01");
        let mut items = Vec::new();
        for (k, n) in names.iter().enumerate() {
            let s: Vec<(i64, f64)> = (0..300).map(|i| (start + i, if start + i >= poison_from { 1000.0 * (k as f64 + 1.0) } else { 50.0 * (1.0 + 0.002 * ((k as f64 + 1.0) * (i as f64)).sin() + 0.0005 * k as f64 * i as f64) })).collect();
            items.push((n.as_str(), s));
        }
        let mut p = prices(items.iter().map(|(n, s)| (*n, s.clone())).collect());
        p.0.insert("SPY".into(), (0..300).map(|i| (start + i, 100.0)).collect());
        let sectors = names.iter().enumerate().map(|(k, n)| (n.clone(), if k < 5 { "X".to_string() } else { "Y".to_string() })).collect();
        (p, names, sectors)
    }

    #[test]
    fn a_price_after_the_date_does_not_change_the_score_the_rank_or_the_entries_only_the_outcome() {
        let rdays: Vec<i64> = (0..6).map(|i| d("2020-05-01") + 7 * i).collect(); // all before the poison day
        let poison = d("2020-06-15");
        let last = d("2020-01-01") + 299;
        let (clean_p, names, sectors) = fixture(i64::MAX);
        let (bad_p, _, _) = fixture(poison);
        let (r1, e1) = collect(&clean_p, &names, &momentum_score, &sectors, &rdays, last);
        let (r2, e2) = collect(&bad_p, &names, &momentum_score, &sectors, &rdays, last);
        assert_eq!(r1.len(), r2.len());
        assert!(!r1.is_empty() && !e1.is_empty());
        let key = |r: &RankRow| (r.date, r.ticker.clone(), r.sector.clone(), r.rank.to_bits(), r.ratio.to_bits());
        for (a, b) in r1.iter().zip(&r2) {
            assert_eq!(key(a), key(b));
        }
        assert_eq!(e1.iter().map(|e| (e.row.date, e.row.ticker.clone(), e.reentry)).collect::<Vec<_>>(), e2.iter().map(|e| (e.row.date, e.row.ticker.clone(), e.reentry)).collect::<Vec<_>>());
        // control: the outcome does change (the poison is not dead code)
        assert!(r1.iter().zip(&r2).any(|(a, b)| a.fwd != b.fwd));
    }

    #[test]
    fn the_outcome_is_the_return_less_the_mean_of_the_scored_list_and_the_entries_are_marked() {
        // 6 tickers in one sector (5 or more scored: its own sector). Score = the ticker number: T5 is always the best.
        let start = d("2020-01-03"); // a Friday
        let names: Vec<String> = (0..6).map(|k| format!("T{k}")).collect();
        let mut items: Vec<(&str, Vec<(i64, f64)>)> = Vec::new();
        for (k, n) in names.iter().enumerate() {
            // the close is 100 up to the day 91 after the first rebalance day, and 100 * (1 + 0.1 k) from then on
            let s: Vec<(i64, f64)> = (0..400).map(|i| (start + i, if i >= 91 { 100.0 * (1.0 + 0.1 * k as f64) } else { 100.0 })).collect();
            items.push((n.as_str(), s));
        }
        let mut p = prices(items);
        p.0.insert("SPY".into(), (0..400).map(|i| (start + i, 100.0)).collect());
        let sectors: BTreeMap<String, String> = names.iter().map(|n| (n.clone(), "X".to_string())).collect();
        let score = |_: &View, u: &[String]| -> Vec<(String, f64)> { u.iter().map(|t| (t.clone(), t[1..].parse::<f64>().unwrap())).collect() };
        let rdays = [start, start + 7, start + 98]; // the day 98: its horizon day 189 is in the data
        let (rows, entries) = collect(&p, &names, &score, &sectors, &rdays, start + 399);
        // day 0: returns 0, 0.1, ..., 0.5; the mean 0.25; the excess of T5 is 0.25
        let t5 = rows.iter().find(|r| r.date == start && r.ticker == "T5").unwrap();
        near(t5.fwd.unwrap(), 0.5);
        near(t5.excess.unwrap(), 0.25);
        near(rows.iter().find(|r| r.date == start && r.ticker == "T0").unwrap().excess.unwrap(), -0.25);
        // day 98: all closes are flat between the day 98 and the day 189: the outcome is 0 for each
        near(rows.iter().find(|r| r.date == start + 98 && r.ticker == "T5").unwrap().excess.unwrap(), 0.0);
        // the best (top fifth of 6 is 1 place) is T5 on day 0; on day 7 T5 is held: no other ticker is in the top fifth
        assert_eq!(entries.len(), 2);
        assert_eq!((entries[0].row.date, entries[0].row.ticker.as_str(), entries[0].reentry), (start, "T5", false));
        // day 98 = 98 days after the entry: T5 was dropped (91 days or more) and chosen again: a re-entry
        assert_eq!((entries[1].row.date, entries[1].row.ticker.as_str(), entries[1].reentry), (start + 98, "T5", true));
        let csv = entries_csv(&entries);
        assert!(csv.starts_with("date,ticker,sector,rank,ratio,reentry,fwd_13w,excess_13w\n2020-01-03,T5,X,0.916667,5.000000,false,0.500000,0.250000\n"), "{csv}");
        assert!(ranks_csv(&rows).starts_with("date,ticker,sector,rank,ratio,fwd_13w,excess_13w\n"));
    }
}
