"""Volatility-risk-premium harvest: ~30-45 DTE iron condor on SPY (or QQQ).

Sell a $1-wing iron condor ~28 sessions out (~41 calendar days), take profit at 50% of
credit, exit at ~21 calendar days (15 sessions) to expiry or on a loss stop.

NOTE the $60 risk cap: with $1 wings the textbook 16-delta condor collects only ~$0.22-0.24
(max loss ~$82 incl. fees) and is rejected by the engine. Short strikes must be ~30+ delta
for the structure to fit, so SHORT_DELTA defaults to 0.32 (see research note).
"""
from __future__ import annotations

from trading.strategy import ExitRules, Strategy
from trading.structures import iron_condor


class VrpCondor(Strategy):
    name = "s11_vrp_condor"
    description = ("30-45 DTE $1-wing iron condor (shorts ~32-delta so max loss fits the $60 cap) "
                   "on SPY; TP 50% of credit, exit at 21 DTE or loss stop; one at a time")
    symbols = ("SPY",)
    interval = "1d"
    start = "2005-01-01"

    DTE_SESSIONS = 28        # +1 session for the next-open fill -> ~41 calendar days
    SHORT_DELTA = 0.32
    WING = 1.0
    TAKE_PROFIT = 0.50       # fraction of credit
    STOP_LOSS = 1.0          # loss = 1x credit
    EXIT_DTE = 15            # sessions (~21 calendar days)
    MAX_CONCURRENT = 1
    MIN_VIX = 0.0            # decimal, e.g. 0.15 -> only sell when index vol >= 15
    ENTRY_WEEKDAY = 0        # decide on Mondays only (speed + staggering)
    FEASIBILITY_CHECK = True  # skip entries the engine would reject as too_risky_for_account

    @staticmethod
    def _strike(ctx, sym, cp, target, exp):
        """Faster than ctx.strike_for_delta (14 bisect steps instead of 40)."""
        spot = ctx.spot(sym)
        d = lambda k: abs(ctx.quote(sym, cp, k, exp).delta)
        lo, hi = spot * 0.7, spot * 1.3
        for _ in range(14):
            m = 0.5 * (lo + hi)
            if (d(m) > target) == (cp == "C"):
                lo = m
            else:
                hi = m
        k0 = round(0.5 * (lo + hi))
        return min((float(k0 + j) for j in (-1, 0, 1)), key=lambda k: abs(d(k) - target))

    def on_bar(self, ctx):
        sym = self.symbols[0]
        if len(ctx.positions) + ctx.pending_entries >= self.MAX_CONCURRENT:
            return
        if self.ENTRY_WEEKDAY >= 0 and ctx.today.weekday() != self.ENTRY_WEEKDAY:
            return
        if ctx.vol(sym).atm < self.MIN_VIX:
            return
        exp = ctx.expiry(self.DTE_SESSIONS + 1)
        ps = self._strike(ctx, sym, "P", self.SHORT_DELTA, exp)
        cs = self._strike(ctx, sym, "C", self.SHORT_DELTA, exp)
        if cs <= ps:
            return
        st = iron_condor(sym, exp, ps - self.WING, ps, cs, cs + self.WING)
        credit = -ctx.price(st)["entry"]
        if credit <= 0:
            return
        if self.FEASIBILITY_CHECK and (self.WING - credit) * 100 + 4 * 0.65 * 2 > 0.03 * ctx.equity:
            return
        ctx.open(st, ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS,
                               exit_dte=self.EXIT_DTE), tag="vrp")


STRATEGY = VrpCondor
