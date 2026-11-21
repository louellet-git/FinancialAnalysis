//! The command `fa portfolio backtest`.

use super::harness::{self, Hypothesis};
use super::strategies::{load_vintages, load_vintages_with, EqualWeight, Momentum, ValueRank};
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
                     momentum (12-1 month price momentum, equal weight in the top quintile)
  --cache DIR        company facts and prices (default <repo>/data/raw). The command only reads it.
  --start D          first day, YYYY-MM-DD
  --end D            last day (default: the last SPY day)
  --rebalance R      monthly (default) or quarterly: the first trading day of the month or the quarter
  --cost-bp N (10)   cost in basis points of each traded value (buy and sell)
  --tickers-file F   the universe: one ticker at the start of each line (default: each ticker of the cache with a price file)
  --paths N (300)    value-rank: paths of each model (100 or more)     --seed S (7): a whole number
  --table FILE       value-rank: parameter table (default analysis/priors/process_priors_2026-11-19.toml)
  --out FILE         write the daily value of the portfolio and of SPY as CSV
  --hypothesis FILE  a registered hypothesis (trading/hypotheses/<id>.toml, committed): the file gives strategy, params and tickers
                     file (a value of the command that differs from the file is an error); the run writes the add-only log
                     trading/runs/<id>/<UTC time>.json and prints the variant counts. No file: an exploratory run, no log.
  --trades FILE      write the trades as CSV: date, ticker, action, weight before and after, price, signal, filing accession
  --poison-check D   run again with each fact filed after D and each close after D multiplied by a factor that differs by
                     ticker; fail if a target weight or a trade before D differs (D is YYYY-MM-DD)
  Each run prints the result at 0x, 1x and 2x the cost, and the flag oos-read (the end is after oos_start of the hypothesis).
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

