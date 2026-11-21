//! The command `fa portfolio backtest`.

use super::harness::{self, Hypothesis};
use super::strategies::{load_vintages_with, EqualWeight, Momentum, Pick, ValueRank, Vintages, WeeklyValueRank};
use super::{perf_line, Config, Prices, Rebalance, Report};
use crate::dcf::auto::{index_from_cache, DEFAULT_TABLE};
use crate::dcf::auto_cli::load_table;
use crate::dcf::auto_pit::parse_rules;
use crate::dcf::auto_returns::RetParams;
use crate::market::splits::Split;
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
                     (--neutral and --pick are not parameters of a hypothesis file: with --hypothesis they are an error)
  --hold true|false  (false) no rebalance: a held ticker that is still in the targets keeps its shares (no trade); a ticker that
                     leaves the targets is sold at that close; a new ticker gets its target weight of the value of that day after
                     the sales (scaled down together if the cash is not enough). Cost on the traded value only. For every strategy;
                     mainly value-rank-weekly (A-009 holds 13 weeks). The equal-weight line is not changed. A hypothesis file can
                     set it (params.hold).
  --out FILE         write the daily value of the portfolio, of SPY and of the equal-weight line as CSV
  --holdings FILE    write the target weights of each rebalance date as CSV (date,ticker,weight)
  --hypothesis FILE  a registered hypothesis (trading/hypotheses/<id>.toml, committed): the file gives strategy, params and tickers
                     file (a value of the command that differs from the file is an error); the run writes the add-only log
                     trading/runs/<id>/<UTC time>.json and prints the variant counts. No file: an exploratory run, no log.
  --trades FILE      write the trades as CSV: date, ticker, action, weight before and after, price, signal, filing accession
  --poison-check D[,D2]  run again with each close and each fact from D on multiplied by a factor that differs by ticker (D: a
                     rebalance day; use one in Feb-Mar and one in the second half of the year); fail if a target weight of a rebalance up to
                     D (D included) or a trade before D differs
  Each run prints the result at 0x, 1x and 2x the cost, and the flag oos-read (the end is after oos_start of the hypothesis).
  Survivorship: the lists of tickers are today's lists, so the result is too good. The output has an equal-weight line of
  the whole list for the comparison.";

pub const CALIBRATE_USAGE: &str = "\
fa portfolio calibrate --start D [--end D] [OPTIONS]
  The calibration of the weekly rule (A-009), tests T2 to T4 of PLANNING/board/T-20261121-blind-projection-test.md
  (Revision 1). Each week (the Friday) each scored ticker gets the rank inside its sector, made from the prices before the
  day and the SEC facts filed before it (the scorer of value-rank-weekly). Only then the outcome is read: the return from the
  close of that day to the close 91 days later, less the equal weight of the scored list of that week. The entries are those
  of value-rank-weekly (re-entries marked). The text on the screen has each test with its pass rule, its value, pass or
  fail, and the headline. No network, no order. Price returns, today's list.
  --cache DIR --tickers-file F --table FILE --paths N --seed S   as for backtest
  --ranks-out FILE     one row for each week and scored ticker: date, ticker, sector, rank, ratio, fwd_13w, excess_13w (T2)
  --entries-out FILE   the entries of the rule with the same columns and a reentry flag (T3)
  --block-weeks N (26) block of the circular bootstrap, in weeks (the checks 13 and 52 are also printed)
  --boot-seed S (7)    seed of the bootstrap          --reps N (10000): resamples
  --stress-file FILE   stress windows, one line name,start,end (analysis/macro/stress-windows.csv); descriptive only
  The command has no run log and no --hypothesis: it is an exploratory measurement, not a registered run.";

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
    let sub = args.first().map(String::as_str);
    if !matches!(sub, Some("backtest" | "calibrate")) || args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}\n\n{CALIBRATE_USAGE}");
        return u8::from(!matches!(sub, Some("backtest" | "calibrate")) && !args.is_empty());
    }
    let result = if sub == Some("calibrate") { calibrate(repo, &args[1..]) } else { backtest(repo, &args[1..]) };
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("fa: {e}");
            2
        }
    }
}

/// Each option that `backtest` knows. Each takes one value.
const KNOWN: [&str; 18] = [
    "--strategy", "--cache", "--start", "--end", "--rebalance", "--cost-bp", "--tickers-file", "--paths", "--seed", "--table", "--out", "--hypothesis", "--trades", "--poison-check", "--neutral", "--pick",
    "--holdings", "--hold",
];

/// Each option that `calibrate` knows. Each takes one value.
const CALIBRATE_KNOWN: [&str; 13] = ["--cache", "--start", "--end", "--tickers-file", "--paths", "--seed", "--table", "--ranks-out", "--entries-out", "--block-weeks", "--boot-seed", "--reps", "--stress-file"];

