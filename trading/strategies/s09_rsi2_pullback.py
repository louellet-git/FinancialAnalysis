"""s09 — Connors-style RSI(2) pullback in an uptrend, expressed with short-dated spreads.

Signal (daily, per symbol, evaluated at the close; no lookahead):
  * trend filter : close > SMA(TREND_LEN)                       (Connors: 200-day)
  * oversold     : RSI(RSI_LEN) < RSI_ENTRY                     (Connors: RSI(2) < 5..10)
Entry: next session's open, one defined-risk bullish spread, DTE sessions to expiry.
  * STRUCTURE = "put_credit": sell the ~SHORT_DELTA put, buy WIDTH lower.
  * STRUCTURE = "call_debit": buy the ~LONG_DELTA call, sell WIDTH higher.
Exit (whichever first):
  * signal  : close > SMA(EXIT_MA_LEN) or RSI > RSI_EXIT        (Connors "close above 5-SMA")
  * time    : MAX_HOLD bars (ExitRules.max_hold_bars), or exit_dte sessions before expiry
  * premium : optional take-profit / stop-loss on the spread (ExitRules)
Signal exits are never taken on the fill day (avoid burning PDT day trades).
"""
from __future__ import annotations

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


def _rsi(close: np.ndarray, n: int) -> float:
    """Wilder RSI of the last value (seeded with a simple mean; converges fast for n=2)."""
    d = np.diff(close)
    if len(d) < n:
        return 50.0
    up, dn = np.clip(d, 0, None), np.clip(-d, 0, None)
    au, ad = up[:n].mean(), dn[:n].mean()
    for u, v in zip(up[n:], dn[n:]):
        au = (au * (n - 1) + u) / n
        ad = (ad * (n - 1) + v) / n
    if ad <= 1e-12:
        return 100.0
    return 100.0 - 100.0 / (1.0 + au / ad)


class RSI2Pullback(Strategy):
    name = "s09_rsi2_pullback"
    description = ("Connors RSI(2)<5 pullback with close>200SMA on SPY/QQQ; 10-session ~0.50-delta $1 call "
                   "debit spread; exit close>5SMA / RSI>65 / 7-bar time stop")
    symbols = ("SPY", "QQQ")
    interval = "1d"
    start = "2004-01-01"
    warmup_bars = 210           # 200-SMA warm-up -> first trades in late 2004 / early 2005

    TREND_LEN = 200
    RSI_LEN = 2
    RSI_ENTRY = 5.0
    RSI_EXIT = 65.0
    EXIT_MA_LEN = 5
    STRUCTURE = "call_debit"    # "put_credit" | "call_debit"
    DTE = 10                    # sessions from the fill to expiry
    SHORT_DELTA = 0.30          # put_credit: |delta| of the short put
    LONG_DELTA = 0.50           # call_debit: delta of the long call
    WIDTH = 1.0
    MAX_HOLD = 7
    EXIT_DTE = 1
    TAKE_PROFIT = None          # fraction of premium (None = off)
    STOP_LOSS = None
    LOOKBACK = 60               # bars of history fed to RSI (Wilder n=2 converges in a few bars)

    def _signals(self, ctx, sym):
        c = ctx.bars(sym)["close"].to_numpy(float)
        if len(c) < self.TREND_LEN + 1 or np.isnan(c[-self.TREND_LEN:]).any():
            return None
        return {
            "close": c[-1],
            "sma_trend": c[-self.TREND_LEN:].mean(),
            "sma_exit": c[-self.EXIT_MA_LEN:].mean(),
            "rsi": _rsi(c[-self.LOOKBACK:], self.RSI_LEN),
        }

    def _structure(self, ctx, sym, exp):
        if self.STRUCTURE == "put_credit":
            k = ctx.strike_for_delta(sym, "P", self.SHORT_DELTA, exp)
            return vertical(sym, exp, "P", k - self.WIDTH, k)
        k = ctx.strike_for_delta(sym, "C", self.LONG_DELTA, exp)
        return vertical(sym, exp, "C", k, k + self.WIDTH)

    def on_bar(self, ctx):
        held = {p.structure.symbol: p for p in ctx.positions}
        for sym in self.symbols:
            s = self._signals(ctx, sym)
            if s is None:
                continue
            pos = held.get(sym)
            if pos is not None:
                if pos.entry_time.date() == ctx.today:
                    continue
                if s["close"] > s["sma_exit"] or s["rsi"] > self.RSI_EXIT:
                    ctx.close(pos, "rsi_or_ma_recovery")
                continue
            if s["close"] > s["sma_trend"] and s["rsi"] < self.RSI_ENTRY:
                exp = ctx.expiry(self.DTE + 1)  # +1: the fill happens next session
                ctx.open(
                    self._structure(ctx, sym, exp),
                    ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS,
                              exit_dte=self.EXIT_DTE, max_hold_bars=self.MAX_HOLD),
                    tag=f"rsi{s['rsi']:.0f}",
                )


STRATEGY = RSI2Pullback
