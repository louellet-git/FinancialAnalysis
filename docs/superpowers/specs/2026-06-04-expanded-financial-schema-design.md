---
id: DOC-SPEC-EXPANDED-FINANCIAL-SCHEMA
type: description
item: 10.14 Database design description
status: superseded
date: 2026-06-04
scope: Historical design of the expanded statement-store schema; superseded by docs/ARCHITECTURE.md and the code.
---
# Expanded Financial Schema Design

**Date:** 2026-06-04
**Status:** Approved for implementation

---

## Goal

Expand the financial data pipeline to capture the complete income statement waterfall, balance sheet structure, and cash flow statement for cybersecurity companies (PANW, CRWD, FTNT). Add a reconciliation pass that validates accounting relationships before any simulation or analysis runs.

---

## Architecture

Four components change or are created:

```
xbrl_tags.py        — tag registry: canonical field name → [XBRL concepts] with comments
schema.py           — PeriodFinancials gains IncomeStatement, BalanceSheet, CashFlow sub-objects
reconciliation.py   — single-pass validator: intra-statement + cross-statement checks
importer.py         — updated to populate full schema and call reconciliation before returning
```

**Data flow:**
```
import_company('PANW')
    ├── fetch XBRL facts (xbrl_api.py)
    ├── map facts → canonical fields via xbrl_tags.py
    ├── populate IncomeStatement / BalanceSheet / CashFlow
    ├── unmapped facts → raw_extras: dict on each sub-object
    ├── run reconciliation pass (reconciliation.py)
    │     ├── intra-statement checks
    │     └── cross-statement checks
    │           ├── gap ≤ 0.1% of revenue → ReconciliationWarning (logged)
    │           └── gap > 0.1% of revenue → ReconciliationError (raised, blocks analysis)
    └── save to data/processed/PANW.json (store.py)
```

---

## Schema Design

### PeriodFinancials (updated)

Existing top-level fields (`revenue`, `gross_profit`, etc.) are **retained** for backwards compatibility with the DCF model. The three new sub-objects are added alongside them.

```python
@dataclass
class PeriodFinancials:
    # ... existing fields unchanged ...
    income_statement: IncomeStatement = field(default_factory=IncomeStatement)
    balance_sheet: BalanceSheet = field(default_factory=BalanceSheet)
    cash_flow: CashFlow = field(default_factory=CashFlow)
```

### IncomeStatement

All monetary fields are `Optional[float]` in USD. Derived fields are `@property`.

**Revenue**
- `revenue_total` — us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax, fallback us-gaap/Revenues
- `revenue_product` — company-specific (panw:ProductRevenue, crwd:ProductRevenue)
- `revenue_subscription_and_support` — company-specific
- `revenue_subscription` — company-specific
- `revenue_support` — company-specific
- `revenue_professional_services` — company-specific

**Cost of Revenue**
- `cogs_total` — us-gaap/CostOfRevenue
- `cogs_product` — company-specific
- `cogs_subscription_and_support` — company-specific
- `cogs_subscription` — company-specific
- `cogs_support` — company-specific
- `cogs_amortization_of_acquired_intangibles` — us-gaap/CostOfGoodsAndServicesSoldAmortization
- `cogs_stock_based_compensation` — company-specific
- `cogs_depreciation` — company-specific

**Gross Profit (derived)**
- `gross_profit_total` — us-gaap/GrossProfit or derived: revenue_total − cogs_total
- `gross_margin` ← derived
- `product_gross_profit` ← derived
- `product_gross_margin` ← derived
- `subscription_gross_profit` ← derived
- `subscription_gross_margin` ← derived

**Operating Expenses**
- `rd_total` — us-gaap/ResearchAndDevelopmentExpense
- `rd_stock_based_compensation` — company-specific
- `rd_depreciation` — company-specific
- `sm_total` — us-gaap/SellingAndMarketingExpense
- `sm_stock_based_compensation` — company-specific
- `sm_depreciation` — company-specific
- `ga_total` — us-gaap/GeneralAndAdministrativeExpense
- `ga_stock_based_compensation` — company-specific
- `ga_depreciation` — company-specific
- `opex_amortization_of_acquired_intangibles` — us-gaap/AmortizationOfIntangibleAssets
- `opex_restructuring` — us-gaap/RestructuringCharges
- `opex_total` — us-gaap/OperatingExpenses or derived

