//! `fa portfolio walkforward`: the walk-forward choice of the rule (ticket `T-20261121-trade-level-backtest`, part 3).
//!
//! The grid of rules is fixed before the run (the rules file of `fa portfolio trades`). At each panel date `t` the rule in
//! use is the rule with the best mean trade return over its trades that CLOSED before `t` (exit day before `t`), among the
//! rules with at least `--min-trades` such trades. Before any rule qualifies there is no trade. The walk-forward trades are
//! the trades of the chosen rule with the signal date `t`. Their mean return is the reported expected return.
//!
//! With `--hypothesis` the run goes through the run harness (`harness.rs`): the committed hypothesis file, the add-only run
//! log with the variant counts, the out-of-sample window; the run log holds the number of rules of the grid. A run is one
//! variant of its family. Without it the run is exploratory and not counted.

use super::cli::{check_options, opt, publish, refuse_existing, write_new};
use super::harness::{self, Hypothesis};
use super::panel::{check_fresh, Header, Provenance};
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa portfolio walkforward --trades TRADES --min-trades N --out FILE [--cache DIR] [--hypothesis FILE]
  The walk-forward choice of the rule. At each signal date t, the rule in use is the rule with the best mean trade return
  over its trades with an exit day before t, among the rules with at least N such trades (before that: no trade). The
  walk-forward trades are the trades of the chosen rule with the signal date t. The text on the screen has their count, mean
  return, hit rate, the table by year, and the t statistic over the calendar months (the mean of the monthly means over its
  standard error; months with a trade only; the trades of a month are not independent and the holds overlap the next months:
  the t is too high). It also has the in-sample best rule of the whole period, labelled \"chosen with later data, not a result\".
  --trades TRADES    the file of `fa portfolio trades` (refused if its code, store or split table is not the current one)
  --min-trades N     the minimum number of closed trades of a rule before it can be chosen
  --out FILE         the walk-forward trades, in the format of the trades file (a file is never replaced)
  --cache DIR        the data store, for the check of the header (default <repo>/data/raw)
  --hypothesis FILE  a registered hypothesis (trading/hypotheses/<id>.toml, committed, strategy trade-walkforward, no params):
                     the run writes the add-only log trading/runs/<id>/<UTC time>.json with the variant counts, the number
                     of rules of the grid and the window flags. The universe of the file must be the tickers file of the panel
                     (same hash). No file: an exploratory run, not counted as a variant.
  The prices are market prices (daily bars), the list of tickers is today's list (survivorship).";

pub const IN_SAMPLE_LABEL: &str = "chosen with later data, not a result";

/// A trade of the file, as the walk-forward reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WfTrade {
    pub rule: u32,
    pub signal: i64,
    pub exit: i64,
    pub ret: f64,
    /// The line of the trades file (to write the chosen trades).
    pub line: u32,
}

/// The trades file: the header, the rule ids, the trades.
pub struct TradesFile {
    pub header: Header,
    pub rule_ids: Vec<String>,
    pub trades: Vec<WfTrade>,
}

