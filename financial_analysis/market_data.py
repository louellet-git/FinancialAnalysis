from __future__ import annotations
import json
import logging
from datetime import date
from pathlib import Path

import yfinance as yf

from financial_analysis.schema import EarningsSurprise, PricePoint

logger = logging.getLogger(__name__)


def load_prices(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> list[PricePoint]:
    path = cache_dir / ticker.upper() / "prices.json"

    if not refresh and path.exists():
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
            return [
                PricePoint(
                    date=date.fromisoformat(r["date"]),
                    open=r["open"],
                    high=r["high"],
                    low=r["low"],
                    close=r["close"],
                    volume=int(r["volume"]),
                )
                for r in raw
            ]
        except (json.JSONDecodeError, KeyError):
            logger.warning("Corrupt price cache for %s — re-fetching", ticker)

    t = yf.Ticker(ticker.upper())
    df = t.history(period="max")

    if df.empty:
        logger.warning("yfinance returned empty DataFrame for %s", ticker)
        return []

    records = []
    for ts, row in df.iterrows():
        records.append({
            "date": ts.strftime("%Y-%m-%d"),
            "open": round(float(row["Open"]), 6),
            "high": round(float(row["High"]), 6),
            "low": round(float(row["Low"]), 6),
            "close": round(float(row["Close"]), 6),
            "volume": int(row["Volume"]),
        })

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(records, indent=2), encoding="utf-8")

    return [
        PricePoint(
            date=date.fromisoformat(r["date"]),
            open=r["open"],
            high=r["high"],
            low=r["low"],
            close=r["close"],
            volume=r["volume"],
        )
        for r in records
    ]


def load_earnings_surprises(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> list[EarningsSurprise]:
    """Load historical earnings surprises via yfinance (EPS estimate vs actual)."""
    path = cache_dir / ticker.upper() / "yf_earnings.json"

    if not refresh and path.exists():
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
            return _parse_yf_earnings(raw)
        except (json.JSONDecodeError, KeyError):
            logger.warning("Corrupt yfinance earnings cache for %s — re-fetching", ticker)

    t = yf.Ticker(ticker.upper())
    try:
        df = t.earnings_dates
    except Exception as e:
        logger.warning("yfinance earnings_dates failed for %s: %s", ticker, e)
        return []

    if df is None or df.empty:
        return []

    records = []
    for ts, row in df.iterrows():
        estimated = row.get("EPS Estimate")
        actual = row.get("Reported EPS")
        if estimated is None or actual is None:
            continue
        try:
            estimated = float(estimated)
            actual = float(actual)
        except (TypeError, ValueError):
            continue
        surprise = actual - estimated
        surprise_pct = surprise / abs(estimated) if estimated != 0 else 0.0
        records.append({
            "date": ts.strftime("%Y-%m-%d"),
            "actual_eps": round(actual, 6),
            "estimated_eps": round(estimated, 6),
            "surprise": round(surprise, 6),
            "surprise_pct": round(surprise_pct, 6),
        })

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(records, indent=2), encoding="utf-8")
    return _parse_yf_earnings(records)


def _parse_yf_earnings(raw: list[dict]) -> list[EarningsSurprise]:
    out = []
    for r in raw:
        try:
            out.append(EarningsSurprise(
                date=date.fromisoformat(r["date"]),
                actual_eps=r["actual_eps"],
                estimated_eps=r["estimated_eps"],
                surprise=r["surprise"],
                surprise_pct=r["surprise_pct"],
            ))
        except (KeyError, ValueError) as exc:
            logger.warning("Skipping malformed yfinance earnings record: %s", exc)
    return out
