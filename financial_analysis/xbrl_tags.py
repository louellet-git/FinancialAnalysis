"""Registry of standard variables: the one mapping from XBRL tags to model inputs.

Each standard variable (a field of ``PeriodFinancials``) has one ``TagSpec``:

- ``concepts``: XBRL tags that mean the same thing, in a fixed sequence. For each
  period the first tag that has a fact gives the value. Put a tag in this list only
  if it has the same scope as the variable. A tag with a different scope (a part, or
  a total that includes more) gets its own variable.
- ``formulas``: used only when no tag has a fact. Each formula is a string of
  standard variables with ``+`` and ``-``, for example ``"revenue - cogs_total"``.
  A term that ends with ``?`` is optional: a missing value counts as zero, but one
  term of the formula must have a value. Formulas are tried in sequence.
  In a sum, two terms with the same value count one time (one item with two tags).
- ``sign_flip``: the value is stored as a negative number (a cash outflow).
- ``period``: ``"duration"`` (a flow over the period) or ``"instant"`` (a balance on
  the period end date). The default comes from the statement.
- ``core``: the variable is a core DCF input. ``scripts.mapping_audit`` reports the
  fill rate of the core variables.

The importer records where each value came from in ``PeriodFinancials.sources``
(a tag name, or the formula). Tags that are not in this registry stay available in
``PeriodFinancials.raw_extras``.

Limits: the SEC companyfacts API has no dimensional facts (segments, products,
regions) and no company-specific tags. Those need filing-level XBRL
(PLANNING/tickets.md, Ticket 7).
"""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class TagSpec:
    concepts: list[str]
    unit: str = "USD"
    sign_flip: bool = False
    formulas: list[str] = field(default_factory=list)
    period: Optional[str] = None  # filled in below from the statement
    core: bool = False


