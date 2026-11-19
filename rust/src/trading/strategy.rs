//! The interface that each bot implements.
//!
//! A bot is a struct that implements `Strategy`. On each bar close it can call `ctx.open(..)`
//! (filled at the NEXT bar open, sized by the risk rules of the engine) and `ctx.close(..)`
//! (filled at this bar close). The engine checks the exits of `ExitRules` on each bar close
//! before `on_bar` runs. A position that is open at expiry settles at intrinsic value.
//! `Ctx` shows data up to the current bar close only: no method gives the future.

use super::data::Interval;
use super::engine::Ctx;

/// All thresholds are fractions of the entry PREMIUM (|net debit| or |net credit|).
/// A debit trade with `take_profit` 1.0 exits when it is up 100% of the debit; a credit trade
/// with `stop_loss` 2.0 exits when the loss is 2x the credit. Exits use the liquidation value
/// (long legs at the bid, short legs at the ask).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ExitRules {
    pub take_profit: Option<f64>,
    pub stop_loss: Option<f64>,
    /// On the expiry day, exit this many minutes before 16:00.
    pub minutes_before_close: Option<i32>,
    /// Exit when this many sessions or fewer remain to expiry.
    pub exit_dte: Option<i32>,
    pub max_hold_bars: Option<i32>,
}

pub trait Strategy: Send {
    fn name(&self) -> &'static str;
    fn symbols(&self) -> &'static [&'static str] {
        &["SPY"]
    }
    fn interval(&self) -> Interval {
        Interval::Daily
    }
    /// Bars before this index never trade (indicator warm-up).
    fn warmup_bars(&self) -> usize {
        0
    }
    /// Default start date of the backtest (ISO), `None` for all data.
    fn start(&self) -> Option<&'static str> {
        None
    }
    /// An `Err` stops the run of this bot (the Python code raised an exception here).
    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String>;
}
