from financial_analysis.xbrl_tags import (
    INCOME_STATEMENT_TAGS,
    BALANCE_SHEET_TAGS,
    CASH_FLOW_TAGS,
    DISCLOSURE_TAGS,
    STATEMENT_MAP,
    UNIT_MAP,
    TagSpec,
)

def test_tagspec_defaults():
    spec = TagSpec(concepts=["us-gaap/Revenues"])
    assert spec.unit == "USD"
    assert spec.sign_flip is False

def test_all_dicts_have_non_empty_concept_lists():
    all_tags = {**INCOME_STATEMENT_TAGS, **BALANCE_SHEET_TAGS, **CASH_FLOW_TAGS}
    for field_name, spec in all_tags.items():
        assert spec.concepts, f"{field_name} has no concepts"

def test_all_concepts_have_namespace_prefix():
    all_tags = {**INCOME_STATEMENT_TAGS, **BALANCE_SHEET_TAGS, **CASH_FLOW_TAGS}
    for field_name, spec in all_tags.items():
        for concept in spec.concepts:
            assert "/" in concept, f"{field_name}: '{concept}' missing namespace prefix"

def test_capex_has_sign_flip():
    assert CASH_FLOW_TAGS["capex"].sign_flip is True

def test_repurchases_has_sign_flip():
    assert CASH_FLOW_TAGS["repurchases_common_stock"].sign_flip is True

def test_revenue_has_no_sign_flip():
    assert INCOME_STATEMENT_TAGS["revenue"].sign_flip is False

def test_eps_diluted_uses_per_share_unit():
    assert INCOME_STATEMENT_TAGS["eps_diluted"].unit == "USD/shares"

def test_shares_diluted_uses_shares_unit():
    assert INCOME_STATEMENT_TAGS["shares_diluted"].unit == "shares"

def test_statement_map_covers_all_tags():
    all_tags = {**INCOME_STATEMENT_TAGS, **BALANCE_SHEET_TAGS, **CASH_FLOW_TAGS, **DISCLOSURE_TAGS}
    for field_name in all_tags:
        assert field_name in STATEMENT_MAP, f"{field_name} missing from STATEMENT_MAP"

def test_statement_map_values_are_valid():
    valid = {"income_statement", "balance_sheet", "cash_flow", "disclosure"}
    for field_name, statement in STATEMENT_MAP.items():
        assert statement in valid, f"{field_name} maps to unknown statement '{statement}'"

def test_no_duplicate_field_names_across_dicts():
    is_keys = set(INCOME_STATEMENT_TAGS)
    bs_keys = set(BALANCE_SHEET_TAGS)
    cf_keys = set(CASH_FLOW_TAGS)
    assert not (is_keys & bs_keys), f"IS/BS overlap: {is_keys & bs_keys}"
    assert not (is_keys & cf_keys), f"IS/CF overlap: {is_keys & cf_keys}"
    assert not (bs_keys & cf_keys), f"BS/CF overlap: {bs_keys & cf_keys}"

def test_rpo_is_a_disclosure_tag():
    assert "remaining_performance_obligation" in DISCLOSURE_TAGS
    assert DISCLOSURE_TAGS["remaining_performance_obligation"].concepts == [
        "us-gaap/RevenueRemainingPerformanceObligation"
    ]
    assert STATEMENT_MAP["remaining_performance_obligation"] == "disclosure"


# --- registry rules ---

def test_schema_fields_match_the_registry():
    from dataclasses import fields
    from financial_analysis.schema import PeriodFinancials
    from financial_analysis.xbrl_tags import ALL_TAGS
    names = {f.name for f in fields(PeriodFinancials)}
    assert set(ALL_TAGS) <= names


def test_each_tag_is_an_alias_of_one_variable_only():
    from financial_analysis.xbrl_tags import ALL_TAGS
    seen = {}
    for name, spec in ALL_TAGS.items():
        for concept in spec.concepts:
            assert concept not in seen, f"{concept} is in {seen[concept]} and {name}"
            seen[concept] = name


def test_formula_terms_are_standard_variables():
    from financial_analysis.importer import _parse_formula
    from financial_analysis.xbrl_tags import ALL_TAGS
    for name, spec in ALL_TAGS.items():
        for formula in spec.formulas:
            for _, term, _ in _parse_formula(formula):
                assert term in ALL_TAGS, f"{name}: '{term}' is not a standard variable"
                assert term != name


def test_period_type_follows_the_statement():
    from financial_analysis.xbrl_tags import BALANCE_SHEET_TAGS, CASH_FLOW_TAGS, INCOME_STATEMENT_TAGS
    assert all(s.period == "duration" for s in INCOME_STATEMENT_TAGS.values())
    assert all(s.period == "duration" for s in CASH_FLOW_TAGS.values())
    assert all(s.period == "instant" for s in BALANCE_SHEET_TAGS.values())


def test_core_inputs_are_declared():
    from financial_analysis.xbrl_tags import CORE_FIELDS
    for name in ("revenue", "operating_cash_flow", "capex", "total_debt",
                 "cash_and_equivalents", "shares_diluted"):
        assert name in CORE_FIELDS


def test_long_term_debt_total_tag_is_not_an_alias_of_noncurrent_debt():
    from financial_analysis.xbrl_tags import BALANCE_SHEET_TAGS
    assert "us-gaap/LongTermDebt" not in BALANCE_SHEET_TAGS["long_term_debt"].concepts
    assert "us-gaap/LongTermDebt" in BALANCE_SHEET_TAGS["long_term_debt_total"].concepts


def test_exclusion_patterns_compile_and_have_reasons():
    import re
    from financial_analysis.xbrl_tags import EXCLUDED_CONCEPTS
    for pattern, reason in EXCLUDED_CONCEPTS:
        re.compile(pattern)
        assert reason
