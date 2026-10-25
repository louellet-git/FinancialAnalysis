---
id: DOC-PLAN-FINANCIAL-STATEMENT-AUTOMATION
type: plan
item: 10.16 Development plan
status: superseded
date: 2026-06-05
scope: Historical implementation plan for the analyze command and xlsx export; superseded by the shipped code.
---
# Financial Statement Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a single `python -m scripts.analyze TICKER` command that fetches, caches, and exports full financial statements for any SEC us-gaap filer to a four-tab xlsx file, eliminating repeated API calls and throwaway scripts.

**Architecture:** `cache.py` wraps `fetch_company_facts()` with disk persistence. `xbrl_tags.py` maps canonical field names to ordered XBRL concept fallback lists with sign normalization. `importer.py` orchestrates cache → tag resolution → schema population → reconciliation → store. `scripts/analyze.py` loads from store and writes a four-tab xlsx (Income Statement, Balance Sheet, Cash Flow, Raw) in long format.

**Tech Stack:** Python 3.12, dataclasses, openpyxl, numpy, pytest, unittest.mock, urllib

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `requirements.txt` | Create | Pin openpyxl + numpy |
| `financial_analysis/xbrl_api.py` | Modify | Add `extract_series()` — generic form/unit extractor |
| `financial_analysis/xbrl_tags.py` | Create | `TagSpec` dataclass, IS/BS/CF tag dicts, `STATEMENT_MAP`, `UNIT_MAP` |
| `financial_analysis/schema.py` | Modify | Add ~33 new `Optional[float]` fields + `raw_extras: dict` to `PeriodFinancials` |
| `financial_analysis/store.py` | Modify | Replace hardcoded serialize/deserialize with field-generic approach |
| `financial_analysis/cache.py` | Create | `load_company_facts()` — disk cache with corruption fallback |
| `financial_analysis/importer.py` | Create | `UnsupportedFilingError` + `import_company()` |
| `scripts/import_financials.py` | Create | CLI: import and save to `data/processed/` |
| `scripts/analyze.py` | Create | CLI: load from store, write four-tab xlsx |
| `tests/test_xbrl_api.py` | Modify | Add tests for `extract_series()` |
| `tests/test_cache.py` | Create | Cache hit, miss, corruption fallback |
| `tests/test_importer.py` | Create | Mocked HTTP, IFRS detection, quarterly periods |
| `tests/test_analyze.py` | Create | Flatten output, four tabs, empty-cell nulls |

---

## Task 1: Create requirements.txt

**Files:**
- Create: `requirements.txt`

- [ ] **Step 1: Create the file**

```
numpy>=1.26
openpyxl>=3.1
```

- [ ] **Step 2: Verify install**

```
pip install -r requirements.txt
```

Expected: both packages install without error.

- [ ] **Step 3: Commit**

```bash
git add requirements.txt
git commit -m "chore: add requirements.txt with openpyxl and numpy"
```

---

## Task 2: Add `extract_series()` to `xbrl_api.py`

**Files:**
- Modify: `financial_analysis/xbrl_api.py`
- Modify: `tests/test_xbrl_api.py`

The existing `extract_annual_series` and `extract_quarterly_series` are USD-only. EPS and share-count concepts use `"USD/shares"` and `"shares"` units. Add a general extractor.

- [ ] **Step 1: Write the failing tests**

Add to `tests/test_xbrl_api.py`:

```python
from financial_analysis.xbrl_api import extract_series

def _make_facts(concept_name: str, unit: str, entries: list) -> dict:
    return {"facts": {"us-gaap": {concept_name: {"units": {unit: entries}}}}}

def test_extract_series_annual_usd():
    facts = _make_facts("Revenues", "USD", [
        {"end": "2024-01-28", "val": 60920000000, "form": "10-K", "filed": "2024-02-21"},
        {"end": "2023-01-29", "val": 26970000000, "form": "10-K", "filed": "2023-02-24"},
        {"end": "2024-04-28", "val": 22100000000, "form": "10-Q", "filed": "2024-05-22"},
    ])
    result = extract_series(facts, "us-gaap/Revenues", form_types=["10-K"], unit="USD", n=10)
    assert len(result) == 2
    assert all(e["form"] == "10-K" for e in result)
    assert result[0]["end"] == "2023-01-29"  # sorted ascending

def test_extract_series_quarterly():
    facts = _make_facts("Revenues", "USD", [
        {"end": "2024-04-28", "val": 22100000000, "form": "10-Q", "filed": "2024-05-22"},
        {"end": "2024-01-28", "val": 60920000000, "form": "10-K", "filed": "2024-02-21"},
    ])
    result = extract_series(facts, "us-gaap/Revenues", form_types=["10-Q"], unit="USD", n=10)
    assert len(result) == 1
    assert result[0]["form"] == "10-Q"

def test_extract_series_non_usd_unit():
    facts = _make_facts("EarningsPerShareDiluted", "USD/shares", [
        {"end": "2024-01-28", "val": 1.19, "form": "10-K", "filed": "2024-02-21"},
    ])
    result = extract_series(facts, "us-gaap/EarningsPerShareDiluted", form_types=["10-K"], unit="USD/shares", n=10)
    assert len(result) == 1
    assert result[0]["val"] == 1.19

def test_extract_series_missing_concept_returns_empty():
    facts = {"facts": {"us-gaap": {}}}
    result = extract_series(facts, "us-gaap/NonExistent", form_types=["10-K"], unit="USD", n=10)
    assert result == []

def test_extract_series_respects_n_limit():
    entries = [
        {"end": f"202{i}-01-28", "val": float(i), "form": "10-K", "filed": f"202{i}-02-01"}
        for i in range(8)
    ]
    facts = _make_facts("Revenues", "USD", entries)
    result = extract_series(facts, "us-gaap/Revenues", form_types=["10-K"], unit="USD", n=3)
    assert len(result) == 3
    assert result[-1]["end"] == "2027-01-28"  # most recent 3

def test_extract_series_deduplicates_by_latest_filed():
    facts = _make_facts("Revenues", "USD", [
        {"end": "2024-01-28", "val": 60920000000, "form": "10-K", "filed": "2024-02-21"},
        {"end": "2024-01-28", "val": 61000000000, "form": "10-K", "filed": "2024-03-01"},
    ])
    result = extract_series(facts, "us-gaap/Revenues", form_types=["10-K"], unit="USD", n=10)
    assert len(result) == 1
    assert result[0]["val"] == 61000000000
```

- [ ] **Step 2: Run tests to verify they fail**

```
pytest tests/test_xbrl_api.py -k "extract_series" -v
```

Expected: `ImportError` or `AttributeError` — `extract_series` not defined.

- [ ] **Step 3: Add `extract_series` to `xbrl_api.py`**

Add after `_deduplicate`:

