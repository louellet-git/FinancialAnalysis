"""Filing-level XBRL: synthetic MetaLinks data and a synthetic instance document."""
import json
from datetime import date

import pytest

from financial_analysis.filing import (
    FilingError, build_statements, capex_lines, debt_lines, depreciation_lines, estimates,
    intangible_purchase_lines, latest_annual_filing, load_filing, parse_instance,
)

GAAP = "http://fasb.org/us-gaap/2025"
ACME = "http://www.acme.com/20241231"
BS, CF, IS = "http://acme/role/BalanceSheet", "http://acme/role/CashFlow", "http://acme/role/Income"


def _tag(prefix, local, label, role=None, parent=None, weight=1.0, order=1.0):
    info = {"nsuri": GAAP if prefix == "us-gaap" else ACME, "localname": local,
            "lang": {"en-us": {"role": {"terseLabel": label}}}}
    if role:
        arc = {"parentTag": parent, "weight": weight if parent else None, "order": order if parent else None}
        if parent is None:
            arc["root"] = True
        info["calculation"] = {role: arc}
    return f"{prefix}_{local}", info


def _meta() -> dict:
    tags = dict([
        # balance sheet
        _tag("us-gaap", "LiabilitiesAndStockholdersEquity", "Total liabilities and equity", BS),
        _tag("us-gaap", "Liabilities", "Total liabilities", BS, "us-gaap_LiabilitiesAndStockholdersEquity", 1.0, 1),
        _tag("us-gaap", "StockholdersEquity", "Equity", BS, "us-gaap_LiabilitiesAndStockholdersEquity", 1.0, 2),
        _tag("us-gaap", "ShortTermBorrowings", "Short-term debt", BS, "us-gaap_Liabilities", 1.0, 1),
        _tag("us-gaap", "LongTermDebtNoncurrent", "Long-term debt", BS, "us-gaap_Liabilities", 1.0, 2),
        _tag("us-gaap", "OperatingLeaseLiabilityNoncurrent", "Lease liabilities", BS, "us-gaap_Liabilities", 1.0, 3),
        _tag("acme", "OtherObligations", "Other obligations", BS, "us-gaap_Liabilities", 1.0, 4),
        # cash flow
        _tag("us-gaap", "CashPeriodIncreaseDecrease", "Change in cash", CF),
        _tag("us-gaap", "NetCashProvidedByUsedInOperatingActivities", "Operating cash flow", CF, "us-gaap_CashPeriodIncreaseDecrease", 1.0, 1),
        _tag("us-gaap", "NetCashProvidedByUsedInInvestingActivities", "Investing cash flow", CF, "us-gaap_CashPeriodIncreaseDecrease", 1.0, 2),
        _tag("us-gaap", "NetIncomeLoss", "Net income", CF, "us-gaap_NetCashProvidedByUsedInOperatingActivities", 1.0, 1),
        _tag("us-gaap", "Depreciation", "Depreciation", CF, "us-gaap_NetCashProvidedByUsedInOperatingActivities", 1.0, 2),
        _tag("us-gaap", "AdjustmentForAmortization", "Amortization", CF, "us-gaap_NetCashProvidedByUsedInOperatingActivities", 1.0, 3),
        _tag("us-gaap", "AmortizationOfDebtDiscountPremium", "Debt discount", CF, "us-gaap_NetCashProvidedByUsedInOperatingActivities", 1.0, 4),
        _tag("us-gaap", "PaymentsToAcquirePropertyPlantAndEquipment", "Purchases of equipment", CF, "us-gaap_NetCashProvidedByUsedInInvestingActivities", -1.0, 1),
        _tag("acme", "PaymentsToAcquireAndDevelopSoftware", "Software", CF, "us-gaap_NetCashProvidedByUsedInInvestingActivities", -1.0, 2),
        _tag("us-gaap", "PaymentsToAcquireIntangibleAssets", "Licenses", CF, "us-gaap_NetCashProvidedByUsedInInvestingActivities", -1.0, 3),
        _tag("us-gaap", "ProceedsFromSaleOfPropertyPlantAndEquipment", "Sale of equipment", CF, "us-gaap_NetCashProvidedByUsedInInvestingActivities", 1.0, 4),
        _tag("us-gaap", "PaymentsToAcquireBusinessesNetOfCashAcquired", "Acquisitions", CF, "us-gaap_NetCashProvidedByUsedInInvestingActivities", -1.0, 5),
        # income statement
        _tag("us-gaap", "GrossProfit", "Gross profit", IS),
        _tag("us-gaap", "Revenues", "Revenue", IS, "us-gaap_GrossProfit", 1.0, 1),
        _tag("us-gaap", "CostOfRevenue", "Cost of revenue", IS, "us-gaap_GrossProfit", -1.0, 2),
    ])
    reports = {
        "R1": {"groupType": "document", "subGroupType": "", "longName": "0001 - Document - Cover", "role": "http://acme/role/Cover"},
        "R2": {"groupType": "statement", "subGroupType": "", "longName": "0002 - Statement - Consolidated Statements of Income", "role": IS},
        "R3": {"groupType": "statement", "subGroupType": "", "longName": "0003 - Statement - Consolidated Statements of Comprehensive Income", "role": "http://acme/role/OCI"},
        "R4": {"groupType": "statement", "subGroupType": "", "longName": "0004 - Statement - Consolidated Balance Sheets", "role": BS},
        "R5": {"groupType": "statement", "subGroupType": "parenthetical", "longName": "0005 - Statement - Consolidated Balance Sheets (Parenthetical)", "role": BS + "P"},
        "R6": {"groupType": "statement", "subGroupType": "", "longName": "0006 - Statement - Consolidated Statements of Cash Flows", "role": CF},
    }
    return {"instance": {"acme-20241231.htm": {"report": reports, "tag": tags}}}


