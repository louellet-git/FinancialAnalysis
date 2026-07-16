from datetime import date

import pytest

from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.scenarios import (
    CAPITAL_OVERRIDES,
    DEFAULT_SCENARIOS,
    CapitalStructure,
    capital_structure_for,
    latest_annual,
    scenarios_for,
)


def _annual_period(year: int, **kwargs) -> PeriodFinancials:
    return PeriodFinancials(
        ticker=kwargs.pop("ticker", "ACME"), period_end=date(year, 12, 31),
        form="10-K", filed_date=date(year + 1, 2, 15), **kwargs,
    )


def _make_company(ticker: str = "ACME", *, long_term_debt: float | None = 10e9,
                  cash: float | None = 2e9, raw_extras: dict | None = None,
                  shares_outstanding: float | None = 1e9,
                  shares_diluted: float | None = None) -> CompanyFinancials:
    periods = [_annual_period(2021 + i, ticker=ticker) for i in range(4)]
    periods.append(_annual_period(
        2025, ticker=ticker, long_term_debt=long_term_debt,
        cash_and_equivalents=cash, raw_extras=raw_extras or {},
        shares_outstanding=shares_outstanding, shares_diluted=shares_diluted,
    ))
    return CompanyFinancials(ticker=ticker, cik="0", company_name="Acme Corp",
                             periods=periods)


# --- scenario registry ---

def test_unknown_ticker_gets_default_scenarios():
    assert scenarios_for("XYZ") is DEFAULT_SCENARIOS


def test_default_scenarios_keys_and_order():
    assert [s.key for s in DEFAULT_SCENARIOS] == ["bear", "base", "bull"]


def test_default_scenario_assumptions():
    by_key = {s.key: s.assumptions for s in DEFAULT_SCENARIOS}
    assert by_key["bear"].growth_rate_mean == 0.01
    assert by_key["bear"].wacc_mean == 0.095
    assert by_key["bear"].terminal_growth_mean == 0.015
    assert by_key["base"].growth_rate_mean == 0.04
    assert by_key["base"].wacc_mean == 0.085
    assert by_key["base"].terminal_growth_mean == 0.025
    assert by_key["bull"].growth_rate_mean == 0.06
    assert by_key["bull"].wacc_mean == 0.08
    assert by_key["bull"].terminal_growth_mean == 0.03
    assert all(a.seed == 7 for a in by_key.values())


def test_ibm_override_lookup_is_case_insensitive():
    scenarios = scenarios_for("ibm")
    assert scenarios is not DEFAULT_SCENARIOS
    assert "disruption" in scenarios[0].label


# --- capital structure ---

def test_ibm_capital_override():
    company = CompanyFinancials(ticker="IBM", cik="0", company_name="IBM", periods=[])
    cap = capital_structure_for(company)
    assert cap == CAPITAL_OVERRIDES["IBM"]
    assert cap.net_debt == 54.0e9
    assert cap.shares == 948_675_228


def test_net_debt_derived_with_short_term_borrowings():
    company = _make_company(raw_extras={"us-gaap/ShortTermBorrowings": 1e9})
    cap = capital_structure_for(company)
    assert cap.net_debt == pytest.approx(10e9 + 1e9 - 2e9)
    assert cap.shares == 1e9


def test_net_debt_falls_back_to_current_lt_debt_key():
    company = _make_company(
        raw_extras={"us-gaap/LongTermDebtAndCapitalLeaseObligationsCurrent": 0.5e9}
    )
    cap = capital_structure_for(company)
    assert cap.net_debt == pytest.approx(10e9 + 0.5e9 - 2e9)


def test_net_debt_without_short_term_key_treats_it_as_zero():
    cap = capital_structure_for(_make_company())
    assert cap.net_debt == pytest.approx(10e9 - 2e9)


def test_net_debt_uses_latest_annual_period():
    company = _make_company()
    # Earlier annuals carry different balance data — the 2025 period must win
    company.periods[0].long_term_debt = 99e9
    company.periods[0].cash_and_equivalents = 0.0
    assert latest_annual(company).period_end.year == 2025
    assert capital_structure_for(company).net_debt == pytest.approx(8e9)


def test_missing_long_term_debt_raises():
    with pytest.raises(ValueError, match="net debt"):
        capital_structure_for(_make_company(long_term_debt=None))


def test_missing_cash_raises():
    with pytest.raises(ValueError, match="net debt"):
        capital_structure_for(_make_company(cash=None))


def test_shares_fall_back_to_diluted():
    company = _make_company(shares_outstanding=None, shares_diluted=1.05e9)
    assert capital_structure_for(company).shares == 1.05e9


def test_missing_shares_raises():
    with pytest.raises(ValueError, match="shares"):
        capital_structure_for(_make_company(shares_outstanding=None))


def test_no_annual_periods_raises():
    company = CompanyFinancials(ticker="ACME", cik="0", company_name="Acme Corp",
                                periods=[])
    with pytest.raises(ValueError, match="annual"):
        capital_structure_for(company)
