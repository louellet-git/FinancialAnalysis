from datetime import date
import pytest

from financial_analysis.schema import PeriodFinancials, CompanyFinancials


# --- PeriodFinancials defaults ---

def test_currency_defaults_to_usd():
    p = PeriodFinancials(ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12))
    assert p.currency == "USD"


def test_all_financial_fields_default_to_none():
    p = PeriodFinancials(ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12))
    for field in (
        "revenue", "gross_profit", "operating_income", "net_income",
        "rd_expense", "sga_expense", "operating_cash_flow", "capex",
        "total_assets", "total_liabilities", "equity", "shares_outstanding",
    ):
        assert getattr(p, field) is None, f"{field} should default to None"


# --- free_cash_flow derived property ---

def test_free_cash_flow_computed_when_both_present():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12),
        operating_cash_flow=3_260_000_000.0,
        capex=157_000_000.0,
    )
    assert p.free_cash_flow == 3_260_000_000.0 - 157_000_000.0


def test_free_cash_flow_none_when_ocf_missing():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12),
        capex=157_000_000.0,
    )
    assert p.free_cash_flow is None


def test_free_cash_flow_none_when_capex_missing():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12),
        operating_cash_flow=3_260_000_000.0,
    )
    assert p.free_cash_flow is None


def test_free_cash_flow_none_when_both_missing():
    p = PeriodFinancials(ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12))
    assert p.free_cash_flow is None


# --- CompanyFinancials ---

def test_company_financials_holds_multiple_periods():
    periods = [
        PeriodFinancials(ticker="PANW", period_end=date(2023, 7, 31), form="10-K", filed_date=date(2023, 9, 11)),
        PeriodFinancials(ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12)),
    ]
    company = CompanyFinancials(ticker="PANW", cik="1327567", company_name="Palo Alto Networks Inc", periods=periods)
    assert len(company.periods) == 2


def test_company_financials_periods_ordered_ascending():
    periods = [
        PeriodFinancials(ticker="PANW", period_end=date(2023, 7, 31), form="10-K", filed_date=date(2023, 9, 11)),
        PeriodFinancials(ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12)),
    ]
    company = CompanyFinancials(ticker="PANW", cik="1327567", company_name="Palo Alto Networks Inc", periods=periods)
    assert company.periods[0].period_end < company.periods[1].period_end


def test_company_financials_empty_periods_by_default():
    company = CompanyFinancials(ticker="PANW", cik="1327567", company_name="Palo Alto Networks Inc")
    assert company.periods == []