/// Read the text of a trades file. The footer must say the number of trades (a cut file is an error).
pub fn read_trades(text: &str) -> Result<TradesFile, String> {
    let lines: Vec<&str> = text.lines().collect();
    let header = Header::parse(lines.first().copied().unwrap_or(""), "fa-trades")?;
    let ids_line = lines.get(1).and_then(|l| l.strip_prefix("# rule_ids ")).ok_or("the second line of the trades file is not '# rule_ids ...'")?;
    let rule_ids: Vec<String> = ids_line.split(',').map(str::to_string).collect();
    if lines.get(2).is_none_or(|l| !l.starts_with("rule,ticker,side,signal_date")) {
        return Err("the trades columns are not the expected ones".into());
    }
    let footer = lines.last().and_then(|l| l.strip_prefix("# end n_trades=")).and_then(|n| n.parse::<usize>().ok()).ok_or("the trades file has no footer '# end n_trades=N': the file is cut")?;
    let index: HashMap<&str, u32> = rule_ids.iter().enumerate().map(|(k, r)| (r.as_str(), k as u32)).collect();
    let body = &lines[3..lines.len() - 1];
    let parsed: Vec<Result<WfTrade, String>> = body
        .par_iter()
        .enumerate()
        .map(|(k, l)| {
            let w: Vec<&str> = l.split(',').collect();
            if w.len() != 11 {
                return Err(format!("trades: a row has {} fields, not 11: '{l}'", w.len()));
            }
            Ok(WfTrade {
                rule: *index.get(w[0]).ok_or_else(|| format!("trades: unknown rule '{}'", w[0]))?,
                signal: mapcheck::dates::parse(w[3]).ok_or_else(|| format!("trades: bad date '{}'", w[3]))?,
                exit: mapcheck::dates::parse(w[6]).ok_or_else(|| format!("trades: bad date '{}'", w[6]))?,
                ret: w[10].parse().map_err(|_| format!("trades: bad return '{}'", w[10]))?,
                line: (k + 3) as u32,
            })
        })
        .collect();
    let trades: Vec<WfTrade> = parsed.into_iter().collect::<Result<_, _>>()?;
    if trades.len() != footer {
        return Err(format!("the footer says {footer} trades, the file has {}", trades.len()));
    }
    Ok(TradesFile { header, rule_ids, trades })
}

/// The result of a walk-forward pass.
#[derive(Debug, Clone, PartialEq)]
pub struct Walk {
    /// The signal dates (all distinct signal dates of the trades), in order, and the rule in use on each.
    pub dates: Vec<i64>,
    pub chosen: Vec<Option<u32>>,
    /// The indices (in the list of trades) of the walk-forward trades.
    pub picked: Vec<usize>,
}

/// The rule with the best mean over the sums and counts, among the rules with `min` trades or more. Ties go to the first rule.
fn best_rule(sum: &[f64], cnt: &[usize], min: usize) -> Option<u32> {
    let mut best: Option<(u32, f64)> = None;
    for k in 0..sum.len() {
        if cnt[k] >= min.max(1) {
            let m = sum[k] / cnt[k] as f64;
            if best.is_none_or(|(_, b)| m > b) {
                best = Some((k as u32, m));
            }
        }
    }
    best.map(|b| b.0)
}

/// The walk-forward pass. At the date `t` only the trades with `exit < t` are counted for the choice.
pub fn walk_forward(trades: &[WfTrade], n_rules: usize, min_trades: usize) -> Walk {
    let mut by_exit: Vec<usize> = (0..trades.len()).collect();
    by_exit.sort_by_key(|&i| (trades[i].exit, i));
    let mut by_signal: Vec<usize> = (0..trades.len()).collect();
    by_signal.sort_by_key(|&i| (trades[i].signal, i));
    let (mut sum, mut cnt) = (vec![0.0f64; n_rules], vec![0usize; n_rules]);
    let (mut p, mut q) = (0usize, 0usize);
    let (mut dates, mut chosen, mut picked) = (Vec::new(), Vec::new(), Vec::new());
    while q < by_signal.len() {
        let t = trades[by_signal[q]].signal;
        let mut r = q;
        while r < by_signal.len() && trades[by_signal[r]].signal == t {
            r += 1;
        }
        while p < by_exit.len() && trades[by_exit[p]].exit < t {
            let tr = &trades[by_exit[p]];
            sum[tr.rule as usize] += tr.ret;
            cnt[tr.rule as usize] += 1;
            p += 1;
        }
        let rule = best_rule(&sum, &cnt, min_trades);
        if let Some(k) = rule {
            picked.extend(by_signal[q..r].iter().copied().filter(|&i| trades[i].rule == k));
        }
        dates.push(t);
        chosen.push(rule);
        q = r;
    }
    Walk { dates, chosen, picked }
}

