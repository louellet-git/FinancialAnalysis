from datetime import date

from financial_analysis.crosscheck import (
    AGREE, DEFINITION, DIFFER, NO_VALUE_OURS, NO_VALUE_THEIRS, Summary, compare_period, match_fiscal_year,
)
from financial_analysis.schema import PeriodFinancials


def _period(**kwargs) -> PeriodFinancials:
    return PeriodFinancials(ticker="ACME", period_end=date(2024, 12, 31), form="10-K",
                            filed_date=date(2025, 2, 1), **kwargs)


def _by_variable(results):
    return {r.variable: r for r in results}


def test_values_that_agree():
    results = _by_variable(compare_period(
        _period(revenue=1000.0, operating_cash_flow=200.0),
        {"Total Revenue": 1004.0, "Operating Cash Flow": 200.0},
    ))
    assert results["revenue"].outcome == AGREE          # 0.4% difference, tolerance 1%
    assert results["operating_cash_flow"].outcome == AGREE


def test_values_that_differ():
    r = _by_variable(compare_period(_period(revenue=1000.0), {"Total Revenue": 1100.0}))["revenue"]
    assert r.outcome == DIFFER and r.ours == 1000.0 and r.theirs == 1100.0


def test_sign_convention_of_an_outflow_is_not_a_difference():
    r = _by_variable(compare_period(_period(capex=-50.0), {"Capital Expenditure": 50.0}))["capex"]
    assert r.outcome == AGREE


def test_capex_standard_is_equipment_and_capitalized_software():
    period = _period(capex=-300.0, capitalized_software=-70.0, capex_total=-370.0)
    assert _by_variable(compare_period(period, {"Capital Expenditure": -370.0}))["capex"].outcome == AGREE


def test_capex_that_agrees_only_with_a_different_definition():
    period = _period(capex=-300.0, capitalized_software=-70.0, capex_total=-370.0, purchases_of_intangibles=-30.0)
    wide = _by_variable(compare_period(period, {"Capital Expenditure": -400.0}))["capex"]
    assert wide.outcome == DEFINITION and wide.note == "with purchased intangibles"
    narrow = _by_variable(compare_period(period, {"Capital Expenditure": -300.0}))["capex"]
    assert narrow.outcome == DEFINITION and narrow.note == "equipment only"


def test_total_debt_is_compared_without_leases_first():
    period = _period(total_debt=1000.0)
    with_leases = {"Total Debt": 1200.0, "Capital Lease Obligations": 200.0}
    assert _by_variable(compare_period(period, with_leases))["total_debt"].outcome == AGREE
    only_total = {"Total Debt": 1000.0, "Capital Lease Obligations": 150.0}
    r = _by_variable(compare_period(period, only_total))["total_debt"]
    assert r.outcome == DEFINITION and "with leases" in r.note


def test_missing_values_are_reported_by_side():
    results = _by_variable(compare_period(_period(revenue=1000.0), {"Net Income": 90.0}))
    assert results["revenue"].outcome == NO_VALUE_THEIRS
    assert results["net_income"].outcome == NO_VALUE_OURS


def test_fiscal_year_match_accepts_a_52_week_year_end():
    ours = [date(2025, 9, 28), date(2026, 9, 27)]
    theirs = [date(2026, 9, 30), date(2025, 9, 30)]
    assert match_fiscal_year(ours, theirs) == (date(2026, 9, 27), date(2026, 9, 30))
    assert match_fiscal_year([date(2024, 12, 31)], [date(2025, 6, 30)]) is None


def test_summary_counts_and_agreement_share():
    summary = Summary()
    summary.add("A", compare_period(_period(revenue=1000.0), {"Total Revenue": 1000.0}))
    summary.add("B", compare_period(_period(revenue=1000.0), {"Total Revenue": 1500.0}))
    summary.add("C", compare_period(_period(), {"Total Revenue": 1500.0}))
    assert summary.companies == 3
    assert summary.counts["revenue"][AGREE] == 1 and summary.counts["revenue"][DIFFER] == 1
    assert summary.counts["revenue"][NO_VALUE_OURS] == 1
    assert summary.agreement("revenue") == 0.5       # companies with a value on both sides
    assert [t for t, _ in summary.examples["revenue"]] == ["B", "C"]
