//! s01 - Opening-range breakout (ORB) with 0DTE debit spreads (5-minute bars).
//!
//! The opening range is the first 6 bars of the session (30 minutes). On the first bar close
//! beyond the range by BUFFER (times the range width), with at least 120 minutes to the close,
//! buy a narrow 0DTE debit spread in the breakout direction. Exits: profit target and stop
//! (`ExitRules`), a close back inside the range, and a time exit before the close. One trade for
//! each day, and at most 3 in 7 days.

use crate::trading::calendar::Date;
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::{round_strike, vertical};

const OR_BARS: usize = 6;
const BUFFER: f64 = 0.25;
const MIN_RANGE_PCT: f64 = 0.0;
const MAX_RANGE_PCT: f64 = 0.02;
const LONG_OFFSET: f64 = 0.0;
const WIDTH: f64 = 1.0;
const TAKE_PROFIT: f64 = 1.0;
const STOP_LOSS: f64 = 0.5;
const EXIT_MINUTES_BEFORE_CLOSE: i32 = 20;
const ENTRY_CUTOFF_MIN: f64 = 120.0;
const MAX_DAY_TRADES_5D: usize = 3;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct OrbZeroDte {
    entries: Vec<Date>,
    range: Option<(f64, f64)>,
    day: Option<Date>,
    traded: Option<Date>,
}

impl Strategy for OrbZeroDte {
    fn name(&self) -> &'static str {
        "s01_orb_0dte"
    }

    fn interval(&self) -> Interval {
        Interval::FiveMinute
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        let tb = ctx.today_bars(SPY);
        let n = tb.len();
        let today = ctx.today();

        // Range re-entry stop on open positions (same day only; 0DTE).
        for pos in ctx.positions() {
            if let Some((lo, hi)) = self.range {
                let c = ctx.spot(SPY);
                if (pos.tag == "orb_C" && c < hi) || (pos.tag == "orb_P" && c > lo) {
                    ctx.close(&pos, "range_reentry");
                }
            }
        }
        if n < OR_BARS {
            return Ok(());
        }
        if n == OR_BARS {
            let lo = tb.low.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = tb.high.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            self.range = Some((lo, hi));
            self.day = Some(today);
            return Ok(());
        }
        if self.day != Some(today) || ctx.open_position_count() > 0 || ctx.pending_entries() > 0 || self.traded == Some(today) {
            return Ok(());
        }
        if ctx.minutes_to_close() < ENTRY_CUTOFF_MIN {
            return Ok(());
        }
        let (lo, hi) = self.range.expect("range is set with the day");
        let spot = ctx.spot(SPY);
        let width = hi - lo;
        if !(MIN_RANGE_PCT * spot <= width && width <= MAX_RANGE_PCT * spot) {
            return Ok(());
        }
        let recent: Vec<Date> = self.entries.iter().copied().filter(|&d| today - d < 7).collect();
        if recent.len() >= MAX_DAY_TRADES_5D {
            return Ok(());
        }
        let buf = BUFFER * width;
        let (cp, s) = if spot > hi + buf {
            let k = round_strike(spot + LONG_OFFSET, 1.0);
            ("C", vertical(SPY, ctx.expiry(0), true, k, k + WIDTH)?)
        } else if spot < lo - buf {
            let k = round_strike(spot - LONG_OFFSET, 1.0);
            ("P", vertical(SPY, ctx.expiry(0), false, k, k - WIDTH)?)
        } else {
            return Ok(());
        };
        self.traded = Some(today);
        self.entries = recent;
        self.entries.push(today);
        let exits = ExitRules { take_profit: Some(TAKE_PROFIT), stop_loss: Some(STOP_LOSS), minutes_before_close: Some(EXIT_MINUTES_BEFORE_CLOSE), ..Default::default() };
        ctx.open(s, exits, &format!("orb_{cp}"), None);
        Ok(())
    }
}