INCOME_STATEMENT_TAGS: dict[str, TagSpec] = {
    # Top line of the income statement. Revenues is the total; the contract-revenue
    # tags are used when the filer has no Revenues tag for the period.
    "revenue": TagSpec([
        "us-gaap/Revenues",
        "us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax",
        "us-gaap/RevenueFromContractWithCustomerIncludingAssessedTax",
        "us-gaap/SalesRevenueNet",
    ], core=True),
    "cogs_total": TagSpec([
        "us-gaap/CostOfRevenue",
        "us-gaap/CostOfGoodsAndServicesSold",
        "us-gaap/CostOfGoodsSold",
    ]),
    "gross_profit": TagSpec(["us-gaap/GrossProfit"], formulas=["revenue - cogs_total"]),
    "rd_expense": TagSpec([
        "us-gaap/ResearchAndDevelopmentExpense",
        "us-gaap/ResearchAndDevelopmentExpenseExcludingAcquiredInProcessCost",
        "us-gaap/ResearchAndDevelopmentExpenseSoftwareExcludingAcquiredInProcessCost",
    ]),
    "sm_expense": TagSpec([
        "us-gaap/SellingAndMarketingExpense",
        "us-gaap/SellingExpense",
    ]),
    "ga_expense": TagSpec(["us-gaap/GeneralAndAdministrativeExpense"]),
    "sga_expense": TagSpec(
        ["us-gaap/SellingGeneralAndAdministrativeExpense"],
        formulas=["sm_expense + ga_expense"],
    ),
    # A part of selling expense that many filers give in the notes.
    "advertising_expense": TagSpec([
        "us-gaap/AdvertisingExpense",
        "us-gaap/MarketingAndAdvertisingExpense",
    ]),
    "opex_amortization": TagSpec(["us-gaap/AmortizationOfIntangibleAssets"]),
    "opex_restructuring": TagSpec([
        "us-gaap/RestructuringCharges",
        "us-gaap/RestructuringCostsAndAssetImpairmentCharges",
    ]),
    "goodwill_impairment": TagSpec(["us-gaap/GoodwillImpairmentLoss"]),
    "asset_impairment": TagSpec(["us-gaap/AssetImpairmentCharges"]),
    # Operating expenses below gross profit (cost of revenue is not included).
    "operating_expenses_total": TagSpec(["us-gaap/OperatingExpenses"]),
    # All costs and expenses. For some filers this includes interest and other
    # non-operating items, thus no formula uses it.
    "costs_and_expenses_total": TagSpec(["us-gaap/CostsAndExpenses"]),
    "operating_income": TagSpec(
        ["us-gaap/OperatingIncomeLoss"],
        formulas=["gross_profit - operating_expenses_total"],
    ),
    # Precedence: InterestIncomeNonoperating (the most specific "interest earned,
    # non-operating" concept) > InvestmentIncomeInterest (interest on investments;
    # common for cash-rich tech filers) > InvestmentIncomeInterestAndDividend >
    # InterestIncomeOther (catch-all "other interest income"; IBM reports ONLY this
    # one). Specific-before-generic so a filer tagging both never gets the catch-all.
    "interest_income": TagSpec([
        "us-gaap/InterestIncomeNonoperating",
        "us-gaap/InvestmentIncomeInterest",
        "us-gaap/InvestmentIncomeInterestAndDividend",
        "us-gaap/InterestIncomeOther",
    ]),
    # Interest cost on debt. InterestExpenseOperating (banks, REITs) is not in the
    # list: for a bank it is a cost of the business, not a financing cost.
    # Fallback: cash interest paid, for a filer that has no standard interest-expense
    # tag (it uses a company-specific tag). This is an approximate value; the source
    # shows "= interest_paid".
    "interest_expense": TagSpec([
        "us-gaap/InterestExpense",
        "us-gaap/InterestExpenseNonoperating",
        "us-gaap/InterestExpenseDebt",
        "us-gaap/InterestAndDebtExpense",
    ], formulas=["interest_paid"], core=True),
    # Interest income less interest expense. Some filers give only this net number.
    "interest_net_nonoperating": TagSpec(
        ["us-gaap/InterestIncomeExpenseNonoperatingNet", "us-gaap/InterestIncomeExpenseNet"],
        formulas=["interest_income - interest_expense"],
    ),
    "gains_losses_on_investments": TagSpec(["us-gaap/GainLossOnInvestments"]),
    "other_nonop_income": TagSpec(["us-gaap/OtherNonoperatingIncomeExpense"]),
    "nonoperating_income_total": TagSpec(["us-gaap/NonoperatingIncomeExpense"]),
    "equity_method_income": TagSpec(["us-gaap/IncomeLossFromEquityMethodInvestments"]),
    "income_before_tax": TagSpec([
        "us-gaap/IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest",
        "us-gaap/IncomeLossFromContinuingOperationsBeforeIncomeTaxesMinorityInterestAndIncomeLossFromEquityMethodInvestments",
    ], formulas=["income_before_tax_domestic + income_before_tax_foreign"], core=True),
    # Split of pre-tax income from the tax note.
    "income_before_tax_domestic": TagSpec([
        "us-gaap/IncomeLossFromContinuingOperationsBeforeIncomeTaxesDomestic",
    ]),
    "income_before_tax_foreign": TagSpec([
        "us-gaap/IncomeLossFromContinuingOperationsBeforeIncomeTaxesForeign",
    ]),
    "income_tax_expense": TagSpec(["us-gaap/IncomeTaxExpenseBenefit"], core=True),
    # Net income that belongs to the shareholders of the parent company.
    "net_income": TagSpec(
        ["us-gaap/NetIncomeLoss"],
        formulas=["net_income_incl_nci - net_income_noncontrolling"],
        core=True,
    ),
    "net_income_incl_nci": TagSpec(["us-gaap/ProfitLoss"]),
    "net_income_noncontrolling": TagSpec([
        "us-gaap/NetIncomeLossAttributableToNoncontrollingInterest",
    ]),
    "discontinued_operations_income": TagSpec([
        "us-gaap/IncomeLossFromDiscontinuedOperationsNetOfTax",
        "us-gaap/IncomeLossFromDiscontinuedOperationsNetOfTaxAttributableToReportingEntity",
    ]),
    "depreciation": TagSpec(["us-gaap/Depreciation"]),
    "da_total": TagSpec([
        "us-gaap/DepreciationDepletionAndAmortization",
        "us-gaap/DepreciationAndAmortization",
        "us-gaap/DepreciationAmortizationAndAccretionNet",
    ], formulas=["depreciation + opex_amortization?"], core=True),
    "sbc_total": TagSpec([
        "us-gaap/ShareBasedCompensation",
        "us-gaap/AllocatedShareBasedCompensationExpense",
    ], core=True),
    "eps_basic": TagSpec(["us-gaap/EarningsPerShareBasic"], unit="USD/shares"),
    "eps_diluted": TagSpec(["us-gaap/EarningsPerShareDiluted"], unit="USD/shares"),
    "shares_basic": TagSpec(
        ["us-gaap/WeightedAverageNumberOfSharesOutstandingBasic"], unit="shares"
    ),
    "shares_diluted": TagSpec(
        ["us-gaap/WeightedAverageNumberOfDilutedSharesOutstanding"], unit="shares",
        core=True,
    ),
}

