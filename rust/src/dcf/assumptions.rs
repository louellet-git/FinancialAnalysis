//! The register of the assumptions of the automatic model (ticket `T-20261120-dcf-assumption-register`).
//! `fa dcf assumptions`.
//!
//! One row for each assumption that a person could choose. A row has the record number (`docs/assumptions/`), the name,
//! the `--rules` key, the default (read from `Rules::default()`: the value is never typed twice), the place of the
//! default in the code, the record file and the state of the decision. The tests make the link between the code and
//! the records: the record of a row has the default in its section "Choice", and each key exists in `parse_rules`.
//!
//! `fa dcf assumptions --effect K=V,..` runs the screen and the point-in-time test with the default rules and with the
//! rules of the text, and prints one block that a record can quote.

use super::auto::{ExplainRow, Rules};
use super::auto_cli::{load_table, map_inputs, read_prices, screen_rows, ScreenConfig, ScreenRow};
use super::auto_pit::{ticker_half, coverage, median_error, parse_rules, pit_observations, pit_setup, PitObs, Series, HORIZON};
use super::stats::percentile;
use super::DcfError;
use std::fmt::Write;
use std::path::{Path, PathBuf};

type R<T> = Result<T, DcfError>;

/// The record of a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Record {
    /// A file in `docs/assumptions/`.
    File(&'static str),
    /// The record is to be written (`docs/models/auto-path-model.md`, "not written").
    NotWritten,
    /// No record: the value is a part of another row, or is not a choice that is studied.
    None,
}

pub const DECIDED_2026_11_20: &str = "decided 2026-11-20";
pub const CONVENTION: &str = "convention, not studied";

/// One assumption.
pub struct Assumption {
    /// `A-001` or `none`.
    pub id: &'static str,
    pub name: &'static str,
    /// The `--rules` key, or `none`.
    pub key: &'static str,
    /// The default, from `Rules::default()`, as the record states it.
    pub default: fn(&Rules) -> String,
    /// The place of the default in the code: file, item, field (or constant).
    pub file: &'static str,
    pub item: &'static str,
    pub field: &'static str,
    pub record: Record,
    pub state: &'static str,
    /// The names of the explain rows of `fa dcf auto --explain` that belong to the row (only a row with a record).
    pub explain: &'static [&'static str],
}

const AUTO: &str = "rust/src/dcf/auto.rs";
const DEF: &str = "Rules::default";

fn pct1(x: f64) -> String {
    format!("{:.1}%", x * 100.0)
}
fn pct2(x: f64) -> String {
    format!("{:.2}%", x * 100.0)
}
/// The form of a flag in a record: `name` on (`--rules name=1`).
fn flag_text(name: &str, on: bool) -> String {
    format!("`{name}` {} (`--rules {name}={}`)", if on { "on" } else { "off" }, u8::from(on))
}
fn onoff(b: bool) -> String {
    if b { "on" } else { "off" }.to_string()
}

macro_rules! row {
    ($id:expr, $name:expr, $key:expr, $default:expr, $field:expr, $record:expr, $state:expr, $explain:expr) => {
        Assumption { id: $id, name: $name, key: $key, default: $default, file: AUTO, item: DEF, field: $field, record: $record, state: $state, explain: $explain }
    };
}