/// The mean, the hit rate and the count of a list of returns.
fn mean_hit(r: &[f64]) -> (f64, f64) {
    if r.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    (r.iter().sum::<f64>() / r.len() as f64, r.iter().filter(|x| **x > 0.0).count() as f64 / r.len() as f64)
}

/// The mean of the monthly means, its standard error, the t statistic and the number of months (months with a trade only).
pub fn monthly_t(picked: &[(i64, f64)]) -> (f64, f64, f64, usize) {
    let mut months: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for (d, r) in picked {
        months.entry(mapcheck::dates::format(*d)[..7].to_string()).or_default().push(*r);
    }
    let m: Vec<f64> = months.values().map(|v| v.iter().sum::<f64>() / v.len() as f64).collect();
    let (mean, se) = crate::dcf::auto_returns::mean_se(&m);
    (mean, se, mean / se, m.len())
}

/// The text of the result.
pub fn report(f: &TradesFile, w: &Walk, min_trades: usize) -> String {
    let t = &f.trades;
    let picked: Vec<(i64, f64)> = w.picked.iter().map(|&i| (t[i].signal, t[i].ret)).collect();
    let rets: Vec<f64> = picked.iter().map(|x| x.1).collect();
    let (mean, hit) = mean_hit(&rets);
    let mut s = String::new();
    let _ = writeln!(s, "WALK-FORWARD OF THE TRADE RULES. {} rules in the grid; minimum {min_trades} closed trades; {} trades in the file; {} signal dates.", f.rule_ids.len(), t.len(), w.dates.len());
    let in_use = w.chosen.iter().filter(|c| c.is_some()).count();
    let mut used: Vec<u32> = w.chosen.iter().flatten().copied().collect();
    used.sort();
    used.dedup();
    let switches = w.chosen.windows(2).filter(|p| p[0].is_some() && p[1].is_some() && p[0] != p[1]).count();
    let _ = writeln!(s, "a rule was in use on {in_use} of {} dates; {} different rules; {switches} changes of the rule.", w.dates.len(), used.len());
    let _ = writeln!(s, "walk-forward trades: {}; mean return of a trade {:+.2}%; hit rate {:.1}%", rets.len(), 100.0 * mean, 100.0 * hit);
    let (mm, se, tt, months) = monthly_t(&picked);
    let _ = writeln!(s, "over {months} calendar months with a trade: mean of the monthly means {:+.2}%, standard error {:.2}%, t {tt:.2} (months of trades overlap in time and trades of a month are correlated: the t is too high)", 100.0 * mm, 100.0 * se);
    let _ = writeln!(s, "by year of the signal:");
    let mut years: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for (d, r) in &picked {
        years.entry(mapcheck::dates::format(*d)[..4].to_string()).or_default().push(*r);
    }
    for (y, v) in &years {
        let (m, h) = mean_hit(v);
        let _ = writeln!(s, "  {y}: {} trades, mean {:+.2}%, hit rate {:.1}%", v.len(), 100.0 * m, 100.0 * h);
    }
    // the in-sample best rule of the whole period
    let (mut sum, mut cnt) = (vec![0.0; f.rule_ids.len()], vec![0usize; f.rule_ids.len()]);
    for x in t {
        sum[x.rule as usize] += x.ret;
        cnt[x.rule as usize] += 1;
    }
    match best_rule(&sum, &cnt, min_trades) {
        Some(k) => {
            let k = k as usize;
            let _ = writeln!(s, "in-sample best rule of the whole period ({IN_SAMPLE_LABEL}): {} with {} trades, mean return of a trade {:+.2}%", f.rule_ids[k], cnt[k], 100.0 * sum[k] / cnt[k] as f64);
        }
        None => {
            let _ = writeln!(s, "in-sample best rule of the whole period ({IN_SAMPLE_LABEL}): no rule has {min_trades} trades");
        }
    }
    let _ = writeln!(s, "{}; {}", harness::UNIVERSE_LABEL, harness::PRICE_LABEL);
    s
}