```python
def extract_series(
    facts: Dict[str, Any],
    concept: str,
    form_types: List[str],
    unit: str = "USD",
    n: int = 10,
) -> List[Dict[str, Any]]:
    parts = concept.split("/", 1)
    if len(parts) != 2:
        return []
    namespace, name = parts
    try:
        entries = facts["facts"][namespace][name]["units"].get(unit, [])
    except KeyError:
        return []
    filtered = [e for e in entries if e.get("form") in form_types]
    return _deduplicate(filtered)[-n:]
```

- [ ] **Step 4: Run tests to verify they pass**

```
pytest tests/test_xbrl_api.py -k "extract_series" -v
```

Expected: 6 PASSED.

- [ ] **Step 5: Run full test suite to check for regressions**

```
pytest tests/ -v
```

Expected: all previously passing tests still pass.

- [ ] **Step 6: Commit**

```bash
git add financial_analysis/xbrl_api.py tests/test_xbrl_api.py
git commit -m "feat: add extract_series() to xbrl_api — generic form/unit extractor"
```

---

## Task 3: Create `xbrl_tags.py`

**Files:**
- Create: `financial_analysis/xbrl_tags.py`
- Create: `tests/test_xbrl_tags.py`

This file is the concept fallback registry. It is the single source of truth for which XBRL concepts map to which canonical field names, what unit each uses, and whether the sign needs flipping.

- [ ] **Step 1: Write the failing tests**

Create `tests/test_xbrl_tags.py`:

```python
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
```

- [ ] **Step 2: Run to verify they fail**

```
pytest tests/test_xbrl_tags.py -v
```

Expected: `ModuleNotFoundError` — file does not exist.

- [ ] **Step 3: Create `financial_analysis/xbrl_tags.py`**

```python
from __future__ import annotations
from dataclasses import dataclass


@dataclass
class TagSpec:
    concepts: list[str]
    unit: str = "USD"
    sign_flip: bool = False


INCOME_STATEMENT_TAGS: dict[str, TagSpec] = {
    "revenue": TagSpec([
        "us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax",
        "us-gaap/Revenues",
    ]),
    "cogs_total": TagSpec([
        "us-gaap/CostOfRevenue",
        "us-gaap/CostOfGoodsAndServicesSold",
    ]),
    "gross_profit": TagSpec(["us-gaap/GrossProfit"]),
    "rd_expense": TagSpec(["us-gaap/ResearchAndDevelopmentExpense"]),
    "sm_expense": TagSpec(["us-gaap/SellingAndMarketingExpense"]),
    "ga_expense": TagSpec(["us-gaap/GeneralAndAdministrativeExpense"]),
    "sga_expense": TagSpec(["us-gaap/SellingGeneralAndAdministrativeExpense"]),
    "opex_amortization": TagSpec(["us-gaap/AmortizationOfIntangibleAssets"]),
    "opex_restructuring": TagSpec(["us-gaap/RestructuringCharges"]),
    "operating_income": TagSpec(["us-gaap/OperatingIncomeLoss"]),
    "interest_income": TagSpec([
        "us-gaap/InterestIncomeNonoperating",
        "us-gaap/InvestmentIncomeInterest",
    ]),
    "interest_expense": TagSpec(["us-gaap/InterestExpense"]),
    "gains_losses_on_investments": TagSpec(["us-gaap/GainLossOnInvestments"]),
    "other_nonop_income": TagSpec(["us-gaap/OtherNonoperatingIncomeExpense"]),
    "income_before_tax": TagSpec([
        "us-gaap/IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest",
        "us-gaap/IncomeLossFromContinuingOperationsBeforeIncomeTaxesMinorityInterestAndIncomeLossFromEquityMethodInvestments",
    ]),
    "income_tax_expense": TagSpec(["us-gaap/IncomeTaxExpenseBenefit"]),
    "net_income": TagSpec(["us-gaap/NetIncomeLoss"]),
    "da_total": TagSpec(["us-gaap/DepreciationDepletionAndAmortization"]),
    "sbc_total": TagSpec(["us-gaap/ShareBasedCompensation"]),
    "eps_basic": TagSpec(["us-gaap/EarningsPerShareBasic"], unit="USD/shares"),
    "eps_diluted": TagSpec(["us-gaap/EarningsPerShareDiluted"], unit="USD/shares"),
    "shares_basic": TagSpec(
        ["us-gaap/WeightedAverageNumberOfSharesOutstandingBasic"], unit="shares"
    ),
    "shares_diluted": TagSpec(
        ["us-gaap/WeightedAverageNumberOfDilutedSharesOutstanding"], unit="shares"
    ),
}

BALANCE_SHEET_TAGS: dict[str, TagSpec] = {
    "cash_and_equivalents": TagSpec(["us-gaap/CashAndCashEquivalentsAtCarryingValue"]),
    "short_term_investments": TagSpec([
        "us-gaap/ShortTermInvestments",
        "us-gaap/AvailableForSaleSecuritiesCurrent",
    ]),
    "accounts_receivable_net": TagSpec(["us-gaap/AccountsReceivableNetCurrent"]),
    "inventory": TagSpec(["us-gaap/InventoryNet"]),
    "total_current_assets": TagSpec(["us-gaap/AssetsCurrent"]),
    "ppe_net": TagSpec(["us-gaap/PropertyPlantAndEquipmentNet"]),
    "goodwill": TagSpec(["us-gaap/Goodwill"]),
    "intangibles_net": TagSpec([
        "us-gaap/IntangibleAssetsNetExcludingGoodwill",
        "us-gaap/FiniteLivedIntangibleAssetsNet",
    ]),
    "operating_lease_rou_assets": TagSpec(["us-gaap/OperatingLeaseRightOfUseAsset"]),
    "deferred_tax_assets": TagSpec(["us-gaap/DeferredIncomeTaxAssetsNet"]),
    "other_noncurrent_assets": TagSpec(["us-gaap/OtherAssetsNoncurrent"]),
    "total_assets": TagSpec(["us-gaap/Assets"]),
    "accounts_payable": TagSpec(["us-gaap/AccountsPayableCurrent"]),
    "accrued_compensation": TagSpec(["us-gaap/EmployeeRelatedLiabilitiesCurrent"]),
    "accrued_liabilities": TagSpec(["us-gaap/AccruedLiabilitiesCurrent"]),
    "deferred_revenue_current": TagSpec([
        "us-gaap/DeferredRevenueCurrent",
        "us-gaap/ContractWithCustomerLiabilityCurrent",
    ]),
    "total_current_liabilities": TagSpec(["us-gaap/LiabilitiesCurrent"]),
    "long_term_debt": TagSpec([
        "us-gaap/LongTermDebtNoncurrent",
        "us-gaap/LongTermDebt",
    ]),
    "deferred_revenue_noncurrent": TagSpec([
        "us-gaap/DeferredRevenueNoncurrent",
        "us-gaap/ContractWithCustomerLiabilityNoncurrent",
    ]),
    "operating_lease_liabilities_noncurrent": TagSpec(["us-gaap/OperatingLeaseLiabilityNoncurrent"]),
    "deferred_tax_liabilities": TagSpec(["us-gaap/DeferredIncomeTaxLiabilitiesNet"]),
    "total_liabilities": TagSpec(["us-gaap/Liabilities"]),
    "equity": TagSpec(["us-gaap/StockholdersEquity"]),
    "shares_outstanding": TagSpec(
        ["us-gaap/CommonStockSharesOutstanding"], unit="shares"
    ),
}

CASH_FLOW_TAGS: dict[str, TagSpec] = {
    "operating_cash_flow": TagSpec(["us-gaap/NetCashProvidedByUsedInOperatingActivities"]),
    "capex": TagSpec([
        "us-gaap/PaymentsToAcquireProductiveAssets",
        "us-gaap/PaymentsToAcquirePropertyPlantAndEquipment",
    ], sign_flip=True),
    "purchases_investments": TagSpec([
        "us-gaap/PaymentsToAcquireAvailableForSaleSecuritiesDebt",
        "us-gaap/PaymentsToAcquireInvestments",
    ], sign_flip=True),
    "proceeds_investments": TagSpec([
        "us-gaap/ProceedsFromMaturitiesPrepaymentsAndCallsOfAvailableForSaleSecurities",
        "us-gaap/ProceedsFromSaleMaturityAndCollectionsOfInvestments",
    ]),
    "acquisitions_net_of_cash": TagSpec([
        "us-gaap/PaymentsToAcquireBusinessesNetOfCashAcquired",
    ], sign_flip=True),
    "net_cash_from_investing": TagSpec(["us-gaap/NetCashProvidedByUsedInInvestingActivities"]),
    "repurchases_common_stock": TagSpec([
        "us-gaap/PaymentsForRepurchaseOfCommonStock",
    ], sign_flip=True),
    "taxes_paid_net_share_settlement": TagSpec([
        "us-gaap/PaymentsRelatedToTaxWithholdingForShareBasedCompensation",
    ], sign_flip=True),
    "dividends_paid": TagSpec(["us-gaap/PaymentsOfDividends"], sign_flip=True),
    "proceeds_issuance_debt": TagSpec(["us-gaap/ProceedsFromIssuanceOfLongTermDebt"]),
    "repayment_of_debt": TagSpec([
        "us-gaap/RepaymentsOfLongTermDebt",
        "us-gaap/RepaymentsOfDebt",
    ], sign_flip=True),
    "net_cash_from_financing": TagSpec(["us-gaap/NetCashProvidedByUsedInFinancingActivities"]),
}

STATEMENT_MAP: dict[str, str] = {
    **{k: "income_statement" for k in INCOME_STATEMENT_TAGS},
    **{k: "balance_sheet" for k in BALANCE_SHEET_TAGS},
    **{k: "cash_flow" for k in CASH_FLOW_TAGS},
    "free_cash_flow": "cash_flow",
    "fcf_margin": "cash_flow",
}

UNIT_MAP: dict[str, str] = {
    "eps_basic": "USD/share",
    "eps_diluted": "USD/share",
    "shares_basic": "shares",
    "shares_diluted": "shares",
    "shares_outstanding": "shares",
    "fcf_margin": "ratio",
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
pytest tests/test_xbrl_tags.py -v
```

