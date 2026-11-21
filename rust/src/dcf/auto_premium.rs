//! `fa dcf premium-series`: one row each Friday with the premium that the list implies (S1), the median weekly return of the
//! list (ret_w) and, with `--bridge`, the "all" line of the full bridge (S2). It makes the series of the design
//! `analysis/macro/2026-11-21-premium-macro-design.md` (rules 1 to 4, rule 7). The command contains no macro data.
//!
//! Each row is point in time: the facts are the facts filed on or before the anchor (`map_inputs_pit`), the close is the last
//! close on or before the anchor, the beta and the curve are those of the anchor. Nothing dated after the anchor enters the row.
//! The list is a list of today's names (survivorship). A market-implied value is a measurement. It is never an input of the
//! model, and the value at the means is not the expected value.

use super::auto::*;
use super::auto_attribute::{attribute_list, finish_weights, full_bridge, stale_note, close_date, View};
use super::auto_cli::{cache_dir, jobs_of, load_table, map_inputs_pit, table_path, tickers_of, Opts};
use super::auto_implied::{list_premium, Bound, ImpliedConfig};
use super::calibrate::{FadeCell, ProcessTable};
use super::stats::{percentile, sorted_copy};
use super::DcfError;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

type R<T> = Result<T, DcfError>;
type Series = [(String, f64)];

fn e<T>(m: impl Into<String>) -> R<T> {
    Err(DcfError(m.into()))
}

pub const PREMIUM_SERIES_USAGE: &str = "\
fa dcf premium-series --tickers-file FILE --start D1 --end D2 --out FILE [OPTIONS]
  One CSV row for each Friday from D1 to D2: the premium that a list of tickers implies, the median return of the list
  and, with --bridge, the weekly bridge of the value of the list. A fixed procedure with no random part. It makes the
  series that the macro design needs. The file has no macro data. A market-implied value is a measurement, never an input
  of the model. The value is the value of the one path at the means; it is not the expected value.
  The anchor of a Friday is the last day on or before it with a SPY close (SPY is the trading calendar). Each number of a
  row uses only data that was public on the anchor: the facts filed on or before it, the last close on or before it, the
  beta and the curve of the anchor. The list is a list of today's names (survivorship).
  The curve of an anchor is the GSW row if GSW has a row on or before the anchor that is at most 3 days older; else it is
  the par fit (the NSS fit to the Treasury par yield curve) of the last par row on or before the anchor. The first
  week with the par fit (the switch week) is printed.
  Columns (rates and returns are decimals, 0.0232 is 2.32%; a missing value is an empty field):
    friday, anchor, curve_source (gsw or par), curve_date, zero10 (10-year zero yield of the curve)
    s1         the equity risk premium for which the median of value / price of the list is 1 (a year). Empty when the
               search has no solution in its range (the bound is not a measurement)
    n_value    tickers with a value and a price (the tickers of the median), n_list: tickers in the list
    ret_w      the median over the list of close(anchor) / close(previous anchor) - 1. A ticker with a close older than
               the SPY close of an anchor, or with no close at an anchor, is out; n_ret is the count that stays
    s2_*       with --bridge: the line \"all\" of the full bridge of `fa dcf attribute` (premium view first) from the
               previous anchor to the anchor: facts, time, rates, premium, growth, residual, nomodel (shares of the
               market value of the list at the previous anchor) and s2_change (the change of the list, value-weighted)
  The first row has no previous week: ret_w, n_ret and s2_* are empty.
  --tickers-file F  the list (one ticker each line)
  --start D1 --end D2  the dates (YYYY-MM-DD); the first row is the first Friday on or after D1
  --cache DIR     as for `fa dcf auto`
  --out FILE      the CSV; each row is written when it is done
  --bridge        also make the s2_* columns (this is the slow part)
  --per-ticker FILE  with --bridge: a second CSV with one row for each week and ticker with a weight (the first week has
                  none). Header: friday, anchor, ticker, weight, change, facts, time, rates, premium, growth, residual,
                  nomodel. Decimals; an empty field is a missing value. weight: the share of the market value of the list at
                  the previous anchor; change: the change of the price of the ticker; the other columns are the bridge line
                  of the ticker (the same parts as the s2_* columns), as shares of the market value of the LIST at the
                  previous anchor. So the rows of a week add to the s2_* columns of the main CSV (weight x change adds to
                  s2_change). The main CSV does not change
  --windows-out FILE  write the windows of the weeks for `fa macro view --windows FILE`: header d1,date; one row for each
                  week after the first: the previous anchor (d1) and the anchor (date). Then the economics view and this
                  series use the same weeks
  --table FILE    parameter table
  --rules K=V,..  change a rule of the builder (the list: fa dcf auto --help)
  --jobs N        threads (default: all); the result does not depend on N";

