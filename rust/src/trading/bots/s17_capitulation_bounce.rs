//! s17 - Capitulation bounce ("buy fear") on SPY (daily bars, 10 DTE, defined risk).
//!
//! Signal at the daily close, filled at the next open:
//!   1. Drawdown: the close is at least 3% below its 10-day closing high.
//!   2. Fear spike: VIX / its 20-day mean reached 1.15 or more in the last 3 sessions.
//!   3. Crash filter (the most important rule): the 200-day SMA must have risen over the last 20
//!      sessions ("buy the dip in an uptrend"; it screens out 2008, early 2020 and 2022).
//!
//! Structure: a bullish $1 call debit spread (long leg near 0.40 delta), 10 sessions to expiry,
//! held to expiry. One position at a time, with 5 sessions of cool-down between entries.
//! (The Python bot also had a put credit mode, a VIX-turn rule and a capitulation-day rule. All
//! were off, and they are not in the Rust code.)

use super::tail;
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::{round_strike, vertical};

const DROP_LOOKBACK: usize = 10;
const DROP_PCT: f64 = 0.03;
const VIX_MA: usize = 20;
const VIX_SPIKE: f64 = 1.15;
const SPIKE_WINDOW: usize = 3;
const TREND_SLOPE_MIN: f64 = 0.0;
const COOLDOWN: usize = 5;
const DTE: i32 = 10;
const LONG_DELTA: f64 = 0.40;
const WIDTH: f64 = 1.0;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct CapitulationBounce {
    last_entry_i: Option<usize>,
}

fn signal(ctx: &Ctx) -> bool {
    let close = ctx.bars(SPY).close;
    let vix = ctx.atm_close_history(SPY);
    let n = DROP_LOOKBACK.max(VIX_MA + SPIKE_WINDOW).max(200 + 20) + 1;
    if close.len() < n {
        return false;
    }
    let c = close[close.len() - 1];
    let hi = tail(close, DROP_LOOKBACK + 1).iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if c > hi * (1.0 - DROP_PCT) {
        return false;
    }
    // The ratio of the vol index to its 20-day mean, for the last SPIKE_WINDOW bars.
    let m = vix.len();
    let ratio_max = (m - SPIKE_WINDOW..m).map(|k| vix[k] / stats::mean(&vix[k + 1 - VIX_MA..=k])).fold(f64::NEG_INFINITY, f64::max);
    if ratio_max < VIX_SPIKE {
        return false;
    }
    // TREND_FILTER: the 200-day SMA now against 20 sessions ago.
    let k = close.len() - 1;
    let sma_now = stats::mean(&close[k - 199..=k]);
    let sma_then = stats::mean(&close[k - 20 - 199..=k - 20]);
    if sma_now / sma_then - 1.0 < TREND_SLOPE_MIN {
        return false;
    }
    true
}

impl Strategy for CapitulationBounce {
    fn name(&self) -> &'static str {
        "s17_capitulation_bounce"
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn warmup_bars(&self) -> usize {
        210
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        if ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        if let Some(last) = self.last_entry_i {
            if ctx.i - last < COOLDOWN {
                return Ok(());
            }
        }
        if !signal(ctx) {
            return Ok(());
        }
        let exp = ctx.expiry(DTE + 1); // +1: the fill happens next session
        let k = ctx.strike_for_delta(SPY, true, LONG_DELTA, exp, 1.0);
        let s = vertical(SPY, exp, true, k, round_strike(k + WIDTH, 1.0))?;
        ctx.open(s, ExitRules { exit_dte: Some(0), ..Default::default() }, "call_debit", None);
        self.last_entry_i = Some(ctx.i);
        Ok(())
    }
}
