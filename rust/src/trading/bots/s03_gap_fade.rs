//! s03 - Overnight gap fade: fade moderate opening gaps (0.5 to 1.5 times the VIX-implied daily
//! move) after the first hour with a 0DTE debit spread. Gap up gives a put debit spread, gap
//! down gives a call debit spread. Exit at the gap fill, at the stop, or by time.

use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::{round_strike, vertical};

const MIN_Z: f64 = 0.5;
const MAX_Z: f64 = 1.5;
const MIN_REMAIN: f64 = 0.5;
const WIDTH: f64 = 1.0;
const TAKE_PROFIT: f64 = 0.8;
const STOP_LOSS: f64 = 0.5;
const MINUTES_BEFORE_CLOSE: i32 = 60;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct GapFade {
    prev: Option<f64>,
}

impl Strategy for GapFade {
    fn name(&self) -> &'static str {
        "s03_gap_fade"
    }

    fn interval(&self) -> Interval {
        Interval::Hourly
    }

    fn warmup_bars(&self) -> usize {
        10
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        if ctx.open_position_count() > 0 {
            if let Some(prev) = self.prev {
                let spot = ctx.spot(SPY);
                for pos in ctx.positions() {
                    let up = pos.tag == "gap_up";
                    if (up && spot <= prev) || (!up && spot >= prev) {
                        ctx.close(&pos, "gap_fill");
                    }
                }
            }
            return Ok(());
        }
        if ctx.pending_entries() > 0 {
            return Ok(());
        }
        let tb = ctx.today_bars(SPY);
        if tb.len() != 1 {
            return Ok(());
        }
        let d = ctx.daily(SPY);
        if d.is_empty() {
            return Ok(());
        }
        let prev = d.close[d.len() - 1];
        let o = tb.open[0];
        let spot = ctx.spot(SPY);
        let gap = o - prev;
        let vix = ctx.vol(SPY).atm;
        let daily_move = vix / 252f64.sqrt() * prev;
        let z = gap.abs() / daily_move;
        if !(MIN_Z <= z && z <= MAX_Z) {
            return Ok(());
        }
        let remain = (spot - prev) * if gap > 0.0 { 1.0 } else { -1.0 }; // the unfilled distance in the gap direction
        if remain < MIN_REMAIN * gap.abs() {
            return Ok(());
        }
        let exp = ctx.expiry(0);
        let k = round_strike(spot, 1.0);
        let st = if gap > 0.0 { vertical(SPY, exp, false, k, k - WIDTH)? } else { vertical(SPY, exp, true, k, k + WIDTH)? };
        self.prev = Some(prev);
        let exits = ExitRules { take_profit: Some(TAKE_PROFIT), stop_loss: Some(STOP_LOSS), minutes_before_close: Some(MINUTES_BEFORE_CLOSE), ..Default::default() };
        ctx.open(st, exits, if gap > 0.0 { "gap_up" } else { "gap_down" }, None);
        Ok(())
    }
}
