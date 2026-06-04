from __future__ import annotations
from dataclasses import dataclass, field
from datetime import date
from typing import List, Optional


@dataclass
class PeriodFinancials:
    ticker: str
    period_end: date
    form: str
    filed_date: date
    currency: str = "USD"
    revenue: Optional[float] = None
    gross_profit: Optional[float] = None
    operating_income: Optional[float] = None
    net_income: Optional[float] = None
    rd_expense: Optional[float] = None
    sga_expense: Optional[float] = None
    operating_cash_flow: Optional[float] = None
    capex: Optional[float] = None
    total_assets: Optional[float] = None
    total_liabilities: Optional[float] = None
    equity: Optional[float] = None
    shares_outstanding: Optional[float] = None

    @property
    def free_cash_flow(self) -> Optional[float]:
        if self.operating_cash_flow is None or self.capex is None:
            return None
        return self.operating_cash_flow - self.capex


@dataclass
class CompanyFinancials:
    ticker: str
    cik: str
    company_name: str
    periods: List[PeriodFinancials] = field(default_factory=list)
