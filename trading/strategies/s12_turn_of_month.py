"""Turn-of-the-month (TOM) seasonality via short-dated defined-risk directional spreads."""
from __future__ import annotations

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


class TurnOfMonth(Strategy):
    name = "s12_turn_of_month"
    description = ("Hold a bullish SPY spread (call debit or put credit) from ~N sessions before month-end "
                   "through the Mth session of the new month (turn-of-month effect); flat otherwise.")
    symbols = ("SPY",)
    interval = "1d"
    start = "2005-01-01"

    # Window: position is live from the open of the PRE-th-to-last session of the month
    # through the close of the POST-th session of the next month.
    PRE = 3
    POST = 3
    KIND = "call_debit"      # "call_debit" or "put_credit"
    DELTA = 0.50             # long-leg (debit) / short-leg (credit) |delta|
    WIDTH = 1.0
    DTE = 9                  # sessions to expiry at the fill (must exceed window length)
    STOP = 0.6               # stop loss as fraction of premium (None = off)

    def on_bar(self, ctx):
        today = ctx.today
        if ctx.positions:
            pos = ctx.positions[0]
            idx = ctx.bars().index
            n_new = int(((idx.month == today.month) & (idx.year == today.year)).sum())
            held_into_new = pos.entry_time.month != today.month or pos.entry_time.year != today.year
            if held_into_new and n_new >= self.POST:
                ctx.close(pos, "tom_window_end")
            return
        if ctx.pending_entries:
            return
        # busdays from today to the first business day of next month
        nxt = np.datetime64(today, "M") + 1
        k = int(np.busday_count(today, nxt.astype("datetime64[D]")))
        if k > self.PRE + 1 or ctx.state.get("traded") == (today.year, today.month):
            return
        ctx.state["traded"] = (today.year, today.month)
        exp = ctx.expiry(self.DTE + 1)
        sym = self.symbols[0]
        if self.KIND == "call_debit":
            kl = ctx.strike_for_delta(sym, "C", self.DELTA, exp)
            s = vertical(sym, exp, "C", kl, kl + self.WIDTH)
        else:
            ks = ctx.strike_for_delta(sym, "P", self.DELTA, exp)
            s = vertical(sym, exp, "P", ks - self.WIDTH, ks)
        ctx.open(s, ExitRules(stop_loss=self.STOP, exit_dte=1), tag="tom")


STRATEGY = TurnOfMonth