BALANCE_SHEET_TAGS: dict[str, TagSpec] = {
    # The second tag includes restricted cash; it is used only when the filer has
    # no cash-and-equivalents tag.
    "cash_and_equivalents": TagSpec([
        "us-gaap/CashAndCashEquivalentsAtCarryingValue",
        "us-gaap/CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalents",
    ], core=True),
    "short_term_investments": TagSpec([
        "us-gaap/ShortTermInvestments",
        "us-gaap/MarketableSecuritiesCurrent",
        "us-gaap/AvailableForSaleSecuritiesDebtSecuritiesCurrent",
        "us-gaap/AvailableForSaleSecuritiesCurrent",
    ], formulas=["cash_and_short_term_investments - cash_and_equivalents"]),
    "cash_and_short_term_investments": TagSpec(
        ["us-gaap/CashCashEquivalentsAndShortTermInvestments"],
        formulas=["cash_and_equivalents + short_term_investments"],
    ),
    "long_term_investments": TagSpec([
        "us-gaap/LongTermInvestments",
        "us-gaap/MarketableSecuritiesNoncurrent",
        "us-gaap/AvailableForSaleSecuritiesDebtSecuritiesNoncurrent",
        "us-gaap/OtherLongTermInvestments",
    ]),
    "equity_method_investments": TagSpec(["us-gaap/EquityMethodInvestments"]),
    "accounts_receivable_net": TagSpec([
        "us-gaap/AccountsReceivableNetCurrent",
        "us-gaap/ReceivablesNetCurrent",
        "us-gaap/AccountsNotesAndLoansReceivableNetCurrent",
        "us-gaap/AccountsAndOtherReceivablesNetCurrent",
    ]),
    "inventory": TagSpec(["us-gaap/InventoryNet"]),
    "prepaid_and_other_current_assets": TagSpec([
        "us-gaap/PrepaidExpenseAndOtherAssetsCurrent",
        "us-gaap/OtherAssetsCurrent",
    ]),
    "total_current_assets": TagSpec(["us-gaap/AssetsCurrent"]),
    "ppe_gross": TagSpec([
        "us-gaap/PropertyPlantAndEquipmentGross",
        "us-gaap/PropertyPlantAndEquipmentAndFinanceLeaseRightOfUseAssetBeforeAccumulatedDepreciationAndAmortization",
    ]),
    "accumulated_depreciation": TagSpec([
        "us-gaap/AccumulatedDepreciationDepletionAndAmortizationPropertyPlantAndEquipment",
        "us-gaap/PropertyPlantAndEquipmentAndFinanceLeaseRightOfUseAssetAccumulatedDepreciationAndAmortization",
    ]),
    # The second tag includes finance-lease assets (the filer shows one line).
    "ppe_net": TagSpec([
        "us-gaap/PropertyPlantAndEquipmentNet",
        "us-gaap/PropertyPlantAndEquipmentAndFinanceLeaseRightOfUseAssetAfterAccumulatedDepreciationAndAmortization",
    ], formulas=["ppe_gross - accumulated_depreciation"]),
    "goodwill": TagSpec(["us-gaap/Goodwill"]),
    "finite_lived_intangibles_net": TagSpec(["us-gaap/FiniteLivedIntangibleAssetsNet"]),
    "indefinite_lived_intangibles": TagSpec([
        "us-gaap/IndefiniteLivedIntangibleAssetsExcludingGoodwill",
    ]),
    "intangibles_net": TagSpec(
        ["us-gaap/IntangibleAssetsNetExcludingGoodwill"],
        formulas=["finite_lived_intangibles_net + indefinite_lived_intangibles?"],
    ),
    "operating_lease_rou_assets": TagSpec(["us-gaap/OperatingLeaseRightOfUseAsset"]),
    "deferred_tax_assets": TagSpec(["us-gaap/DeferredIncomeTaxAssetsNet"]),
    "other_noncurrent_assets": TagSpec(["us-gaap/OtherAssetsNoncurrent"]),
    "total_assets": TagSpec(["us-gaap/Assets"], core=True),
    "accounts_payable": TagSpec([
        "us-gaap/AccountsPayableCurrent",
        "us-gaap/AccountsPayableTradeCurrent",
    ]),
    # Some filers show payables and accrued liabilities as one line.
    "accounts_payable_and_accrued": TagSpec(
        ["us-gaap/AccountsPayableAndAccruedLiabilitiesCurrent"],
        formulas=["accounts_payable + accrued_liabilities"],
    ),
    "accrued_compensation": TagSpec(["us-gaap/EmployeeRelatedLiabilitiesCurrent"]),
    "accrued_liabilities": TagSpec(["us-gaap/AccruedLiabilitiesCurrent"]),
    "income_taxes_payable": TagSpec([
        "us-gaap/AccruedIncomeTaxesCurrent",
        "us-gaap/TaxesPayableCurrent",
    ]),
    "deferred_revenue_current": TagSpec([
        "us-gaap/DeferredRevenueCurrent",
        "us-gaap/ContractWithCustomerLiabilityCurrent",
    ]),
    "other_current_liabilities": TagSpec(["us-gaap/OtherLiabilitiesCurrent"]),
    "total_current_liabilities": TagSpec(["us-gaap/LiabilitiesCurrent"]),
    # --- Debt. Each tag scope is different; do not put them in one list. ---
    # Borrowings with an original term of one year or less.
    "short_term_borrowings": TagSpec([
        "us-gaap/ShortTermBorrowings",
        "us-gaap/CommercialPaper",
    ]),
    # The part of long-term debt that is due in one year.
    "long_term_debt_current": TagSpec([
        "us-gaap/LongTermDebtCurrent",
        "us-gaap/LongTermDebtAndCapitalLeaseObligationsCurrent",
        "us-gaap/ConvertibleDebtCurrent",
        "us-gaap/ConvertibleNotesPayableCurrent",
    ], formulas=["long_term_debt_total - long_term_debt"]),
    # All debt in current liabilities.
    "debt_current": TagSpec(
        ["us-gaap/DebtCurrent"],
        formulas=["short_term_borrowings? + long_term_debt_current?"],
    ),
    # Long-term debt in non-current liabilities (the part due in one year is not
    # included). For some filers the "AndCapitalLeaseObligations" tags include
    # finance leases; the audit identity for long-term debt shows the difference.
    # The convertible-debt tags are for filers whose only debt is convertible.
    "long_term_debt": TagSpec([
        "us-gaap/LongTermDebtNoncurrent",
        "us-gaap/LongTermDebtAndCapitalLeaseObligations",
        "us-gaap/LongTermNotesPayable",
        "us-gaap/ConvertibleDebtNoncurrent",
        "us-gaap/ConvertibleLongTermNotesPayable",
    ], formulas=["long_term_debt_total - long_term_debt_current"]),
    # Long-term debt, current part included.
    "long_term_debt_total": TagSpec([
        "us-gaap/LongTermDebt",
        "us-gaap/LongTermDebtAndCapitalLeaseObligationsIncludingCurrentMaturities",
    ], formulas=["long_term_debt + long_term_debt_current"]),
    # All interest-bearing debt. Operating leases are not included. Filers tag debt
    # in different ways and the tags can overlap, thus this value is a first
    # estimate. The audit compares the two formulas; when they disagree, examine the
    # debt note of the filing and put a reviewed value in scenarios.CAPITAL_OVERRIDES.
    "total_debt": TagSpec(
        ["us-gaap/DebtLongtermAndShorttermCombinedAmount",
         "us-gaap/DebtAndCapitalLeaseObligations"],
        formulas=[
            "debt_current? + long_term_debt",
            "short_term_borrowings? + long_term_debt_total",
        ],
        core=True,
    ),
    "finance_lease_liabilities": TagSpec(["us-gaap/FinanceLeaseLiability"]),
    "operating_lease_liabilities_current": TagSpec([
        "us-gaap/OperatingLeaseLiabilityCurrent",
    ]),
    "operating_lease_liabilities_noncurrent": TagSpec([
        "us-gaap/OperatingLeaseLiabilityNoncurrent",
    ]),
    "operating_lease_liabilities": TagSpec(
        ["us-gaap/OperatingLeaseLiability"],
        formulas=["operating_lease_liabilities_current? + operating_lease_liabilities_noncurrent"],
    ),
    "deferred_revenue_noncurrent": TagSpec([
        "us-gaap/DeferredRevenueNoncurrent",
        "us-gaap/ContractWithCustomerLiabilityNoncurrent",
    ]),
    "pension_liabilities_noncurrent": TagSpec([
        "us-gaap/PensionAndOtherPostretirementDefinedBenefitPlansLiabilitiesNoncurrent",
        "us-gaap/PensionAndOtherPostretirementAndPostemploymentBenefitPlansLiabilitiesNoncurrent",
        "us-gaap/DefinedBenefitPensionPlanLiabilitiesNoncurrent",
    ]),
    "deferred_tax_liabilities": TagSpec(["us-gaap/DeferredIncomeTaxLiabilitiesNet"]),
    "other_noncurrent_liabilities": TagSpec(["us-gaap/OtherLiabilitiesNoncurrent"]),
    "total_liabilities": TagSpec(
        ["us-gaap/Liabilities"],
        formulas=["total_liabilities_and_equity - total_equity_incl_nci - temporary_equity?"],
    ),
    # Equity outside permanent equity (for example redeemable noncontrolling interest).
    "temporary_equity": TagSpec([
        "us-gaap/RedeemableNoncontrollingInterestEquityCarryingAmount",
        "us-gaap/TemporaryEquityCarryingAmountIncludingPortionAttributableToNoncontrollingInterests",
    ]),
    "retained_earnings": TagSpec(["us-gaap/RetainedEarningsAccumulatedDeficit"]),
    # Equity that belongs to the shareholders of the parent company.
    "equity": TagSpec(
        ["us-gaap/StockholdersEquity"],
        formulas=["total_equity_incl_nci - noncontrolling_interest?"],
        core=True,
    ),
    "noncontrolling_interest": TagSpec(["us-gaap/MinorityInterest"]),
    "total_equity_incl_nci": TagSpec(
        ["us-gaap/StockholdersEquityIncludingPortionAttributableToNoncontrollingInterest"],
        formulas=["equity + noncontrolling_interest?"],
    ),
    "total_liabilities_and_equity": TagSpec(["us-gaap/LiabilitiesAndStockholdersEquity"]),
    "shares_outstanding": TagSpec(
        ["us-gaap/CommonStockSharesOutstanding"], unit="shares"
    ),
}

