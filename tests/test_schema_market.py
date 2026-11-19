from datetime import date
from financial_analysis.schema import PricePoint, CompanyFinancials


def test_price_point_fields():
    p = PricePoint(date=date(2024, 1, 2), open=100.0, high=105.0, low=99.0, close=103.0, volume=1_000_000)
    assert p.close == 103.0
    assert p.volume == 1_000_000


def test_company_financials_market_defaults():
    c = CompanyFinancials(ticker="NVDA", cik="1045810", company_name="NVIDIA")
    assert c.prices == []
    assert c.option_chain is None
