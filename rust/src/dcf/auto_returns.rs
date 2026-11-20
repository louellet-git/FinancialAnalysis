//! `fa dcf returns`: the value of the automatic model against the later stock returns (ticket
//! `T-20261120-dcf-investigation`, step 3). A measurement: the price is used to rank, not to make the model.
//!
//! For each company of the standard template with a price file in the cache, and for each fiscal year `T`, the model is
//! made from the fiscal years up to `T` only (as `fa dcf pit`), with the balance sheet of `T`, on the date
//! `D = end of T + lag` (default 100 days: a 10-K is due 60 to 90 days after the end of the year, so its numbers are
//! public on `D`). The curve, the beta and the price are those of `D` (the first trading day on or after `D`). The
//! later returns are the price changes from `D` to one and three years later.
//!
//! Limits (printed with the result): the price files are adjusted for splits, not for dividends (an estimate of the
//! dividend yield from `dividends_paid` is shown beside); the mapping takes the latest filing of each year (a
//! restatement enters); the companies are those of the cache now (survivorship).

use super::auto::*;
use super::auto_cli::load_table;
use super::auto_pit::{load_company_years, parse_rules};
use super::calibrate::{FadeCell, ProcessTable};
use super::panel::{bucket, BUCKET_LABELS};
use super::paths::{Mods, PathEngine};
use super::process::PathsSpec;
use super::DcfError;
use mapcheck::registry::Registry;
use rayon::prelude::*;
use std::fmt::Write;
use std::path::{Path, PathBuf};

type R<T> = Result<T, DcfError>;

/// A year with fewer observations is not a cluster of the mean over the years (its rank correlation is too noisy).
pub const MIN_YEAR_OBS: usize = 20;

pub const USAGE: &str = "\
fa dcf returns [OPTIONS]
  The value of the automatic model (P50 / price) against the later stock returns. For each company of the standard
  template with a price file, and each fiscal year T: the model from the data up to T only, on the date D = end of T +
  lag, with the curve, the beta and the price of D; then the price return from D to D + 1 year and D + 3 years.
  Output: the rank correlation of P50 / price with the later return (each year, pooled, and the mean over the years with
  its standard error: the years are the clusters), the mean return by quintile of P50 / price (quintiles inside each
  year), and the same for each size bucket. No network.
  --cache DIR      company facts and prices (default <repo>/data/raw)
  --table FILE     parameter table (default analysis/priors/process_priors_2026-11-19.toml)
  --rules K=V,..   rules of the builder (fa dcf auto --help)
  --lag DAYS (100) days from the end of the fiscal year to the date of the model
  --paths N (1000) paths for each model        --seed S (7)
  --out FILE       write one CSV line for each observation
  --limit N        the first N companies only (a quick check)";

/// One company and fiscal year.
#[derive(Debug, Clone, PartialEq)]
pub struct RetObs {
    pub ticker: String,
    pub fy_end: String,
    pub as_of: String,
    pub year: i32,
    pub size: usize,
    pub p50: f64,
    pub price: f64,
    /// Price return to one and three years later (`None`: no price then).
    pub r1: Option<f64>,
    pub r3: Option<f64>,
    /// Dividends of the fiscal year over the market value on the date of the model (an estimate of the yield).
    pub div_yield: Option<f64>,
}

impl RetObs {
    pub fn ratio(&self) -> f64 {
        self.p50 / self.price
    }
}

/// `(day, close)` of a price series, in date order.
pub fn days_of(series: &[(String, f64)]) -> Vec<(i64, f64)> {
    series.iter().filter_map(|(d, c)| Some((mapcheck::dates::parse(d.get(..10)?)?, *c))).filter(|(_, c)| c.is_finite() && *c > 0.0).collect()
}

/// The first close on or after `day`, at most `max_gap` days later.
pub fn close_on_or_after(prices: &[(i64, f64)], day: i64, max_gap: i64) -> Option<(i64, f64)> {
    let i = prices.partition_point(|(d, _)| *d < day);
    prices.get(i).filter(|(d, _)| *d - day <= max_gap).copied()
}