**Stock-Based Compensation Summary**
- `sbc_in_cogs` — company-specific
- `sbc_in_rd` — company-specific
- `sbc_in_sm` — company-specific
- `sbc_in_ga` — company-specific
- `sbc_total` — us-gaap/ShareBasedCompensation

**Operating Income**
- `operating_income` — us-gaap/OperatingIncomeLoss or derived: gross_profit − opex_total
- `operating_margin` ← derived

**Non-Operating**
- `interest_income` — us-gaap/InterestIncomeNonoperating
- `interest_expense` — us-gaap/InterestExpense
- `amortization_of_debt_issuance_costs` — us-gaap/AmortizationOfFinancingCosts
- `gains_losses_on_investments` — us-gaap/GainLossOnInvestments
- `fx_gains_losses` — us-gaap/ForeignCurrencyTransactionGainLossBeforeTax
- `other_nonop_net` — us-gaap/OtherNonoperatingIncomeExpense
- `nonop_total` ← derived

**Pre-Tax / Tax / Net Income**
- `income_before_tax` — us-gaap/IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest
- `income_tax_expense` — us-gaap/IncomeTaxExpenseBenefit
- `deferred_tax_component` — us-gaap/DeferredIncomeTaxExpenseBenefit
- `net_income` — us-gaap/NetIncomeLoss
- `effective_tax_rate` ← derived
- `net_margin` ← derived

**Per Share**
- `eps_basic` — us-gaap/EarningsPerShareBasic
- `eps_diluted` — us-gaap/EarningsPerShareDiluted
- `shares_basic` — us-gaap/WeightedAverageNumberOfSharesOutstandingBasic
- `shares_diluted` — us-gaap/WeightedAverageNumberOfDilutedSharesOutstanding
- `shares_period_end` — us-gaap/CommonStockSharesOutstanding

**SaaS Operating Metrics**
- `rpo_total` — us-gaap/RevenueRemainingPerformanceObligation
- `rpo_next_12m` — us-gaap/RevenueRemainingPerformanceObligationExpectedTimingOfSatisfactionPeriod1
- `rpo_beyond_12m` ← derived
- `deferred_revenue_beginning` — from prior period balance sheet
- `deferred_revenue_ending` — from current period balance sheet
- `change_in_deferred_revenue` ← derived
- `billings` ← derived: revenue_total + change_in_deferred_revenue

**Non-GAAP / EBITDA Family (all derived)**
- `non_gaap_gross_profit`
- `non_gaap_gross_margin`
- `non_gaap_operating_income`
- `non_gaap_operating_margin`
- `ebit` (alias: operating_income)
- `ebitda`
- `ebitda_margin`
- `adjusted_ebitda`
- `adjusted_ebitda_margin`

**raw_extras: dict** — any us-gaap or company-specific USD facts not mapped to canonical fields

---

### BalanceSheet

**Current Assets**
- `cash_and_equivalents` — us-gaap/CashAndCashEquivalentsAtCarryingValue
- `short_term_investments` — us-gaap/ShortTermInvestments
- `st_investments_us_govt` — company-specific
- `st_investments_corporate_debt` — company-specific
- `st_investments_commercial_paper` — company-specific
- `st_investments_certificates_of_deposit` — company-specific
- `st_investments_agency_securities` — company-specific
- `accounts_receivable_gross` — us-gaap/AccountsReceivableGrossCurrent
- `allowance_for_doubtful_accounts` — us-gaap/AllowanceForDoubtfulAccountsReceivableCurrent
- `accounts_receivable_net` — us-gaap/AccountsReceivableNetCurrent
- `deferred_contract_costs_current` — us-gaap/CapitalizedContractCostNetCurrent
- `income_tax_receivable` — us-gaap/IncomeTaxesReceivable
- `prepaid_and_other_current` — us-gaap/PrepaidExpenseAndOtherAssetsCurrent
- `total_current_assets` — us-gaap/AssetsCurrent

