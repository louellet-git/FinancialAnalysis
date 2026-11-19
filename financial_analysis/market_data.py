"""Cached daily prices (read side).

Legacy reader of ``data/raw/TICKER/prices.json``. The Rust program makes that file
(``fa analyze``, from the Yahoo chart endpoint); the Rust form of this module is
``rust/src/market/prices.rs``. This file stays only because ``scripts/visualize_dcf.py``
(the HTML report, not yet in Rust) reads the cache through it. Delete it when that report
moves to Rust.
"""
from __future__ import annotations
import json
from datetime import date
from pathlib import Path

from financial_analysis.schema import PricePoint


def load_prices(ticker: str, cache_dir: Path) -> list[PricePoint]:
    """The prices of the cache file. There is no download in Python: run ``fa analyze TICKER``."""
    path = cache_dir / ticker.upper() / "prices.json"
    if not path.exists():
        raise FileNotFoundError(f"No price cache for {ticker.upper()} at {path}. Run: fa analyze {ticker.upper()}")
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
