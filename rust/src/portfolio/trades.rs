//! `fa portfolio trades`: the trade simulator of the trade-level backtest (ticket `T-20261121-trade-level-backtest`, part 2).
//!
//! A rule reads the panel row of the date `t` of one ticker (the close of the day before and the quantiles of the model)
//! and the daily bars from the day after `t` on, only to fill the trade. A trade enters at the OPEN of the first trading
//! day after `t` (the daily open of the price file, not an auction fill). It leaves at the first of: the target, the stop,
//! the time limit. Exits use the daily bars:
//!
//! - long: a gap of the open at or below the stop leaves at the open (reason `stop`); a gap of the open at or above the
//!   target leaves at the open (`target`); the day low at or below the stop leaves at the stop; the day high at or above
//!   the target leaves at the target. Short: the mirror (the high at or above the stop, the low at or below the target).
//! - a day that reaches both the stop and the target counts as the stop (the bars do not give the order: the adverse one).
//! - the entry day is checked too: the orders work from the open.
//! - the time limit is the close of the first trading day on or after `entry day + 7 * max_weeks` days (`time`).
//! - a trade whose target is already reached by the entry open (the signal is old) is not made (counted as `no_room`).
//! - a trade that has no exit before the data of the ticker ends is not in the result (counted as `unresolved`).
//! - one trade at a time for each ticker and rule: a new trade enters after the exit day of the last one.
//!
//! Return (on the entry price, `c` = cost_bp / 10,000 on each side): long `exit (1 - c) / (entry (1 + c)) - 1`; short
//! `(entry (1 - c) - exit (1 + c)) / entry - borrow_bp / 10,000 * days / 365` (`days` = calendar days held). The borrow
//! fee is an input (`--borrow-bp`, default 50 a year, an ASSUMPTION: the store has no borrow-fee data).
//!
//! The simulator is a function of the panel rows and the bars: no hidden state, the rules run in parallel.

use super::cli::{check_options, opt, write_new};
use super::harness::sha256_hex;
use super::panel::{check_fresh, read_panel, Header, PanelRow, Provenance};
use crate::dcf::stats::{percentile, sorted_copy};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa portfolio trades --panel PANEL --cache DIR --rules RULES --out TRADES [--summary FILE] [--borrow-bp N]
  The trade simulator. For each rule of the grid (a TOML list), each ticker, each panel date: a signal from the panel row of
  that date only; the trade enters at the OPEN of the first trading day after the panel date (the daily open of the price
  file, not an auction fill) and leaves at the first of the target, the stop and the time limit. The daily bars are read from
  the entry day on, only to fill the trade. Rules run in parallel. No network. A file is never replaced.
  --panel PANEL      the file of `fa portfolio panel`. It is refused if its code, store or split table is not the current one.
  --cache DIR        the data store, for the daily bars (default <repo>/data/raw)
  --rules RULES      TOML: one [[rule]] table for each rule, with the fields
                       id            text, optional (default r0001, r0002, ... in the order of the file)
                       side          \"long\", \"short\" or \"both\"
                       entry_long_q  \"P10\", \"P25\", \"P50\" or \"none\" (default): long if the close of the day before is at or
                                     below this quantile of the value
                       entry_short_q \"P50\", \"P75\", \"P90\" or \"none\" (default): short if the close is at or above it
                       target        \"p50\" (the P50 of the entry row), \"mean\" (the mean of the entry row) or a number: the
                                     move from the entry price, 0.15 = 15% (up for a long, down for a short)
                       stop          a number, the loss from the entry price (0.2 = 20%), or \"none\" (default)
                       max_weeks     the time limit in weeks from the entry day
                       cost_bp       cost on each side, in basis points
                       borrow_bp     the borrow fee of a short, basis points a year (default: --borrow-bp)
  --out TRADES       one row for each trade: rule, ticker, side, signal date, entry date and price, exit date and price,
                     reason (target, stop, time), days, return after cost (and the borrow fee). One header line, one footer line.
  --summary FILE     one row for each rule: the count, the mean return of a trade, the hit rate, the mean days, the return
                     for a year of capital (365 * the sum of the returns / the sum of the days), the P10 and the P5 of the
                     return of a trade, the signals not made (no_room) and the trades with no exit in the data (unresolved)
  --borrow-bp N (50) the borrow fee of a short, basis points a year. An ASSUMPTION (general collateral): the store has no
                     borrow-fee data. A rule can set its own.
  Limits: the daily bars only (a stop or a target that the day range touches is filled at its price: no queue, no slippage
  beyond the cost); price returns (no dividend, a short pays none); today's list of tickers (survivorship).";

/// A daily bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub day: i64,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Long,
    Short,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    P50,
    Mean,
    /// The move from the entry price, as a fraction.
    Pct(f64),
}

/// A rule of the grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub id: String,
    pub side: Side,
    /// The index in `q` (0 = P10 ... 4 = P90) of the long entry, if any.
    pub long_q: Option<usize>,
    pub short_q: Option<usize>,
    pub target: Target,
    pub stop: Option<f64>,
    pub max_weeks: u32,
    pub cost_bp: f64,
    pub borrow_bp: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRule {
    id: Option<String>,
    side: String,
    entry_long_q: Option<String>,
    entry_short_q: Option<String>,
    target: toml::Value,
    stop: Option<toml::Value>,
    max_weeks: u32,
    cost_bp: f64,
    borrow_bp: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    rule: Vec<RawRule>,
}

fn quantile_index(text: Option<&str>, allowed: &[usize], what: &str) -> Result<Option<usize>, String> {
    let Some(t) = text else { return Ok(None) };
    let i = match t.to_ascii_uppercase().as_str() {
        "NONE" => return Ok(None),
        "P10" => 0,
        "P25" => 1,
        "P50" => 2,
        "P75" => 3,
        "P90" => 4,
        _ => return Err(format!("{what} '{t}': P10, P25, P50, P75, P90 or none")),
    };
    if allowed.contains(&i) {
        Ok(Some(i))
    } else {
        Err(format!("{what} '{t}' is not allowed here"))
    }
}

