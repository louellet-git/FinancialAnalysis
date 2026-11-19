"""s08_trend_regime — trend-following regime filter driving rolling vertical spreads.

Regime (computed on completed daily closes, no lookahead):
  UP   : close > SMA(LONG) and SMA(FAST) > SMA(LONG)   (price above 200d, golden-cross state)
  DOWN : close < SMA(LONG) and SMA(FAST) < SMA(LONG)   (price below 200d, death-cross state)
  else : NEUTRAL (mixed signals) -> no new entries
An optional ADX filter (ADX_MIN > 0) additionally requires a trending tape for entries.

Structure (defined risk, sized to the 3%-of-equity cap by the engine):
  UP   -> $WIDTH call debit spread, long leg at ~CALL_DELTA, ~DTE sessions to expiry
  DOWN -> $WIDTH put debit spread at ~PUT_DELTA (or flat when TRADE_DOWN = False)

Exits:
  * roll: dte_exit=ROLL_DTE closes the spread ROLL_DTE sessions before expiry; a fresh
    spread is opened the next session if the regime still holds
  * take profit at TP x debit (spread near max value -> recycle into a new one)
  * stop loss at SL x debit (None = hold to roll; max loss is already capped by width)
  * regime flip: any open spread whose direction disagrees with the current regime
    is closed at the close (UP spread when regime leaves UP, etc.)
"""
from __future__ import annotations

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical


def _adx(high: np.ndarray, low: np.ndarray, close: np.ndarray, n: int) -> float:
    """Wilder ADX of the last bar (simple EMA-style smoothing with alpha=1/n)."""
    if len(close) < 3 * n:
        return float("nan")
    h, l, c = high[-6 * n:], low[-6 * n:], close[-6 * n:]
    up = h[1:] - h[:-1]
    dn = l[:-1] - l[1:]
    pdm = np.where((up > dn) & (up > 0), up, 0.0)
    mdm = np.where((dn > up) & (dn > 0), dn, 0.0)
    tr = np.maximum.reduce([h[1:] - l[1:], np.abs(h[1:] - c[:-1]), np.abs(l[1:] - c[:-1])])
    a = 1.0 / n

    def wilder(x):
        out = np.empty_like(x)
        out[0] = x[:n].mean()
        for k in range(1, len(x)):
            out[k] = out[k - 1] + a * (x[k] - out[k - 1])
        return out

    atr, sp, sm = wilder(tr), wilder(pdm), wilder(mdm)
    pdi = 100 * sp / np.where(atr > 0, atr, np.nan)
    mdi = 100 * sm / np.where(atr > 0, atr, np.nan)
    dx = 100 * np.abs(pdi - mdi) / np.where((pdi + mdi) > 0, pdi + mdi, np.nan)
    dx = np.nan_to_num(dx)
    return float(wilder(dx)[-1])


class TrendRegime(Strategy):
    name = "s08_trend_regime"
    description = ("Trend regime (close vs 200d SMA + 50/200 cross): uptrend -> rolling ~30-session "
                   "$1 call debit spreads; downtrend -> flat (TRADE_DOWN=True: $1 put debit spreads); roll 10 sessions "
                   "before expiry, close on regime flip")
    symbols = ("SPY",)
    interval = "1d"
    start = "2005-01-01"
    warmup_bars = 200

    FAST = 50
    LONG = 200
    ADX_N = 14
    ADX_MIN = 0.0           # 0 disables the ADX filter
    DTE = 30                # sessions to expiry at entry (20-45 DTE band)
    ROLL_DTE = 10           # close this many sessions before expiry, then re-enter
    CALL_DELTA = 0.50
    PUT_DELTA = 0.50
    WIDTH = 1.0
    TP = 0.80               # take profit at +80% of debit
    SL = None               # no premium stop; width caps the loss
    UP_MODE = "CALL_DEBIT"  # "PUT_CREDIT" is infeasible on this account (see research note): a $1-wide
    PUT_SHORT_DELTA = 0.25  # credit spread risks ~$0.75-0.9/share > the ~$57 per-trade budget -> always rejected
    NEUTRAL_EXIT = False    # also close open spreads when the regime goes NEUTRAL (False = only on opposite regime)
    TRADE_DOWN = False      # True = flat in downtrends
    MAX_CONCURRENT = 1      # spreads held at once (staggered entries if > 1)
    STAGGER = 10            # min sessions between entries when MAX_CONCURRENT > 1

    def regime(self, ctx, sym: str) -> str:
        b = ctx.bars(sym)
        c = b["close"].to_numpy(float)
        if len(c) < self.LONG:
            return "NEUTRAL"
        last = c[-1]
        sma_l = c[-self.LONG:].mean()
        sma_f = c[-self.FAST:].mean()
        if self.ADX_MIN > 0:
            adx = _adx(b["high"].to_numpy(float), b["low"].to_numpy(float), c, self.ADX_N)
            if not adx >= self.ADX_MIN:
                # weak trend: allow holding but no new entries; treat as neutral for entry
                trending = False
            else:
                trending = True
        else:
            trending = True
        if last > sma_l and sma_f > sma_l:
            return "UP" if trending else "UP_WEAK"
        if last < sma_l and sma_f < sma_l:
            return "DOWN" if trending else "DOWN_WEAK"
        return "NEUTRAL"

    def on_bar(self, ctx):
        for sym in self.symbols:
            reg = self.regime(ctx, sym)
            hold_dir = {"UP": "C", "UP_WEAK": "C", "DOWN": "P", "DOWN_WEAK": "P"}.get(reg)
            mine = [p for p in ctx.positions if p.structure.symbol == sym]
            # regime flip -> close disagreeing spreads
            for p in mine:
                if hold_dir is None and not self.NEUTRAL_EXIT:
                    continue
                if p.tag != hold_dir:
                    ctx.close(p, "regime_flip")
            mine = [p for p in ctx.positions if p.structure.symbol == sym]
            if reg not in ("UP", "DOWN") or (reg == "DOWN" and not self.TRADE_DOWN):
                continue
            if ctx.pending_entries or len(mine) >= self.MAX_CONCURRENT:
                continue
            last_i = ctx.state.get(("last_entry", sym), -10**9)
            if mine and ctx.i - last_i < self.STAGGER:
                continue
            exp = ctx.expiry(self.DTE + 1)  # fill is next session
            if reg == "UP" and self.UP_MODE == "PUT_CREDIT":
                k = ctx.strike_for_delta(sym, "P", self.PUT_SHORT_DELTA, exp)
                s = vertical(sym, exp, "P", k - self.WIDTH, k)
                tag = "C"  # bullish
            elif reg == "UP":
                k = ctx.strike_for_delta(sym, "C", self.CALL_DELTA, exp)
                s = vertical(sym, exp, "C", k, k + self.WIDTH)
                tag = "C"
            else:
                k = ctx.strike_for_delta(sym, "P", self.PUT_DELTA, exp)
                s = vertical(sym, exp, "P", k, k - self.WIDTH)
                tag = "P"
            ctx.open(s, ExitRules(take_profit=self.TP, stop_loss=self.SL, exit_dte=self.ROLL_DTE), tag=tag)
            ctx.state[("last_entry", sym)] = ctx.i


STRATEGY = TrendRegime
