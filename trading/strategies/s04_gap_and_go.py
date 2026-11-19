"""Gap-and-Go: continuation debit spreads after a large, unfilled opening gap."""
from __future__ import annotations

import math

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


class GapAndGo(Strategy):
    name = "s04_gap_and_go"
    description = ("Large gap (vs VIX-implied daily move) that has NOT filled after the first hour: buy a narrow "
                   "debit spread in the gap direction; exit on target/stop, on a move back into the prior-day "
                   "range, or by time. Contrast with gap-FADE bots.")
    symbols = ("SPY",)
    interval = "1h"
    warmup_bars = 3

    MIN_GAP_Z = 0.6        # |gap| / (VIX/sqrt(252)) threshold
    MAX_GAP_Z = 3.0        # skip extreme gaps (news shocks, mean-revert risk)
    HOLD_FRAC = 0.6        # first-hour close must retain >= this fraction of the gap
    CLOSE_CONFIRM = True   # first-hour bar must close in the gap direction (vs its open)
    DTE = 1                # 0 = 0DTE, 1 = 1DTE
    WIDTH = 1.0
    LONG_DELTA = 0.45
    TAKE_PROFIT = 0.8
    STOP_LOSS = 0.5
    EXIT_RANGE = True      # exit if spot returns inside prior-day high/low range
    EXIT_MINUTES = 30      # on expiry day, flatten this long before close

    def on_bar(self, ctx):
        sym = self.symbols[0]
        # manage open positions: re-entry into prior-day range => exit
        st = ctx.state
        reg = st.setdefault("pos", {})
        for p in ctx.positions:           # bind the queued entry's gap info to its position id
            if p.id not in reg and "pending_dir" in st:
                reg[p.id] = st.pop("pending_dir")
        if st.get("day") != ctx.today:
            st["day"] = ctx.today
            st["traded"] = False
        if self.EXIT_RANGE:
            for p in ctx.positions:
                info = reg.get(p.id)
                if info is None:
                    continue
                d, lvl = info
                s = ctx.spot(sym)
                if (d > 0 and s <= lvl) or (d < 0 and s >= lvl):
                    ctx.close(p, "back_in_range")
        if st["traded"] or ctx.positions or ctx.pending_entries:
            return
        tb = ctx.today_bars(sym)
        if len(tb) != 1:          # decide only at the close of the first hourly bar
            return
        dly = ctx.daily(sym)
        if len(dly) < 1:
            return
        prev = dly.iloc[-1]
        op, cl = float(tb["open"].iat[0]), float(tb["close"].iat[0])
        gap = op - float(prev["close"])
        daily_sd = ctx.vol(sym).atm / math.sqrt(252)
        z = abs(gap) / float(prev["close"]) / daily_sd
        if not (self.MIN_GAP_Z <= z <= self.MAX_GAP_Z):
            return
        d = 1 if gap > 0 else -1
        # hold test: still beyond prior close by HOLD_FRAC of gap, and not beyond-fill into prior range
        retained = (cl - float(prev["close"])) * d
        if retained < self.HOLD_FRAC * abs(gap):
            return
        if self.CLOSE_CONFIRM and (cl - op) * d < 0:
            return
        # no fill: first hour must not have traded back through prior close
        if d > 0 and float(tb["low"].iat[0]) <= float(prev["close"]):
            return
        if d < 0 and float(tb["high"].iat[0]) >= float(prev["close"]):
            return
        exp = ctx.expiry(self.DTE)
        cp = "C" if d > 0 else "P"
        k = ctx.strike_for_delta(sym, cp, self.LONG_DELTA, exp)
        short = k + self.WIDTH * d
        ctx.open(vertical(sym, exp, cp, k, short),
                 ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS,
                           minutes_before_close=self.EXIT_MINUTES),
                 tag=f"gap{'up' if d > 0 else 'dn'}_z{z:.1f}")
        st["traded"] = True
        st["pending_dir"] = (d, float(prev["high"]) if d > 0 else float(prev["low"]))


STRATEGY = GapAndGo