Expected: 11 PASSED.

- [ ] **Step 5: Commit**

```bash
git add financial_analysis/xbrl_tags.py tests/test_xbrl_tags.py
git commit -m "feat: add xbrl_tags.py — TagSpec registry with sign_flip and STATEMENT_MAP"
```

---

## Task 4: Extend `schema.py` and update `store.py`

**Files:**
- Modify: `financial_analysis/schema.py`
- Modify: `financial_analysis/store.py`
- Modify: `tests/test_schema.py` (add new field assertions)
- Modify: `tests/test_store.py` (add raw_extras round-trip test)

Add ~33 new `Optional[float]` fields and `raw_extras: dict` to `PeriodFinancials`. Update store to serialize/deserialize generically.

- [ ] **Step 1: Write new failing tests**

Add to `tests/test_schema.py`:

```python
def test_period_financials_new_income_statement_fields():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        cogs_total=10_000_000,
        interest_income=500_000,
        interest_expense=250_000,
        income_before_tax=5_000_000,
        income_tax_expense=1_000_000,
        da_total=800_000,
        sbc_total=1_200_000,
        eps_diluted=2.50,
        shares_diluted=2_500_000_000,
    )
    assert p.cogs_total == 10_000_000
    assert p.eps_diluted == 2.50

def test_period_financials_new_balance_sheet_fields():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        cash_and_equivalents=7_000_000_000,
        accounts_receivable_net=10_000_000_000,
        total_current_assets=44_000_000_000,
        long_term_debt=9_000_000_000,
        total_current_liabilities=10_000_000_000,
    )
    assert p.cash_and_equivalents == 7_000_000_000

def test_period_financials_new_cash_flow_fields():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        operating_cash_flow=28_000_000_000,
        capex=-1_000_000_000,
        repurchases_common_stock=-9_500_000_000,
        net_cash_from_investing=-10_000_000_000,
        net_cash_from_financing=-13_000_000_000,
    )
    assert p.repurchases_common_stock == -9_500_000_000

def test_period_financials_raw_extras_defaults_empty():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
    )
    assert p.raw_extras == {}

def test_period_financials_raw_extras_stores_concepts():
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        raw_extras={"us-gaap/PaymentsToAcquireEquitySecuritiesFvNi": 17_500_000_000},
    )
    assert p.raw_extras["us-gaap/PaymentsToAcquireEquitySecuritiesFvNi"] == 17_500_000_000
```

Add to `tests/test_store.py`:

```python
def test_save_and_load_preserves_new_fields(tmp_path):
    from financial_analysis.store import save, load, clear_cache
    clear_cache()
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        revenue=60_000_000_000,
        cash_and_equivalents=7_000_000_000,
        repurchases_common_stock=-9_500_000_000,
        raw_extras={"us-gaap/SomeUnmappedConcept": 999_000_000},
    )
    company = CompanyFinancials(ticker="TEST", cik="0001", company_name="Test Co", periods=[p])
    save(company, data_dir=tmp_path)
    clear_cache()
    loaded = load("TEST", data_dir=tmp_path)
    lp = loaded.periods[0]
    assert lp.cash_and_equivalents == 7_000_000_000
    assert lp.repurchases_common_stock == -9_500_000_000
    assert lp.raw_extras["us-gaap/SomeUnmappedConcept"] == 999_000_000
```

- [ ] **Step 2: Run to verify they fail**

```
pytest tests/test_schema.py tests/test_store.py -v
```

Expected: `TypeError` — unexpected keyword arguments for new fields.

- [ ] **Step 3: Update `financial_analysis/schema.py`**

Replace the file with:

