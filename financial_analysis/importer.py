"""SEC companyfacts XBRL → PeriodFinancials importer.

Segment revenue finding (P1 importer-completeness, 2026-07-17):
the SEC ``companyfacts`` JSON API returns only non-dimensional (consolidated)
facts. Every fact entry carries exactly the keys
``{accn, end, filed, form, fp, frame, fy, start, val}`` — there is no segment
axis / member metadata anywhere in the payload (verified by scanning every
entry of every us-gaap concept in IBM's cached companyfacts: no other keys
occur). Per-segment revenue therefore CANNOT be extracted from this data
source, and ``PeriodFinancials.segment_revenue`` stays None rather than being
faked from another source. Segment-tagged facts DO exist in the filing-level
XBRL instance documents (and the frames API); extraction from those is
ticketed — see PLANNING/tickets.md, Ticket 7.
"""
from __future__ import annotations
from dataclasses import fields as dataclass_fields
from datetime import date
from pathlib import Path
from typing import Any, Dict, Optional, Tuple

from financial_analysis import store
from financial_analysis.cache import load_company_facts
from financial_analysis.facts import (
    ANNUAL_FORMS,
    QUARTERLY_FORMS,
    entries_for,
    period_ends,
    select,
)
from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.xbrl_tags import ALL_TAGS, ANCHOR_CONCEPTS, TagSpec

_ALL_TAGS: dict[str, TagSpec] = ALL_TAGS

_PERIOD_FIELDS = {f.name for f in dataclass_fields(PeriodFinancials)}

DEFAULT_N_PERIODS = 12


class UnsupportedFilingError(Exception):
    pass


def _parse_formula(formula: str) -> list[Tuple[float, str, bool]]:
    """``"a - b + c?"`` -> [(+1, "a", False), (-1, "b", False), (+1, "c", True)]."""
    terms: list[Tuple[float, str, bool]] = []
    sign = 1.0
    for token in formula.split():
        if token == "+":
            sign = 1.0
        elif token == "-":
            sign = -1.0
        else:
            optional = token.endswith("?")
            terms.append((sign, token.rstrip("?"), optional))
            sign = 1.0
    return terms


def _apply_formulas(values: Dict[str, float], sources: Dict[str, str]) -> None:
    """Fill the variables that no tag gave, from the registry formulas.

    A formula is used only if each required term has a value and one term (required
    or optional) has a value. Terms can be other formula results; a variable that is
    in work is not used again (no loops).
    """
    in_work: set[str] = set()

    def resolve(name: str) -> Optional[float]:
        if name in values:
            return values[name]
        spec = _ALL_TAGS.get(name)
        if spec is None or not spec.formulas or name in in_work:
            return None
        in_work.add(name)
        try:
            for formula in spec.formulas:
                terms = _parse_formula(formula)
                total = 0.0
                used: list[str] = []
                seen: Dict[float, str] = {}
                same: list[str] = []
                ok = True
                for sign, term, optional in terms:
                    val = resolve(term)
                    if val is None:
                        if optional:
                            continue
                        ok = False
                        break
                    used.append(term)
                    # Two terms of a sum with the same value are one item that the
                    # filer gave two tags (for example IBM short-term debt). Count
                    # the value one time.
                    if sign > 0 and val != 0 and val in seen and all(sg > 0 for sg, _, _ in terms):
                        same.append(f"{seen[val]} = {term}")
                        continue
                    seen.setdefault(val, term)
                    total += sign * val
                if ok and used:
                    values[name] = total
                    missing = [t for _, t, opt in terms if opt and t not in used]
                    notes = []
                    if missing:
                        notes.append(f"no value: {', '.join(missing)}")
                    if same:
                        notes.append(f"same value, counted one time: {'; '.join(same)}")
                    note = f" ({'; '.join(notes)})" if notes else ""
                    sources[name] = f"= {formula.replace('?', '')}{note}"
                    return total
            return None
        finally:
            in_work.discard(name)

    for name in _ALL_TAGS:
        resolve(name)


SHARE_SCALE_FACTORS = (1_000.0, 1_000_000.0)
SHARE_SCALE_TOLERANCE = 0.10
MIN_EPS_FOR_SHARE_CHECK = 0.05


def implied_share_count(values: Dict[str, float], eps_name: str) -> Optional[float]:
    """Net income for the common shareholders divided by EPS (None if not available)."""
    eps = values.get(eps_name)
    income = values.get("net_income_to_common", values.get("net_income"))
    if eps is None or income is None or abs(eps) < MIN_EPS_FOR_SHARE_CHECK:
        return None
    implied = income / eps
    return implied if implied > 0 else None