/// What the observations of one company need besides its data.
pub struct RetParams<'a> {
    pub table: &'a ProcessTable,
    pub fade: &'a [FadeCell],
    pub rules: &'a Rules,
    pub curves: Option<&'a crate::forwards::Dataset>,
    pub index: Option<&'a [(String, f64)]>,
    pub lag: i64,
    pub paths: usize,
    pub seed: u64,
}

/// The automatic model of one company on `as_of`: `years` is the data known on that date (the last item is the base
/// year), `price` is the close of `as_of`, `prices` is the price series (only the part up to `as_of` is read: the
/// beta uses the weeks up to `as_of`). Returns the P50 and the base revenue, `None` if the model refuses.
/// `fa dcf returns` and `fa portfolio backtest` (strategy `value-rank`) use this one function.
pub fn model_p50(ticker: &str, years: &[YearData], as_of: &str, price: f64, prices: &[(String, f64)], p: &RetParams) -> Option<(f64, f64)> {
    let last = years.last()?;
    let inputs = AutoInputs {
        ticker: ticker.to_string(),
        template: "standard".into(),
        status: "PASS".into(),
        years: years.to_vec(),
        balance: Some(BalanceData::from_year(last)),
        ..Default::default()
    };
    let curve = p.curves.and_then(|ds| curve_on(ds, as_of).ok());
    let beta = p.index.and_then(|m| regression_beta(prices, m, as_of, 260, p.rules.beta_weight, "SPY"));
    let market = Market { as_of: as_of.to_string(), curve, price: Some(price), price_source: "price cache".into(), beta };
    let model = build(&inputs, p.table, p.fade, &market, p.rules, BuildOptions::default()).ok()?;
    let spec = PathsSpec::from_toml(&model.toml).ok()?;
    let eng = PathEngine::new(&spec).ok()?;
    let sim = eng.simulate_serial(p.paths, p.seed, &Mods::default()).ok()?;
    Some((sim.p50(), model.summary.base_revenue))
}

/// The observations of one company: one for each fiscal year with a model and a price on the date of the model.
pub fn company_returns(ticker: &str, years: &[YearData], prices: &[(String, f64)], p: &RetParams) -> Vec<RetObs> {
    let days = days_of(prices);
    let mut out = Vec::new();
    for b in 0..years.len() {
        let Some(end) = mapcheck::dates::parse(years[b].end.get(..10).unwrap_or("")) else { continue };
        let Some((d0, p0)) = close_on_or_after(&days, end + p.lag, 7) else { continue };
        let as_of = mapcheck::dates::format(d0);
        let Some((p50, base_revenue)) = model_p50(ticker, &years[..=b], &as_of, p0, prices, p) else { continue };
        let later = |years_after: i64| close_on_or_after(&days, d0 + 365 * years_after + years_after / 4, 7).map(|(_, c)| c / p0 - 1.0);
        let y = &years[b];
        let div_yield = match (y.dividends, y.shares) {
            (Some(dv), Some(sh)) if sh > 0.0 => Some(dv / (sh * p0)),
            (None, Some(_)) => Some(0.0),
            _ => None,
        };
        out.push(RetObs {
            ticker: ticker.to_string(),
            fy_end: y.end.clone(),
            as_of: as_of.clone(),
            year: as_of.get(..4).and_then(|x| x.parse().ok()).unwrap_or(0),
            size: bucket(base_revenue),
            p50,
            price: p0,
            r1: later(1),
            r3: later(3),
            div_yield,
        });
    }
    out
}

/// Ranks from 1 (ties get the mean rank).
pub fn ranks(x: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..x.len()).collect();
    idx.sort_by(|a, b| x[*a].total_cmp(&x[*b]));
    let mut r = vec![0.0; x.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && x[idx[j + 1]] == x[idx[i]] {
            j += 1;
        }
        let rank = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j {
            r[idx[k]] = rank;
        }
        i = j + 1;
    }
    r
}