/// The header of the CSV: a fixed contract with the programs that read the file.
pub const HEADER: &str = "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change";

/// The header of the second CSV (`--per-ticker`): a fixed contract. Decimals. The parts are shares of the market value of the
/// list at the previous anchor, so the rows of a week add to the s2_* columns of the main CSV.
pub const PER_TICKER_HEADER: &str = "friday,anchor,ticker,weight,change,facts,time,rates,premium,growth,residual,nomodel";

/// A curve may be this many days older than its anchor.
pub const CURVE_LAG_DAYS: i64 = 3;
/// A SPY close must be less than this many days before its Friday.
const ANCHOR_MAX_AGE: i64 = 7;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveSource {
    Gsw,
    Par,
}

impl CurveSource {
    pub fn name(self) -> &'static str {
        match self {
            CurveSource::Gsw => "gsw",
            CurveSource::Par => "par",
        }
    }
}

/// The source of the curve of an anchor: GSW when its last row on or before the anchor (`gsw_date`) is at most
/// `CURVE_LAG_DAYS` days older; else the par fit.
pub fn curve_source(gsw_date: Option<&str>, anchor: &str) -> CurveSource {
    let lag = gsw_date.and_then(|g| Some(mapcheck::dates::parse(anchor.get(..10)?)? - mapcheck::dates::parse(g.get(..10)?)?));
    match lag {
        Some(l) if (0..=CURVE_LAG_DAYS).contains(&l) => CurveSource::Gsw,
        _ => CurveSource::Par,
    }
}

/// The Fridays (as `YYYY-MM-DD`) from `start` to `end`, both included.
pub fn fridays(start: i64, end: i64) -> Vec<String> {
    // day 0 (1970-01-01) is a Thursday: with Monday as 0, `(day + 3) mod 7` is 4 on a Friday
    let first = start + (4 - (start + 3).rem_euclid(7)).rem_euclid(7);
    (0..).map(|k| first + 7 * k).take_while(|d| *d <= end).map(mapcheck::dates::format).collect()
}

/// The anchor of a Friday: the last date on or before it with a SPY close. An error when it is `ANCHOR_MAX_AGE` days or more
/// before the Friday (the SPY series of the cache ends too early).
pub fn anchor_of(spy: &[(String, f64)], friday: &str) -> R<String> {
    let day = mapcheck::dates::parse(friday).ok_or_else(|| DcfError(format!("'{friday}' is not a date")))?;
    let last = spy.iter().filter(|(d, c)| d.get(..10).is_some_and(|d| d <= friday) && c.is_finite() && *c > 0.0).map(|(d, _)| d[..10].to_string()).max();
    match last {
        Some(a) if mapcheck::dates::parse(&a).is_some_and(|x| day - x < ANCHOR_MAX_AGE) => Ok(a),
        Some(a) => e(format!("the last SPY close on or before {friday} is {a}: the SPY series of the cache is too short")),
        None => e(format!("no SPY close on or before {friday}")),
    }
}