def _correct_share_scale(values: Dict[str, float], sources: Dict[str, str]) -> None:
    """Correct a share count that the filer reports in thousands or in millions.

    Some filers tag the share count as the number in the statement (for example 716.4
    for 716.4 million shares). Net income divided by EPS gives the real size. If the
    two differ by a factor of 1,000 or 1,000,000 (10% tolerance), the share count is
    multiplied by that factor and the source records the correction.
    """
    applied: Optional[float] = None
    for shares_name, eps_name in (("shares_diluted", "eps_diluted"), ("shares_basic", "eps_basic")):
        shares = values.get(shares_name)
        implied = implied_share_count(values, eps_name)
        if not shares or shares <= 0 or implied is None:
            continue
        for factor in SHARE_SCALE_FACTORS:
            if abs(implied / (shares * factor) - 1.0) <= SHARE_SCALE_TOLERANCE:
                values[shares_name] = shares * factor
                sources[shares_name] = f"{sources.get(shares_name, '')} (x{int(factor):,}: scale from net income / EPS)"
                applied = factor
                break
    # The balance-sheet share count has the same scale if it is near the weighted count.
    outstanding = values.get("shares_outstanding")
    weighted = values.get("shares_diluted") or values.get("shares_basic")
    if applied and outstanding and weighted and 0.5 <= outstanding * applied / weighted <= 2.0:
        values["shares_outstanding"] = outstanding * applied
        sources["shares_outstanding"] = (
            f"{sources.get('shares_outstanding', '')} (x{int(applied):,}: same scale as the weighted share count)"
        )


def resolve_period(
    facts: dict, end: str, forms: Tuple[str, ...]
) -> Tuple[Dict[str, float], Dict[str, str], Optional[str]]:
    """Standard variables for one period.

    Returns (values, sources, filed). ``sources[field]`` is the tag that gave the
    value, or the formula. ``filed`` is the filing date of the revenue fact, or the
    latest filing date of the facts in use if the period has no revenue fact.
    """
    values: Dict[str, float] = {}
    sources: Dict[str, str] = {}
    filed_dates: Dict[str, str] = {}
    for name, spec in _ALL_TAGS.items():
        for concept in spec.concepts:
            sel = select(entries_for(facts, concept, spec.unit), end, forms, spec.period)
            if sel is None:
                continue
            if spec.sign_flip:
                values[name] = -abs(sel.value)
            elif spec.negate:
                values[name] = -sel.value
            else:
                values[name] = sel.value
            sources[name] = f"{concept} ({sel.note})" if sel.note else concept
            if sel.filed:
                filed_dates[name] = sel.filed
            break
    _apply_formulas(values, sources)
    _correct_share_scale(values, sources)
    filed = filed_dates.get("revenue") or (max(filed_dates.values()) if filed_dates else None)
    return values, sources, filed


def _collect_raw_extras(
    facts: dict,
    canonical_concepts: set[str],
    end: str,
    forms: Tuple[str, ...],
) -> dict[str, float]:
    """{full_concept: value} for the us-gaap USD concepts that have no standard variable."""
    extras: dict[str, float] = {}
    gaap = facts.get("facts", {}).get("us-gaap", {})
    for concept_name, concept_data in gaap.items():
        full = f"us-gaap/{concept_name}"
        if full in canonical_concepts:
            continue
        entries = concept_data.get("units", {}).get("USD", [])
        if not entries:
            continue
        period = "duration" if any("start" in e for e in entries) else "instant"
        sel = select(entries, end, forms, period)
        if sel is not None:
            extras[full] = sel.value
    return extras


def import_company(
    ticker: str,
    n_periods: int = DEFAULT_N_PERIODS,
    save: bool = True,
    refresh: bool = False,
    cache_dir: Path = Path("data/raw"),
) -> CompanyFinancials:
    """Import the latest ``n_periods`` fiscal years and ``n_periods`` quarters.

    Each annual row is a full fiscal year and each quarterly row is one quarter
    (financial_analysis/facts.py). Each value has a recorded source
    (``PeriodFinancials.sources``).
    """
    facts = load_company_facts(ticker, cache_dir=cache_dir, refresh=refresh)

    if "us-gaap" not in facts.get("facts", {}):
        namespaces = list(facts.get("facts", {}).keys())
        raise UnsupportedFilingError(
            f"{ticker} does not report under us-gaap (namespaces found: {namespaces}). "
            "IFRS and non-US-GAAP filers are not supported."
        )

    cik = str(facts.get("cik", ""))
    company_name = facts.get("entityName", ticker.upper())
    canonical_concepts = {c for spec in _ALL_TAGS.values() for c in spec.concepts}

    periods: list[PeriodFinancials] = []
    for forms in (ANNUAL_FORMS, QUARTERLY_FORMS):
        primary_form = forms[0]  # "10-K" or "10-Q"
        for end in period_ends(facts, ANCHOR_CONCEPTS, forms, n_periods):
            values, sources, filed = resolve_period(facts, end, forms)
            kwargs: dict[str, Any] = {
                "ticker": ticker.upper(),
                "period_end": date.fromisoformat(end),
                "form": primary_form,
                "filed_date": date.fromisoformat(filed or end),
                "currency": "USD",
                "sources": sources,
                "raw_extras": _collect_raw_extras(facts, canonical_concepts, end, forms),
            }
            kwargs.update({k: v for k, v in values.items() if k in _PERIOD_FIELDS})
            periods.append(PeriodFinancials(**kwargs))

    periods.sort(key=lambda p: (p.period_end, p.form))

    company = CompanyFinancials(
        ticker=ticker.upper(),
        cik=cik,
        company_name=company_name,
        periods=periods,
    )

    if save:
        store.save(company)

    return company
