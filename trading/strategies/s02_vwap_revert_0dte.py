"""Intraday VWAP mean reversion via 0DTE $1-wide debit spreads (hourly bars).

Session VWAP is built from ctx.today_bars (typical price x volume). When the close
stretches more than K x (VIX1D-implied daily 1-sd move) away from VWAP in a session that
has NOT been trending (efficiency ratio open->now / range below ER_MAX), buy a 0DTE debit
vertical betting on a move back toward VWAP. Exit when price re-touches VWAP (bar close
crosses it), on the engine's TP/SL, or ~1h before the close.
"""
from __future__ import annotations

import math

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


class VwapRevert0DTE(Strategy):
    name = "s02_vwap_revert_0dte"
    description = ("Fade stretches from session VWAP (> K x VIX1D daily sd) in non-trending sessions "
                   "with a 0DTE $1 debit spread; exit at VWAP touch / stop / 1h before close")
    symbols = ("QQQ",)
    interval = "1h"
    warmup_bars = 10
    start = None

    K = 0.25            # stretch threshold, in units of VIX1D-implied daily 1-sd move
    ER_MAX = 0.6        # max |close-open| / (high-low) so far => non-trending session
    FIRST_BAR = 1       # earliest decision bar index in the day (1 = close of 10:30 bar)
    LAST_BAR = 4        # latest decision bar index (4 = 13:30-14:30 bar, fills 14:30)
    DELTA = 0.45        # long-leg delta
    WIDTH = 1.0
    TAKE_PROFIT = 0.5
    STOP_LOSS = 1.0
    MINUTES_BEFORE_CLOSE = 60
    MODE = "credit"     # "debit" (long delta DELTA) or "credit" (short-leg delta CREDIT_DELTA)
    CREDIT_DELTA = 0.50
    VWAP_EXIT_BUFFER = 0.0   # exit when |dist| < buffer*sd on the reverting side

    def _vwap(self, tb):
        tp = (tb["high"] + tb["low"] + tb["close"]) / 3.0
        vol = tb["volume"]
        return float((tp * vol).sum() / vol.sum()) if vol.sum() > 0 else float(tb["close"].mean())

    def on_bar(self, ctx):
        sym = self.symbols[0]
        tb = ctx.today_bars(sym)
        n = len(tb)
        if n < 2:
            return
        vwap = self._vwap(tb)
        px = ctx.spot(sym)
        sd = ctx.vol(sym).short / math.sqrt(252.0)  # daily 1-sd, as fraction

        # --- exits: VWAP touch (reversion complete) -----------------------
        for pos in ctx.positions:
            if pos.tag == "long_rev" and px >= vwap * (1 - self.VWAP_EXIT_BUFFER * sd):
                ctx.close(pos, "vwap_touch")
            elif pos.tag == "short_rev" and px <= vwap * (1 + self.VWAP_EXIT_BUFFER * sd):
                ctx.close(pos, "vwap_touch")

        # --- entries ---------------------------------------------------------
        if ctx.positions or ctx.pending_entries:
            return
        j = n - 1
        if j < self.FIRST_BAR or j > self.LAST_BAR:
            return
        if ctx.state.get("traded_day") == ctx.today:
            return
        rng = float(tb["high"].max() - tb["low"].min())
        er = abs(px - float(tb["open"].iat[0])) / rng if rng > 0 else 0.0
        if er > self.ER_MAX:
            return
        z = (px - vwap) / (px * sd) if sd > 0 else 0.0
        if abs(z) < self.K:
            return
        exp = ctx.expiry(0)
        exits = ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS,
                          minutes_before_close=self.MINUTES_BEFORE_CLOSE)
        if self.MODE == "credit":
            # below VWAP -> expect up/hold -> put credit spread (short put below spot)
            if z < 0:
                k = ctx.strike_for_delta(sym, "P", self.CREDIT_DELTA, exp)
                st = vertical(sym, exp, "P", k - self.WIDTH, k); tag = "long_rev"
            else:
                k = ctx.strike_for_delta(sym, "C", self.CREDIT_DELTA, exp)
                st = vertical(sym, exp, "C", k + self.WIDTH, k); tag = "short_rev"
        elif z < 0:   # below VWAP -> expect up -> call debit spread
            k = ctx.strike_for_delta(sym, "C", self.DELTA, exp)
            st = vertical(sym, exp, "C", k, k + self.WIDTH); tag = "long_rev"
        else:
            k = ctx.strike_for_delta(sym, "P", self.DELTA, exp)
            st = vertical(sym, exp, "P", k, k - self.WIDTH); tag = "short_rev"
        ctx.open(st, exits, tag=tag)
        ctx.state["traded_day"] = ctx.today


STRATEGY = VwapRevert0DTE
