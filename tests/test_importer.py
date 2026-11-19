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


# --- standard mapping: period length, aliases, formulas, sources ---

def _f(start, end, val, form="10-K", filed="2025-02-01"):
    entry = {"end": end, "val": val, "form": form, "filed": filed}
    if start:
        entry["start"] = start
    return entry


def _company(gaap: dict, tmp_path):
    facts = {"entityName": "ACME", "cik": 1, "facts": {"us-gaap": {
        name: {"units": {"USD": entries}} for name, entries in gaap.items()
    }}}
    with patch("financial_analysis.importer.load_company_facts", return_value=facts):
        return import_company("ACME", save=False, cache_dir=tmp_path)


_FY = ("2024-01-01", "2024-12-31")


def test_annual_row_is_a_full_fiscal_year(tmp_path):
    company = _company({"Revenues": [
        _f(*_FY, 100.0),
        _f("2024-10-01", "2024-12-31", 27.0, filed="2025-02-02"),   # Q4 in the 10-K
        _f("2024-07-01", "2024-09-29", 26.0),                        # Q3 in the 10-K
    ]}, tmp_path)
    annual = [p for p in company.periods if p.form == "10-K"]
    assert [p.period_end for p in annual] == [date(2024, 12, 31)]
    assert annual[0].revenue == 100.0


def test_same_variable_from_different_tags(tmp_path):
    # Two filers, two tag names, one standard variable.
    a = _company({"Revenues": [_f(*_FY, 100.0)],
                  "InterestExpense": [_f(*_FY, 5.0)]}, tmp_path)
    b = _company({"Revenues": [_f(*_FY, 100.0)],
                  "InterestExpenseNonoperating": [_f(*_FY, 7.0)]}, tmp_path)
    assert a.periods[0].interest_expense == 5.0
    assert b.periods[0].interest_expense == 7.0
    assert a.periods[0].sources["interest_expense"] == "us-gaap/InterestExpense"
    assert b.periods[0].sources["interest_expense"] == "us-gaap/InterestExpenseNonoperating"


def test_formula_fills_a_variable_that_has_no_tag(tmp_path):
    company = _company({"Revenues": [_f(*_FY, 100.0)],
                        "CostOfRevenue": [_f(*_FY, 60.0)]}, tmp_path)
    p = company.periods[0]
    assert p.gross_profit == 40.0
    assert p.sources["gross_profit"] == "= revenue - cogs_total"


def test_tag_wins_over_formula(tmp_path):
    company = _company({"Revenues": [_f(*_FY, 100.0)],
                        "CostOfRevenue": [_f(*_FY, 60.0)],
                        "GrossProfit": [_f(*_FY, 41.0)]}, tmp_path)
    p = company.periods[0]
    assert p.gross_profit == 41.0
    assert p.sources["gross_profit"] == "us-gaap/GrossProfit"


def test_total_debt_from_parts(tmp_path):
    company = _company({"Revenues": [_f(*_FY, 100.0)],
                        "CommercialPaper": [_f(None, "2024-12-31", 2.0)],
                        "LongTermDebtCurrent": [_f(None, "2024-12-31", 10.0)],
                        "LongTermDebtNoncurrent": [_f(None, "2024-12-31", 68.0)]}, tmp_path)
    p = company.periods[0]
    assert p.debt_current == 12.0
    assert p.long_term_debt == 68.0
    assert p.long_term_debt_total == 78.0
    assert p.total_debt == 80.0
    assert p.sources["total_debt"].startswith("= debt_current + long_term_debt")


def test_long_term_debt_tag_with_current_part_is_not_the_noncurrent_variable(tmp_path):
    # us-gaap/LongTermDebt includes the current part.
    company = _company({"Revenues": [_f(*_FY, 100.0)],
                        "LongTermDebt": [_f(None, "2024-12-31", 78.0)],
                        "LongTermDebtCurrent": [_f(None, "2024-12-31", 10.0)]}, tmp_path)
    p = company.periods[0]
    assert p.long_term_debt_total == 78.0
    assert p.long_term_debt == 68.0
    assert p.total_debt == 78.0


def test_two_tags_with_the_same_value_count_one_time(tmp_path):
    # One short-term debt item with two tags (the IBM case).
    company = _company({"Revenues": [_f(*_FY, 100.0)],
                        "ShortTermBorrowings": [_f(None, "2024-12-31", 6.4)],
                        "LongTermDebtAndCapitalLeaseObligationsCurrent": [_f(None, "2024-12-31", 6.4)],
                        "LongTermDebtNoncurrent": [_f(None, "2024-12-31", 54.8)]}, tmp_path)
    p = company.periods[0]
    assert p.debt_current == 6.4
    assert "counted one time" in p.sources["debt_current"]
    assert p.total_debt == pytest.approx(61.2)


def test_optional_formula_term_is_recorded_when_absent(tmp_path):
    company = _company({"Revenues": [_f(*_FY, 100.0)],
                        "Depreciation": [_f(*_FY, 8.0)]}, tmp_path)
    p = company.periods[0]
    assert p.da_total == 8.0
    assert "no value: opex_amortization" in p.sources["da_total"]


