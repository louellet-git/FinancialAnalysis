"""Audit of the standard-variable mapping (financial_analysis/xbrl_tags.py).

For one period the audit gives:
- the core DCF inputs that have no value;
- accounting identities that do not agree (a sign of an incorrect mapping);
- material tags of the filer that have no standard variable and no exclusion.

The audit changes no data.
"""
from __future__ import annotations
import re
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple

from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.xbrl_tags import ALL_TAGS, CORE_FIELDS, EXCLUDED_CONCEPTS

IDENTITY_TOLERANCE = 0.005   # of total assets or of revenue
MATERIAL_SHARE = 0.05        # of the smaller of revenue and total assets
TOTAL_DEBT_TOLERANCE = 0.02  # difference between total-debt estimates

_EXCLUDED = [(re.compile(pattern), reason) for pattern, reason in EXCLUDED_CONCEPTS]

# Tags that show a business with a different statement structure. The standard
# variables (cost of revenue, operating income, debt as financing) do not fit these.
_TEMPLATE_MARKERS: Tuple[Tuple[str, Tuple[str, ...]], ...] = (
    ("bank", ("Deposits", "InterestExpenseDeposits", "InterestAndDividendIncomeOperating")),
    ("insurer", ("PremiumsEarnedNet", "PolicyholderBenefitsAndClaimsIncurredNet")),
    ("reit", ("RealEstateInvestmentPropertyNet", "RealEstateInvestmentPropertyAtCost")),
)


def filer_template(facts: dict, end: Optional[str] = None) -> str:
    """"standard", or "bank" / "insurer" / "reit" (from the tags the filer uses).

    With ``end`` (a fiscal year end, ISO date), only tags that have a fact on that
    date count. This ignores a business that the filer sold years ago.
    """
    gaap = facts.get("facts", {}).get("us-gaap", {})

    def in_use(marker: str) -> bool:
        if marker not in gaap:
            return False
        if end is None:
            return True
        return any(
            e.get("end") == end
            for unit_entries in gaap[marker].get("units", {}).values()
            for e in unit_entries
        )

    for template, markers in _TEMPLATE_MARKERS:
        if any(in_use(m) for m in markers):
            return template
    return "standard"


def exclusion_reason(concept: str) -> Optional[str]:
    name = concept.split("/", 1)[-1]
    for pattern, reason in _EXCLUDED:
        if pattern.search(name):
            return reason
    return None


@dataclass
class PeriodAudit:
    ticker: str
    period_end: str
    core_missing: List[str] = field(default_factory=list)
    filled: int = 0
    from_formula: int = 0
    identity_failures: List[str] = field(default_factory=list)
    unmapped: List[Tuple[str, float]] = field(default_factory=list)

    @property
    def core_filled(self) -> int:
        return len(CORE_FIELDS) - len(self.core_missing)


def _gap(left: Optional[float], right: Optional[float], base: Optional[float]) -> Optional[float]:
    if left is None or right is None or not base:
        return None
    return (left - right) / abs(base)


def _from_tag(p: PeriodFinancials, name: str) -> bool:
    # A value with no recorded source (a period made by hand) counts as a tag value.
    return not p.sources.get(name, "").startswith("=")