```python
from __future__ import annotations
from dataclasses import dataclass, field
from datetime import date
from typing import Any, Dict, List, Optional


@dataclass
class PeriodFinancials:
    ticker: str
    period_end: date
    form: str
    filed_date: date
    currency: str = "USD"

    # --- Income Statement ---
    revenue: Optional[float] = None
    cogs_total: Optional[float] = None
    gross_profit: Optional[float] = None
    rd_expense: Optional[float] = None
    sm_expense: Optional[float] = None
    ga_expense: Optional[float] = None
    sga_expense: Optional[float] = None
    opex_amortization: Optional[float] = None
    opex_restructuring: Optional[float] = None
    operating_income: Optional[float] = None
    interest_income: Optional[float] = None
    interest_expense: Optional[float] = None
    gains_losses_on_investments: Optional[float] = None
    other_nonop_income: Optional[float] = None
    income_before_tax: Optional[float] = None
    income_tax_expense: Optional[float] = None
    net_income: Optional[float] = None
    da_total: Optional[float] = None
    sbc_total: Optional[float] = None
    eps_basic: Optional[float] = None
    eps_diluted: Optional[float] = None
    shares_basic: Optional[float] = None
    shares_diluted: Optional[float] = None

    # --- Balance Sheet ---
    cash_and_equivalents: Optional[float] = None
    short_term_investments: Optional[float] = None
    accounts_receivable_net: Optional[float] = None
    inventory: Optional[float] = None
    total_current_assets: Optional[float] = None
    ppe_net: Optional[float] = None
    goodwill: Optional[float] = None
    intangibles_net: Optional[float] = None
    operating_lease_rou_assets: Optional[float] = None
    deferred_tax_assets: Optional[float] = None
    other_noncurrent_assets: Optional[float] = None
    total_assets: Optional[float] = None
    accounts_payable: Optional[float] = None
    accrued_compensation: Optional[float] = None
    accrued_liabilities: Optional[float] = None
    deferred_revenue_current: Optional[float] = None
    total_current_liabilities: Optional[float] = None
    long_term_debt: Optional[float] = None
    deferred_revenue_noncurrent: Optional[float] = None
    operating_lease_liabilities_noncurrent: Optional[float] = None
    deferred_tax_liabilities: Optional[float] = None
    total_liabilities: Optional[float] = None
    equity: Optional[float] = None
    shares_outstanding: Optional[float] = None

    # --- Cash Flow ---
    operating_cash_flow: Optional[float] = None
    capex: Optional[float] = None
    purchases_investments: Optional[float] = None
    proceeds_investments: Optional[float] = None
    acquisitions_net_of_cash: Optional[float] = None
    net_cash_from_investing: Optional[float] = None
    repurchases_common_stock: Optional[float] = None
    taxes_paid_net_share_settlement: Optional[float] = None
    dividends_paid: Optional[float] = None
    proceeds_issuance_debt: Optional[float] = None
    repayment_of_debt: Optional[float] = None
    net_cash_from_financing: Optional[float] = None

    # --- Unmapped XBRL concepts ---
    raw_extras: Dict[str, Any] = field(default_factory=dict)

    @property
    def free_cash_flow(self) -> Optional[float]:
        if self.operating_cash_flow is None or self.capex is None:
            return None
        return self.operating_cash_flow - self.capex

    @property
    def fcf_margin(self) -> Optional[float]:
        fcf = self.free_cash_flow
        if fcf is None or not self.revenue:
            return None
        return fcf / self.revenue


@dataclass
class CompanyFinancials:
    ticker: str
    cik: str
    company_name: str
    periods: List[PeriodFinancials] = field(default_factory=list)
```

- [ ] **Step 4: Update `financial_analysis/store.py`**

Replace `_period_to_dict` and the `PeriodFinancials(...)` construction block in `load()` with generic field iteration. Replace the entire file:

```python
from __future__ import annotations
import json
from dataclasses import fields as dataclass_fields
from datetime import date, datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from financial_analysis.schema import CompanyFinancials, PeriodFinancials

_CACHE: Dict[str, CompanyFinancials] = {}
_DEFAULT_DATA_DIR = Path("data/processed")

_DATE_FIELDS = {"period_end", "filed_date"}
_SKIP_SERIALIZE = set()  # properties (free_cash_flow, fcf_margin) not in __dataclass_fields__

_VALID_METRICS = {
    f.name for f in dataclass_fields(PeriodFinancials)
} | {"free_cash_flow", "fcf_margin"}


def clear_cache() -> None:
    _CACHE.clear()


def save(company: CompanyFinancials, data_dir: Path = _DEFAULT_DATA_DIR) -> Path:
    data_dir.mkdir(parents=True, exist_ok=True)
    path = data_dir / f"{company.ticker.upper()}.json"

    def _period_to_dict(p: PeriodFinancials) -> dict:
        result: dict[str, Any] = {}
        for f in dataclass_fields(p):
            val = getattr(p, f.name)
            if f.name in _DATE_FIELDS and isinstance(val, date):
                result[f.name] = val.isoformat()
            else:
                result[f.name] = val
        return result

    payload = {
        "ticker": company.ticker,
        "cik": company.cik,
        "company_name": company.company_name,
        "periods": [_period_to_dict(p) for p in company.periods],
    }
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    _CACHE[company.ticker.upper()] = company
    return path


def load(ticker: str, data_dir: Path = _DEFAULT_DATA_DIR) -> CompanyFinancials:
    key = ticker.upper()
    if key in _CACHE:
        return _CACHE[key]

    path = data_dir / f"{key}.json"
    if not path.exists():
        raise FileNotFoundError(f"No saved data for {key} at {path}")

    raw = json.loads(path.read_text(encoding="utf-8"))

    def _to_date(s: str) -> date:
        return datetime.strptime(s, "%Y-%m-%d").date()

    def _build_period(p: dict) -> PeriodFinancials:
        kwargs: dict[str, Any] = {}
        for f in dataclass_fields(PeriodFinancials):
            if f.name in _DATE_FIELDS:
                kwargs[f.name] = _to_date(p[f.name])
            else:
                kwargs[f.name] = p.get(f.name, f.default if f.default is not f.default_factory else f.default_factory())
        return PeriodFinancials(**kwargs)

    periods = [_build_period(p) for p in raw["periods"]]
    company = CompanyFinancials(
        ticker=raw["ticker"],
        cik=raw["cik"],
        company_name=raw["company_name"],
        periods=periods,
    )
    _CACHE[key] = company
    return company


def get_series(
    ticker: str,
    metric: str,
    data_dir: Path = _DEFAULT_DATA_DIR,
) -> List[Tuple[date, Optional[float]]]:
    if metric not in _VALID_METRICS:
        raise ValueError(
            f"'{metric}' is not a valid metric. Valid options: {sorted(_VALID_METRICS)}"
        )
    company = load(ticker, data_dir=data_dir)
    return [(p.period_end, getattr(p, metric)) for p in company.periods]


def compare(
    tickers: List[str],
    metric: str,
    data_dir: Path = _DEFAULT_DATA_DIR,
) -> Dict[str, List[Tuple[date, Optional[float]]]]:
    return {t.upper(): get_series(t, metric, data_dir=data_dir) for t in tickers}
```

