"""CLI: export financial statements for a ticker to an eight-tab xlsx file.

Usage:
    python -m scripts.analyze NVDA
    python -m scripts.analyze NVDA --refresh
    python -m scripts.analyze NVDA --no-market
    python -m scripts.analyze NVDA --no-options
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
from financial_analysis.fmp import FMPClientError, load_estimates, load_surprises, load_fmp_statements
from financial_analysis.market_data import load_earnings_surprises, load_prices
from financial_analysis.options import compute_chain_metrics, fetch_option_chain
from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.xbrl_tags import STATEMENT_MAP, UNIT_MAP

COLUMNS = [
    "ticker", "cik", "company_name", "period_end", "form_type",
    "filed_date", "statement", "metric", "value", "unit",
]

PRICE_COLUMNS = ["ticker", "date", "open", "high", "low", "close", "volume"]
ESTIMATE_COLUMNS = ["ticker", "period_end", "period_type", "metric", "value", "unit"]
OPTION_COLUMNS = [
    "ticker", "snapshot_date", "expiry", "option_type", "contract_symbol",
    "strike", "last_price", "bid", "ask", "volume", "open_interest",
    "implied_volatility", "in_the_money",
]
OPTION_SUMMARY_COLUMNS = [
    "ticker", "snapshot_date", "spot_price", "expiry", "atm_strike", "atm_iv",
    "straddle_mid", "implied_move", "skew", "call_volume", "put_volume",
    "call_open_interest", "put_open_interest",
    "put_call_volume_ratio", "put_call_oi_ratio",
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
        "Prices": [],
        "Estimates": [],
        "Options": [],
        "Options Summary": [],
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

        for concept, val in (p.raw_extras or {}).items():
            if val is not None:
                tabs["Raw"].append({
                    **base,
                    "statement": "raw",
                    "metric": concept,
                    "value": val,
                    "unit": "USD",
                })

    for pp in company.prices:
        tabs["Prices"].append({
            "ticker": company.ticker,
            "date": pp.date.isoformat(),
            "open": pp.open,
            "high": pp.high,
            "low": pp.low,
            "close": pp.close,
            "volume": pp.volume,
        })

    for est in company.estimates:
        for metric, val in [
            ("estimated_eps", est.estimated_eps),
            ("estimated_revenue", est.estimated_revenue),
        ]:
            if val is not None:
                tabs["Estimates"].append({
                    "ticker": company.ticker,
                    "period_end": est.period_end.isoformat(),
                    "period_type": est.period_type,
                    "metric": metric,
                    "value": val,
                    "unit": "USD/share" if "eps" in metric else "USD",
                })
        if est.number_of_analysts is not None:
            tabs["Estimates"].append({
                "ticker": company.ticker,
                "period_end": est.period_end.isoformat(),
                "period_type": est.period_type,
                "metric": "number_of_analysts",
                "value": est.number_of_analysts,
                "unit": "count",
            })

    for surp in company.surprises:
        for metric, val in [
            ("actual_eps", surp.actual_eps),
            ("estimated_eps", surp.estimated_eps),
            ("surprise", surp.surprise),
            ("surprise_pct", surp.surprise_pct),
        ]:
            tabs["Estimates"].append({
                "ticker": company.ticker,
                "period_end": surp.date.isoformat(),
                "period_type": "surprise",
                "metric": metric,
                "value": val,
                "unit": "USD/share" if "eps" in metric or metric == "surprise" else "ratio",
            })

    snap = company.option_chain
    if snap is not None:
        for q in snap.quotes:
            tabs["Options"].append({
                "ticker": snap.ticker,
                "snapshot_date": snap.snapshot_date.isoformat(),
                "expiry": q.expiry.isoformat(),
                "option_type": q.option_type,
                "contract_symbol": q.contract_symbol,
                "strike": q.strike,
                "last_price": q.last_price,
                "bid": q.bid,
                "ask": q.ask,
                "volume": q.volume,
                "open_interest": q.open_interest,
                "implied_volatility": q.implied_volatility,
                "in_the_money": q.in_the_money,
            })

        chain_metrics = compute_chain_metrics(snap)
        for em in chain_metrics.expiries:
            tabs["Options Summary"].append({
                "ticker": snap.ticker,
                "snapshot_date": snap.snapshot_date.isoformat(),
                "spot_price": snap.spot_price,
                "expiry": em.expiry.isoformat(),
                "atm_strike": em.atm_strike,
                "atm_iv": em.atm_iv,
                "straddle_mid": em.straddle_mid,
                "implied_move": em.implied_move,
                "skew": em.skew,
                "call_volume": em.call_volume,
                "put_volume": em.put_volume,
                "call_open_interest": em.call_open_interest,
                "put_open_interest": em.put_open_interest,
                "put_call_volume_ratio": em.put_call_volume_ratio,
                "put_call_oi_ratio": em.put_call_oi_ratio,
            })
        if chain_metrics.expiries:
            tabs["Options Summary"].append({
                "ticker": snap.ticker,
                "snapshot_date": snap.snapshot_date.isoformat(),
                "spot_price": snap.spot_price,
                "expiry": "ALL",
                "call_volume": sum(em.call_volume for em in chain_metrics.expiries),
                "put_volume": sum(em.put_volume for em in chain_metrics.expiries),
                "call_open_interest": sum(em.call_open_interest for em in chain_metrics.expiries),
                "put_open_interest": sum(em.put_open_interest for em in chain_metrics.expiries),
                "put_call_volume_ratio": chain_metrics.put_call_volume_ratio,
                "put_call_oi_ratio": chain_metrics.put_call_oi_ratio,
            })

    return tabs


def write_xlsx(company: CompanyFinancials, out_path: Path) -> int:
    wb = openpyxl.Workbook()
    wb.remove(wb.active)

    tabs = _flatten(company)
    total_rows = 0

    tab_columns = {
        "Prices": PRICE_COLUMNS,
        "Estimates": ESTIMATE_COLUMNS,
        "Options": OPTION_COLUMNS,
        "Options Summary": OPTION_SUMMARY_COLUMNS,
    }

    for tab_name, rows in tabs.items():
        ws = wb.create_sheet(tab_name)
        cols = tab_columns.get(tab_name, COLUMNS)
        ws.append(cols)
        for row in rows:
            ws.append([row.get(c, "") for c in cols])
            total_rows += 1

    out_path.parent.mkdir(parents=True, exist_ok=True)
    wb.save(str(out_path))
    return total_rows


def main() -> None:
    parser = argparse.ArgumentParser(description="Export financial statements to xlsx.")
    parser.add_argument("ticker", help="Stock ticker symbol (e.g. NVDA)")
    parser.add_argument("--refresh", action="store_true", help="Force re-fetch from SEC API")
    parser.add_argument("--out", type=Path, help="Output file path (default: data/processed/TICKER_financials.xlsx)")
    parser.add_argument("--no-market", action="store_true", dest="no_market", help="Skip stock prices and analyst estimates")
    parser.add_argument("--no-options", action="store_true", dest="no_options", help="Skip the option-chain snapshot")
    args = parser.parse_args()

    ticker = args.ticker.upper()
    out_path = args.out or Path(f"data/processed/{ticker}_financials.xlsx")
    cache_dir = Path("data/raw")

    print(f"Importing {ticker}...")
    try:
        import_company(ticker, save=True, refresh=args.refresh)
    except UnsupportedFilingError as e:
        print(f"XBRL import failed ({e}). Trying FMP IFRS fallback...")
        try:
            load_fmp_statements(ticker)
        except NotImplementedError:
            print("IFRS via FMP not yet implemented. Exiting.")
            sys.exit(1)
    except EdgarClientError as e:
        print(f"SEC API error: {e}")
        sys.exit(1)

    store.clear_cache()
    company = store.load(ticker)

    if not args.no_market:
        print("Fetching stock prices (yfinance)...")
        try:
            company.prices = load_prices(ticker, cache_dir=cache_dir, refresh=args.refresh)
            print(f"  {len(company.prices)} price points loaded")
        except Exception as e:
            print(f"  Warning: price fetch failed — {e}")

        print("Fetching earnings surprises (yfinance)...")
        try:
            company.surprises = load_earnings_surprises(ticker, cache_dir=cache_dir, refresh=args.refresh)
            print(f"  {len(company.surprises)} earnings surprise records loaded")
        except Exception as e:
            print(f"  Warning: earnings fetch failed — {e}")

        print("Fetching analyst estimates (FMP, optional)...")
        try:
            company.estimates = load_estimates(ticker, cache_dir=cache_dir, refresh=args.refresh)
            fmp_surprises = load_surprises(ticker, cache_dir=cache_dir, refresh=args.refresh)
            if fmp_surprises:
                company.surprises = fmp_surprises
            print(f"  {len(company.estimates)} FMP estimate records loaded")
        except FMPClientError as e:
            print(f"  FMP unavailable ({e}) — using yfinance earnings only")

        if not args.no_options:
            print("Fetching option chain (yfinance)...")
            try:
                company.option_chain = fetch_option_chain(ticker, cache_dir=cache_dir, refresh=args.refresh)
                n_expiries = len({q.expiry for q in company.option_chain.quotes})
                print(f"  {len(company.option_chain.quotes)} option contracts across {n_expiries} expiries loaded")
            except Exception as e:
                print(f"  Warning: option-chain fetch failed — {e}")

    with_options = not args.no_market and not args.no_options
    n_tabs = 4 if args.no_market else (8 if with_options else 6)
    print(f"Writing {out_path} ...")
    total = write_xlsx(company, out_path)
    print(f"Done: {out_path} ({total} rows across {n_tabs} tabs)")


if __name__ == "__main__":
    main()
