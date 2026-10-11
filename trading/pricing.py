"""Option pricing. The default ``SyntheticPricer`` is a MODEL, not market quotes.

Every backtest number from this harness is model-priced:
- ATM vol from the vol index (VIX for SPY, VXN for QQQ), VIX1D for <= ~1 trading day,
  linear-in-T blend between them out to 30 trading days.
- Skew: IV multiplier quadratic in standardized log-moneyness, puts richer than calls.
- Spread: half-spread = clip(0.5% of mid, $0.01, $0.05) per leg (SPY/QQQ near-the-money
  quotes are typically 1-10 cents wide); long legs pay ask, short
  legs receive bid. Commission is charged by the engine.
- European Black-Scholes, settled at intrinsic on expiry (SPY/QQQ options are American
  and physically settled; early assignment and pin risk are ignored).

Swap in a real-quote pricer by implementing ``OptionPricer.quote`` (``LiveChainPricer``
does this for the current chain only).
"""
from __future__ import annotations

import math
from dataclasses import dataclass
from datetime import date, timedelta
from typing import Optional, Protocol

import numpy as np
import pandas as pd

from trading.data import SESSION_MINUTES

TRADING_DAYS = 252
MINUTES_PER_YEAR = TRADING_DAYS * SESSION_MINUTES
_SQRT2 = math.sqrt(2.0)
_ONE_DAY = timedelta(days=1)


def norm_cdf(x: float) -> float:
    return 0.5 * (1.0 + math.erf(x / _SQRT2))


def intrinsic(cp: str, spot: float, strike: float) -> float:
    return max(spot - strike, 0.0) if cp == "C" else max(strike - spot, 0.0)


def bs_price(cp: str, spot: float, strike: float, t: float, rate: float, vol: float) -> float:
    if t <= 0 or vol <= 0:
        return intrinsic(cp, spot, strike)
    sq = vol * math.sqrt(t)
    d1 = (math.log(spot / strike) + (rate + 0.5 * vol * vol) * t) / sq
    d2 = d1 - sq
    disc = math.exp(-rate * t)
    if cp == "C":
        return spot * norm_cdf(d1) - strike * disc * norm_cdf(d2)
    return strike * disc * norm_cdf(-d2) - spot * norm_cdf(-d1)


def bs_delta(cp: str, spot: float, strike: float, t: float, rate: float, vol: float) -> float:
    if t <= 0 or vol <= 0:
        itm = spot > strike if cp == "C" else spot < strike
        return (1.0 if cp == "C" else -1.0) if itm else 0.0
    d1 = (math.log(spot / strike) + (rate + 0.5 * vol * vol) * t) / (vol * math.sqrt(t))
    return norm_cdf(d1) if cp == "C" else norm_cdf(d1) - 1.0


def year_fraction(now: pd.Timestamp, expiry: date) -> float:
    """Trading-time years from ``now`` to the 16:00 close on ``expiry`` (weekends skipped)."""
    today = now.date()
    if expiry < today:
        return 0.0
    close_today = now.normalize() + pd.Timedelta(hours=16)
    mins_today = min(max((close_today - now).total_seconds() / 60.0, 0.0), SESSION_MINUTES)
    if expiry == today:
        return mins_today / MINUTES_PER_YEAR
    sessions_after = int(np.busday_count(today + _ONE_DAY, expiry + _ONE_DAY))
    return (mins_today + sessions_after * SESSION_MINUTES) / MINUTES_PER_YEAR


@dataclass(frozen=True)
class VolSnapshot:
    atm: float    # 30-day ATM IV (decimal)
    short: float  # ~1-day ATM IV (decimal)
    rate: float


@dataclass(frozen=True)
class Quote:
    bid: float
    ask: float
    mid: float
    iv: float
    delta: float


class OptionPricer(Protocol):
    def quote(self, symbol: str, cp: str, strike: float, expiry: date,
              now: pd.Timestamp, spot: float, vol: VolSnapshot) -> Quote: ...


@dataclass
class SyntheticPricer:
    skew_slope: float = -0.12     # IV change per unit standardized moneyness (puts richer)
    skew_curve: float = 0.03      # smile curvature
    mult_floor: float = 0.6
    mult_cap: float = 2.5
    spread_rel: float = 0.005
    spread_min: float = 0.01
    spread_max: float = 0.05

    def atm_vol(self, t: float, vol: VolSnapshot) -> float:
        t1, t30 = 1.0 / TRADING_DAYS, 30.0 / TRADING_DAYS
        if t <= t1:
            return vol.short
        if t >= t30:
            return vol.atm
        w = (t - t1) / (t30 - t1)
        return (1 - w) * vol.short + w * vol.atm

    def implied_vol(self, spot: float, strike: float, t: float, vol: VolSnapshot) -> float:
        atm = max(self.atm_vol(t, vol), 0.01)
        tt = max(t, 1.0 / MINUTES_PER_YEAR)
        z = math.log(strike / spot) / (atm * math.sqrt(tt))
        z = min(max(z, -6.0), 6.0)
        mult = 1.0 + self.skew_slope * z + self.skew_curve * z * z
        return atm * min(max(mult, self.mult_floor), self.mult_cap)

    def quote(self, symbol, cp, strike, expiry, now, spot, vol) -> Quote:
        t = year_fraction(now, expiry)
        if t <= 0:
            px = intrinsic(cp, spot, strike)
            return Quote(px, px, px, 0.0, bs_delta(cp, spot, strike, 0.0, 0.0, 0.0))
        iv = self.implied_vol(spot, strike, t, vol)
        mid = bs_price(cp, spot, strike, t, vol.rate, iv)
        half = min(max(self.spread_rel * mid, self.spread_min), self.spread_max)
        bid = max(mid - half, 0.0)
        return Quote(bid, mid + half, mid, iv, bs_delta(cp, spot, strike, t, vol.rate, iv))


class LiveChainPricer:
    """Real bid/ask from today's yfinance chain; falls back to synthetic when missing.

    Only meaningful for ``now`` ~ the present (the free chain is current-only).
    """

    def __init__(self, fallback: Optional[SyntheticPricer] = None):
        self.fallback = fallback or SyntheticPricer()
        self._chains: dict = {}

    def _chain(self, symbol: str, expiry: date):
        key = (symbol, expiry)
        if key not in self._chains:
            import yfinance as yf

            try:
                ch = yf.Ticker(symbol).option_chain(expiry.isoformat())
                self._chains[key] = {"C": ch.calls.set_index("strike"), "P": ch.puts.set_index("strike")}
            except Exception:
                self._chains[key] = None
        return self._chains[key]

    def quote(self, symbol, cp, strike, expiry, now, spot, vol) -> Quote:
        synth = self.fallback.quote(symbol, cp, strike, expiry, now, spot, vol)
        chain = self._chain(symbol, expiry)
        if chain is None or strike not in chain[cp].index:
            return synth
        row = chain[cp].loc[strike]
        bid, ask = float(row.get("bid", np.nan)), float(row.get("ask", np.nan))
        if not (bid >= 0 and ask > 0 and ask >= bid):
            return synth
        iv = float(row.get("impliedVolatility", synth.iv) or synth.iv)
        return Quote(bid, ask, 0.5 * (bid + ask), iv, synth.delta)
