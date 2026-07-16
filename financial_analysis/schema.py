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
class EarningsEstimate:
    period_end: date
    period_type: str  # "annual" or "quarterly"
    estimated_eps: Optional[float] = None
    estimated_revenue: Optional[float] = None
    number_of_analysts: Optional[int] = None


@dataclass
class EarningsSurprise:
    date: date
    actual_eps: float
    estimated_eps: float
    surprise: float
    surprise_pct: float


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
        # capex is a cash outflow; the importer stores it sign-flipped (negative).
        # abs() makes the subtraction correct for either sign convention.
        return self.operating_cash_flow - abs(self.capex)

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
    estimates: List[EarningsEstimate] = field(default_factory=list)
    surprises: List[EarningsSurprise] = field(default_factory=list)