def _instance() -> bytes:
    def fact(prefix, local, ctx, value):
        return f'<{prefix}:{local} contextRef="{ctx}" unitRef="usd" decimals="-6">{value}</{prefix}:{local}>'
    facts = "".join([
        fact("us-gaap", "LiabilitiesAndStockholdersEquity", "now", 1500), fact("us-gaap", "Liabilities", "now", 600),
        fact("us-gaap", "StockholdersEquity", "now", 900), fact("us-gaap", "ShortTermBorrowings", "now", 64),
        fact("us-gaap", "LongTermDebtNoncurrent", "now", 400), fact("us-gaap", "OperatingLeaseLiabilityNoncurrent", "now", 36),
        fact("acme", "OtherObligations", "now", 100),
        fact("us-gaap", "LongTermDebtNoncurrent", "before", 380),            # the prior year: not used
        fact("us-gaap", "NetCashProvidedByUsedInOperatingActivities", "fy", 200),
        fact("us-gaap", "NetCashProvidedByUsedInInvestingActivities", "fy", -95),
        fact("us-gaap", "NetIncomeLoss", "fy", 100), fact("us-gaap", "Depreciation", "fy", 40),
        fact("us-gaap", "AdjustmentForAmortization", "fy", 25), fact("us-gaap", "AmortizationOfDebtDiscountPremium", "fy", 2),
        fact("us-gaap", "PaymentsToAcquirePropertyPlantAndEquipment", "fy", 50),
        fact("acme", "PaymentsToAcquireAndDevelopSoftware", "fy", 30),
        fact("us-gaap", "PaymentsToAcquireIntangibleAssets", "fy", 10),
        fact("us-gaap", "ProceedsFromSaleOfPropertyPlantAndEquipment", "fy", 5),
        fact("us-gaap", "PaymentsToAcquireBusinessesNetOfCashAcquired", "fy", 10),
        fact("us-gaap", "Revenues", "fy", 1000), fact("us-gaap", "Revenues", "q4", 270),      # a quarter: not used
        fact("us-gaap", "Revenues", "segment", 600),                                          # a segment: not used
        fact("us-gaap", "CostOfRevenue", "fy", 560), fact("us-gaap", "GrossProfit", "fy", 440),
    ])
    entity = '<entity><identifier scheme="http://www.sec.gov/CIK">1</identifier></entity>'
    seg = ('<entity><identifier scheme="http://www.sec.gov/CIK">1</identifier><segment>'
           '<xbrldi:explicitMember dimension="us-gaap:StatementBusinessSegmentsAxis">acme:CloudMember</xbrldi:explicitMember>'
           '</segment></entity>')
    return (f'<?xml version="1.0" encoding="utf-8"?><xbrl xmlns="http://www.xbrl.org/2003/instance" '
            f'xmlns:us-gaap="{GAAP}" xmlns:acme="{ACME}" xmlns:dei="http://xbrl.sec.gov/dei/2025" '
            f'xmlns:xbrldi="http://xbrl.org/2006/xbrldi">'
            f'<context id="fy">{entity}<period><startDate>2024-01-01</startDate><endDate>2024-12-31</endDate></period></context>'
            f'<context id="q4">{entity}<period><startDate>2024-10-01</startDate><endDate>2024-12-31</endDate></period></context>'
            f'<context id="segment">{seg}<period><startDate>2024-01-01</startDate><endDate>2024-12-31</endDate></period></context>'
            f'<context id="now">{entity}<period><instant>2024-12-31</instant></period></context>'
            f'<context id="before">{entity}<period><instant>2023-12-31</instant></period></context>'
            f'<dei:DocumentPeriodEndDate contextRef="fy">2024-12-31</dei:DocumentPeriodEndDate>'
            f'{facts}</xbrl>').encode("utf-8")


