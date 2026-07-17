from datetime import date
from unittest.mock import patch

import pytest

from financial_analysis.importer import import_company, UnsupportedFilingError
from financial_analysis.schema import CompanyFinancials


def _make_facts(revenue_val: float = 60_920_000_000) -> dict:
    return {
        "entityName": "NVIDIA CORP",
        "cik": 1045810,
        "facts": {
            "us-gaap": {
                "Revenues": {
                    "units": {
                        "USD": [
                            {"end": "2024-01-28", "val": revenue_val, "form": "10-K", "filed": "2024-02-21"},
                            {"end": "2023-10-29", "val": 18_120_000_000, "form": "10-Q", "filed": "2023-11-21"},
                        ]
                    }
                },
                "NetCashProvidedByUsedInOperatingActivities": {
                    "units": {
                        "USD": [
                            {"end": "2024-01-28", "val": 28_090_000_000, "form": "10-K", "filed": "2024-02-21"},
                        ]
                    }
                },
            }
        },
    }


def test_import_company_returns_company_financials(tmp_path):
    facts = _make_facts()
    with patch("financial_analysis.importer.load_company_facts", return_value=facts), \
         patch("financial_analysis.importer.store.save"):
        result = import_company("NVDA", save=False, cache_dir=tmp_path)
    assert isinstance(result, CompanyFinancials)
    assert result.ticker == "NVDA"
    assert result.company_name == "NVIDIA CORP"


def test_import_company_populates_annual_periods(tmp_path):
    facts = _make_facts()
    with patch("financial_analysis.importer.load_company_facts", return_value=facts), \
         patch("financial_analysis.importer.store.save"):
        result = import_company("NVDA", save=False, cache_dir=tmp_path)
    annual = [p for p in result.periods if p.form == "10-K"]
    assert len(annual) >= 1
    assert annual[0].revenue == 60_920_000_000


def test_import_company_populates_quarterly_periods(tmp_path):
    facts = _make_facts()
    with patch("financial_analysis.importer.load_company_facts", return_value=facts), \
         patch("financial_analysis.importer.store.save"):
        result = import_company("NVDA", save=False, cache_dir=tmp_path)
    quarterly = [p for p in result.periods if p.form == "10-Q"]
    assert len(quarterly) >= 1


def test_import_company_applies_sign_flip_to_capex(tmp_path):
    facts = _make_facts()
    facts["facts"]["us-gaap"]["PaymentsToAcquireProductiveAssets"] = {
        "units": {"USD": [{"end": "2024-01-28", "val": 1_070_000_000, "form": "10-K", "filed": "2024-02-21"}]}
    }
    with patch("financial_analysis.importer.load_company_facts", return_value=facts), \
         patch("financial_analysis.importer.store.save"):
        result = import_company("NVDA", save=False, cache_dir=tmp_path)
    annual = [p for p in result.periods if p.form == "10-K"][0]
    assert annual.capex == -1_070_000_000


def test_import_company_raises_for_ifrs_filer(tmp_path):
    ifrs_facts = {
        "entityName": "SPOTIFY AB",
        "cik": 1639920,
        "facts": {"ifrs-full": {"Revenue": {"units": {"EUR": []}}}},
    }
    with patch("financial_analysis.importer.load_company_facts", return_value=ifrs_facts):
        with pytest.raises(UnsupportedFilingError, match="us-gaap"):
            import_company("SPOT", save=False, cache_dir=tmp_path)


def test_import_company_captures_raw_extras(tmp_path):
    facts = _make_facts()
    facts["facts"]["us-gaap"]["PaymentsToAcquireEquitySecuritiesFvNi"] = {
        "units": {"USD": [{"end": "2024-01-28", "val": 17_500_000_000, "form": "10-K", "filed": "2024-02-21"}]}
    }
    with patch("financial_analysis.importer.load_company_facts", return_value=facts), \
         patch("financial_analysis.importer.store.save"):
        result = import_company("NVDA", save=False, cache_dir=tmp_path)
    annual = [p for p in result.periods if p.form == "10-K"][0]
    assert "us-gaap/PaymentsToAcquireEquitySecuritiesFvNi" in annual.raw_extras