CASH_FLOW_TAGS: dict[str, TagSpec] = {
    "operating_cash_flow": TagSpec([
        "us-gaap/NetCashProvidedByUsedInOperatingActivities",
        "us-gaap/NetCashProvidedByUsedInOperatingActivitiesContinuingOperations",
    ], core=True),
    # Changes in working capital, as the filer reports them: a positive number is an
    # increase of the balance (for an asset, this uses cash).
    "change_receivables": TagSpec(["us-gaap/IncreaseDecreaseInAccountsReceivable"]),
    "change_inventories": TagSpec(["us-gaap/IncreaseDecreaseInInventories"]),
    "change_payables": TagSpec([
        "us-gaap/IncreaseDecreaseInAccountsPayable",
        "us-gaap/IncreaseDecreaseInAccountsPayableAndAccruedLiabilities",
    ]),
    "change_deferred_revenue": TagSpec([
        "us-gaap/IncreaseDecreaseInContractWithCustomerLiability",
        "us-gaap/IncreaseDecreaseInDeferredRevenue",
    ]),
    "deferred_income_tax": TagSpec(["us-gaap/DeferredIncomeTaxExpenseBenefit"]),
    "capex": TagSpec([
        "us-gaap/PaymentsToAcquireProductiveAssets",
        "us-gaap/PaymentsToAcquirePropertyPlantAndEquipment",
    ], sign_flip=True, core=True),
    "capitalized_software": TagSpec([
        "us-gaap/PaymentsToDevelopSoftware",
        "us-gaap/PaymentsForSoftware",
    ], sign_flip=True),
    "purchases_of_intangibles": TagSpec([
        "us-gaap/PaymentsToAcquireIntangibleAssets",
    ], sign_flip=True),
    "purchases_investments": TagSpec([
        "us-gaap/PaymentsToAcquireAvailableForSaleSecuritiesDebt",
        "us-gaap/PaymentsToAcquireInvestments",
        "us-gaap/PaymentsToAcquireMarketableSecurities",
        "us-gaap/PaymentsToAcquireShortTermInvestments",
    ], sign_flip=True),
    "proceeds_investments": TagSpec([
        "us-gaap/ProceedsFromMaturitiesPrepaymentsAndCallsOfAvailableForSaleSecurities",
        "us-gaap/ProceedsFromSaleMaturityAndCollectionsOfInvestments",
        "us-gaap/ProceedsFromSaleAndMaturityOfMarketableSecurities",
    ]),
    "acquisitions_net_of_cash": TagSpec([
        "us-gaap/PaymentsToAcquireBusinessesNetOfCashAcquired",
    ], sign_flip=True),
    "net_cash_from_investing": TagSpec(["us-gaap/NetCashProvidedByUsedInInvestingActivities"]),
    "repurchases_common_stock": TagSpec([
        "us-gaap/PaymentsForRepurchaseOfCommonStock",
        "us-gaap/PaymentsForRepurchaseOfEquity",
    ], sign_flip=True),
    "taxes_paid_net_share_settlement": TagSpec([
        "us-gaap/PaymentsRelatedToTaxWithholdingForShareBasedCompensation",
    ], sign_flip=True),
    # Precedence: PaymentsOfDividends (total cash dividends paid, all classes) >
    # PaymentsOfDividendsCommonStock (common-only cash paid; IBM reports ONLY this
    # one) > DividendsCommonStockCash (dividends DECLARED, an equity charge, not a
    # cash-flow line — last-resort proxy; for steady payers declared ~= paid).
    # First-wins merging means the declared proxy is used only for periods where
    # neither payments concept exists.
    "dividends_paid": TagSpec([
        "us-gaap/PaymentsOfDividends",
        "us-gaap/PaymentsOfDividendsCommonStock",
        "us-gaap/DividendsCommonStockCash",
    ], sign_flip=True),
    "proceeds_issuance_debt": TagSpec([
        "us-gaap/ProceedsFromIssuanceOfLongTermDebt",
        "us-gaap/ProceedsFromIssuanceOfDebt",
        "us-gaap/ProceedsFromIssuanceOfLongTermDebtAndCapitalSecuritiesNet",
        "us-gaap/ProceedsFromIssuanceOfSeniorLongTermDebt",
    ]),
    "repayment_of_debt": TagSpec([
        "us-gaap/RepaymentsOfLongTermDebt",
        "us-gaap/RepaymentsOfDebt",
        "us-gaap/RepaymentsOfLongTermDebtAndCapitalSecurities",
    ], sign_flip=True),
    "net_cash_from_financing": TagSpec(["us-gaap/NetCashProvidedByUsedInFinancingActivities"]),
    # Cash paid in the period (supplemental cash-flow data). These are not the
    # expense numbers of the income statement.
    "interest_paid": TagSpec(["us-gaap/InterestPaidNet", "us-gaap/InterestPaid"]),
    "income_taxes_paid": TagSpec(["us-gaap/IncomeTaxesPaidNet", "us-gaap/IncomeTaxesPaid"]),
}

