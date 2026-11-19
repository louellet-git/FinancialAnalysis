from datetime import date

import pytest

from financial_analysis.mapping_audit import (
    audit_period,
    core_fill_rates,
    exclusion_reason,
    filer_template,
    identity_checks,
    total_debt_alternatives,
)
from financial_analysis.schema import PeriodFinancials
from financial_analysis.xbrl_tags import CORE_FIELDS


def _period(**kwargs) -> PeriodFinancials:
    sources = kwargs.pop("sources", None)
    p = PeriodFinancials(ticker="ACME", period_end=date(2024, 12, 31), form="10-K",
                         filed_date=date(2025, 2, 1), **kwargs)
    # By default each value came from a tag.
    p.sources = sources if sources is not None else {k: f"us-gaap/{k}" for k in kwargs}
    return p


def test_balanced_period_has_no_identity_failures():
    p = _period(revenue=100.0, total_assets=500.0, total_liabilities_and_equity=500.0,
                total_liabilities=300.0, equity=200.0)
    assert audit_period(p).identity_failures == []


def test_balance_sheet_gap_is_reported():
    p = _period(revenue=100.0, total_assets=500.0, total_liabilities=300.0, equity=150.0)
    failures = audit_period(p).identity_failures
    assert len(failures) == 1 and "total_liabilities + equity" in failures[0]


def test_temporary_equity_and_nci_close_the_balance_sheet():
    p = _period(revenue=100.0, total_assets=500.0, total_liabilities=300.0, equity=170.0,
                total_equity_incl_nci=190.0, temporary_equity=10.0)
    assert audit_period(p).identity_failures == []


def test_gross_profit_identity_only_when_all_three_come_from_tags():
    derived = _period(revenue=100.0, cogs_total=60.0, gross_profit=40.0, total_assets=1.0,
                      sources={"revenue": "us-gaap/Revenues", "cogs_total": "us-gaap/CostOfRevenue",
                               "gross_profit": "= revenue - cogs_total"})
    assert not any("gross_profit" in name for name, _ in identity_checks(derived))
    tagged = _period(revenue=100.0, cogs_total=60.0, gross_profit=45.0, total_assets=1.0)
    assert any("gross_profit" in f for f in audit_period(tagged).identity_failures)


def test_total_debt_estimates_that_disagree_are_reported():
    p = _period(revenue=100.0, total_assets=500.0, debt_current=10.9, long_term_debt=39.8,
                short_term_borrowings=1.0, long_term_debt_total=39.8, total_debt=50.7,
                sources={"total_debt": "= debt_current + long_term_debt"})
    alternatives = total_debt_alternatives(p)
    assert alternatives["debt_current + long_term_debt"] == pytest.approx(50.7)
    assert alternatives["short_term_borrowings + long_term_debt_total"] == pytest.approx(40.8)
    assert any("total_debt estimates are equal" in f for f in audit_period(p).identity_failures)


def test_total_debt_estimates_that_agree_are_not_reported():
    p = _period(revenue=100.0, total_assets=500.0, debt_current=11.9, long_term_debt=68.2,
                short_term_borrowings=2.0, long_term_debt_total=78.1, total_debt=80.1,
                sources={"total_debt": "= debt_current + long_term_debt"})
    assert audit_period(p).identity_failures == []


def test_core_missing_and_fill_rate():
    p = _period(revenue=100.0, total_assets=500.0)
    audit = audit_period(p)
    assert "total_debt" in audit.core_missing and "revenue" not in audit.core_missing
    assert audit.core_filled == 2
    rates = core_fill_rates([audit])
    assert rates["revenue"] == 1.0 and rates["capex"] == 0.0
    assert set(rates) == set(CORE_FIELDS)


def test_material_unmapped_tag_is_reported_and_excluded_tag_is_not():
    p = _period(revenue=100.0, total_assets=500.0)
    p.raw_extras = {
        "us-gaap/AssetRetirementObligation": 20.0,              # material, no variable
        "us-gaap/ComprehensiveIncomeNetOfTax": 50.0,            # excluded with a reason
        "us-gaap/SmallThing": 1.0,                              # not material
    }
    assert audit_period(p).unmapped == [("us-gaap/AssetRetirementObligation", 20.0)]
    assert exclusion_reason("us-gaap/ComprehensiveIncomeNetOfTax")
    assert exclusion_reason("us-gaap/AssetRetirementObligation") is None


def test_formula_values_are_counted():
    p = _period(revenue=100.0, cogs_total=60.0, gross_profit=40.0, total_assets=1.0,
                sources={"revenue": "us-gaap/Revenues", "cogs_total": "us-gaap/CostOfRevenue",
                         "gross_profit": "= revenue - cogs_total"})
    audit = audit_period(p)
    assert audit.filled == 4 and audit.from_formula == 1


def test_filer_template_uses_only_tags_of_the_given_year():
    old_bank_unit = {"facts": {"us-gaap": {
        "Deposits": {"units": {"USD": [{"end": "2015-12-31", "val": 1.0, "form": "10-K"}]}},
        "Assets": {"units": {"USD": [{"end": "2024-12-31", "val": 9.0, "form": "10-K"}]}},
    }}}
    assert filer_template(old_bank_unit) == "bank"
    assert filer_template(old_bank_unit, "2024-12-31") == "standard"
    assert filer_template({"facts": {"us-gaap": {}}}) == "standard"