/// An option that the command does not know (a typo, or `--option=value`), or an option with no value, is an error. It runs before any work.
fn check_options(args: &[String], known: &[&str]) -> Result<(), String> {
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
    Ok(())
}

/// The days of `--poison-check D1,D2`: (text, day number).
fn poison_days(args: &[String]) -> Result<Vec<(String, i64)>, String> {
    let Some(list) = opt(args, "--poison-check") else { return Ok(Vec::new()) };
    list.split(',').map(|t| mapcheck::dates::parse(t).map(|d| (t.to_string(), d)).ok_or_else(|| format!("--poison-check '{t}' is not YYYY-MM-DD (a list: D1,D2)"))).collect()
}

/// The files that the command writes (`--trades`, `--out`, ...) must not exist: it does not replace a file. Runs before any work.
fn refuse_existing(args: &[String], names: &[&str]) -> Result<(), String> {
    for name in names {
        if let Some(p) = opt(args, name) {
            if Path::new(p).exists() {
                return Err(format!("{name} {p}: the file exists; the command does not replace a file"));
            }
        }
    }
    Ok(())
}

/// Close the run log, and only then give the text to the writer. If the log cannot be closed, nothing is written: no result is
/// visible that the log does not hold.
fn publish(finish: impl FnOnce() -> Result<(), String>, text: &str, write: &mut dyn FnMut(&str)) -> Result<(), String> {
    finish()?;
    write(text);
    Ok(())
}

/// Write a new file. A file that exists is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), String> {
    use std::io::Write;
    std::fs::OpenOptions::new().write(true).create_new(true).open(path).and_then(|mut f| f.write_all(text.as_bytes())).map_err(|e| format!("{path}: {e} (the command does not replace a file)"))
}

/// The tickers and the closes that a command reads (`--tickers-file`, the cache).
struct Inputs {
    /// The tickers with a price file (SPY is not in the list).
    tickers: Vec<String>,
    raw: BTreeMap<String, Vec<(String, f64)>>,
    prices: Prices,
    /// The text of `--tickers-file` (the run log holds its hash).
    universe_text: Option<String>,
}

fn load_inputs(args: &[String], cache: &Path) -> Result<Inputs, String> {
    let universe_text = match opt(args, "--tickers-file") {
        Some(f) => Some(std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?),
        None => None,
    };
    let mut tickers = match &universe_text {
        Some(t) => read_tickers(t),
        None => crate::dcf::panel::cache_tickers(cache)?,
    };
    tickers.retain(|t| t != "SPY");
    let mut names = tickers.clone();
    names.push("SPY".into());
    let raw: BTreeMap<String, Vec<(String, f64)>> = names
        .par_iter()
        .filter_map(|t| {
            let p = crate::market::prices::read_cache(&crate::market::prices::cache_path(cache, t)).ok()?;
            Some((t.clone(), p.into_iter().map(|x| (x.date, x.close)).collect()))
        })
        .collect();
    if !raw.contains_key("SPY") {
        return Err(format!("no price file of SPY in {}", cache.display()));
    }
    tickers.retain(|t| raw.contains_key(t));
    let prices = Prices::from_raw(&raw);
    Ok(Inputs { tickers, raw, prices, universe_text })
}

/// What a value-rank scorer needs besides the prices. One loader for `backtest`, the poison runs and `calibrate`.
struct VrParts {
    table: crate::dcf::calibrate::ProcessTable,
    fade: Vec<crate::dcf::calibrate::FadeCell>,
    registry: Registry,
    rules: crate::dcf::auto::Rules,
    curves: Option<crate::forwards::Dataset>,
    /// The split table: the closes are split-adjusted to the last date of each price file, and the share counts of each
    /// vintage go to the same basis. A missing file is an error.
    splits: Vec<Split>,
    /// The check report of the split table for the tickers of the run (print it before the result).
    split_note: String,
}

fn table_file_of(repo: &Path, args: &[String]) -> PathBuf {
    opt(args, "--table").map(PathBuf::from).unwrap_or_else(|| repo.join(DEFAULT_TABLE))
}

fn load_vr_parts(repo: &Path, args: &[String], cache: &Path, tickers: &[String]) -> Result<VrParts, String> {
    let (table, fade) = load_table(&table_file_of(repo, args)).map_err(|e| e.0)?;
    let registry = Registry::embedded()?;
    let rules = parse_rules("").map_err(|e| e.0)?;
    let curves = std::fs::read_to_string(cache.join("rates").join("gsw_feds200628.csv")).ok().and_then(|t| crate::forwards::parse_gsw(&t).ok());
    let splits = crate::market::splits::load_splits(cache)?;
    let split_note = crate::dcf::share_basis::check_report(&splits, tickers);
    Ok(VrParts { table, fade, registry, rules, curves, splits, split_note })
}