@pytest.fixture
def filing(tmp_path):
    folder = tmp_path / "ACME" / "filings" / "0000000001-25-000001"
    folder.mkdir(parents=True)
    (folder / "MetaLinks.json").write_text(json.dumps(_meta()), encoding="utf-8")
    (folder / "acme-20241231_htm.xml").write_bytes(_instance())
    return load_filing("ACME", folder)


def test_instance_gives_the_consolidated_values_of_the_fiscal_year():
    instance = next(iter(_meta()["instance"].values()))
    tag_of = {(i["nsuri"], i["localname"]): t for t, i in instance["tag"].items()}
    values, fiscal_year_end, total, with_segment = parse_instance(_instance(), tag_of)
    assert fiscal_year_end == date(2024, 12, 31)
    assert values["us-gaap_Revenues"] == 1000.0                 # not the quarter, not the segment
    assert values["us-gaap_LongTermDebtNoncurrent"] == 400.0    # not the prior year
    assert values["acme_PaymentsToAcquireAndDevelopSoftware"] == 30.0
    assert (total, with_segment) == (24, 1)


def test_the_three_statements_are_found_and_others_are_not(filing):
    assert [s.kind for s in filing.statements] == ["income", "balance", "cash"]
    assert filing.fiscal_year_end == date(2024, 12, 31)
    assert filing.statement("balance").name == "Consolidated Balance Sheets"


def test_tree_has_parents_weights_and_values(filing):
    income = filing.statement("income")
    root = income.roots[0]
    assert root.tag == "us-gaap_GrossProfit" and root.value == 440.0
    assert [(c.tag, c.weight, c.value) for c in root.children] == [
        ("us-gaap_Revenues", 1.0, 1000.0), ("us-gaap_CostOfRevenue", -1.0, 560.0)]
    assert root.parts_sum() == 440.0 and root.sum_agrees() is True


def test_line_shows_its_standard_variable_or_that_it_is_a_company_tag(filing):
    balance = filing.statement("balance")
    assert balance.find("us-gaap_LongTermDebtNoncurrent").variable == "long_term_debt"
    other = balance.find("acme_OtherObligations")
    assert other.company_tag and other.variable is None
    assert not balance.find("us-gaap_Liabilities").company_tag


def test_parts_that_do_not_sum_are_found(filing):
    liabilities = filing.statement("balance").find("us-gaap_Liabilities")
    assert liabilities.parts_sum() == 600.0 and liabilities.sum_agrees() is True
    liabilities.children[0].value = 164.0
    assert liabilities.sum_agrees() is False


def test_capex_lines_include_a_company_tag_and_no_proceeds_and_no_acquisitions(filing):
    assert [line.tag for line in capex_lines(filing)] == [
        "us-gaap_PaymentsToAcquirePropertyPlantAndEquipment", "acme_PaymentsToAcquireAndDevelopSoftware"]
    assert [line.tag for line in intangible_purchase_lines(filing)] == ["us-gaap_PaymentsToAcquireIntangibleAssets"]


def test_depreciation_lines_do_not_include_debt_discount(filing):
    assert [line.tag for line in depreciation_lines(filing)] == ["us-gaap_Depreciation", "us-gaap_AdjustmentForAmortization"]


def test_debt_lines_do_not_include_operating_leases(filing):
    assert [line.tag for line in debt_lines(filing)] == ["us-gaap_ShortTermBorrowings", "us-gaap_LongTermDebtNoncurrent"]


def test_estimates_from_the_filing_lines(filing):
    e = estimates(filing)
    assert e.capex == -80.0                       # equipment 50 and software 30, cash out
    assert e.capex_with_intangibles == -90.0
    assert e.da_total == 65.0
    assert e.total_debt == 464.0


def test_latest_annual_filing_is_the_latest_10k():
    facts = {"cik": 7, "facts": {"us-gaap": {"Assets": {"units": {"USD": [
        {"end": "2023-12-31", "val": 1, "form": "10-K", "filed": "2024-02-01", "accn": "0000000007-24-000001"},
        {"end": "2024-12-31", "val": 1, "form": "10-K", "filed": "2025-02-01", "accn": "0000000007-25-000001"},
        {"end": "2025-03-31", "val": 1, "form": "10-Q", "filed": "2025-05-01", "accn": "0000000007-25-000009"},
    ]}}}}}
    assert latest_annual_filing(facts) == ("7", "0000000007-25-000001")
    with pytest.raises(FilingError):
        latest_annual_filing({"cik": 7, "facts": {}})


def test_folder_with_no_files_gives_an_error(tmp_path):
    with pytest.raises(FilingError):
        load_filing("ACME", tmp_path)
