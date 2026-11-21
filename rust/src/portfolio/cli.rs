//! The command `fa portfolio backtest`.

use super::strategies::{load_vintages, EqualWeight, Momentum, Pick, ValueRank, WeeklyValueRank};
use super::{perf_line, Config, Prices, Rebalance, Report};
use crate::dcf::auto::{index_from_cache, DEFAULT_TABLE};
use crate::dcf::auto_cli::load_table;
use crate::dcf::auto_pit::parse_rules;
use crate::dcf::auto_returns::RetParams;
use mapcheck::registry::Registry;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa portfolio backtest --strategy NAME --start D [--end D] [OPTIONS]
  Backtest of a strategy as a portfolio: target weights on each rebalance date, daily value at the close, a cost on each
  traded value, SPY as the benchmark. The strategy sees the prices of the days before the rebalance date only; the
  engine trades at the close of the rebalance date. No network, no order. Price returns (no dividends).
  --strategy NAME    value-rank (automatic model value / price, equal weight in the top quintile)
                     value-rank-weekly (the rule of docs/assumptions/A-009: each week up to 3 new tickers, the highest
                       rank inside the sector, one for each sector, each in the top fifth of its sector, a hold of 13
                       weeks; the sector of each ticker is today's SIC code; use it with --rebalance weekly. The engine
                       trades each open position back to 1/39 at each rebalance (A-009 holds without a rebalance). The
                       data checks that need the mapping status and the share-count guard are not in the backtest.)
                     momentum (12-1 month price momentum, equal weight in the top quintile)
  --cache DIR        company facts and prices (default <repo>/data/raw). The command only reads it.
  --start D          first day, YYYY-MM-DD
  --end D            last day (default: the last SPY day)
  --rebalance R      monthly (default) or quarterly: the first trading day of the month or the quarter; weekly: the Friday
                     (the last trading day of the week if Friday is not one); the strategy sees the closes before that day
  --cost-bp N (10)   cost in basis points of each traded value (buy and sell)
  --tickers-file F   the universe: one ticker at the start of each line (default: each ticker of the cache with a price file)
  --paths N (300)    value-rank: paths of each model (100 or more)     --seed S (7): a whole number
  --table FILE       value-rank: parameter table (default analysis/priors/process_priors_2026-11-19.toml)
  --neutral sector   value-rank: the top quintile inside each Fama-French 12 sector (today's SIC code of the cache),
                     with the sector weights of the scored list; the stock choice without the sector tilt
  --pick top|bottom  with value-rank --neutral sector: the top fifth (default) or the bottom fifth of each sector, held
                     long (the test of the short side of A-009: does the bottom fifth return less than the equal weight)
  --out FILE        write the daily value of the portfolio, of SPY and of the equal-weight line as CSV
  --holdings FILE    write the target weights of each rebalance date as CSV (date,ticker,weight)
  Survivorship: the lists of tickers are today's lists, so the result is too good. The output has an equal-weight line of
  the whole list for the comparison.";

pub const LIMITS: &str = "\
LIMITS
  - Survivorship: the ticker list is today's list. A company that failed, was bought or left the index is not in it, and
    a company that joined the index after it grew (index addition) is in it for the whole test. The result is too good,
    for both strategies. Compare it with the equal-weight line of the list, not only with SPY. Do not use it as a forecast.
  - Price returns: the price files are adjusted for splits, not for dividends (997 of 999 files in the data store; the
    files of IBM and JNJ are Yahoo files adjusted for dividends). SPY is cap-weighted, has the same limit, and is bought
    with no cost.
  - Cost: only the basis points on each traded value. No tax, no short sale, no market impact, no limit on the size.
  - A ticker with no close for more than 10 days is not bought; a held ticker that stops trading is sold at its last close
    (the count is above): there is no failure loss.
  - value-rank: the SEC facts are cut by the filing date (a vintage for each rebalance date: a restatement filed later is
    not read). The template of the filer and the list of tickers with facts come from today's facts. The parameter table
    was estimated on all years of the cache: an in-sample parameter table, not out of sample.";

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn day_of(args: &[String], name: &str) -> Result<Option<i64>, String> {
    match opt(args, name) {
        None => Ok(None),
        Some(t) => mapcheck::dates::parse(t).map(Some).ok_or_else(|| format!("{name} '{t}' is not YYYY-MM-DD")),
    }
}

/// The tickers of a file: the first word of each line that does not start with `#`.
pub fn read_tickers(text: &str) -> Vec<String> {
    let mut v: Vec<String> = text.lines().filter(|l| !l.trim_start().starts_with('#')).filter_map(|l| l.split_whitespace().next()).map(str::to_uppercase).collect();
    v.sort();
    v.dedup();
    v
}

pub fn run(repo: &Path, args: &[String]) -> u8 {
    if args.first().map(String::as_str) != Some("backtest") || args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return u8::from(args.first().map(String::as_str) != Some("backtest") && !args.is_empty());
    }
    match backtest(repo, &args[1..]) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("fa: {e}");
            2
        }
    }
}