/// The median of the returns of the list names between two anchors, and their count. A name is out when it has no close at an
/// anchor, or when its close at an anchor is older than the SPY close of that anchor (`stale_note`, the rule of the bridge).
pub fn week_return<'a>(tickers: &[String], closes: &dyn Fn(&str) -> Option<&'a Series>, spy: &[(String, f64)], a0: &str, a1: &str) -> (Option<f64>, usize) {
    let (w0, w1) = (close_date(spy, a0), close_date(spy, a1));
    let mut v: Vec<f64> = Vec::new();
    for t in tickers {
        let Some(s) = closes(t) else { continue };
        let (Some((_, c0)), Some((_, c1))) = (close_on_or_before(s, a0), close_on_or_before(s, a1)) else { continue };
        if stale_note(s, a0, w0.as_deref()).is_some() || stale_note(s, a1, w1.as_deref()).is_some() {
            continue;
        }
        let r = c1 / c0 - 1.0;
        if r.is_finite() {
            v.push(r);
        }
    }
    if v.is_empty() {
        return (None, 0);
    }
    let n = v.len();
    (Some(percentile(&sorted_copy(&v), 50.0)), n)
}

/// One row of the series. A missing value is `None`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    pub friday: String,
    pub anchor: String,
    pub source: Option<CurveSource>,
    pub curve_date: String,
    pub zero10: Option<f64>,
    pub s1: Option<f64>,
    pub n_value: usize,
    pub n_list: usize,
    pub ret: Option<f64>,
    pub n_ret: Option<usize>,
    /// facts, time, rates, premium, growth, residual, nomodel, change
    pub s2: Option<[f64; 8]>,
    /// With the bridge: one line for each ticker with a weight (sorted by ticker). Not a column of the main CSV.
    pub per_ticker: Vec<TickerRow>,
}

/// The bridge line of one ticker for one week (the full bridge of `fa dcf attribute`, premium view first).
#[derive(Debug, Clone, PartialEq)]
pub struct TickerRow {
    pub ticker: String,
    /// The share of the market value of the list at the previous anchor.
    pub weight: f64,
    /// The change of the price of the ticker.
    pub change: f64,
    /// facts, time, rates, premium, growth, residual, nomodel (shares of the market value of the list at the previous anchor)
    pub parts: [f64; 7],
}

impl TickerRow {
    pub fn csv_line(&self, friday: &str, anchor: &str) -> String {
        let p: Vec<String> = self.parts.iter().map(|x| f(Some(*x))).collect();
        format!("{friday},{anchor},{},{},{},{}", self.ticker, f(Some(self.weight)), f(Some(self.change)), p.join(","))
    }
}

fn f(x: Option<f64>) -> String {
    x.filter(|x| x.is_finite()).map_or(String::new(), |x| format!("{x}"))
}

impl Row {
    pub fn csv_line(&self) -> String {
        let s2: Vec<String> = (0..8).map(|k| f(self.s2.map(|a| a[k]))).collect();
        format!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            self.friday,
            self.anchor,
            self.source.map_or("", |s| s.name()),
            self.curve_date,
            f(self.zero10),
            f(self.s1),
            self.n_value,
            self.n_list,
            f(self.ret),
            self.n_ret.map_or(String::new(), |n| n.to_string()),
            s2.join(",")
        )
    }
}

/// What the next week needs from this week.
pub struct State {
    pub anchor: String,
    pub curve: CurveNumbers,
    pub inputs: Vec<AutoInputs>,
}

pub type CurveOf<'a> = Box<dyn Fn(&str) -> R<(CurveNumbers, CurveSource)> + Sync + 'a>;
pub type MapperOf<'a> = Box<dyn Fn(&str) -> R<Vec<AutoInputs>> + Sync + 'a>;

