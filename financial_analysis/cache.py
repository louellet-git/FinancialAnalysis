from __future__ import annotations
import json
import logging
from pathlib import Path

from financial_analysis.edgar import get_cik_for_ticker
from financial_analysis.xbrl_api import fetch_company_facts

logger = logging.getLogger(__name__)


def load_company_facts(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> dict:
    path = cache_dir / ticker.upper() / "company_facts.json"
    if not refresh and path.exists():
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            logger.warning("Corrupt cache for %s — re-fetching from SEC", ticker)

    cik = get_cik_for_ticker(ticker)
    facts = fetch_company_facts(cik)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(facts, indent=2), encoding="utf-8")
    return facts
