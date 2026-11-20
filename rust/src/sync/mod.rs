//! `fa sync`: one command that brings the cached data sets up to date, and only the old ones.
//!
//! The sync runs on request only: nothing runs between two commands (owner decision of 2026-11-19, no watch mode).
//!
//! - `plan`: what is old and what is current, from the cache files and the ledger (no network request);
//! - `exec`: one thread and one queue for each host, all hosts at the same time, a block stops its host only;
//! - `runner`: the real jobs. Each job calls the function of the existing data command (the sync does not copy their logic);
//! - `fresh`, `calendar`, `ledger`, `config`, `dataset`: the rules of the age of the data.
//!
//! The value commands (`fa dcf auto`, `fa verdict`, `fa estimates show`) do not start a sync. They call `old_data_warning`
//! and print one line when their data is older than its maximum age.

pub mod calendar;
pub mod config;
pub mod dataset;
pub mod exec;
pub mod fresh;
pub mod interrupt;
pub mod ledger;
pub mod plan;
pub mod runner;

use crate::market::price_cli;
use config::SyncConfig;
use dataset::{AgeUnit, DataSet, ALL};
use exec::{execute, summarize, ExecOptions, RunReport, Runner, Status, StopReason};
use fresh::Ctx;
use ledger::Ledger;
use plan::{make_plan, KeyInfo, Plan, PlanInput};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

pub use fresh::old_data_warning;

pub const USAGE: &str = "\
fa sync - bring the cached data sets up to date (only the old ones), on request

USAGE:
  fa sync [TICKER...] [--tickers-file F] [--only A,B] [--skip A,B] [--force] [--dry-run] [--max-minutes N]
          [--option-symbols A,B] [--intraday-symbols A,B]
  fa sync --status [TICKER...] [--tickers-file F] [--only A,B] [--skip A,B]

  TICKER...          the companies of the run (the tickers that you name also get the 10-K files). With no ticker and no
                     --tickers-file the list `tickers_file` of sync.toml is used; with none of the three, the data sets of
                     companies are skipped and the other data sets run.
  --only A,B         run only these data sets;   --skip A,B   run all except these.
  --force            ignore the maximum ages: ask each source again.
  --dry-run          print the plan (requests for each host, estimated time). No request is made.
  --status           print one line for each data set: current, old, no data. No request is made.
  --max-minutes N    stop after N minutes. A second run continues with what is old.
  --option-symbols   symbols of the option chains (default SPY, or option_symbols of sync.toml).
  --intraday-symbols more symbols of the 5-minute bars (SPY and QQQ are always in).

DATA SETS (name: what; maximum age; hosts):
  sec        company facts of the SEC; looks at the SEC filing list once each market day; 1 day; sec
  filings    the files of the latest 10-K (named tickers, and tickers that have them); with sec; sec
  prices     daily prices; 1 day; tiingo (with a key), yahoo, nasdaq, alpaca (with keys, last)
  intraday   5-minute bars of SPY, QQQ and --intraday-symbols; 1 day; alpaca (with keys), yahoo
  estimates  consensus, price targets, surprises; 7 days; nasdaq
  earnings   calendar scan and past report dates; 1 day; nasdaq, sec
  options    option chain of today for the option symbols; 1 day; cboe, alpaca (with keys)
  rates      Treasury curves, CPI; 1 day; treasury, bls
  gsw        Federal Reserve curve; 1 day; federalreserve
  fred       the FRED series of the bond functions; 1 day; fred
  futures    VIX futures, CFTC positions, policy path, Yahoo futures; 1 day; cboe, cftc, atlantafed, yahoo
  vix        Cboe VIX history file; 1 day; cboe
  bonds      FedInvest prices of the last market day; 1 day; treasurydirect
  A maximum age of 1 day counts market days: a weekend or a market holiday does not make the data old. The ages are in
  sync.toml (optional).

