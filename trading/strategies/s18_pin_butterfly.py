"""Late-day 0DTE pin butterfly.

Around DECISION_MIN_TO_CLOSE minutes before the close, if the afternoon has been quiet
and spot sits close to a candidate pin level (round number, session VWAP or the
afternoon range midpoint), buy a cheap $WIDTH-wide butterfly (or sell an iron
butterfly) centred on that level and HOLD TO EXPIRY (settlement is not a day trade, so
no PDT slot is used). Max loss = debit (butterfly) or width - credit (iron fly).

NOTE: real-world pinning is driven by dealer gamma hedging / open interest, which this
synthetic data cannot see. Any edge here is purely price-path based (intraday vol
persistence vs the VIX1D-based model price).
"""
from __future__ import annotations

import math

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import butterfly, iron_condor, round_strike


class PinButterfly(Strategy):
    name = "s18_pin_butterfly"
    description = ("Late-day 0DTE call butterfly / iron butterfly centred on a pin level "
                   "(round number, VWAP or afternoon midpoint), held to expiry")
    symbols = ("SPY",)
    interval = "1h"
    warmup_bars = 10

    DECISION_MIN_TO_CLOSE = 90      # decide on the bar closing nearest this many minutes before 16:00
    CENTER_MODE = "vwap"            # "round" | "vwap" | "mid" (afternoon range midpoint)
    ROUND_STEP = 5.0                # for "round": nearest multiple (SPY 5, QQQ 5)
    WIDTH = 1.0                     # wing width in $
    STRUCT = "fly"                  # "fly" (debit call butterfly) | "iron" (credit iron butterfly)
    MAX_DIST_SIGMA = 0.35           # |spot - center| <= this * remaining 1-sigma move
    QUIET_RATIO = 0.8               # realized afternoon move / implied must be <= this (0 = off)
    MAX_COST_FRAC = 0.35            # fly debit <= this * WIDTH (cheap entries only)
    MIN_CREDIT_FRAC = 0.55          # iron: credit >= this * WIDTH

    def _center(self, ctx, bars) -> float:
        spot = ctx.spot()
        if self.CENTER_MODE == "round":
            c = round(spot / self.ROUND_STEP) * self.ROUND_STEP
        elif self.CENTER_MODE == "vwap":
            tp = (bars["high"] + bars["low"] + bars["close"]) / 3.0
            v = bars["volume"].astype(float)
            c = float((tp * v).sum() / v.sum()) if v.sum() > 0 else float(tp.mean())
        else:
            c = float((bars["high"].max() + bars["low"].min()) / 2.0)
        return c

    def on_bar(self, ctx):
        if ctx.positions or ctx.pending_entries:
            return
        # one decision per day: the bar whose close is nearest the target time
        mtc = ctx.minutes_to_close
        step = 60 if self.interval == "1h" else 5
        if not (self.DECISION_MIN_TO_CLOSE - step / 2 < mtc <= self.DECISION_MIN_TO_CLOSE + step / 2):
            return
        bars = ctx.today_bars()
        if len(bars) < 3:
            return
        spot = ctx.spot()
        vol = ctx.vol()
        sigma_rem = vol.short * math.sqrt(mtc / (252 * 390.0)) * spot  # 1-sigma $ move to close
        if sigma_rem <= 0:
            return
        center = round_strike(self._center(ctx, bars), 1.0)
        if abs(spot - center) > self.MAX_DIST_SIGMA * sigma_rem:
            return
        if self.QUIET_RATIO > 0:
            # realized move since noon (or whole day so far if shorter): range / implied sigma over same span
            pm = bars[bars.index.hour >= 12] if self.interval == "5m" else bars.iloc[-3:]
            if len(pm) < 2:
                return
            rng = float(pm["high"].max() - pm["low"].min())
            span_min = len(pm) * (5 if self.interval == "5m" else 60)
            implied = vol.short * math.sqrt(span_min / (252 * 390.0)) * spot
            if rng > self.QUIET_RATIO * 2.0 * implied:
                return
        exp = ctx.expiry(0)
        w = self.WIDTH
        if self.STRUCT == "fly":
            s = butterfly(self.symbols[0], exp, "C", center - w, center, center + w)
            cost = ctx.price(s)["entry"]
            if cost <= 0 or cost > self.MAX_COST_FRAC * w:
                return
        else:
            s = iron_condor(self.symbols[0], exp, center - w, center, center, center + w)
            credit = -ctx.price(s)["entry"]
            if credit < self.MIN_CREDIT_FRAC * w:
                return
        ctx.open(s, ExitRules(), tag=f"{self.CENTER_MODE}")


STRATEGY = PinButterfly
