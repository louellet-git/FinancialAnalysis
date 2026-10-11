"""s15 — volatility-compression breakout ("squeeze" / NR7) with short-dated debit spreads.

Hypothesis: volatility clusters and mean-reverts; a period of unusually compressed range
(Bollinger Bands inside Keltner Channels — the Carter "TTM squeeze" — or Crabel's NR7)
tends to be followed by range expansion. If the expansion has directional persistence,
a cheap 1-5 DTE debit spread in the breakout direction, bought while implied vol is
still "compressed", should have positive expectancy.

Signals (hourly bars, per symbol, computed only from ``ctx.bars`` / ``ctx.daily``):
- SQUEEZE: BB(BB_LEN, BB_K) fully inside KC(BB_LEN, KC_MULT * ATR) for at least
  MIN_SQUEEZE_BARS consecutive bars, and the current bar is the first bar OUT of the
  squeeze ("release"). Direction = sign of close vs the squeeze-window midpoint
  (TTM-style momentum: close - mean(Donchian mid, SMA)); with CONFIRM_RANGE the close must
  also be beyond the high/low of the squeeze window.
- NR7 (MODE "nr7"/"either"): yesterday's daily range is the narrowest of the last 7
  sessions; the first hourly close today beyond yesterday's high (low) is the breakout.

Entry: queued at the release bar's close, filled next bar open: a $WIDTH-wide debit
vertical (call spread up / put spread down), long leg at ~LONG_DELTA, expiring DTE
sessions out. One position per symbol; no entries off the session's last bar (avoids
overnight-gap fills) unless ALLOW_LAST_BAR.
Exit: take profit TP x debit, stop SL x debit, MAX_HOLD_BARS hourly bars, and on expiry
day close EXIT_MIN_BEFORE_CLOSE minutes before 16:00 (else settle at intrinsic).
"""
from __future__ import annotations

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


class SqueezeBreakout(Strategy):
    name = "s15_squeeze_breakout"
    description = ("Hourly BB-inside-KC squeeze release (or daily NR7 break) -> 1-5 DTE "
                   "$1 debit vertical in the breakout direction; TP/SL/time exits")
    symbols = ("SPY", "QQQ")
    interval = "1h"
    warmup_bars = 40

    MODE = "squeeze"          # "squeeze" | "nr7" | "either"
    BB_LEN = 20
    BB_K = 2.0
    KC_MULT = 1.5
    MIN_SQUEEZE_BARS = 6
    CONFIRM_RANGE = True       # release close must clear the squeeze-window high/low
    ALLOW_LAST_BAR = False
    DTE = 3                    # sessions to expiry at signal time
    LONG_DELTA = 0.40
    WIDTH = 1.0
    TP = 0.8
    SL = 0.5
    MAX_HOLD_BARS = 14
    EXIT_MIN_BEFORE_CLOSE = 30

    # ------------------------------------------------------------------ signals
    def _squeeze_signal(self, ctx, sym):
        n = self.BB_LEN + self.MIN_SQUEEZE_BARS + 2
        b = ctx.bars(sym).iloc[-n:]
        if len(b) < n:
            return 0
        h, l, c = b["high"].to_numpy(), b["low"].to_numpy(), b["close"].to_numpy()
        prev_c = np.r_[c[0], c[:-1]]
        tr = np.maximum(h - l, np.maximum(abs(h - prev_c), abs(l - prev_c)))
        L = self.BB_LEN
        in_sq = []
        for j in range(L, n + 1):  # window ending at index j-1
            cw = c[j - L:j]
            sma, sd = cw.mean(), cw.std()
            atr = tr[j - L:j].mean()
            in_sq.append(self.BB_K * sd < self.KC_MULT * atr)
        in_sq = np.array(in_sq)
        if in_sq[-1] or not in_sq[-1 - self.MIN_SQUEEZE_BARS:-1].all():
            return 0
        # squeeze window = the MIN_SQUEEZE_BARS bars before the release bar
        wh = h[-1 - self.MIN_SQUEEZE_BARS:-1].max()
        wl = l[-1 - self.MIN_SQUEEZE_BARS:-1].min()
        cw = c[-L:]
        mom = c[-1] - 0.5 * (0.5 * (h[-L:].max() + l[-L:].min()) + cw.mean())
        d = 1 if mom > 0 else -1 if mom < 0 else 0
        if self.CONFIRM_RANGE and not ((d > 0 and c[-1] > wh) or (d < 0 and c[-1] < wl)):
            return 0
        return d

    def _nr7_signal(self, ctx, sym):
        st = ctx.state.setdefault("nr7_done", {})
        if st.get(sym) == ctx.today:
            return 0
        d = ctx.daily(sym).iloc[-7:]
        if len(d) < 7:
            return 0
        rng = (d["high"] - d["low"]).to_numpy()
        if rng[-1] > rng[:-1].min():
            return 0
        c = ctx.spot(sym)
        hi, lo = float(d["high"].iat[-1]), float(d["low"].iat[-1])
        sig = 1 if c > hi else -1 if c < lo else 0
        if sig:
            st[sym] = ctx.today
        return sig

    # ------------------------------------------------------------------ loop
    def on_bar(self, ctx):
        if ctx.is_last_bar_of_day and not self.ALLOW_LAST_BAR:
            return
        held = {p.structure.symbol for p in ctx.positions}
        for sym in self.symbols:
            if sym in held:
                continue
            sig = 0
            if self.MODE in ("squeeze", "either"):
                sig = self._squeeze_signal(ctx, sym)
            if not sig and self.MODE in ("nr7", "either"):
                sig = self._nr7_signal(ctx, sym)
            if not sig:
                continue
            exp = ctx.expiry(self.DTE)
            cp = "C" if sig > 0 else "P"
            k = ctx.strike_for_delta(sym, cp, self.LONG_DELTA, exp)
            short_k = k + self.WIDTH if sig > 0 else k - self.WIDTH
            ctx.open(
                vertical(sym, exp, cp, k, short_k),
                ExitRules(take_profit=self.TP, stop_loss=self.SL, max_hold_bars=self.MAX_HOLD_BARS,
                          minutes_before_close=self.EXIT_MIN_BEFORE_CLOSE),
                tag=f"{self.MODE}_{'up' if sig > 0 else 'dn'}",
            )


STRATEGY = SqueezeBreakout
