//! s06 - Last-hour momentum (Gao, Han, Li and Zhou 2018, "Market intraday momentum").
//!
//! The return from the prior close to the end of the first bar predicts the return of the last
//! half hour, with the same sign. At 15:30 ET buy a narrow 0DTE debit spread in that direction
//! and hold it to expiry (settlement at expiry is not a day trade).
//!
//! Hourly bars: the decision is made at the 14:30 bar close (15:30 ET); the order fills at the
//! open of the 15:30 bar.

use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const Z_MIN: f64 = 0.3;
const WIDTH: f64 = 1.0;
const LONG_OFFSET: f64 = 0.0;
const MAX_DAILY_Z: f64 = 3.0;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct LastHourMomentum;

impl Strategy for LastHourMomentum {
    fn name(&self) -> &'static str {
        "s06_last_hour_momentum"
    }

    fn interval(&self) -> Interval {
        Interval::Hourly
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        // Decide when the bar that closes at 15:30 ET has just printed.
        if ctx.minutes_to_close().round_ties_even() != 30.0 || ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        let d = ctx.daily(SPY);
        if d.is_empty() {
            return Ok(());
        }
        let prev = d.close[d.len() - 1];
        let tb = ctx.today_bars(SPY);
        if tb.len() < 2 {
            return Ok(());
        }
        let first = tb.close[0] / prev - 1.0;
        let day = ctx.spot(SPY) / prev - 1.0;
        let sigma_d = ctx.vol(SPY).atm / 252f64.sqrt();
        let (zf, zd) = (first / sigma_d, day / sigma_d);
        let z = zf; // SIGNAL = "first"
        if z.abs() < Z_MIN || zd.abs() > MAX_DAILY_Z {
            return Ok(());
        }
        let spot = ctx.spot(SPY);
        let exp = ctx.expiry(0);
        let call = z > 0.0;
        let k = if call { spot.ceil() + LONG_OFFSET } else { spot.floor() - LONG_OFFSET };
        let k2 = if call { k + WIDTH } else { k - WIDTH };
        ctx.open(vertical(SPY, exp, call, k, k2)?, ExitRules::default(), &format!("{}_z{:+.2}", if call { "C" } else { "P" }, z), None);
        Ok(())
    }
}
