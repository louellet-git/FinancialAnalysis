"""s05_ic_0dte - regime-gated 0DTE iron butterfly / iron condor on SPY (1h bars).

Sell a narrow, near-ATM 0DTE iron fly/condor mid-morning ONLY when the intraday
volatility regime looks calm: the 1-day implied vol (VIX1D) is low relative to 30-day
VIX, the morning's realized move is small relative to what the implied vol priced, and
the prior day was not a trend day. Exits: profit target, stop, otherwise hold to expiry
(expiry settlement is not a PDT day trade; an early exit IS, and only 3 are allowed per
5 sessions on a <$25k account).
"""
from __future__ import annotations

import math

from trading.strategy import ExitRules, Strategy
from trading.structures import iron_condor, round_strike


class IC0DTE(Strategy):
    name = "s05_ic_0dte"
    description = ("Gated 0DTE SPY iron fly/condor, mid-morning entry, $2 wings, "
                   "VIX1D/VIX + realized-move + prior-day-range gates")
    symbols = ("SPY",)
    interval = "1h"
    start = None

    ENTRY_BAR = 1            # decide after this many bars of the day have closed (1 -> fill 10:30)
    WING = 2.0               # wing width in $ ($2 iron fly: credit ~1.5, max loss ~0.5/share)
    BODY_OFFSET = 0.0        # short strikes this many $ beyond ATM each side (0 = iron fly)
    MIN_CREDIT = 1.40        # skip if net credit/share (after spread) is below this (keeps max loss in budget)
    MAX_RATIO = 0.80         # VIX1D / VIX must be <= this (calm intraday regime)
    MAX_SHORT_VOL = 0.20     # skip when 1-day IV is above this (event / stress days)
    MAX_MORNING_MOVE = 0.60  # |spot - day open| / (1-day expected move to date) must be <= this
    MAX_PRIOR_RANGE = 1.5    # prior day (high-low)/close in units of prior-day implied 1-day move
    TAKE_PROFIT = 0.50       # fraction of credit
    STOP_LOSS = 1.00         # loss as multiple of credit; effectively inactive (wings cap loss at ~0.3x credit)
    USE_STOP = True

    def on_bar(self, ctx):
        if ctx.positions or ctx.pending_entries:
            return
        tb = ctx.today_bars()
        if len(tb) != self.ENTRY_BAR:
            return
        spot = ctx.spot()
        v = ctx.vol()
        if v.atm <= 0 or v.short / v.atm > self.MAX_RATIO or v.short > self.MAX_SHORT_VOL:
            return
        # realized morning move vs the move the 1-day IV priced for the elapsed time
        elapsed_frac = len(tb) * 60.0 / 390.0
        exp_move = spot * v.short * math.sqrt(elapsed_frac / 252.0)
        if exp_move > 0 and abs(spot - float(tb["open"].iat[0])) / exp_move > self.MAX_MORNING_MOVE:
            return
        d = ctx.daily()
        if len(d) >= 2:
            last = d.iloc[-1]
            day_move = last["close"] * v.short / math.sqrt(252.0)
            if day_move > 0 and (last["high"] - last["low"]) > self.MAX_PRIOR_RANGE * day_move * 2.0:
                return
        exp = ctx.expiry(0)
        atm = round_strike(spot)
        ps, cs = atm - self.BODY_OFFSET, atm + self.BODY_OFFSET
        st = iron_condor("SPY", exp, ps - self.WING, ps, cs, cs + self.WING)
        credit = -ctx.price(st)["entry"]
        if credit < self.MIN_CREDIT:
            return
        ctx.open(st, ExitRules(take_profit=self.TAKE_PROFIT,
                               stop_loss=self.STOP_LOSS if self.USE_STOP else None), tag="ic0dte")


STRATEGY = IC0DTE