def test_quarterly_cash_flow_is_one_quarter_not_year_to_date(tmp_path):
    company = _company({
        "Revenues": [_f("2024-01-01", "2024-03-31", 24.0, "10-Q"),
                     _f("2024-04-01", "2024-06-30", 26.0, "10-Q")],
        "NetCashProvidedByUsedInOperatingActivities": [
            _f("2024-01-01", "2024-03-31", 5.0, "10-Q"),
            _f("2024-01-01", "2024-06-30", 12.0, "10-Q")],
    }, tmp_path)
    q2 = [p for p in company.periods if p.period_end == date(2024, 6, 30)][0]
    assert q2.operating_cash_flow == 7.0
    assert "year-to-date difference" in q2.sources["operating_cash_flow"]


def test_n_periods_counts_fiscal_years(tmp_path):
    company_facts = {"Revenues": [_f(f"{y}-01-01", f"{y}-12-31", float(y)) for y in range(2010, 2025)]}
    facts = {"entityName": "ACME", "cik": 1, "facts": {"us-gaap": {
        name: {"units": {"USD": entries}} for name, entries in company_facts.items()}}}
    with patch("financial_analysis.importer.load_company_facts", return_value=facts):
        company = import_company("ACME", n_periods=5, save=False, cache_dir=tmp_path)
    assert [p.period_end.year for p in company.periods] == [2020, 2021, 2022, 2023, 2024]


def test_filed_date_is_a_filing_date_when_the_period_has_no_revenue(tmp_path):
    company = _company({"NetIncomeLoss": [_f(*_FY, 9.0, filed="2025-02-20")]}, tmp_path)
    p = company.periods[0]
    assert p.revenue is None
    assert p.filed_date == date(2025, 2, 20)   # not the period end


def test_unmapped_tag_stays_in_raw_extras_with_the_annual_value(tmp_path):
    company = _company({"Revenues": [_f(*_FY, 100.0)],
                        "SomeCompanyDetail": [_f(*_FY, 3.0),
                                              _f("2024-10-01", "2024-12-31", 1.0, filed="2025-03-01")]},
                       tmp_path)
    assert company.periods[0].raw_extras == {"us-gaap/SomeCompanyDetail": 3.0}


# --- share count scale ---

def _shares_company(tmp_path, shares, eps=11.95, income=8_563_000_000.0, extra=None):
    facts = {"entityName": "ACME", "cik": 1, "facts": {"us-gaap": {
        "Revenues": {"units": {"USD": [_f(*_FY, 26_000_000_000.0)]}},
        "NetIncomeLoss": {"units": {"USD": [_f(*_FY, income)]}},
        "EarningsPerShareDiluted": {"units": {"USD/shares": [_f(*_FY, eps)]}},
        "WeightedAverageNumberOfDilutedSharesOutstanding": {"units": {"shares": [_f(*_FY, shares)]}},
        **(extra or {}),
    }}}
    with patch("financial_analysis.importer.load_company_facts", return_value=facts):
        return import_company("ACME", save=False, cache_dir=tmp_path).periods[0]


def test_share_count_in_millions_is_corrected(tmp_path):
    # The filer tags 716.4 for 716.4 million shares (the McDonald's case).
    p = _shares_company(tmp_path, 716.4)
    assert p.shares_diluted == pytest.approx(716_400_000.0)
    assert "x1,000,000" in p.sources["shares_diluted"]


def test_share_count_in_thousands_is_corrected(tmp_path):
    p = _shares_company(tmp_path, 716_400.0)
    assert p.shares_diluted == pytest.approx(716_400_000.0)
    assert "x1,000:" in p.sources["shares_diluted"]


def test_correct_share_count_does_not_change(tmp_path):
    p = _shares_company(tmp_path, 716_400_000.0)
    assert p.shares_diluted == 716_400_000.0
    assert p.sources["shares_diluted"] == "us-gaap/WeightedAverageNumberOfDilutedSharesOutstanding"


def test_share_count_that_differs_for_a_different_reason_does_not_change(tmp_path):
    # A stock split after the period: a factor of 4, not 1,000. No correction.
    p = _shares_company(tmp_path, 179_100_000.0)
    assert p.shares_diluted == 179_100_000.0


def test_small_eps_gives_no_correction(tmp_path):
    p = _shares_company(tmp_path, 500.0, eps=0.01, income=5_000_000.0)
    assert p.shares_diluted == 500.0


def test_balance_sheet_share_count_gets_the_same_scale(tmp_path):
    extra = {"CommonStockSharesOutstanding": {"units": {"shares": [_f(None, "2024-12-31", 714.9)]}}}
    p = _shares_company(tmp_path, 716.4, extra=extra)
    assert p.shares_outstanding == pytest.approx(714_900_000.0)
    assert "same scale" in p.sources["shares_outstanding"]


def test_share_check_uses_net_income_for_the_common_shareholders(tmp_path):
    # Preferred dividends: net income 2,235 but 1,890 for the common shareholders.
    extra = {"NetIncomeLossAvailableToCommonStockholdersDiluted": {"units": {"USD": [_f(*_FY, 1_890_000_000.0)]}}}
    p = _shares_company(tmp_path, 762.3, eps=2.48, income=2_235_000_000.0, extra=extra)
    assert p.shares_diluted == pytest.approx(762_300_000.0)
