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


# --- fcf_margin derived property ---

def test_fcf_margin_computed_when_revenue_and_fcf_present():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12),
        revenue=8_030_000_000.0,
        operating_cash_flow=3_260_000_000.0,
        capex=157_000_000.0,
    )
    assert abs(p.fcf_margin - (3_260_000_000.0 - 157_000_000.0) / 8_030_000_000.0) < 1e-9


def test_fcf_margin_none_when_revenue_missing():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12),
        operating_cash_flow=3_260_000_000.0,
        capex=157_000_000.0,
    )
    assert p.fcf_margin is None


def test_fcf_margin_none_when_fcf_missing():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12),
        revenue=8_030_000_000.0,
    )
    assert p.fcf_margin is None


# --- New Income Statement fields ---

def test_period_financials_new_income_statement_fields():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        cogs_total=10_000_000,
        interest_income=500_000,
        interest_expense=250_000,
        income_before_tax=5_000_000,
        income_tax_expense=1_000_000,
        da_total=800_000,
        sbc_total=1_200_000,
        eps_diluted=2.50,
        shares_diluted=2_500_000_000,
    )
    assert p.cogs_total == 10_000_000
    assert p.eps_diluted == 2.50


def test_period_financials_new_balance_sheet_fields():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        cash_and_equivalents=7_000_000_000,
        accounts_receivable_net=10_000_000_000,
        total_current_assets=44_000_000_000,
        long_term_debt=9_000_000_000,
        total_current_liabilities=10_000_000_000,
    )
    assert p.cash_and_equivalents == 7_000_000_000


def test_period_financials_new_cash_flow_fields():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        operating_cash_flow=28_000_000_000,
        capex=-1_000_000_000,
        repurchases_common_stock=-9_500_000_000,
        net_cash_from_investing=-10_000_000_000,
        net_cash_from_financing=-13_000_000_000,
    )
    assert p.repurchases_common_stock == -9_500_000_000


def test_period_financials_raw_extras_defaults_empty():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
    )
    assert p.raw_extras == {}


def test_period_financials_raw_extras_stores_concepts():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        raw_extras={"us-gaap/PaymentsToAcquireEquitySecuritiesFvNi": 17_500_000_000},
    )
    assert p.raw_extras["us-gaap/PaymentsToAcquireEquitySecuritiesFvNi"] == 17_500_000_000
