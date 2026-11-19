//! s07 - Overnight drift: buy a short-dated call debit spread late in the session, sell it the
//! next morning.
//!
//! The signal is on the 14:30 bar (known at 15:30). The engine fills at the open of the 15:30
//! bar (the latest entry that hourly bars allow). The exit is `max_hold_bars` = 2: the 15:30 bar
//! is bar 1, the 09:30 bar of the next day is bar 2, so the exit is at the CLOSE of that bar
//! (10:30). (The engine cannot sell at the open: a limit of the harness.)

use crate::trading::calendar::weekday;
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const DTE: i32 = 1;
const DELTA: f64 = 0.50;
const WIDTH: f64 = 1.0;
const SMA_DAYS: usize = 50;
const VIX_MAX: f64 = 30.0;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct OvernightDrift;

impl Strategy for OvernightDrift {
    fn name(&self) -> &'static str {
        "s07_overnight_drift"
    }

    fn interval(&self) -> Interval {
        Interval::Hourly
    }

    fn warmup_bars(&self) -> usize {
        10
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        if ctx.bar_minute() != 14 * 60 + 30 {
            return Ok(());
        }
        if ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        if weekday(ctx.today()) == 4 {
            return Ok(()); // skip Fridays
        }
        let d = ctx.daily(SPY);
        if d.len() < SMA_DAYS || ctx.spot(SPY) < stats::mean(&d.close[d.len() - SMA_DAYS..]) {
            return Ok(());
        }
        if ctx.vol(SPY).atm * 100.0 > VIX_MAX {
            return Ok(());
        }
        let exp = ctx.expiry(DTE);
        let k = ctx.strike_for_delta(SPY, true, DELTA, exp, 1.0);
        ctx.open(vertical(SPY, exp, true, k, k + WIDTH)?, ExitRules { max_hold_bars: Some(2), ..Default::default() }, "overnight", None);
        Ok(())
    }
}
