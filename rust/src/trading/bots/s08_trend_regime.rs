//! s08 - Trend regime filter that drives rolling vertical spreads (SPY, daily bars).
//!
//! The regime uses completed daily closes only (no lookahead):
//!   UP:   close > SMA(200) and SMA(50) > SMA(200)
//!   DOWN: close < SMA(200) and SMA(50) < SMA(200)
//!   else NEUTRAL (no new entries)
//! In an UP regime, buy a $1 call debit spread (long leg ~0.50 delta, 30 sessions to expiry).
//! Downtrends stay flat (`TRADE_DOWN` is off). Exits: roll with `exit_dte` = 10, take profit at
//! +80% of the debit, and close a spread when the regime turns DOWN.
//! (The Python bot also had an ADX filter, a put credit mode and staggered entries. All were off,
//! and they are not in the Rust code.)

use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const FAST: usize = 50;
const LONG: usize = 200;
const DTE: i32 = 30;
const ROLL_DTE: i32 = 10;
const CALL_DELTA: f64 = 0.50;
const WIDTH: f64 = 1.0;
const TP: f64 = 0.80;
const MAX_CONCURRENT: usize = 1;
const SPY: &str = "SPY";

#[derive(PartialEq)]
enum Regime {
    Up,
    Down,
    Neutral,
}

#[derive(Default)]
pub struct TrendRegime;

fn regime(ctx: &Ctx) -> Regime {
    let c = ctx.bars(SPY).close;
    if c.len() < LONG {
        return Regime::Neutral;
    }
    let last = c[c.len() - 1];
    let sma_l = stats::mean(&c[c.len() - LONG..]);
    let sma_f = stats::mean(&c[c.len() - FAST..]);
    if last > sma_l && sma_f > sma_l {
        Regime::Up
    } else if last < sma_l && sma_f < sma_l {
        Regime::Down
    } else {
        Regime::Neutral
    }
}

impl Strategy for TrendRegime {
    fn name(&self) -> &'static str {
        "s08_trend_regime"
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn warmup_bars(&self) -> usize {
        200
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        let reg = regime(ctx);
        let hold_dir = match reg {
            Regime::Up => Some("C"),
            Regime::Down => Some("P"),
            Regime::Neutral => None,
        };
        // A regime flip closes the spreads that disagree with the regime.
        for p in ctx.positions() {
            let Some(dir) = hold_dir else { continue };
            if p.tag != dir {
                ctx.close(&p, "regime_flip");
            }
        }
        if reg != Regime::Up {
            return Ok(()); // DOWN is flat (`TRADE_DOWN` off) and NEUTRAL gives no entry
        }
        if ctx.pending_entries() > 0 || ctx.open_position_count() >= MAX_CONCURRENT {
            return Ok(());
        }
        let exp = ctx.expiry(DTE + 1); // the fill is next session
        let k = ctx.strike_for_delta(SPY, true, CALL_DELTA, exp, 1.0);
        let exits = ExitRules { take_profit: Some(TP), exit_dte: Some(ROLL_DTE), ..Default::default() };
        ctx.open(vertical(SPY, exp, true, k, k + WIDTH)?, exits, "C", None);
        Ok(())
    }
}