- [ ] **Step 5: Run tests to verify they pass**

```
pytest tests/test_schema.py tests/test_store.py -v
```

Expected: all PASSED including the 5 new schema tests and the new store round-trip test.

- [ ] **Step 6: Run full suite**

```
pytest tests/ -v
```

Expected: all previously passing tests still pass.

- [ ] **Step 7: Commit**

```bash
git add financial_analysis/schema.py financial_analysis/store.py tests/test_schema.py tests/test_store.py
git commit -m "feat: extend PeriodFinancials with full IS/BS/CF fields and raw_extras; update store serialization"
```

---

## Task 5: Create `cache.py`

**Files:**
- Create: `financial_analysis/cache.py`
- Create: `tests/test_cache.py`

- [ ] **Step 1: Write the failing tests**

Create `tests/test_cache.py`:

```python
import json
from pathlib import Path
from unittest.mock import patch

import pytest

from financial_analysis.cache import load_company_facts


SAMPLE_FACTS = {"entityName": "NVIDIA CORP", "cik": 1045810, "facts": {"us-gaap": {}}}


def test_cache_miss_fetches_and_writes(tmp_path):
    with patch("financial_analysis.cache.get_cik_for_ticker", return_value="1045810"), \
         patch("financial_analysis.cache.fetch_company_facts", return_value=SAMPLE_FACTS) as mock_fetch:
        result = load_company_facts("NVDA", cache_dir=tmp_path, refresh=False)
    assert result == SAMPLE_FACTS
    mock_fetch.assert_called_once_with("1045810")
    cache_file = tmp_path / "NVDA" / "company_facts.json"
    assert cache_file.exists()
    assert json.loads(cache_file.read_text(encoding="utf-8")) == SAMPLE_FACTS


def test_cache_hit_returns_disk_without_fetch(tmp_path):
    cache_file = tmp_path / "NVDA" / "company_facts.json"
    cache_file.parent.mkdir(parents=True)
    cache_file.write_text(json.dumps(SAMPLE_FACTS), encoding="utf-8")
    with patch("financial_analysis.cache.fetch_company_facts") as mock_fetch:
        result = load_company_facts("NVDA", cache_dir=tmp_path, refresh=False)
    assert result == SAMPLE_FACTS
    mock_fetch.assert_not_called()


def test_refresh_flag_bypasses_cache(tmp_path):
    cache_file = tmp_path / "NVDA" / "company_facts.json"
    cache_file.parent.mkdir(parents=True)
    stale = {"entityName": "STALE"}
    cache_file.write_text(json.dumps(stale), encoding="utf-8")
    fresh = {"entityName": "FRESH", "facts": {"us-gaap": {}}}
    with patch("financial_analysis.cache.get_cik_for_ticker", return_value="1045810"), \
         patch("financial_analysis.cache.fetch_company_facts", return_value=fresh):
        result = load_company_facts("NVDA", cache_dir=tmp_path, refresh=True)
    assert result["entityName"] == "FRESH"


def test_corrupt_cache_falls_back_to_fetch(tmp_path):
    cache_file = tmp_path / "NVDA" / "company_facts.json"
    cache_file.parent.mkdir(parents=True)
    cache_file.write_text("NOT VALID JSON", encoding="utf-8")
    with patch("financial_analysis.cache.get_cik_for_ticker", return_value="1045810"), \
         patch("financial_analysis.cache.fetch_company_facts", return_value=SAMPLE_FACTS):
        result = load_company_facts("NVDA", cache_dir=tmp_path, refresh=False)
    assert result == SAMPLE_FACTS
    assert json.loads(cache_file.read_text(encoding="utf-8")) == SAMPLE_FACTS


def test_ticker_uppercased_in_path(tmp_path):
    with patch("financial_analysis.cache.get_cik_for_ticker", return_value="1045810"), \
         patch("financial_analysis.cache.fetch_company_facts", return_value=SAMPLE_FACTS):
        load_company_facts("nvda", cache_dir=tmp_path, refresh=False)
    assert (tmp_path / "NVDA" / "company_facts.json").exists()
```

- [ ] **Step 2: Run to verify they fail**

```
pytest tests/test_cache.py -v
```

Expected: `ModuleNotFoundError` — file does not exist.

- [ ] **Step 3: Create `financial_analysis/cache.py`**

```python
from __future__ import annotations
import json
import logging
from pathlib import Path

from financial_analysis.edgar import get_cik_for_ticker
from financial_analysis.xbrl_api import fetch_company_facts

logger = logging.getLogger(__name__)


def load_company_facts(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> dict:
    path = cache_dir / ticker.upper() / "company_facts.json"
    if not refresh and path.exists():
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            logger.warning("Corrupt cache for %s — re-fetching from SEC", ticker)

    cik = get_cik_for_ticker(ticker)
    facts = fetch_company_facts(cik)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(facts), encoding="utf-8")
    return facts
```

- [ ] **Step 4: Run tests to verify they pass**

```
pytest tests/test_cache.py -v
```

Expected: 5 PASSED.

- [ ] **Step 5: Commit**

```bash
git add financial_analysis/cache.py tests/test_cache.py
git commit -m "feat: add cache.py — disk-backed XBRL fetch with corruption fallback"
```

---

## Task 6: Create `importer.py`

**Files:**
- Create: `financial_analysis/importer.py`
- Create: `tests/test_importer.py`

- [ ] **Step 1: Write the failing tests**

Create `tests/test_importer.py`:

```python
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
```

- [ ] **Step 2: Run to verify they fail**

```
pytest tests/test_importer.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 3: Create `financial_analysis/importer.py`**

```python
from __future__ import annotations
from dataclasses import fields as dataclass_fields
from datetime import date
from pathlib import Path
from typing import Any

from financial_analysis import store
from financial_analysis.cache import load_company_facts
from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.xbrl_api import _deduplicate, extract_series
from financial_analysis.xbrl_tags import (
    BALANCE_SHEET_TAGS,
    CASH_FLOW_TAGS,
    INCOME_STATEMENT_TAGS,
    TagSpec,
)

_ANNUAL_FORMS = ["10-K", "10-K/A"]
_QUARTERLY_FORMS = ["10-Q", "10-Q/A"]

_ALL_TAGS: dict[str, TagSpec] = {
    **INCOME_STATEMENT_TAGS,
    **BALANCE_SHEET_TAGS,
    **CASH_FLOW_TAGS,
}

_PERIOD_FIELDS = {f.name for f in dataclass_fields(PeriodFinancials)}