/// The vintages of the tickers with SEC facts of the standard template (split-corrected), and the scored universe.
/// `poison`: the day from which each fact is multiplied by a factor of its ticker (the poison check).
fn value_rank_years(p: &VrParts, cache: &Path, tickers: &[String], start: i64, poison: Option<&str>) -> (BTreeMap<String, Vintages>, Vec<String>) {
    let years: BTreeMap<String, Vintages> = tickers
        .par_iter()
        .filter_map(|t| {
            let (tpl, y) = match poison {
                None => load_vintages_with(&p.registry, cache, t, start, &p.splits, |_| {})?,
                Some(day) => load_vintages_with(&p.registry, cache, t, start, &p.splits, |f| harness::poison_facts(f, day, harness::fact_factor(t)))?,
            };
            (tpl == "standard").then(|| (t.clone(), y))
        })
        .collect();
    let universe: Vec<String> = tickers.iter().filter(|t| years.contains_key(*t)).cloned().collect();
    (years, universe)
}

/// The Fama-French 12 sector of each ticker (today's SIC code of the cache); `None` mode: no sectors.
fn sector_map(cache: &Path, universe: &[String], mode: Option<&str>) -> Result<Option<BTreeMap<String, String>>, String> {
    match mode {
        None => Ok(None),
        Some("sector") => Ok(Some(
            universe.iter().map(|t| (t.clone(), mapcheck::sec::company_sic(cache, t).map_or("Other", |(sic, _)| mapcheck::sec::ff12_sector(sic)).to_string())).collect(),
        )),
        Some(x) => Err(format!("--neutral '{x}': sector")),
    }
}

fn calibrate(repo: &Path, args: &[String]) -> Result<(), String> {
    check_options(args, &CALIBRATE_KNOWN)?;
    refuse_existing(args, &["--ranks-out", "--entries-out"])?;
    let num = |name: &str, default: u64| -> Result<u64, String> { opt(args, name).map_or(Ok(default), |v| v.parse().map_err(|_| format!("{name} '{v}': a whole number"))) };
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let paths = num("--paths", 300)? as usize;
    if paths < 100 {
        return Err(format!("--paths {paths}: 100 or more"));
    }
    let seed = num("--seed", 7)?;
    let params = super::calibrate::StatParams { block_weeks: num("--block-weeks", 26)?.max(1) as usize, seed: num("--boot-seed", 7)?, reps: num("--reps", 10_000)?.max(1) as usize };
    let start = day_of(args, "--start")?.ok_or("--start is needed")?;
    let windows = match opt(args, "--stress-file") {
        Some(f) => super::calibrate::parse_windows(&std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?)?,
        None => Vec::new(),
    };
    let Inputs { tickers, raw, prices, .. } = load_inputs(args, &cache)?;
    let spy_last = prices.0["SPY"].last().map(|x| x.0).ok_or("SPY has no prices")?;
    let end = day_of(args, "--end")?.unwrap_or(spy_last);
    let cal: Vec<i64> = prices.0["SPY"].iter().map(|x| x.0).filter(|d| *d >= start && *d <= end).collect();
    let rdays = super::rebalance_days(&cal, start, end, Rebalance::Weekly);

    let parts = load_vr_parts(repo, args, &cache, &tickers)?;
    let (years, universe) = value_rank_years(&parts, &cache, &tickers, start, None);
    eprintln!("value-rank: {} of {} tickers have SEC facts with the standard template", universe.len(), tickers.len());
    let sectors = sector_map(&cache, &universe, Some("sector"))?.ok_or("the sector of each ticker is needed")?;
    let index = index_from_cache(&cache);
    let vparams = RetParams { table: &parts.table, fade: &parts.fade, rules: &parts.rules, curves: parts.curves.as_ref(), index: None, lag: 100, paths, seed };
    let s = ValueRank { years, series: &raw, index: index.as_deref(), params: vparams, scored: Default::default(), signals: Default::default(), sectors: Some(sectors.clone()), pick: Pick::Top };
    let (rows, entries) = super::calibrate::collect(&prices, &universe, &|v, u| s.score(v, u), &sectors, &rdays, end.min(spy_last));
    if let Some(f) = opt(args, "--ranks-out") {
        write_new(f, &super::calibrate::ranks_csv(&rows))?;
    }
    if let Some(f) = opt(args, "--entries-out") {
        write_new(f, &super::calibrate::entries_csv(&entries))?;
    }
    print!("{}{}", parts.split_note, super::calibrate::summary(&rows, &entries, &windows, &params));
    Ok(())
}

