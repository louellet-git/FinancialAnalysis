//! s10 - Volatility-regime timing for QQQ put credit spreads (daily bars).
//!
//! The regime comes from the 30-day vol index (VXN for QQQ) only, with no lookahead:
//!   calm: VXN within a band of its 50-day mean (ratio <= 1.05), its 252-day percentile in
//!         [0.20, 0.75], VXN <= 24 and QQQ above 98% of its 50-day mean. Sell a put credit
//!         spread (the lowest-delta short put at or above 0.30 delta whose $1 spread fits the risk cap).
//!   breakout (VXN jumped) or panic (VXN > 35): stand aside. (The put debit spread of a breakout
//!         was off in the Python bot, and it is not in the Rust code.)

use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::{vertical, Structure};

const DTE: i32 = 21;
const WIDTH: f64 = 1.0;
const SHORT_DELTA: f64 = 0.30;
const MA_LEN: usize = 50;
const PCT_LEN: usize = 252;
const PCT_LO: f64 = 0.20;
const PCT_HI: f64 = 0.75;
const CALM_RATIO_MAX: f64 = 1.05;
const VIX_MAX: f64 = 24.0;
const PANIC_VIX: f64 = 35.0;
const BREAKOUT_RATIO: f64 = 1.25;
const BREAKOUT_5D: f64 = 0.30;
const CR_TP: f64 = 0.50;
const CR_SL: f64 = 9.0; // effectively off: the spread caps the loss
const EXIT_DTE: i32 = 3;
const QQQ: &str = "QQQ";

#[derive(Default)]
pub struct VixRegime;

/// The lowest-delta short put (at or above SHORT_DELTA) whose $-wide credit spread fits the risk cap.
fn fit_credit(ctx: &Ctx, exp: i32) -> Result<Option<Structure>, String> {
    let budget = ctx.equity() * 0.03;
    let k0 = ctx.strike_for_delta(QQQ, false, SHORT_DELTA, exp, 1.0);
    let spot = ctx.spot(QQQ);
    let mut k = k0;
    while k <= spot * 1.02 {
        let st = vertical(QQQ, exp, false, k - WIDTH, k)?;
        let loss = (ctx.price(&st).entry + WIDTH) * 100.0 + 2.6;
        if loss <= budget * 0.97 {
            return Ok(Some(st));
        }
        k += 1.0;
    }
    Ok(None)
}

impl Strategy for VixRegime {
    fn name(&self) -> &'static str {
        "s10_vix_regime"
    }

    fn symbols(&self) -> &'static [&'static str] {
        &[QQQ]
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn warmup_bars(&self) -> usize {
        260
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        if ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        let hist = ctx.atm_close_history(QQQ);
        let v = super::tail(hist, PCT_LEN + 1);
        if v.len() < PCT_LEN {
            return Ok(());
        }
        let vix = v[v.len() - 1];
        let ma = stats::mean(&v[v.len() - MA_LEN..]);
        let below = v[..v.len() - 1].iter().filter(|&&x| x < vix).count();
        let pct = below as f64 / (v.len() - 1) as f64;
        let ratio = vix / ma;
        let px = ctx.bars(QQQ).close;
        let below_ma20 = px[px.len() - 1] < stats::mean(&px[px.len() - 20..]);
        let chg5 = vix / v[v.len() - 6] - 1.0;

        if vix > PANIC_VIX {
            return Ok(());
        }
        let exp = ctx.expiry(DTE + 1);
        let breakout = ratio >= BREAKOUT_RATIO || (chg5 >= BREAKOUT_5D && below_ma20);
        if breakout {
            return Ok(()); // the put debit spread is off
        }
        let calm = ratio <= CALM_RATIO_MAX && PCT_LO <= pct && pct <= PCT_HI && vix <= VIX_MAX && px[px.len() - 1] > stats::mean(&px[px.len() - 50..]) * 0.98;
        if calm {
            let Some(st) = fit_credit(ctx, exp)? else { return Ok(()) };
            let exits = ExitRules { take_profit: Some(CR_TP), stop_loss: Some(CR_SL), exit_dte: Some(EXIT_DTE), ..Default::default() };
            ctx.open(st, exits, "calm", None);
        }
        Ok(())
    }
}