class UnsupportedFilingError(Exception):
    pass


def _resolve_field(
    facts: dict,
    field_name: str,
    spec: TagSpec,
    form_types: list[str],
    n: int,
) -> dict[str, float]:
    """Returns {period_end: value} for the first concept in spec that has data."""
    for concept in spec.concepts:
        entries = extract_series(facts, concept, form_types=form_types, unit=spec.unit, n=n)
        if entries:
            result = {}
            for e in entries:
                val = e["val"]
                if spec.sign_flip:
                    val = -abs(val)
                result[e["end"]] = val
            return result
    return {}


def _collect_raw_extras(
    facts: dict,
    canonical_concepts: set[str],
    form_types: list[str],
    known_period_ends: set[str],
    n: int,
) -> dict[str, dict[str, float]]:
    """Returns {period_end: {full_concept: value}} for unmapped us-gaap USD concepts."""
    extras: dict[str, dict[str, float]] = {end: {} for end in known_period_ends}
    gaap = facts.get("facts", {}).get("us-gaap", {})
    for concept_name, concept_data in gaap.items():
        full = f"us-gaap/{concept_name}"
        if full in canonical_concepts:
            continue
        entries = [
            e for e in concept_data.get("units", {}).get("USD", [])
            if e.get("form") in form_types and e["end"] in known_period_ends
        ]
        for e in _deduplicate(entries)[-n:]:
            extras.setdefault(e["end"], {})[full] = e["val"]
    return extras


def import_company(
    ticker: str,
    n_periods: int = 8,
    save: bool = True,
    refresh: bool = False,
    cache_dir: Path = Path("data/raw"),
) -> CompanyFinancials:
    facts = load_company_facts(ticker, cache_dir=cache_dir, refresh=refresh)

    if "us-gaap" not in facts.get("facts", {}):
        namespaces = list(facts.get("facts", {}).keys())
        raise UnsupportedFilingError(
            f"{ticker} does not report under us-gaap (namespaces found: {namespaces}). "
            "IFRS and non-US-GAAP filers are not supported."
        )

    cik = str(facts.get("cik", ""))
    company_name = facts.get("entityName", ticker.upper())

    canonical_concepts = {c for spec in _ALL_TAGS.values() for c in spec.concepts}

    all_periods: dict[tuple[str, str], dict[str, Any]] = {}

    for form_types in (_ANNUAL_FORMS, _QUARTERLY_FORMS):
        period_ends: dict[str, str] = {}

        for field_name, spec in _ALL_TAGS.items():
            values = _resolve_field(facts, field_name, spec, form_types, n_periods)
            for end, val in values.items():
                if field_name not in ("form", "ticker", "period_end", "filed_date", "currency"):
                    key = (end, form_types[0])
                    if key not in all_periods:
                        all_periods[key] = {
                            "ticker": ticker.upper(),
                            "period_end": end,
                            "form": form_types[0],
                            "filed_date": end,
                            "currency": "USD",
                            "raw_extras": {},
                        }
                    if field_name in _PERIOD_FIELDS and field_name not in all_periods[key]:
                        all_periods[key][field_name] = val

        known_ends = {k[0] for k in all_periods if k[1] == form_types[0]}
        extras = _collect_raw_extras(facts, canonical_concepts, form_types, known_ends, n_periods)
        for end, extra_dict in extras.items():
            key = (end, form_types[0])
            if key in all_periods:
                all_periods[key]["raw_extras"].update(extra_dict)

    # Resolve actual filed_date from the most data-rich concept
    _populate_filed_dates(facts, all_periods, _ANNUAL_FORMS + _QUARTERLY_FORMS, n_periods)

    periods: list[PeriodFinancials] = []
    for data in sorted(all_periods.values(), key=lambda d: (d["period_end"], d["form"])):
        kwargs: dict[str, Any] = {}
        for f in dataclass_fields(PeriodFinancials):
            if f.name in ("period_end", "filed_date"):
                raw = data.get(f.name, data["period_end"])
                kwargs[f.name] = date.fromisoformat(raw) if isinstance(raw, str) else raw
            elif f.name == "raw_extras":
                kwargs[f.name] = data.get("raw_extras", {})
            else:
                kwargs[f.name] = data.get(f.name)
        periods.append(PeriodFinancials(**kwargs))

    company = CompanyFinancials(
        ticker=ticker.upper(),
        cik=cik,
        company_name=company_name,
        periods=periods,
    )

    if save:
        store.save(company)

    return company


def _populate_filed_dates(
    facts: dict,
    all_periods: dict[tuple[str, str], dict],
    form_types: list[str],
    n: int,
) -> None:
    """Best-effort: fill filed_date from the first concept that has it."""
    ref_concept = "us-gaap/Revenues"
    ns, name = ref_concept.split("/", 1)
    try:
        entries = facts["facts"][ns][name]["units"]["USD"]
    except KeyError:
        return
    for e in entries:
        if e.get("form") in form_types:
            key_form = e["form"] if e["form"] in ("10-K", "10-Q") else (
                "10-K" if "K" in e["form"] else "10-Q"
            )
            key = (e["end"], key_form)
            if key in all_periods and "filed" in e:
                all_periods[key]["filed_date"] = e["filed"]
