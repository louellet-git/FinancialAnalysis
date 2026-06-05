import json
from typing import Any, Dict, List

from financial_analysis.edgar import _http_get, _zero_pad_cik

COMPANY_FACTS_URL = "https://data.sec.gov/api/xbrl/companyfacts/CIK{cik}.json"


def fetch_company_facts(cik: str) -> Dict[str, Any]:
    url = COMPANY_FACTS_URL.format(cik=_zero_pad_cik(cik))
    return json.loads(_http_get(url))


def _get_usd_entries(facts: Dict[str, Any], concept: str) -> List[Dict[str, Any]]:
    parts = concept.split("/", 1)
    if len(parts) != 2:
        return []
    namespace, name = parts
    try:
        return facts["facts"][namespace][name]["units"]["USD"]
    except KeyError:
        return []


def _deduplicate(entries: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """Keep the latest-filed entry per end date."""
    best: Dict[str, Dict[str, Any]] = {}
    for e in entries:
        end = e["end"]
        if end not in best or e.get("filed", "") > best[end].get("filed", ""):
            best[end] = e
    return sorted(best.values(), key=lambda e: e["end"])


def extract_annual_series(facts: Dict[str, Any], concept: str) -> List[Dict[str, Any]]:
    entries = [e for e in _get_usd_entries(facts, concept) if e.get("form") == "10-K"]
    return _deduplicate(entries)


def extract_quarterly_series(facts: Dict[str, Any], concept: str) -> List[Dict[str, Any]]:
    entries = [e for e in _get_usd_entries(facts, concept) if e.get("form") == "10-Q"]
    return _deduplicate(entries)


def extract_series(
    facts: Dict[str, Any],
    concept: str,
    form_types: List[str],
    unit: str = "USD",
    n: int = 10,
) -> List[Dict[str, Any]]:
    """Extract a time series of financial data for any concept, form types, and units.

    Args:
        facts: SEC XBRL company facts dict
        concept: Concept identifier in "namespace/name" format (e.g., "us-gaap/Revenues")
        form_types: List of form types to include (e.g., ["10-K"] or ["10-Q"])
        unit: Unit to extract (e.g., "USD", "USD/shares"). Defaults to "USD".
        n: Maximum number of most recent entries to return. Defaults to 10.

    Returns:
        List of dicts sorted by end date (ascending), deduplicated by latest filing,
        limited to the n most recent entries.
    """
    parts = concept.split("/", 1)
    if len(parts) != 2:
        return []
    namespace, name = parts
    try:
        entries = facts["facts"][namespace][name]["units"].get(unit, [])
    except KeyError:
        return []
    filtered = [e for e in entries if e.get("form") in form_types]
    return _deduplicate(filtered)[-n:]
