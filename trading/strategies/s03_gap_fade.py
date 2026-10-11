"""Overnight gap fade: fade moderate opening gaps toward the prior close with 0DTE debit spreads."""
from __future__ import annotations

import math

from trading.strategy import ExitRules, Strategy
from trading.structures import round_strike, vertical


class GapFade(Strategy):
    name = "s03_gap_fade"
    description = ("Fade moderate overnight gaps (0.5-1.5x VIX-implied daily move) after the first hour: "
                   "gap up -> 0DTE put debit spread, gap down -> 0DTE call debit spread, "
                   "exit at gap fill / stop / time.")
    symbols = ("SPY",)
    interval = "1h"
    warmup_bars = 10

    MIN_Z = 0.5          # min |gap| / (VIX-implied 1-day move)
    MAX_Z = 1.5          # skip big news gaps
    MIN_REMAIN = 0.5     # require >= this fraction of the gap still unfilled at 10:30
    WIDTH = 1.0
    DTE = 0             # 0 = 0DTE; 1 = 1DTE
    TAKE_PROFIT = 0.8
    STOP_LOSS = 0.5
    MINUTES_BEFORE_CLOSE = 60
    GAP_FILL_EXIT = True  # close once spot has retraced to the prior close (gap filled)

    def on_bar(self, ctx):
        if ctx.positions:
            prev = ctx.state.get("prev")
            if self.GAP_FILL_EXIT and prev is not None:
                spot = ctx.spot()
                for pos in ctx.positions:
                    up = pos.tag == "gap_up"
                    if (up and spot <= prev) or (not up and spot >= prev):
                        ctx.close(pos, "gap_fill")
            return
        if ctx.pending_entries:
            return
        tb = ctx.today_bars()
        if len(tb) != 1:
            return
        d = ctx.daily()
        if len(d) == 0:
            return
        prev = float(d["close"].iat[-1])
        o = float(tb["open"].iat[0])
        spot = ctx.spot()
        gap = o - prev
        vix = ctx.vol().atm
        daily_move = vix / math.sqrt(252.0) * prev
        z = abs(gap) / daily_move
        if not (self.MIN_Z <= z <= self.MAX_Z):
            return
        remain = (spot - prev) * (1 if gap > 0 else -1)   # still-unfilled distance in gap direction
        if remain < self.MIN_REMAIN * abs(gap):
            return
        sym = self.symbols[0]
        exp = ctx.expiry(self.DTE)
        k = round_strike(spot)
        if gap > 0:
            st = vertical(sym, exp, "P", k, k - self.WIDTH)
        else:
            st = vertical(sym, exp, "C", k, k + self.WIDTH)
        ctx.state["prev"] = prev
        ctx.open(st, ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS,
                               minutes_before_close=self.MINUTES_BEFORE_CLOSE if self.DTE == 0 else None,
                               exit_dte=None if self.DTE == 0 else 0),
                 tag="gap_up" if gap > 0 else "gap_down")


STRATEGY = GapFade
