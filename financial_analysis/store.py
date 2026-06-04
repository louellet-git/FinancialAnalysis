from __future__ import annotations
import json
from datetime import date, datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from financial_analysis.schema import CompanyFinancials, PeriodFinancials

_CACHE: Dict[str, CompanyFinancials] = {}

_DEFAULT_DATA_DIR = Path("data/processed")

# All valid metrics: dataclass fields + derived properties
_VALID_METRICS = {
    f for f in PeriodFinancials.__dataclass_fields__
} | {"free_cash_flow", "fcf_margin"}


def clear_cache() -> None:
    _CACHE.clear()


def save(company: CompanyFinancials, data_dir: Path = _DEFAULT_DATA_DIR) -> Path:
    data_dir.mkdir(parents=True, exist_ok=True)
    path = data_dir / f"{company.ticker.upper()}.json"

    def _period_to_dict(p: PeriodFinancials) -> dict:
        return {
            "ticker": p.ticker,
            "period_end": p.period_end.isoformat(),
            "form": p.form,
            "filed_date": p.filed_date.isoformat(),
            "currency": p.currency,
            "revenue": p.revenue,
            "gross_profit": p.gross_profit,
            "operating_income": p.operating_income,
            "net_income": p.net_income,
            "rd_expense": p.rd_expense,
            "sga_expense": p.sga_expense,
            "operating_cash_flow": p.operating_cash_flow,
            "capex": p.capex,
            "total_assets": p.total_assets,
            "total_liabilities": p.total_liabilities,
            "equity": p.equity,
            "shares_outstanding": p.shares_outstanding,
        }

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

    periods = [
        PeriodFinancials(
            ticker=p["ticker"],
            period_end=_to_date(p["period_end"]),
            form=p["form"],
            filed_date=_to_date(p["filed_date"]),
            currency=p.get("currency", "USD"),
            revenue=p.get("revenue"),
            gross_profit=p.get("gross_profit"),
            operating_income=p.get("operating_income"),
            net_income=p.get("net_income"),
            rd_expense=p.get("rd_expense"),
            sga_expense=p.get("sga_expense"),
            operating_cash_flow=p.get("operating_cash_flow"),
            capex=p.get("capex"),
            total_assets=p.get("total_assets"),
            total_liabilities=p.get("total_liabilities"),
            equity=p.get("equity"),
            shares_outstanding=p.get("shares_outstanding"),
        )
        for p in raw["periods"]
    ]

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
        raise ValueError(f"'{metric}' is not a valid metric. Valid options: {sorted(_VALID_METRICS)}")
    company = load(ticker, data_dir=data_dir)
    return [(p.period_end, getattr(p, metric)) for p in company.periods]


def compare(
    tickers: List[str],
    metric: str,
    data_dir: Path = _DEFAULT_DATA_DIR,
) -> Dict[str, List[Tuple[date, Optional[float]]]]:
    return {t.upper(): get_series(t, metric, data_dir=data_dir) for t in tickers}