# Point-in-time (instant) facts disclosed in the notes, not statement line items.
# remaining_performance_obligation: total RPO (RULES.md R1 — forward-looking
# disclosures bearing on growth drivers must be consumed). The current/noncurrent
# duration split is a text disclosure only — not extractable from XBRL numerics.
DISCLOSURE_TAGS: dict[str, TagSpec] = {
    "remaining_performance_obligation": TagSpec([
        "us-gaap/RevenueRemainingPerformanceObligation",
    ]),
}

for _tags, _period in (
    (INCOME_STATEMENT_TAGS, "duration"),
    (BALANCE_SHEET_TAGS, "instant"),
    (CASH_FLOW_TAGS, "duration"),
    (DISCLOSURE_TAGS, "instant"),
):
    for _spec in _tags.values():
        if _spec.period is None:
            _spec.period = _period

ALL_TAGS: dict[str, TagSpec] = {
    **INCOME_STATEMENT_TAGS,
    **BALANCE_SHEET_TAGS,
    **CASH_FLOW_TAGS,
    **DISCLOSURE_TAGS,
}

CORE_FIELDS: tuple[str, ...] = tuple(k for k, s in ALL_TAGS.items() if s.core)

# Tags that show that a period exists. A fiscal year (or a quarter) is in the
# output only if one of these tags has a fact of the correct length for it.
ANCHOR_CONCEPTS: tuple[str, ...] = (
    *INCOME_STATEMENT_TAGS["revenue"].concepts,
    "us-gaap/NetIncomeLoss",
    "us-gaap/ProfitLoss",
    "us-gaap/NetCashProvidedByUsedInOperatingActivities",
    "us-gaap/IncomeTaxExpenseBenefit",
)

