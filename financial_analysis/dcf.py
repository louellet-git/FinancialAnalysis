from __future__ import annotations
from dataclasses import dataclass, field
from typing import Optional

import numpy as np

from financial_analysis.schema import CompanyFinancials, PeriodFinancials

_ANNUAL_FORMS = {"10-K", "10-K/A"}
_MIN_POSITIVE_FCF_PERIODS = 4
_MARGIN_LOOKBACK = 5


@dataclass
class DCFAssumptions:
    n_simulations: int = 10_000
    projection_years: int = 10
    growth_rate_mean: float = 0.15
    growth_rate_std: float = 0.05
    terminal_growth_mean: float = 0.03
    terminal_growth_std: float = 0.005
    wacc_mean: float = 0.10
    wacc_std: float = 0.01
    fcf_margin_mean: Optional[float] = None
    fcf_margin_std: Optional[float] = None
    seed: Optional[int] = None


@dataclass
class DCFResult:
    median_value: float
    p10_value: float
    p90_value: float
    mean_value: float
    std_value: float
    median_per_share: Optional[float]
    simulations: np.ndarray = field(repr=False)
    fcfs: np.ndarray = field(repr=False)          # (projection_years, n_simulations)
    pv_fcfs: np.ndarray = field(repr=False)       # (projection_years, n_simulations)
    pv_terminal: np.ndarray = field(repr=False)   # (n_simulations,)


def derive_fcf_margin(company: CompanyFinancials) -> tuple[float, float]:
    """Historical FCF margin (mean, std) over the last annual periods.

    Same derivation StochasticDCF applies when assumptions leave the margin
    unset — exposed so callers (e.g. sensitivity analysis) can pin it explicitly.
    """
    annuals = [p for p in company.periods if p.form in _ANNUAL_FORMS]
    margins = [
        p.fcf_margin for p in annuals[-_MARGIN_LOOKBACK:]
        if p.fcf_margin is not None
    ]
    if not margins:
        raise ValueError(
            f"{company.ticker}: FCF margin cannot be derived — no annual "
            "periods with both FCF and revenue data."
        )
    arr = np.array(margins)
    return float(arr.mean()), max(float(arr.std()), 0.01)


class StochasticDCF:
    """Monte Carlo DCF over revenue growth, FCF margin, WACC, and terminal growth.

    Loads nothing itself — pass a CompanyFinancials from store.load(). Only
    annual (10-K) periods feed the model; quarterly rows are ignored.
    """

    def __init__(self, company: CompanyFinancials, assumptions: DCFAssumptions) -> None:
        self.company = company
        self.assumptions = assumptions

    def _annual_periods(self) -> list[PeriodFinancials]:
        return [p for p in self.company.periods if p.form in _ANNUAL_FORMS]

    def _check_minimum_history(self, annuals: list[PeriodFinancials]) -> None:
        positive = [
            p for p in annuals
            if p.free_cash_flow is not None and p.free_cash_flow > 0
        ]
        if len(positive) < _MIN_POSITIVE_FCF_PERIODS:
            raise ValueError(
                f"{self.company.ticker}: only {len(positive)} annual period(s) with "
                f"positive FCF — at least {_MIN_POSITIVE_FCF_PERIODS} are required to fit "
                "Monte Carlo input distributions. Short or loss-making histories "
                "(e.g. recently IPO'd companies) cannot be valued meaningfully with this model."
            )

    def _derive_fcf_margin(self, annuals: list[PeriodFinancials]) -> tuple[float, float]:
        return derive_fcf_margin(self.company)

    def _base_revenue(self, annuals: list[PeriodFinancials]) -> float:
        for p in reversed(annuals):
            if p.revenue is not None:
                return p.revenue
        raise ValueError(f"{self.company.ticker}: no annual revenue data available.")

    def _shares_outstanding(self, annuals: list[PeriodFinancials]) -> Optional[float]:
        for p in reversed(annuals):
            if p.shares_outstanding:
                return p.shares_outstanding
            if p.shares_diluted:
                return p.shares_diluted
        return None

    def run(self) -> DCFResult:
        a = self.assumptions
        n = a.n_simulations

        annuals = self._annual_periods()
        self._check_minimum_history(annuals)

        fcf_margin_mean = a.fcf_margin_mean
        fcf_margin_std = a.fcf_margin_std
        if fcf_margin_mean is None or fcf_margin_std is None:
            fcf_margin_mean, fcf_margin_std = self._derive_fcf_margin(annuals)

        base_revenue = self._base_revenue(annuals)
        rng = np.random.default_rng(a.seed)

        # Sampled inputs — shape (n,)
        growth_rates = rng.normal(a.growth_rate_mean, a.growth_rate_std, n)
        terminal_growth = rng.normal(a.terminal_growth_mean, a.terminal_growth_std, n)
        wacc = np.clip(rng.normal(a.wacc_mean, a.wacc_std, n), 0.01, None)
        fcf_margins = np.clip(rng.normal(fcf_margin_mean, fcf_margin_std, n), 0.0, 1.0)

        # Projected revenue per year — shape (projection_years, n)
        years = np.arange(1, a.projection_years + 1).reshape(-1, 1)
        revenues = base_revenue * (1 + growth_rates) ** years
        fcfs = revenues * fcf_margins

        discount_factors = 1 / (1 + wacc) ** years
        pv_fcfs = fcfs * discount_factors

        # Terminal value: Gordon Growth on final-year FCF
        terminal_fcf = fcfs[-1] * (1 + terminal_growth)
        terminal_value = terminal_fcf / np.clip(wacc - terminal_growth, 0.001, None)
        pv_terminal = terminal_value / (1 + wacc) ** a.projection_years

        total_values = pv_fcfs.sum(axis=0) + pv_terminal

        median_value = float(np.percentile(total_values, 50))
        shares = self._shares_outstanding(annuals)

        return DCFResult(
            median_value=median_value,
            p10_value=float(np.percentile(total_values, 10)),
            p90_value=float(np.percentile(total_values, 90)),
            mean_value=float(total_values.mean()),
            std_value=float(total_values.std()),
            median_per_share=(median_value / shares) if shares else None,
            simulations=total_values,
            fcfs=fcfs,
            pv_fcfs=pv_fcfs,
            pv_terminal=pv_terminal,
        )
