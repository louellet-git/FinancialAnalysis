"""Last-hour momentum (Gao, Han, Li & Zhou 2018, "Market intraday momentum", JFE).

The return over the first part of the session (prior close -> first bar close) and the
return of the day so far predict the return of the final half hour, same sign. At 15:30
ET buy a narrow 0DTE debit spread in the signalled direction and hold it to expiry
(expiry settlement is NOT a day trade under the PDT rule, so no PDT slot is used).

1h bars: 09:30..14:30 bars are full hours, the 15:30 bar is the final half hour. The
decision is made at the 14:30 bar close (15:30 ET); the order fills at the 15:30 open.
"""
from __future__ import annotations

import math

from trading.strategy import ExitRules, Strategy
import math as _m
from trading.structures import vertical


class LastHourMomentum(Strategy):
    name = "s06_last_hour_momentum"
    description = ("15:30 ET 0DTE debit spread in the direction of first-hour / day-so-far "
                   "return (Gao et al. intraday momentum), held to expiry")
    symbols = ("SPY",)
    interval = "1h"
    warmup_bars = 0

    SIGNAL = "first"        # "first" = prior close->end of first bar; "day" = prior close->15:30;
                            # "both" = require first and day-so-far to agree
    Z_MIN = 0.3             # |return| must exceed Z_MIN * (daily sigma implied by VIX)
    WIDTH = 1.0
    LONG_OFFSET = 0.0       # long strike distance from spot, $ beyond the first strike on the OTM side of spot (ceil for calls, floor for puts)
    MAX_DAILY_Z = 3.0       # skip extreme-move days (gap/news; spread would be far ITM/OTM)
    TAKE_PROFIT = None
    STOP_LOSS = None

    def on_start(self, ctx):
        ctx.state["done"] = None

    def on_bar(self, ctx):
        # Decide when the bar that closes at 15:30 ET has just printed.
        if round(ctx.minutes_to_close) != 30 or ctx.positions or ctx.pending_entries:
            return
        sym = self.symbols[0]
        d = ctx.daily(sym)
        if len(d) < 1:
            return
        prev = float(d["close"].iat[-1])
        tb = ctx.today_bars(sym)
        if len(tb) < 2:
            return
        first = float(tb["close"].iat[0]) / prev - 1.0
        day = ctx.spot(sym) / prev - 1.0
        sigma_d = ctx.vol(sym).atm / math.sqrt(252.0)
        zf, zd = first / sigma_d, day / sigma_d
        if self.SIGNAL == "first":
            z = zf
        elif self.SIGNAL == "day":
            z = zd
        else:
            z = zf if (zf * zd > 0 and min(abs(zf), abs(zd)) >= self.Z_MIN) else 0.0
        if abs(z) < self.Z_MIN or abs(zd) > self.MAX_DAILY_Z:
            return
        spot = ctx.spot(sym)
        exp = ctx.expiry(0)
        cp = "C" if z > 0 else "P"
        k = float(_m.ceil(spot) + self.LONG_OFFSET) if cp == "C" else float(_m.floor(spot) - self.LONG_OFFSET)
        k2 = k + self.WIDTH if cp == "C" else k - self.WIDTH
        ctx.open(vertical(sym, exp, cp, k, k2),
                 ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS),
                 tag=f"{cp}_z{z:+.2f}")


STRATEGY = LastHourMomentum