fn backtest(repo: &Path, args_in: &[String]) -> Result<(), String> {
    let hyp: Option<(Hypothesis, harness::FileProof)> = match opt(args_in, "--hypothesis") {
        Some(f) => {
            let file = PathBuf::from(f);
            let proof = harness::prove_committed(repo, &file)?;
            let h = Hypothesis::parse(&std::fs::read_to_string(&file).map_err(|e| format!("{f}: {e}"))?)?;
            let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            if stem != h.id {
                return Err(format!("the id '{}' of the hypothesis is not the file name '{stem}'", h.id));
            }
            Some((h, proof))
        }
        None => None,
    };
    let merged: Vec<String> = match &hyp {
        Some((h, _)) => h.merge_args(args_in, repo)?,
        None => args_in.to_vec(),
    };
    let args = &merged[..];
    let strategy = opt(args, "--strategy").ok_or("--strategy is needed: value-rank or momentum")?;
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let rebalance = match opt(args, "--rebalance").unwrap_or("monthly") {
        "monthly" => Rebalance::Monthly,
        "quarterly" => Rebalance::Quarterly,
        x => return Err(format!("--rebalance '{x}': monthly or quarterly")),
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
    let poison_day = day_of(args, "--poison-check")?;

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

    // What a value-rank run needs besides the prices (loaded once; the poison run uses the same).
    let vr = if strategy == "value-rank" {
        let table_file = opt(args, "--table").map(PathBuf::from).unwrap_or_else(|| repo.join(DEFAULT_TABLE));
        let (table, fade) = load_table(&table_file).map_err(|e| e.0)?;
        let registry = Registry::embedded()?;
        let rules = parse_rules("").map_err(|e| e.0)?;
        let curves = std::fs::read_to_string(cache.join("rates").join("gsw_feds200628.csv")).ok().and_then(|t| crate::forwards::parse_gsw(&t).ok());
        Some((table, fade, registry, rules, curves))
    } else {
        None
    };
    let index = if vr.is_some() { index_from_cache(&cache) } else { None };
    // One run of the strategy on the given closes. `poison`: the day after which each fact is multiplied by a factor of its ticker.
    let run_with = |raw: &BTreeMap<String, Vec<(String, f64)>>, prices: &Prices, index: Option<&[(String, f64)]>, poison: Option<&str>, cfg: &Config| -> Result<(Report, Option<usize>, Vec<String>), String> {
        match (strategy, &vr) {
            ("momentum", _) => Ok((super::run(prices, &tickers, &Momentum, cfg)?, None, tickers.clone())),
            ("value-rank", Some((table, fade, registry, rules, curves))) => {
                let years: BTreeMap<String, _> = tickers
                    .par_iter()
                    .filter_map(|t| {
                        let (tpl, y) = match poison {
                            None => load_vintages(registry, &cache, t, start)?,
                            Some(day) => load_vintages_with(registry, &cache, t, start, |f| harness::poison_facts(f, day, harness::fact_factor(t)))?,
                        };
                        (tpl == "standard").then(|| (t.clone(), y))
                    })
                    .collect();
                let universe: Vec<String> = tickers.iter().filter(|t| years.contains_key(*t)).cloned().collect();
                if poison.is_none() {
                    eprintln!("value-rank: {} of {} tickers have SEC facts with the standard template", universe.len(), tickers.len());
                }
                let params = RetParams { table, fade, rules, curves: curves.as_ref(), index: None, lag: 100, paths, seed };
                let s = ValueRank { years, series: raw, index, params, scored: Default::default(), signals: Default::default() };
                let n = universe.len();
                Ok((super::run(prices, &universe, &s, cfg)?, Some(n), universe))
            }
            (x, _) => Err(format!("unknown strategy '{x}': value-rank or momentum")),
        }
    };
    let (report, n_scored, universe) = run_with(&raw, &prices, index.as_deref(), None, &cfg)?;
    let ew = super::run(&prices, &tickers, &EqualWeight, &cfg)?;
    let info = Info { n_tickers: tickers.len(), n_scored, late, seconds: t0.elapsed().as_secs_f64() };
    print!("{}", render(&report, &ew, &cfg, args_in, &info));

    // The harness: the cost levels, the labels, the out-of-sample flag, the poison check, the run log.
    let (zero, two) = harness::cost_levels(&prices, &universe, &report, &cfg)?;
    let mut h = String::new();
    let _ = writeln!(h, "\nRUN HARNESS");
    let _ = writeln!(h, "{}; {}", harness::UNIVERSE_LABEL, harness::PRICE_LABEL);
    let _ = writeln!(h, "costs (the same strategy, the same target weights and dates; cost {} bp is 1x):", cost_bp);
    for (name, r) in [("cost 0x", &zero), ("cost 1x", &report), ("cost 2x", &two)] {
        let _ = writeln!(h, "{}", perf_line(name, &r.fund));
    }
    let oos = hyp.as_ref().map(|(hy, _)| harness::oos_read(end, hy.oos_day()));
    match (&hyp, oos) {
        (Some((hy, _)), Some(read)) => {
            let _ = writeln!(h, "hypothesis {} (registered {}), out-of-sample window from {}", hy.id, hy.date, hy.oos_start);
            let _ = writeln!(h, "oos-read: {read}");
            if read {
                let _ = writeln!(h, "WARNING: the run ends after oos_start ({}): it reads the out-of-sample window. Do not tune on this result.", hy.oos_start);
            }
        }
        _ => {
            let _ = writeln!(h, "exploratory run: no hypothesis file, not usable for a decision");
        }
    }
    let mut poison_text = None;
    let mut poison_error = None;
    if let (Some(pd), Some(day)) = (poison_day, opt(args, "--poison-check")) {
        let poisoned_raw: BTreeMap<String, Vec<(String, f64)>> = raw.iter().map(|(t, s)| (t.clone(), harness::poison_series(s, day, t))).collect();
        let poisoned_index = index.as_ref().map(|s| harness::poison_series(s, day, "SPY"));
        let pcfg = Config { end: end.min(pd - 1), ..cfg.clone() };
        let result = run_with(&poisoned_raw, &Prices::from_raw(&poisoned_raw), poisoned_index.as_deref(), Some(day), &pcfg).and_then(|(p, _, _)| harness::poison_compare(&report, &p, pd));
        match result {
            Ok(n) => {
                let t = format!("poison check {day}: PASS ({n} rebalances before {day} identical)");
                let _ = writeln!(h, "{t}");
                poison_text = Some(t);
            }
            Err(e) => {
                let t = format!("poison check {day}: FAIL: {e}");
                let _ = writeln!(h, "{t}");
                poison_text = Some(t);
                poison_error = Some(e);
            }
        }
    }
    if let Some(path) = opt(args, "--trades") {
        std::fs::write(path, harness::trades_csv(&report.trades, None)).map_err(|e| format!("{path}: {e}"))?;
        let _ = writeln!(h, "trades: {} written to {path}", report.trades.len());
    }
    let mut log_path = None;
    if let Some((hy, proof)) = &hyp {
        let (code, dirty) = harness::code_state(repo);
        let (store, store_dirty) = harness::store_state(&cache);
        let level = |r: &Report| harness::perf_json(&r.fund);
        let stamp = harness::utc_stamp(harness::now_secs());
        let (path, n_id, n_strategy) = harness::write_run_log(&repo.join("trading").join("runs"), &hy.id, &hy.strategy, &stamp, |n_id, n_strategy| {
            serde_json::json!({
                "id": hy.id,
                "strategy": hy.strategy,
                "parent": hy.parent,
                "run_time_utc": stamp,
                "labels": { "universe": harness::UNIVERSE_LABEL, "prices": harness::PRICE_LABEL },
                "code_commit": code,
                "code_dirty": dirty,
                "store_commit": store,
                "store_dirty": store_dirty,
                "hypothesis": {
                    "file": proof.path, "sha256": proof.sha256, "commit": proof.commit, "commit_time": proof.commit_time,
                    "registered": hy.date, "oos_start": hy.oos_start, "universe": hy.universe, "params": hy.params, "text": hy.hypothesis,
                },
                "oos_read": oos,
                "variant_count": { "hypothesis": n_id, "strategy": n_strategy },
                "config": {
                    "command": args_in, "effective_arguments": args, "cache": cache.display().to_string(),
                    "start": mapcheck::dates::format(start), "end": mapcheck::dates::format(end), "rebalance": format!("{rebalance:?}"),
                    "cost_bp": cost_bp, "paths": paths, "seed": seed, "n_tickers": tickers.len(), "n_scored": n_scored,
                    "first_position": report.dates.first().map(|d| mapcheck::dates::format(*d)), "rebalances": report.rebalances, "trades": report.trades.len(),
                },
                "results": {
                    "cost_0x": level(&zero), "cost_1x": level(&report), "cost_2x": level(&two),
                    "spy": harness::perf_json(&report.bench_perf), "equal_weight": harness::perf_json(&ew.fund),
                    "turnover_total": report.turnover_total, "cost_total": report.cost_total,
                },
                "poison_check": poison_text,
            })
        })?;
        let _ = writeln!(h, "runs of this hypothesis: {n_id}; runs of strategy {}: {n_strategy}", hy.strategy);
        let _ = writeln!(h, "run log: {}", path.display());
        log_path = Some(path);
    }
    print!("{h}");
    let _ = log_path;
    if let Some(path) = opt(args, "--out") {
        let mut csv = String::from("date,portfolio,spy\n");
        for k in 0..report.dates.len() {
            let _ = writeln!(csv, "{},{:.6},{:.6}", mapcheck::dates::format(report.dates[k]), report.nav[k], report.bench[k]);
        }
        std::fs::write(path, csv).map_err(|e| format!("{path}: {e}"))?;
    }
    match poison_error {
        Some(e) => Err(format!("poison check failed: {e}")),
        None => Ok(()),
    }
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
    let table = if r.strategy == "value-rank" { " The parameter table is in-sample (estimated on all years)." } else { "" };
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
