//! Robustness grids of `fa trade backtest` for one bot: `--cut-scan` (the OOS result at each cut) and
//! `--price-grid` (the OOS result under the price knobs of A-154). All option prices are MODEL prices.
//! A grid is a screen: it shows how far the result moves, it does not choose a cut or a price setting.

use super::calendar::{iso, Date};
use super::engine::{BacktestResult, Trade};
use super::metrics::{cut_index, summarize};
use super::stats;

/// The footer of both grids.
pub const FOOTER: &str = "a sensitivity, not a choice: no cut or knob is selected; the DSR N stays 20 and is a lower bound";

/// The largest span of the cut scan, in days.
pub const MAX_SPAN: i32 = 365;

/// One row of the cut scan.
#[derive(Debug, Clone, PartialEq)]
pub struct CutRow {
    pub date: Date,
    pub days_from_default: i32,
    pub oos_trades: usize,
    pub oos_sharpe: f64,
    pub t_pnl: f64,
}

/// Bar positions of the cuts: the default cut (`is_fraction`) moved by whole steps of `step` calendar
/// days, up to `span` days each way. A cut is the first bar on or after the target date. A cut at bar 0
/// (no in-sample part) or a repeated bar is dropped. Returns (bar, days from the default cut).
pub fn cut_positions(dates: &[Date], is_fraction: f64, step: i32, span: i32) -> Vec<(usize, i32)> {
    let n = dates.len();
    if n < 2 || step < 1 {
        return Vec::new();
    }
    let base = dates[cut_index(n, is_fraction)];
    let m = span.max(0) / step;
    let mut out: Vec<(usize, i32)> = Vec::new();
    for k in -m..=m {
        let target = base + k * step;
        let p = dates.partition_point(|&d| d < target);
        if p >= 1 && p < n && out.last().is_none_or(|l| l.0 != p) {
            out.push((p, p_days(dates, p, base)));
        }
    }
    out
}

fn p_days(dates: &[Date], p: usize, base: Date) -> i32 {
    dates[p] - base
}

/// The OOS trade P&Ls of the cut at bar `p` (a trade counts when it enters on or after the cut bar).
fn oos_pnl(r: &BacktestResult, p: usize) -> Vec<f64> {
    let key = r.date[p] as i64 * 1440 + r.minute[p] as i64;
    r.trades.iter().filter(|t: &&Trade| t.entry_time.key() >= key).map(|t| t.pnl).collect()
}

/// The OOS Sharpe (annual, of the equity) and the t of the trade P&L at each cut.
pub fn cut_scan(r: &BacktestResult, interval: &'static str, is_fraction: f64, step: i32, span: i32) -> Vec<CutRow> {
    let n = r.equity.len();
    cut_positions(&r.date, is_fraction, step, span)
        .into_iter()
        .map(|(p, dd)| {
            // The fraction (p + 0.5) / n gives `cut_index` = p exactly, so `summarize` splits at bar p.
            let s = summarize(r, interval, (p as f64 + 0.5) / n as f64);
            let pnl = oos_pnl(r, p);
            CutRow { date: r.date[p], days_from_default: dd, oos_trades: pnl.len(), oos_sharpe: s.oos.sharpe, t_pnl: stats::t_mean(&pnl) }
        })
        .collect()
}

pub fn format_cut_scan(first_bar: Date, default_cut: Date, rows: &[CutRow], step: i32, span: i32) -> String {
    let mut s = format!("first bar of the data {}; default cut {} (the in-sample fraction)\n", iso(first_bar), iso(default_cut));
    s.push_str(&format!("{:>12} {:>9} {:>11} {:>11} {:>9}\n", "cut", "days", "oos_trades", "oos_sharpe", "t(P&L)"));
    for r in rows {
        s.push_str(&format!("{:>12} {:>+9} {:>11} {:>11.2} {:>9.2}\n", iso(r.date), r.days_from_default, r.oos_trades, r.oos_sharpe, r.t_pnl));
    }
    let sh: Vec<f64> = rows.iter().map(|r| r.oos_sharpe).filter(|x| x.is_finite()).collect();
    if !sh.is_empty() {
        let (lo, hi) = sh.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &x| (a.0.min(x), a.1.max(x)));
        s.push_str(&format!("OOS Sharpe range over the {} cuts within +/- {span} days, step {step}: {lo:.2} to {hi:.2} (same trades, same data; only the cut moves; it depends on the step, the span and the end of the data, so it is not the range)\n", sh.len()));
    }
    s.push_str(&format!("{} (step {step} days, span {span} days, {} cuts)\n", FOOTER, rows.len()));
    s
}

