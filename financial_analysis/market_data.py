from __future__ import annotations
import json
import logging
from datetime import date
from pathlib import Path

import yfinance as yf

from financial_analysis.schema import PricePoint

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
