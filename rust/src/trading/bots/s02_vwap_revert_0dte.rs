//! s02 - Intraday VWAP mean reversion with 0DTE $1-wide credit spreads (QQQ, hourly bars).
//!
//! The session VWAP is built from the bars of today (typical price times volume). When the
//! close is more than K times the VIX1D-implied daily 1-sigma move away from the VWAP, in a
//! session that has NOT trended (efficiency ratio below ER_MAX), sell a 0DTE credit spread
//! (short leg at 0.50 delta) that bets on a move back toward the VWAP. Exit when the price
//! touches the VWAP again, on the engine take profit and stop, or 60 minutes before the close.
//! (The Python bot also had a debit mode. It was off, and it is not in the Rust code.)

use crate::trading::calendar::Date;
use crate::trading::data::Interval;
use crate::trading::engine::{BarView, Ctx};
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const K: f64 = 0.25;
const ER_MAX: f64 = 0.6;
const FIRST_BAR: usize = 1;
const LAST_BAR: usize = 4;
const WIDTH: f64 = 1.0;
const TAKE_PROFIT: f64 = 0.5;
const STOP_LOSS: f64 = 1.0;
const MINUTES_BEFORE_CLOSE: i32 = 60;
const CREDIT_DELTA: f64 = 0.50;
const VWAP_EXIT_BUFFER: f64 = 0.0;
const QQQ: &str = "QQQ";

/// Volume-weighted typical price of the bars (the mean close when the volume is 0).
pub fn vwap(tb: &BarView) -> f64 {
    let tp: Vec<f64> = (0..tb.len()).map(|k| (tb.high[k] + tb.low[k] + tb.close[k]) / 3.0).collect();
    let tpv: Vec<f64> = (0..tb.len()).map(|k| tp[k] * tb.volume[k]).collect();
    let vol = stats::sum(tb.volume);
    if vol > 0.0 {
        stats::sum(&tpv) / vol
    } else {
        stats::mean(tb.close)
    }
}

#[derive(Default)]
pub struct VwapRevert0Dte {
    traded_day: Option<Date>,
}

impl Strategy for VwapRevert0Dte {
    fn name(&self) -> &'static str {
        "s02_vwap_revert_0dte"
    }

    fn symbols(&self) -> &'static [&'static str] {
        &[QQQ]
    }

    fn interval(&self) -> Interval {
        Interval::Hourly
    }

    fn warmup_bars(&self) -> usize {
        10
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        let tb = ctx.today_bars(QQQ);
        let n = tb.len();
        if n < 2 {
            return Ok(());
        }
        let vwap = vwap(&tb);
        let px = ctx.spot(QQQ);
        let sd = ctx.vol(QQQ).short / 252f64.sqrt(); // the daily 1-sigma move, as a fraction

        // Exits: the VWAP touch (the reversion is complete).
        for pos in ctx.positions() {
            let touched = (pos.tag == "long_rev" && px >= vwap * (1.0 - VWAP_EXIT_BUFFER * sd)) || (pos.tag == "short_rev" && px <= vwap * (1.0 + VWAP_EXIT_BUFFER * sd));
            if touched {
                ctx.close(&pos, "vwap_touch");
            }
        }

        if ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        let j = n - 1;
        if j < FIRST_BAR || j > LAST_BAR {
            return Ok(());
        }
        let today = ctx.today();
        if self.traded_day == Some(today) {
            return Ok(());
        }
        let hi = tb.high.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let lo = tb.low.iter().copied().fold(f64::INFINITY, f64::min);
        let rng = hi - lo;
        let er = if rng > 0.0 { (px - tb.open[0]).abs() / rng } else { 0.0 };
        if er > ER_MAX {
            return Ok(());
        }
        let z = if sd > 0.0 { (px - vwap) / (px * sd) } else { 0.0 };
        if z.abs() < K {
            return Ok(());
        }
        let exp = ctx.expiry(0);
        let exits = ExitRules { take_profit: Some(TAKE_PROFIT), stop_loss: Some(STOP_LOSS), minutes_before_close: Some(MINUTES_BEFORE_CLOSE), ..Default::default() };
        // Below the VWAP: expect up or flat, so sell a put credit spread (short put below spot).
        let (st, tag) = if z < 0.0 {
            let k = ctx.strike_for_delta(QQQ, false, CREDIT_DELTA, exp, 1.0);
            (vertical(QQQ, exp, false, k - WIDTH, k)?, "long_rev")
        } else {
            let k = ctx.strike_for_delta(QQQ, true, CREDIT_DELTA, exp, 1.0);
            (vertical(QQQ, exp, true, k + WIDTH, k)?, "short_rev")
        };
        ctx.open(st, exits, tag, None);
        self.traded_day = Some(today);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vwap_hand_value() {
        // Typical prices 10 and 12 with volumes 1 and 3: (10 + 36) / 4 = 11.5. Zero volume gives the mean close.
        let bars = |v: &'static [f64]| BarView {
            open: &[0.0, 0.0],
            high: &[11.0, 13.0],
            low: &[9.0, 11.0],
            close: &[10.0, 12.0],
            volume: v,
            minute: &[570, 630],
        };
        assert!((vwap(&bars(&[1.0, 3.0])) - 11.5).abs() < 1e-12);
        assert!((vwap(&bars(&[0.0, 0.0])) - 11.0).abs() < 1e-12);
    }
}