/// The Spearman rank correlation (`None` for fewer than 5 pairs).
pub fn spearman(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() || a.len() < 5 {
        return None;
    }
    let (ra, rb) = (ranks(a), ranks(b));
    let n = a.len() as f64;
    let (ma, mb) = (ra.iter().sum::<f64>() / n, rb.iter().sum::<f64>() / n);
    let cov: f64 = ra.iter().zip(&rb).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let va: f64 = ra.iter().map(|x| (x - ma).powi(2)).sum();
    let vb: f64 = rb.iter().map(|y| (y - mb).powi(2)).sum();
    (va > 0.0 && vb > 0.0).then(|| cov / (va * vb).sqrt())
}

/// The mean and its standard error (the standard deviation over `sqrt(n)`).
pub fn mean_se(x: &[f64]) -> (f64, f64) {
    let n = x.len() as f64;
    if x.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    let m = x.iter().sum::<f64>() / n;
    if x.len() < 2 {
        return (m, f64::NAN);
    }
    let v = x.iter().map(|y| (y - m).powi(2)).sum::<f64>() / (n - 1.0);
    (m, (v / n).sqrt())
}

/// The return of an observation at the horizon (1 or 3 years), with the dividend estimate when `with_div`.
fn ret(o: &RetObs, h: usize, with_div: bool) -> Option<f64> {
    let r = if h == 1 { o.r1? } else { o.r3? };
    Some(if with_div { r + h as f64 * o.div_yield.unwrap_or(0.0) } else { r })
}

/// The quintile (0 to 4) of each observation by P50 / price, inside its year.
pub fn quintiles_in_year(obs: &[&RetObs]) -> Vec<usize> {
    let mut q = vec![0; obs.len()];
    let mut years: Vec<i32> = obs.iter().map(|o| o.year).collect();
    years.sort();
    years.dedup();
    for y in years {
        let idx: Vec<usize> = (0..obs.len()).filter(|i| obs[*i].year == y).collect();
        let r = ranks(&idx.iter().map(|i| obs[*i].ratio()).collect::<Vec<_>>());
        let n = idx.len() as f64;
        for (k, i) in idx.iter().enumerate() {
            q[*i] = (((r[k] - 0.5) / n * 5.0).floor() as usize).min(4);
        }
    }
    q
}

/// The text of the result for one horizon and one set of observations.
pub fn render_horizon(obs: &[RetObs], h: usize, with_div: bool, keep: &dyn Fn(&RetObs) -> bool) -> String {
    let set: Vec<&RetObs> = obs.iter().filter(|o| keep(o) && ret(o, h, with_div).is_some()).collect();
    let mut t = String::new();
    if set.len() < 20 {
        let _ = writeln!(t, "    {} observations: too few", set.len());
        return t;
    }
    let mut years: Vec<i32> = set.iter().map(|o| o.year).collect();
    years.sort();
    years.dedup();
    years.retain(|y| set.iter().filter(|o| o.year == *y).count() >= MIN_YEAR_OBS);
    let pooled = spearman(&set.iter().map(|o| o.ratio()).collect::<Vec<_>>(), &set.iter().map(|o| ret(o, h, with_div).unwrap_or(0.0)).collect::<Vec<_>>());
    let mut by_year = Vec::new();
    let mut line = String::new();
    for y in &years {
        let s: Vec<&&RetObs> = set.iter().filter(|o| o.year == *y).collect();
        if let Some(c) = spearman(&s.iter().map(|o| o.ratio()).collect::<Vec<_>>(), &s.iter().map(|o| ret(o, h, with_div).unwrap_or(0.0)).collect::<Vec<_>>()) {
            by_year.push(c);
            let _ = write!(line, " {y}: {c:+.3} (n {})", s.len());
        }
    }
    let (m, se) = mean_se(&by_year);
    let _ = writeln!(t, "    rank correlation of P50/price with the {h}-year return: pooled {:+.3} (n {}); mean over {} years (each with {MIN_YEAR_OBS} or more) {m:+.3}, standard error {se:.3} (t {:.1})", pooled.unwrap_or(f64::NAN), set.len(), by_year.len(), m / se);
    let _ = writeln!(t, "    each year:{line}");
    // quintiles inside each year; the return less the mean of its year (the market level of the year goes out)
    let q = quintiles_in_year(&set);
    let mut sums = [[0.0f64; 3]; 5];
    for (o, k) in set.iter().zip(&q) {
        let y_mean = {
            let s: Vec<f64> = set.iter().filter(|x| x.year == o.year).filter_map(|x| ret(x, h, with_div)).collect();
            s.iter().sum::<f64>() / s.len() as f64
        };
        let r = ret(o, h, with_div).unwrap_or(0.0);
        sums[*k][0] += r;
        sums[*k][1] += r - y_mean;
        sums[*k][2] += 1.0;
    }
    let mut row = String::from("    quintile of P50/price (1 = lowest): mean return | less the mean of the year:");
    for (k, s) in sums.iter().enumerate() {
        let _ = write!(row, "  Q{} {:+.1}% | {:+.1}% (n {})", k + 1, 100.0 * s[0] / s[2].max(1.0), 100.0 * s[1] / s[2].max(1.0), s[2]);
    }
    let _ = writeln!(t, "{row}");
    // the spread Q5 - Q1 of each year, with the years as clusters
    let mut spreads = Vec::new();
    for y in &years {
        let (mut a, mut na, mut b, mut nb) = (0.0, 0.0, 0.0, 0.0);
        for (o, k) in set.iter().zip(&q) {
            if o.year != *y {
                continue;
            }
            let r = ret(o, h, with_div).unwrap_or(0.0);
            if *k == 4 {
                a += r;
                na += 1.0;
            } else if *k == 0 {
                b += r;
                nb += 1.0;
            }
        }
        if na > 0.0 && nb > 0.0 {
            spreads.push(a / na - b / nb);
        }
    }
    let (m, se) = mean_se(&spreads);
    let _ = writeln!(t, "    Q5 - Q1 return: mean over {} years {:+.1}%, standard error {:.1}% (t {:.1})", spreads.len(), 100.0 * m, 100.0 * se, m / se);
    t
}

