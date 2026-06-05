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

        # Canonical fields from STATEMENT_MAP (excluding derived metrics handled separately)
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