/// The register. The order is the order of the print.
pub static TABLE: &[Assumption] = &[
    row!("A-001", "one long-run growth (fade level and terminal mean)", "long_run", |r| r.long_run.map_or("off".into(), pct1), "long_run", Record::File("A-001-terminal-growth.md"), DECIDED_2026_11_20, &["growth: long run", "terminal growth"]),
    row!("A-002", "acquisition charge (share w of the mean spend)", "acq_w", |r| flag_text("acq_w", r.acq_w), "acq_w", Record::File("A-002-acquisition-charge.md"), DECIDED_2026_11_20, &["acquisition charge"]),
    row!("A-002", "acquisition charge fades with the credited growth", "acq_fade", |r| flag_text("acq_fade", r.acq_fade), "acq_fade", Record::File("A-002-acquisition-charge.md"), DECIDED_2026_11_20, &[]),
    row!("A-003", "floor on the growth of a base year that fell", "last_floor", |r| r.last_floor.map_or("off".into(), |x| format!("{:.0}%", x * 100.0)), "last_floor", Record::File("A-003-base-year-decline-floor.md"), DECIDED_2026_11_20, &[]),
    row!("none", "margin: weight of the table in the shock size (an option, off)", "margin_sigma_nu", |r| r.margin_sigma_nu.map_or("off".into(), |x| format!("{x}")), "margin_sigma_nu", Record::None, CONVENTION, &[]),
    row!("none", "statutory tax rate (when the effective rate is not stable)", "none", |r| pct2(r.statutory_tax), "statutory_tax", Record::None, CONVENTION, &[]),
    row!("none", "least number of fiscal years in the chain", "none", |r| format!("{} years", r.min_years), "min_years", Record::None, CONVENTION, &[]),
    row!("none", "slow-fade half-life when the table has no fade cells", "none", |r| format!("{} years", r.default_slow_half_life), "default_slow_half_life", Record::None, CONVENTION, &[]),
    row!("none", "the company growth fades to the long-run level", "fade_to_cap", |r| onoff(r.fade_to_cap), "fade_to_cap", Record::None, CONVENTION, &[]),
    row!("none", "equity weight is 1 (cost of equity for the whole company)", "equity_weight_one", |r| onoff(r.equity_weight_one), "equity_weight_one", Record::None, CONVENTION, &[]),
    row!("none", "terminal growth mean is the 10-year zero yield", "terminal_rf", |r| onoff(r.terminal_rf), "terminal_rf", Record::None, CONVENTION, &[]),
    row!("none", "terminal growth, mean (used when long_run is off)", "terminal_growth", |r| pct2(r.terminal_growth.0), "terminal_growth", Record::None, CONVENTION, &[]),
    row!("none", "equity risk premium, mean", "erp", |r| pct2(r.erp.0), "erp", Record::NotWritten, CONVENTION, &[]),
    row!("none", "equity risk premium, standard deviation", "none", |r| pct2(r.erp.1), "erp", Record::None, CONVENTION, &[]),
    row!("none", "risk-free rate (10-year zero yield of the GSW curve)", "rf", |r| r.rf_fixed.map_or("GSW 10-year zero yield".into(), pct2), "rf_fixed", Record::NotWritten, CONVENTION, &[]),
    row!("none", "Hull-White mean reversion a", "none", |r| format!("{}", r.hw_a), "hw_a", Record::None, CONVENTION, &[]),
    row!("none", "Hull-White volatility sigma", "none", |r| format!("{}", r.hw_sigma), "hw_sigma", Record::None, CONVENTION, &[]),
    row!("none", "cost of debt: spread over the risk-free rate", "none", |r| pct2(r.kd_spread), "kd_spread", Record::None, CONVENTION, &[]),
    row!("none", "horizon", "years", |r| format!("{} years", r.years), "years", Record::NotWritten, CONVENTION, &[]),
    row!("none", "beta: weight of the regression beta (the rest is 1.0)", "beta_weight", |r| format!("{:.3}", r.beta_weight), "beta_weight", Record::NotWritten, CONVENTION, &[]),
    row!("none", "SBC is a cost of the model", "sbc_cost", |r| onoff(r.sbc_cost), "sbc_cost", Record::NotWritten, CONVENTION, &[]),
    row!("none", "terminal growth, standard deviation", "none", |r| pct2(r.terminal_growth.1), "terminal_growth", Record::None, CONVENTION, &[]),
    row!("none", "timing: discount from the model date (stub)", "stub", |r| onoff(r.stub), "stub", Record::None, CONVENTION, &[]),
    row!("none", "timing: cash flows in the middle of the year", "mid_year", |r| onoff(r.mid_year), "mid_year", Record::None, CONVENTION, &[]),
    row!("none", "bridge: long-term investments are cash", "bridge_lti", |r| onoff(r.bridge_lti), "bridge_lti", Record::None, CONVENTION, &[]),
    row!("none", "bridge: operating leases are debt", "bridge_leases", |r| onoff(r.bridge_leases), "bridge_leases", Record::None, CONVENTION, &[]),
    row!("none", "interest income is out of the free cash flow", "no_interest_income", |r| onoff(r.no_interest_income), "no_interest_income", Record::None, CONVENTION, &[]),
    row!("none", "share count: latest count plus dilution", "shares_latest", |r| onoff(r.shares_latest), "shares_latest", Record::None, CONVENTION, &[]),
    row!("none", "growth history: an organic jump stays", "history_organic", |r| onoff(r.history_organic), "history_organic", Record::None, CONVENTION, &[]),
    row!("none", "shock size: prior weight of the table, in years", "own_sigma_nu", |r| format!("{}", r.own_sigma_nu), "own_sigma_nu", Record::None, CONVENTION, &[]),
    row!("none", "shock size: least ratio to the growth level (negative: the fade table)", "level_kappa", |r| format!("{}", r.level_kappa), "level_kappa", Record::None, CONVENTION, &[]),
    row!("none", "margin: phi of all sizes above this standard error", "margin_pool_se", |r| format!("{}", r.margin_pool_se), "margin_pool_se", Record::None, CONVENTION, &[]),
];