# Material tags that have no standard variable on purpose. The audit does not
# report a tag that matches one of these patterns. Each entry: (regex, reason).
EXCLUDED_CONCEPTS: tuple[tuple[str, str], ...] = (
    (r"^ComprehensiveIncome|^OtherComprehensiveIncome|^AccumulatedOtherComprehensiveIncome",
     "other comprehensive income is not a DCF input"),
    (r"^IncomeTaxReconciliation|^UnrecognizedTaxBenefits|^DeferredTaxAssets|^DeferredTaxLiabilities"
     r"|^DeferredIncomeTaxLiabilities$|^(Current|Deferred)(Federal|Foreign|StateAndLocal)?(Income)?TaxExpenseBenefit$"
     r"|^IncomeTaxPaid|^EffectiveIncomeTaxRate|^DeferredFederal|^CurrentFederal|^CurrentForeign|^DeferredForeign",
     "tax note detail; tax expense, tax paid and the net deferred tax lines have variables"),
    (r"^LongTermDebtMaturities|^LesseeOperatingLeaseLiabilityPaymentsDue|^FinanceLeaseLiabilityPaymentsDue"
     r"|^ContractualObligation|^OperatingLeasesFutureMinimumPayments|^LesseeOperatingLeaseLiabilityUndiscounted"
     r"|^FinanceLeaseLiabilityUndiscounted|^UnrecordedUnconditionalPurchaseObligation|^PurchaseObligation",
     "maturity schedule in the notes; the balance has a variable"),
    (r"FairValue|^AssetsFairValueDisclosure|^DerivativeNotional|^Derivative|^AvailableForSale"
     r"|^DebtSecuritiesAvailableForSale|^EquitySecuritiesFvNi|^HeldToMaturity|^DebtInstrument(CarryingAmount|FaceAmount)",
     "fair-value or instrument note; the balance-sheet line has a variable"),
    (r"^CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalents",
     "cash-flow statement cash total; cash_and_equivalents has a variable"),
    (r"^StockIssuedDuringPeriod|^StockRepurchased|^AdjustmentsToAdditionalPaidInCapital|^AdditionalPaidInCapital"
     r"|^CommonStock|^TreasuryStock|^Dividends|^StockholdersEquityOther|^PreferredStock|^MinorityInterestDecrease",
     "equity statement detail; total equity has a variable"),
    (r"^NetIncomeLossAvailableToCommonStockholders|^NetIncomeLossAttributableToParentDiluted"
     r"|^IncomeLossFromContinuingOperations(IncludingPortionAttributableToNoncontrollingInterest)?$"
     r"|^NetIncomeLossIncludingPortionAttributableToNonredeemableNoncontrollingInterest$"
     r"|^UndistributedEarnings|^IncomeLossFromContinuingOperationsPerBasicShare",
     "net income for another group of holders; net_income has a variable"),
    (r"^FiniteLivedIntangibleAssets(Gross|AccumulatedAmortization)|^IntangibleAssetsGrossExcludingGoodwill"
     r"|^FiniteLivedIntangibleAssetsAmortizationExpense|^Goodwill(AcquiredDuringPeriod|Gross|Impaired|Translation|Other|Foreign|PeriodIncrease|WrittenOff|Purchase)",
     "intangible asset note detail; the net balances have variables"),
    (r"^AllocatedShareBasedCompensationExpenseNetOfTax|^EmployeeServiceShareBasedCompensation|^ShareBasedCompensationArrangement",
     "share-based compensation note; sbc_total has a variable"),
    (r"^DefinedBenefitPlan|^DefinedContributionPlan|^PensionAndOtherPostretirementBenefit",
     "pension note detail"),
    (r"^NoncurrentAssets$|^AssetsNoncurrent$|^LiabilitiesNoncurrent$|^Liabilities(AndStockholdersEquity)?Abstract",
     "subtotal that few filers give"),
    (r"^SupplierFinanceProgram|^ContractWithCustomerLiability(RevenueRecognized)?$|^RevenueRemainingPerformanceObligation"
     r"|^ContractWithCustomerAsset|^SegmentExpenditure|^SegmentReporting|^Noncash|^CapitalExpendituresIncurredButNotYetPaid"
     r"|^RightOfUseAssetObtained|^OperatingLeaseCost|^OperatingLeasePayments|^LeaseCost|^OperatingLeaseExpense",
     "note disclosure, not a statement line"),
    (r"^BusinessCombination|^BusinessAcquisition|^DisposalGroup|^AssetsOfDisposalGroup|^LiabilitiesOfDisposalGroup"
     r"|^AssetsHeldForSale",
     "acquisition or disposal note"),
    (r"^Cash$|^CashEquivalentsAtCarryingValue$|^RestrictedCash",
     "part of cash; cash_and_equivalents has a variable"),
    (r"^InventoryFinishedGoods|^InventoryWorkInProcess|^InventoryRawMaterials|^InventoryGross|^InventoryLIFOReserve"
     r"|^InventoryValuationReserves",
     "inventory note detail; inventory has a variable"),
    (r"^PropertyPlantAndEquipmentAdditions|^ConstructionInProgressGross|^Land$|^BuildingsAndImprovementsGross"
     r"|^MachineryAndEquipmentGross|^FurnitureAndFixturesGross|^LeaseholdImprovementsGross|^CapitalizedComputerSoftware",
     "property note detail; ppe_gross and ppe_net have variables"),
)

STATEMENT_MAP: dict[str, str] = {
    **{k: "income_statement" for k in INCOME_STATEMENT_TAGS},
    **{k: "balance_sheet" for k in BALANCE_SHEET_TAGS},
    **{k: "cash_flow" for k in CASH_FLOW_TAGS},
    **{k: "disclosure" for k in DISCLOSURE_TAGS},
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