**Non-Current Assets**
- `long_term_investments` — us-gaap/LongTermInvestments
- `ppe_gross` — us-gaap/PropertyPlantAndEquipmentGross
- `ppe_accumulated_depreciation` — us-gaap/AccumulatedDepreciationDepletionAndAmortizationPropertyPlantAndEquipment
- `ppe_net` — us-gaap/PropertyPlantAndEquipmentNet
- `ppe_servers_and_networking` — company-specific
- `ppe_leasehold_improvements` — company-specific
- `ppe_computers_and_equipment` — company-specific
- `ppe_furniture_and_fixtures` — company-specific
- `operating_lease_rou_assets` — us-gaap/OperatingLeaseRightOfUseAsset
- `goodwill` — us-gaap/Goodwill
- `intangibles_gross` — us-gaap/FiniteLivedIntangibleAssetsGross
- `intangibles_accumulated_amortization` — us-gaap/FiniteLivedIntangibleAssetsAccumulatedAmortization
- `intangibles_net` — us-gaap/FiniteLivedIntangibleAssetsNet
- `intangibles_developed_technology` — company-specific
- `intangibles_customer_relationships` — company-specific
- `intangibles_trade_names` — company-specific
- `deferred_contract_costs_noncurrent` — us-gaap/CapitalizedContractCostNetNoncurrent
- `deferred_tax_assets` — us-gaap/DeferredIncomeTaxAssetsNet
- `other_noncurrent_assets` — us-gaap/OtherAssetsNoncurrent
- `total_noncurrent_assets` ← derived: total_assets − total_current_assets
- `total_assets` — us-gaap/Assets

**Current Liabilities**
- `accounts_payable` — us-gaap/AccountsPayableCurrent
- `accrued_compensation` — us-gaap/EmployeeRelatedLiabilitiesCurrent
- `accrued_and_other_current` — us-gaap/AccruedLiabilitiesCurrent
- `operating_lease_liabilities_current` — us-gaap/OperatingLeaseLiabilityCurrent
- `deferred_revenue_current` — us-gaap/DeferredRevenueCurrent
- `short_term_debt` — us-gaap/DebtCurrent
- `convertible_notes_current` — us-gaap/ConvertibleNotesPayableCurrent
- `income_taxes_payable` — us-gaap/TaxesPayableCurrent
- `total_current_liabilities` — us-gaap/LiabilitiesCurrent

**Non-Current Liabilities**
- `long_term_debt` — us-gaap/LongTermDebtNoncurrent
- `convertible_notes_noncurrent` — us-gaap/ConvertibleNotesPayable
- `operating_lease_liabilities_noncurrent` — us-gaap/OperatingLeaseLiabilityNoncurrent
- `deferred_revenue_noncurrent` — us-gaap/DeferredRevenueNoncurrent
- `deferred_tax_liabilities` — us-gaap/DeferredIncomeTaxLiabilitiesNet
- `other_noncurrent_liabilities` — us-gaap/OtherLiabilitiesNoncurrent
- `total_noncurrent_liabilities` ← derived: total_liabilities − total_current_liabilities
- `total_liabilities` — us-gaap/Liabilities

**Equity**
- `common_stock` — us-gaap/CommonStockValue
- `additional_paid_in_capital` — us-gaap/AdditionalPaidInCapital
- `common_stock_and_apic` — us-gaap/CommonStocksIncludingAdditionalPaidInCapital
- `retained_earnings_accumulated_deficit` — us-gaap/RetainedEarningsAccumulatedDeficit
- `accumulated_other_comprehensive_loss` — us-gaap/AccumulatedOtherComprehensiveIncomeLossNetOfTax
- `treasury_stock` — us-gaap/TreasuryStockValue
- `total_equity` — us-gaap/StockholdersEquity
- `total_liabilities_and_equity` — us-gaap/LiabilitiesAndStockholdersEquity

