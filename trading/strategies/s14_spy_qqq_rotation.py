"""s14 — SPY/QQQ relative-strength rotation expressed with narrow call debit spreads.

Hypothesis: (1) relative momentum — the QQQ/SPY ratio trends (growth-vs-broad-market
leadership regimes persist for months: 2009-2011, 2017-2021, 2023-2024 QQQ; 2022 SPY),
so the recent leader tends to keep outperforming over the next few weeks; (2) absolute
(time-series) momentum — only be long when the chosen leader is itself in an uptrend
(Moskowitz/Ooi/Pedersen 2012; Antonacci "dual momentum"; Faber 10-month SMA), which
sidesteps most of the deep bear-market drawdowns (2008, 2022).

Signal (evaluated at each Friday close, entries fill Monday's open):
  ratio  = QQQ.close / SPY.close
  leader = QQQ if ratio > SMA(ratio, RS_SMA) and ratio LOOKBACK-day return > 0
           SPY if ratio < SMA(ratio, RS_SMA) and ratio LOOKBACK-day return < 0
           otherwise keep the previous leader (hysteresis, avoids whipsaw)
  risk-on = leader.close > SMA(leader, TREND_SMA)  and  VIX-proxy IV < MAX_IV
Structure: $WIDTH-wide call debit spread on the leader, long strike at ~LONG_DELTA,
  ~DTE sessions; if the debit (+ round-trip commissions) exceeds the per-trade budget
  the strikes are walked up $1 at a time until it fits.
  Staggered: one new spread per week while risk-on, at most MAX_POSITIONS open.
  MODE == "pair" additionally buys a put debit spread on the laggard (relative-value
  expression); default "leader" (see research note for why).
Exits (final): HOLD TO EXPIRY (settle at intrinsic; max loss = debit, already capped by the
  $60 sizing rule). Optional knobs TAKE_PROFIT / STOP_LOSS / EXIT_DTE / CLOSE_ON_FLIP exist
  but all lose money on $1-wide spreads because round-trip friction (~$9) is ~18% of the debit.
"""
from __future__ import annotations

from trading.strategy import ExitRules, Strategy
from trading.structures import MULTIPLIER, vertical


class SpyQqqRotation(Strategy):
    name = "s14_spy_qqq_rotation"
    description = ("Dual momentum: QQQ/SPY ratio trend picks the leader, leader>SMA200 risk filter; "
                   "weekly staggered ~20-session $1 call debit spreads on the leader")
    symbols = ("SPY", "QQQ")
    interval = "1d"
    warmup_bars = 210
    start = "2005-01-01"

    RS_SMA = 50            # ratio trend filter
    LOOKBACK = 63          # ratio momentum window (~3 months)
    TREND_SMA = 200        # absolute momentum filter on the leader
    MAX_IV = 0.35          # skip new entries when the leader's 30d IV is above this
    DTE = 20               # sessions to expiry at entry
    LONG_DELTA = 0.50
    WIDTH = 1.0
    BUDGET_FRAC = 0.95     # of equity * 3% risk cap (leave room for open-price slippage)
    RISK_PER_TRADE = 0.03  # mirrors the engine's RiskConfig default
    TAKE_PROFIT = None     # variant tested: 0.8 -> account ruin (see research note)
    STOP_LOSS = None
    EXIT_DTE = None        # hold to expiry: settles at intrinsic, no exit commission/spread
    MAX_POSITIONS = 3
    CLOSE_ON_FLIP = False  # closing on signal flips crystallised whipsaw losses (-$1.5k/96 trades)
    MODE = "leader"        # "leader" or "pair"
    FIXED_LEADER = None    # "SPY"/"QQQ" disables rotation (ablation; see s14b)

    def on_start(self, ctx):
        ctx.state["leader"] = None

    # --- signal -------------------------------------------------------------
    def _signal(self, ctx):
        spy, qqq = ctx.bars("SPY")["close"], ctx.bars("QQQ")["close"]
        ratio = qqq / spy
        sma = ratio.iloc[-self.RS_SMA:].mean()
        mom = ratio.iat[-1] / ratio.iat[-1 - self.LOOKBACK] - 1
        r = ratio.iat[-1]
        leader = ctx.state["leader"]
        if r > sma and mom > 0:
            leader = "QQQ"
        elif r < sma and mom < 0:
            leader = "SPY"
        elif leader is None:
            leader = "QQQ" if mom > 0 else "SPY"
        ctx.state["leader"] = leader
        if self.FIXED_LEADER:
            leader = self.FIXED_LEADER
        px = ctx.bars(leader)["close"]
        trend_ok = px.iat[-1] > px.iloc[-self.TREND_SMA:].mean()
        iv_ok = ctx.vol(leader).atm < self.MAX_IV
        return leader, trend_ok, iv_ok

    # --- structure ------------------------------------------------------------
    def _spread(self, ctx, sym, cp, exp):
        budget = ctx.equity * self.RISK_PER_TRADE * self.BUDGET_FRAC
        k = ctx.strike_for_delta(sym, cp, self.LONG_DELTA, exp)
        step = 1.0 if cp == "C" else -1.0
        for _ in range(8):
            s = vertical(sym, exp, cp, k, k + step * self.WIDTH)
            debit = ctx.price(s)["entry"]
            if debit > 0 and debit * MULTIPLIER + 4 * 0.65 <= budget:
                return s
            k += step  # move further OTM -> cheaper
        return None

    def on_bar(self, ctx):
        if len(ctx.bars("SPY")) < max(self.TREND_SMA, self.LOOKBACK + 1, self.RS_SMA):
            return
        leader, trend_ok, iv_ok = self._signal(ctx)
        laggard = "SPY" if leader == "QQQ" else "QQQ"

        if self.CLOSE_ON_FLIP:
            for p in ctx.positions:
                is_call = p.structure.legs[0].cp == "C"
                wrong = (is_call and (p.structure.symbol != leader or not trend_ok)) or \
                        (not is_call and p.structure.symbol != laggard)
                if wrong:
                    ctx.close(p, "signal_flip")

        if ctx.today.weekday() != 4 or ctx.pending_entries:
            return
        if not (trend_ok and iv_ok):
            return
        per_trade = 2 if self.MODE == "pair" else 1
        if len(ctx.positions) + per_trade > self.MAX_POSITIONS:
            return
        exp = ctx.expiry(self.DTE + 1)
        exits = ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS, exit_dte=self.EXIT_DTE)
        s = self._spread(ctx, leader, "C", exp)
        if s is None:
            return
        ctx.open(s, exits, tag=f"bull_{leader}")
        if self.MODE == "pair":
            p = self._spread(ctx, laggard, "P", exp)
            if p is not None:
                ctx.open(p, exits, tag=f"bear_{laggard}")


STRATEGY = SpyQqqRotation