```

- [ ] **Step 4: Run tests to verify they pass**

```
pytest tests/test_importer.py -v
```

Expected: 7 PASSED.

- [ ] **Step 5: Run full suite**

```
pytest tests/ -v
```

Expected: all PASSED.

- [ ] **Step 6: Commit**

```bash
git add financial_analysis/importer.py tests/test_importer.py
git commit -m "feat: add importer.py — cache-backed XBRL import with quarterly support and IFRS detection"
```

---

## Task 7: Create `scripts/import_financials.py`

**Files:**
- Create: `scripts/import_financials.py`

- [ ] **Step 1: Create the script**

```python
"""CLI: import financial data for a ticker and save to data/processed/.

Usage:
    python -m scripts.import_financials NVDA
    python -m scripts.import_financials NVDA --refresh
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from financial_analysis.importer import UnsupportedFilingError, import_company
from financial_analysis.edgar import EdgarClientError


def main() -> None:
    parser = argparse.ArgumentParser(description="Import SEC financial data for a ticker.")
    parser.add_argument("ticker", help="Stock ticker symbol (e.g. NVDA)")
    parser.add_argument("--refresh", action="store_true", help="Force re-fetch from SEC API")
    args = parser.parse_args()

    ticker = args.ticker.upper()
    print(f"Importing {ticker}...")

    try:
        company = import_company(ticker, save=True, refresh=args.refresh)
    except UnsupportedFilingError as e:
        print(f"Error: {e}")
        sys.exit(1)
    except EdgarClientError as e:
        print(f"SEC API error: {e}")
        sys.exit(1)

    annual = [p for p in company.periods if p.form == "10-K"]
    quarterly = [p for p in company.periods if p.form == "10-Q"]

    print(f"  Company:   {company.company_name}")
    print(f"  CIK:       {company.cik}")
    print(f"  Annual periods (10-K):    {len(annual)}")
    print(f"  Quarterly periods (10-Q): {len(quarterly)}")
    print(f"  Saved: data/processed/{ticker}.json")
    print(f"  Cache: data/raw/{ticker}/company_facts.json")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Verify the script is importable**

```
python -c "import scripts.import_financials"
```

Expected: no error.

- [ ] **Step 3: Commit**

```bash
git add scripts/import_financials.py
git commit -m "feat: add scripts/import_financials.py CLI entrypoint"
```

---

## Task 8: Smoke Test — Ticket 4

This is a live call against the SEC API. No mocks.

- [ ] **Step 1: Run the importer against NVDA**

```
python -m scripts.import_financials NVDA
```

Expected output (exact counts may vary):
```
Importing NVDA...
  Company:   NVIDIA CORP
  CIK:       1045810
  Annual periods (10-K):    8
  Quarterly periods (10-Q): 12
  Saved: data/processed/NVDA.json
  Cache: data/raw/NVDA/company_facts.json
```

- [ ] **Step 2: Verify cache file exists**

```
python -c "import json; d=json.load(open('data/raw/NVDA/company_facts.json')); print('entityName:', d['entityName'])"
```

Expected: `entityName: NVIDIA CORP`

- [ ] **Step 3: Verify second run uses cache (fast, no API call)**

Run again and confirm it completes in under 2 seconds (vs 5–10s for a live fetch):

```
python -m scripts.import_financials NVDA
```

- [ ] **Step 4: Verify revenue is non-zero for recent annual period**

```
python -c "
from financial_analysis.store import load, clear_cache
clear_cache()
c = load('NVDA')
annual = [p for p in c.periods if p.form == '10-K']
for p in annual[-3:]:
    print(p.period_end, p.revenue, p.operating_cash_flow, p.capex)
"
```

Expected: revenue, OCF are positive; capex is negative (sign_flip applied).

- [ ] **Step 5: Verify raw_extras is populated**

```
python -c "
from financial_analysis.store import load, clear_cache
clear_cache()
c = load('NVDA')
p = [x for x in c.periods if x.form == '10-K'][-1]
print('raw_extras count:', len(p.raw_extras))
for k, v in list(p.raw_extras.items())[:5]:
    print(f'  {k}: {v:,.0f}')
"
```

Expected: `raw_extras count` is > 100.

- [ ] **Step 6: Mark Ticket 4 closed in PLANNING/tickets.md**

Open `PLANNING/tickets.md` and update Ticket 4 status to `✅ CLOSED` with today's date and smoke test results.

- [ ] **Step 7: Commit**

```bash
git add PLANNING/tickets.md data/processed/NVDA.json
git commit -m "feat: close Ticket 4 — importer smoke test passed for NVDA"
```

---

## Task 9: Create `scripts/analyze.py`

**Files:**
- Create: `scripts/analyze.py`
- Create: `tests/test_analyze.py`

- [ ] **Step 1: Write the failing tests**

Create `tests/test_analyze.py`:

```python
from datetime import date
from pathlib import Path

import pytest

from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from scripts.analyze import _flatten, write_xlsx, COLUMNS


def _make_company() -> CompanyFinancials:
    p1 = PeriodFinancials(
        ticker="TEST",
        period_end=date(2024, 1, 28),
        form="10-K",
        filed_date=date(2024, 2, 21),
        revenue=60_920_000_000,
        operating_cash_flow=28_090_000_000,
        capex=-1_070_000_000,
        total_assets=65_730_000_000,
        raw_extras={"us-gaap/PaymentsToAcquireEquitySecuritiesFvNi": 860_000_000},
    )
    p2 = PeriodFinancials(
        ticker="TEST",
        period_end=date(2023, 10, 29),
        form="10-Q",
        filed_date=date(2023, 11, 21),
        revenue=18_120_000_000,
        raw_extras={},
    )
    return CompanyFinancials(ticker="TEST", cik="1045810", company_name="Test Co", periods=[p1, p2])


def test_flatten_returns_four_tabs():
    company = _make_company()
    tabs = _flatten(company)
    assert set(tabs.keys()) == {"Income Statement", "Balance Sheet", "Cash Flow", "Raw"}


def test_flatten_income_statement_has_revenue_row():
    tabs = _flatten(_make_company())
    is_rows = tabs["Income Statement"]
    metrics = [r["metric"] for r in is_rows]
    assert "revenue" in metrics


def test_flatten_cash_flow_has_free_cash_flow():
    tabs = _flatten(_make_company())
    cf_rows = tabs["Cash Flow"]
    metrics = [r["metric"] for r in cf_rows]
    assert "free_cash_flow" in metrics


def test_flatten_raw_tab_has_unmapped_concepts():
    tabs = _flatten(_make_company())
    raw_rows = tabs["Raw"]
    metrics = [r["metric"] for r in raw_rows]
    assert "us-gaap/PaymentsToAcquireEquitySecuritiesFvNi" in metrics


def test_flatten_none_values_not_included():
    tabs = _flatten(_make_company())
    for tab_rows in tabs.values():
        for row in tab_rows:
            assert row["value"] != "" or row["value"] is None


def test_flatten_row_has_all_columns():
    tabs = _flatten(_make_company())
    for tab_rows in tabs.values():
        for row in tab_rows:
            for col in COLUMNS:
                assert col in row, f"Column '{col}' missing from row"


def test_flatten_capex_unit_is_usd():
    tabs = _flatten(_make_company())
    capex_rows = [r for r in tabs["Cash Flow"] if r["metric"] == "capex"]
    assert all(r["unit"] == "USD" for r in capex_rows)


def test_write_xlsx_creates_file_with_four_tabs(tmp_path):
    import openpyxl
    company = _make_company()
    out = tmp_path / "TEST_financials.xlsx"
    write_xlsx(company, out)
    assert out.exists()
    wb = openpyxl.load_workbook(out)
    assert set(wb.sheetnames) == {"Income Statement", "Balance Sheet", "Cash Flow", "Raw"}


def test_write_xlsx_has_header_row(tmp_path):
    import openpyxl
    company = _make_company()
    out = tmp_path / "TEST_financials.xlsx"
    write_xlsx(company, out)
    wb = openpyxl.load_workbook(out)
    ws = wb["Income Statement"]
    header = [cell.value for cell in ws[1]]
    assert header == COLUMNS


def test_write_xlsx_returns_total_row_count(tmp_path):
    company = _make_company()
    out = tmp_path / "TEST_financials.xlsx"
    total = write_xlsx(company, out)
    assert total > 0
```

- [ ] **Step 2: Run to verify they fail**

```
pytest tests/test_analyze.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 3: Create `scripts/analyze.py`**

```python
"""CLI: export financial statements for a ticker to a four-tab xlsx file.