fn backtest(repo: &Path, args: &[String]) -> Result<(), String> {
    let known = ["--strategy", "--cache", "--start", "--end", "--rebalance", "--cost-bp", "--tickers-file", "--paths", "--seed", "--table", "--out", "--holdings", "--neutral", "--pick"];
    let mut i = 0;
    while i < args.len() {
        if !known.contains(&args[i].as_str()) {
            return Err(format!("unknown option '{}' (fa portfolio --help)", args[i]));
        }
        match args.get(i + 1) {
            Some(v) if !v.starts_with("--") => {}
            _ => return Err(format!("option '{}' needs a value", args[i])),
        }
        i += 2;
    }
    let strategy = opt(args, "--strategy").ok_or("--strategy is needed: value-rank or momentum")?;
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let rebalance = match opt(args, "--rebalance").unwrap_or("monthly") {
        "monthly" => Rebalance::Monthly,
        "quarterly" => Rebalance::Quarterly,
        "weekly" => Rebalance::Weekly,
        x => return Err(format!("--rebalance '{x}': monthly, quarterly or weekly")),
    };
    let cost_bp: f64 = opt(args, "--cost-bp").map_or(Ok(10.0), |v| v.parse().map_err(|_| format!("bad number for --cost-bp: '{v}'")))?;
    if !(0.0..=1000.0).contains(&cost_bp) {
        return Err("--cost-bp is 0 to 1000".into());
    }
    let paths: usize = opt(args, "--paths").map_or(Ok(300), |v| v.parse().map_err(|_| format!("--paths '{v}': a whole number")))?;
    if paths < 100 {
        return Err(format!("--paths {paths}: 100 or more"));
    }
    let seed: u64 = opt(args, "--seed").map_or(Ok(7), |v| v.parse().map_err(|_| format!("--seed '{v}': a whole number")))?;
    let start = day_of(args, "--start")?.ok_or("--start is needed")?;

    let t0 = std::time::Instant::now();
    let mut tickers = match opt(args, "--tickers-file") {
        Some(f) => read_tickers(&std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?),
        None => crate::dcf::panel::cache_tickers(&cache)?,
    };
    tickers.retain(|t| t != "SPY");
    let mut names = tickers.clone();
    names.push("SPY".into());
    let raw: BTreeMap<String, Vec<(String, f64)>> = names
        .par_iter()
        .filter_map(|t| {
            let p = crate::market::prices::read_cache(&crate::market::prices::cache_path(&cache, t)).ok()?;
            Some((t.clone(), p.into_iter().map(|x| (x.date, x.close)).collect()))
        })
        .collect();
    if !raw.contains_key("SPY") {
        return Err(format!("no price file of SPY in {}", cache.display()));
    }
    tickers.retain(|t| raw.contains_key(t));
    let prices = Prices::from_raw(&raw);
    let end = match day_of(args, "--end")? {
        Some(d) => d,
        None => prices.0["SPY"].last().map(|x| x.0).ok_or("SPY has no prices")?,
    };
    let cfg = Config { start, end, rebalance, cost_bp };
    let late = tickers.iter().filter(|t| prices.0.get(*t).and_then(|s| s.first()).is_some_and(|f| f.0 > start)).count();
    let mut n_scored = None;
    let mut split_note = String::new();

    let report = match strategy {
        "momentum" => super::run(&prices, &tickers, &Momentum, &cfg)?,
        "value-rank" | "value-rank-weekly" => {
            let weekly = strategy == "value-rank-weekly";
            let pick = match opt(args, "--pick") {
                None | Some("top") => Pick::Top,
                Some("bottom") => Pick::Bottom,
                Some(x) => return Err(format!("--pick '{x}': top or bottom")),
            };
            if pick == Pick::Bottom && (weekly || opt(args, "--neutral").is_none()) {
                return Err("--pick bottom needs --strategy value-rank with --neutral sector".into());
            }
            if weekly && opt(args, "--neutral").is_some() {
                return Err("value-rank-weekly ranks inside the sector by its own rule: no --neutral".into());
            }
            let table_file = opt(args, "--table").map(PathBuf::from).unwrap_or_else(|| repo.join(DEFAULT_TABLE));
            let (table, fade) = load_table(&table_file).map_err(|e| e.0)?;
            let registry = Registry::embedded()?;
            let rules = parse_rules("").map_err(|e| e.0)?;
            let curves = std::fs::read_to_string(cache.join("rates").join("gsw_feds200628.csv")).ok().and_then(|t| crate::forwards::parse_gsw(&t).ok());
            let index = index_from_cache(&cache);
            // the closes are split-adjusted to the last date of each price file: the share counts of each vintage go to the same basis (a missing file is an error)
            let splits = crate::dcf::splits::load_splits(&cache)?;
            split_note = crate::dcf::splits::check_report(&splits, &tickers);
            let years: BTreeMap<String, _> = tickers
                .par_iter()
                .filter_map(|t| {
                    let (tpl, y) = load_vintages(&registry, &cache, t, start, &splits)?;
                    (tpl == "standard").then(|| (t.clone(), y))
                })
                .collect();
            let universe: Vec<String> = tickers.iter().filter(|t| years.contains_key(*t)).cloned().collect();
            eprintln!("value-rank: {} of {} tickers have SEC facts with the standard template", universe.len(), tickers.len());
            n_scored = Some(universe.len());
            let params = RetParams { table: &table, fade: &fade, rules: &rules, curves: curves.as_ref(), index: None, lag: 100, paths, seed };
            let sectors = match opt(args, "--neutral").or(weekly.then_some("sector")) {
                None => None,
                Some("sector") => Some(
                    universe
                        .iter()
                        .map(|t| (t.clone(), mapcheck::sec::company_sic(&cache, t).map_or("Other", |(sic, _)| mapcheck::sec::ff12_sector(sic)).to_string()))
                        .collect::<BTreeMap<String, String>>(),
                ),
                Some(x) => return Err(format!("--neutral '{x}': sector")),
            };
            let s = ValueRank { years, series: &raw, index: index.as_deref(), params, scored: Default::default(), sectors, pick };
            if weekly {
                super::run(&prices, &universe, &WeeklyValueRank { base: s, open: Default::default() }, &cfg)?
            } else {
                super::run(&prices, &universe, &s, &cfg)?
            }
        }
        x => return Err(format!("unknown strategy '{x}': value-rank, value-rank-weekly or momentum")),
    };
    let ew = super::run(&prices, &tickers, &EqualWeight, &cfg)?;
    let info = Info { n_tickers: tickers.len(), n_scored, late, seconds: t0.elapsed().as_secs_f64() };
    print!("{split_note}{}", render(&report, &ew, &cfg, args, &info));
    if let Some(path) = opt(args, "--out") {
        // the equal-weight line of the list, on the dates of the strategy (empty where it has no value on that date)
        let ew_at: BTreeMap<i64, f64> = ew.dates.iter().copied().zip(ew.nav.iter().copied()).collect();
        let mut csv = String::from("date,portfolio,spy,equal_weight\n");
        for k in 0..report.dates.len() {
            let e = ew_at.get(&report.dates[k]).map_or(String::new(), |v| format!("{v:.6}"));
            let _ = writeln!(csv, "{},{:.6},{:.6},{e}", mapcheck::dates::format(report.dates[k]), report.nav[k], report.bench[k]);
        }
        std::fs::write(path, csv).map_err(|e| format!("{path}: {e}"))?;
    }
    if let Some(path) = opt(args, "--holdings") {
        let mut csv = String::from("date,ticker,weight\n");
        for (d, w) in &report.holdings {
            for (t, x) in w {
                let _ = writeln!(csv, "{},{t},{x:.6}", mapcheck::dates::format(*d));
            }
        }
        std::fs::write(path, csv).map_err(|e| format!("{path}: {e}"))?;
    }
    Ok(())
}

