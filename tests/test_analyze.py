import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from datetime import date

import pytest

from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from scripts.analyze import _flatten, write_xlsx, COLUMNS


def _make_company() -> CompanyFinancials:
    p1 = PeriodFinancials(
        ticker="TEST",
        period_end=date(2024, 1, 28),
        form="10-K",
        filed_date=date(2024, 2, 21),
        revenue=60_920_000_000,
        operating_cash_flow=28_090_000_000,
        capex=-1_070_000_000,
        total_assets=65_730_000_000,
        raw_extras={"us-gaap/PaymentsToAcquireEquitySecuritiesFvNi": 860_000_000},
    )
    p2 = PeriodFinancials(
        ticker="TEST",
        period_end=date(2023, 10, 29),
        form="10-Q",
        filed_date=date(2023, 11, 21),
        revenue=18_120_000_000,
        raw_extras={},
    )
    return CompanyFinancials(ticker="TEST", cik="1045810", company_name="Test Co", periods=[p1, p2])


def test_flatten_returns_four_tabs():
    company = _make_company()
    tabs = _flatten(company)
    assert set(tabs.keys()) == {"Income Statement", "Balance Sheet", "Cash Flow", "Raw"}


def test_flatten_income_statement_has_revenue_row():
    tabs = _flatten(_make_company())
    is_rows = tabs["Income Statement"]
    metrics = [r["metric"] for r in is_rows]
    assert "revenue" in metrics


def test_flatten_cash_flow_has_free_cash_flow():
    tabs = _flatten(_make_company())
    cf_rows = tabs["Cash Flow"]
    metrics = [r["metric"] for r in cf_rows]
    assert "free_cash_flow" in metrics


def test_flatten_raw_tab_has_unmapped_concepts():
    tabs = _flatten(_make_company())
    raw_rows = tabs["Raw"]
    metrics = [r["metric"] for r in raw_rows]
    assert "us-gaap/PaymentsToAcquireEquitySecuritiesFvNi" in metrics


def test_flatten_none_values_not_included():
    tabs = _flatten(_make_company())
    for tab_name, tab_rows in tabs.items():
        for row in tab_rows:
            assert row.get("value") is not None, f"None value in {tab_name}: {row['metric']}"


def test_flatten_row_has_all_columns():
    tabs = _flatten(_make_company())
    for tab_name, tab_rows in tabs.items():
        for row in tab_rows:
            for col in COLUMNS:
                assert col in row, f"Column '{col}' missing from row in {tab_name}"


def test_flatten_capex_unit_is_usd():
    tabs = _flatten(_make_company())
    capex_rows = [r for r in tabs["Cash Flow"] if r["metric"] == "capex"]
    assert all(r["unit"] == "USD" for r in capex_rows)


def test_write_xlsx_creates_file_with_four_tabs(tmp_path):
    import openpyxl
    company = _make_company()
    out = tmp_path / "TEST_financials.xlsx"
    write_xlsx(company, out)
    assert out.exists()
    wb = openpyxl.load_workbook(out)
    assert set(wb.sheetnames) == {"Income Statement", "Balance Sheet", "Cash Flow", "Raw"}


def test_write_xlsx_has_header_row(tmp_path):
    import openpyxl
    company = _make_company()
    out = tmp_path / "TEST_financials.xlsx"
    write_xlsx(company, out)
    wb = openpyxl.load_workbook(out)
    ws = wb["Income Statement"]
    header = [cell.value for cell in ws[1]]
    assert header == COLUMNS


def test_write_xlsx_returns_total_row_count(tmp_path):
    company = _make_company()
    out = tmp_path / "TEST_financials.xlsx"
    total = write_xlsx(company, out)
    assert total > 0