**Derived Metrics**
- `current_ratio`
- `quick_ratio`
- `total_debt` ← short_term_debt + convertible_notes_current + long_term_debt + convertible_notes_noncurrent
- `net_cash` ← cash + investments − total_debt
- `deferred_revenue_total` ← current + noncurrent
- `net_working_capital`
- `days_sales_outstanding`
- `debt_to_equity`
- `book_value_per_share`

**raw_extras: dict**

---

### CashFlow

**Operating Activities**
- `cfo_net_income` — us-gaap/NetIncomeLoss
- `cfo_depreciation_ppe` — us-gaap/Depreciation
- `cfo_amortization_acquired_intangibles` — us-gaap/AmortizationOfIntangibleAssets
- `cfo_amortization_rou_assets` — us-gaap/OperatingLeaseRightOfUseAssetAmortization
- `cfo_stock_based_compensation` — us-gaap/ShareBasedCompensation
- `cfo_amortization_deferred_contract_costs` — us-gaap/CapitalizedContractCostAmortization
- `cfo_amortization_debt_discount` — us-gaap/AmortizationOfFinancingCosts
- `cfo_deferred_income_taxes` — us-gaap/DeferredIncomeTaxExpenseBenefit
- `cfo_gain_loss_on_investments` — us-gaap/GainLossOnInvestments
- `cfo_other_noncash` — us-gaap/OtherOperatingActivitiesCashFlowStatement
- `wc_change_accounts_receivable` — us-gaap/IncreaseDecreaseInAccountsReceivable
- `wc_change_deferred_contract_costs` — us-gaap/IncreaseDecreaseInCapitalizedContractCost
- `wc_change_prepaid_and_other` — us-gaap/IncreaseDecreaseInPrepaidDeferredExpenseAndOtherAssets
- `wc_change_accounts_payable` — us-gaap/IncreaseDecreaseInAccountsPayable
- `wc_change_accrued_compensation` — us-gaap/IncreaseDecreaseInEmployeeRelatedLiabilities
- `wc_change_accrued_and_other` — us-gaap/IncreaseDecreaseInAccruedLiabilities
- `wc_change_deferred_revenue` — us-gaap/IncreaseDecreaseInDeferredRevenue
- `wc_change_income_taxes` — us-gaap/IncreaseDecreaseInIncomeTaxesPayableNetOfIncomeTaxesReceivable
- `net_cash_from_operations` — us-gaap/NetCashProvidedByUsedInOperatingActivities

**Investing Activities**
- `capex` — us-gaap/PaymentsToAcquireProductiveAssets, fallback PaymentsToAcquirePropertyPlantAndEquipment
- `purchases_st_investments` — us-gaap/PaymentsToAcquireShortTermInvestments
- `proceeds_maturities_st_investments` — us-gaap/ProceedsFromMaturitiesPrepaymentsAndCallsOfShorttermInvestments
- `proceeds_sales_st_investments` — us-gaap/ProceedsFromSaleOfShortTermInvestments
- `purchases_lt_investments` — us-gaap/PaymentsToAcquireInvestments
- `proceeds_lt_investments` — us-gaap/ProceedsFromSaleAndMaturityOfOtherInvestments
- `acquisitions_net_of_cash` — us-gaap/PaymentsToAcquireBusinessesNetOfCashAcquired
- `purchases_intangible_assets` — us-gaap/PaymentsToAcquireIntangibleAssets
- `other_investing` — us-gaap/PaymentsForProceedsFromOtherInvestingActivities
- `net_cash_from_investing` — us-gaap/NetCashProvidedByUsedInInvestingActivities