/// Parse the TOML list of rules. `default_borrow_bp` fills a rule with no `borrow_bp`.
pub fn parse_rules(text: &str, default_borrow_bp: f64) -> Result<Vec<Rule>, String> {
    let raw: RawFile = toml::from_str(text).map_err(|e| format!("rules file: {e}"))?;
    let mut out: Vec<Rule> = Vec::new();
    for (k, r) in raw.rule.into_iter().enumerate() {
        let id = r.id.unwrap_or_else(|| format!("r{:04}", k + 1));
        if id.is_empty() || id.contains(|c: char| c == ',' || c.is_whitespace()) {
            return Err(format!("rule id '{id}': no comma and no space"));
        }
        if out.iter().any(|o| o.id == id) {
            return Err(format!("rule id '{id}' twice"));
        }
        let err = |m: &str| format!("rule {id}: {m}");
        let side = match r.side.as_str() {
            "long" => Side::Long,
            "short" => Side::Short,
            "both" => Side::Both,
            x => return Err(err(&format!("side '{x}': long, short or both"))),
        };
        let long_q = quantile_index(r.entry_long_q.as_deref(), &[0, 1, 2], "entry_long_q").map_err(|e| err(&e))?;
        let short_q = quantile_index(r.entry_short_q.as_deref(), &[2, 3, 4], "entry_short_q").map_err(|e| err(&e))?;
        if matches!(side, Side::Long | Side::Both) && long_q.is_none() {
            return Err(err("a long side needs entry_long_q"));
        }
        if matches!(side, Side::Short | Side::Both) && short_q.is_none() {
            return Err(err("a short side needs entry_short_q"));
        }
        if side == Side::Long && short_q.is_some() || side == Side::Short && long_q.is_some() {
            return Err(err("an entry quantile of the other side is set: it would be ignored"));
        }
        let target = match &r.target {
            toml::Value::String(s) if s.eq_ignore_ascii_case("p50") => Target::P50,
            toml::Value::String(s) if s.eq_ignore_ascii_case("mean") => Target::Mean,
            toml::Value::Float(x) if *x > 0.0 && x.is_finite() => Target::Pct(*x),
            toml::Value::Integer(x) if *x > 0 => Target::Pct(*x as f64),
            _ => return Err(err("target: \"p50\", \"mean\" or a positive number (the move as a fraction)")),
        };
        let stop = match &r.stop {
            None => None,
            Some(toml::Value::String(s)) if s.eq_ignore_ascii_case("none") => None,
            Some(toml::Value::Float(x)) if *x > 0.0 && x.is_finite() => Some(*x),
            Some(toml::Value::Integer(x)) if *x > 0 => Some(*x as f64),
            _ => return Err(err("stop: a positive number (the loss as a fraction) or \"none\"")),
        };
        if r.max_weeks == 0 {
            return Err(err("max_weeks is 1 or more"));
        }
        if !(0.0..=1000.0).contains(&r.cost_bp) {
            return Err(err("cost_bp is 0 to 1000"));
        }
        let borrow_bp = r.borrow_bp.unwrap_or(default_borrow_bp);
        if !(0.0..=10_000.0).contains(&borrow_bp) {
            return Err(err("borrow_bp is 0 to 10000"));
        }
        out.push(Rule { id, side, long_q, short_q, target, stop, max_weeks: r.max_weeks, cost_bp: r.cost_bp, borrow_bp });
    }
    if out.is_empty() {
        return Err("rules file: no [[rule]]".into());
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Target,
    Stop,
    Time,
}

impl Reason {
    pub fn name(self) -> &'static str {
        match self {
            Reason::Target => "target",
            Reason::Stop => "stop",
            Reason::Time => "time",
        }
    }
}

/// The exit of a trade that entered at the open of `bars[i0]`: the first of the stop, the target and the time limit, with
/// the rules of the module text. `target` and `stop` are prices (`stop` on the loss side of the entry). The index of the exit
/// bar, the exit price and the reason; `None` if the data ends first.
pub fn fill(bars: &[Bar], i0: usize, long: bool, target: f64, stop: Option<f64>, limit_day: i64) -> Option<(usize, f64, Reason)> {
    for (i, b) in bars.iter().enumerate().skip(i0) {
        if long {
            if i > i0 {
                if stop.is_some_and(|s| b.o <= s) {
                    return Some((i, b.o, Reason::Stop));
                }
                if b.o >= target {
                    return Some((i, b.o, Reason::Target));
                }
            }
            if let Some(s) = stop.filter(|s| b.l <= *s) {
                return Some((i, s, Reason::Stop));
            }
            if b.h >= target {
                return Some((i, target, Reason::Target));
            }
        } else {
            if i > i0 {
                if stop.is_some_and(|s| b.o >= s) {
                    return Some((i, b.o, Reason::Stop));
                }
                if b.o <= target {
                    return Some((i, b.o, Reason::Target));
                }
            }
            if let Some(s) = stop.filter(|s| b.h >= *s) {
                return Some((i, s, Reason::Stop));
            }
            if b.l <= target {
                return Some((i, target, Reason::Target));
            }
        }
        if b.day >= limit_day {
            return Some((i, b.c, Reason::Time));
        }
    }
    None
}

/// The return of a trade after the cost on each side and, for a short, the borrow fee for the calendar days held.
pub fn trade_return(long: bool, entry: f64, exit: f64, cost_bp: f64, borrow_bp: f64, days: i64) -> f64 {
    let c = cost_bp / 1e4;
    if long {
        exit * (1.0 - c) / (entry * (1.0 + c)) - 1.0
    } else {
        (entry * (1.0 - c) - exit * (1.0 + c)) / entry - borrow_bp / 1e4 * days as f64 / 365.0
    }
}

/// One ticker: its panel rows (by date) and its bars (by day), and for each row the first bar after the panel date.
#[derive(Debug, Clone)]
pub struct TickerData {
    pub ticker: String,
    pub bars: Vec<Bar>,
    pub rows: Vec<PanelRow>,
    /// The index of the bar of the entry (the first trading day after the panel date, at most 7 days later), if any.
    pub entry: Vec<Option<usize>>,
}

/// A bar is usable if the four prices are finite and above 0.
fn good(b: &Bar) -> bool {
    [b.o, b.h, b.l, b.c].iter().all(|x| x.is_finite() && *x > 0.0)
}

