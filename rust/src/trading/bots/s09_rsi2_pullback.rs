//! s09 - Connors-style RSI(2) pullback in an uptrend, with short-dated call debit spreads.
//!
//! Signal (daily, for each symbol, at the close): close > SMA(200) and RSI(2) < 5.
//! Entry at the next open: a $1 call debit spread with a ~0.50-delta long leg, 10 sessions to
//! expiry. Exit (the first of): close > SMA(5) or RSI > 65 (never on the fill day, to save the
//! pattern-day-trader slots), 7 bars held, or 1 session before expiry.

use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const TREND_LEN: usize = 200;
const RSI_LEN: usize = 2;
const RSI_ENTRY: f64 = 5.0;
const RSI_EXIT: f64 = 65.0;
const EXIT_MA_LEN: usize = 5;
const DTE: i32 = 10;
const LONG_DELTA: f64 = 0.50;
const WIDTH: f64 = 1.0;
const MAX_HOLD: i32 = 7;
const EXIT_DTE: i32 = 1;
const LOOKBACK: usize = 60;

/// Wilder RSI of the last value (seeded with a simple mean).
fn rsi(close: &[f64], n: usize) -> f64 {
    let d: Vec<f64> = close.windows(2).map(|w| w[1] - w[0]).collect();
    if d.len() < n {
        return 50.0;
    }
    let up: Vec<f64> = d.iter().map(|&x| x.max(0.0)).collect();
    let dn: Vec<f64> = d.iter().map(|&x| (-x).max(0.0)).collect();
    let (mut au, mut ad) = (stats::mean(&up[..n]), stats::mean(&dn[..n]));
    let nf = n as f64;
    for (u, v) in up[n..].iter().zip(&dn[n..]) {
        au = (au * (nf - 1.0) + u) / nf;
        ad = (ad * (nf - 1.0) + v) / nf;
    }
    if ad <= 1e-12 {
        return 100.0;
    }
    100.0 - 100.0 / (1.0 + au / ad)
}

#[derive(Default)]
pub struct Rsi2Pullback;

impl Strategy for Rsi2Pullback {
    fn name(&self) -> &'static str {
        "s09_rsi2_pullback"
    }

    fn symbols(&self) -> &'static [&'static str] {
        &["SPY", "QQQ"]
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn warmup_bars(&self) -> usize {
        210
    }

    fn start(&self) -> Option<&'static str> {
        Some("2004-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        let held = ctx.positions();
        for sym in self.symbols().iter().copied() {
            let c = ctx.bars(sym).close;
            if c.len() < TREND_LEN + 1 || c[c.len() - TREND_LEN..].iter().any(|x| x.is_nan()) {
                continue;
            }
            let close = c[c.len() - 1];
            let sma_trend = stats::mean(&c[c.len() - TREND_LEN..]);
            let sma_exit = stats::mean(&c[c.len() - EXIT_MA_LEN..]);
            let r = rsi(&c[c.len().saturating_sub(LOOKBACK)..], RSI_LEN);
            if let Some(pos) = held.iter().rev().find(|p| p.structure.symbol == sym) {
                if pos.entry_time.date == ctx.today() {
                    continue;
                }
                if close > sma_exit || r > RSI_EXIT {
                    ctx.close(pos, "rsi_or_ma_recovery");
                }
                continue;
            }
            if close > sma_trend && r < RSI_ENTRY {
                let exp = ctx.expiry(DTE + 1); // +1: the fill happens next session
                let k = ctx.strike_for_delta(sym, true, LONG_DELTA, exp, 1.0);
                let exits = ExitRules { exit_dte: Some(EXIT_DTE), max_hold_bars: Some(MAX_HOLD), ..Default::default() };
                ctx.open(vertical(sym, exp, true, k, k + WIDTH)?, exits, &format!("rsi{:.0}", r), None);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsi_hand_values() {
        // Changes 1, -0.5, 0.3, -0.6. Wilder smoothing with n = 2 starts from the mean of the first 2.
        // avg up: 0.5 -> 0.4 -> 0.2; avg down: 0.25 -> 0.125 -> 0.3625; RSI = 100 - 100 / (1 + 0.2 / 0.3625).
        let r = rsi(&[10.0, 11.0, 10.5, 10.8, 10.2], 2);
        assert!((r - (100.0 - 100.0 / (1.0 + 0.2 / 0.3625))).abs() < 1e-12);
        assert_eq!(rsi(&[1.0, 2.0, 3.0, 4.0], 2), 100.0); // no down moves
        assert_eq!(rsi(&[1.0, 2.0], 2), 50.0); // too short
    }
}
