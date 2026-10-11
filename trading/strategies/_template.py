"""TEMPLATE / BASELINE — copy this file to start a bot.

Baseline: every Monday, buy a ~0.40-delta SPY call debit spread ($1 wide) expiring in
~5 sessions; take profit at +80% of debit, stop at -50%, exit 1 session before expiry.
It exists to exercise the harness, not to make money.
"""
from __future__ import annotations

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


class WeeklyCallSpreadBaseline(Strategy):
    name = "baseline_weekly_call_spread"
    description = "Monday 5-session 0.40-delta $1 call debit spread on SPY"
    symbols = ("SPY",)
    interval = "1d"
    warmup_bars = 0

    DTE = 5
    DELTA = 0.40
    WIDTH = 1.0

    def on_bar(self, ctx):
        # Decide at Friday's close -> fills at Monday's open.
        if ctx.today.weekday() != 4 or ctx.positions or ctx.pending_entries:
            return
        exp = ctx.expiry(self.DTE + 1)  # +1: the fill happens next session
        k = ctx.strike_for_delta("SPY", "C", self.DELTA, exp)
        ctx.open(
            vertical("SPY", exp, "C", k, k + self.WIDTH),
            ExitRules(take_profit=0.8, stop_loss=0.5, exit_dte=1),
            tag="weekly",
        )


STRATEGY = WeeklyCallSpreadBaseline