The SEC data sets need the environment variable SEC_USER_AGENT (a name and a contact). Without it they are skipped.
Keys (names in .env.example) add sources; the sync prints which key would add which source. Ctrl+C stops the run with all
files in a correct state; run the command again to continue.";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Args {
    pub tickers: Vec<String>,
    pub tickers_file: Option<String>,
    pub only: Option<Vec<DataSet>>,
    pub skip: Vec<DataSet>,
    pub force: bool,
    pub dry_run: bool,
    pub status: bool,
    pub max_minutes: Option<f64>,
    pub option_symbols: Option<Vec<String>>,
    pub intraday_symbols: Vec<String>,
    /// The words of the command line that say which companies and symbols (for the line "run again").
    pub echo: Vec<String>,
}

fn sets_of(list: &str, name: &str) -> Result<Vec<DataSet>, String> {
    let mut out = Vec::new();
    for w in list.split(',').map(str::trim).filter(|w| !w.is_empty()) {
        let set = DataSet::from_name(w).ok_or_else(|| format!("{name}: unknown data set '{w}'; the data sets are {}", ALL.iter().map(|d| d.name()).collect::<Vec<_>>().join(", ")))?;
        if !out.contains(&set) {
            out.push(set);
        }
    }
    if out.is_empty() {
        return Err(format!("{name} needs at least one data set"));
    }
    Ok(out)
}

fn symbols_of(list: &str) -> Vec<String> {
    list.split(',').map(|s| s.trim().to_uppercase()).filter(|s| !s.is_empty()).collect()
}

fn valid_symbol(t: &str) -> bool {
    !t.is_empty() && t.len() <= 12 && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '^')
}

pub fn parse_args(rest: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let mut i = 0;
    let value = |i: usize, name: &str| -> Result<String, String> { rest.get(i + 1).filter(|v| !v.starts_with("--")).cloned().ok_or_else(|| format!("{name} needs a value")) };
    let mut named: Vec<String> = Vec::new();
    while i < rest.len() {
        let w = rest[i].as_str();
        match w {
            "--tickers-file" => {
                let v = value(i, w)?;
                a.echo.extend([w.to_string(), v.clone()]);
                a.tickers_file = Some(v);
                i += 2;
            }
            "--only" => {
                a.only = Some(sets_of(&value(i, w)?, w)?);
                i += 2;
            }
            "--skip" => {
                a.skip.extend(sets_of(&value(i, w)?, w)?);
                i += 2;
            }
            "--max-minutes" => {
                let v = value(i, w)?;
                let n: f64 = v.parse().map_err(|_| format!("bad --max-minutes '{v}'"))?;
                if n.is_nan() || n <= 0.0 {
                    return Err("--max-minutes must be more than 0".into());
                }
                a.max_minutes = Some(n);
                i += 2;
            }
            "--option-symbols" => {
                let v = value(i, w)?;
                a.echo.extend([w.to_string(), v.clone()]);
                a.option_symbols = Some(symbols_of(&v));
                i += 2;
            }
            "--intraday-symbols" => {
                let v = value(i, w)?;
                a.echo.extend([w.to_string(), v.clone()]);
                a.intraday_symbols = symbols_of(&v);
                i += 2;
            }
            "--force" => {
                a.force = true;
                i += 1;
            }
            "--dry-run" => {
                a.dry_run = true;
                i += 1;
            }
            "--status" => {
                a.status = true;
                i += 1;
            }
            s if s.starts_with("--") => return Err(format!("unknown option {s}; see fa sync --help")),
            s => {
                let t = s.trim().to_uppercase();
                if !valid_symbol(&t) {
                    return Err(format!("'{s}' is not a ticker symbol"));
                }
                if !a.tickers.contains(&t) {
                    a.tickers.push(t.clone());
                    named.push(t);
                }
                i += 1;
            }
        }
    }
    let mut echo = named;
    echo.append(&mut a.echo);
    a.echo = echo;
    if a.status && (a.force || a.dry_run) {
        return Err("--status makes no request; it takes no --force and no --dry-run".into());
    }
    if let (Some(only), true) = (&a.only, !a.skip.is_empty()) {
        if only.iter().all(|s| a.skip.contains(s)) {
            return Err("--only and --skip leave no data set".into());
        }
    }
    Ok(a)
}

