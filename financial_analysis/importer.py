from __future__ import annotations
from dataclasses import fields as dataclass_fields
from datetime import date
from pathlib import Path
from typing import Any

from financial_analysis import store
from financial_analysis.cache import load_company_facts
from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.xbrl_api import _deduplicate, extract_series
from financial_analysis.xbrl_tags import (
    BALANCE_SHEET_TAGS,
    CASH_FLOW_TAGS,
    INCOME_STATEMENT_TAGS,
    TagSpec,
)

_ANNUAL_FORMS = ["10-K", "10-K/A"]
_QUARTERLY_FORMS = ["10-Q", "10-Q/A"]

_ALL_TAGS: dict[str, TagSpec] = {
    **INCOME_STATEMENT_TAGS,
    **BALANCE_SHEET_TAGS,
    **CASH_FLOW_TAGS,
}

_PERIOD_FIELDS = {f.name for f in dataclass_fields(PeriodFinancials)}


class UnsupportedFilingError(Exception):
    pass


def _resolve_field(
    facts: dict,
    field_name: str,
    spec: TagSpec,
    form_types: list[str],
    n: int,
) -> dict[str, float]:
    """Returns {period_end: value}, merging all concepts (first concept wins per period_end).

    Merging handles companies that use different concepts for different fiscal years
    (e.g. NVDA used RevenueFromContractWith... through FY2022, then Revenues for FY2023+).
    """
    result: dict[str, float] = {}
    for concept in spec.concepts:
        entries = extract_series(facts, concept, form_types=form_types, unit=spec.unit, n=n)
        for e in entries:
            if e["end"] not in result:
                val = e["val"]
                if spec.sign_flip:
                    val = -abs(val)
                result[e["end"]] = val
    return result


def _collect_raw_extras(
    facts: dict,
    canonical_concepts: set[str],
    form_types: list[str],
    known_period_ends: set[str],
    n: int,
) -> dict[str, dict[str, float]]:
    """Returns {period_end: {full_concept: value}} for unmapped us-gaap USD concepts."""
    extras: dict[str, dict[str, float]] = {end: {} for end in known_period_ends}
    gaap = facts.get("facts", {}).get("us-gaap", {})
    for concept_name, concept_data in gaap.items():
        full = f"us-gaap/{concept_name}"
        if full in canonical_concepts:
            continue
        entries = [
            e for e in concept_data.get("units", {}).get("USD", [])
            if e.get("form") in form_types and e["end"] in known_period_ends
        ]
        for e in _deduplicate(entries)[-n:]:
            extras.setdefault(e["end"], {})[full] = e["val"]
    return extras


def _populate_filed_dates(
    facts: dict,
    all_periods: dict[tuple[str, str], dict],
    form_types: list[str],
) -> None:
    """Best-effort: fill filed_date by scanning all revenue concept fallbacks."""
    from financial_analysis.xbrl_tags import INCOME_STATEMENT_TAGS
    revenue_concepts = INCOME_STATEMENT_TAGS["revenue"].concepts
    for concept in revenue_concepts:
        ns, name = concept.split("/", 1)
        try:
            entries = facts["facts"][ns][name]["units"]["USD"]
        except KeyError:
            continue
        for e in entries:
            if e.get("form") not in form_types or "filed" not in e:
                continue
            key_form = "10-K" if "K" in e["form"] else "10-Q"
            key = (e["end"], key_form)
            if key in all_periods:
                all_periods[key]["filed_date"] = e["filed"]


def import_company(
    ticker: str,
    n_periods: int = 8,
    save: bool = True,
    refresh: bool = False,
    cache_dir: Path = Path("data/raw"),
) -> CompanyFinancials:
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

    all_periods: dict[tuple[str, str], dict[str, Any]] = {}

    for form_types in (_ANNUAL_FORMS, _QUARTERLY_FORMS):
        primary_form = form_types[0]  # "10-K" or "10-Q"

        for field_name, spec in _ALL_TAGS.items():
            values = _resolve_field(facts, field_name, spec, form_types, n_periods)
            for end, val in values.items():
                key = (end, primary_form)
                if key not in all_periods:
                    all_periods[key] = {
                        "ticker": ticker.upper(),
                        "period_end": end,
                        "form": primary_form,
                        "filed_date": end,  # placeholder, overwritten below
                        "currency": "USD",
                        "raw_extras": {},
                    }
                # Only set if not already populated (first-wins per concept fallback order)
                if field_name not in all_periods[key]:
                    all_periods[key][field_name] = val

        known_ends = {k[0] for k in all_periods if k[1] == primary_form}
        extras = _collect_raw_extras(facts, canonical_concepts, form_types, known_ends, n_periods)
        for end, extra_dict in extras.items():
            key = (end, primary_form)
            if key in all_periods:
                all_periods[key]["raw_extras"].update(extra_dict)

        _populate_filed_dates(facts, all_periods, form_types)

    periods: list[PeriodFinancials] = []
    for data in sorted(all_periods.values(), key=lambda d: (d["period_end"], d["form"])):
        kwargs: dict[str, Any] = {}
        for f in dataclass_fields(PeriodFinancials):
            if f.name in ("period_end", "filed_date"):
                raw = data.get(f.name, data["period_end"])
                kwargs[f.name] = date.fromisoformat(raw) if isinstance(raw, str) else raw
            elif f.name == "raw_extras":
                kwargs[f.name] = data.get("raw_extras", {})
            elif f.name == "currency":
                kwargs[f.name] = data.get("currency", "USD")
            else:
                kwargs[f.name] = data.get(f.name)
        periods.append(PeriodFinancials(**kwargs))

    company = CompanyFinancials(
        ticker=ticker.upper(),
        cik=cik,
        company_name=company_name,
        periods=periods,
    )

    if save:
        store.save(company)

    return company