**Financing Activities**
- `proceeds_stock_plans_espp` — us-gaap/ProceedsFromIssuanceOfSharesUnderIncentiveAndShareBasedCompensationPlansIncludingStockOptions
- `taxes_paid_net_share_settlement` — us-gaap/PaymentsRelatedToTaxWithholdingForShareBasedCompensation
- `repurchases_common_stock` — us-gaap/PaymentsForRepurchaseOfCommonStock
- `proceeds_issuance_debt` — us-gaap/ProceedsFromIssuanceOfLongTermDebt
- `repayment_of_debt` — us-gaap/RepaymentsOfLongTermDebt
- `payments_debt_issuance_costs` — us-gaap/PaymentsOfDebtIssuanceCosts
- `finance_lease_payments` — us-gaap/FinanceLeasePrincipalPayments
- `other_financing` — us-gaap/ProceedsFromPaymentsForOtherFinancingActivities
- `net_cash_from_financing` — us-gaap/NetCashProvidedByUsedInFinancingActivities

**Cash Summary**
- `fx_effect_on_cash` — us-gaap/EffectOfExchangeRateOnCashCashEquivalentsRestrictedCashAndRestrictedCashEquivalents
- `net_change_in_cash` ← derived: ops + investing + financing + fx
- `cash_beginning_of_period` — us-gaap/CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalents (prior period)
- `cash_end_of_period` — us-gaap/CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalentsIncludingDisposalGroupAndDiscontinuedOperations

**Supplemental Disclosures**
- `cash_paid_for_taxes` — us-gaap/IncomeTaxesPaid
- `cash_paid_for_interest` — us-gaap/InterestPaid
- `noncash_rou_from_operating_leases` — us-gaap/RightOfUseAssetObtainedInExchangeForOperatingLeaseLiability
- `noncash_stock_consideration_acquisitions` — company-specific

**Derived Metrics**
- `free_cash_flow` ← net_cash_from_operations − capex
- `fcf_margin` ← free_cash_flow / revenue
- `levered_free_cash_flow` ← free_cash_flow − cash_paid_for_interest − repayment_of_debt
- `cash_conversion_ratio` ← free_cash_flow / net_income
- `capex_intensity` ← capex / revenue
- `net_investment_in_growth` ← capex + acquisitions_net_of_cash + wc_change_deferred_contract_costs

**raw_extras: dict**

---

## XBRL Tag Registry (xbrl_tags.py)

Single file containing three dicts — `INCOME_STATEMENT_TAGS`, `BALANCE_SHEET_TAGS`, `CASH_FLOW_TAGS` — each mapping canonical field path strings to an ordered list of XBRL concept strings. The importer tries each concept in order and uses the first that returns data. Comments on each entry document what the tag represents and which companies use it.

```python
INCOME_STATEMENT_TAGS: dict[str, list[str]] = {
    # Total revenue. PANW, CRWD, FTNT all use the contract-based tag.
    # Fallback to Revenues for older filings.
    "revenue_total": [
        "us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax",
        "us-gaap/Revenues",
    ],
    # Product revenue — hardware/appliance sales. Company-specific extension tags.
    "revenue_product": [
        "panw/ProductRevenue",
        "crwd/ProductRevenue",
        "ftnt/ProductRevenue",
    ],
    # ... all ~80 canonical fields follow the same pattern.
    # Full registry implemented in financial_analysis/xbrl_tags.py.
}
```

---

## Reconciliation (reconciliation.py)

### Tolerance
- Gap ≤ 0.1% of `revenue_total` → `ReconciliationWarning` (logged, does not block)
- Gap > 0.1% of `revenue_total` → `ReconciliationError` (raised, blocks analysis)
- If `revenue_total` is None, use absolute tolerance of $1M

### Intra-Statement Checks (19 total)

**Income Statement (7)**
1. `revenue_product + revenue_subscription_and_support [+ revenue_professional_services] ≈ revenue_total`
2. `revenue_subscription + revenue_support ≈ revenue_subscription_and_support`
3. `revenue_total − cogs_total ≈ gross_profit_total`
4. `gross_profit_total − opex_total ≈ operating_income`
5. `sbc_in_cogs + sbc_in_rd + sbc_in_sm + sbc_in_ga ≈ sbc_total`
6. `operating_income + nonop_total − income_tax_expense ≈ net_income`
7. `income_before_tax − income_tax_expense ≈ net_income`

