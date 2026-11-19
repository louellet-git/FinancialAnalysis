"""Compare the standard variables with an independent source (Yahoo Finance).

Our mapping and the Yahoo standardized financials are two independent ways to get
the same numbers from the same filings. Where they agree, the value is probably
correct. Where they differ, a person must find the cause: an incorrect mapping, a
different definition, or an error in the independent source.

This module has no network code. ``scripts.mapping_crosscheck`` gets the data.
"""
from __future__ import annotations
from dataclasses import dataclass, field
from datetime import date
from typing import Callable, Dict, List, Optional, Tuple

from financial_analysis.schema import PeriodFinancials

# Yahoo dates a fiscal year at the month end; a 52/53-week filer ends some days before.
MATCH_DAYS = 7

Ours = Callable[[PeriodFinancials], Optional[float]]
Theirs = Callable[[Dict[str, float]], Optional[float]]


def _get(name: str) -> Ours:
    return lambda p: getattr(p, name)


def _label(name: str) -> Theirs:
    return lambda y: y.get(name)


def _capex_total(p: PeriodFinancials) -> Optional[float]:
    return p.capex_total if p.capex_total is not None else p.capex


def _wide_capex(p: PeriodFinancials) -> Optional[float]:
    total = _capex_total(p)
    if total is None:
        return None
    return total + (p.purchases_of_intangibles or 0.0)


def _debt_without_leases(y: Dict[str, float]) -> Optional[float]:
    total = y.get("Total Debt")
    if total is None:
        return None
    return total - (y.get("Capital Lease Obligations") or 0.0)


@dataclass(frozen=True)
class Comparison:
    variable: str
    # (definition name, our value, the Yahoo value). The first pair is the standard
    # definition; a later pair that agrees shows a difference of definition only.
    definitions: Tuple[Tuple[str, Ours, Theirs], ...]
    tolerance: float = 0.01


COMPARISONS: Tuple[Comparison, ...] = (
    Comparison("revenue", (("standard", _get("revenue"), _label("Total Revenue")),)),
    Comparison("income_before_tax", (("standard", _get("income_before_tax"), _label("Pretax Income")),)),
    Comparison("income_tax_expense", (("standard", _get("income_tax_expense"), _label("Tax Provision")),)),
    Comparison("net_income", (("standard", _get("net_income"), _label("Net Income")),)),
    Comparison("shares_diluted", (("standard", _get("shares_diluted"), _label("Diluted Average Shares")),), 0.02),
    Comparison("operating_cash_flow", (("standard", _get("operating_cash_flow"), _label("Operating Cash Flow")),)),
    # The standard is capex_total: equipment and capitalized software (RULES.md R2).
    Comparison("capex", (
        ("standard", _capex_total, _label("Capital Expenditure")),
        ("with purchased intangibles", _wide_capex, _label("Capital Expenditure")),
        ("equipment only", _get("capex"), _label("Capital Expenditure")),
    )),
    Comparison("da_total", (("standard", _get("da_total"), _label("Depreciation And Amortization")),), 0.02),
    Comparison("sbc_total", (("standard", _get("sbc_total"), _label("Stock Based Compensation")),)),
    Comparison("cash_and_equivalents", (("standard", _get("cash_and_equivalents"), _label("Cash And Cash Equivalents")),)),
    Comparison("total_assets", (("standard", _get("total_assets"), _label("Total Assets")),)),
    Comparison("total_liabilities", (("standard", _get("total_liabilities"), _label("Total Liabilities Net Minority Interest")),)),
    Comparison("equity", (("standard", _get("equity"), _label("Stockholders Equity")),)),
    Comparison("total_debt", (
        ("standard (leases not included)", _get("total_debt"), _debt_without_leases),
        ("Yahoo total debt with leases", _get("total_debt"), _label("Total Debt")),
    ), 0.02),
)

AGREE = "agree"
DEFINITION = "definition"   # agrees with a different definition
DIFFER = "differ"
NO_VALUE_OURS = "no value (ours)"
NO_VALUE_THEIRS = "no value (Yahoo)"
OUTCOMES = (AGREE, DEFINITION, DIFFER, NO_VALUE_OURS, NO_VALUE_THEIRS)


@dataclass
class Result:
    variable: str
    outcome: str
    ours: Optional[float] = None
    theirs: Optional[float] = None
    note: str = ""


def _near(ours: float, theirs: float, tolerance: float) -> bool:
    # Signs are not compared: the sources use different sign conventions for outflows.
    return abs(abs(ours) - abs(theirs)) <= tolerance * max(abs(theirs), 1.0)


def compare_period(period: PeriodFinancials, yahoo: Dict[str, float]) -> List[Result]:
    """One result for each comparison. ``yahoo`` is {Yahoo row label: value}."""
    results: List[Result] = []
    for comparison in COMPARISONS:
        _, ours_fn, theirs_fn = comparison.definitions[0]
        ours, theirs = ours_fn(period), theirs_fn(yahoo)
        if theirs is None:
            results.append(Result(comparison.variable, NO_VALUE_THEIRS, ours, None))
            continue
        if ours is None:
            results.append(Result(comparison.variable, NO_VALUE_OURS, None, theirs))
            continue
        if _near(ours, theirs, comparison.tolerance):
            results.append(Result(comparison.variable, AGREE, ours, theirs))
            continue
        outcome, note = DIFFER, ""
        for name, alt_ours_fn, alt_theirs_fn in comparison.definitions[1:]:
            alt_ours, alt_theirs = alt_ours_fn(period), alt_theirs_fn(yahoo)
            if alt_ours is not None and alt_theirs is not None and _near(alt_ours, alt_theirs, comparison.tolerance):
                outcome, note = DEFINITION, name
                break
        results.append(Result(comparison.variable, outcome, ours, theirs, note))
    return results


def match_fiscal_year(period_ends: List[date], yahoo_dates: List[date]) -> Optional[Tuple[date, date]]:
    """The latest (our period end, Yahoo date) pair that is the same fiscal year."""
    for theirs in sorted(yahoo_dates, reverse=True):
        for ours in sorted(period_ends, reverse=True):
            if abs((ours - theirs).days) <= MATCH_DAYS:
                return ours, theirs
    return None


@dataclass
class Summary:
    companies: int = 0
    counts: Dict[str, Dict[str, int]] = field(default_factory=dict)
    examples: Dict[str, List[Tuple[str, Result]]] = field(default_factory=dict)

    def add(self, ticker: str, results: List[Result]) -> None:
        self.companies += 1
        for r in results:
            self.counts.setdefault(r.variable, {o: 0 for o in OUTCOMES})[r.outcome] += 1
            if r.outcome in (DIFFER, NO_VALUE_OURS, DEFINITION):
                self.examples.setdefault(r.variable, []).append((ticker, r))

    def agreement(self, variable: str) -> Optional[float]:
        """Share of the companies where both sources have a value and the values agree."""
        c = self.counts.get(variable)
        if not c:
            return None
        both = c[AGREE] + c[DEFINITION] + c[DIFFER]
        return c[AGREE] / both if both else None
