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


def test_import_company_saves_when_save_true(tmp_path):
    facts = _make_facts()
    with patch("financial_analysis.importer.load_company_facts", return_value=facts), \
         patch("financial_analysis.importer.store.save") as mock_save:
        import_company("NVDA", save=True, cache_dir=tmp_path)
    mock_save.assert_called_once()
