"""s17 — Capitulation bounce ("buy fear") on SPY, daily bars, 5-20 DTE defined risk.

Signal (evaluated at the daily close, filled at the next open):
  1. Drawdown: close is >= DROP_PCT below its DROP_LOOKBACK-day closing high
     (multi-day large decline).
  2. Fear spike: the vol index (VIX for SPY) reached >= VIX_SPIKE x its VIX_MA-day
     average within the last SPIKE_WINDOW sessions ...
  3. (optional, REQUIRE_TURN; OFF in the final config because it hurt in-sample) ... and
     is now turning down: today's VIX close is >= VIX_TURN below its recent
     SPIKE_WINDOW-day high and below yesterday's close (fear starting to unwind).
  Optional confirmation: a "capitulation day" inside the window — range >= RANGE_MULT x
  the 20-day average range with the close in the bottom CLOSE_LOC of the bar.
  Crash filter (ON, the single most important rule): skip unless the 200-day SMA has risen
  over the last 20 sessions (TREND_SLOPE_MIN=0) -> "buy the dip in an uptrend", which
  screens out 2008 / early-2020 / 2022-style crash continuation.

Structure (MODE):
  "call_debit"  — buy ~LONG_DELTA call, sell call WIDTH above; TP/SL on debit.
  "put_credit"  — sell ~SHORT_DELTA put, buy put WIDTH below; collects elevated IV.
Exits: hold to expiry by default (IS tests: stops/targets only added churn); optional
TAKE_PROFIT / STOP_LOSS / EXIT_DTE. One position at a time; COOLDOWN sessions between entries.
"""
from __future__ import annotations

from trading.strategy import ExitRules, Strategy
from trading.structures import round_strike, vertical


class CapitulationBounce(Strategy):
    name = "s17_capitulation_bounce"
    description = ("Buy fear: after a >=3% SPY pullback with a VIX spike over its MA, in a "
                   "rising-200d-SMA regime only, open a bullish 10-DTE $1 call debit spread "
                   "held to expiry (put-credit variant is infeasible under the $60 cap).")
    symbols = ("SPY",)
    interval = "1d"
    start = "2005-01-01"
    warmup_bars = 210

    # --- signal ---
    DROP_LOOKBACK = 10      # sessions for the closing high
    DROP_PCT = 0.03         # min decline from that high
    VIX_MA = 20
    VIX_SPIKE = 1.15        # VIX / its MA must have hit this within SPIKE_WINDOW
    SPIKE_WINDOW = 3
    VIX_TURN = 0.05         # VIX close at least this far below its window high
    REQUIRE_TURN = False    # VIX must have started to fall from its spike high
    REQUIRE_CAPIT_DAY = False
    RANGE_MULT = 1.5
    CLOSE_LOC = 0.30
    TREND_FILTER = True     # skip when the 200d SMA is falling steeply
    TREND_SLOPE_MIN = 0.0  # 20-session % change of the 200d SMA
    COOLDOWN = 5            # sessions after an entry before another may be queued

    # --- structure ---
    MODE = "call_debit"     # or "put_credit"
    DTE = 10                # sessions to expiry at fill
    LONG_DELTA = 0.40       # call_debit long leg
    SHORT_DELTA = 0.45      # put_credit short leg
    WIDTH = 1.0

    # --- exits ---
    TAKE_PROFIT = None      # fraction of premium
    STOP_LOSS = None
    EXIT_DTE = 0

    def _signal(self, ctx) -> bool:
        bars = ctx.bars("SPY")
        vix = ctx.vol_history("SPY")["atm_close"]
        n = max(self.DROP_LOOKBACK, self.VIX_MA + self.SPIKE_WINDOW, 200 + 20) + 1
        if len(bars) < n:
            return False
        close = bars["close"]
        c = float(close.iat[-1])
        hi = float(close.iloc[-self.DROP_LOOKBACK - 1:].max())
        if c > hi * (1 - self.DROP_PCT):
            return False
        ma = vix.rolling(self.VIX_MA).mean()
        ratio = (vix / ma).iloc[-self.SPIKE_WINDOW:]
        if float(ratio.max()) < self.VIX_SPIKE:
            return False
        v, v_prev = float(vix.iat[-1]), float(vix.iat[-2])
        v_hi = float(vix.iloc[-self.SPIKE_WINDOW:].max())
        if self.REQUIRE_TURN and not (v <= v_hi * (1 - self.VIX_TURN) and v < v_prev):
            return False
        if self.REQUIRE_CAPIT_DAY:
            w = bars.iloc[-self.SPIKE_WINDOW:]
            rng = (bars["high"] - bars["low"])
            avg = rng.rolling(20).mean().iloc[-self.SPIKE_WINDOW:]
            loc = (w["close"] - w["low"]) / (w["high"] - w["low"]).where(lambda x: x > 0)
            if not ((rng.iloc[-self.SPIKE_WINDOW:] >= self.RANGE_MULT * avg) & (loc <= self.CLOSE_LOC)).any():
                return False
        if self.TREND_FILTER:
            sma = close.rolling(200).mean()
            if float(sma.iat[-1] / sma.iat[-21] - 1) < self.TREND_SLOPE_MIN:
                return False
        return True

    def on_bar(self, ctx):
        if ctx.positions or ctx.pending_entries:
            return
        last = ctx.state.get("last_entry_i")
        if last is not None and ctx.i - last < self.COOLDOWN:
            return
        if not self._signal(ctx):
            return
        exp = ctx.expiry(self.DTE + 1)  # +1: fill happens next session
        if self.MODE == "call_debit":
            k = ctx.strike_for_delta("SPY", "C", self.LONG_DELTA, exp)
            s = vertical("SPY", exp, "C", k, round_strike(k + self.WIDTH))
        elif self.MODE == "put_credit":
            k = ctx.strike_for_delta("SPY", "P", self.SHORT_DELTA, exp)
            s = vertical("SPY", exp, "P", round_strike(k - self.WIDTH), k)
        else:
            raise ValueError(f"unknown MODE {self.MODE!r}")
        ctx.open(s, ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS,
                              exit_dte=self.EXIT_DTE), tag=self.MODE)
        ctx.state["last_entry_i"] = ctx.i


STRATEGY = CapitulationBounce
