"""s10_vix_regime — volatility-regime timing for SPY/QQQ short-dated vertical spreads.

Regimes are classified from the 30-day vol index only (VIX for SPY, VXN for QQQ), using
ctx.vol_history (no lookahead):
  calm (sell put credit spread): VIX within a band of its 50d MA, percentile (252d) in
        [PCT_LO, PCT_HI], VIX below VIX_MAX  -> premium is decent but not a spike, and the
        market is not complacent (floor on percentile avoids selling at vol lows).
  breakout (buy put debit spread): VIX jumped >= BREAKOUT_RATIO x its MA, or rose
        >= BREAKOUT_5D over 5 sessions while the index is below its 20d MA.
  otherwise / panic (VIX > PANIC_VIX): stand aside.
"""
from __future__ import annotations

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


class VixRegime(Strategy):
    name = "s10_vix_regime"
    description = ("VIX/VXN regime timing: put credit spreads in calm-not-complacent vol, "
                   "put debit spreads on vol breakouts, flat in panic/otherwise")
    symbols = ("QQQ",)
    interval = "1d"
    warmup_bars = 260
    start = "2005-01-01"

    DTE = 21
    WIDTH = 1.0
    SHORT_DELTA = 0.30
    MAX_DELTA = 0.55
    MA_LEN = 50
    PCT_LEN = 252
    PCT_LO = 0.20
    PCT_HI = 0.75
    CALM_RATIO_MAX = 1.05      # VIX / MA50 upper bound for "calm"
    VIX_MAX = 24.0
    PANIC_VIX = 35.0
    BREAKOUT_RATIO = 1.25
    BREAKOUT_5D = 0.30
    DEBIT_DELTA = 0.45         # long put delta for the debit spread
    DEBIT = False
    CR_TP = 0.50
    CR_SL = 9.0   # effectively off: loss is capped by the spread
    DB_TP = 0.80
    DB_SL = 0.50
    EXIT_DTE = 3

    def _fit_credit(self, ctx, sym, exp):
        """Lowest-delta short put (>= SHORT_DELTA) whose $-width credit spread fits the risk cap."""
        budget = ctx.equity * 0.03
        k0 = ctx.strike_for_delta(sym, "P", self.SHORT_DELTA, exp)
        spot = ctx.spot(sym)
        k = k0
        while k <= spot * 1.02:
            st = vertical(sym, exp, "P", k - self.WIDTH, k)
            loss = (ctx.price(st)["entry"] + self.WIDTH) * 100 + 2.6
            if loss <= budget * 0.97:
                return st
            k += 1.0
        return None

    def on_bar(self, ctx):
        if ctx.positions or ctx.pending_entries:
            return
        sym = self.symbols[0]
        v = ctx.vol_history(sym)["atm_close"].to_numpy()[-(self.PCT_LEN + 1):]
        if len(v) < self.PCT_LEN:
            return
        vix = v[-1]
        ma = v[-self.MA_LEN:].mean()
        pct = (v[:-1] < vix).mean()
        ratio = vix / ma
        px = ctx.bars(sym)["close"].to_numpy()
        below_ma20 = px[-1] < px[-20:].mean()
        chg5 = vix / v[-6] - 1.0

        if vix > self.PANIC_VIX:
            return
        exp = ctx.expiry(self.DTE + 1)
        breakout = ratio >= self.BREAKOUT_RATIO or (chg5 >= self.BREAKOUT_5D and below_ma20)
        if breakout:
            if not self.DEBIT:
                return
            k = ctx.strike_for_delta(sym, "P", self.DEBIT_DELTA, exp)
            ctx.open(vertical(sym, exp, "P", k, k - self.WIDTH),
                     ExitRules(take_profit=self.DB_TP, stop_loss=self.DB_SL, exit_dte=1), tag="breakout")
            return
        calm = (ratio <= self.CALM_RATIO_MAX and self.PCT_LO <= pct <= self.PCT_HI
                and vix <= self.VIX_MAX and px[-1] > px[-50:].mean() * 0.98)
        if calm:
            st = self._fit_credit(ctx, sym, exp)
            if st is None:
                return
            ctx.open(st,
                     ExitRules(take_profit=self.CR_TP, stop_loss=self.CR_SL, exit_dte=self.EXIT_DTE),
                     tag="calm")


STRATEGY = VixRegime