/// Counts for the header of the output.
pub struct Info {
    pub n_tickers: usize,
    /// value-rank: the tickers that have SEC facts with the standard template (the rest cannot be scored).
    pub n_scored: Option<usize>,
    /// Tickers whose first price is after the start day.
    pub late: usize,
    pub seconds: f64,
}

/// The minimum, the median (the upper one for an even count) and the maximum of a list of counts.
fn spread(v: &[usize]) -> Option<(usize, usize, usize)> {
    let mut s = v.to_vec();
    s.sort_unstable();
    Some((*s.first()?, s[s.len() / 2], *s.last()?))
}

pub fn render(r: &Report, ew: &Report, cfg: &Config, args: &[String], info: &Info) -> String {
    let day = |d: Option<&i64>| d.map_or(String::new(), |d| mapcheck::dates::format(*d));
    let mut t = String::new();
    let _ = writeln!(t, "PORTFOLIO BACKTEST: {}. Command: fa portfolio backtest {}", r.strategy, args.join(" "));
    let table = if r.strategy.starts_with("value-rank") { " The parameter table is in-sample (estimated on all years)." } else { "" };
    let _ = writeln!(
        t,
        "WARNING: the ticker list is today's list (survivorship, index addition): the result is too good. Price returns, no dividends.{table} A test, not a forecast."
    );
    let _ = writeln!(
        t,
        "{} tickers{}; {} of them have the first price after the start day; start asked {}, first position {}, last day {} ({:.1} years); {} rebalances ({:?}); cost {} bp on each traded value; {} target weights dropped (no fresh close); {} positions sold at a stale close; {:.1} s",
        info.n_tickers,
        info.n_scored.map_or(String::new(), |n| format!(" ({n} scored: SEC facts with the standard template)")),
        info.late,
        mapcheck::dates::format(cfg.start),
        day(r.dates.first()),
        day(r.dates.last()),
        r.years(),
        r.rebalances,
        cfg.rebalance,
        cfg.cost_bp,
        r.skipped,
        r.stale_sales,
        info.seconds
    );
    let _ = writeln!(t, "{}", perf_line(&r.strategy, &r.fund));
    let _ = writeln!(t, "{}", perf_line("SPY", &r.bench_perf));
    let _ = writeln!(t, "{}  (same costs, same dates rule; first position {})", perf_line("equal-weight", &ew.fund), day(ew.dates.first()));
    let _ = writeln!(
        t,
        "turnover (one way, sum of the rebalances) {:.2}, {:.2} a year; total cost {:.2}% of the start value ({:.2}% a year)",
        r.turnover_total,
        r.turnover_total / r.years(),
        100.0 * r.cost_total,
        100.0 * r.cost_total / r.years()
    );
    let _ = writeln!(t, "days with no position (cash): {} of {} days ({:.0}%)", r.cash_days, r.dates.len(), 100.0 * r.cash_days as f64 / r.dates.len().max(1) as f64);
    if let Some((lo, mid, hi)) = spread(&r.scored) {
        let _ = writeln!(t, "tickers scored at the rebalances: min {lo}, median {mid}, max {hi} ({} rebalances)", r.scored.len());
    }
    let _ = writeln!(
        t,
        "excess annual return against SPY: {:+.2}%; against the equal-weight line of the list: {:+.2}% (first position: {} for the strategy, {} for the equal-weight run; the two runs cover different spans if these dates differ)",
        100.0 * (r.fund.annual - r.bench_perf.annual),
        100.0 * (r.fund.annual - ew.fund.annual),
        day(r.dates.first()),
        day(ew.dates.first())
    );
    t.push('\n');
    t.push_str(LIMITS);
    t.push('\n');
    t
}

#[cfg(test)]
mod tests {
    use super::spread;

    #[test]
    fn the_spread_of_the_scored_counts_is_min_median_max() {
        assert_eq!(spread(&[]), None);
        assert_eq!(spread(&[57, 93, 80]), Some((57, 80, 93)));
        assert_eq!(spread(&[0, 19]), Some((0, 19, 19)));
    }
}
