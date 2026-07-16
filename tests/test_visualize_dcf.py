from datetime import date

import pytest

from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from scripts.visualize_dcf import PANEL_TITLES, write_report


def _annual_period(year: int, revenue: float, ocf: float, capex: float,
                   **kwargs) -> PeriodFinancials:
    return PeriodFinancials(
        ticker="ACME", period_end=date(year, 12, 31),
        form="10-K", filed_date=date(year + 1, 2, 15),
        revenue=revenue, operating_cash_flow=ocf, capex=capex, **kwargs,
    )


def _make_company() -> CompanyFinancials:
    periods = [
        _annual_period(2021 + i, revenue=50e9 * (1.05 ** i),
                       ocf=12e9 * (1.05 ** i), capex=-2e9)
        for i in range(4)
    ]
    periods.append(_annual_period(
        2025, revenue=50e9 * (1.05 ** 4), ocf=12e9 * (1.05 ** 4), capex=-2e9,
        long_term_debt=20e9, cash_and_equivalents=5e9,
        raw_extras={"us-gaap/ShortTermBorrowings": 1e9},
        shares_outstanding=1e9,
    ))
    return CompanyFinancials(ticker="ACME", cik="0", company_name="Acme Corp",
                             periods=periods)


@pytest.fixture(scope="module")
def report_html(tmp_path_factory) -> str:
    out = tmp_path_factory.mktemp("reports") / "ACME_dcf.html"
    path = write_report(_make_company(), out, price=150.0,
                        price_note="price $150.00 (test)")
    assert path.exists()
    return path.read_text(encoding="utf-8")


def test_report_contains_plotly(report_html):
    assert "Plotly.newPlot" in report_html
    assert 'class="plotly-graph-div"' in report_html


def test_report_contains_all_panel_titles(report_html):
    for title in PANEL_TITLES:
        assert title in report_html


def test_report_is_self_contained(report_html):
    # plotly.js is inlined; the markup loads nothing over the network
    # (the inlined bundle itself contains URL string literals — those never load
    # for the trace types used here, so only actual load-bearing tags are checked)
    assert "<script src=" not in report_html
    assert "<link " not in report_html
    assert '<img src="http' not in report_html
    assert len(report_html) > 1_000_000  # the ~4MB bundle is really embedded


def test_report_shows_price_marker(report_html):
    assert "Price $150.00" in report_html


def test_report_without_price_omits_marker(tmp_path):
    out = tmp_path / "ACME_dcf.html"
    note = "current price unavailable (offline?) — price marker omitted"
    html = write_report(_make_company(), out, price=None,
                        price_note=note).read_text(encoding="utf-8")
    assert "Price $" not in html
    assert "price marker omitted" in html