/// What a finished run gives, to print and to log, after the log record exists.
struct Body {
    out: String,
    results: serde_json::Value,
    poison_lines: Vec<String>,
    poison_error: Option<String>,
    trades_csv: String,
    series_csv: String,
    holdings_csv: String,
}

fn backtest(repo: &Path, args_in: &[String]) -> Result<(), String> {
    check_options(args_in, &KNOWN)?;
    refuse_existing(args_in, &["--trades", "--out", "--holdings"])?;
    let hyp: Option<(Hypothesis, harness::FileProof)> = match opt(args_in, "--hypothesis") {
        Some(f) => {
            let file = PathBuf::from(f);
            let proof = harness::prove_committed(repo, &file)?;
            let h = Hypothesis::parse(&proof.text)?;
            let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            if stem != h.id {
                return Err(format!("the id '{}' of the hypothesis is not the file name '{stem}'", h.id));
            }
            Some((h, proof))
        }
        None => None,
    };
    if hyp.is_some() {
        if let Some(x) = ["--neutral", "--pick"].iter().find(|x| opt(args_in, x).is_some()) {
            return Err(format!("{x} is not a parameter of a hypothesis file: the run would differ from the registered one"));
        }
        if let Some(why) = harness::history_state(repo) {
            return Err(format!("the git history of the run logs is incomplete ({why}): a hypothesis run is refused, because the variant count could be too low"));
        }
    }
    let merged: Vec<String> = match &hyp {
        Some((h, _)) => h.merge_args(args_in, repo)?,
        None => args_in.to_vec(),
    };
    let args = &merged[..];
    let strategy = opt(args, "--strategy").ok_or("--strategy is needed: value-rank, value-rank-weekly or momentum")?;
    let hold = match opt(args, "--hold").unwrap_or("false") {
        "true" => true,
        "false" => false,
        x => return Err(format!("--hold '{x}': true or false")),
    };
    let is_vr = strategy.starts_with("value-rank");
    let weekly = strategy == "value-rank-weekly";
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let rebalance = match opt(args, "--rebalance").unwrap_or("monthly") {
        "monthly" => Rebalance::Monthly,
        "quarterly" => Rebalance::Quarterly,
        "weekly" => Rebalance::Weekly,
        x => return Err(format!("--rebalance '{x}': monthly, quarterly or weekly")),
    };
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
    if !is_vr && (opt(args, "--neutral").is_some() || opt(args, "--pick").is_some()) {
        return Err("--neutral and --pick are for the value-rank strategies".into());
    }
    let sector_mode = opt(args, "--neutral").or(weekly.then_some("sector"));
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
    let poisons = poison_days(args)?;

    let t0 = std::time::Instant::now();
    let Inputs { tickers, raw, prices, universe_text } = load_inputs(args, &cache)?;
    let end = match day_of(args, "--end")? {
        Some(d) => d,
        None => prices.0["SPY"].last().map(|x| x.0).ok_or("SPY has no prices")?,
    };
    let cfg = Config { start, end, rebalance, cost_bp };
    let late = tickers.iter().filter(|t| prices.0.get(*t).and_then(|s| s.first()).is_some_and(|f| f.0 > start)).count();

    // What a value-rank run needs besides the prices (loaded once; the poison runs use the same).
    let table_file = table_file_of(repo, args);
    let vr = if is_vr { Some(load_vr_parts(repo, args, &cache, &tickers)?) } else { None };
    let index = if vr.is_some() { index_from_cache(&cache) } else { None };

    // The record of the run is written BEFORE the first result exists: a run that stops leaves its trace (a counted variant).
    let (code, dirty) = harness::code_state(repo);
    let mut start_record = serde_json::Value::Null;
    let mut state: Option<(PathBuf, harness::Counts, bool, bool, Vec<String>, i64)> = None;
    let mut explore_reads: Vec<harness::ExploreRead> = Vec::new();
    let mut explore_files: Vec<String> = Vec::new();
    let mut explore_incomplete: Option<String> = None;
    if let Some((hy, proof)) = &hyp {
        let res = harness::resolve(repo, hy)?;
        let counts = harness::count_runs(repo, hy, &res);
        let read = harness::oos_read(end, res.window_start);
        let (clean, why) = harness::oos_clean(hy, &res.window_id, &counts.earlier_reads, &counts.exploratory_dates, is_vr);
        let (store, store_dirty) = harness::store_state(&cache);
        let stamp = harness::utc_stamp(harness::now_secs());
        let hash_of = |p: &Path| std::fs::read(p).ok().map(|b| harness::sha256_hex(&b));
        let table_json = is_vr.then(|| {
            serde_json::json!({
                "file": harness::rel_path(repo, &table_file.to_string_lossy()), "sha256": hash_of(&table_file),
                "fit_window": "all fiscal years of the data store at the table date (2026-11-19), through 2025: in-sample for any window from 2023 on",
            })
        });
        start_record = serde_json::json!({
            "schema": 2,
            "status": "started",
            "id": hy.id,
            "strategy": hy.strategy,
            "family": res.family,
            "parent": hy.parent,
            "ancestors": res.chain,
            "run_time_utc": stamp,
            "labels": { "universe": harness::UNIVERSE_LABEL, "prices": harness::PRICE_LABEL },
            "code": { "commit": code, "dirty": dirty },
            "build": { "commit": harness::BUILD_COMMIT, "dirty": harness::BUILD_DIRTY, "is_head": !harness::BUILD_COMMIT.is_empty() && code.as_deref() == Some(harness::BUILD_COMMIT) },
            "store": { "commit": store, "dirty": store_dirty },
            "hypothesis": {
                "file": proof.path, "sha256": proof.sha256, "commit": proof.commit, "commit_time": proof.commit_time,
                "registered": hy.date, "universe": hy.universe, "universe_sha256": universe_text.as_ref().map(|t| harness::sha256_hex(t.as_bytes())),
                "params": hy.params, "text": hy.hypothesis, "success": hy.success, "prior_variants": hy.prior_variants,
            },
            "oos_read": read,
            "oos_clean": clean,
            "oos": { "window_start": mapcheck::dates::format(res.window_start), "window_owner": res.window_id, "reasons_not_clean": why },
            "variant_count": {
                "hypothesis": counts.variants_id(), "family": counts.variants_family(),
                "logged_runs_hypothesis": counts.n_id, "logged_runs_family": counts.n_family,
                "prior_variants_hypothesis": counts.prior_id, "prior_variants_family": counts.prior_family,
            },
            "table": table_json,
            "config": {
                "command": harness::log_args(repo, args_in), "effective_arguments": harness::log_args(repo, args), "store": "<store>",
                "start": mapcheck::dates::format(start), "end": mapcheck::dates::format(end), "rebalance": format!("{rebalance:?}"),
                "cost_bp": cost_bp, "hold": hold, "paths": paths, "seed": seed, "n_tickers": tickers.len(),
                "poison_check_days": poisons.iter().map(|p| p.0.clone()).collect::<Vec<_>>(),
            },
        });
        let path = harness::write_started(repo, &hy.id, &stamp, &start_record)?;
        eprintln!("run log started: {} (no result is printed before the run ends)", harness::rel_path(repo, &path.to_string_lossy()));
        state = Some((path, counts, read, clean, why, res.window_start));
    } else {
        explore_reads = harness::exploratory_reads(repo, strategy, end);
        if !explore_reads.is_empty() {
            explore_incomplete = harness::history_state(repo);
        }
        // Each registered window that this run reads gets a record BEFORE any result: a new file for each window (a read, not a variant).
        let stamp = harness::utc_stamp(harness::now_secs());
        for read in &explore_reads {
            let record = serde_json::json!({
                "kind": "window-read",
                "run_time_utc": stamp,
                "code": { "commit": code, "dirty": dirty },
                "window": { "read_hypothesis": read.id, "owner": read.window_id, "start": mapcheck::dates::format(read.window_start) },
                "run": { "start": mapcheck::dates::format(start), "end": mapcheck::dates::format(end) },
                "strategy": strategy,
                "command": harness::log_args(repo, args_in),
                "store": "<store>",
                "history_incomplete": explore_incomplete,
            });
            let path = harness::write_window_read(repo, read, &stamp, &record)?;
            explore_files.push(harness::rel_path(repo, &path.to_string_lossy()));
        }
    }

    let work = || -> Result<Body, String> {
        // One run of the strategy on the given closes. `poison`: the day from which each fact is multiplied by a factor of its ticker.
        let run_with = |raw: &BTreeMap<String, Vec<(String, f64)>>, prices: &Prices, index: Option<&[(String, f64)]>, poison: Option<&str>, cfg: &Config| -> Result<(Report, Option<usize>, Vec<String>), String> {
            match (strategy, &vr) {
                ("momentum", _) => Ok((super::run_mode(prices, &tickers, &Momentum, cfg, hold)?, None, tickers.clone())),
                ("value-rank" | "value-rank-weekly", Some(p)) => {
                    let (years, universe) = value_rank_years(p, &cache, &tickers, start, poison);
                    if poison.is_none() {
                        eprintln!("value-rank: {} of {} tickers have SEC facts with the standard template", universe.len(), tickers.len());
                    }
                    let params = RetParams { table: &p.table, fade: &p.fade, rules: &p.rules, curves: p.curves.as_ref(), index: None, lag: 100, paths, seed };
                    let sectors = sector_map(&cache, &universe, sector_mode)?;
                    let s = ValueRank { years, series: raw, index, params, scored: Default::default(), signals: Default::default(), sectors, pick };
                    let n = universe.len();
                    let report = if weekly { super::run_mode(prices, &universe, &WeeklyValueRank { base: s, open: Default::default() }, cfg, hold)? } else { super::run_mode(prices, &universe, &s, cfg, hold)? };
                    Ok((report, Some(n), universe))
                }
                (x, _) => Err(format!("unknown strategy '{x}': value-rank, value-rank-weekly or momentum")),
            }
        };
        let (report, n_scored, universe) = run_with(&raw, &prices, index.as_deref(), None, &cfg)?;
        let ew = super::run(&prices, &tickers, &EqualWeight, &cfg)?;
        let info = Info { n_tickers: tickers.len(), n_scored, late, seconds: t0.elapsed().as_secs_f64() };
        let mut out = format!("{}{}", vr.as_ref().map_or("", |p| p.split_note.as_str()), render(&report, &ew, &cfg, args_in, &info));

        let (zero, two) = harness::cost_levels(&prices, &universe, &report, &cfg, hold)?;
        let mut h = String::new();
        let _ = writeln!(h, "\nRUN HARNESS");
        let _ = writeln!(h, "{}; {}", harness::UNIVERSE_LABEL, harness::PRICE_LABEL);
        let _ = writeln!(h, "costs (the same strategy, the same target weights and dates; cost {cost_bp} bp is 1x):");
        for (name, r) in [("cost 0x", &zero), ("cost 1x", &report), ("cost 2x", &two)] {
            let _ = writeln!(h, "{}", perf_line(name, &r.fund));
        }
        let _ = writeln!(h, "{}", harness::cost_note(&zero, &report, ew.fund.annual));
        let _ = writeln!(h, "note: the equal-weight line is of the whole list ({} tickers), not of the scored names{}; the line on the scored names is part 2.", tickers.len(), n_scored.map_or(String::new(), |n| format!(" ({n})")));
        match (&hyp, &state) {
            (Some((hy, _)), Some((_, counts, read, clean, why, window))) => {
                let _ = writeln!(h, "hypothesis {} (registered {}), prior variants {}; out-of-sample window from {}", hy.id, hy.date, hy.prior_variants.unwrap_or(0), mapcheck::dates::format(*window));
                if let Some(s) = &hy.success {
                    let _ = writeln!(h, "success criterion (written before the run): {s}");
                }
                let _ = writeln!(h, "oos-read: {read}");
                let _ = writeln!(h, "oos_clean: {clean}");
                for w in why {
                    let _ = writeln!(h, "  - not clean: {w}");
                }
                if *read {
                    let _ = writeln!(h, "WARNING: the run ends on or after the start of the out-of-sample window ({}): it reads the window. Do not tune on this result.", mapcheck::dates::format(*window));
                }
                let _ = writeln!(
                    h,
                    "runs of this hypothesis: {} (prior variants {} + {} logged {}, this run included); runs of family {}: {} (prior {} + {} logged {} of all ids, git history included)",
                    counts.variants_id(),
                    counts.prior_id,
                    counts.n_id,
                    if counts.n_id == 1 { "run" } else { "runs" },
                    start_record["family"].as_str().unwrap_or("?"),
                    counts.variants_family(),
                    counts.prior_family,
                    counts.n_family,
                    if counts.n_family == 1 { "run" } else { "runs" }
                );
            }
            _ => {
                let _ = writeln!(h, "exploratory run: no hypothesis file, not usable for a decision (not counted as a variant)");
                if !explore_reads.is_empty() {
                    let names: Vec<String> = explore_reads.iter().map(|r| format!("'{}' (window of '{}' from {})", r.id, r.window_id, mapcheck::dates::format(r.window_start))).collect();
                    let _ = writeln!(h, "WARNING: this exploratory run reads the out-of-sample window of {}: not counted as a variant; the window is not clean after this read", names.join(", "));
                    for p in &explore_files {
                        let _ = writeln!(h, "window read recorded: {p} (commit it with your work)");
                    }
                    if let Some(why) = &explore_incomplete {
                        let _ = writeln!(h, "WARNING: the git history is incomplete ({why}): the counts and the earlier reads of this window may be too low (history_incomplete in the record)");
                    }
                }
            }
        }
        if harness::BUILD_COMMIT.is_empty() {
            let _ = writeln!(h, "WARNING: the commit of the build is unknown.");
        } else if code.as_deref() != Some(harness::BUILD_COMMIT) {
            let _ = writeln!(h, "WARNING: this binary was built from commit {}, not from HEAD {}.", harness::BUILD_COMMIT, code.as_deref().unwrap_or("?"));
        }
        if dirty {
            let _ = writeln!(h, "WARNING: the code has changes that are not committed (code_dirty).");
        }
        let (mut poison_lines, mut poison_error) = (Vec::new(), None);
        for (day, pd) in &poisons {
            let line = if !report.targets.iter().any(|(d, _)| d == pd) {
                poison_error = Some(format!("{day} is not a rebalance day of the run"));
                format!("poison check {day}: FAIL: {day} is not a rebalance day of the run (use the first trading day of a month)")
            } else {
                let poisoned_raw: BTreeMap<String, Vec<(String, f64)>> = raw.iter().map(|(t, s)| (t.clone(), harness::poison_series(s, day, t))).collect();
                let poisoned_index = index.as_ref().map(|s| harness::poison_series(s, day, "SPY"));
                let pcfg = Config { end: *pd, ..cfg.clone() };
                let result = run_with(&poisoned_raw, &Prices::from_raw(&poisoned_raw), poisoned_index.as_deref(), Some(day), &pcfg).and_then(|(p, _, _)| harness::poison_compare(&report, &p, *pd));
                match result {
                    Ok(n) => format!("poison check {day}: PASS ({n} rebalances up to {day}, the day included, identical; closes and facts from {day} on were changed)"),
                    Err(e) => {
                        poison_error.get_or_insert(e.clone());
                        format!("poison check {day}: FAIL: {e}")
                    }
                }
            };
            let _ = writeln!(h, "{line}");
            poison_lines.push(line);
        }
        if !poison_lines.is_empty() {
            let _ = writeln!(h, "{}", harness::POISON_NOT_COVERED);
        }
        out.push_str(&h);
        let level = |r: &Report| harness::perf_json(&r.fund);
        let results = serde_json::json!({
            "cost_0x": level(&zero), "cost_1x": level(&report), "cost_2x": level(&two),
            "spy": harness::perf_json(&report.bench_perf), "equal_weight_whole_list": harness::perf_json(&ew.fund),
            "turnover_total": report.turnover_total, "cost_total": report.cost_total,
            "n_scored": n_scored, "first_position": report.dates.first().map(|d| mapcheck::dates::format(*d)),
            "rebalances": report.rebalances, "trades": report.trades.len(),
        });
        // the equal-weight line of the list, on the dates of the strategy (empty where it has no value on that date)
        let ew_at: BTreeMap<i64, f64> = ew.dates.iter().copied().zip(ew.nav.iter().copied()).collect();
        let mut series_csv = String::from("date,portfolio,spy,equal_weight\n");
        for k in 0..report.dates.len() {
            let e = ew_at.get(&report.dates[k]).map_or(String::new(), |v| format!("{v:.6}"));
            let _ = writeln!(series_csv, "{},{:.6},{:.6},{e}", mapcheck::dates::format(report.dates[k]), report.nav[k], report.bench[k]);
        }
        // the target weights of each rebalance date: the one copy is `Report::targets`
        let mut holdings_csv = String::from("date,ticker,weight\n");
        for (d, w) in &report.targets {
            for (t, x) in w {
                let _ = writeln!(holdings_csv, "{},{t},{x:.6}", mapcheck::dates::format(*d));
            }
        }
        Ok(Body { out, results, poison_lines, poison_error, trades_csv: harness::trades_csv(&report.trades, None), series_csv, holdings_csv })
    };
    let outcome = work();

    // The same file gets the final record; then the result is printed.
    let finish = |extra: serde_json::Value| -> Result<(), String> {
        let Some((path, ..)) = &state else { return Ok(()) };
        let mut rec = start_record.clone();
        if let (Some(r), Some(e)) = (rec.as_object_mut(), extra.as_object()) {
            r.extend(e.clone());
        }
        harness::finalize(path, &rec)
    };
    match outcome {
        Err(e) => {
            if let Err(f) = finish(serde_json::json!({ "status": "failed", "error": e })) {
                eprintln!("fa: the run log: {f}");
            }
            Err(e)
        }
        Ok(b) => {
            let mut text = b.out.clone();
            if let Some((path, ..)) = &state {
                let _ = writeln!(text, "run log ({}): {}", if b.poison_error.is_some() { "failed" } else { "done" }, harness::rel_path(repo, &path.to_string_lossy()));
            }
            // The files come first: a write error leaves the log "failed", not "done".
            let mut files = Vec::new();
            if let Some(path) = opt(args, "--trades") {
                files.push((path, &b.trades_csv));
                let _ = writeln!(text, "trades written to {path}");
            }
            if let Some(path) = opt(args, "--out") {
                files.push((path, &b.series_csv));
            }
            if let Some(path) = opt(args, "--holdings") {
                files.push((path, &b.holdings_csv));
            }
            if let Some(e) = files.iter().find_map(|(p, t)| write_new(p, t).err()) {
                if let Err(f) = finish(serde_json::json!({ "status": "failed", "error": e })) {
                    eprintln!("fa: the run log: {f}");
                }
                return Err(e);
            }
            let status = if b.poison_error.is_some() { "failed" } else { "done" };
            publish(
                || {
                    finish(serde_json::json!({
                        "status": status, "results": b.results, "poison_checks": b.poison_lines, "poison_error": b.poison_error,
                        "poison_not_covered": (!b.poison_lines.is_empty()).then_some(harness::POISON_NOT_COVERED),
                    }))
                },
                &text,
                &mut |t| print!("{t}"),
            )?;
            match b.poison_error {
                Some(e) => Err(format!("poison check failed: {e}")),
                None => Ok(()),
            }
        }
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
    use super::{check_options, poison_days, publish, refuse_existing, spread, write_new, CALIBRATE_KNOWN, KNOWN};

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn an_unknown_option_or_a_missing_value_is_an_error_before_any_work() {
        assert!(check_options(&a(&["--strategy", "momentum", "--start", "2018-01-01", "--hypothesis", "f.toml", "--trades", "t.csv", "--poison-check", "2022-03-01,2022-09-01"]), &KNOWN).is_ok());
        assert!(check_options(&a(&["--cost_bp", "99"]), &KNOWN).unwrap_err().contains("unknown option"));
        assert!(check_options(&a(&["--hypothesis=trading/hypotheses/x.toml", "--strategy", "momentum"]), &KNOWN).unwrap_err().contains("--hypothesis="));
        assert!(check_options(&a(&["--strategy"]), &KNOWN).unwrap_err().contains("needs a value"));
        assert!(check_options(&a(&["--strategy", "--start"]), &KNOWN).is_err());
        assert!(check_options(&a(&["momentum"]), &KNOWN).is_err());
        // the options of each command are its own
        assert!(check_options(&a(&["--neutral", "sector", "--pick", "bottom", "--holdings", "h.csv", "--rebalance", "weekly"]), &KNOWN).is_ok());
        assert!(check_options(&a(&["--ranks-out", "r.csv", "--stress-file", "s.csv"]), &CALIBRATE_KNOWN).is_ok());
        assert!(check_options(&a(&["--hypothesis", "f.toml"]), &CALIBRATE_KNOWN).is_err());
        assert!(check_options(&a(&["--ranks-out", "r.csv"]), &KNOWN).is_err());
    }

    #[test]
    fn the_result_is_written_only_after_the_log_is_closed() {
        use std::cell::RefCell;
        let order = RefCell::new(Vec::<String>::new());
        let ok = publish(|| { order.borrow_mut().push("finish".into()); Ok(()) }, "result", &mut |t| order.borrow_mut().push(format!("print {t}")));
        assert!(ok.is_ok());
        assert_eq!(*order.borrow(), ["finish", "print result"]);
        // a finish that fails: nothing is printed
        let order = RefCell::new(Vec::<String>::new());
        let e = publish(|| Err("disk full".to_string()), "result", &mut |t| order.borrow_mut().push(t.to_string()));
        assert_eq!(e, Err("disk full".to_string()));
        assert!(order.borrow().is_empty());
    }

    #[test]
    fn an_existing_output_file_is_refused_before_any_work() {
        let dir = std::env::temp_dir().join(format!("fa-cli-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (have, missing) = (dir.join("have.csv"), dir.join("missing.csv"));
        std::fs::write(&have, "x").unwrap();
        let (h, m) = (have.to_string_lossy().into_owned(), missing.to_string_lossy().into_owned());
        for opt in ["--trades", "--out"] {
            assert!(refuse_existing(&a(&[opt, &h]), &["--trades", "--out"]).unwrap_err().contains("the file exists"));
            assert!(refuse_existing(&a(&[opt, &m]), &["--trades", "--out"]).is_ok());
        }
        assert!(refuse_existing(&a(&["--start", "2018-01-01"]), &["--trades", "--out"]).is_ok());
        // the writer does not replace a file either
        assert!(write_new(&h, "y").is_err());
        assert_eq!(std::fs::read_to_string(&have).unwrap(), "x");
        assert!(write_new(&m, "y").is_ok() && write_new(&m, "z").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_poison_days_are_a_list_of_dates() {
        assert_eq!(poison_days(&a(&["--poison-check", "2022-03-01,2022-09-01"])).unwrap().len(), 2);
        assert!(poison_days(&a(&["--poison-check", "2022-03-01,x"])).is_err());
        assert!(poison_days(&a(&[])).unwrap().is_empty());
    }

    #[test]
    fn the_spread_of_the_scored_counts_is_min_median_max() {
        assert_eq!(spread(&[]), None);
        assert_eq!(spread(&[57, 93, 80]), Some((57, 80, 93)));
        assert_eq!(spread(&[0, 19]), Some((0, 19, 19)));
    }
}
