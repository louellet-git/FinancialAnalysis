from financial_analysis.xbrl_tags import (
    INCOME_STATEMENT_TAGS,
    BALANCE_SHEET_TAGS,
    CASH_FLOW_TAGS,
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
    all_tags = {**INCOME_STATEMENT_TAGS, **BALANCE_SHEET_TAGS, **CASH_FLOW_TAGS}
    for field_name in all_tags:
        assert field_name in STATEMENT_MAP, f"{field_name} missing from STATEMENT_MAP"

def test_statement_map_values_are_valid():
    valid = {"income_statement", "balance_sheet", "cash_flow"}
    for field_name, statement in STATEMENT_MAP.items():
        assert statement in valid, f"{field_name} maps to unknown statement '{statement}'"

def test_no_duplicate_field_names_across_dicts():
    is_keys = set(INCOME_STATEMENT_TAGS)
    bs_keys = set(BALANCE_SHEET_TAGS)
    cf_keys = set(CASH_FLOW_TAGS)
    assert not (is_keys & bs_keys), f"IS/BS overlap: {is_keys & bs_keys}"
    assert not (is_keys & cf_keys), f"IS/CF overlap: {is_keys & cf_keys}"
    assert not (bs_keys & cf_keys), f"BS/CF overlap: {bs_keys & cf_keys}"