/// The record number of an explain row, when the row belongs to a row of the register that has a record.
pub fn explain_id(input: &str) -> Option<&'static str> {
    TABLE.iter().find(|a| a.id != "none" && a.explain.contains(&input)).map(|a| a.id)
}

/// The explain rows with the record number in the name.
pub fn tag_rows(rows: &[ExplainRow]) -> Vec<ExplainRow> {
    rows.iter().map(|r| ExplainRow { input: explain_id(&r.input).map_or(r.input.clone(), |id| format!("{} ({id})", r.input)), ..r.clone() }).collect()
}

fn record_text(a: &Assumption) -> String {
    match a.record {
        Record::File(f) => format!("docs/assumptions/{f}"),
        Record::NotWritten => "record not written".into(),
        Record::None => "-".into(),
    }
}

fn place_text(a: &Assumption) -> String {
    format!("{}, {}, {}", a.file, a.item, a.field)
}

/// The text of the table. `md`: a Markdown table.
pub fn render_table(rules: &Rules, md: bool) -> String {
    let rows: Vec<[String; 7]> = TABLE.iter().map(|a| [a.id.into(), a.name.into(), a.key.into(), (a.default)(rules), a.state.into(), record_text(a), place_text(a)]).collect();
    let head = ["id", "name", "rule key", "default", "state", "record", "code"];
    let mut out = String::new();
    if md {
        let _ = writeln!(out, "| {} |", head.join(" | "));
        let _ = writeln!(out, "|{}", "---|".repeat(head.len()));
        for r in &rows {
            let _ = writeln!(out, "| {} |", r.iter().map(|c| c.replace('|', "/")).collect::<Vec<_>>().join(" | "));
        }
    } else {
        let w: Vec<usize> = (0..head.len()).map(|i| rows.iter().map(|r| r[i].chars().count()).chain([head[i].len()]).max().unwrap_or(0)).collect();
        let line = |cells: &[String]| cells.iter().zip(&w).map(|(c, n)| format!("{c:<n$}")).collect::<Vec<_>>().join("  ").trim_end().to_string();
        let _ = writeln!(out, "{}", line(&head.map(String::from)));
        for r in &rows {
            let _ = writeln!(out, "{}", line(r));
        }
    }
    out
}

/// The section "Choice" of a record text (from the line `## Choice` to the next `## ` line).
pub fn choice_section(text: &str) -> Option<String> {
    let mut inside = false;
    let mut out = String::new();
    for line in text.lines() {
        if line.trim_end() == "## Choice" {
            inside = true;
            continue;
        }
        if inside && line.starts_with("## ") {
            break;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    inside.then_some(out)
}

// ---------------------------------------------------------------------------------------------
// The effect of an alternative
// ---------------------------------------------------------------------------------------------

/// The numbers of a screen that the effect block shows.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenNumbers {
    /// Tickers with a value.
    pub n_values: usize,
    /// Tickers with a value and a price.
    pub n_priced: usize,
    pub median_ratio: Option<f64>,
    /// Tickers with a P50 at or below 0.
    pub nonpositive: usize,
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    Some(percentile(&v, 50.0))
}

pub fn screen_numbers(rows: &[ScreenRow]) -> ScreenNumbers {
    let values: Vec<_> = rows.iter().filter_map(|r| r.outcome.as_ref().ok()).collect();
    let ratios: Vec<f64> = values.iter().filter_map(|v| v.price.filter(|p| *p > 0.0).map(|p| v.p50 / p)).collect();
    ScreenNumbers { n_values: values.len(), n_priced: ratios.len(), median_ratio: median(ratios), nonpositive: values.iter().filter(|v| v.p50 <= 0.0).count() }
}

/// The median of `ln(new / old)` of the P50 over the tickers that have a positive P50 in both runs: (median, n).
pub fn median_log_change(old: &[ScreenRow], new: &[ScreenRow]) -> (Option<f64>, usize) {
    let p50 = |r: &ScreenRow| r.outcome.as_ref().ok().map(|v| v.p50).filter(|x| *x > 0.0);
    let v: Vec<f64> = old.iter().zip(new).filter(|(a, b)| a.ticker == b.ticker).filter_map(|(a, b)| Some((p50(b)? / p50(a)?).ln())).collect();
    let n = v.len();
    (median(v), n)
}

/// The numbers of the point-in-time test of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct PitNumbers {
    pub n: usize,
    /// For T+1 to T+3: median revenue error, median free cash flow error, coverage of revenue and of the free cash flow
    /// (percent inside P10 to P90).
    pub rev_err: [Option<f64>; HORIZON],
    pub fcff_err: [Option<f64>; HORIZON],
    pub rev_cover: [f64; HORIZON],
    pub fcff_cover: [f64; HORIZON],
}