/// Group the rows by ticker and bind the bars. A ticker with no bar is left out.
pub fn build_data(rows: Vec<PanelRow>, bars: &BTreeMap<String, Vec<Bar>>) -> Vec<TickerData> {
    let mut by: BTreeMap<String, Vec<PanelRow>> = BTreeMap::new();
    for r in rows {
        by.entry(r.ticker.clone()).or_default().push(r);
    }
    by.into_iter()
        .filter_map(|(ticker, mut rows)| {
            let b: Vec<Bar> = bars.get(&ticker)?.iter().copied().filter(good).collect();
            if b.is_empty() {
                return None;
            }
            rows.sort_by_key(|r| r.date);
            let entry = rows
                .iter()
                .map(|r| {
                    let i = b.partition_point(|x| x.day <= r.date);
                    b.get(i).filter(|x| x.day - r.date <= 7).map(|_| i)
                })
                .collect();
            Some(TickerData { ticker, bars: b, rows, entry })
        })
        .collect()
}

/// One trade.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeRow {
    /// The index in the list of `TickerData`.
    pub ticker: usize,
    pub long: bool,
    pub signal: i64,
    pub entry_day: i64,
    pub entry_px: f64,
    pub exit_day: i64,
    pub exit_px: f64,
    pub reason: Reason,
    pub ret: f64,
}

/// The trades of one rule over all tickers, and the counts of the signals not made.
#[derive(Debug, Clone, Default)]
pub struct RuleRun {
    pub trades: Vec<TradeRow>,
    pub no_room: usize,
    pub unresolved: usize,
}

/// Run one rule. Reads, for the row of date `t`, the row itself and the bars from the entry day on.
pub fn simulate_rule(rule: &Rule, data: &[TickerData]) -> RuleRun {
    let mut run = RuleRun::default();
    for (ti, d) in data.iter().enumerate() {
        let mut busy_until = i64::MIN; // the exit day of the last trade of this ticker
        for (k, row) in d.rows.iter().enumerate() {
            let Some(i0) = d.entry[k] else { continue };
            let entry_bar = d.bars[i0];
            if entry_bar.day <= busy_until {
                continue;
            }
            let long = match rule.side {
                Side::Long => rule.long_q.is_some_and(|q| row.close <= row.q[q]),
                Side::Short => false,
                Side::Both => rule.long_q.is_some_and(|q| row.close <= row.q[q]),
            };
            let short = !long && matches!(rule.side, Side::Short | Side::Both) && rule.short_q.is_some_and(|q| row.close >= row.q[q]);
            if !long && !short {
                continue;
            }
            let entry = entry_bar.o;
            let target = match rule.target {
                Target::P50 => row.q[2],
                Target::Mean => row.mean,
                Target::Pct(x) => {
                    if long {
                        entry * (1.0 + x)
                    } else {
                        entry * (1.0 - x)
                    }
                }
            };
            if !target.is_finite() || target <= 0.0 {
                continue;
            }
            if (long && entry >= target) || (short && entry <= target) {
                run.no_room += 1;
                continue;
            }
            let stop = rule.stop.map(|s| if long { entry * (1.0 - s) } else { entry * (1.0 + s) });
            let limit = entry_bar.day + 7 * i64::from(rule.max_weeks);
            let Some((j, px, reason)) = fill(&d.bars, i0, long, target, stop, limit) else {
                run.unresolved += 1;
                continue;
            };
            let exit_day = d.bars[j].day;
            busy_until = exit_day;
            run.trades.push(TradeRow {
                ticker: ti,
                long,
                signal: row.date,
                entry_day: entry_bar.day,
                entry_px: entry,
                exit_day,
                exit_px: px,
                reason,
                ret: trade_return(long, entry, px, rule.cost_bp, rule.borrow_bp, exit_day - entry_bar.day),
            });
        }
    }
    run
}

/// The figures of one rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub n: usize,
    pub mean: f64,
    pub hit: f64,
    pub mean_days: f64,
    /// 365 * the sum of the returns / the sum of the days held: the return for a year of capital.
    pub per_year: f64,
    pub p10: f64,
    pub p5: f64,
    pub no_room: usize,
    pub unresolved: usize,
}

pub fn summarize(run: &RuleRun) -> Stats {
    let n = run.trades.len();
    let nan = f64::NAN;
    if n == 0 {
        return Stats { n, mean: nan, hit: nan, mean_days: nan, per_year: nan, p10: nan, p5: nan, no_room: run.no_room, unresolved: run.unresolved };
    }
    let r: Vec<f64> = run.trades.iter().map(|t| t.ret).collect();
    let sum: f64 = r.iter().sum();
    let days: f64 = run.trades.iter().map(|t| (t.exit_day - t.entry_day) as f64).sum();
    let s = sorted_copy(&r);
    Stats {
        n,
        mean: sum / n as f64,
        hit: r.iter().filter(|x| **x > 0.0).count() as f64 / n as f64,
        mean_days: days / n as f64,
        per_year: if days > 0.0 { 365.0 * sum / days } else { nan },
        p10: percentile(&s, 10.0),
        p5: percentile(&s, 5.0),
        no_room: run.no_room,
        unresolved: run.unresolved,
    }
}

/// The text of a day (a table for the days of the bars: the format of a day is slow for millions of rows).
struct Days {
    base: i64,
    text: Vec<String>,
}

impl Days {
    fn new(lo: i64, hi: i64) -> Days {
        Days { base: lo, text: (lo..=hi).map(mapcheck::dates::format).collect() }
    }
    fn get(&self, d: i64) -> &str {
        self.text.get((d - self.base) as usize).map_or("", String::as_str)
    }
}

const TRADE_COLUMNS: &str = "rule,ticker,side,signal_date,entry_date,entry_price,exit_date,exit_price,reason,days,ret";

fn rule_csv(rule: &Rule, run: &RuleRun, data: &[TickerData], days: &Days) -> String {
    let mut s = String::with_capacity(run.trades.len() * 100);
    for t in &run.trades {
        let _ = writeln!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{}",
            rule.id,
            data[t.ticker].ticker,
            if t.long { "long" } else { "short" },
            days.get(t.signal),
            days.get(t.entry_day),
            t.entry_px,
            days.get(t.exit_day),
            t.exit_px,
            t.reason.name(),
            t.exit_day - t.entry_day,
            t.ret
        );
    }
    s
}

