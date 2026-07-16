"""Named bear/base/bull DCF scenario presets and per-ticker capital structure.

The generic presets suit a mature large-cap. Per-ticker overrides — scenario
labels tuned to the company's story, pinned net debt / share counts — live in
the registries here so analysis scripts and the visualization share one source
of truth.
"""
from __future__ import annotations
from dataclasses import dataclass
from typing import Dict, Tuple

from financial_analysis.dcf import _ANNUAL_FORMS, DCFAssumptions
from financial_analysis.schema import CompanyFinancials, PeriodFinancials

SCENARIO_SEED = 7

_SHORT_TERM_DEBT_KEYS = (
    "us-gaap/ShortTermBorrowings",
    "us-gaap/LongTermDebtAndCapitalLeaseObligationsCurrent",
)


@dataclass(frozen=True)
class Scenario:
    key: str    # "bear" | "base" | "bull"
    label: str  # display label for reports
    assumptions: DCFAssumptions


@dataclass(frozen=True)
class CapitalStructure:
    net_debt: float
    shares: float


def _preset(growth: float, wacc: float, terminal: float) -> DCFAssumptions:
    return DCFAssumptions(
        growth_rate_mean=growth, growth_rate_std=0.02,
        wacc_mean=wacc, wacc_std=0.01,
        terminal_growth_mean=terminal, terminal_growth_std=0.005,
        seed=SCENARIO_SEED,
    )


DEFAULT_SCENARIOS: Tuple[Scenario, ...] = (
    Scenario("bear", "Bear  (growth stalls)", _preset(0.01, 0.095, 0.015)),
    Scenario("base", "Base  (mature grower, historical trend)", _preset(0.04, 0.085, 0.025)),
    Scenario("bull", "Bull  (growth re-accelerates)", _preset(0.06, 0.08, 0.03)),
)

def _ibm_v3(growth: float, wacc: float, terminal: float, margin: float) -> DCFAssumptions:
    # v3 framework (analysis/IBM/REVIEW.md): FCFF after SBC with paired M&A charge,
    # margins pinned from the FY2021-25 build in analysis/IBM/dcf_scenarios.py.
    return DCFAssumptions(
        growth_rate_mean=growth, growth_rate_std=0.02,
        wacc_mean=wacc, wacc_std=0.0075,
        terminal_growth_mean=terminal, terminal_growth_std=0.005,
        fcf_margin_mean=margin, fcf_margin_std=0.02,
        seed=SCENARIO_SEED,
    )


_SCENARIO_OVERRIDES: Dict[str, Tuple[Scenario, ...]] = {
    "IBM": (
        Scenario("bear", "Bear  (disruption: g 1%, 100% M&A charged)",
                 _ibm_v3(0.01, 0.0825, 0.015, 0.092)),
        Scenario("base", "Base  (organic + half-paid acquired growth)",
                 _ibm_v3(0.0215, 0.075, 0.0225, 0.145)),
        Scenario("bull", "Bull  (guidance-anchored recovery)",
                 _ibm_v3(0.045, 0.0725, 0.025, 0.161)),
    ),
}

CAPITAL_OVERRIDES: Dict[str, CapitalStructure] = {
    # v3 bridge (analysis/IBM/REVIEW.md F15): $61.3B debt + $7.1B after-tax pension
    # - $14.4B cash & securities; diluted shares, held flat (SBC charged in margin)
    "IBM": CapitalStructure(net_debt=54.0e9, shares=948_675_228),
}


def scenarios_for(ticker: str) -> Tuple[Scenario, ...]:
    return _SCENARIO_OVERRIDES.get(ticker.upper(), DEFAULT_SCENARIOS)


def latest_annual(company: CompanyFinancials) -> PeriodFinancials:
    annuals = [p for p in company.periods if p.form in _ANNUAL_FORMS]
    if not annuals:
        raise ValueError(f"{company.ticker}: no annual (10-K) periods available.")
    return max(annuals, key=lambda p: p.period_end)


def capital_structure_for(company: CompanyFinancials) -> CapitalStructure:
    """Net debt and share count for equity-per-share math.

    A ticker override in CAPITAL_OVERRIDES wins. Otherwise derived from the
    latest annual period: long_term_debt + short-term borrowings (first
    available key in _SHORT_TERM_DEBT_KEYS; 0 when neither is reported) minus
    cash_and_equivalents. Shares come from shares_outstanding, falling back to
    shares_diluted. Raises ValueError when required fields are missing.
    """
    override = CAPITAL_OVERRIDES.get(company.ticker.upper())
    if override is not None:
        return override

    latest = latest_annual(company)
    if latest.long_term_debt is None or latest.cash_and_equivalents is None:
        raise ValueError(
            f"{company.ticker}: cannot derive net debt — latest annual period "
            f"({latest.period_end}) is missing long_term_debt or "
            "cash_and_equivalents. Add an override to scenarios.CAPITAL_OVERRIDES."
        )
    short_term = next(
        (latest.raw_extras[k] for k in _SHORT_TERM_DEBT_KEYS
         if latest.raw_extras.get(k) is not None),
        0.0,
    )
    shares = latest.shares_outstanding or latest.shares_diluted
    if not shares:
        raise ValueError(
            f"{company.ticker}: no shares_outstanding or shares_diluted in the "
            f"latest annual period ({latest.period_end}). Add an override to "
            "scenarios.CAPITAL_OVERRIDES."
        )
    net_debt = latest.long_term_debt + short_term - latest.cash_and_equivalents
    return CapitalStructure(net_debt=float(net_debt), shares=float(shares))