pub fn pit_numbers(obs: &[PitObs]) -> PitNumbers {
    let all = |_: &PitObs| true;
    let mut p = PitNumbers { n: obs.len(), rev_err: [None; HORIZON], fcff_err: [None; HORIZON], rev_cover: [0.0; HORIZON], fcff_cover: [0.0; HORIZON] };
    for h in 0..HORIZON {
        p.rev_err[h] = median_error(obs, Series::Revenue, h, &all);
        p.fcff_err[h] = median_error(obs, Series::Fcff, h, &all);
        p.rev_cover[h] = coverage(obs, Series::Revenue, h, &all).shares()[1];
        p.fcff_cover[h] = coverage(obs, Series::Fcff, h, &all).shares()[1];
    }
    p
}

/// The short hash of the head commit of the cache, or "not a git repo".
pub fn cache_commit(dir: &Path) -> String {
    std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "not a git repo".into())
}

fn num(x: Option<f64>, d: usize) -> String {
    x.map_or("none".into(), |v| format!("{v:.d$}"))
}
fn pcts(x: Option<f64>) -> String {
    x.map_or("none".into(), |v| format!("{:+.2}%", v * 100.0))
}

/// The effect block.
pub struct Effect<'a> {
    pub command: &'a str,
    pub rules_text: &'a str,
    pub seed: u64,
    pub sims: usize,
    pub pit_paths: usize,
    pub as_of: &'a str,
    pub cache_commit: &'a str,
    pub tickers: usize,
    pub pit_cut: Option<i32>,
}

#[allow(clippy::too_many_arguments)]
pub fn render_effect(e: &Effect, s0: &ScreenNumbers, s1: &ScreenNumbers, change: (Option<f64>, usize), p0: &PitNumbers, p1: &PitNumbers) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "EFFECT of `--rules {}` against the default rules", e.rules_text);
    let _ = writeln!(o, "Command: {}", e.command);
    let _ = writeln!(o, "Seed {}; screen {} paths for each ticker; point-in-time test {} paths for each model.", e.seed, e.sims, e.pit_paths);
    let _ = writeln!(o, "Data date (as-of): {}; cache commit: {}; {} tickers in the list.", e.as_of, e.cache_commit, e.tickers);
    let _ = writeln!(o, "\nSCREEN                                   default    alternative");
    let _ = writeln!(o, "  tickers with a value                   {:>7}    {:>7}", s0.n_values, s1.n_values);
    let _ = writeln!(o, "  tickers with a value and a price       {:>7}    {:>7}", s0.n_priced, s1.n_priced);
    let _ = writeln!(o, "  median P50 / price                     {:>7}    {:>7}", num(s0.median_ratio, 4), num(s1.median_ratio, 4));
    let _ = writeln!(o, "  tickers with P50 at or below 0         {:>7}    {:>7}", s0.nonpositive, s1.nonpositive);
    let _ = writeln!(o, "  median change of the P50, ln(new/old): {} (n = {}, tickers with a positive P50 in both runs)", num(change.0, 4), change.1);
    let label = e.pit_cut.map_or("in sample (table estimated on all years)".to_string(), |y| format!("out of sample (table up to {y})"));
    let _ = writeln!(o, "\nPOINT-IN-TIME TEST, on the companies of the list, {label}; {} / {} company-years (default / alternative)", p0.n, p1.n);
    let _ = writeln!(o, "  median error of the P50 (revenue: real / P50 - 1; free cash flow: (real - P50) / revenue of T)");
    let _ = writeln!(o, "  coverage: real value inside P10 to P90 (target 80%)");
    for (name, a0, a1, c0, c1) in [("revenue", &p0.rev_err, &p1.rev_err, &p0.rev_cover, &p1.rev_cover), ("free cash flow", &p0.fcff_err, &p1.fcff_err, &p0.fcff_cover, &p1.fcff_cover)] {
        for h in 0..HORIZON {
            let _ = writeln!(o, "  {name:<15} T+{}: error {:>7} / {:>7}   coverage {:>6.2}% / {:>6.2}%", h + 1, pcts(a0[h]), pcts(a1[h]), c0[h], c1[h]);
        }
    }
    let _ = writeln!(o, "Limits: the table is in sample unless a cut is set; the companies are those that exist now (survivorship); the effects of several rules do not add.");
    o
}

