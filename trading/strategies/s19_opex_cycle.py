"""s19_opex_cycle - monthly options-expiration (OPEX) cycle, defined-risk weekly spreads.

Calendar (no data needed): third Friday of each month = monthly OPEX; Mar/Jun/Sep/Dec = quad
witching. Each week is classified as the OPEX week or the week AFTER OPEX, and in quad vs
non-quad months. Per bucket a side is chosen (+1 bullish call debit spread, -1 bearish put
debit spread, 0 skip). Entry: last bar of the prior week (fills Monday open). Expiry: Friday
(or Thursday if Friday is a holiday) of the target week.
"""
from __future__ import annotations

from datetime import timedelta

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


def _third_friday(d):
    return 15 <= d.day <= 21 and d.weekday() == 4


class OpexCycle(Strategy):
    name = "s19_opex_cycle"
    description = ("Monthly OPEX-cycle weekly $1 debit spreads on SPY: bullish in OPEX week, "
                   "bearish in the week after quad-witching, Monday open -> Friday expiry.")
    symbols = ("SPY",)
    interval = "1d"
    start = "2005-01-01"

    # side per bucket: +1 bull call spread, -1 bear put spread, 0 skip
    SIDE_OPEX_QUAD = 1
    SIDE_OPEX_NON = 1
    SIDE_AFTER_QUAD = -1
    SIDE_AFTER_NON = 0
    DELTA = 0.45
    WIDTH = 1.0
    TAKE_PROFIT = 0.8
    STOP_LOSS = 0.5
    EXIT_DTE = 1

    def on_bar(self, ctx):
        if ctx.positions or ctx.pending_entries:
            return
        today = ctx.today
        nxt = ctx.expiry(1)
        if nxt.isocalendar()[1] == today.isocalendar()[1] and nxt.year == today.year:
            return  # only decide on the last session of a week
        fri = nxt + timedelta(days=4 - nxt.weekday())
        if _third_friday(fri):
            kind, quad = "opex", fri.month % 3 == 0
        elif _third_friday(fri - timedelta(days=7)):
            kind, quad = "after", (fri - timedelta(days=7)).month % 3 == 0
        else:
            return
        side = {("opex", True): self.SIDE_OPEX_QUAD, ("opex", False): self.SIDE_OPEX_NON,
                ("after", True): self.SIDE_AFTER_QUAD, ("after", False): self.SIDE_AFTER_NON}[(kind, quad)]
        if side == 0:
            return
        exp = np.busday_offset(fri, 0, roll="backward").astype(object)
        cp = "C" if side > 0 else "P"
        k = ctx.strike_for_delta("SPY", cp, self.DELTA, exp)
        other = k + self.WIDTH if side > 0 else k - self.WIDTH
        ctx.open(vertical("SPY", exp, cp, k, other),
                 ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS, exit_dte=self.EXIT_DTE),
                 tag=f"{kind}_{'quad' if quad else 'non'}_{'bull' if side > 0 else 'bear'}")


STRATEGY = OpexCycle
