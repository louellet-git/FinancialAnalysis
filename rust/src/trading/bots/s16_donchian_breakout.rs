//! s16 - Turtle-style Donchian channel breakout with debit spreads (daily bars).
//!
//! A LONG trend starts when the close is above the highest high of the prior 20 sessions. A
//! SHORT trend starts when the close is below the lowest low of the prior 20 sessions. N is the
//! Wilder ATR(20). The trend ends when the close crosses the opposite 10-session channel, or moves
//! 2N against the breakout close, or an opposite breakout fires.
//!
//! While a trend is live, hold ONE debit vertical for each symbol in the trend direction (call
//! spread for an uptrend, put spread for a downtrend), about 20 sessions to expiry. The strike
//! and the width are the first (offset, width) from ATM outward whose maximum loss fits the
//! budget (width 2 before width 1). Roll (close, then open again) when 5 sessions or fewer remain
//! or when the spread has captured 85% of its width. Close all when the trend ends.

use super::tail;
use crate::trading::calendar::busday_count;
use crate::trading::data::Interval;
use crate::trading::engine::{Ctx, Position};
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::{vertical, Structure, MULTIPLIER};

const ENTRY_N: usize = 20;
const EXIT_N: usize = 10;
const ATR_N: usize = 20;
const STOP_ATR: f64 = 2.0;
const DTE: i32 = 20;
const ROLL_DTE: i32 = 5;
const ROLL_CAPTURE: f64 = 0.85;
const OTM_STEPS: [f64; 6] = [0.0, 0.003, 0.006, 0.010, 0.015, 0.02];
const WIDTHS: [f64; 2] = [2.0, 1.0];
const BUDGET_FRAC: f64 = 0.95;

fn max_of(a: &[f64]) -> f64 {
    a.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

fn min_of(a: &[f64]) -> f64 {
    a.iter().copied().fold(f64::INFINITY, f64::min)
}

/// Wilder ATR of the last bar.
fn atr(h: &[f64], l: &[f64], c: &[f64], n: usize) -> f64 {
    let tr: Vec<f64> = (0..c.len())
        .map(|k| {
            let prev = if k == 0 { c[0] } else { c[k - 1] };
            (h[k] - l[k]).max((h[k] - prev).abs().max((l[k] - prev).abs()))
        })
        .collect();
    let mut a = crate::trading::stats::mean(&tr[..n.min(tr.len())]);
    for &x in tr.iter().skip(n) {
        a += (x - a) / n as f64;
    }
    a
}

#[derive(Clone, Copy, Default)]
struct Trend {
    dir: i32,
    stop: f64,
}

#[derive(Default)]
pub struct DonchianBreakoutSpreads {
    trend: [Trend; 2],
}

struct Signal {
    close: f64,
    hi_entry: f64,
    lo_entry: f64,
    hi_exit: f64,
    lo_exit: f64,
    atr: f64,
}

fn signal(ctx: &Ctx, sym: &str) -> Signal {
    let keep = (ENTRY_N.max(EXIT_N) + 3 * ATR_N + 2).max(1);
    let b = ctx.bars(sym);
    let (h, l, c) = (tail(b.high, keep), tail(b.low, keep), tail(b.close, keep));
    let n = c.len();
    let m = 3 * ATR_N + 2;
    Signal {
        close: c[n - 1],
        hi_entry: max_of(&h[n - ENTRY_N - 1..n - 1]),
        lo_entry: min_of(&l[n - ENTRY_N - 1..n - 1]),
        hi_exit: max_of(&h[n - EXIT_N - 1..n - 1]),
        lo_exit: min_of(&l[n - EXIT_N - 1..n - 1]),
        atr: atr(tail(h, m), tail(l, m), tail(c, m), ATR_N),
    }
}

/// The first (offset, width) from ATM outward whose maximum loss fits the budget.
fn spread(ctx: &Ctx, sym: &'static str, direction: i32) -> Result<Option<Structure>, String> {
    let exp = ctx.expiry(DTE + 1); // +1: the fill happens next session
    let call = direction > 0;
    let spot = ctx.spot(sym);
    let budget = ctx.equity() * 0.03 * BUDGET_FRAC;
    for off in OTM_STEPS {
        let k = (if call { spot * (1.0 + off) } else { spot * (1.0 - off) }).round_ties_even();
        for w in WIDTHS {
            let s = vertical(sym, exp, call, k, if call { k + w } else { k - w })?;
            let loss = ctx.price(&s).entry * MULTIPLIER + 4.0 * 0.65;
            if loss <= budget {
                return Ok(Some(s));
            }
        }
    }
    Ok(None) // nothing is affordable: skip
}

fn width(p: &Position) -> f64 {
    (p.structure.legs[0].strike - p.structure.legs[1].strike).abs()
}

impl DonchianBreakoutSpreads {
    fn run_symbol(&mut self, ctx: &mut Ctx, idx: usize, sym: &'static str) -> Result<(), String> {
        let sig = signal(ctx, sym);
        let c = sig.close;
        let st = &mut self.trend[idx];

        // 1) Update the trend: exits first, then (maybe reversing) entries.
        if (st.dir > 0 && (c < sig.lo_exit || c < st.stop)) || (st.dir < 0 && (c > sig.hi_exit || c > st.stop)) {
            st.dir = 0;
        }
        if c > sig.hi_entry && st.dir <= 0 {
            st.dir = 1;
            st.stop = c - STOP_ATR * sig.atr;
        } else if c < sig.lo_entry && st.dir >= 0 {
            st.dir = -1;
            st.stop = c + STOP_ATR * sig.atr;
        }
        let dir = st.dir;

        // 2) Manage the open spread of this symbol.
        for p in ctx.positions().iter().filter(|p| p.structure.symbol == sym) {
            let pdir = if p.structure.legs[0].call { 1 } else { -1 };
            if pdir != dir {
                ctx.close(p, "trend_end");
                continue;
            }
            let dte = busday_count(ctx.today(), p.structure.expiry);
            let liq = ctx.price(&p.structure).liquidation;
            if dte <= ROLL_DTE {
                ctx.close(p, "roll_dte");
            } else if liq >= ROLL_CAPTURE * width(p) {
                ctx.close(p, "roll_capture");
            }
        }

        // 3) Enter again while the trend is live and nothing is held for this symbol.
        if dir == 0 {
            return Ok(());
        }
        if ctx.positions().iter().any(|p| p.structure.symbol == sym) {
            return Ok(()); // still held (for example the close was blocked by the PDT rule)
        }
        if let Some(s) = spread(ctx, sym, dir)? {
            ctx.open(s, ExitRules { exit_dte: Some(1), ..Default::default() }, &format!("{sym}{}", if dir > 0 { "+" } else { "-" }), None);
        }
        Ok(())
    }
}

impl Strategy for DonchianBreakoutSpreads {
    fn name(&self) -> &'static str {
        "s16_donchian_breakout"
    }

    fn symbols(&self) -> &'static [&'static str] {
        &["SPY", "QQQ"]
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn warmup_bars(&self) -> usize {
        60
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        for (idx, sym) in self.symbols().iter().copied().enumerate() {
            self.run_symbol(ctx, idx, sym)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilder_atr_hand_value() {
        // True ranges 2, 2, 3. The first 2 give 2.0; then ATR += (3 - 2.0) / 2.
        let a = atr(&[11.0, 12.0, 14.0], &[9.0, 10.0, 11.0], &[10.0, 11.0, 13.0], 2);
        assert!((a - 2.5).abs() < 1e-12);
    }
}