pub const LIMITS: &str = "\
LIMITS
  - The price files are adjusted for splits, not for dividends: the returns are price returns. The lines \"with dividends\" add
    the dividend yield of the fiscal year (dividends_paid / market value on the date of the model) for each year of the horizon.
  - Survivorship: the companies are those of the cache now (mostly the S&P 500 of 2026). A company that failed or was bought
    is not in the test. The firms that fell out of the index had low later returns; if the model ranked them high (cheap), the
    test without them shows a better ranking than the real one; if it ranked them low, a worse one.
  - The mapping takes the latest filing of each year: a restatement after T enters the data of T.
  - The parameter table was estimated on all years (also the years after T).
  - The 3-year returns of the years in sequence overlap: the observations of near years are not independent.";

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

/// `fa dcf returns`. Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> u8 {
    match command_inner(repo, args) {
        Ok(()) => 0,
        Err(x) => {
            eprintln!("fa: {x}");
            2
        }
    }
}

fn command_inner(repo: &Path, args: &[String]) -> R<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    let known = ["--cache", "--table", "--rules", "--lag", "--paths", "--seed", "--out", "--limit"];
    let mut i = 0;
    while i < args.len() {
        if !known.contains(&args[i].as_str()) {
            return Err(DcfError(format!("unknown option '{}'", args[i])));
        }
        i += 2;
    }
    let num = |k: &str, d: i64| -> R<i64> { opt(args, k).map_or(Ok(d), |v| v.parse().map_err(|_| DcfError(format!("bad {k} '{v}'")))) };
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let table_file = opt(args, "--table").map(PathBuf::from).unwrap_or_else(|| repo.join(DEFAULT_TABLE));
    let rules = parse_rules(opt(args, "--rules").unwrap_or(""))?;
    let (lag, paths, seed, limit) = (num("--lag", 100)?, num("--paths", 1000)?.max(100) as usize, num("--seed", 7)? as u64, num("--limit", 0)?);
    let (table, fade) = load_table(&table_file)?;
    let registry = Registry::embedded().map_err(DcfError)?;
    let mut tickers = super::panel::cache_tickers(&cache).map_err(DcfError)?;
    tickers.retain(|t| crate::market::prices::cache_path(&cache, t).exists() && t != "SPY");
    if limit > 0 {
        tickers.truncate(limit as usize);
    }
    let curves = std::fs::read_to_string(cache.join("rates").join("gsw_feds200628.csv")).ok().and_then(|t| crate::forwards::parse_gsw(&t).ok());
    let index = index_from_cache(&cache);
    let p = RetParams { table: &table, fade: &fade, rules: &rules, curves: curves.as_ref(), index: index.as_deref(), lag, paths, seed };
    let t0 = std::time::Instant::now();
    let mut obs: Vec<RetObs> = tickers
        .par_iter()
        .flat_map(|t| {
            let Some((tpl, years)) = load_company_years(&registry, &cache, t) else { return Vec::new() };
            if tpl != "standard" {
                return Vec::new();
            }
            let Ok(pr) = crate::market::prices::read_cache(&crate::market::prices::cache_path(&cache, t)) else { return Vec::new() };
            let series: Vec<(String, f64)> = pr.into_iter().map(|x| (x.date, x.close)).collect();
            company_returns(t, &years, &series, &p)
        })
        .collect();
    obs.sort_by(|a, b| (a.ticker.as_str(), a.as_of.as_str()).cmp(&(b.ticker.as_str(), b.as_of.as_str())));
    let companies = {
        let mut v: Vec<&str> = obs.iter().map(|o| o.ticker.as_str()).collect();
        v.dedup();
        v.len()
    };
    let mut text = format!(
        "THE VALUE AGAINST THE LATER RETURNS (automatic model, point in time). Command: fa dcf returns {}\nRules: {:?}\n{} observations of {companies} companies (standard template, with a price file); lag {lag} days; {paths} paths; {:.1} s. curve: {}; index: {}.\n",
        args.join(" "),
        rules,
        obs.len(),
        t0.elapsed().as_secs_f64(),
        if curves.is_some() { "GSW" } else { "none (fixed rate)" },
        if index.is_some() { "SPY" } else { "none (beta 1)" }
    );
    let ratio: Vec<f64> = obs.iter().map(|o| o.ratio()).collect();
    let mut s = ratio.clone();
    s.sort_by(|a, b| a.total_cmp(b));
    if !s.is_empty() {
        let _ = writeln!(text, "P50 / price over all observations: median {:.3}; share above 1: {:.1}%; share at or below 0: {:.1}%", s[s.len() / 2], 100.0 * s.iter().filter(|x| **x > 1.0).count() as f64 / s.len() as f64, 100.0 * s.iter().filter(|x| **x <= 0.0).count() as f64 / s.len() as f64);
    }
    for h in [1usize, 3] {
        for with_div in [false, true] {
            let _ = writeln!(text, "\n{h}-YEAR RETURN{}", if with_div { ", WITH DIVIDENDS (estimate)" } else { " (price only)" });
            let _ = writeln!(text, "  all sizes");
            text.push_str(&render_horizon(&obs, h, with_div, &|_| true));
            if !with_div {
                for (k, label) in BUCKET_LABELS.iter().enumerate().take(5) {
                    let _ = writeln!(text, "  size {label}");
                    text.push_str(&render_horizon(&obs, h, with_div, &|o: &RetObs| o.size == k));
                }
            }
        }
    }
    text.push('\n');
    text.push_str(LIMITS);
    text.push('\n');
    print!("{text}");
    if let Some(path) = opt(args, "--out") {
        let mut csv = String::from("ticker,fy_end,as_of,size,p50,price,p50_over_price,r1,r3,div_yield\n");
        for o in &obs {
            let f = |x: Option<f64>| x.map_or(String::new(), |v| format!("{v:.4}"));
            let _ = writeln!(csv, "{},{},{},{},{:.2},{:.2},{:.4},{},{},{}", o.ticker, o.fy_end, o.as_of, BUCKET_LABELS[o.size], o.p50, o.price, o.ratio(), f(o.r1), f(o.r3), f(o.div_yield));
        }
        std::fs::write(path, csv).map_err(|x| DcfError(format!("{path}: {x}")))?;
    }
    Ok(())
}