pub const SUMMARY_COLUMNS: &str = "rule,trades,mean_return,hit_rate,mean_days,return_per_capital_year,p10_return,p5_return,no_room,unresolved";

fn summary_line(id: &str, s: &Stats) -> String {
    format!("{id},{},{},{},{},{},{},{},{},{}", s.n, s.mean, s.hit, s.mean_days, s.per_year, s.p10, s.p5, s.no_room, s.unresolved)
}

/// Run the grid: the rules in parallel, in chunks (the trade text of a chunk is written, then dropped). Returns the stats of
/// each rule in the order of the rules, and the number of trades. `sink` gets the CSV text of each rule in the order of the rules.
pub fn run_grid(rules: &[Rule], data: &[TickerData], mut sink: impl FnMut(&str) -> Result<(), String>) -> Result<(Vec<Stats>, usize), String> {
    let (lo, hi) = data.iter().flat_map(|d| d.bars.iter().map(|b| b.day)).fold((i64::MAX, i64::MIN), |(a, b), x| (a.min(x), b.max(x)));
    let days = if lo <= hi { Days::new(lo, hi) } else { Days::new(0, 0) };
    let mut stats = Vec::with_capacity(rules.len());
    let mut n_trades = 0;
    for chunk in rules.chunks(32) {
        let done: Vec<(String, Stats)> = chunk
            .par_iter()
            .map(|r| {
                let run = simulate_rule(r, data);
                (rule_csv(r, &run, data, &days), summarize(&run))
            })
            .collect();
        for (text, st) in done {
            sink(&text)?;
            n_trades += st.n;
            stats.push(st);
        }
    }
    Ok((stats, n_trades))
}

/// The daily bars of the tickers from the cache, by ticker (a ticker with no file is not in the map).
pub fn load_bars(cache: &Path, tickers: &[String]) -> BTreeMap<String, Vec<Bar>> {
    tickers
        .par_iter()
        .filter_map(|t| {
            let p = crate::market::prices::read_cache(&crate::market::prices::cache_path(cache, t)).ok()?;
            let mut v: Vec<Bar> = p.into_iter().filter_map(|x| Some(Bar { day: mapcheck::dates::parse(x.date.get(..10)?)?, o: x.open, h: x.high, l: x.low, c: x.close })).collect();
            v.sort_by_key(|b| b.day);
            Some((t.clone(), v))
        })
        .collect()
}

const KNOWN: [&str; 6] = ["--panel", "--cache", "--rules", "--out", "--summary", "--borrow-bp"];