/// The price knobs of A-154: (label, spread x, skew x, mid fill). The first is the baseline.
pub const KNOB_SET: [(&str, f64, f64, bool); 5] =
    [("baseline", 1.0, 1.0, false), ("spread x2", 2.0, 1.0, false), ("skew x1.5", 1.0, 1.5, false), ("skew x0.5", 1.0, 0.5, false), ("mid fill (better)", 1.0, 1.0, true)];

/// One row of the price grid, from the OOS part of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct GridRow {
    pub label: &'static str,
    pub oos_trades: usize,
    pub oos_return_pct: f64,
    pub oos_sharpe: f64,
    pub mean_pnl: f64,
    pub t_pnl: f64,
    /// The mean P&L of a trade that gives t = 2 at this n and this sd: 2 sd / sqrt(n).
    pub need_t2: f64,
}

pub fn grid_row(label: &'static str, r: &BacktestResult, interval: &'static str, is_fraction: f64) -> GridRow {
    let n = r.equity.len();
    let s = summarize(r, interval, is_fraction);
    let pnl = if n == 0 { Vec::new() } else { oos_pnl(r, cut_index(n, is_fraction)) };
    let need_t2 = if pnl.len() < 2 { f64::NAN } else { 2.0 * stats::std(&pnl, 1) / (pnl.len() as f64).sqrt() };
    GridRow {
        label,
        oos_trades: pnl.len(),
        oos_return_pct: s.oos.return_pct,
        oos_sharpe: s.oos.sharpe,
        mean_pnl: if pnl.is_empty() { f64::NAN } else { stats::mean(&pnl) },
        t_pnl: stats::t_mean(&pnl),
        need_t2,
    }
}

/// Run the bot once for each knob set. `set` sets the process-wide knobs (`pricing::set_price_knobs`);
/// the knobs go back to `restore` after each run, also after an error. `run` makes one backtest.
pub fn price_grid(
    set: &dyn Fn(f64, f64, bool),
    restore: (f64, f64, bool),
    run: &mut dyn FnMut() -> Result<BacktestResult, String>,
    interval: &'static str,
    is_fraction: f64,
) -> Result<Vec<GridRow>, String> {
    let mut rows = Vec::new();
    for (label, sp, sk, mid) in KNOB_SET {
        set(sp, sk, mid);
        let r = run();
        set(restore.0, restore.1, restore.2);
        rows.push(grid_row(label, &r?, interval, is_fraction));
    }
    Ok(rows)
}

