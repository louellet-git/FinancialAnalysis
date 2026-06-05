from datetime import date
from financial_analysis.schema import (
    PricePoint, EarningsEstimate, EarningsSurprise, CompanyFinancials,
)


def test_price_point_fields():
    p = PricePoint(date=date(2024, 1, 2), open=100.0, high=105.0, low=99.0, close=103.0, volume=1_000_000)
    assert p.close == 103.0
    assert p.volume == 1_000_000


def test_earnings_estimate_optional_fields():
    e = EarningsEstimate(period_end=date(2024, 3, 31), period_type="quarterly")
    assert e.estimated_eps is None
    assert e.number_of_analysts is None


def test_earnings_surprise_fields():
    s = EarningsSurprise(
        date=date(2024, 4, 24),
        actual_eps=2.50,
        estimated_eps=2.30,
        surprise=0.20,
        surprise_pct=0.087,
    )
    assert s.surprise == 0.20


def test_company_financials_market_defaults():
    c = CompanyFinancials(ticker="NVDA", cik="1045810", company_name="NVIDIA")
    assert c.prices == []
    assert c.estimates == []
    assert c.surprises == []