Usage:
    python -m scripts.analyze NVDA
    python -m scripts.analyze NVDA --refresh
    python -m scripts.analyze NVDA --out path/to/custom.xlsx
"""
from __future__ import annotations
import argparse
import sys
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import openpyxl

from financial_analysis import store
from financial_analysis.importer import UnsupportedFilingError, import_company
from financial_analysis.edgar import EdgarClientError
from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.xbrl_tags import STATEMENT_MAP, UNIT_MAP

COLUMNS = [
    "ticker", "cik", "company_name", "period_end", "form_type",
    "filed_date", "statement", "metric", "value", "unit",
]

_TAB_FOR = {
    "income_statement": "Income Statement",
    "balance_sheet": "Balance Sheet",
    "cash_flow": "Cash Flow",
}

_DERIVED_METRICS = {
    "free_cash_flow": "cash_flow",
    "fcf_margin": "cash_flow",
}


def _unit_for(field: str) -> str:
    return UNIT_MAP.get(field, "USD")


def _flatten(company: CompanyFinancials) -> dict[str, list[dict[str, Any]]]:
    tabs: dict[str, list[dict]] = {
        "Income Statement": [],
        "Balance Sheet": [],
        "Cash Flow": [],
        "Raw": [],
    }

    for p in company.periods:
        base = {
            "ticker": p.ticker,
            "cik": company.cik,
            "company_name": company.company_name,
            "period_end": p.period_end.isoformat(),
            "form_type": p.form,
            "filed_date": p.filed_date.isoformat(),
        }

        # Canonical fields from STATEMENT_MAP
        for metric, statement in STATEMENT_MAP.items():
            if metric in _DERIVED_METRICS:
                continue
            val = getattr(p, metric, None)
            if val is None:
                continue
            tabs[_TAB_FOR[statement]].append({
                **base,
                "statement": statement,
                "metric": metric,
                "value": val,
                "unit": _unit_for(metric),
            })

        # Derived properties
        for metric, statement in _DERIVED_METRICS.items():
            val = getattr(p, metric, None)
            if val is not None:
                tabs[_TAB_FOR[statement]].append({
                    **base,
                    "statement": statement,
                    "metric": metric,
                    "value": val,
                    "unit": _unit_for(metric),
                })

        # Raw extras
        for concept, val in (p.raw_extras or {}).items():
            if val is not None:
                tabs["Raw"].append({
                    **base,
                    "statement": "raw",
                    "metric": concept,
                    "value": val,
                    "unit": "USD",
                })

    return tabs


def write_xlsx(company: CompanyFinancials, out_path: Path) -> int:
    wb = openpyxl.Workbook()
    wb.remove(wb.active)

    tabs = _flatten(company)
    total_rows = 0

    for tab_name, rows in tabs.items():
        ws = wb.create_sheet(tab_name)
        ws.append(COLUMNS)
        for row in rows:
            ws.append([row.get(c, "") for c in COLUMNS])
            total_rows += 1

    out_path.parent.mkdir(parents=True, exist_ok=True)
    wb.save(str(out_path))
    return total_rows


def main() -> None:
    parser = argparse.ArgumentParser(description="Export financial statements to xlsx.")
    parser.add_argument("ticker", help="Stock ticker symbol (e.g. NVDA)")
    parser.add_argument("--refresh", action="store_true", help="Force re-fetch from SEC API")
    parser.add_argument("--out", type=Path, help="Output file path (default: data/processed/TICKER_financials.xlsx)")
    args = parser.parse_args()

    ticker = args.ticker.upper()
    out_path = args.out or Path(f"data/processed/{ticker}_financials.xlsx")

    print(f"Importing {ticker}...")
    try:
        import_company(ticker, save=True, refresh=args.refresh)
    except UnsupportedFilingError as e:
        print(f"Error: {e}")
        sys.exit(1)
    except EdgarClientError as e:
        print(f"SEC API error: {e}")
        sys.exit(1)

    store.clear_cache()
    company = store.load(ticker)

    print(f"Writing {out_path} ...")
    total = write_xlsx(company, out_path)
    print(f"Done: {out_path} ({total} rows across 4 tabs)")


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run tests to verify they pass**

```
pytest tests/test_analyze.py -v
```

Expected: 11 PASSED.

- [ ] **Step 5: Run full suite**

```
pytest tests/ -v
```

Expected: all PASSED.

- [ ] **Step 6: Commit**

```bash
git add scripts/analyze.py tests/test_analyze.py
git commit -m "feat: add scripts/analyze.py — four-tab xlsx export with canonical and raw XBRL rows"
```

---

## Task 10: Smoke Test — Ticket 4.5

- [ ] **Step 1: Run the analyze command against NVDA**

```
python -m scripts.analyze NVDA
```

Expected:
```
Importing NVDA...
Writing data/processed/NVDA_financials.xlsx ...
Done: data/processed/NVDA_financials.xlsx (N rows across 4 tabs)
```

- [ ] **Step 2: Open the xlsx and verify four tabs**

Open `data/processed/NVDA_financials.xlsx` in Excel or LibreOffice. Confirm:
- Four tabs: Income Statement, Balance Sheet, Cash Flow, Raw
- Income Statement: `revenue` row present with positive values for recent periods
- Cash Flow: `capex` values are negative; `free_cash_flow` row present
- Raw tab: 400+ rows for NVDA (626 us-gaap concepts)
- No encoding errors or `#NAME?` cells

- [ ] **Step 3: Spot-check a known value**

```
python -c "
from financial_analysis.store import load, clear_cache
clear_cache()
c = load('NVDA')
p = [x for x in c.periods if x.form == '10-K' and str(x.period_end) == '2025-01-26'][0]
print('Revenue FY2025:', p.revenue / 1e9, 'B')
print('FCF FY2025:', p.free_cash_flow / 1e9, 'B')
print('Capex FY2025:', p.capex / 1e9, 'B (should be negative)')
"
```

Expected: Revenue ~130.5B, FCF ~60.8B, CapEx ~-3.24B.

- [ ] **Step 4: Verify --refresh flag works**

```
python -m scripts.analyze NVDA --refresh
```

Expected: fetches from SEC (slower) and overwrites `data/raw/NVDA/company_facts.json`.

- [ ] **Step 5: Test with a second ticker**

```
python -m scripts.analyze PANW
```

Expected: produces `data/processed/PANW_financials.xlsx` without error.

- [ ] **Step 6: Mark Ticket 4.5 closed in PLANNING/tickets.md**

Update `PLANNING/tickets.md` — add Ticket 4.5 as `✅ CLOSED` with smoke test results.

- [ ] **Step 7: Final commit**

```bash
git add PLANNING/tickets.md data/processed/PANW_financials.xlsx data/processed/NVDA_financials.xlsx
git commit -m "feat: close Ticket 4.5 — analyze command smoke test passed for NVDA and PANW"
```