pub const USAGE: &str = "\
fa dcf assumptions [--md]
  The register of the assumptions of the automatic model: id, name, `--rules` key, default (read from the code),
  state of the decision, record, and the place of the default in the code. --md: a Markdown table.
fa dcf assumptions --effect K=V[,K=V] --tickers-file FILE [OPTIONS]
  The effect of an alternative: the screen and the point-in-time test with the default rules and with the rules,
  one block that a record can quote (command, seed, as-of, cache commit).
  --cache DIR (default <repo>/data/raw)   --as-of DATE (default today)   --table FILE
  --sims N (4000)  --paths N (2000)  --seed S (7)   screen paths, point-in-time paths, seed
  --cut YEAR       point-in-time test out of sample (as `fa dcf pit --cut`)
  --half H         one half of the tickers, 0 or 1 (as `fa dcf pit --half`; the rule of the hash of the ticker)
  --prices FILE    prices `ticker,price` (else the price cache)   --jobs N";

fn value_of<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

/// `fa dcf assumptions`. Returns the exit code.
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
    if args.iter().any(|a| a == "--effect") && value_of(args, "--effect").is_none_or(|v| v.is_empty() || v.starts_with("--")) {
        return Err(DcfError("--effect needs a value K=V[,K=V]. Usage: fa dcf assumptions --effect K=V[,K=V] --tickers-file FILE [--cache DIR] [--as-of DATE]".into()));
    }
    let known = ["--effect", "--tickers-file", "--cache", "--as-of", "--table", "--sims", "--paths", "--seed", "--cut", "--half", "--prices", "--jobs"];
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if known.contains(&a) {
            i += 1;
        } else if a != "--md" {
            return Err(DcfError(format!("unknown option '{a}'")));
        }
        i += 1;
    }
    let Some(effect) = value_of(args, "--effect") else {
        print!("{}", render_table(&Rules::default(), args.iter().any(|a| a == "--md")));
        return Ok(());
    };
    let bad = |k: &str, v: &str| DcfError(format!("bad value for {k}: '{v}'"));
    let cache: PathBuf = value_of(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let table_file: PathBuf = value_of(args, "--table").map(PathBuf::from).unwrap_or_else(|| repo.join(super::auto::DEFAULT_TABLE));
    let sims: usize = value_of(args, "--sims").map_or(Ok(4000), |v| v.parse().map_err(|_| bad("--sims", v)))?;
    let paths: usize = value_of(args, "--paths").map_or(Ok(2000), |v| v.parse().map_err(|_| bad("--paths", v)))?;
    let seed: u64 = value_of(args, "--seed").map_or(Ok(7), |v| v.parse().map_err(|_| bad("--seed", v)))?;
    let cut: Option<i32> = value_of(args, "--cut").map(|v| v.parse().map_err(|_| bad("--cut", v))).transpose()?;
    let half: Option<u64> = value_of(args, "--half").map(|v| match v { "0" => Ok(0), "1" => Ok(1), _ => Err(bad("--half", v)) }).transpose()?;
    let jobs: Option<usize> = value_of(args, "--jobs").map(|v| v.parse().map_err(|_| bad("--jobs", v))).transpose()?;
    if sims == 0 || paths == 0 {
        return Err(DcfError("--sims and --paths must be positive".into()));
    }
    let (as_of, as_of_day) = match value_of(args, "--as-of") {
        Some(t) => (t.to_string(), mapcheck::dates::parse(t).ok_or_else(|| DcfError(format!("--as-of '{t}' is not YYYY-MM-DD")))?),
        None => (crate::market::today_iso(), mapcheck::dates::today()),
    };
    let Some(list) = value_of(args, "--tickers-file") else { return Err(DcfError("--effect needs --tickers-file".into())) };
    let mut tickers = mapcheck::cli::read_ticker_file(list).map_err(DcfError)?;
    let mut seen = std::collections::HashSet::new();
    tickers.retain(|t| seen.insert(t.clone()));
    if let Some(h) = half {
        tickers.retain(|t| ticker_half(t) == h);
    }
    let prices = match value_of(args, "--prices") {
        Some(p) => read_prices(p)?,
        None => Default::default(),
    };
    let base = Rules::default();
    let alt = parse_rules(effect)?;
    let (table, fade) = load_table(&table_file)?;

    let (inputs, _) = map_inputs(&cache, &tickers, as_of_day, jobs)?;
    let run = |rules: &Rules| -> R<Vec<ScreenRow>> {
        let cfg = ScreenConfig { table: &table, fade: &fade, rules: rules.clone(), sims, seed, implied_rate: false };
        Ok(screen_rows(&inputs, &cache, &as_of, &prices, &cfg, jobs)?.0)
    };
    let rows0 = run(&base)?;
    let rows1 = run(&alt)?;
    let setup = pit_setup(&cache, &table_file, cut, None, Some(&tickers), half)?;
    let obs0 = pit_observations(&setup, paths, seed, &base);
    let obs1 = pit_observations(&setup, paths, seed, &alt);

    let command = format!("fa dcf assumptions {}", args.join(" "));
    let commit = cache_commit(&cache);
    let e = Effect { command: &command, rules_text: effect, seed, sims, pit_paths: paths, as_of: &as_of, cache_commit: &commit, tickers: tickers.len(), pit_cut: cut };
    print!("{}", render_effect(&e, &screen_numbers(&rows0), &screen_numbers(&rows1), median_log_change(&rows0, &rows1), &pit_numbers(&obs0), &pit_numbers(&obs1)));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::auto_cli::ScreenValue;
    use super::*;

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    #[test]
    fn each_record_exists_and_its_choice_has_the_default_as_the_table_prints_it() {
        let rules = Rules::default();
        let mut checked = 0;
        for a in TABLE {
            let Record::File(f) = a.record else { continue };
            let path = repo().join("docs").join("assumptions").join(f);
            let text = std::fs::read_to_string(&path).unwrap_or_else(|x| panic!("{}: the record of {} ({}) cannot be read: {x}", path.display(), a.id, a.name));
            assert!(text.starts_with(&format!("# {}:", a.id)), "{f} does not start with the id {}", a.id);
            let choice = choice_section(&text).unwrap_or_else(|| panic!("{f} has no section \"Choice\""));
            let d = (a.default)(&rules);
            assert!(choice.contains(&d), "the default of {} ({}) is \"{d}\" in the code, but the section Choice of {f} does not contain it", a.id, a.name);
            checked += 1;
        }
        assert!(checked >= 3, "the table has the rows of A-001 and A-002");
    }

    #[test]
    fn a_default_that_changes_without_its_record_fails_the_check() {
        // the check of the test above on a changed default: 4.0% becomes 3.8% in the code, the record keeps 4.0%
        let rules = Rules { long_run: Some(0.038), ..Rules::default() };
        let a = TABLE.iter().find(|a| a.id == "A-001").unwrap();
        let Record::File(f) = a.record else { panic!() };
        let text = std::fs::read_to_string(repo().join("docs/assumptions").join(f)).unwrap();
        let choice = choice_section(&text).unwrap();
        assert_eq!((a.default)(&rules), "3.8%");
        assert!(!choice.contains(&(a.default)(&rules)));
        assert!(choice.contains(&(a.default)(&Rules::default())));
    }

    #[test]
    fn each_rules_key_of_the_table_exists_in_the_rules_parser() {
        for a in TABLE.iter().filter(|a| a.key != "none") {
            let r = parse_rules(&format!("{}=1", a.key));
            assert!(r.is_ok(), "the key '{}' of {} is not a rule of parse_rules: {:?}", a.key, a.name, r.err());
        }
        assert!(parse_rules("no_such_rule=1").is_err());
    }

    #[test]
    fn each_row_names_a_field_that_is_in_its_file_and_the_ids_are_in_the_records() {
        for a in TABLE {
            let text = std::fs::read_to_string(repo().join(a.file)).unwrap_or_else(|x| panic!("{}: {x}", a.file));
            let block = default_block(&text);
            assert!(block.contains(&format!("{}:", a.field)), "the field '{}' is not set in the block `impl Default for Rules` of {}", a.field, a.file);
            assert_eq!(a.id == "none", !matches!(a.record, Record::File(_)));
            if a.id != "none" {
                assert_eq!(a.state, DECIDED_2026_11_20);
            }
        }
    }

    /// The fields of `Rules` that are options of a measurement, off by default: each one is an alternative of a row of
    /// the table or a way to test it. They are not assumptions of their own.
    const NOT_ASSUMPTIONS: [&str; 9] = ["long_cap", "fade_years", "growth_w", "growth_phi", "margin_w", "margin_last", "acq_charge", "acq_f", "acq_mult"];

    fn default_block(text: &str) -> String {
        let start = text.find("impl Default for Rules").expect("impl Default for Rules");
        let rest = &text[start..];
        rest[..rest.find("\n}\n").expect("end of the block")].to_string()
    }

    fn struct_fields(text: &str) -> Vec<String> {
        let start = text.find("pub struct Rules {").expect("struct Rules");
        let rest = &text[start..];
        rest[..rest.find("\n}\n").unwrap()]
            .lines()
            .skip(1)
            .filter_map(|l| l.trim().strip_prefix("pub ").and_then(|x| x.split(':').next()).map(str::to_string))
            .collect()
    }

    #[test]
    fn each_field_of_rules_is_in_the_table_or_in_the_short_list_of_options() {
        let text = std::fs::read_to_string(repo().join(AUTO)).unwrap();
        let fields = struct_fields(&text);
        assert!(fields.len() > 30, "the parser of the fields found {} fields", fields.len());
        let block = default_block(&text);
        for f in &fields {
            let in_table = TABLE.iter().any(|a| a.field == f);
            let option = NOT_ASSUMPTIONS.contains(&f.as_str());
            assert!(in_table || option, "the field '{f}' of Rules is not in the register (add a row) and not in NOT_ASSUMPTIONS");
            assert!(!(in_table && option), "the field '{f}' is in the table and in NOT_ASSUMPTIONS");
            assert!(block.contains(&format!("{f}:")), "the field '{f}' has no default in impl Default for Rules");
        }
        for o in NOT_ASSUMPTIONS {
            assert!(fields.iter().any(|f| f == o), "'{o}' of NOT_ASSUMPTIONS is not a field of Rules");
        }
    }

    #[test]
    fn effect_without_a_value_is_an_error_with_a_usage_line() {
        for args in [vec!["--effect"], vec!["--effect", "--tickers-file", "x.txt"], vec!["--effect", ""]] {
            let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            let e = command_inner(&repo(), &a).unwrap_err().to_string();
            assert!(e.contains("--effect needs a value") && e.contains("Usage: fa dcf assumptions --effect"), "{e}");
            assert_eq!(command(&repo(), &a), 2);
        }
    }

    #[test]
    fn the_flags_of_a_record_have_the_form_with_the_rules_text() {
        assert_eq!(flag_text("acq_w", true), "`acq_w` on (`--rules acq_w=1`)");
        assert_eq!(flag_text("acq_fade", false), "`acq_fade` off (`--rules acq_fade=0`)");
        let a = TABLE.iter().find(|a| a.key == "acq_w").unwrap();
        assert_eq!((a.default)(&Rules::default()), "`acq_w` on (`--rules acq_w=1`)");
    }

    #[test]
    fn the_defaults_are_read_from_the_rules_and_follow_a_changed_rule() {
        let d = Rules::default();
        let text = |r: &Rules, key: &str| (TABLE.iter().find(|a| a.key == key).unwrap().default)(r);
        assert_eq!(text(&d, "long_run"), "4.0%");
        assert_eq!(text(&d, "erp"), "4.41%");
        assert_eq!(text(&d, "years"), "10 years");
        assert_eq!(text(&d, "beta_weight"), "0.667");
        assert_eq!(text(&d, "mid_year"), "off");
        assert_eq!(text(&d, "stub"), "on");
        let alt = parse_rules("long_run=off,mid_year=1,erp=0.05").unwrap();
        assert_eq!(text(&alt, "long_run"), "off");
        assert_eq!(text(&alt, "mid_year"), "on");
        assert_eq!(text(&alt, "erp"), "5.00%");
    }

    #[test]
    fn the_explain_rows_of_a_record_name_its_id() {
        assert_eq!(explain_id("growth: long run"), Some("A-001"));
        assert_eq!(explain_id("terminal growth"), Some("A-001"));
        assert_eq!(explain_id("acquisition charge"), Some("A-002"));
        assert_eq!(explain_id("equity risk premium"), None);
        let rows = vec![ExplainRow { input: "terminal growth".into(), value: "v".into(), source: "s".into(), rule: "r".into() }];
        assert_eq!(tag_rows(&rows)[0].input, "terminal growth (A-001)");
    }

    #[test]
    fn the_table_prints_in_two_forms() {
        let t = render_table(&Rules::default(), false);
        assert!(t.lines().next().unwrap().starts_with("id"));
        assert!(t.contains("A-001") && t.contains("4.0%") && t.contains("decided 2026-11-20"));
        assert_eq!(t.lines().count(), TABLE.len() + 1);
        let m = render_table(&Rules::default(), true);
        assert!(m.starts_with("| id | name |"));
        assert_eq!(m.lines().count(), TABLE.len() + 2);
    }

    fn row(ticker: &str, p50: f64, price: Option<f64>) -> ScreenRow {
        ScreenRow { ticker: ticker.into(), name: String::new(), status: String::new(), template: "standard".into(), outcome: Ok(ScreenValue { p50, price, ..Default::default() }) }
    }

    #[test]
    fn the_effect_numbers_have_the_hand_calculated_answers() {
        let old = vec![row("A", 50.0, Some(100.0)), row("B", 200.0, Some(100.0)), row("C", 30.0, Some(100.0)), row("D", -5.0, None), row("E", 10.0, None)];
        let new = vec![row("A", 100.0, Some(100.0)), row("B", 200.0, Some(100.0)), row("C", 15.0, Some(100.0)), row("D", 5.0, None), row("E", 10.0, None)];
        let a = screen_numbers(&old);
        // ratios 0.5, 2.0, 0.3: the median is 0.5; P50 at or below 0: D
        assert_eq!((a.n_values, a.n_priced, a.nonpositive), (5, 3, 1));
        assert!((a.median_ratio.unwrap() - 0.5).abs() < 1e-12);
        let b = screen_numbers(&new);
        assert_eq!(b.nonpositive, 0);
        assert!((b.median_ratio.unwrap() - 1.0).abs() < 1e-12); // 1.0, 2.0, 0.15
        // changes ln(2), 0, ln(0.5), (D has a P50 not positive in the old run: left out), 0: sorted -ln2, 0, 0, ln2: median 0
        let (m, n) = median_log_change(&old, &new);
        assert_eq!(n, 4);
        assert!(m.unwrap().abs() < 1e-12);
        // the block names the command, the seed, the date and the commit
        let p = PitNumbers { n: 0, rev_err: [None; HORIZON], fcff_err: [None; HORIZON], rev_cover: [0.0; HORIZON], fcff_cover: [0.0; HORIZON] };
        let e = Effect { command: "fa dcf assumptions --effect long_run=off", rules_text: "long_run=off", seed: 7, sims: 4000, pit_paths: 2000, as_of: "2026-11-19", cache_commit: "abc1234", tickers: 5, pit_cut: None };
        let text = render_effect(&e, &a, &b, (m, n), &p, &p);
        for needle in ["fa dcf assumptions --effect long_run=off", "Seed 7", "2026-11-19", "abc1234", "0.5000", "1.0000"] {
            assert!(text.contains(needle), "missing '{needle}' in\n{text}");
        }
    }

    #[test]
    fn the_cache_commit_of_a_folder_that_is_not_a_repo_is_named() {
        let dir = std::env::temp_dir().join("fa_assumptions_not_a_repo");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(cache_commit(&dir), "not a git repo");
        // the repo itself gives a hash of 7 or more hexadecimal characters
        let c = cache_commit(&repo());
        assert!(c == "not a git repo" || c.chars().all(|ch| ch.is_ascii_hexdigit()), "{c}");
    }

    #[test]
    fn the_median_log_change_is_zero_for_equal_runs_and_ln_two_for_a_doubling() {
        let a = vec![row("A", 10.0, None), row("B", 20.0, None)];
        let b = vec![row("A", 20.0, None), row("B", 40.0, None)];
        assert!((median_log_change(&a, &b).0.unwrap() - 2f64.ln()).abs() < 1e-12);
        assert_eq!(median_log_change(&a, &a).0, Some(0.0));
    }
}
