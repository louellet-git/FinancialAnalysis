//! s15 - Volatility-compression breakout ("squeeze") with short-dated debit spreads (hourly bars).
//!
//! Signal, for each symbol, from `ctx.bars` only: the Bollinger Bands (20, 2 sigma) are inside
//! the Keltner Channel (1.5 ATR) for at least 6 bars in a row, and the current bar is the first
//! bar OUT of the squeeze ("release"). The direction is the sign of the momentum (close minus the
//! mean of the Donchian midpoint and the SMA), and the close must also be beyond the high or low
//! of the squeeze window.
//!
//! Entry: queued at the release bar close, filled at the next bar open: a $1 debit vertical in the
//! breakout direction, long leg near 0.40 delta, 3 sessions to expiry. One position for each symbol.
//! No entry on the last bar of the session. Exit: take profit 0.8, stop 0.5, 14 bars held, and 30
//! minutes before the close on the expiry day.
//! (The Python bot also had an NR7 mode. It was off, and it is not in the Rust code.)

use super::tail;
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const BB_LEN: usize = 20;
const BB_K: f64 = 2.0;
const KC_MULT: f64 = 1.5;
const MIN_SQUEEZE_BARS: usize = 6;
const DTE: i32 = 3;
const LONG_DELTA: f64 = 0.40;
const WIDTH: f64 = 1.0;
const TP: f64 = 0.8;
const SL: f64 = 0.5;
const MAX_HOLD_BARS: i32 = 14;
const EXIT_MIN_BEFORE_CLOSE: i32 = 30;

fn max_of(a: &[f64]) -> f64 {
    a.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

fn min_of(a: &[f64]) -> f64 {
    a.iter().copied().fold(f64::INFINITY, f64::min)
}

#[derive(Default)]
pub struct SqueezeBreakout;

impl SqueezeBreakout {
    /// +1 for a release up, -1 for a release down, 0 for no signal.
    fn squeeze_signal(ctx: &Ctx, sym: &str) -> i32 {
        let n = BB_LEN + MIN_SQUEEZE_BARS + 2;
        let b = ctx.bars(sym);
        if b.len() < n {
            return 0;
        }
        let (h, l, c) = (tail(b.high, n), tail(b.low, n), tail(b.close, n));
        let tr: Vec<f64> = (0..n)
            .map(|k| {
                let prev = if k == 0 { c[0] } else { c[k - 1] };
                (h[k] - l[k]).max((h[k] - prev).abs().max((l[k] - prev).abs()))
            })
            .collect();
        let big_l = BB_LEN;
        let in_sq: Vec<bool> = (big_l..=n)
            .map(|j| {
                let sd = stats::std(&c[j - big_l..j], 0);
                let atr = stats::mean(&tr[j - big_l..j]);
                BB_K * sd < KC_MULT * atr
            })
            .collect();
        let m = in_sq.len();
        if in_sq[m - 1] || !in_sq[m - 1 - MIN_SQUEEZE_BARS..m - 1].iter().all(|&x| x) {
            return 0;
        }
        // The squeeze window is the MIN_SQUEEZE_BARS bars before the release bar.
        let wh = max_of(&h[n - 1 - MIN_SQUEEZE_BARS..n - 1]);
        let wl = min_of(&l[n - 1 - MIN_SQUEEZE_BARS..n - 1]);
        let cw = &c[n - big_l..];
        let mom = c[n - 1] - 0.5 * (0.5 * (max_of(&h[n - big_l..]) + min_of(&l[n - big_l..])) + stats::mean(cw));
        let d = if mom > 0.0 {
            1
        } else if mom < 0.0 {
            -1
        } else {
            0
        };
        // CONFIRM_RANGE: the release close must be beyond the high or the low of the squeeze window.
        if !((d > 0 && c[n - 1] > wh) || (d < 0 && c[n - 1] < wl)) {
            return 0;
        }
        d
    }
}

impl Strategy for SqueezeBreakout {
    fn name(&self) -> &'static str {
        "s15_squeeze_breakout"
    }

    fn symbols(&self) -> &'static [&'static str] {
        &["SPY", "QQQ"]
    }

    fn interval(&self) -> Interval {
        Interval::Hourly
    }

    fn warmup_bars(&self) -> usize {
        40
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        if ctx.is_last_bar_of_day() {
            return Ok(());
        }
        let held = ctx.positions();
        for sym in self.symbols().iter().copied() {
            if held.iter().any(|p| p.structure.symbol == sym) {
                continue;
            }
            let sig = Self::squeeze_signal(ctx, sym);
            if sig == 0 {
                continue;
            }
            let exp = ctx.expiry(DTE);
            let call = sig > 0;
            let k = ctx.strike_for_delta(sym, call, LONG_DELTA, exp, 1.0);
            let short_k = if call { k + WIDTH } else { k - WIDTH };
            let exits = ExitRules { take_profit: Some(TP), stop_loss: Some(SL), max_hold_bars: Some(MAX_HOLD_BARS), minutes_before_close: Some(EXIT_MIN_BEFORE_CLOSE), ..Default::default() };
            ctx.open(vertical(sym, exp, call, k, short_k)?, exits, if call { "squeeze_up" } else { "squeeze_dn" }, None);
        }
        Ok(())
    }
}
