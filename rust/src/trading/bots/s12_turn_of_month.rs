//! s12 - Turn-of-the-month seasonality with a short-dated call debit spread (SPY, daily bars).
//!
//! Hold a bullish $1 call debit spread (0.50-delta long leg) from the open of the 3rd-to-last
//! session of a month through the close of the 3rd session of the next month. Stop at -60% of
//! the debit. The expiry is 9 sessions after the fill.

use crate::trading::calendar::{busday_count, civil_from_days, days_from_civil};
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const PRE: i32 = 3;
const POST: usize = 3;
const DELTA: f64 = 0.50;
const WIDTH: f64 = 1.0;
const DTE: i32 = 9;
const STOP: f64 = 0.6;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct TurnOfMonth {
    traded: Option<(i32, u32)>,
}

impl Strategy for TurnOfMonth {
    fn name(&self) -> &'static str {
        "s12_turn_of_month"
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        let today = ctx.today();
        let (year, month, _) = civil_from_days(today);
        if let Some(pos) = ctx.positions().first() {
            // The number of sessions of this month so far.
            let n_new = (0..=ctx.i)
                .rev()
                .take_while(|&j| {
                    let (y, m, _) = civil_from_days(ctx.date_at(j));
                    y == year && m == month
                })
                .count();
            let (ey, em, _) = civil_from_days(pos.entry_time.date);
            let held_into_new = em != month || ey != year;
            if held_into_new && n_new >= POST {
                ctx.close(pos, "tom_window_end");
            }
            return Ok(());
        }
        if ctx.pending_entries() > 0 {
            return Ok(());
        }
        // The business days from today to the first business day of next month.
        let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        let k = busday_count(today, days_from_civil(ny, nm, 1));
        if k > PRE + 1 || self.traded == Some((year, month)) {
            return Ok(());
        }
        self.traded = Some((year, month));
        let exp = ctx.expiry(DTE + 1);
        let kl = ctx.strike_for_delta(SPY, true, DELTA, exp, 1.0);
        let s = vertical(SPY, exp, true, kl, kl + WIDTH)?;
        ctx.open(s, ExitRules { stop_loss: Some(STOP), exit_dte: Some(1), ..Default::default() }, "tom", None);
        Ok(())
    }
}
