//! s11 - Volatility-risk-premium harvest: a ~41-calendar-day iron condor on SPY (daily bars).
//!
//! Sell a $1-wing iron condor 28 sessions out (+1 for the fill) on Mondays, take profit at 50%
//! of the credit, exit at 15 sessions to expiry or at a loss of 1x the credit.
//!
//! The risk cap of $60 matters: with $1 wings the textbook 16-delta condor pays only
//! $0.22 to $0.24 (maximum loss about $82 with fees) and the engine would reject it. The short
//! strikes must be about 30 delta or more, so `SHORT_DELTA` is 0.32.

use crate::trading::calendar::weekday;
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::iron_condor;

const DTE_SESSIONS: i32 = 28;
const SHORT_DELTA: f64 = 0.32;
const WING: f64 = 1.0;
const TAKE_PROFIT: f64 = 0.50;
const STOP_LOSS: f64 = 1.0;
const EXIT_DTE: i32 = 15;
const MAX_CONCURRENT: usize = 1;
const MIN_VIX: f64 = 0.0;
const ENTRY_WEEKDAY: u32 = 0;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct VrpCondor;

impl Strategy for VrpCondor {
    fn name(&self) -> &'static str {
        "s11_vrp_condor"
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        if ctx.open_position_count() + ctx.pending_entries() >= MAX_CONCURRENT {
            return Ok(());
        }
        if weekday(ctx.today()) != ENTRY_WEEKDAY {
            return Ok(());
        }
        if ctx.vol(SPY).atm < MIN_VIX {
            return Ok(());
        }
        let exp = ctx.expiry(DTE_SESSIONS + 1);
        // A faster bisection than `strike_for_delta`: 14 steps between 0.7 and 1.3 times spot.
        let ps = ctx.bisect_strike(SPY, false, SHORT_DELTA, exp, (0.7, 1.3), 14, 1.0);
        let cs = ctx.bisect_strike(SPY, true, SHORT_DELTA, exp, (0.7, 1.3), 14, 1.0);
        if cs <= ps {
            return Ok(());
        }
        let st = iron_condor(SPY, exp, ps - WING, ps, cs, cs + WING)?;
        let credit = -ctx.price(&st).entry;
        if credit <= 0.0 {
            return Ok(());
        }
        // Skip entries that the engine would reject as too_risky_for_account.
        if (WING - credit) * 100.0 + 4.0 * 0.65 * 2.0 > 0.03 * ctx.equity() {
            return Ok(());
        }
        let exits = ExitRules { take_profit: Some(TAKE_PROFIT), stop_loss: Some(STOP_LOSS), exit_dte: Some(EXIT_DTE), ..Default::default() };
        ctx.open(st, exits, "vrp", None);
        Ok(())
    }
}