pub fn format_price_grid(rows: &[GridRow]) -> String {
    let mut s = format!(
        "{:<18} {:>6} {:>8} {:>7} {:>9} {:>7} {:>10} {:>8} {:>8} {:>10}\n",
        "run", "trades", "oos_ret%", "sharpe", "mean_pnl", "t(P&L)", "d_mean", "d_mean%", "d_t", "need_t2"
    );
    let base = rows.first();
    for r in rows {
        let (dm, dp, dt) = match base {
            Some(b) if r.label != b.label => (r.mean_pnl - b.mean_pnl, 100.0 * (r.mean_pnl / b.mean_pnl - 1.0), r.t_pnl - b.t_pnl),
            _ => (0.0, 0.0, 0.0),
        };
        s.push_str(&format!(
            "{:<18} {:>6} {:>8.1} {:>7.2} {:>9.2} {:>7.2} {:>+10.2} {:>+8.1} {:>+8.2} {:>10.2}\n",
            r.label, r.oos_trades, r.oos_return_pct, r.oos_sharpe, r.mean_pnl, r.t_pnl, dm, dp, dt, r.need_t2
        ));
    }
    s.push_str("d_mean, d_mean%, d_t: change against the baseline row. need_t2: the mean P&L of a trade (dollars) that gives t = 2 with this n and this sd.\n");
    s.push_str("The rows can have different trade sets: a price change can add, drop or move a trade (for example s08: 3, 1, 5 and 64 trades), so d_mean compares different samples.\n");
    s.push_str(&format!("{} ({} runs)\n", FOOTER, rows.len()));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::engine::{RiskConfig, Time};

    /// 20 daily bars, dates 20000 to 20019; the equity of bar k is 100 + k^2 (rising); one trade enters on
    /// each bar k with P&L k.
    fn synthetic() -> BacktestResult {
        let trades = (0..20)
            .map(|k| Trade {
                id: k as u64,
                strategy: "t",
                symbol: "SPY",
                structure: "x",
                description: String::new(),
                tag: String::new(),
                units: 1,
                entry_time: Time { date: 20_000 + k, minute: 0 },
                exit_time: Time { date: 20_000 + k, minute: 0 },
                entry_net: 0.0,
                exit_net: 0.0,
                max_loss: 0.0,
                pnl: k as f64,
                exit_reason: "x".into(),
            })
            .collect();
        BacktestResult {
            strategy: "t",
            equity: (0..20).map(|k| 100.0 + (k * k) as f64).collect(),
            date: (0..20).map(|k| 20_000 + k).collect(),
            minute: vec![0; 20],
            utc: (0..20i64).map(|k| (20_000 + k) * 86_400).collect(),
            trades,
            open_positions: vec![],
            pending: vec![],
            rejections: vec![],
            pdt_blocks: 0,
            risk: RiskConfig::default(),
        }
    }

    #[test]
    fn the_cut_scan_of_a_synthetic_series_has_a_fixed_answer() {
        let r = synthetic();
        // Default cut: bar 10 (fraction 0.5). Step 2 days, span 4 days: the cuts are bars 6, 8, 10, 12, 14.
        let rows = cut_scan(&r, "1d", 0.5, 2, 4);
        let bars: Vec<i32> = rows.iter().map(|x| x.date - 20_000).collect();
        assert_eq!(bars, vec![6, 8, 10, 12, 14]);
        assert_eq!(rows.iter().map(|x| x.days_from_default).collect::<Vec<_>>(), vec![-4, -2, 0, 2, 4]);
        // The trades k = p..19 are OOS: n = 20 - p, mean (p + 19) / 2, sd = sqrt(n (n + 1) / 12).
        // p = 10: n 10, mean 14.5, sd 3.02765, t = 14.5 / (3.02765 / sqrt 10) = 15.1448.
        assert_eq!(rows[2].oos_trades, 10);
        assert!((rows[2].t_pnl - 15.1448).abs() < 1e-3, "{}", rows[2].t_pnl);
        // p = 6: n 14, mean 12.5, sd = sqrt(14 x 15 / 12) = 4.18330, t = 12.5 / (4.18330 / sqrt 14) = 11.1803.
        assert_eq!(rows[0].oos_trades, 14);
        assert!((rows[0].t_pnl - 11.1803).abs() < 1e-3, "{}", rows[0].t_pnl);
        // The Sharpe at the default cut is the Sharpe of `summarize` with the same fraction.
        assert_eq!(rows[2].oos_sharpe, summarize(&r, "1d", 0.5).oos.sharpe);
        assert!(rows[0].oos_sharpe != rows[4].oos_sharpe);
        let text = format_cut_scan(r.date[0], r.date[10], &rows, 2, 4);
        assert!(text.contains("first bar of the data 2024-10-04"), "{text}");
        assert!(text.contains("OOS Sharpe range over the 5 cuts within +/- 4 days, step 2"), "{text}");
        assert!(text.contains(FOOTER) && text.contains("step 2 days, span 4 days, 5 cuts"));
    }

    #[test]
    fn cuts_outside_the_data_are_dropped() {
        let d: Vec<Date> = (0..10).map(|k| 100 + k).collect();
        // Default bar 5; step 3, span 30: targets 5-3*10... only bars 1..9 are valid, no repeat.
        let c = cut_positions(&d, 0.5, 3, 30);
        assert!(c.iter().all(|&(p, _)| (1..10).contains(&p)));
        assert!(c.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(c.iter().map(|x| x.0).collect::<Vec<_>>(), vec![2, 5, 8]);
        assert!(cut_positions(&d, 0.5, 0, 30).is_empty() && cut_positions(&d[..1], 0.5, 1, 5).is_empty());
    }

    #[test]
    fn the_power_figure_and_the_price_grid_have_fixed_answers() {
        // P&L 0..19 all OOS at fraction 0.05 (cut bar 1: P&L 1..19, n 19, mean 10, sd = sqrt(19 x 20 / 12) = 5.62731).
        // need_t2 = 2 x 5.62731 / sqrt 19 = 2.58199; t = 10 / (5.62731 / sqrt 19) = 7.74597.
        let g = grid_row("x", &synthetic(), "1d", 0.05);
        assert_eq!(g.oos_trades, 19);
        assert!((g.mean_pnl - 10.0).abs() < 1e-12 && (g.t_pnl - 7.74597).abs() < 1e-4 && (g.need_t2 - 2.58199).abs() < 1e-4, "{g:?}");
        // t = 2 at the needed mean: mean / need_t2 = t / 2.
        assert!((g.mean_pnl / g.need_t2 - g.t_pnl / 2.0).abs() < 1e-9);

        // The grid runs the closure five times, sets each knob set, and restores the knobs after each run.
        let log = std::cell::RefCell::new(Vec::<(f64, f64, bool)>::new());
        let set = |a: f64, b: f64, c: bool| log.borrow_mut().push((a, b, c));
        let mut runs = 0;
        let mut run = || {
            runs += 1;
            // The pnl of the run is scaled by the run number: the baseline has the smallest mean.
            let mut r = synthetic();
            for t in r.trades.iter_mut() {
                t.pnl *= runs as f64;
            }
            Ok(r)
        };
        let rows = price_grid(&set, (1.0, 1.0, false), &mut run, "1d", 0.5).unwrap();
        assert_eq!(rows.len(), 5);
        let l = log.borrow();
        assert_eq!(l.len(), 10);
        assert_eq!((l[0], l[1]), ((1.0, 1.0, false), (1.0, 1.0, false)));
        assert_eq!((l[2], l[3]), ((2.0, 1.0, false), (1.0, 1.0, false)));
        assert_eq!((l[4], l[5]), ((1.0, 1.5, false), (1.0, 1.0, false)));
        assert_eq!((l[6], l[7]), ((1.0, 0.5, false), (1.0, 1.0, false)));
        assert_eq!((l[8], l[9]), ((1.0, 1.0, true), (1.0, 1.0, false)));
        // OOS trades k = 10..19, mean 14.5 x run number.
        assert!((rows[0].mean_pnl - 14.5).abs() < 1e-12 && (rows[1].mean_pnl - 29.0).abs() < 1e-12);
        let text = format_price_grid(&rows);
        assert!(text.contains("spread x2") && text.contains("+14.50"), "{text}");
        assert!(text.contains(FOOTER) && text.contains("(5 runs)") && text.contains("different trade sets"));
        // An error of the run still restores the knobs.
        drop(l);
        log.borrow_mut().clear();
        let mut bad = || Err("boom".to_string());
        assert_eq!(price_grid(&set, (1.0, 1.0, false), &mut bad, "1d", 0.5).unwrap_err(), "boom");
        assert_eq!(log.borrow().last(), Some(&(1.0, 1.0, false)));
    }
}
