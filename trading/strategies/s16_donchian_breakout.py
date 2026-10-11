"""s16 — Turtle-style Donchian channel breakout, expressed as debit spreads.

Signal (daily bars, per symbol, evaluated at the close; orders fill next open):
  * LONG trend starts when the close exceeds the highest high of the prior ENTRY_N
    sessions; SHORT trend when the close breaks the lowest low of the prior ENTRY_N.
  * N = Wilder ATR(ATR_N). The trend ends when the close crosses the opposite
    EXIT_N-session Donchian channel (Turtle exit) OR moves STOP_ATR * N against the
    breakout close (Turtle 2N stop), or when an opposite breakout fires.

Expression: while a trend is live, hold ONE debit vertical per symbol in the trend
direction (call debit spread for up-trends, put debit spread for down-trends), long
strike at ~LONG_DELTA, DTE sessions to expiry, short strike WIDTH dollars further OTM
(widest of WIDTHS whose max loss fits the per-trade budget). Roll (close + re-open
re-centred at the next open) when the spread is within ROLL_DTE sessions of expiry or
has captured ROLL_CAPTURE of its width. Close everything when the trend ends.
"""
from __future__ import annotations

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import MULTIPLIER, vertical


class DonchianBreakoutSpreads(Strategy):
    name = "s16_donchian_breakout"
    description = ("Turtle Donchian breakout (20d entry / 10d exit / 2N ATR stop) on SPY+QQQ, "
                   "expressed as ~20-session ATM debit verticals in the breakout direction, "
                   "rolled while the trend persists")
    symbols = ("SPY", "QQQ")
    interval = "1d"
    start = "2005-01-01"
    warmup_bars = 60

    ENTRY_N = 20          # Donchian breakout lookback (Turtle System 1)
    EXIT_N = 10           # opposite-channel exit lookback
    ATR_N = 20            # Wilder ATR ("N")
    STOP_ATR = 2.0        # hard stop: 2N from the breakout close
    REGIME_SMA = 0        # >0: longs only above / shorts only below this SMA
    ALLOW_SHORT = True    # trade downside breakouts with put debit spreads
    DTE = 20              # sessions to expiry at entry
    ROLL_DTE = 5          # roll when <= this many sessions remain
    ROLL_CAPTURE = 0.85   # roll when spread liquidation value >= this fraction of width
    OTM_STEPS = (0.0, 0.003, 0.006, 0.010, 0.015, 0.02)  # long strike distance OTM (fraction of spot), ATM first
    WIDTHS = (2.0, 1.0)   # widest that fits the risk budget wins
    BUDGET_FRAC = 0.95    # use at most this fraction of the engine's per-trade budget

    # ------------------------------------------------------------------ helpers
    @staticmethod
    def _atr(h, l, c, n):
        prev = np.r_[c[0], c[:-1]]
        tr = np.maximum(h - l, np.maximum(abs(h - prev), abs(l - prev)))
        atr = tr[:n].mean()
        for x in tr[n:]:
            atr += (x - atr) / n
        return atr

    def _signal(self, ctx, sym):
        b = ctx.bars(sym).iloc[-max(max(self.ENTRY_N, self.EXIT_N) + 3 * self.ATR_N + 2, self.REGIME_SMA + 1):]
        h, l, c = (b[k].to_numpy(float) for k in ("high", "low", "close"))
        close = c[-1]
        return {
            "close": close,
            "hi_entry": h[-self.ENTRY_N - 1:-1].max(),
            "lo_entry": l[-self.ENTRY_N - 1:-1].min(),
            "hi_exit": h[-self.EXIT_N - 1:-1].max(),
            "lo_exit": l[-self.EXIT_N - 1:-1].min(),
            "atr": self._atr(h[-(3 * self.ATR_N + 2):], l[-(3 * self.ATR_N + 2):], c[-(3 * self.ATR_N + 2):], self.ATR_N),
            "sma": c[-self.REGIME_SMA:].mean() if self.REGIME_SMA else None,
        }

    def _spread(self, ctx, sym, direction):
        """Cheapest-first search is avoided (it would pick junk lottery tickets): walk from
        ATM outward and take the first (offset, width) whose max loss fits the budget.
        Strike selection is closed-form (spot * (1 -/+ offset)) -- strike_for_delta's
        40-step bisection made the backtest 4+ minutes."""
        exp = ctx.expiry(self.DTE + 1)  # +1: fill happens next session
        cp = "C" if direction > 0 else "P"
        spot = ctx.spot(sym)
        budget = ctx.equity * 0.03 * self.BUDGET_FRAC
        for off in self.OTM_STEPS:
            k = round(spot * (1 + off) if cp == "C" else spot * (1 - off))
            for w in self.WIDTHS:
                s = vertical(sym, exp, cp, k, k + w if cp == "C" else k - w)
                loss = ctx.price(s)["entry"] * MULTIPLIER + 4 * 0.65
                if loss <= budget:
                    return s
        return None  # nothing affordable: skip

    @staticmethod
    def _width(pos):
        ks = [leg.strike for leg in pos.structure.legs]
        return abs(ks[0] - ks[1])

    # ------------------------------------------------------------------ loop
    def on_start(self, ctx):
        ctx.state["trend"] = {s: {"dir": 0, "stop": None} for s in self.symbols}

    def on_bar(self, ctx):
        for sym in self.symbols:
            self._run_symbol(ctx, sym)

    def _run_symbol(self, ctx, sym):
        st = ctx.state["trend"][sym]
        sig = self._signal(ctx, sym)
        c = sig["close"]

        # 1) update trend state: exits first, then (possibly reversing) entries
        if st["dir"] > 0 and (c < sig["lo_exit"] or c < st["stop"]):
            st["dir"] = 0
        elif st["dir"] < 0 and (c > sig["hi_exit"] or c > st["stop"]):
            st["dir"] = 0
        up_ok = not self.REGIME_SMA or c > sig["sma"]
        dn_ok = not self.REGIME_SMA or c < sig["sma"]
        if c > sig["hi_entry"] and st["dir"] <= 0 and up_ok:
            st["dir"], st["stop"] = 1, c - self.STOP_ATR * sig["atr"]
        elif c < sig["lo_entry"] and st["dir"] >= 0 and self.ALLOW_SHORT and dn_ok:
            st["dir"], st["stop"] = -1, c + self.STOP_ATR * sig["atr"]

        # 2) manage this symbol's open spread
        mine = [p for p in ctx.positions if p.structure.symbol == sym]
        for p in mine:
            pdir = 1 if p.structure.legs[0].cp == "C" else -1
            if pdir != st["dir"]:
                ctx.close(p, "trend_end")
                continue
            dte = int(np.busday_count(ctx.today, p.structure.expiry))
            liq = ctx.price(p.structure)["liquidation"]
            if dte <= self.ROLL_DTE:
                ctx.close(p, "roll_dte")
            elif liq >= self.ROLL_CAPTURE * self._width(p):
                ctx.close(p, "roll_capture")

        # 3) (re-)enter while the trend is live and nothing is held for this symbol
        if st["dir"] == 0:
            return
        if any(p.structure.symbol == sym for p in ctx.positions):
            return  # still held (e.g. close PDT-blocked)
        s = self._spread(ctx, sym, st["dir"])
        if s is not None:
            ctx.open(s, ExitRules(exit_dte=1), tag=f"{sym}{'+' if st['dir'] > 0 else '-'}")


STRATEGY = DonchianBreakoutSpreads
