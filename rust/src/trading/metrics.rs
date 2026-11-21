//! Performance statistics with an in-sample / out-of-sample split.
//!
//! The split is chronological (the first 70% of the bars are in-sample by default). A bot is
//! TUNED on in-sample data only; the out-of-sample columns are the honest result.

use super::calendar::iso;
use super::engine::{BacktestResult, Trade};
use super::stats;

#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub return_pct: f64,
    pub cagr_pct: f64,
    pub max_dd_pct: f64,
    pub sharpe: f64,
    pub trades: usize,
    pub win_rate_pct: f64,
    pub profit_factor: f64,
    pub avg_pnl: f64,
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub strategy: &'static str,
    pub interval: &'static str,
    pub start: String,
    pub end: String,
    pub oos_start: String,
    pub all: Stats,
    pub is: Stats,
    pub oos: Stats,
    pub final_equity: f64,
    pub rejected: u32,
    pub pdt_blocks: u32,
}

/// `equity` with the UTC second of each bar.
fn equity_stats(equity: &[f64], utc: &[i64], date: &[i32]) -> (f64, f64, f64, f64) {
    if equity.len() < 2 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    // Last equity of each day, then the daily returns (pandas `pct_change().dropna()`).
    let mut daily: Vec<f64> = Vec::new();
    for k in 0..equity.len() {
        if k + 1 == equity.len() || date[k] != date[k + 1] {
            daily.push(equity[k]);
        }
    }
    let rets: Vec<f64> = daily.windows(2).map(|w| w[1] / w[0] - 1.0).filter(|r| !r.is_nan()).collect();
    let first = equity[0];
    let last = equity[equity.len() - 1];
    let total = last / first - 1.0;
    let days = (utc[utc.len() - 1] - utc[0]).div_euclid(86_400) as f64;
    let years = (days / 365.25).max(1.0 / 365.25);
    let cagr = if last > 0.0 { (last / first).powf(1.0 / years) - 1.0 } else { -1.0 };
    let mut peak = f64::NEG_INFINITY;
    let mut dd = f64::INFINITY;
    for &e in equity {
        peak = peak.max(e);
        dd = dd.min(e / peak - 1.0);
    }
    let sd = if rets.len() > 1 { stats::std(&rets, 1) } else { f64::NAN };
    let sharpe = if rets.len() > 1 && sd > 0.0 { stats::mean(&rets) / sd * 252f64.sqrt() } else { 0.0 };
    (100.0 * total, 100.0 * cagr, 100.0 * dd, sharpe)
}

fn trade_stats(trades: &[&Trade]) -> (usize, f64, f64, f64) {
    if trades.is_empty() {
        return (0, 0.0, 0.0, 0.0);
    }
    let pnl: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    let wins = stats::sum(&pnl.iter().copied().filter(|&p| p > 0.0).collect::<Vec<_>>());
    let losses = -stats::sum(&pnl.iter().copied().filter(|&p| p < 0.0).collect::<Vec<_>>());
    let win_share = pnl.iter().filter(|&&p| p > 0.0).count() as f64 / pnl.len() as f64;
    let pf = if losses > 0.0 {
        wins / losses
    } else if wins > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };
    (trades.len(), 100.0 * win_share, pf, stats::mean(&pnl))
}

fn part(r: &BacktestResult, from: usize, to: usize, trades: &[&Trade]) -> Stats {
    let (return_pct, cagr_pct, max_dd_pct, sharpe) = equity_stats(&r.equity[from..to], &r.utc[from..to], &r.date[from..to]);
    let (n, win_rate_pct, profit_factor, avg_pnl) = trade_stats(trades);
    Stats { return_pct, cagr_pct, max_dd_pct, sharpe, trades: n, win_rate_pct, profit_factor, avg_pnl }
}

pub fn summarize(r: &BacktestResult, interval: &'static str, is_fraction: f64) -> Summary {
    let n = r.equity.len();
    let cut_pos = if n == 0 { None } else { Some((n as f64 * is_fraction) as usize) };
    let cut_key = cut_pos.map(|p| r.date[p] as i64 * 1440 + r.minute[p] as i64);
    let all_trades: Vec<&Trade> = r.trades.iter().collect();
    let is_trades: Vec<&Trade> = r.trades.iter().filter(|t| cut_key.is_none_or(|c| t.entry_time.key() < c)).collect();
    let oos_trades: Vec<&Trade> = r.trades.iter().filter(|t| cut_key.is_some_and(|c| t.entry_time.key() >= c)).collect();
    let (is_to, oos_from) = match cut_pos {
        Some(p) => (p, p),
        None => (n, n),
    };
    Summary {
        strategy: r.strategy,
        interval,
        start: if n > 0 { iso(r.date[0]) } else { String::new() },
        end: if n > 0 { iso(r.date[n - 1]) } else { String::new() },
        oos_start: cut_pos.map_or(String::new(), |p| iso(r.date[p])),
        all: part(r, 0, n, &all_trades),
        is: part(r, 0, is_to, &is_trades),
        oos: part(r, oos_from, n, &oos_trades),
        final_equity: if n > 0 { r.equity[n - 1] } else { r.risk.starting_equity },
        rejected: r.rejections.iter().map(|(_, c)| c).sum(),
        pdt_blocks: r.pdt_blocks,
    }
}

/// Python `f"{x:,.0f}"`.
fn thousands(x: f64) -> String {
    let s = format!("{:.0}", x.abs());
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if x < 0.0 && s != "0" {
        out.insert(0, '-');
    }
    out
}

