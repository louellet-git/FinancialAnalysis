"""Bars, volatility indices and short rates for SPY/QQQ.

Free-data reality (checked 2026-10-11 via yfinance):
- daily bars back to 1993 (SPY) / 1999 (QQQ)
- hourly bars ~730 days, 5-minute bars ~60 days
- ^VIX (SPX 30d IV) since 1990, ^VXN (NDX 30d IV) since 2001, ^VIX1D (SPX 1-day IV) since 2023
- NO historical option quotes

The cache merges every refresh into the stored CSV, so intraday history grows past the
60-day window the longer the paper trader runs. Cache dir: ``$TRADING_DATA_DIR`` or
``data/raw/trading/`` (gitignored).
"""
from __future__ import annotations

import logging
import os
from dataclasses import dataclass, field
from datetime import datetime, time, timedelta
from pathlib import Path
from typing import Dict, Optional

import numpy as np
import pandas as pd

logger = logging.getLogger(__name__)

NY = "America/New_York"
MARKET_OPEN = time(9, 30)
MARKET_CLOSE = time(16, 0)
SESSION_MINUTES = 390

INTERVALS = ("1d", "1h", "5m")
INTERVAL_PERIOD = {"1d": "max", "1h": "730d", "5m": "60d"}
INTERVAL_DELTA = {"1h": timedelta(hours=1), "5m": timedelta(minutes=5)}

SYMBOLS = ("SPY", "QQQ")
VOL_INDEX = {"SPY": "^VIX", "QQQ": "^VXN"}
SHORT_VOL_INDEX = "^VIX1D"
RATE_INDEX = "^IRX"
DEFAULT_RATE = 0.04

_REPO_ROOT = Path(__file__).resolve().parent.parent


def data_dir() -> Path:
    d = Path(os.environ.get("TRADING_DATA_DIR", _REPO_ROOT / "data" / "raw" / "trading"))
    d.mkdir(parents=True, exist_ok=True)
    return d


def _cache_path(symbol: str, interval: str) -> Path:
    return data_dir() / f"{symbol.replace('^', '_')}_{interval}.csv"


def _normalize(df: pd.DataFrame, interval: str) -> pd.DataFrame:
    df = df.rename(columns=str.lower)[["open", "high", "low", "close", "volume"]].copy()
    idx = pd.DatetimeIndex(df.index)
    idx = idx.tz_localize(NY) if idx.tz is None else idx.tz_convert(NY)
    if interval == "1d":
        idx = idx.normalize()
    df.index = idx
    df = df[~df.index.duplicated(keep="last")].sort_index()
    df = df.dropna(subset=["open", "high", "low", "close"])
    if interval != "1d":  # regular session only
        t = df.index.time
        df = df[(t >= MARKET_OPEN) & (t < MARKET_CLOSE)]
    return df


def fetch_bars(symbol: str, interval: str) -> pd.DataFrame:
    import yfinance as yf

    raw = yf.Ticker(symbol).history(
        period=INTERVAL_PERIOD[interval], interval=interval, auto_adjust=False, actions=False
    )
    if raw is None or raw.empty:
        return pd.DataFrame(columns=["open", "high", "low", "close", "volume"])
    return _normalize(raw, interval)


def _read_cache(path: Path) -> Optional[pd.DataFrame]:
    if not path.exists():
        return None
    df = pd.read_csv(path, index_col=0)
    df.index = pd.to_datetime(df.index, utc=True).tz_convert(NY)
    return df


def load_bars(symbol: str, interval: str, refresh: bool = False) -> pd.DataFrame:
    """Cached bars; ``refresh`` re-downloads and MERGES into the cache (never shrinks it)."""
    if interval not in INTERVALS:
        raise ValueError(f"interval must be one of {INTERVALS}, got {interval!r}")
    path = _cache_path(symbol, interval)
    cached = _read_cache(path)
    if cached is not None and not refresh:
        return cached
    try:
        fresh = fetch_bars(symbol, interval)
    except Exception as exc:  # network failure: fall back to cache if we have one
        if cached is not None:
            logger.warning("refresh of %s %s failed (%s); using cache", symbol, interval, exc)
            return cached
        raise
    merged = fresh if cached is None else pd.concat([cached, fresh])
    merged = merged[~merged.index.duplicated(keep="last")].sort_index()
    merged.to_csv(path)
    return merged


# ---------------------------------------------------------------------------
# Bar timing
# ---------------------------------------------------------------------------

def bar_open_time(ts: pd.Timestamp, interval: str) -> pd.Timestamp:
    if interval == "1d":
        return ts.normalize() + pd.Timedelta(hours=9, minutes=30)
    return ts


def bar_close_time(ts: pd.Timestamp, interval: str) -> pd.Timestamp:
    session_close = ts.normalize() + pd.Timedelta(hours=16)
    if interval == "1d":
        return session_close
    return min(ts + INTERVAL_DELTA[interval], session_close)


