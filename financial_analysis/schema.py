"""Legacy Python data classes of the company data (the Rust form is ``rust/src/company.rs``).

Rust is the only program that makes the standard variables (``rust/mapcheck``). These classes stay
only because the Python engines, the backtest and ``scripts/visualize_dcf.py`` are not yet in Rust. The
store file ``data/processed/<TICKER>.json`` is read through ``store.py``.
"""
from __future__ import annotations
from dataclasses import dataclass, field
from datetime import date
from typing import Any, Dict, List, Optional


@dataclass
class PricePoint:
    date: date
    open: float
    high: float
    low: float
    close: float
    volume: int


@dataclass
class OptionQuote:
    contract_symbol: str
    option_type: str  # "call" or "put"
    expiry: date
    strike: float
    last_price: Optional[float] = None
    bid: Optional[float] = None
    ask: Optional[float] = None
    volume: Optional[int] = None
    open_interest: Optional[int] = None
    implied_volatility: Optional[float] = None
    in_the_money: Optional[bool] = None


@dataclass
class OptionChainSnapshot:
    ticker: str
    snapshot_date: date
    spot_price: Optional[float] = None
    quotes: List[OptionQuote] = field(default_factory=list)


@dataclass
class PeriodFinancials:
    ticker: str
    period_end: date
    form: str
    filed_date: date
    currency: str = "USD"

    # The fields below are the standard variables. Their tags, formulas and scope
    # are in financial_analysis/standard_variables.toml (same names, same sequence).

    # --- Income Statement ---
    revenue: Optional[float] = None
    revenue_from_contracts: Optional[float] = None
    lease_income: Optional[float] = None
    investment_income_operating: Optional[float] = None
    net_interest_income: Optional[float] = None
    noninterest_income: Optional[float] = None
    cogs_total: Optional[float] = None
    gross_profit: Optional[float] = None
    rd_expense: Optional[float] = None
    sm_expense: Optional[float] = None
    ga_expense: Optional[float] = None
    sga_expense: Optional[float] = None
    advertising_expense: Optional[float] = None
    opex_amortization: Optional[float] = None
    opex_restructuring: Optional[float] = None
    goodwill_impairment: Optional[float] = None
    asset_impairment: Optional[float] = None
    operating_expenses_total: Optional[float] = None
    costs_and_expenses_total: Optional[float] = None
    operating_income: Optional[float] = None
    interest_income: Optional[float] = None
    interest_expense: Optional[float] = None
    interest_net_nonoperating: Optional[float] = None
    gains_losses_on_investments: Optional[float] = None
    other_nonop_income: Optional[float] = None
    nonoperating_income_total: Optional[float] = None
    equity_method_income: Optional[float] = None
    income_before_tax: Optional[float] = None
    income_before_tax_domestic: Optional[float] = None
    income_before_tax_foreign: Optional[float] = None
    income_tax_expense: Optional[float] = None
    net_income: Optional[float] = None
    net_income_incl_nci: Optional[float] = None
    net_income_noncontrolling: Optional[float] = None
    net_income_to_common: Optional[float] = None
    discontinued_operations_income: Optional[float] = None
    depreciation: Optional[float] = None
    da_total: Optional[float] = None
    sbc_total: Optional[float] = None
    eps_basic: Optional[float] = None
    eps_diluted: Optional[float] = None
    shares_basic: Optional[float] = None
    shares_diluted: Optional[float] = None

    # --- Balance Sheet ---
    cash_and_equivalents: Optional[float] = None
    short_term_investments: Optional[float] = None
    cash_and_short_term_investments: Optional[float] = None
    long_term_investments: Optional[float] = None
    equity_method_investments: Optional[float] = None
    accounts_receivable_net: Optional[float] = None
    inventory: Optional[float] = None
    prepaid_and_other_current_assets: Optional[float] = None
    prepaid_expenses: Optional[float] = None
    contract_assets_current: Optional[float] = None
    other_receivables_current: Optional[float] = None
    total_current_assets: Optional[float] = None
    ppe_gross: Optional[float] = None
    accumulated_depreciation: Optional[float] = None
    ppe_net: Optional[float] = None
    goodwill: Optional[float] = None
    finite_lived_intangibles_net: Optional[float] = None
    indefinite_lived_intangibles: Optional[float] = None
    intangibles_net: Optional[float] = None
    operating_lease_rou_assets: Optional[float] = None
    deferred_tax_assets: Optional[float] = None
    other_noncurrent_assets: Optional[float] = None
    total_assets: Optional[float] = None
    accounts_payable: Optional[float] = None
    accounts_payable_and_accrued: Optional[float] = None
    accrued_compensation: Optional[float] = None
    accrued_liabilities: Optional[float] = None
    income_taxes_payable: Optional[float] = None
    deferred_revenue_current: Optional[float] = None
    other_current_liabilities: Optional[float] = None
    other_accrued_liabilities: Optional[float] = None
    dividends_payable: Optional[float] = None
    interest_payable: Optional[float] = None
    total_current_liabilities: Optional[float] = None
    short_term_borrowings: Optional[float] = None
    long_term_debt_current: Optional[float] = None
    debt_current: Optional[float] = None
    long_term_debt: Optional[float] = None
    long_term_debt_total: Optional[float] = None
    total_debt: Optional[float] = None
    finance_lease_liabilities: Optional[float] = None
    finance_lease_liabilities_current: Optional[float] = None
    finance_lease_liabilities_noncurrent: Optional[float] = None
    operating_lease_liabilities_current: Optional[float] = None
    operating_lease_liabilities_noncurrent: Optional[float] = None
    operating_lease_liabilities: Optional[float] = None
    deferred_revenue_noncurrent: Optional[float] = None
    pension_liabilities_noncurrent: Optional[float] = None
    income_taxes_payable_noncurrent: Optional[float] = None
    asset_retirement_obligations_noncurrent: Optional[float] = None
    deferred_tax_liabilities: Optional[float] = None
    other_noncurrent_liabilities: Optional[float] = None
    total_liabilities: Optional[float] = None
    temporary_equity: Optional[float] = None
    retained_earnings: Optional[float] = None
    equity: Optional[float] = None
    noncontrolling_interest: Optional[float] = None
    total_equity_incl_nci: Optional[float] = None
    total_liabilities_and_equity: Optional[float] = None
    shares_outstanding: Optional[float] = None

    # --- Cash Flow ---
    operating_cash_flow: Optional[float] = None
    change_receivables: Optional[float] = None
    change_inventories: Optional[float] = None
    change_payables: Optional[float] = None
    change_deferred_revenue: Optional[float] = None
    deferred_income_tax: Optional[float] = None
    capex: Optional[float] = None
    capitalized_software: Optional[float] = None
    capex_total: Optional[float] = None
    purchases_of_intangibles: Optional[float] = None
    purchases_investments: Optional[float] = None
    proceeds_investments: Optional[float] = None
    acquisitions_net_of_cash: Optional[float] = None
    proceeds_divestitures: Optional[float] = None
    proceeds_sale_ppe: Optional[float] = None
    other_investing: Optional[float] = None
    net_cash_from_investing: Optional[float] = None
    repurchases_common_stock: Optional[float] = None
    taxes_paid_net_share_settlement: Optional[float] = None
    dividends_paid: Optional[float] = None
    proceeds_issuance_debt: Optional[float] = None
    repayment_of_debt: Optional[float] = None
    proceeds_short_term_debt: Optional[float] = None
    repayment_short_term_debt: Optional[float] = None
    net_short_term_borrowings: Optional[float] = None
    proceeds_stock_issuance: Optional[float] = None
    other_financing: Optional[float] = None
    net_cash_from_financing: Optional[float] = None
    interest_paid: Optional[float] = None
    income_taxes_paid: Optional[float] = None
    fx_effect_on_cash: Optional[float] = None
    net_change_in_cash: Optional[float] = None
    net_change_in_cash_before_fx: Optional[float] = None

    # --- Disclosures (point-in-time facts from the notes) ---
    # Total remaining performance obligation (RPO). The current/noncurrent
    # duration split is a text-only disclosure and is not extracted.
    remaining_performance_obligation: Optional[float] = None
    # Per-segment revenue {segment_name: value}. Always None when imported from
    # the SEC companyfacts API, which strips dimensional (segment-axis) facts —
    # see the importer.py module docstring. Extraction from filing-level XBRL
    # instance documents is ticketed (PLANNING/tickets.md, Ticket 7).
    segment_revenue: Optional[Dict[str, float]] = None

    # --- Source of each value: {field: XBRL tag, or "= formula"} ---
    sources: Dict[str, str] = field(default_factory=dict)

    # --- Unmapped XBRL concepts ---
    raw_extras: Dict[str, Any] = field(default_factory=dict)

    @property
    def free_cash_flow(self) -> Optional[float]:
        """Operating cash flow less capital expenditure.

        Capital expenditure is ``capex_total``: equipment and capitalized software
        (PLANNING/RULES.md R2). Purchased intangibles are not included. A period with
        no ``capex_total`` (made by hand, or from an old store file) uses ``capex``.
        """
        capex = self.capex_total if self.capex_total is not None else self.capex
        if self.operating_cash_flow is None or capex is None:
            return None
        # A cash outflow is stored as a negative number. abs() makes the subtraction
        # correct for either sign convention.
        return self.operating_cash_flow - abs(capex)

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
    prices: List[PricePoint] = field(default_factory=list)
    option_chain: Optional[OptionChainSnapshot] = None
