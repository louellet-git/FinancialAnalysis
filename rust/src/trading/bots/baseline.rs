//! TEMPLATE / BASELINE. Every Friday close, buy a ~0.40-delta SPY call debit spread ($1 wide)
//! that expires in 5 sessions after the fill; take profit at +80% of the debit, stop at -50%,
//! exit 1 session before expiry. It exists to exercise the harness, not to make money.
//! Copy this file to start a new bot.

use crate::trading::calendar::weekday;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const DTE: i32 = 5;
const DELTA: f64 = 0.40;
const WIDTH: f64 = 1.0;

pub struct WeeklyCallSpreadBaseline;

impl Strategy for WeeklyCallSpreadBaseline {
    fn name(&self) -> &'static str {
        "baseline_weekly_call_spread"
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        // Decide at the Friday close; the order fills at the Monday open.
        if weekday(ctx.today()) != 4 || ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        let exp = ctx.expiry(DTE + 1); // +1: the fill happens next session
        let k = ctx.strike_for_delta("SPY", true, DELTA, exp, 1.0);
        let exits = ExitRules { take_profit: Some(0.8), stop_loss: Some(0.5), exit_dte: Some(1), ..Default::default() };
        ctx.open(vertical("SPY", exp, true, k, k + WIDTH)?, exits, "weekly", None);
        Ok(())
    }
}
