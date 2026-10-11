"""s21 — Bollinger Band fade: bet against closes outside the 20-day, 2-sigma bands.

Hypothesis (user request, 2026-10-11): a close above the upper band is stretched, so
lean bearish for a week or two — sell a bear call spread above the market, or buy a put
debit spread. Mirror: a close below the lower band leans bullish (bull put spread or call
debit spread). Low time in market: SPY closes outside a band on ~4% of days.

What the underlying says (SPY/QQQ 2005-2026, first close outside the band):
- above upper band: SPY is down only 44-50% of the time over 1-10 days (all days: 38-45%),
  mean ~0; QQQ keeps rising (momentum). There is little "reversion" to buy puts on.
- but SPY rarely runs much further: 10-day max rally median +0.9%, >2% only 18% of the
  time. That favors SELLING a call spread above the market (high win rate, small wins).
- below lower band: SPY up 61-64% over 5-10 days, mean +0.6-0.8% — the stronger side.

Rules
  signal   first close outside the BB(20, 2) band (UPPER / LOWER sides switchable)
  entry    next open; one position per side at a time
  bear_call   sell call at ~SHORT_DELTA, buy call WIDTH higher        (upper band)
  put_debit   buy put at ~DEBIT_DELTA, sell put WIDTH lower           (upper band)
  bull_put    sell put at ~SHORT_DELTA, buy put WIDTH lower           (lower band)
  call_debit  buy call at ~DEBIT_DELTA, sell call WIDTH higher        (lower band)
  width    the widest of WIDTHS whose worst-case loss fits ctx.risk_budget (1 unit);
           at $60/trade most credit spreads cannot fit — run with --max-risk 250
  exits    credit: take profit at TP_CREDIT of the credit, stop at SL_CREDIT x credit,
           else hold to expiry.  debit: close when price reaches the 20-day average,
           take profit TP_DEBIT, stop SL_DEBIT, else expiry.
"""
from __future__ import annotations

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import MULTIPLIER, vertical


class BollingerFade(Strategy):
    name = "s21_bollinger_fade"
    description = "Fade closes outside the 20-day 2-sigma Bollinger Bands with short-dated defined-risk spreads"
    symbols = ("SPY",)
    interval = "1d"
    start = "2005-01-01"
    warmup_bars = 25

    BB_LEN = 20
    BB_K = 2.0
    UPPER = "bear_call"      # "bear_call", "put_debit" or None
    LOWER = "bull_put"       # "bull_put", "call_debit" or None
    DTE = 10                 # sessions to expiry at entry
    SHORT_DELTA = 0.25       # short strike of credit spreads
    DEBIT_DELTA = 0.45       # long strike of debit spreads
    WIDTHS = (5.0, 4.0, 3.0, 2.0, 1.0)
    MIN_CREDIT_FRAC = 0.15   # skip credit spreads paying < 15% of width
    TP_CREDIT = 0.5
    SL_CREDIT = None         # chosen in-sample: a 2x-credit stop cut PF 1.84 -> 1.63
    TP_DEBIT = 1.0
    SL_DEBIT = 0.6

    def _bands(self, ctx, sym):
        c = ctx.bars(sym)["close"].to_numpy()
        w = c[-self.BB_LEN - 1:]
        mid, sd = w[1:].mean(), w[1:].std(ddof=1)
        pmid, psd = w[:-1].mean(), w[:-1].std(ddof=1)
        return c[-1], c[-2], mid, mid + self.BB_K * sd, mid - self.BB_K * sd, pmid + self.BB_K * psd, pmid - self.BB_K * psd

    def _fit_width(self, ctx, build):
        """Widest spread whose worst case (incl. round-trip commission) fits the budget."""
        budget = ctx.risk_budget * 0.95  # room for the next-open fill to move
        fees = 4 * ctx.commission_per_contract
        for w in self.WIDTHS:
            s = build(w)
            px = ctx.price(s)["entry"]
            if (s.max_loss(px) * MULTIPLIER + fees) <= budget:
                return s, px, w
        return None, None, None

    def _open_side(self, ctx, sym, side, kind):
        exp = ctx.expiry(self.DTE + 1)
        if kind in ("bear_call", "bull_put"):
            cp = "C" if kind == "bear_call" else "P"
            k = ctx.strike_for_delta(sym, cp, self.SHORT_DELTA, exp)
            step = 1 if cp == "C" else -1
            s, px, w = self._fit_width(ctx, lambda w: vertical(sym, exp, cp, k + step * w, k))
            if s is None or -px < self.MIN_CREDIT_FRAC * w:
                return
            exits = ExitRules(take_profit=self.TP_CREDIT, stop_loss=self.SL_CREDIT)
        else:
            cp = "P" if kind == "put_debit" else "C"
            k = ctx.strike_for_delta(sym, cp, self.DEBIT_DELTA, exp)
            step = -1 if cp == "P" else 1
            s, px, w = self._fit_width(ctx, lambda w: vertical(sym, exp, cp, k, k + step * w))
            if s is None:
                return
            exits = ExitRules(take_profit=self.TP_DEBIT, stop_loss=self.SL_DEBIT)
        ctx.open(s, exits, tag=f"{side}:{kind}", max_units=1)

    def on_bar(self, ctx):
        for sym in self.symbols:
            c, pc, mid, up, lo, pup, plo = self._bands(ctx, sym)
            mine = [p for p in ctx.positions if p.structure.symbol == sym]
            # debit trades: the thesis is "back to the average" — take it when it happens
            for p in mine:
                if p.tag.endswith("put_debit") and c <= mid:
                    ctx.close(p, "reached_average")
                elif p.tag.endswith("call_debit") and c >= mid:
                    ctx.close(p, "reached_average")
            sides = {p.tag.split(":")[0] for p in ctx.positions if p.structure.symbol == sym}
            if self.UPPER and c > up and not pc > pup and "upper" not in sides:
                self._open_side(ctx, sym, "upper", self.UPPER)
            if self.LOWER and c < lo and not pc < plo and "lower" not in sides:
                self._open_side(ctx, sym, "lower", self.LOWER)


STRATEGY = BollingerFade
