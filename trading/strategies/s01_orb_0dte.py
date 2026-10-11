"""s01 - Opening-range breakout (ORB) on 0DTE debit spreads.

Opening range = first OR_BARS bars of the session (5m: 6 bars = 30 min; 1h: 1 bar = first
hour). On the first bar close beyond the range by BUFFER (x range width) before ENTRY_CUTOFF
minutes-to-close, buy a narrow 0DTE debit spread in the breakout direction. Exit: profit
target / stop (ExitRules), range re-entry (close back inside the range), time exit before
the close. One trade per day; self-limits to PDT day-trade budget.
"""
from __future__ import annotations

from trading.strategy import ExitRules, Strategy
from trading.structures import round_strike, vertical


class OrbZeroDte(Strategy):
    name = "s01_orb_0dte"
    description = "Opening-range breakout -> 0DTE call/put debit spread, TP / range re-entry / time exit"
    symbols = ("SPY",)
    interval = "5m"
    warmup_bars = 0
    start = None

    OR_BARS = 6              # 5m: 6 = 30 min ; for 1h set 1
    BUFFER = 0.25            # break must exceed range by BUFFER * range width
    MIN_RANGE_PCT = 0.0      # skip tiny ranges (fraction of spot)
    MAX_RANGE_PCT = 0.02     # skip wide, already-exhausted ranges
    LONG_OFFSET = 0.0        # long strike offset from spot, in $ (+ = OTM in trade direction)
    WIDTH = 1.0
    TAKE_PROFIT = 1.0
    STOP_LOSS = 0.5
    REENTRY_STOP = True
    EXIT_MINUTES_BEFORE_CLOSE = 20
    ENTRY_CUTOFF_MIN = 120   # no new entries with less than this many minutes to close
    MAX_DAY_TRADES_5D = 3

    def on_start(self, ctx):
        ctx.state["entries"] = []

    def on_bar(self, ctx):
        sym = self.symbols[0]
        tb = ctx.today_bars(sym)
        n = len(tb)
        st = ctx.state

        # re-entry stop on open positions (same-day only; 0DTE)
        for pos in ctx.positions:
            lo_hi = st.get("range")
            if self.REENTRY_STOP and lo_hi and pos.tag in ("orb_C", "orb_P"):
                lo, hi = lo_hi
                c = ctx.spot(sym)
                if (pos.tag == "orb_C" and c < hi) or (pos.tag == "orb_P" and c > lo):
                    ctx.close(pos, "range_reentry")
        if n < self.OR_BARS:
            return
        if n == self.OR_BARS:
            st["range"] = (float(tb["low"].min()), float(tb["high"].max()))
            st["day"] = ctx.today
            return
        if st.get("day") != ctx.today or ctx.positions or ctx.pending_entries or st.get("traded") == ctx.today:
            return
        if ctx.minutes_to_close < self.ENTRY_CUTOFF_MIN:
            return
        lo, hi = st["range"]
        spot = ctx.spot(sym)
        width = hi - lo
        if not (self.MIN_RANGE_PCT * spot <= width <= self.MAX_RANGE_PCT * spot):
            return
        recent = [d for d in st["entries"] if (ctx.today - d).days < 7]
        if len(recent) >= self.MAX_DAY_TRADES_5D:
            return
        buf = self.BUFFER * width
        if spot > hi + buf:
            cp, k = "C", round_strike(spot + self.LONG_OFFSET)
            s = vertical(sym, ctx.expiry(0), "C", k, k + self.WIDTH)
        elif spot < lo - buf:
            cp, k = "P", round_strike(spot - self.LONG_OFFSET)
            s = vertical(sym, ctx.expiry(0), "P", k, k - self.WIDTH)
        else:
            return
        st["traded"] = ctx.today
        st["entries"] = recent + [ctx.today]
        ctx.open(s, ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS,
                              minutes_before_close=self.EXIT_MINUTES_BEFORE_CLOSE), tag=f"orb_{cp}")


STRATEGY = OrbZeroDte
