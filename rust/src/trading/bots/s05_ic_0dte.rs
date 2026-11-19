//! s05 - Regime-gated 0DTE iron butterfly on SPY (hourly bars).
//!
//! Sell a narrow ATM 0DTE iron butterfly mid-morning ONLY when the intraday volatility regime
//! is calm: the 1-day implied vol (VIX1D) is low compared with the 30-day VIX, the morning move
//! is small compared with the move that the implied vol priced, and the prior day was not a
//! trend day. Exits: profit target and stop; otherwise hold to expiry (settlement at expiry is
//! not a day trade; an early exit is one, and a < $25k account has 3 in 5 sessions).

use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::{iron_condor, round_strike};

const ENTRY_BAR: usize = 1;
const WING: f64 = 2.0;
const BODY_OFFSET: f64 = 0.0;
const MIN_CREDIT: f64 = 1.40;
const MAX_RATIO: f64 = 0.80;
const MAX_SHORT_VOL: f64 = 0.20;
const MAX_MORNING_MOVE: f64 = 0.60;
const MAX_PRIOR_RANGE: f64 = 1.5;
const TAKE_PROFIT: f64 = 0.50;
const STOP_LOSS: f64 = 1.00;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct IcZeroDte;

impl Strategy for IcZeroDte {
    fn name(&self) -> &'static str {
        "s05_ic_0dte"
    }

    fn interval(&self) -> Interval {
        Interval::Hourly
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        if ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        let tb = ctx.today_bars(SPY);
        if tb.len() != ENTRY_BAR {
            return Ok(());
        }
        let spot = ctx.spot(SPY);
        let v = ctx.vol(SPY);
        if v.atm <= 0.0 || v.short / v.atm > MAX_RATIO || v.short > MAX_SHORT_VOL {
            return Ok(());
        }
        // The realized morning move against the move that the 1-day IV priced for the elapsed time.
        let elapsed_frac = tb.len() as f64 * 60.0 / 390.0;
        let exp_move = spot * v.short * (elapsed_frac / 252.0).sqrt();
        if exp_move > 0.0 && (spot - tb.open[0]).abs() / exp_move > MAX_MORNING_MOVE {
            return Ok(());
        }
        let d = ctx.daily(SPY);
        if d.len() >= 2 {
            let k = d.len() - 1;
            let day_move = d.close[k] * v.short / 252f64.sqrt();
            if day_move > 0.0 && (d.high[k] - d.low[k]) > MAX_PRIOR_RANGE * day_move * 2.0 {
                return Ok(());
            }
        }
        let exp = ctx.expiry(0);
        let atm = round_strike(spot, 1.0);
        let (ps, cs) = (atm - BODY_OFFSET, atm + BODY_OFFSET);
        let st = iron_condor(SPY, exp, ps - WING, ps, cs, cs + WING)?;
        let credit = -ctx.price(&st).entry;
        if credit < MIN_CREDIT {
            return Ok(());
        }
        ctx.open(st, ExitRules { take_profit: Some(TAKE_PROFIT), stop_loss: Some(STOP_LOSS), ..Default::default() }, "ic0dte", None);
        Ok(())
    }
}
