//! s04 - Gap and go: continuation debit spreads after a large opening gap that does not fill.
//!
//! The gap must be 0.6 to 3.0 times the VIX-implied daily move, and the first hour must keep
//! 60% of the gap and close in the gap direction without a touch of the prior close. Buy a
//! 1DTE debit spread in the gap direction. Exit at the target or stop, on a move back into the
//! prior-day range, or 30 minutes before the close on the expiry day.

use crate::trading::calendar::Date;
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;
use std::collections::HashMap;

const MIN_GAP_Z: f64 = 0.6;
const MAX_GAP_Z: f64 = 3.0;
const HOLD_FRAC: f64 = 0.6;
const DTE: i32 = 1;
const WIDTH: f64 = 1.0;
const LONG_DELTA: f64 = 0.45;
const TAKE_PROFIT: f64 = 0.8;
const STOP_LOSS: f64 = 0.5;
const EXIT_MINUTES: i32 = 30;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct GapAndGo {
    /// Position id to (direction, prior-day level).
    reg: HashMap<u64, (i32, f64)>,
    pending_dir: Option<(i32, f64)>,
    day: Option<Date>,
    traded: bool,
}

impl Strategy for GapAndGo {
    fn name(&self) -> &'static str {
        "s04_gap_and_go"
    }

    fn interval(&self) -> Interval {
        Interval::Hourly
    }

    fn warmup_bars(&self) -> usize {
        3
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        // Bind the gap information of the queued entry to its position id.
        for p in ctx.positions() {
            if !self.reg.contains_key(&p.id) {
                if let Some(info) = self.pending_dir.take() {
                    self.reg.insert(p.id, info);
                }
            }
        }
        if self.day != Some(ctx.today()) {
            self.day = Some(ctx.today());
            self.traded = false;
        }
        // A move back into the prior-day range closes the position.
        for p in ctx.positions() {
            let Some(&(d, lvl)) = self.reg.get(&p.id) else { continue };
            let s = ctx.spot(SPY);
            if (d > 0 && s <= lvl) || (d < 0 && s >= lvl) {
                ctx.close(&p, "back_in_range");
            }
        }
        if self.traded || ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        let tb = ctx.today_bars(SPY);
        if tb.len() != 1 {
            return Ok(()); // decide only at the close of the first hourly bar
        }
        let dly = ctx.daily(SPY);
        if dly.is_empty() {
            return Ok(());
        }
        let last = dly.len() - 1;
        let (prev_close, prev_high, prev_low) = (dly.close[last], dly.high[last], dly.low[last]);
        let (op, cl) = (tb.open[0], tb.close[0]);
        let gap = op - prev_close;
        let daily_sd = ctx.vol(SPY).atm / 252f64.sqrt();
        let z = gap.abs() / prev_close / daily_sd;
        if !(MIN_GAP_Z <= z && z <= MAX_GAP_Z) {
            return Ok(());
        }
        let d: i32 = if gap > 0.0 { 1 } else { -1 };
        let df = d as f64;
        // Hold test: the first hour is still beyond the prior close by HOLD_FRAC of the gap.
        let retained = (cl - prev_close) * df;
        if retained < HOLD_FRAC * gap.abs() {
            return Ok(());
        }
        if (cl - op) * df < 0.0 {
            return Ok(()); // the first-hour bar must close in the gap direction
        }
        // No fill: the first hour must not trade back through the prior close.
        if d > 0 && tb.low[0] <= prev_close {
            return Ok(());
        }
        if d < 0 && tb.high[0] >= prev_close {
            return Ok(());
        }
        let exp = ctx.expiry(DTE);
        let call = d > 0;
        let k = ctx.strike_for_delta(SPY, call, LONG_DELTA, exp, 1.0);
        let short = k + WIDTH * df;
        let exits = ExitRules { take_profit: Some(TAKE_PROFIT), stop_loss: Some(STOP_LOSS), minutes_before_close: Some(EXIT_MINUTES), ..Default::default() };
        ctx.open(vertical(SPY, exp, call, k, short)?, exits, &format!("gap{}_z{:.1}", if d > 0 { "up" } else { "dn" }, z), None);
        self.traded = true;
        self.pending_dir = Some((d, if d > 0 { prev_high } else { prev_low }));
        Ok(())
    }
}