/// The data sets of the run: `--only`, else all, without `--skip`.
pub fn selected_sets(a: &Args) -> Vec<DataSet> {
    let base: Vec<DataSet> = a.only.clone().unwrap_or_else(|| ALL.to_vec());
    base.into_iter().filter(|s| !a.skip.contains(s)).collect()
}

/// The tickers of the run, in a stable order: the named tickers, then the file (the file of `--tickers-file`, or the default
/// file of the config when the person gave no ticker and no file).
pub fn resolve_tickers(repo: &Path, a: &Args, cfg: &SyncConfig) -> Result<Vec<String>, String> {
    let mut out = a.tickers.clone();
    let file = a.tickers_file.clone().or_else(|| if a.tickers.is_empty() { cfg.tickers_file.clone() } else { None });
    if let Some(f) = file {
        let direct = Path::new(&f).to_path_buf();
        let path = if direct.is_absolute() || direct.exists() { direct } else { repo.join(&f) };
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for t in price_cli::tickers_from_text(&text) {
            if !valid_symbol(&t) {
                return Err(format!("{}: '{t}' is not a ticker symbol", path.display()));
            }
            if !out.contains(&t) {
                out.push(t);
            }
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ text

pub fn fmt_seconds(secs: f64) -> String {
    let s = secs.round().max(0.0) as i64;
    if s >= 3600 {
        format!("{} h {:02} min", s / 3600, s % 3600 / 60)
    } else if s >= 60 {
        format!("{} min {:02} s", s / 60, s % 60)
    } else {
        format!("{s} s")
    }
}

fn unit_text(set: DataSet, n: u32) -> String {
    match set.age_unit() {
        AgeUnit::MarketDays => format!("{n} market day{}", if n == 1 { "" } else { "s" }),
        AgeUnit::CalendarDays => format!("{n} day{}", if n == 1 { "" } else { "s" }),
    }
}

/// What the run knows about the keys and the SEC contact (no value of a key).
#[derive(Debug, Clone, PartialEq)]
pub struct EnvInfo {
    pub sec_contact: bool,
    pub keys: KeyInfo,
}

impl EnvInfo {
    pub fn read(repo: &Path) -> EnvInfo {
        let sec = std::env::var(mapcheck::http::SEC_USER_AGENT_ENV).map(|v| !v.trim().is_empty()).unwrap_or(false);
        EnvInfo { sec_contact: sec, keys: KeyInfo::read(repo) }
    }

    pub fn describe(&self) -> String {
        let yn = |b: bool| if b { "set" } else { "not set" };
        format!(
            "SEC contact ({}): {}; keys: tiingo {}, alpaca {}, fred {}, fmp {}\n",
            mapcheck::http::SEC_USER_AGENT_ENV,
            yn(self.sec_contact),
            yn(self.keys.tiingo),
            yn(self.keys.alpaca),
            yn(self.keys.fred),
            yn(self.keys.fmp)
        )
    }
}

/// The text of `--status`.
pub fn render_status(plan: &Plan, cfg: &SyncConfig, ctx: &Ctx, env: &EnvInfo) -> String {
    let mut out = format!("fa sync status of {} (cache files and ledger; no request is made)\n", ctx.today_text());
    for (set, c) in &plan.counts {
        let oldest = plan.oldest.iter().find(|(s, _)| s == set).map_or(String::new(), |(_, d)| format!("; oldest record {d}"));
        out += &format!("  {:<10} {:>5} current, {:>5} old, {:>5} no data   (max age {}{oldest})\n", set.name(), c.current, c.old, c.missing, unit_text(*set, cfg.age_of(*set)));
    }
    for (set, why) in &plan.skipped {
        out += &format!("  {:<10} skipped: {why}\n", set.name());
    }
    out += &env.describe();
    out
}

/// The text of `--dry-run`.
pub fn render_dry_run(plan: &Plan, ctx: &Ctx, env: &EnvInfo) -> String {
    let mut out = format!("fa sync plan of {} (dry run: no request is made)\n", ctx.today_text());
    for (set, c) in &plan.counts {
        let n = plan.items.iter().filter(|i| i.set == *set).count();
        out += &format!("  {:<10} {:>5} current, {:>5} old, {:>5} no data -> {n} item(s)\n", set.name(), c.current, c.old, c.missing);
    }
    for (set, why) in &plan.skipped {
        out += &format!("  {:<10} skipped: {why}\n", set.name());
    }
    let load = plan.host_load();
    if load.is_empty() {
        out += "nothing to ask: all data sets are current\n";
    } else {
        out += "requests for each host (the hosts run at the same time; the pause of each host is the pause of its client):\n";
        for (h, (n, secs)) in &load {
            out += &format!("  {h:<15} {n:>6} request(s)  about {}\n", fmt_seconds(*secs));
        }
        out += &format!("estimated time of the run: about {} (the longest queue)\n", fmt_seconds(plan.estimated_seconds()));
    }
    out += &env.describe();
    out
}

/// The text of the summary of a run, and the exit code.
pub fn render_summary(plan: &Plan, report: &RunReport, args: &Args, ctx: &Ctx) -> (String, i32) {
    let summary = summarize(plan, report);
    let mut out = format!("fa sync summary of {} ({})\n", ctx.today_text(), fmt_seconds(report.elapsed.as_secs_f64()));
    let mut failed_sets = Vec::new();
    for (set, _) in &plan.counts {
        let s = summary.get(set).cloned().unwrap_or_default();
        let fmt_reason = |r: &Option<(String, usize)>| match r {
            Some((why, n)) => format!(" ({why}{})", if *n > 1 { format!(" x{n}") } else { String::new() }),
            None => String::new(),
        };
        let current = s.current_before + s.current_run;
        let not_yet = if s.not_yet > 0 { format!(", {} not available yet", s.not_yet) } else { String::new() };
        let (failed_reason, blocked_reason) = (fmt_reason(&s.reason), fmt_reason(&s.blocked_reason));
        out += &format!(
            "  {:<10} {} updated, {} current, {} no data, {} failed{failed_reason}, {} blocked{blocked_reason}, {} not tried{not_yet}\n",
            set.name(),
            s.updated,
            current,
            s.no_data,
            s.failed,
            s.blocked,
            s.not_tried
        );
        if s.failed_fully() {
            failed_sets.push(set.name());
        }
    }
    for (set, why) in &plan.skipped {
        out += &format!("  {:<10} skipped: {why}\n", set.name());
    }
    out += "hosts:\n";
    for (h, r) in &report.hosts {
        let blocked = match &r.blocked {
            Some(why) => format!("BLOCKED for this run ({why})"),
            None => "not blocked".to_string(),
        };
        out += &format!("  {h:<15} {:>5} request(s), {} in {} item(s), {blocked}\n", r.requests, fmt_seconds(r.busy.as_secs_f64()), r.items_run);
    }
    // What stays old.
    let mut stays: BTreeMap<DataSet, Vec<String>> = BTreeMap::new();
    for r in &report.results {
        if matches!(r.outcome.status, Status::Failed | Status::Blocked | Status::NotTried | Status::NotYet) {
            stays.entry(r.item.set).or_default().push(r.item.key.clone());
        }
    }
    if !stays.is_empty() {
        out += "stays old:\n";
        for (set, keys) in &stays {
            let shown: Vec<&str> = keys.iter().take(12).map(String::as_str).collect();
            out += &format!("  {:<10} {}{}\n", set.name(), shown.join(" "), if keys.len() > 12 { format!(" ... {} more", keys.len() - 12) } else { String::new() });
        }
        let sets: Vec<&str> = stays.keys().map(|s| s.name()).collect();
        out += &format!("run again: fa sync {} --only {}\n", args.echo.join(" "), sets.join(","));
    }
    match &report.stopped {
        Some(StopReason::Interrupted) => out += "stopped by Ctrl+C. All files are in a correct state. Run the same command again to continue with what is old.\n",
        Some(StopReason::TimeLimit) => out += &format!("stopped at the time limit of {} minute(s). Run the same command again to continue with what is old.\n", args.max_minutes.unwrap_or(0.0)),
        None => {}
    }
    for h in &plan.hints {
        out += &format!("key: {h}\n");
    }
    let code = if failed_sets.is_empty() { 0 } else { 1 };
    if code == 1 {
        out += &format!("failed data set(s): {}\n", failed_sets.join(", "));
    }
    (out, code)
}

// ------------------------------------------------------------------ the command

/// Builds the runner of a run from the tickers and the ledger (a test gives a fake).
pub type MakeRunner<'a> = &'a dyn Fn(&[String], &Ledger) -> Box<dyn Runner>;

/// Everything that a run needs from outside: the repo, the config, the world (keys, contact, the day) and the stop flag.
pub struct RunSetup<'a> {
    pub repo: &'a Path,
    pub ctx: Ctx,
    pub cfg: SyncConfig,
    pub env: EnvInfo,
    pub stop: &'a std::sync::atomic::AtomicBool,
    pub job_gap: BTreeMap<String, f64>,
}

/// The planning and the printing of one command. `make_runner` builds the runner when requests are to be made (a test gives a
/// fake that counts calls). Returns the exit code.
pub fn run_with(args: &Args, setup: &RunSetup, make_runner: MakeRunner, out: &mut dyn FnMut(&str), progress: &(dyn Fn(&str) + Sync)) -> Result<i32, String> {
    let tickers = resolve_tickers(setup.repo, args, &setup.cfg)?;
    let ledger = Ledger::load(&setup.ctx.cache);
    let option_symbols = args.option_symbols.clone().unwrap_or_else(|| setup.cfg.option_symbols.clone());
    let input = PlanInput {
        ctx: &setup.ctx,
        cfg: &setup.cfg,
        ledger: &ledger,
        tickers: tickers.clone(),
        focus: args.tickers.clone(),
        sets: selected_sets(args),
        force: args.force,
        sec_contact: setup.env.sec_contact,
        keys: setup.env.keys,
        intraday_extra: args.intraday_symbols.clone(),
        option_symbols,
    };
    let plan = make_plan(&input);
    if args.status {
        out(&render_status(&plan, &setup.cfg, &setup.ctx, &setup.env));
        return Ok(0);
    }
    if args.dry_run {
        out(&render_dry_run(&plan, &setup.ctx, &setup.env));
        return Ok(0);
    }
    progress(&format!(
        "fa sync: {} item(s) on {} host(s), estimated {}; Ctrl+C stops with all files in a correct state",
        plan.items.len(),
        plan.host_load().len(),
        fmt_seconds(plan.estimated_seconds())
    ));
    let runner = make_runner(&tickers, &ledger);
    let opts = ExecOptions { time_limit: args.max_minutes.map(|m| Duration::from_secs_f64(m * 60.0)), job_gap: setup.job_gap.clone() };
    let report = execute(&plan, runner.as_ref(), &opts, setup.stop, progress);
    let (text, code) = render_summary(&plan, &report, args, &setup.ctx);
    out(&text);
    Ok(code)
}

/// `fa sync`.
pub fn command(repo: &Path, rest: &[String]) -> Result<i32, String> {
    if rest.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let args = parse_args(rest)?;
    let cfg = SyncConfig::load(repo)?;
    let ctx = Ctx::new(repo);
    let env = EnvInfo::read(repo);
    let requests = !args.status && !args.dry_run;
    if requests {
        interrupt::install();
        crate::hostnet::clear_stop();
        crate::hostnet::reset();
        crate::hostnet::enable_gate(true);
    }
    let mut job_gap = BTreeMap::new();
    job_gap.insert("sec".to_string(), dataset::host_spec("sec").pause);
    let setup = RunSetup { repo, ctx: ctx.clone(), cfg: cfg.clone(), env: env.clone(), stop: interrupt::flag(), job_gap };
    let (keys, force) = (env.keys, args.force);
    let make = move |tickers: &[String], ledger: &Ledger| -> Box<dyn Runner> {
        Box::new(runner::RealRunner::new(ctx.clone(), cfg.clone(), keys, force, tickers.to_vec(), ledger.clone()))
    };
    let code = run_with(&args, &setup, &make, &mut |t| print!("{t}"), &|l| eprintln!("{l}"));
    if requests {
        crate::hostnet::enable_gate(false);
    }
    code
}

#[cfg(test)]
mod tests;
