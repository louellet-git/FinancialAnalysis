//! s18 - Late-day 0DTE pin butterfly (SPY, hourly bars).
//!
//! About 90 minutes before the close, if the afternoon was quiet and the spot is close to the
//! session VWAP, buy a cheap $1-wide call butterfly centred on the VWAP (rounded to $1) and HOLD TO
//! EXPIRY (settlement is not a day trade). The cost must be at most 35% of the width.
//!
//! Real pinning comes from dealer gamma hedging and open interest, which this data cannot show.
//! Any edge here comes from the price path only. (The Python bot also had the "round" and "mid"
//! centres and an iron butterfly mode. All were off, and they are not in the Rust code.)

use super::tail;
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::{butterfly, round_strike};

use super::s02_vwap_revert_0dte::vwap;

const DECISION_MIN_TO_CLOSE: f64 = 90.0;
const WIDTH: f64 = 1.0;
const MAX_DIST_SIGMA: f64 = 0.35;
const QUIET_RATIO: f64 = 0.8;
const MAX_COST_FRAC: f64 = 0.35;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct PinButterfly;

impl Strategy for PinButterfly {
    fn name(&self) -> &'static str {
        "s18_pin_butterfly"
    }

    fn interval(&self) -> Interval {
        Interval::Hourly
    }

    fn warmup_bars(&self) -> usize {
        10
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        if ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        // One decision for each day: the bar whose close is nearest to the target time.
        let mtc = ctx.minutes_to_close();
        let step = 60.0;
        if !(DECISION_MIN_TO_CLOSE - step / 2.0 < mtc && mtc <= DECISION_MIN_TO_CLOSE + step / 2.0) {
            return Ok(());
        }
        let bars = ctx.today_bars(SPY);
        if bars.len() < 3 {
            return Ok(());
        }
        let spot = ctx.spot(SPY);
        let vol = ctx.vol(SPY);
        let sigma_rem = vol.short * (mtc / (252.0 * 390.0)).sqrt() * spot; // the 1-sigma $ move to the close
        if sigma_rem <= 0.0 {
            return Ok(());
        }
        let center = round_strike(vwap(&bars), 1.0);
        if (spot - center).abs() > MAX_DIST_SIGMA * sigma_rem {
            return Ok(());
        }
        // The realized range of the last 3 bars against the range that the 1-day IV priced.
        let hi = tail(bars.high, 3);
        let lo = tail(bars.low, 3);
        if hi.len() < 2 {
            return Ok(());
        }
        let rng = hi.iter().copied().fold(f64::NEG_INFINITY, f64::max) - lo.iter().copied().fold(f64::INFINITY, f64::min);
        let span_min = (hi.len() * 60) as f64;
        let implied = vol.short * (span_min / (252.0 * 390.0)).sqrt() * spot;
        if rng > QUIET_RATIO * 2.0 * implied {
            return Ok(());
        }
        let exp = ctx.expiry(0);
        let w = WIDTH;
        let s = butterfly(SPY, exp, true, center - w, center, center + w)?;
        let cost = ctx.price(&s).entry;
        if cost <= 0.0 || cost > MAX_COST_FRAC * w {
            return Ok(());
        }
        ctx.open(s, ExitRules::default(), "vwap", None);
        Ok(())
    }
}
