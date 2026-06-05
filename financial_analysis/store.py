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
            elif f.name == "raw_extras":
                kwargs[f.name] = p.get(f.name, {})
            elif f.name == "currency":
                kwargs[f.name] = p.get(f.name, "USD")
            else:
                kwargs[f.name] = p.get(f.name)
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
