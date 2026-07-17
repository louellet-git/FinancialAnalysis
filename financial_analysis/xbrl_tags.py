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
    # Precedence: InterestIncomeNonoperating (the most specific "interest earned,
    # non-operating" concept) > InvestmentIncomeInterest (interest on investments;
    # common for cash-rich tech filers) > InterestIncomeOther (catch-all "other
    # interest income"; IBM reports ONLY this one). Specific-before-generic so a
    # filer tagging both never gets the catch-all.
    "interest_income": TagSpec([
        "us-gaap/InterestIncomeNonoperating",
        "us-gaap/InvestmentIncomeInterest",
        "us-gaap/InterestIncomeOther",
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
    "proceeds_issuance_debt": TagSpec(["us-gaap/ProceedsFromIssuanceOfLongTermDebt"]),
    "repayment_of_debt": TagSpec([
        "us-gaap/RepaymentsOfLongTermDebt",
        "us-gaap/RepaymentsOfDebt",
    ], sign_flip=True),
    "net_cash_from_financing": TagSpec(["us-gaap/NetCashProvidedByUsedInFinancingActivities"]),
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