const KNOWN: [&str; 5] = ["--trades", "--min-trades", "--out", "--cache", "--hypothesis"];

/// `fa portfolio walkforward`.
pub fn command(repo: &Path, args: &[String]) -> Result<(), String> {
    check_options(args, &KNOWN)?;
    refuse_existing(args, &["--out"])?;
    let trades_file = opt(args, "--trades").ok_or("--trades is needed")?;
    let out_file = opt(args, "--out").ok_or("--out is needed")?;
    let min_trades: usize = opt(args, "--min-trades").ok_or("--min-trades is needed")?.parse().map_err(|_| "--min-trades: a whole number".to_string())?;
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let text = std::fs::read_to_string(trades_file).map_err(|e| format!("{trades_file}: {e}"))?;
    let tf = read_trades(&text)?;
    let prov = Provenance::current(repo, &cache);
    check_fresh(&tf.header, &prov)?;
    let (first, last) = tf.trades.iter().fold((i64::MAX, i64::MIN), |(a, b), x| (a.min(x.signal), b.max(x.signal)));
    if tf.trades.is_empty() {
        return Err("the trades file has no trade".into());
    }
    let strategy = harness::WALKFORWARD;

    // The record of the run is written BEFORE the first result exists.
    let (code, dirty) = harness::code_state(repo);
    let mut state: Option<(PathBuf, harness::Counts, bool, bool, Vec<String>, i64)> = None;
    let mut record = serde_json::Value::Null;
    let mut exploratory_note = String::new();
    let grid = serde_json::json!({
        "n_rules": tf.rule_ids.len(), "rules_sha256": tf.header.get("rules_sha256"), "panel_sha256": tf.header.get("panel_sha256"),
        "min_trades": min_trades, "borrow_bp_default": tf.header.get("borrow_bp_default"), "n_trades": tf.trades.len(),
        "panel": { "start": tf.header.get("panel_start"), "end": tf.header.get("panel_end"), "paths": tf.header.get("panel_paths"), "seed": tf.header.get("panel_seed"), "tickers_sha256": tf.header.get("panel_tickers_sha256"), "table_sha256": tf.header.get("panel_table_sha256") },
    });
    match opt(args, "--hypothesis") {
        Some(f) => {
            let proof = harness::prove_committed(repo, &PathBuf::from(f))?;
            let h = Hypothesis::parse(&proof.text)?;
            let stem = Path::new(f).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            if stem != h.id {
                return Err(format!("the id '{}' of the hypothesis is not the file name '{stem}'", h.id));
            }
            if h.strategy != strategy {
                return Err(format!("the strategy of the hypothesis '{}' is '{}', not {strategy}", h.id, h.strategy));
            }
            if !h.params.is_empty() {
                return Err("a trade-walkforward hypothesis has no params (the grid is the rules file; the minimum is logged)".into());
            }
            if let Some(why) = harness::history_state(repo) {
                return Err(format!("the git history of the run logs is incomplete ({why}): a hypothesis run is refused, because the variant count could be too low"));
            }
            // the universe of the file is the tickers file of the panel
            let uni = std::fs::read(h.universe_path(repo)).map_err(|e| format!("universe {}: {e}", h.universe))?;
            if harness::sha256_hex(&uni) != tf.header.get("panel_tickers_sha256") {
                return Err(format!("the universe file of the hypothesis is not the tickers file of the panel (hash {} against {})", harness::sha256_hex(&uni), tf.header.get("panel_tickers_sha256")));
            }
            let day = |v: &Option<String>| v.as_deref().and_then(mapcheck::dates::parse);
            if day(&h.start).is_some_and(|s| first < s) || day(&h.end).is_some_and(|e| last > e) {
                return Err("the signal dates of the trades are outside the start and end of the hypothesis file".into());
            }
            let res = harness::resolve(repo, &h)?;
            let counts = harness::count_runs(repo, &h, &res);
            let read = harness::oos_read(last, res.window_start);
            let (clean, why) = harness::oos_clean(&h, &res.window_id, &counts.earlier_reads, &counts.exploratory_dates, true);
            let stamp = harness::utc_stamp(harness::now_secs());
            record = serde_json::json!({
                "schema": 2, "status": "started", "kind": "trade-walkforward", "id": h.id, "strategy": strategy, "family": res.family, "parent": h.parent, "ancestors": res.chain,
                "run_time_utc": stamp,
                "labels": { "universe": harness::UNIVERSE_LABEL, "prices": harness::PRICE_LABEL },
                "code": { "commit": code, "dirty": dirty },
                "build": { "commit": harness::BUILD_COMMIT, "dirty": harness::BUILD_DIRTY, "is_head": !harness::BUILD_COMMIT.is_empty() && code.as_deref() == Some(harness::BUILD_COMMIT) },
                "store": { "commit": prov.store, "dirty": null },
                "split_table": { "sha256": prov.splits_sha256, "commit": prov.splits_commit },
                "hypothesis": {
                    "file": proof.path, "sha256": proof.sha256, "commit": proof.commit, "commit_time": proof.commit_time, "registered": h.date,
                    "universe": h.universe, "universe_sha256": harness::sha256_hex(&uni), "text": h.hypothesis, "success": h.success, "prior_variants": h.prior_variants,
                },
                "oos_read": read, "oos_clean": clean,
                "oos": { "window_start": mapcheck::dates::format(res.window_start), "window_owner": res.window_id, "reasons_not_clean": why },
                "variant_count": {
                    "hypothesis": counts.variants_id(), "family": counts.variants_family(), "logged_runs_hypothesis": counts.n_id, "logged_runs_family": counts.n_family,
                    "prior_variants_hypothesis": counts.prior_id, "prior_variants_family": counts.prior_family,
                },
                "grid": grid,
                "config": { "command": harness::log_args(repo, args), "start": mapcheck::dates::format(first), "end": mapcheck::dates::format(last), "store": "<store>" },
            });
            let path = harness::write_started(repo, &h.id, &stamp, &record)?;
            eprintln!("run log started: {} (no result is printed before the run ends)", harness::rel_path(repo, &path.to_string_lossy()));
            state = Some((path, counts, read, clean, why, res.window_start));
        }
        None => {
            let reads = harness::exploratory_reads(repo, strategy, last);
            let stamp = harness::utc_stamp(harness::now_secs());
            for read in &reads {
                let rec = serde_json::json!({
                    "kind": "window-read", "run_time_utc": stamp, "code": { "commit": code, "dirty": dirty },
                    "window": { "read_hypothesis": read.id, "owner": read.window_id, "start": mapcheck::dates::format(read.window_start) },
                    "run": { "start": mapcheck::dates::format(first), "end": mapcheck::dates::format(last) },
                    "strategy": strategy, "command": harness::log_args(repo, args), "store": "<store>", "grid": grid,
                });
                let p = harness::write_window_read(repo, read, &stamp, &rec)?;
                let _ = writeln!(exploratory_note, "WARNING: this exploratory run reads the out-of-sample window of '{}' from {}: window read recorded: {} (commit it with your work)", read.id, mapcheck::dates::format(read.window_start), harness::rel_path(repo, &p.to_string_lossy()));
            }
            exploratory_note.push_str("exploratory run: no hypothesis file, not usable for a decision (not counted as a variant)\n");
        }
    }

    let work = || -> Result<(String, String, usize, f64), String> {
        let w = walk_forward(&tf.trades, tf.rule_ids.len(), min_trades);
        let mut lines: Vec<u32> = w.picked.iter().map(|&i| tf.trades[i].line).collect();
        lines.sort_unstable();
        let all: Vec<&str> = text.lines().collect();
        let mut out = format!("{}\n# rule_ids {}\n{}\n", tf.header.line("fa-trades"), tf.rule_ids.join(","), all[2]);
        for l in &lines {
            out.push_str(all[*l as usize]);
            out.push('\n');
        }
        let _ = writeln!(out, "# end n_trades={}", lines.len());
        let rets: Vec<f64> = w.picked.iter().map(|&i| tf.trades[i].ret).collect();
        let mean = if rets.is_empty() { f64::NAN } else { rets.iter().sum::<f64>() / rets.len() as f64 };
        Ok((report(&tf, &w, min_trades), out, rets.len(), mean))
    };
    let outcome = work();
    let finish = |extra: serde_json::Value| -> Result<(), String> {
        let Some((path, ..)) = &state else { return Ok(()) };
        let mut rec = record.clone();
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
        Ok((rep, out, n, mean)) => {
            let mut body = rep;
            body.push_str(&exploratory_note);
            if let Some((path, counts, read, clean, why, window)) = &state {
                let _ = writeln!(body, "hypothesis run: oos-read {read}; oos_clean {clean}; out-of-sample window from {}", mapcheck::dates::format(*window));
                for x in why {
                    let _ = writeln!(body, "  - not clean: {x}");
                }
                let _ = writeln!(body, "runs of this hypothesis: {}; runs of the family: {}; rules in the grid: {} (logged for the deflation of the result)", counts.variants_id(), counts.variants_family(), tf.rule_ids.len());
                let _ = writeln!(body, "run log (done): {}", harness::rel_path(repo, &path.to_string_lossy()));
            }
            if dirty {
                body.push_str("WARNING: the code has changes that are not committed (code_dirty).\n");
            }
            if let Err(e) = write_new(out_file, &out) {
                if let Err(f) = finish(serde_json::json!({ "status": "failed", "error": e })) {
                    eprintln!("fa: the run log: {f}");
                }
                return Err(e);
            }
            let _ = writeln!(body, "walk-forward trades written to {out_file}");
            publish(
                || finish(serde_json::json!({ "status": "done", "results": { "walkforward_trades": n, "mean_return_per_trade": mean, "rules_in_grid": tf.rule_ids.len(), "min_trades": min_trades } })),
                &body,
                &mut |t| print!("{t}"),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> i64 {
        mapcheck::dates::parse(s).unwrap()
    }

    fn t(rule: u32, signal: &str, exit: &str, ret: f64) -> WfTrade {
        WfTrade { rule, signal: d(signal), exit: d(exit), ret, line: 0 }
    }

    /// Rule 0 wins (+10%) and rule 1 loses (-10%) on the trades of 2020-01-03; both close on 2020-01-10.
    fn base() -> Vec<WfTrade> {
        vec![
            t(0, "2020-01-03", "2020-01-10", 0.10),
            t(1, "2020-01-03", "2020-01-10", -0.10),
            t(0, "2020-01-10", "2020-01-20", 0.0),
            t(1, "2020-01-10", "2020-01-20", 0.0),
            t(0, "2020-01-17", "2020-01-30", 0.05),
            t(1, "2020-01-17", "2020-01-30", 0.0),
        ]
    }

    fn chosen(w: &Walk) -> Vec<(String, Option<u32>)> {
        w.dates.iter().map(|x| mapcheck::dates::format(*x)).zip(w.chosen.iter().copied()).collect()
    }

    #[test]
    fn a_rule_is_chosen_only_from_trades_that_closed_before_the_date() {
        let w = walk_forward(&base(), 2, 1);
        // 01-03: nothing closed. 01-10: the trades of 01-03 exit ON 01-10: not before it, so still nothing.
        // 01-17: they have closed (rule 0 +10%, rule 1 -10%) and the trades of 01-10 (exit 01-20) have not.
        assert_eq!(chosen(&w), [("2020-01-03".to_string(), None), ("2020-01-10".to_string(), None), ("2020-01-17".to_string(), Some(0))]);
        // the walk-forward trade is the trade of rule 0 with the signal 2020-01-17
        assert_eq!(w.picked.len(), 1);
        assert_eq!(base()[w.picked[0]].ret, 0.05);
    }

    #[test]
    fn the_minimum_count_keeps_a_rule_with_few_trades_out() {
        let w = walk_forward(&base(), 2, 2);
        assert!(w.chosen.iter().all(|c| c.is_none()));
        assert!(w.picked.is_empty());
    }

    #[test]
    fn a_planted_future_trade_does_not_change_the_choice_at_t() {
        let clean = walk_forward(&base(), 2, 1);
        // plant a huge win for rule 1 that closes ON the date 2020-01-17 and one that closes after it
        let mut planted = base();
        planted.push(t(1, "2020-01-13", "2020-01-17", 5.0));
        planted.push(t(1, "2020-01-14", "2020-01-24", 9.0));
        let w = walk_forward(&planted, 2, 1);
        let at = |w: &Walk, day: &str| w.chosen[w.dates.iter().position(|x| *x == d(day)).unwrap()];
        // the dates of the walk are the signal dates: the planted signals add two dates, but the choice on 01-17 stays rule 0
        assert_eq!(at(&clean, "2020-01-17"), Some(0));
        assert_eq!(at(&w, "2020-01-17"), Some(0));
        // control: the planted trade counts once it has closed (01-24 is after the exit 01-17): the choice changes
        let mut later = planted.clone();
        later.push(t(0, "2020-01-24", "2020-02-07", 0.0));
        let w = walk_forward(&later, 2, 1);
        assert_eq!(at(&w, "2020-01-24"), Some(1));
    }

    #[test]
    fn a_later_trade_changes_no_earlier_choice() {
        let clean = walk_forward(&base(), 2, 1);
        let mut more = base();
        more.push(t(1, "2020-03-02", "2020-03-09", 3.0));
        let w = walk_forward(&more, 2, 1);
        assert_eq!(&w.chosen[..clean.chosen.len()], &clean.chosen[..]);
        assert_eq!(&w.dates[..clean.dates.len()], &clean.dates[..]);
    }

    #[test]
    fn the_monthly_t_has_a_fixed_answer() {
        // January: 0.0 and 0.2 (mean 0.1); February: 0.3. Months: 0.1 and 0.3: mean 0.2, sd 0.1414, se 0.1, t 2
        let p = [(d("2020-01-10"), 0.0), (d("2020-01-17"), 0.2), (d("2020-02-07"), 0.3)];
        let (m, se, tt, n) = monthly_t(&p);
        assert!((m - 0.2).abs() < 1e-12 && (se - 0.1).abs() < 1e-12 && (tt - 2.0).abs() < 1e-9);
        assert_eq!(n, 2);
    }

    #[test]
    fn the_report_labels_the_in_sample_rule_and_chooses_it_with_the_later_data() {
        let mut tr = base();
        // after the walk-forward dates: rule 1 gets large later wins; the in-sample best is rule 1
        tr.push(t(1, "2020-06-05", "2020-06-12", 2.0));
        tr.push(t(1, "2020-06-12", "2020-06-19", 2.0));
        let f = TradesFile { header: Header::default(), rule_ids: vec!["A".into(), "B".into()], trades: tr };
        let w = walk_forward(&f.trades, 2, 1);
        let text = report(&f, &w, 1);
        let line = text.lines().find(|l| l.starts_with("in-sample best rule")).unwrap();
        assert!(line.contains(IN_SAMPLE_LABEL), "{line}");
        assert!(line.contains(": B with 5 trades"), "{line}");
        // the walk-forward chose rule A on 01-17: its trade is in the result, the in-sample rule B is not
        assert!(text.contains("walk-forward trades:"));
        assert!(!text.lines().find(|l| l.starts_with("walk-forward trades")).unwrap().contains("trades: 0;"));
        assert!(text.contains(harness::UNIVERSE_LABEL));
        // no rule has the minimum
        let none = report(&f, &walk_forward(&f.trades, 2, 99), 99);
        assert!(none.contains(&format!("({IN_SAMPLE_LABEL}): no rule has 99 trades")));
    }
}