/// The data of a run.
pub struct Ctx<'a> {
    pub cache: &'a Path,
    pub table: &'a ProcessTable,
    pub fade: &'a [FadeCell],
    pub rules: Rules,
    pub tickers: Vec<String>,
    /// The SPY series (the trading calendar and the index of the beta).
    pub spy: Vec<(String, f64)>,
    pub closes: HashMap<String, Vec<(String, f64)>>,
    pub bridge: bool,
    /// The curve of an anchor.
    pub curve_of: CurveOf<'a>,
    /// The mapped inputs of the list with the facts filed on or before an anchor.
    pub mapper: MapperOf<'a>,
}

impl<'a> Ctx<'a> {
    /// The closes of all tickers are read one time.
    pub fn load_closes(cache: &Path, tickers: &[String]) -> HashMap<String, Vec<(String, f64)>> {
        tickers.iter().filter_map(|t| price_series(cache, t).map(|s| (t.clone(), s))).collect()
    }
}

/// The row of one Friday, and the state for the next week. `prior`: the state of the week before (none for the first week).
pub fn week_row(ctx: &Ctx, friday: &str, prior: Option<&State>) -> R<(Row, State)> {
    let anchor = anchor_of(&ctx.spy, friday)?;
    let (curve, source) = (ctx.curve_of)(&anchor)?;
    let inputs = (ctx.mapper)(&anchor)?;
    let cfg = ImpliedConfig { table: ctx.table, fade: ctx.fade, rules: ctx.rules.clone(), cache: Some(ctx.cache) };
    let market_at_anchor = |a: &str, c: &CurveNumbers| {
        let (a, c) = (a.to_string(), c.clone());
        move |t: &str| -> Market { market_at(ctx.cache, t, &a, None, Some(&a), &ctx.rules, Some(&c), Some(ctx.spy.as_slice())) }
    };
    let market_of = market_at_anchor(&anchor, &curve);
    let mp = list_premium(&inputs, &market_of, &cfg)?;
    let mut row = Row {
        friday: friday.to_string(),
        anchor: anchor.clone(),
        source: Some(source),
        curve_date: curve.date.clone(),
        zero10: Some(curve.zero10),
        s1: mp.filter(|m| m.found.bound == Bound::Exact && m.found.x.is_finite()).map(|m| m.found.x),
        n_value: mp.map_or(0, |m| m.n),
        n_list: ctx.tickers.len(),
        ..Default::default()
    };
    if let Some(p) = prior {
        let (ret, n) = week_return(&ctx.tickers, &|t| ctx.closes.get(t).map(Vec::as_slice), &ctx.spy, &p.anchor, &anchor);
        row.ret = ret;
        row.n_ret = Some(n);
        if ctx.bridge {
            let m1 = market_at_anchor(&p.anchor, &p.curve);
            let mut rows = attribute_list(&p.inputs, &inputs, &m1, &market_of, &HashMap::new(), &cfg, None)?;
            // the stale-price rule compares each close with the SPY close: `ctx.closes` has the tickers of the list only,
            // so SPY comes from `ctx.spy` (without it the rule did not run in the bridge of the series)
            let closes = |t: &str| if t == "SPY" { Some(ctx.spy.clone()) } else { ctx.closes.get(t).cloned() };
            finish_weights(&mut rows, &closes, &p.anchor, &anchor);
            if let Some(b) = full_bridge(&rows, View::Premium) {
                let a = &b.all;
                row.s2 = Some([a.parts[0], a.parts[1], a.parts[2], a.parts[3], a.parts[4], a.parts[5], a.parts[6], a.change]);
                row.per_ticker = b.tickers.iter().map(|l| TickerRow { ticker: l.name.clone(), weight: l.share, change: l.change, parts: l.parts }).collect();
                row.per_ticker.sort_by(|x, y| x.ticker.cmp(&y.ticker));
            }
        }
    }
    Ok((row, State { anchor, curve, inputs }))
}

