from __future__ import annotations
import gzip
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
    """SEC company facts for a ticker, from the disk cache or from SEC EDGAR.

    The cache file is ``<cache_dir>/<TICKER>/company_facts.json``. A compressed file
    ``company_facts.json.gz`` (written by ``scripts.fetch_facts``) is also read.
    """
    path = cache_dir / ticker.upper() / "company_facts.json"
    gz_path = path.with_name("company_facts.json.gz")
    if not refresh:
        if path.exists():
            try:
                return json.loads(path.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                logger.warning("Corrupt cache for %s — re-fetching from SEC", ticker)
        elif gz_path.exists():
            try:
                with gzip.open(gz_path, "rt", encoding="utf-8") as handle:
                    return json.load(handle)
            except (json.JSONDecodeError, OSError):
                logger.warning("Corrupt cache for %s — re-fetching from SEC", ticker)

    cik = get_cik_for_ticker(ticker)
    facts = fetch_company_facts(cik)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(facts, indent=2), encoding="utf-8")
    return facts