def _import(facts, tmp_path):
    with patch("financial_analysis.importer.load_company_facts", return_value=facts), \
         patch("financial_analysis.importer.store.save"):
        return import_company("NVDA", save=False, cache_dir=tmp_path)


def _annual(result):
    return [p for p in result.periods if p.form == "10-K"][0]


def _add_concept(facts, name, val, unit="USD"):
    facts["facts"]["us-gaap"][name] = {
        "units": {unit: [{"end": "2024-01-28", "val": val, "form": "10-K", "filed": "2024-02-21"}]}
    }


# ---------------------------------------------------------------------------
# P1 importer completeness: dividends, buybacks, interest income, RPO, segments
# ---------------------------------------------------------------------------

def test_dividends_paid_maps_payments_of_dividends_common_stock(tmp_path):
    """IBM-style filer: only the common-stock payments concept is tagged."""
    facts = _make_facts()
    _add_concept(facts, "PaymentsOfDividendsCommonStock", 6_147_000_000)
    assert _annual(_import(facts, tmp_path)).dividends_paid == -6_147_000_000


def test_dividends_paid_prefers_total_payments_over_common_stock(tmp_path):
    facts = _make_facts()
    _add_concept(facts, "PaymentsOfDividends", 6_200_000_000)
    _add_concept(facts, "PaymentsOfDividendsCommonStock", 6_147_000_000)
    assert _annual(_import(facts, tmp_path)).dividends_paid == -6_200_000_000


def test_dividends_paid_falls_back_to_declared(tmp_path):
    facts = _make_facts()
    _add_concept(facts, "DividendsCommonStockCash", 6_100_000_000)
    assert _annual(_import(facts, tmp_path)).dividends_paid == -6_100_000_000


def test_dividends_paid_none_when_absent(tmp_path):
    assert _annual(_import(_make_facts(), tmp_path)).dividends_paid is None


def test_repurchases_common_stock_mapped_when_present(tmp_path):
    facts = _make_facts()
    _add_concept(facts, "PaymentsForRepurchaseOfCommonStock", 4_400_000_000)
    assert _annual(_import(facts, tmp_path)).repurchases_common_stock == -4_400_000_000


def test_repurchases_common_stock_none_when_absent(tmp_path):
    assert _annual(_import(_make_facts(), tmp_path)).repurchases_common_stock is None


def test_interest_income_maps_interest_income_other(tmp_path):
    """IBM-style filer: only the catch-all InterestIncomeOther is tagged."""
    facts = _make_facts()
    _add_concept(facts, "InterestIncomeOther", 966_000_000)
    assert _annual(_import(facts, tmp_path)).interest_income == 966_000_000


def test_interest_income_prefers_specific_concepts_over_catch_all(tmp_path):
    facts = _make_facts()
    _add_concept(facts, "InvestmentIncomeInterest", 500_000_000)
    _add_concept(facts, "InterestIncomeOther", 966_000_000)
    assert _annual(_import(facts, tmp_path)).interest_income == 500_000_000


def test_interest_income_none_when_absent(tmp_path):
    assert _annual(_import(_make_facts(), tmp_path)).interest_income is None


def test_rpo_mapped_when_present(tmp_path):
    facts = _make_facts()
    _add_concept(facts, "RevenueRemainingPerformanceObligation", 71_000_000_000)
    annual = _annual(_import(facts, tmp_path))
    assert annual.remaining_performance_obligation == 71_000_000_000
    # now canonical → must no longer sit in raw_extras
    assert "us-gaap/RevenueRemainingPerformanceObligation" not in annual.raw_extras


def test_rpo_none_when_absent(tmp_path):
    assert _annual(_import(_make_facts(), tmp_path)).remaining_performance_obligation is None


def test_segment_revenue_always_none_from_companyfacts(tmp_path):
    """companyfacts strips dimensional facts — see importer.py module docstring."""
    result = _import(_make_facts(), tmp_path)
    assert all(p.segment_revenue is None for p in result.periods)


def test_import_company_saves_when_save_true(tmp_path):
    facts = _make_facts()
    with patch("financial_analysis.importer.load_company_facts", return_value=facts), \
         patch("financial_analysis.importer.store.save") as mock_save:
        import_company("NVDA", save=True, cache_dir=tmp_path)
    mock_save.assert_called_once()