/// The curve of an anchor from the cache: GSW, or the par fit when GSW is more than `CURVE_LAG_DAYS` days older.
fn cache_curve(cache: &Path, gsw: Option<&crate::forwards::Dataset>, anchor: &str) -> R<(CurveNumbers, CurveSource)> {
    let g = gsw.and_then(|ds| curve_on(ds, anchor).ok());
    if let (CurveSource::Gsw, Some(c)) = (curve_source(g.as_ref().map(|c| c.date.as_str()), anchor), &g) {
        return Ok((c.clone(), CurveSource::Gsw));
    }
    load_curve_par(cache, anchor).map(|c| (c, CurveSource::Par)).map_err(DcfError)
}

/// `fa dcf premium-series`. Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> u8 {
    match command_inner(repo, args) {
        Ok(()) => 0,
        Err(x) => {
            eprintln!("fa: {x}");
            2
        }
    }
}

fn date_of(o: &Opts, key: &str) -> R<i64> {
    match o.get(key) {
        Some(t) => mapcheck::dates::parse(t).ok_or_else(|| DcfError(format!("{key} '{t}' is not YYYY-MM-DD"))),
        None => e(format!("{key} DATE is required")),
    }
}

/// The header of the windows file of `--windows-out` (the input of `fa macro view --windows`).
pub const WINDOWS_HEADER: &str = "d1,date";

/// One row of the windows file: the previous anchor (D1) and the anchor (D2) of a week.
pub fn windows_line(prev_anchor: &str, anchor: &str) -> String {
    format!("{prev_anchor},{anchor}")
}