**Balance Sheet (7)**
1. `total_current_assets + total_noncurrent_assets ≈ total_assets`
2. `total_current_liabilities + total_noncurrent_liabilities ≈ total_liabilities`
3. `total_liabilities + total_equity ≈ total_assets` ← fundamental equation
4. `total_liabilities + total_equity ≈ total_liabilities_and_equity`
5. `deferred_revenue_current + deferred_revenue_noncurrent ≈ deferred_revenue_total`
6. `accounts_receivable_gross − allowance_for_doubtful_accounts ≈ accounts_receivable_net`
7. `ppe_gross − ppe_accumulated_depreciation ≈ ppe_net`

**Cash Flow (5)**
1. `net_cash_from_operations + net_cash_from_investing + net_cash_from_financing + fx_effect ≈ net_change_in_cash`
2. `cash_beginning_of_period + net_change_in_cash ≈ cash_end_of_period`
3. `cfo_net_income + [all non-cash adjustments] + [all working capital changes] ≈ net_cash_from_operations`
4. `net_cash_from_operations − capex ≈ free_cash_flow`
5. `capex` must be non-negative (sign check — some companies file as negative)

### Cross-Statement Checks (2)
1. `cash_flow.cash_end_of_period ≈ balance_sheet.cash_and_equivalents`
2. `cash_flow.wc_change_deferred_revenue ≈ balance_sheet.deferred_revenue_current(t) + balance_sheet.deferred_revenue_noncurrent(t) − [same fields prior period]`
   - Requires prior period balance sheet. If no prior period exists in `CompanyFinancials.periods`, this check is **skipped** (not warned, not errored).
   - `fcf_margin` and `capex_intensity` in `CashFlow` derived metrics require `revenue_total` from `IncomeStatement`. The reconciliation pass receives the full `PeriodFinancials` so both sub-objects are accessible.

### ReconciliationReport
```python
@dataclass
class ReconciliationReport:
    ticker: str
    period_end: date
    warnings: list[ReconciliationWarning]
    errors: list[ReconciliationError]
    passed: bool  # True if no errors (warnings are OK)
```

Reconciliation runs after import, before `store.save()`. If `passed` is False, `ReconciliationError` is raised and the data is not saved.

---

## New Files

| File | Action |
|------|--------|
| `financial_analysis/xbrl_tags.py` | Create — full tag registry for all three statements |
| `financial_analysis/reconciliation.py` | Create — reconciliation pass + report dataclass |
| `financial_analysis/schema.py` | Modify — add IncomeStatement, BalanceSheet, CashFlow dataclasses; update PeriodFinancials |
| `financial_analysis/importer.py` | Modify — populate full schema from registry; call reconciliation before save |
| `tests/test_xbrl_tags.py` | Create — verify tag registry is complete and well-formed |
| `tests/test_reconciliation.py` | Create — unit tests for all 19 intra + 2 cross-statement checks |
| `tests/test_schema_expanded.py` | Create — unit tests for new sub-object dataclasses and derived fields |
| `tests/test_importer.py` | Modify — update mocks for expanded schema |

---

## Backwards Compatibility

The existing top-level fields on `PeriodFinancials` (`revenue`, `gross_profit`, `operating_income`, `net_income`, `operating_cash_flow`, `capex`, `free_cash_flow`, `fcf_margin`, `shares_outstanding`) are **kept as-is**. The importer continues to populate them. The DCF model continues to read from them. The new sub-objects are additive.

`store.py` serialises the new sub-objects into the JSON. `load()` deserialises them. The `get_series()` and `compare()` helpers continue to work on top-level fields; accessing sub-object fields requires `store.load('PANW').periods[-1].income_statement.revenue_total`.

---

## Reconciliation Timing

```
import_company()
    └── after schema population, before store.save()
        └── reconciliation.run(period, revenue_base)
              ├── warnings → printed/logged
              └── errors   → ReconciliationError raised, save blocked
```

Analysis code (`StochasticDCF`, `compare()`, `get_series()`) reads from already-saved, already-reconciled data and does not re-run reconciliation.
