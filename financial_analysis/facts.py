"""Selection of XBRL facts by period length.

The SEC companyfacts payload holds, for one tag and one end date, facts of
different lengths: a 10-K contains full-year values and also quarter values, and a
10-Q contains quarter values and year-to-date values. This module selects the fact
of the correct length:

- an annual row uses only facts that cover a full fiscal year (350 to 380 days);
- a quarterly row uses a fact that covers one quarter (80 to 100 days). If the
  filer gives only year-to-date values (usual for the cash-flow statement), the
  quarter value is the year-to-date value less the prior year-to-date value.

Balance-sheet facts (instants) have no length; the end date selects them.
A duration fact without a ``start`` date has an unknown length and is accepted.
When more than one filing reports the same period, the latest filing wins (this is
the restated value).
"""
from __future__ import annotations
from dataclasses import dataclass
from datetime import date
from typing import Any, Dict, Iterable, List, Optional, Tuple

ANNUAL_FORMS = ("10-K", "10-K/A")
QUARTERLY_FORMS = ("10-Q", "10-Q/A")

_ANNUAL_DAYS = (350, 380)
_QUARTER_DAYS = (80, 100)

YTD_NOTE = "year-to-date difference"


@dataclass(frozen=True)
class Selected:
    value: float
    filed: str
    note: str = ""


def _days(entry: Dict[str, Any]) -> Optional[int]:
    start = entry.get("start")
    if not start:
        return None
    return (date.fromisoformat(entry["end"]) - date.fromisoformat(start)).days


def _in(days: Optional[int], bounds: Tuple[int, int]) -> bool:
    return days is None or bounds[0] <= days <= bounds[1]


def entries_for(facts: Dict[str, Any], concept: str, unit: str = "USD") -> List[Dict[str, Any]]:
    """All facts for a ``namespace/Name`` concept in one unit ([] if absent)."""
    namespace, _, name = concept.partition("/")
    try:
        return facts["facts"][namespace][name]["units"].get(unit, [])
    except KeyError:
        return []


def _latest(entries: Iterable[Dict[str, Any]]) -> Optional[Dict[str, Any]]:
    best = None
    for e in entries:
        if best is None or e.get("filed", "") > best.get("filed", ""):
            best = e
    return best


def period_ends(
    facts: Dict[str, Any],
    anchors: Iterable[str],
    forms: Tuple[str, ...],
    n: Optional[int] = None,
) -> List[str]:
    """End dates of the periods that exist for these forms, oldest first.

    A period exists if one anchor concept has a fact of the correct length (a full
    year for annual forms, one quarter for quarterly forms) that ends on that date.
    ``n`` keeps the latest n periods.
    """
    bounds = _ANNUAL_DAYS if forms == ANNUAL_FORMS else _QUARTER_DAYS
    ends = set()
    for concept in anchors:
        for e in entries_for(facts, concept):
            if e.get("form") in forms and _in(_days(e), bounds):
                ends.add(e["end"])
    ordered = sorted(ends)
    return ordered[-n:] if n else ordered


def select_instant(
    entries: List[Dict[str, Any]], end: str, forms: Tuple[str, ...]
) -> Optional[Selected]:
    best = _latest(e for e in entries if e["end"] == end and e.get("form") in forms)
    if best is None:
        return None
    return Selected(best["val"], best.get("filed", ""))


def select_annual(entries: List[Dict[str, Any]], end: str) -> Optional[Selected]:
    best = _latest(
        e for e in entries
        if e["end"] == end and e.get("form") in ANNUAL_FORMS and _in(_days(e), _ANNUAL_DAYS)
    )
    if best is None:
        return None
    return Selected(best["val"], best.get("filed", ""))


def select_quarter(entries: List[Dict[str, Any]], end: str) -> Optional[Selected]:
    """The value for the one quarter that ends on ``end``.

    Uses a fact of quarter length if one exists. If not, uses the longest
    year-to-date fact that ends on ``end`` less the fact with the same start date
    that ends one quarter before.
    """
    here = [e for e in entries if e["end"] == end and e.get("form") in QUARTERLY_FORMS]
    best = _latest(e for e in here if _in(_days(e), _QUARTER_DAYS))
    if best is not None:
        return Selected(best["val"], best.get("filed", ""))

    ytd_candidates = [e for e in here if (_days(e) or 0) > _QUARTER_DAYS[1]]
    if not ytd_candidates:
        return None
    longest = max(_days(e) for e in ytd_candidates)
    ytd = _latest(e for e in ytd_candidates if _days(e) == longest)
    end_date = date.fromisoformat(end)
    prior = _latest(
        e for e in entries
        if e.get("start") == ytd["start"]
        and e.get("form") in QUARTERLY_FORMS
        and _QUARTER_DAYS[0] <= (end_date - date.fromisoformat(e["end"])).days <= _QUARTER_DAYS[1]
    )
    if prior is None:
        return None
    filed = max(ytd.get("filed", ""), prior.get("filed", ""))
    return Selected(ytd["val"] - prior["val"], filed, YTD_NOTE)


def select(
    entries: List[Dict[str, Any]], end: str, forms: Tuple[str, ...], period: str
) -> Optional[Selected]:
    """Select the fact for one period. ``period`` is "duration" or "instant"."""
    if period == "instant":
        return select_instant(entries, end, forms)
    if forms == ANNUAL_FORMS:
        return select_annual(entries, end)
    return select_quarter(entries, end)
