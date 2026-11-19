"""Overnight drift: buy a short-dated call debit spread late in the session, sell next morning.

Signal on the 14:30 bar (known at 15:30) -> engine fills at the 15:30 bar's open (the
latest 1h-bar entry available). Exit by max_hold_bars=2: the 15:30 bar counts as bar 1,
the next day's 09:30 bar as bar 2, so the exit happens at that bar's CLOSE (10:30).
(The engine cannot sell at the open; this is a harness limitation.)
"""
from __future__ import annotations

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


class OvernightDrift(Strategy):
    name = "s07_overnight_drift"
    description = ("Buy ~ATM call debit spread at 15:30 (1-2DTE), sell next morning 10:30; "
                   "filters: daily close > SMA, VIX below cap, skip Fridays/prior-day-gap-down.")
    symbols = ("SPY",)
    interval = "1h"
    warmup_bars = 10

    DTE = 1              # sessions to expiry from signal day
    DELTA = 0.50
    WIDTH = 1.0
    SMA_DAYS = 50        # 0 disables trend filter
    VIX_MAX = 30.0       # on atm vol (VIX/VXN) in points; 0 disables
    SKIP_FRIDAY = True
    STOP_LOSS = None

    def on_bar(self, ctx):
        ts = ctx.ts
        if not (ts.hour == 14 and ts.minute == 30):
            return
        if ctx.positions or ctx.pending_entries:
            return
        if self.SKIP_FRIDAY and ctx.today.weekday() == 4:
            return
        sym = self.symbols[0]
        if self.SMA_DAYS:
            d = ctx.daily(sym)
            if len(d) < self.SMA_DAYS or ctx.spot(sym) < d["close"].iloc[-self.SMA_DAYS:].mean():
                return
        if self.VIX_MAX and ctx.vol(sym).atm * 100 > self.VIX_MAX:
            return
        exp = ctx.expiry(self.DTE)
        k = ctx.strike_for_delta(sym, "C", self.DELTA, exp)
        ctx.open(vertical(sym, exp, "C", k, k + self.WIDTH),
                 ExitRules(max_hold_bars=2, stop_loss=self.STOP_LOSS), tag="overnight")


STRATEGY = OvernightDrift