pub fn format_summary(r: &Summary) -> String {
    format!(
        "{}: {}→{}  final ${}\n  ALL  ret {:+.1}%  CAGR {:+.1}%  maxDD {:.1}%  Sharpe {:.2}  trades {}  win {:.0}%  PF {:.2}\n  OOS (from {})  ret {:+.1}%  maxDD {:.1}%  Sharpe {:.2}  trades {}  win {:.0}%  PF {:.2}\n  rejected entries {}  PDT-blocked exits {}",
        r.strategy, r.start, r.end, thousands(r.final_equity),
        r.all.return_pct, r.all.cagr_pct, r.all.max_dd_pct, r.all.sharpe, r.all.trades, r.all.win_rate_pct, r.all.profit_factor,
        r.oos_start, r.oos.return_pct, r.oos.max_dd_pct, r.oos.sharpe, r.oos.trades, r.oos.win_rate_pct, r.oos.profit_factor,
        r.rejected, r.pdt_blocks
    )
}

/// The daily returns of the equity curve for the bars from the in-sample cut to the end (the
/// out-of-sample part of `summarize`), or of all bars when `oos` is false.
pub fn daily_returns(r: &BacktestResult, is_fraction: f64, oos: bool) -> Vec<f64> {
    let n = r.equity.len();
    let from = if oos && n > 0 { (n as f64 * is_fraction) as usize } else { 0 };
    let mut daily: Vec<f64> = Vec::new();
    for k in from..n {
        if k + 1 == n || r.date[k] != r.date[k + 1] {
            daily.push(r.equity[k]);
        }
    }
    daily.windows(2).map(|w| w[1] / w[0] - 1.0).filter(|x| !x.is_nan()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::engine::RiskConfig;

    fn result(equity: &[f64], pnls: &[(f64, i64)]) -> BacktestResult {
        use crate::trading::engine::Time;
        let n = equity.len();
        let trades = pnls
            .iter()
            .enumerate()
            .map(|(k, &(pnl, key))| Trade {
                id: k as u64,
                strategy: "t",
                symbol: "SPY",
                structure: "x",
                description: String::new(),
                tag: String::new(),
                units: 1,
                entry_time: Time { date: (key / 1440) as i32, minute: (key % 1440) as i32 },
                exit_time: Time { date: (key / 1440) as i32, minute: 0 },
                entry_net: 0.0,
                exit_net: 0.0,
                max_loss: 0.0,
                pnl,
                exit_reason: "x".into(),
            })
            .collect();
        BacktestResult {
            strategy: "t",
            equity: equity.to_vec(),
            date: (0..n as i32).map(|d| 20_000 + d).collect(),
            minute: vec![0; n],
            utc: (0..n as i64).map(|d| (20_000 + d) * 86_400).collect(),
            trades,
            open_positions: vec![],
            pending: vec![],
            rejections: vec![("too_risky_for_account", 2), ("max_total_risk", 1)],
            pdt_blocks: 4,
            risk: RiskConfig::default(),
        }
    }

    #[test]
    fn equity_and_trade_statistics() {
        // 11 daily bars, +10% then -10% ... checked by hand.
        let eq = [100.0, 110.0, 99.0, 99.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0];
        let r = result(&eq, &[(10.0, 20_000 * 1440), (-5.0, 20_001 * 1440), (15.0, 20_009 * 1440)]);
        let s = summarize(&r, "1d", 0.7);
        assert!((s.all.return_pct - 20.0).abs() < 1e-12);
        // Peak 110, trough 99: drawdown -10%.
        assert!((s.all.max_dd_pct + 10.0).abs() < 1e-12);
        assert_eq!(s.all.trades, 3);
        assert!((s.all.win_rate_pct - 200.0 / 3.0).abs() < 1e-12);
        assert!((s.all.profit_factor - 5.0).abs() < 1e-12); // 25 / 5
        assert!((s.all.avg_pnl - 20.0 / 3.0).abs() < 1e-12);
        // Years: 10 days -> max(10/365.25, 1/365.25); CAGR = 1.2^(365.25/10) - 1.
        assert!((s.all.cagr_pct - 100.0 * (1.2f64.powf(365.25 / 10.0) - 1.0)).abs() < 1e-6);
        // The split is at bar 7 (int(11 * 0.7)): the third trade (bar 9) is out-of-sample.
        assert_eq!(s.oos_start, iso(20_007));
        assert_eq!(s.is.trades, 2);
        assert_eq!(s.oos.trades, 1);
        assert_eq!(s.rejected, 3);
        assert_eq!(s.pdt_blocks, 4);
        // Sharpe of the daily returns [0.1, -0.1, 0, 0.2121.., 0, ...].
        let rets = [110.0 / 100.0 - 1.0, 99.0 / 110.0 - 1.0, 0.0, 120.0 / 99.0 - 1.0];
        let rets: Vec<f64> = rets.iter().copied().chain(std::iter::repeat_n(0.0, 6)).collect();
        let m = rets.iter().sum::<f64>() / 10.0;
        let sd = (rets.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / 9.0).sqrt();
        assert!((s.all.sharpe - m / sd * 252f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn empty_and_flat_inputs() {
        let s = summarize(&result(&[], &[]), "1d", 0.7);
        assert_eq!(s.final_equity, 2000.0);
        assert_eq!(s.all.trades, 0);
        let flat = summarize(&result(&[100.0; 5], &[]), "1d", 0.7);
        assert_eq!(flat.all.sharpe, 0.0);
        assert_eq!(flat.all.return_pct, 0.0);
    }

    #[test]
    fn number_format_like_python() {
        assert_eq!(thousands(1_234_567.4), "1,234,567");
        assert_eq!(thousands(999.5), "1,000");
        assert_eq!(thousands(-1500.0), "-1,500");
        assert_eq!(thousands(0.2), "0");
    }
}