fn command_inner(repo: &Path, args: &[String]) -> R<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{PREMIUM_SERIES_USAGE}");
        return Ok(());
    }
    let o = Opts::parse(args, &["--cache", "--out", "--jobs", "--table", "--start", "--end", "--tickers-file", "--rules", "--per-ticker", "--windows-out"], &["--bridge"])?;
    if o.get("--per-ticker").is_some() && !o.flag("--bridge") {
        return e("--per-ticker needs --bridge");
    }
    let cache = cache_dir(repo, &o);
    let (table, fade) = load_table(&table_path(repo, &o))?;
    let (start, end) = (date_of(&o, "--start")?, date_of(&o, "--end")?);
    if end < start {
        return e("--end must not be earlier than --start");
    }
    let out = o.get("--out").ok_or_else(|| DcfError("--out FILE is required".into()))?;
    if o.get("--tickers-file").is_none() {
        return e("--tickers-file FILE is required");
    }
    let jobs = jobs_of(&o)?;
    let tickers = tickers_of(&o, &cache)?;
    let rules = super::auto_pit::parse_rules(o.get("--rules").unwrap_or(""))?;
    let fridays = fridays(start, end);
    if fridays.is_empty() {
        return e("no Friday between --start and --end");
    }
    let spy = index_from_cache(&cache).ok_or_else(|| DcfError("no SPY price series in the cache".into()))?;
    let gsw = std::fs::read_to_string(cache.join("rates").join("gsw_feds200628.csv")).ok().and_then(|t| crate::forwards::parse_gsw(&t).ok());
    if gsw.is_none() {
        eprintln!("fa: no GSW file in the cache: every week uses the par fit");
    }
    let ctx = Ctx {
        cache: &cache,
        table: &table,
        fade: &fade,
        rules,
        closes: Ctx::load_closes(&cache, &tickers),
        tickers: tickers.clone(),
        spy,
        bridge: o.flag("--bridge"),
        curve_of: Box::new(|a| cache_curve(&cache, gsw.as_ref(), a)),
        mapper: Box::new(|a| map_inputs_pit(&cache, &tickers, a, mapcheck::dates::parse(a).unwrap_or(0), None).map(|x| x.0)),
    };
    let mut file = std::fs::File::create(out).map_err(|x| DcfError(format!("{out}: {x}")))?;
    writeln!(file, "{HEADER}").map_err(|x| DcfError(x.to_string()))?;
    let mut per_ticker_file = match o.get("--per-ticker") {
        Some(p) => {
            let mut pf = std::fs::File::create(p).map_err(|x| DcfError(format!("{p}: {x}")))?;
            writeln!(pf, "{PER_TICKER_HEADER}").map_err(|x| DcfError(x.to_string()))?;
            Some(pf)
        }
        None => None,
    };
    let mut windows_file = match o.get("--windows-out") {
        Some(p) => {
            let mut wf = std::fs::File::create(p).map_err(|x| DcfError(format!("{p}: {x}")))?;
            writeln!(wf, "{WINDOWS_HEADER}").map_err(|x| DcfError(x.to_string()))?;
            Some(wf)
        }
        None => None,
    };
    let mut prev_anchor: Option<String> = None;
    println!("PREMIUM SERIES {} to {}: {} Fridays, {} tickers, {}. No random part. A market-implied value is a measurement, not an input. The list is a list of today's names (survivorship).", fridays[0], fridays[fridays.len() - 1], fridays.len(), tickers.len(), if ctx.bridge { "with the bridge" } else { "no bridge" });
    let mut run = || -> R<()> {
        let t_all = Instant::now();
        let mut prior: Option<State> = None;
        let mut switch: Option<String> = None;
        for fr in &fridays {
            let t0 = Instant::now();
            let (row, state) = week_row(&ctx, fr, prior.as_ref())?;
            writeln!(file, "{}", row.csv_line()).and_then(|_| file.flush()).map_err(|x| DcfError(x.to_string()))?;
            if let Some(pf) = per_ticker_file.as_mut() {
                for t in &row.per_ticker {
                    writeln!(pf, "{}", t.csv_line(&row.friday, &row.anchor)).map_err(|x| DcfError(x.to_string()))?;
                }
                pf.flush().map_err(|x| DcfError(x.to_string()))?;
            }
            if let (Some(wf), Some(d1)) = (windows_file.as_mut(), prev_anchor.as_deref()) {
                writeln!(wf, "{}", windows_line(d1, &row.anchor)).and_then(|_| wf.flush()).map_err(|x| DcfError(x.to_string()))?;
            }
            prev_anchor = Some(row.anchor.clone());
            if row.source == Some(CurveSource::Par) && switch.is_none() {
                switch = Some(fr.clone());
                println!("  SWITCH WEEK: {fr} is the first week with the par fit (curve date {})", row.curve_date);
            }
            println!(
                "  {fr} anchor {} curve {} {} s1 {} ({}/{} with a value) ret {} (n {}){} | {:.1} s",
                row.anchor,
                row.source.map_or("", |s| s.name()),
                row.curve_date,
                row.s1.map_or("none (no solution in the range)".to_string(), |x| format!("{:.4}", x)),
                row.n_value,
                row.n_list,
                row.ret.map_or("-".to_string(), |x| format!("{x:.4}")),
                row.n_ret.unwrap_or(0),
                row.s2.map_or(String::new(), |s| format!(" s2 premium {:.4} rates {:.4} change {:.4}", s[3], s[2], s[7])),
                t0.elapsed().as_secs_f64()
            );
            prior = Some(state);
        }
        println!("DONE {} weeks in {:.1} s ({:.1} s each); switch week: {}; file {out}", fridays.len(), t_all.elapsed().as_secs_f64(), t_all.elapsed().as_secs_f64() / fridays.len() as f64, switch.unwrap_or_else(|| "none".into()));
        Ok(())
    };
    match jobs {
        Some(n) => rayon::ThreadPoolBuilder::new().num_threads(n.max(1)).build().map_err(|x| DcfError(x.to_string()))?.install(run),
        None => run(),
    }
}

#[cfg(test)]
#[path = "auto_premium_tests.rs"]
mod tests;
