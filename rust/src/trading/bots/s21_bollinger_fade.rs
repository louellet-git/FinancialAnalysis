//! s21 - Bollinger Band fade: bet against closes outside the 20-day, 2-sigma bands (SPY, daily bars).
//!
//! A close above the upper band is stretched: sell a bear call spread above the market (0.25 delta
//! short leg, 10 sessions out). A close below the lower band: sell a bull put spread. The signal is
//! the FIRST close outside the band. One position for each side at a time. Take profit at 50% of
//! the credit, else hold to expiry.
//!
//! The width is the widest of $5, 4, 3, 2, 1 whose worst-case loss fits `ctx.risk_budget()` (1 unit).
//! At the default $60 for each trade, most credit spreads do not fit: run with `--max-risk 250`.
//! (The Python bot also had two put/call debit modes. Both were off, and they are not in the Rust code.)

use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::{vertical, Structure, MULTIPLIER};

const BB_LEN: usize = 20;
const BB_K: f64 = 2.0;
const DTE: i32 = 10;
const SHORT_DELTA: f64 = 0.25;
const WIDTHS: [f64; 5] = [5.0, 4.0, 3.0, 2.0, 1.0];
const MIN_CREDIT_FRAC: f64 = 0.15;
const TP_CREDIT: f64 = 0.5;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct BollingerFade;

struct Bands {
    c: f64,
    pc: f64,
    up: f64,
    lo: f64,
    pup: f64,
    plo: f64,
}

fn bands(ctx: &Ctx) -> Bands {
    let c = ctx.bars(SPY).close;
    let w = &c[c.len() - BB_LEN - 1..];
    let (cur, prev) = (&w[1..], &w[..w.len() - 1]);
    let (mid, sd) = (stats::mean(cur), stats::std(cur, 1));
    let (pmid, psd) = (stats::mean(prev), stats::std(prev, 1));
    Bands { c: c[c.len() - 1], pc: c[c.len() - 2], up: mid + BB_K * sd, lo: mid - BB_K * sd, pup: pmid + BB_K * psd, plo: pmid - BB_K * psd }
}

/// The widest spread whose worst case (with the round-trip commission) fits the budget.
fn fit_width(ctx: &Ctx, build: &dyn Fn(f64) -> Result<Structure, String>) -> Result<Option<(Structure, f64, f64)>, String> {
    let budget = ctx.risk_budget() * 0.95; // room for the next-open fill to move
    let fees = 4.0 * ctx.commission_per_contract();
    for w in WIDTHS {
        let s = build(w)?;
        let px = ctx.price(&s).entry;
        if s.max_loss(px) * MULTIPLIER + fees <= budget {
            return Ok(Some((s, px, w)));
        }
    }
    Ok(None)
}

/// A credit spread: "upper" is a bear call spread, "lower" is a bull put spread.
fn open_side(ctx: &mut Ctx, side: &str, call: bool) -> Result<(), String> {
    let exp = ctx.expiry(DTE + 1);
    let k = ctx.strike_for_delta(SPY, call, SHORT_DELTA, exp, 1.0);
    let step = if call { 1.0 } else { -1.0 };
    let Some((s, px, w)) = fit_width(ctx, &|w| vertical(SPY, exp, call, k + step * w, k))? else { return Ok(()) };
    if -px < MIN_CREDIT_FRAC * w {
        return Ok(());
    }
    let exits = ExitRules { take_profit: Some(TP_CREDIT), ..Default::default() };
    let kind = if call { "bear_call" } else { "bull_put" };
    ctx.open(s, exits, &format!("{side}:{kind}"), Some(1));
    Ok(())
}

impl Strategy for BollingerFade {
    fn name(&self) -> &'static str {
        "s21_bollinger_fade"
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn warmup_bars(&self) -> usize {
        25
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        let b = bands(ctx);
        let sides: Vec<String> = ctx.positions().iter().map(|p| p.tag.split(':').next().unwrap_or("").to_string()).collect();
        if b.c > b.up && !(b.pc > b.pup) && !sides.iter().any(|s| s == "upper") {
            open_side(ctx, "upper", true)?;
        }
        if b.c < b.lo && !(b.pc < b.plo) && !sides.iter().any(|s| s == "lower") {
            open_side(ctx, "lower", false)?;
        }
        Ok(())
    }
}