/// `fa portfolio trades`.
pub fn command(repo: &Path, args: &[String]) -> Result<(), String> {
    check_options(args, &KNOWN)?;
    super::cli::refuse_existing(args, &["--out", "--summary"])?;
    let panel_file = opt(args, "--panel").ok_or("--panel is needed")?;
    let rules_file = opt(args, "--rules").ok_or("--rules is needed")?;
    let out_file = opt(args, "--out").ok_or("--out is needed")?;
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let borrow: f64 = opt(args, "--borrow-bp").map_or(Ok(50.0), |v| v.parse().map_err(|_| format!("--borrow-bp '{v}': a number")))?;
    let t0 = std::time::Instant::now();
    let rules_text = std::fs::read_to_string(rules_file).map_err(|e| format!("{rules_file}: {e}"))?;
    let rules = parse_rules(&rules_text, borrow)?;
    let panel_bytes = std::fs::read(panel_file).map_err(|e| format!("{panel_file}: {e}"))?;
    let (ph, rows) = read_panel(&String::from_utf8_lossy(&panel_bytes))?;
    let prov = Provenance::current(repo, &cache);
    check_fresh(&ph, &prov)?;
    let tickers: Vec<String> = {
        let mut v: Vec<String> = rows.iter().map(|r| r.ticker.clone()).collect();
        v.sort();
        v.dedup();
        v
    };
    let bars = load_bars(&cache, &tickers);
    let data = build_data(rows, &bars);
    let t_load = t0.elapsed().as_secs_f64();

    let mut h = Header::default();
    prov.put(&mut h);
    for k in ["start", "end", "paths", "seed", "tickers_sha256", "table_sha256"] {
        h.0.insert(format!("panel_{k}"), ph.get(k).to_string());
    }
    for (k, v) in [
        ("panel_sha256", sha256_hex(&panel_bytes)),
        ("rules_sha256", sha256_hex(rules_text.as_bytes())),
        ("n_rules", rules.len().to_string()),
        ("borrow_bp_default", borrow.to_string()),
    ] {
        h.0.insert(k.to_string(), v);
    }
    let mut f = std::io::BufWriter::new(std::fs::OpenOptions::new().write(true).create_new(true).open(out_file).map_err(|e| format!("{out_file}: {e} (the command does not replace a file)"))?);
    let io = |e: std::io::Error| format!("{out_file}: {e}");
    let ids: Vec<&str> = rules.iter().map(|r| r.id.as_str()).collect();
    writeln!(f, "{}\n# rule_ids {}\n{TRADE_COLUMNS}", h.line("fa-trades"), ids.join(",")).map_err(io)?;
    let t1 = std::time::Instant::now();
    let (stats, n_trades) = run_grid(&rules, &data, |t| f.write_all(t.as_bytes()).map_err(io))?;
    writeln!(f, "# end n_trades={n_trades}").map_err(io)?;
    f.flush().map_err(io)?;
    let secs = t1.elapsed().as_secs_f64();

    let mut summary = format!("{SUMMARY_COLUMNS}\n");
    for (r, s) in rules.iter().zip(&stats) {
        summary.push_str(&summary_line(&r.id, s));
        summary.push('\n');
    }
    if let Some(p) = opt(args, "--summary") {
        write_new(p, &summary)?;
    }
    println!(
        "trades written: {out_file} ({n_trades} trades of {} rules over {} tickers and {} panel rows). Panel and bars loaded in {t_load:.1} s; the rules ran and were written in {secs:.1} s.",
        rules.len(),
        data.len(),
        data.iter().map(|d| d.rows.len()).sum::<usize>()
    );
    println!("borrow fee of a short: {borrow} bp a year unless the rule sets its own (an ASSUMPTION: the store has no borrow-fee data). Entry at the daily open, not an auction fill. Today's list of tickers (survivorship).");
    let mut best: Vec<(usize, f64)> = stats.iter().enumerate().filter(|(_, s)| s.n > 0).map(|(k, s)| (k, s.mean)).collect();
    best.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("rules with the best mean return of a trade over the WHOLE period (a description, chosen with later data: not a result; the result is the walk-forward):");
    for (k, _) in best.iter().take(5) {
        println!("  {}", summary_line(&rules[*k].id, &stats[*k]));
    }
    if prov.code_dirty {
        println!("WARNING: the code has changes that are not committed (code_dirty=true in the header).");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> i64 {
        mapcheck::dates::parse(s).unwrap()
    }

    fn bar(day: &str, o: f64, h: f64, l: f64, c: f64) -> Bar {
        Bar { day: d(day), o, h, l, c }
    }

    fn row(date: &str, close: f64) -> PanelRow {
        PanelRow { date: d(date), ticker: "X".into(), close_day: d(date) - 1, close, q: [50.0, 80.0, 100.0, 120.0, 150.0], mean: 105.0, accession: String::new(), sector: "S".into() }
    }

    fn rule(side: Side, target: Target, stop: Option<f64>, weeks: u32) -> Rule {
        Rule {
            id: "r".into(),
            side,
            long_q: matches!(side, Side::Long | Side::Both).then_some(1),
            short_q: matches!(side, Side::Short | Side::Both).then_some(3),
            target,
            stop,
            max_weeks: weeks,
            cost_bp: 0.0,
            borrow_bp: 0.0,
        }
    }

    fn data(rows: Vec<PanelRow>, bars: Vec<Bar>) -> Vec<TickerData> {
        build_data(rows, &BTreeMap::from([("X".to_string(), bars)]))
    }

    /// Friday 2020-01-03 is the panel date (close 90, at or below P25 = 80? no: P25 is 80, so the long tests use close 70).
    fn long_row() -> PanelRow {
        row("2020-01-03", 70.0)
    }

    /// Bars from 2020-01-02: the days before the entry carry other opens, so a wrong entry shows.
    fn bars_with(tail: Vec<Bar>) -> Vec<Bar> {
        let mut v = vec![bar("2020-01-02", 71.0, 72.0, 70.0, 71.0), bar("2020-01-03", 72.0, 73.0, 71.0, 72.0)];
        v.extend(tail);
        v
    }

    fn one(r: &Rule, rows: Vec<PanelRow>, bars: Vec<Bar>) -> RuleRun {
        simulate_rule(r, &data(rows, bars))
    }

    #[test]
    fn the_entry_is_the_open_of_the_first_trading_day_after_the_panel_date() {
        // the panel date is a Friday; the bars of Thursday and Friday have other opens; Monday 2020-01-06 opens at 88
        let bars = bars_with(vec![bar("2020-01-06", 88.0, 95.0, 87.0, 90.0), bar("2020-01-07", 91.0, 101.0, 90.0, 99.0)]);
        let run = one(&rule(Side::Long, Target::P50, None, 1), vec![long_row()], bars);
        let t = &run.trades[0];
        assert_eq!((t.signal, t.entry_day, t.entry_px), (d("2020-01-03"), d("2020-01-06"), 88.0));
        // the target is the P50 of the entry row (100): the high of Tuesday reaches it, the fill is at the target
        assert_eq!((t.exit_day, t.exit_px, t.reason), (d("2020-01-07"), 100.0, Reason::Target));
        assert!((t.ret - (100.0 / 88.0 - 1.0)).abs() < 1e-15);
    }

    #[test]
    fn the_entry_needs_a_bar_within_7_days() {
        let bars = vec![bar("2020-01-02", 71.0, 72.0, 70.0, 71.0), bar("2020-01-20", 88.0, 101.0, 87.0, 99.0)];
        assert!(one(&rule(Side::Long, Target::P50, None, 1), vec![long_row()], bars).trades.is_empty());
    }

    #[test]
    fn a_long_target_gap_fills_at_the_open_not_at_the_target() {
        let bars = bars_with(vec![bar("2020-01-06", 88.0, 95.0, 87.0, 90.0), bar("2020-01-07", 104.0, 110.0, 103.0, 108.0)]);
        let t = one(&rule(Side::Long, Target::P50, None, 1), vec![long_row()], bars).trades.remove(0);
        assert_eq!((t.exit_px, t.reason), (104.0, Reason::Target));
    }

    #[test]
    fn a_long_stop_fills_at_the_stop_inside_the_day_and_at_the_open_on_a_gap() {
        let r = rule(Side::Long, Target::P50, Some(0.10), 1); // the stop is 88 * 0.9 = 79.2
        let inside = bars_with(vec![bar("2020-01-06", 88.0, 95.0, 87.0, 90.0), bar("2020-01-07", 85.0, 90.0, 79.0, 80.0)]);
        let t = one(&r, vec![long_row()], inside).trades.remove(0);
        assert_eq!((t.exit_day, t.reason), (d("2020-01-07"), Reason::Stop));
        assert!((t.exit_px - 79.2).abs() < 1e-12);
        let gap = bars_with(vec![bar("2020-01-06", 88.0, 95.0, 87.0, 90.0), bar("2020-01-07", 75.0, 80.0, 74.0, 78.0)]);
        let t = one(&r, vec![long_row()], gap).trades.remove(0);
        assert_eq!((t.exit_px, t.reason), (75.0, Reason::Stop)); // worse than the stop price
    }

    #[test]
    fn a_day_that_reaches_the_stop_and_the_target_counts_as_the_stop() {
        let r = rule(Side::Long, Target::P50, Some(0.10), 1);
        let bars = bars_with(vec![bar("2020-01-06", 88.0, 101.0, 79.0, 90.0)]); // the entry day itself reaches both
        let t = one(&r, vec![long_row()], bars).trades.remove(0);
        assert_eq!((t.exit_day, t.reason), (d("2020-01-06"), Reason::Stop));
    }

    #[test]
    fn the_time_limit_is_the_close_of_the_first_day_on_or_after_entry_plus_the_weeks() {
        // entry 2020-01-06, one week: the limit is 2020-01-13, no stop and no target is reached before
        let tail: Vec<Bar> = (6..=15).map(|k| bar(&format!("2020-01-{k:02}"), 88.0, 90.0, 87.0, 88.0 + k as f64 * 0.1)).collect();
        let t = one(&rule(Side::Long, Target::P50, None, 1), vec![long_row()], bars_with(tail)).trades.remove(0);
        assert_eq!((t.exit_day, t.reason), (d("2020-01-13"), Reason::Time));
        assert!((t.exit_px - 89.3).abs() < 1e-12);
        // a limit that falls on a day with no bar: the next bar (the Monday after the Saturday)
        let tail = vec![bar("2020-01-06", 88.0, 90.0, 87.0, 88.0), bar("2020-01-10", 88.0, 90.0, 87.0, 89.0), bar("2020-01-14", 88.0, 90.0, 87.0, 87.5)];
        let t = one(&rule(Side::Long, Target::P50, None, 1), vec![long_row()], bars_with(tail)).trades.remove(0);
        assert_eq!((t.exit_day, t.exit_px), (d("2020-01-14"), 87.5));
    }

    #[test]
    fn a_trade_with_no_exit_before_the_data_ends_is_not_made() {
        let tail = vec![bar("2020-01-06", 88.0, 90.0, 87.0, 88.0), bar("2020-01-07", 88.0, 90.0, 87.0, 89.0)];
        let run = one(&rule(Side::Long, Target::P50, None, 1), vec![long_row()], bars_with(tail));
        assert_eq!((run.trades.len(), run.unresolved), (0, 1));
    }

    #[test]
    fn a_target_that_the_entry_open_has_passed_is_not_traded() {
        let bars = bars_with(vec![bar("2020-01-06", 101.0, 105.0, 100.0, 103.0)]);
        let run = one(&rule(Side::Long, Target::P50, None, 1), vec![long_row()], bars);
        assert_eq!((run.trades.len(), run.no_room), (0, 1));
    }

    #[test]
    fn the_short_side_mirrors_the_long_side() {
        // close 130 is at or above P75 = 120; the target is the P50 = 100; the stop 10% above the entry open 131 is 144.1
        let r = rule(Side::Short, Target::P50, Some(0.10), 2);
        let row = row("2020-01-03", 130.0);
        let hit = bars_with(vec![bar("2020-01-06", 131.0, 135.0, 128.0, 130.0), bar("2020-01-07", 120.0, 121.0, 99.0, 105.0)]);
        let t = one(&r, vec![row.clone()], hit).trades.remove(0);
        assert_eq!((t.long, t.entry_px, t.exit_px, t.reason), (false, 131.0, 100.0, Reason::Target));
        let gap = bars_with(vec![bar("2020-01-06", 131.0, 135.0, 128.0, 130.0), bar("2020-01-07", 97.0, 98.0, 95.0, 96.0)]);
        let t = one(&r, vec![row.clone()], gap).trades.remove(0);
        assert_eq!((t.exit_px, t.reason), (97.0, Reason::Target));
        let stop = bars_with(vec![bar("2020-01-06", 131.0, 135.0, 128.0, 130.0), bar("2020-01-07", 135.0, 150.0, 134.0, 149.0)]);
        let t = one(&r, vec![row.clone()], stop).trades.remove(0);
        assert_eq!(t.reason, Reason::Stop);
        assert!((t.exit_px - 144.1).abs() < 1e-12);
        let gap_up = bars_with(vec![bar("2020-01-06", 131.0, 135.0, 128.0, 130.0), bar("2020-01-07", 150.0, 155.0, 149.0, 152.0)]);
        let t = one(&r, vec![row], gap_up).trades.remove(0);
        assert_eq!((t.exit_px, t.reason), (150.0, Reason::Stop));
    }

    #[test]
    fn a_percent_target_moves_from_the_entry_price() {
        // long +10% from 88 = 96.8; short -10% from 131 = 117.9
        let bars = bars_with(vec![bar("2020-01-06", 88.0, 95.0, 87.0, 90.0), bar("2020-01-07", 91.0, 97.0, 90.0, 96.0)]);
        let t = one(&rule(Side::Long, Target::Pct(0.10), None, 1), vec![long_row()], bars).trades.remove(0);
        assert!((t.exit_px - 96.8).abs() < 1e-12 && t.reason == Reason::Target);
        let bars = bars_with(vec![bar("2020-01-06", 131.0, 135.0, 128.0, 130.0), bar("2020-01-07", 125.0, 126.0, 117.0, 118.0)]);
        let t = one(&rule(Side::Short, Target::Pct(0.10), None, 1), vec![row("2020-01-03", 130.0)], bars).trades.remove(0);
        assert!((t.exit_px - 117.9).abs() < 1e-12 && t.reason == Reason::Target);
    }

    #[test]
    fn the_mean_is_a_target_and_a_missing_mean_makes_no_trade() {
        let bars = bars_with(vec![bar("2020-01-06", 88.0, 95.0, 87.0, 90.0), bar("2020-01-07", 91.0, 106.0, 90.0, 96.0)]);
        let t = one(&rule(Side::Long, Target::Mean, None, 1), vec![long_row()], bars.clone()).trades.remove(0);
        assert_eq!(t.exit_px, 105.0);
        let mut nan = long_row();
        nan.mean = f64::NAN;
        assert!(one(&rule(Side::Long, Target::Mean, None, 1), vec![nan], bars).trades.is_empty());
    }

    #[test]
    fn the_costs_and_the_borrow_fee_have_fixed_answers() {
        // long: buy at 100 (cost 10 bp), sell at 110 (cost 10 bp)
        let r = trade_return(true, 100.0, 110.0, 10.0, 999.0, 30);
        assert!((r - (110.0 * 0.999 / (100.0 * 1.001) - 1.0)).abs() < 1e-15);
        assert!((r - 0.097_802_197_8).abs() < 1e-9);
        // short: sell at 100, cover at 90, 10 bp each side, 50 bp a year for 73 days (0.2 of a year): 0.2 * 0.005 = 0.001
        let r = trade_return(false, 100.0, 90.0, 10.0, 50.0, 73);
        assert!((r - ((100.0 * 0.999 - 90.0 * 1.001) / 100.0 - 0.001)).abs() < 1e-12);
        assert!((r - 0.0971).abs() < 1e-12);
        // the fee grows with the days; a long pays none
        assert!(trade_return(false, 100.0, 100.0, 0.0, 50.0, 365) + 0.005 < 1e-15);
        assert_eq!(trade_return(true, 100.0, 100.0, 0.0, 500.0, 365), 0.0);
        // through the simulator: the rule cost and the borrow fee of the rule
        let mut rl = rule(Side::Short, Target::P50, None, 2);
        rl.cost_bp = 10.0;
        rl.borrow_bp = 50.0;
        let bars = bars_with(vec![bar("2020-01-06", 131.0, 135.0, 128.0, 130.0), bar("2020-01-07", 120.0, 121.0, 99.0, 105.0)]);
        let t = one(&rl, vec![row("2020-01-03", 130.0)], bars).trades.remove(0);
        assert!((t.ret - trade_return(false, 131.0, 100.0, 10.0, 50.0, 1)).abs() < 1e-15);
    }

    #[test]
    fn one_trade_at_a_time_for_each_ticker_and_rule() {
        // flat bars on each calendar day of January and February: only the time limit (2 weeks) can end a trade
        let tail: Vec<Bar> = (6..=31).map(|k| bar(&format!("2020-01-{k:02}"), 88.0, 90.0, 87.0, 88.0)).chain((1..=20).map(|k| bar(&format!("2020-02-{k:02}"), 88.0, 90.0, 87.0, 88.0))).collect();
        let rows = ["2020-01-03", "2020-01-10", "2020-01-17", "2020-01-24"].map(|x| row(x, 70.0)).to_vec();
        let run = one(&rule(Side::Long, Target::P50, None, 2), rows, bars_with(tail));
        // the first trade: entry 01-06, exit 01-20. The signals of 01-10 (entry 01-13) and 01-17 (entry 01-20) enter at or before
        // the exit day: not made. The signal of 01-24 enters on 01-27.
        let signals: Vec<i64> = run.trades.iter().map(|t| t.signal).collect();
        assert_eq!(signals, [d("2020-01-03"), d("2020-01-24")]);
        assert_eq!(run.trades[0].exit_day, d("2020-01-20"));
    }

    #[test]
    fn the_signal_uses_the_panel_row_and_the_trade_reads_only_the_bars_from_the_entry_day() {
        let tail: Vec<Bar> = (6..=31).map(|k| bar(&format!("2020-01-{k:02}"), 88.0, 90.0, 87.0, 88.0)).chain((1..=20).map(|k| bar(&format!("2020-02-{k:02}"), 88.0, 90.0, 87.0, 88.0))).collect();
        let rows = vec![row("2020-01-03", 70.0), row("2020-01-24", 70.0)];
        let r = rule(Side::Long, Target::P50, None, 2);
        let base = one(&r, rows.clone(), bars_with(tail.clone()));
        // the bars before the entry day of the second trade (2020-01-27) and the rows after the first trade change: the first trade stays
        let mut bars2 = bars_with(tail);
        for b in bars2.iter_mut().filter(|b| b.day < d("2020-01-06")) {
            b.o *= 7.0;
            b.h *= 7.0;
            b.l *= 7.0;
            b.c *= 7.0;
        }
        let mut rows2 = rows;
        rows2[1].close = 500.0; // the second row no longer signals
        let other = one(&r, rows2, bars2);
        assert_eq!(base.trades[0], other.trades[0]);
        assert_eq!(base.trades.len(), 2);
        assert_eq!(other.trades.len(), 1);
    }

    #[test]
    fn the_side_both_takes_the_long_first_and_a_row_between_the_quantiles_gives_no_trade() {
        let bars = bars_with(vec![bar("2020-01-06", 88.0, 95.0, 87.0, 90.0), bar("2020-01-07", 91.0, 101.0, 90.0, 99.0)]);
        let both = rule(Side::Both, Target::P50, None, 1);
        assert!(one(&both, vec![row("2020-01-03", 100.0)], bars.clone()).trades.is_empty()); // 80 < 100 < 120
        let run = one(&both, vec![row("2020-01-03", 70.0)], bars.clone());
        assert_eq!(run.trades.len(), 1);
        assert!(run.trades[0].long);
        let run = one(&both, vec![row("2020-01-03", 130.0)], bars);
        // 130 is at or above P75: a short, never a long; the entry open 88 is below the target 100 (no room for a short)
        assert_eq!((run.trades.len(), run.no_room), (0, 1));
    }

    #[test]
    fn the_rules_file_has_checks() {
        let ok = "[[rule]]\nside = \"long\"\nentry_long_q = \"P25\"\ntarget = \"p50\"\nmax_weeks = 13\ncost_bp = 10\n";
        let r = parse_rules(ok, 50.0).unwrap();
        assert_eq!((r[0].id.as_str(), r[0].long_q, r[0].stop, r[0].borrow_bp), ("r0001", Some(1), None, 50.0));
        let bad = |s: &str| parse_rules(s, 50.0).unwrap_err();
        assert!(bad(&ok.replace("P25", "P75")).contains("not allowed"));
        assert!(bad(&ok.replace("long\"\nentry", "short\"\nentry")).contains("entry_short_q"));
        assert!(bad(&format!("{ok}stop = -1\n")).contains("stop"));
        assert!(bad(&format!("{ok}colour = 1\n")).contains("unknown field"));
        let two = format!("{ok}id = \"a\"\n[[rule]]\nid = \"a\"\nside = \"long\"\nentry_long_q = \"P10\"\ntarget = 0.2\nmax_weeks = 4\ncost_bp = 5\n");
        assert!(bad(&two).contains("twice"));
        assert!(bad("").contains("rule"));
    }

    #[test]
    fn the_summary_has_fixed_answers() {
        let t = |ret: f64, days: i64| TradeRow { ticker: 0, long: true, signal: 0, entry_day: 0, entry_px: 1.0, exit_day: days, exit_px: 1.0, reason: Reason::Time, ret };
        let run = RuleRun { trades: vec![t(0.10, 10), t(-0.20, 20), t(0.30, 30), t(0.0, 40), t(0.05, 50)], no_room: 2, unresolved: 1 };
        let s = summarize(&run);
        assert_eq!(s.n, 5);
        assert!((s.mean - 0.05).abs() < 1e-15);
        assert!((s.hit - 0.6).abs() < 1e-15); // 0.0 is not a win
        assert!((s.mean_days - 30.0).abs() < 1e-15);
        assert!((s.per_year - 365.0 * 0.25 / 150.0).abs() < 1e-12);
        // sorted: -0.2, 0, 0.05, 0.1, 0.3; P10 at index 0.4, P5 at index 0.2 (linear)
        assert!((s.p10 - (-0.2 + 0.4 * 0.2)).abs() < 1e-12);
        assert!((s.p5 - (-0.2 + 0.2 * 0.2)).abs() < 1e-12);
        assert_eq!((s.no_room, s.unresolved), (2, 1));
        assert!(summarize(&RuleRun::default()).mean.is_nan());
    }

    #[test]
    fn the_trades_text_is_read_back_by_the_walk_forward() {
        let tail: Vec<Bar> = (6..=31).map(|k| bar(&format!("2020-01-{k:02}"), 88.0, 90.0, 87.0, 88.0)).collect();
        let dt = data(vec![row("2020-01-03", 70.0), row("2020-01-24", 70.0)], bars_with(tail));
        let rules = [rule(Side::Long, Target::P50, None, 2), Rule { id: "q".into(), ..rule(Side::Long, Target::Pct(0.01), None, 2) }];
        let mut body = String::new();
        let (stats, n) = run_grid(&rules, &dt, |t| {
            body.push_str(t);
            Ok(())
        })
        .unwrap();
        assert_eq!((stats.len(), n), (2, stats[0].n + stats[1].n));
        let text = format!("# fa-trades v1 code=x n_rules=2\n# rule_ids r,q\n{TRADE_COLUMNS}\n{body}# end n_trades={n}\n");
        let f = super::super::walkforward::read_trades(&text).unwrap();
        assert_eq!((f.trades.len(), f.rule_ids.len()), (n, 2));
        assert_eq!(f.trades[0].signal, d("2020-01-03"));
        // a cut file has no footer, a wrong count is refused
        assert!(super::super::walkforward::read_trades(&text.replace(&format!("# end n_trades={n}\n"), "")).is_err());
        assert!(super::super::walkforward::read_trades(&text.replace(&format!("n_trades={n}"), "n_trades=99")).is_err());
    }

    /// The grid of the speed test: 500 rules.
    fn grid_of_500() -> Vec<Rule> {
        let mut v = Vec::new();
        for side in [Side::Long, Side::Short, Side::Both] {
            for lq in [0usize, 1, 2] {
                for sq in [2usize, 3, 4] {
                    for target in [Target::P50, Target::Mean, Target::Pct(0.1), Target::Pct(0.2)] {
                        for stop in [None, Some(0.15)] {
                            for weeks in [4u32, 13, 26] {
                                if side == Side::Long && sq != 2 || side == Side::Short && lq != 0 {
                                    continue;
                                }
                                v.push(Rule {
                                    id: format!("g{}", v.len()),
                                    side,
                                    long_q: (side != Side::Short).then_some(lq),
                                    short_q: (side != Side::Long).then_some(sq),
                                    target,
                                    stop,
                                    max_weeks: weeks,
                                    cost_bp: 10.0,
                                    borrow_bp: 50.0,
                                });
                                let last = v.len() - 1;
                                v.push(Rule { id: format!("g{}", v.len()), cost_bp: 5.0, ..v[last].clone() });
                            }
                        }
                    }
                }
            }
        }
        v.truncate(500);
        v
    }

    /// The speed target: 500 rules on a panel of the full size (430 tickers, 460 weeks, 2018 to 2026), in memory.
    /// Run: `cargo test --release --manifest-path rust/Cargo.toml speed_of_500_rules -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn speed_of_500_rules_on_a_full_size_synthetic_panel() {
        let (n_tickers, n_weeks) = (430usize, 460usize);
        let start = d("2018-01-05");
        let mut rows = Vec::new();
        let mut bars: BTreeMap<String, Vec<Bar>> = BTreeMap::new();
        let mut x: u64 = 12345;
        let mut rnd = || {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((x >> 33) as f64) / (1u64 << 31) as f64
        };
        for k in 0..n_tickers {
            let name = format!("T{k:03}");
            let mut px = 50.0 + rnd() * 100.0;
            let mut v = Vec::new();
            for day in (start - 10)..(start + 7 * n_weeks as i64 + 400) {
                if (day + 3).rem_euclid(7) >= 5 {
                    continue; // no weekend
                }
                let o = px * (1.0 + 0.01 * (rnd() - 0.5));
                let c = o * (1.0 + 0.03 * (rnd() - 0.5));
                v.push(Bar { day, o, h: o.max(c) * (1.0 + 0.01 * rnd()), l: o.min(c) * (1.0 - 0.01 * rnd()), c });
                px = c.max(1.0);
            }
            for w in 0..n_weeks {
                let date = start + 7 * w as i64;
                let close = v.iter().rev().find(|b| b.day < date).map_or(50.0, |b| b.c);
                let fair = close * (0.6 + 0.8 * rnd());
                rows.push(PanelRow { date, ticker: name.clone(), close_day: date - 1, close, q: [fair * 0.5, fair * 0.75, fair, fair * 1.3, fair * 1.7], mean: fair * 1.1, accession: String::new(), sector: "S".into() });
            }
            bars.insert(name, v);
        }
        let dt = build_data(rows, &bars);
        let rules = grid_of_500();
        assert_eq!(rules.len(), 500);
        let t = std::time::Instant::now();
        let mut bytes = 0usize;
        let (_, n) = run_grid(&rules, &dt, |s| {
            bytes += s.len();
            Ok(())
        })
        .unwrap();
        let secs = t.elapsed().as_secs_f64();
        println!("SPEED: 500 rules, {n_tickers} tickers x {n_weeks} weeks: {n} trades, {:.0} MB of text, {secs:.1} s on {} threads", bytes as f64 / 1e6, rayon::current_num_threads());
        assert!(secs < 60.0);
    }
}
