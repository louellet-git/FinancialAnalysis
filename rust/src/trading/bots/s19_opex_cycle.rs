//! s19 - Monthly options-expiration (OPEX) cycle with weekly debit spreads (SPY, daily bars).
//!
//! The calendar needs no data: the third Friday of each month is the monthly OPEX, and March,
//! June, September and December are quad witching. Each week is the OPEX week or the week AFTER
//! OPEX, in a quad month or not. A bucket has a side: +1 bullish call debit spread, -1 bearish put
//! debit spread, 0 skip. Entry at the last session of the prior week (fill on Monday). The expiry
//! is the Friday of the target week.

use crate::trading::calendar::{civil_from_days, iso_week, weekday, Date};
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const DELTA: f64 = 0.45;
const WIDTH: f64 = 1.0;
const TAKE_PROFIT: f64 = 0.8;
const STOP_LOSS: f64 = 0.5;
const EXIT_DTE: i32 = 1;
const SPY: &str = "SPY";

fn third_friday(d: Date) -> bool {
    let (_, _, day) = civil_from_days(d);
    (15..=21).contains(&day) && weekday(d) == 4
}

#[derive(Default)]
pub struct OpexCycle;

impl Strategy for OpexCycle {
    fn name(&self) -> &'static str {
        "s19_opex_cycle"
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        if ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        let today = ctx.today();
        let nxt = ctx.expiry(1);
        if iso_week(nxt) == iso_week(today) && civil_from_days(nxt).0 == civil_from_days(today).0 {
            return Ok(()); // decide only on the last session of a week
        }
        let fri = nxt + (4 - weekday(nxt) as i32);
        let (kind, quad) = if third_friday(fri) {
            ("opex", civil_from_days(fri).1 % 3 == 0)
        } else if third_friday(fri - 7) {
            ("after", civil_from_days(fri - 7).1 % 3 == 0)
        } else {
            return Ok(());
        };
        // Side for each bucket: OPEX week bullish; the week after a quad-witching OPEX bearish.
        let side: i32 = match (kind, quad) {
            ("opex", _) => 1,
            ("after", true) => -1,
            _ => 0,
        };
        if side == 0 {
            return Ok(());
        }
        let exp = fri; // Friday (the Python code rolls a weekend back; a Friday never rolls)
        let call = side > 0;
        let k = ctx.strike_for_delta(SPY, call, DELTA, exp, 1.0);
        let other = if call { k + WIDTH } else { k - WIDTH };
        let exits = ExitRules { take_profit: Some(TAKE_PROFIT), stop_loss: Some(STOP_LOSS), exit_dte: Some(EXIT_DTE), ..Default::default() };
        let tag = format!("{kind}_{}_{}", if quad { "quad" } else { "non" }, if call { "bull" } else { "bear" });
        ctx.open(vertical(SPY, exp, call, k, other)?, exits, &tag, None);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::calendar::days_from_civil;

    #[test]
    fn third_friday_rule() {
        assert!(third_friday(days_from_civil(2026, 11, 20)));
        assert!(!third_friday(days_from_civil(2026, 11, 13)));
        assert!(!third_friday(days_from_civil(2026, 11, 27)));
        assert!(!third_friday(days_from_civil(2026, 11, 19))); // a Thursday
    }
}
