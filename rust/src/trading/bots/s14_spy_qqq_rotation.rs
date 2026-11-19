//! s14 - SPY/QQQ relative-strength rotation with narrow call debit spreads (daily bars).
//!
//! Idea: (1) the QQQ/SPY ratio trends (leadership regimes last for months), and (2) only be long
//! when the leader is in an uptrend itself (absolute momentum, a 200-day SMA filter).
//!
//! The signal is evaluated on each bar, and the entries are on Fridays (fill on Monday):
//!   ratio  = QQQ.close / SPY.close
//!   leader = QQQ if ratio > SMA(ratio, 50) and the 63-day return of the ratio > 0
//!            SPY if ratio < SMA(ratio, 50) and the 63-day return of the ratio < 0
//!            else keep the previous leader (hysteresis)
//!   risk-on = leader.close > SMA(leader, 200) and the 30-day IV of the leader < 35%
//! Structure: a $1 call debit spread on the leader, long strike near 0.50 delta, 20 sessions to
//! expiry. If the debit with the commissions is above the budget, the strikes move up $1 at a
//! time (8 steps at most). At most 3 positions are open. HOLD TO EXPIRY (settle at intrinsic).
//! (The Python bot also had a "pair" mode, a flip exit and take profit knobs. All were off, and
//! they are not in the Rust code.)
//!
//! `s14b_spy_trend_calls` is the same bot with the leader fixed to SPY (the ablation without rotation).

use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::{vertical, Structure, MULTIPLIER};

const RS_SMA: usize = 50;
const LOOKBACK: usize = 63;
const TREND_SMA: usize = 200;
const MAX_IV: f64 = 0.35;
const DTE: i32 = 20;
const LONG_DELTA: f64 = 0.50;
const WIDTH: f64 = 1.0;
const BUDGET_FRAC: f64 = 0.95;
const RISK_PER_TRADE: f64 = 0.03;
const MAX_POSITIONS: usize = 3;

pub struct SpyQqqRotation {
    name: &'static str,
    fixed_leader: Option<&'static str>,
    leader: Option<&'static str>,
}

impl SpyQqqRotation {
    pub fn rotation() -> Self {
        SpyQqqRotation { name: "s14_spy_qqq_rotation", fixed_leader: None, leader: None }
    }

    /// s14b: always SPY.
    pub fn spy_only() -> Self {
        SpyQqqRotation { name: "s14b_spy_trend_calls", fixed_leader: Some("SPY"), leader: None }
    }

    fn signal(&mut self, ctx: &Ctx) -> (&'static str, bool, bool) {
        let spy = ctx.bars("SPY").close;
        let qqq = ctx.bars("QQQ").close;
        let n = spy.len();
        let ratio_at = |j: usize| qqq[j] / spy[j];
        let window: Vec<f64> = (n - RS_SMA..n).map(ratio_at).collect();
        let sma = stats::mean(&window);
        let r = ratio_at(n - 1);
        let mom = r / ratio_at(n - 1 - LOOKBACK) - 1.0;
        let mut leader = self.leader;
        if r > sma && mom > 0.0 {
            leader = Some("QQQ");
        } else if r < sma && mom < 0.0 {
            leader = Some("SPY");
        } else if leader.is_none() {
            leader = Some(if mom > 0.0 { "QQQ" } else { "SPY" });
        }
        self.leader = leader;
        let leader = self.fixed_leader.or(leader).expect("leader is set");
        let px = ctx.bars(leader).close;
        let trend_ok = px[px.len() - 1] > stats::mean(&px[px.len() - TREND_SMA..]);
        let iv_ok = ctx.vol(leader).atm < MAX_IV;
        (leader, trend_ok, iv_ok)
    }

    /// A call spread whose debit fits the budget; the strikes move up until it does.
    fn spread(ctx: &Ctx, sym: &'static str, exp: i32) -> Result<Option<Structure>, String> {
        let budget = ctx.equity() * RISK_PER_TRADE * BUDGET_FRAC;
        let mut k = ctx.strike_for_delta(sym, true, LONG_DELTA, exp, 1.0);
        for _ in 0..8 {
            let s = vertical(sym, exp, true, k, k + WIDTH)?;
            let debit = ctx.price(&s).entry;
            if debit > 0.0 && debit * MULTIPLIER + 4.0 * 0.65 <= budget {
                return Ok(Some(s));
            }
            k += 1.0; // further OTM is cheaper
        }
        Ok(None)
    }
}

impl Strategy for SpyQqqRotation {
    fn name(&self) -> &'static str {
        self.name
    }

    fn symbols(&self) -> &'static [&'static str] {
        &["SPY", "QQQ"]
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
        if ctx.bars("SPY").len() < TREND_SMA.max(LOOKBACK + 1).max(RS_SMA) {
            return Ok(());
        }
        let (leader, trend_ok, iv_ok) = self.signal(ctx);
        if crate::trading::calendar::weekday(ctx.today()) != 4 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        if !(trend_ok && iv_ok) {
            return Ok(());
        }
        if ctx.open_position_count() + 1 > MAX_POSITIONS {
            return Ok(());
        }
        let exp = ctx.expiry(DTE + 1);
        let Some(s) = Self::spread(ctx, leader, exp)? else { return Ok(()) };
        ctx.open(s, ExitRules::default(), &format!("bull_{leader}"), None);
        Ok(())
    }
}
