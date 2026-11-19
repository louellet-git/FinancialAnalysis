"""s14b — s14 with the rotation removed: always SPY (orchestrator ablation, 2026-10-11).

Same rules as s14 (weekly staggered ~20-session $1 call debit spreads, ~0.50 delta,
held to expiry, only when SPY > 200-day SMA and VIX < 35%) but never switches to QQQ.

Why it exists: the rotation did not reliably beat this. From 2005 the two are close
(+399% vs +358%, OOS Sharpe 0.60 vs 0.66); from 2012 rotation is ahead (+306% vs +212%);
but started in mid-2001 the rotation rode QQQ through the dot-com bust and was ruined
(-93%) while always-SPY made +461%. The robust ingredient is the trend-filtered long
call-spread carry, not the SPY/QQQ choice. See trading/research/LEADERBOARD.md.
"""
from trading.strategies.s14_spy_qqq_rotation import SpyQqqRotation


class SpyTrendCalls(SpyQqqRotation):
    name = "s14b_spy_trend_calls"
    description = "s14 without rotation: weekly SPY call debit spreads held to expiry, SPY>SMA200 & VIX<35%"
    FIXED_LEADER = "SPY"


STRATEGY = SpyTrendCalls
