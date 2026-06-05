from __future__ import annotations
import json
import logging
import os
from datetime import date
from pathlib import Path

import requests
from dotenv import load_dotenv

from financial_analysis.schema import (
    CompanyFinancials,
    EarningsEstimate,
    EarningsSurprise,
)

load_dotenv()

logger = logging.getLogger(__name__)
_BASE = "https://financialmodelingprep.com/api/v3"


class FMPClientError(Exception):
    pass


def _fmp_get(endpoint: str) -> list[dict]:
    api_key = os.environ.get("FMP_API_KEY")
    if not api_key:
        raise FMPClientError("FMP_API_KEY not set in environment — add it to .env")
    url = f"{_BASE}/{endpoint}"
    resp = requests.get(url, params={"apikey": api_key}, timeout=30)
    if resp.status_code != 200:
        raise FMPClientError(f"FMP API returned HTTP {resp.status_code} for {url}")
    return resp.json()


def load_estimates(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> list[EarningsEstimate]:
    path = cache_dir / ticker.upper() / "estimates.json"

    if not refresh and path.exists():
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
            return _parse_estimates(raw)
        except (json.JSONDecodeError, KeyError):
            logger.warning("Corrupt estimates cache for %s — re-fetching", ticker)

    raw = _fmp_get(f"analyst-estimates/{ticker.upper()}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(raw, indent=2), encoding="utf-8")
    return _parse_estimates(raw)


def _parse_estimates(raw: list[dict]) -> list[EarningsEstimate]:
    out = []
    for r in raw:
        period_str = r.get("period", "")
        period_type = "quarterly" if period_str == "quarter" else "annual"
        try:
            out.append(EarningsEstimate(
                period_end=date.fromisoformat(r["date"]),
                period_type=period_type,
                estimated_eps=r.get("estimatedEpsAvg"),
                estimated_revenue=r.get("estimatedRevenueAvg"),
                number_of_analysts=r.get("numberAnalystEstimatedEps"),
            ))
        except (KeyError, ValueError) as exc:
            logger.warning("Skipping malformed estimate record: %s", exc)
    return out


def load_surprises(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> list[EarningsSurprise]:
    path = cache_dir / ticker.upper() / "surprises.json"

    if not refresh and path.exists():
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
            return _parse_surprises(raw)
        except (json.JSONDecodeError, KeyError):
            logger.warning("Corrupt surprises cache for %s — re-fetching", ticker)

    raw = _fmp_get(f"earnings-surprises/{ticker.upper()}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(raw, indent=2), encoding="utf-8")
    return _parse_surprises(raw)


def _parse_surprises(raw: list[dict]) -> list[EarningsSurprise]:
    out = []
    for r in raw:
        try:
            actual = float(r["actualEarningResult"])
            estimated = float(r["estimatedEarning"])
            surprise = actual - estimated
            surprise_pct = surprise / abs(estimated) if estimated != 0 else 0.0
            out.append(EarningsSurprise(
                date=date.fromisoformat(r["date"]),
                actual_eps=actual,
                estimated_eps=estimated,
                surprise=round(surprise, 6),
                surprise_pct=round(surprise_pct, 6),
            ))
        except (KeyError, ValueError, ZeroDivisionError) as exc:
            logger.warning("Skipping malformed surprise record: %s", exc)
    return out


def load_fmp_statements(ticker: str) -> CompanyFinancials:
    raise NotImplementedError(
        f"IFRS via FMP not yet implemented for {ticker}. "
        "This stub exists to wire the fallback path for future development."
    )
