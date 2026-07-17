"""Forward consensus estimates (revenue, EPS, price targets) via yfinance.

yfinance only exposes the CURRENT consensus — there is no free historical
endpoint. Every pull is therefore persisted as a dated snapshot under
``data/raw/TICKER/estimates/YYYY-MM-DD.json`` (one per calendar day;
re-pulling on the same day overwrites). History accumulates from first use,
mirroring options.py.

Absence is expected (small-caps with no coverage, delisted tickers): each of
the three yfinance surfaces (``revenue_estimate``, ``earnings_estimate``,
``analyst_price_targets``) is fetched independently and degrades to an empty
dict on error or empty data — an empty snapshot is still saved so the absence
itself is dated evidence.
"""
from __future__ import annotations
import json
import logging
import math
from datetime import date
from pathlib import Path
from typing import Dict, List, Optional

import yfinance as yf

from financial_analysis.schema import ConsensusEstimates

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Persistence: one dated snapshot per calendar day
# ---------------------------------------------------------------------------

def _estimates_dir(ticker: str, cache_dir: Path) -> Path:
    return cache_dir / ticker.upper() / "estimates"


def _snapshot_to_dict(snapshot: ConsensusEstimates) -> dict:
    return {
        "ticker": snapshot.ticker,
        "snapshot_date": snapshot.snapshot_date.isoformat(),
        "revenue_estimate": snapshot.revenue_estimate,
        "earnings_estimate": snapshot.earnings_estimate,
        "analyst_price_targets": snapshot.analyst_price_targets,
    }


def _snapshot_from_dict(raw: dict) -> ConsensusEstimates:
    return ConsensusEstimates(
        ticker=raw["ticker"],
        snapshot_date=date.fromisoformat(raw["snapshot_date"]),
        revenue_estimate=raw.get("revenue_estimate", {}),
        earnings_estimate=raw.get("earnings_estimate", {}),
        analyst_price_targets=raw.get("analyst_price_targets", {}),
    )


def save_snapshot(snapshot: ConsensusEstimates, cache_dir: Path) -> Path:
    path = _estimates_dir(snapshot.ticker, cache_dir) / f"{snapshot.snapshot_date.isoformat()}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(_snapshot_to_dict(snapshot), indent=2), encoding="utf-8")
    return path


def list_snapshots(ticker: str, cache_dir: Path) -> List[date]:
    """Return the dates of all persisted snapshots for a ticker, oldest first."""
    directory = _estimates_dir(ticker, cache_dir)
    if not directory.exists():
        return []
    dates = []
    for p in directory.glob("*.json"):
        try:
            dates.append(date.fromisoformat(p.stem))
        except ValueError:
            logger.warning("Ignoring non-snapshot file in %s: %s", directory, p.name)
    return sorted(dates)


def load_snapshot(
    ticker: str,
    cache_dir: Path,
    snapshot_date: Optional[date] = None,
) -> ConsensusEstimates:
    """Load a persisted estimates snapshot. None → most recent snapshot."""
    key = ticker.upper()
    if snapshot_date is None:
        available = list_snapshots(key, cache_dir)
        if not available:
            raise FileNotFoundError(
                f"No estimates snapshots for {key} under {_estimates_dir(key, cache_dir)}"
            )
        snapshot_date = available[-1]

    path = _estimates_dir(key, cache_dir) / f"{snapshot_date.isoformat()}.json"
    if not path.exists():
        raise FileNotFoundError(f"No estimates snapshot for {key} on {snapshot_date} at {path}")
    return _snapshot_from_dict(json.loads(path.read_text(encoding="utf-8")))


def latest(ticker: str, cache_dir: Path) -> Optional[ConsensusEstimates]:
    """Most recent persisted snapshot, or None if the ticker has never been pulled."""
    try:
        return load_snapshot(ticker, cache_dir)
    except FileNotFoundError:
        return None


# ---------------------------------------------------------------------------
# Fetching
# ---------------------------------------------------------------------------

def _clean_float(v) -> Optional[float]:
    if v is None:
        return None
    try:
        f = float(v)
    except (TypeError, ValueError):
        return None
    if math.isnan(f) or math.isinf(f):
        return None
    return round(f, 6)


def _df_to_periods(df) -> Dict[str, Dict[str, Optional[float]]]:
    """yfinance estimate DataFrame → {period: {column: value}}.

    Index is yfinance's forward-period key ("0q", "+1q", "0y", "+1y");
    columns are kept as reported (numberOfAnalysts, avg, low, high, growth, ...).
    """
    if df is None or getattr(df, "empty", True):
        return {}
    out: Dict[str, Dict[str, Optional[float]]] = {}
    for period, row in df.iterrows():
        out[str(period)] = {str(col): _clean_float(val) for col, val in row.items()}
    return out


def fetch_estimates(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> ConsensusEstimates:
    """Pull forward consensus via yfinance and persist a dated snapshot.

    One snapshot per calendar day: a same-day re-run loads from disk unless
    refresh=True, in which case the day's snapshot is overwritten.
    """
    key = ticker.upper()
    today = date.today()

    if not refresh:
        path = _estimates_dir(key, cache_dir) / f"{today.isoformat()}.json"
        if path.exists():
            try:
                return load_snapshot(key, cache_dir, snapshot_date=today)
            except (json.JSONDecodeError, KeyError, ValueError):
                logger.warning("Corrupt estimates snapshot for %s — re-fetching", key)

    t = yf.Ticker(key)

    revenue: Dict[str, Dict[str, Optional[float]]] = {}
    earnings: Dict[str, Dict[str, Optional[float]]] = {}
    targets: Dict[str, Optional[float]] = {}

    try:
        revenue = _df_to_periods(t.revenue_estimate)
    except Exception as e:
        logger.warning("yfinance revenue_estimate failed for %s: %s", key, e)

    try:
        earnings = _df_to_periods(t.earnings_estimate)
    except Exception as e:
        logger.warning("yfinance earnings_estimate failed for %s: %s", key, e)

    try:
        raw_targets = t.analyst_price_targets
        if raw_targets:
            targets = {str(k): _clean_float(v) for k, v in dict(raw_targets).items()}
    except Exception as e:
        logger.warning("yfinance analyst_price_targets failed for %s: %s", key, e)

    if not (revenue or earnings or targets):
        logger.warning("No consensus estimates found for %s — saving empty snapshot", key)

    snapshot = ConsensusEstimates(
        ticker=key,
        snapshot_date=today,
        revenue_estimate=revenue,
        earnings_estimate=earnings,
        analyst_price_targets=targets,
    )
    save_snapshot(snapshot, cache_dir)
    return snapshot