def identity_checks(p: PeriodFinancials) -> List[Tuple[str, Optional[float]]]:
    """(name, gap as a share of total assets or revenue). None = cannot be checked."""
    def v(name: str) -> Optional[float]:
        return getattr(p, name)

    checks: List[Tuple[str, Optional[float]]] = []
    checks.append((
        "total_assets = total_liabilities_and_equity",
        _gap(v("total_assets"), v("total_liabilities_and_equity"), v("total_assets")),
    ))
    if all(_from_tag(p, n) for n in ("total_liabilities", "total_equity_incl_nci")) or all(
        _from_tag(p, n) for n in ("total_liabilities", "equity")
    ):
        equity_all = v("total_equity_incl_nci") if v("total_equity_incl_nci") is not None else v("equity")
        right = None
        if v("total_liabilities") is not None and equity_all is not None:
            right = v("total_liabilities") + equity_all + (v("temporary_equity") or 0.0)
        checks.append((
            "total_assets = total_liabilities + equity (+ temporary equity)",
            _gap(v("total_assets"), right, v("total_assets")),
        ))
    if all(_from_tag(p, n) for n in ("gross_profit", "revenue", "cogs_total")):
        checks.append((
            "gross_profit = revenue - cogs_total",
            _gap(v("gross_profit"), (v("revenue") or 0.0) - (v("cogs_total") or 0.0), v("revenue")),
        ))
    if v("long_term_debt") is not None and v("long_term_debt_total") is not None and v("long_term_debt_current") is not None:
        checks.append((
            "long_term_debt_total = long_term_debt + long_term_debt_current",
            _gap(v("long_term_debt_total"), v("long_term_debt") + v("long_term_debt_current"), v("total_assets")),
        ))
    alternatives = total_debt_alternatives(p)
    if len(alternatives) > 1:
        low, high = min(alternatives.values()), max(alternatives.values())
        gap = (high - low) / high if high else 0.0
        label = "total_debt estimates are equal (" + " | ".join(
            f"{k}: {val / 1e9:,.1f}B" for k, val in alternatives.items()) + ")"
        # The gap is a share of the larger estimate; the tolerance is 2%.
        checks.append((label, gap if gap > TOTAL_DEBT_TOLERANCE else 0.0))
    if _from_tag(p, "net_income_incl_nci") and v("income_before_tax") is not None and v("income_tax_expense") is not None:
        expected = (
            v("income_before_tax") - v("income_tax_expense")
            + (v("equity_method_income") or 0.0 if _after_tax_equity_method(p) else 0.0)
            + (v("discontinued_operations_income") or 0.0)
        )
        checks.append((
            "net_income_incl_nci = income_before_tax - income_tax_expense (+ discontinued)",
            _gap(v("net_income_incl_nci"), expected, v("revenue")),
        ))
    return checks


def total_debt_alternatives(p: PeriodFinancials) -> Dict[str, float]:
    """The ways to calculate total debt from the standard variables, with values."""
    out: Dict[str, float] = {}
    if _from_tag(p, "total_debt") and p.total_debt is not None:
        out["tag"] = p.total_debt
    if p.long_term_debt is not None:
        out["debt_current + long_term_debt"] = (p.debt_current or 0.0) + p.long_term_debt
    if p.long_term_debt_total is not None:
        out["short_term_borrowings + long_term_debt_total"] = (
            (p.short_term_borrowings or 0.0) + p.long_term_debt_total
        )
    return out


def _after_tax_equity_method(p: PeriodFinancials) -> bool:
    # The second income_before_tax tag is before equity-method income.
    return "MinorityInterestAndIncomeLossFromEquityMethodInvestments" in p.sources.get("income_before_tax", "")


def audit_period(p: PeriodFinancials) -> PeriodAudit:
    audit = PeriodAudit(ticker=p.ticker, period_end=p.period_end.isoformat())
    for name in ALL_TAGS:
        if getattr(p, name) is not None:
            audit.filled += 1
            if not _from_tag(p, name):
                audit.from_formula += 1
    audit.core_missing = [n for n in CORE_FIELDS if getattr(p, n) is None]
    for name, gap in identity_checks(p):
        if gap is not None and abs(gap) > IDENTITY_TOLERANCE:
            audit.identity_failures.append(f"{name}: gap {gap:+.1%}")
    bases = [abs(x) for x in (p.revenue, p.total_assets) if x]
    if bases:
        threshold = MATERIAL_SHARE * min(bases)
        for concept, value in (p.raw_extras or {}).items():
            if isinstance(value, (int, float)) and abs(value) >= threshold and exclusion_reason(concept) is None:
                audit.unmapped.append((concept, float(value)))
        audit.unmapped.sort(key=lambda cv: -abs(cv[1]))
    return audit


def latest_annual_period(company: CompanyFinancials) -> Optional[PeriodFinancials]:
    annuals = [p for p in company.periods if p.form == "10-K" and p.total_assets is not None]
    return max(annuals, key=lambda p: p.period_end) if annuals else None


def core_fill_rates(audits: List[PeriodAudit]) -> Dict[str, float]:
    """Share of the audits in which each core variable has a value."""
    if not audits:
        return {}
    return {
        name: sum(1 for a in audits if name not in a.core_missing) / len(audits)
        for name in CORE_FIELDS
    }