# ---------------------------------------------------------------------------
# Aligned market data bundle
# ---------------------------------------------------------------------------

@dataclass
class MarketData:
    """Everything a backtest needs, aligned on one bar index.

    ``vol[sym]`` columns (decimals, e.g. 0.18): atm_open, atm_close (30-day index),
    short_open, short_close (1-day IV; falls back to the 30-day value before VIX1D exists).
    ``rate``: annualized short rate per bar (prior-day ^IRX, so no lookahead).
    ``daily[sym]``: full daily bars, for intraday strategies that need prior-day context.
    """

    interval: str
    bars: Dict[str, pd.DataFrame]
    vol: Dict[str, pd.DataFrame]
    rate: pd.Series
    daily: Dict[str, pd.DataFrame] = field(default_factory=dict)

    @property
    def index(self) -> pd.DatetimeIndex:
        return next(iter(self.bars.values())).index


def _daily_prev_close(daily: pd.DataFrame, index: pd.DatetimeIndex) -> pd.Series:
    """For each bar, the PREVIOUS session's daily close (no lookahead)."""
    prev = daily["close"].shift(1)
    prev.index = prev.index.date
    return pd.Series(pd.Index(index.date).map(prev).astype(float), index=index)


def _vol_frame(index: pd.DatetimeIndex, interval: str, vol_sym: str, refresh: bool) -> pd.DataFrame:
    """open/close vol for each bar; intraday falls back to prior daily close when missing."""
    daily = load_bars(vol_sym, "1d", refresh=refresh)
    if interval == "1d":
        src = daily.reindex(index)
    else:
        try:
            intr = load_bars(vol_sym, interval, refresh=refresh)
        except Exception:
            intr = pd.DataFrame(columns=["open", "close"])
        src = intr.reindex(index).ffill(limit=12) if len(intr) else pd.DataFrame(index=index, columns=["open", "close"])
    fallback = _daily_prev_close(daily, index)
    out = pd.DataFrame(index=index)
    out["open"] = src["open"].astype(float).fillna(fallback)
    out["close"] = src["close"].astype(float).fillna(fallback)
    return out / 100.0


def load_market(
    symbols=SYMBOLS,
    interval: str = "1d",
    start: Optional[str] = None,
    end: Optional[str] = None,
    refresh: bool = False,
) -> MarketData:
    bars = {s: load_bars(s, interval, refresh=refresh) for s in symbols}
    index = None
    for df in bars.values():
        index = df.index if index is None else index.intersection(df.index)
    if start:
        index = index[index >= pd.Timestamp(start, tz=NY)]
    if end:
        index = index[index <= pd.Timestamp(end, tz=NY) + pd.Timedelta(days=1)]
    bars = {s: df.reindex(index) for s, df in bars.items()}

    short = _vol_frame(index, interval, SHORT_VOL_INDEX, refresh)
    vix = _vol_frame(index, interval, VOL_INDEX["SPY"], refresh)
    vol = {}
    for s in symbols:
        atm = _vol_frame(index, interval, VOL_INDEX.get(s, "^VIX"), refresh)
        # QQQ has no 1-day index: scale VIX1D by the VXN/VIX ratio.
        ratio = (atm / vix).where(vix > 0, 1.0) if s != "SPY" else 1.0
        sh = short * ratio
        frame = pd.DataFrame(index=index)
        frame["atm_open"], frame["atm_close"] = atm["open"], atm["close"]
        frame["short_open"] = sh["open"].fillna(atm["open"])
        frame["short_close"] = sh["close"].fillna(atm["close"])
        vol[s] = frame.ffill().bfill()

    try:
        irx = load_bars(RATE_INDEX, "1d", refresh=refresh)
        rate = (_daily_prev_close(irx, index) / 100.0).ffill().fillna(DEFAULT_RATE)
    except Exception:
        rate = pd.Series(DEFAULT_RATE, index=index)

    daily = bars if interval == "1d" else {s: load_bars(s, "1d", refresh=refresh) for s in symbols}
    return MarketData(interval=interval, bars=bars, vol=vol, rate=rate, daily=daily)


def prefetch(refresh: bool = True) -> Dict[str, int]:
    """Download/merge everything the harness uses; returns row counts."""
    counts = {}
    tickers = list(SYMBOLS) + list(VOL_INDEX.values()) + [SHORT_VOL_INDEX]
    for t in tickers:
        for iv in INTERVALS:
            try:
                counts[f"{t} {iv}"] = len(load_bars(t, iv, refresh=refresh))
            except Exception as exc:
                logger.warning("prefetch %s %s failed: %s", t, iv, exc)
                counts[f"{t} {iv}"] = 0
    counts[f"{RATE_INDEX} 1d"] = len(load_bars(RATE_INDEX, "1d", refresh=refresh))
    return counts
