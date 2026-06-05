"""CLI: import financial data for a ticker and save to data/processed/.

Usage:
    python -m scripts.import_financials NVDA
    python -m scripts.import_financials NVDA --refresh
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from financial_analysis.importer import UnsupportedFilingError, import_company
from financial_analysis.edgar import EdgarClientError


def main() -> None:
    parser = argparse.ArgumentParser(description="Import SEC financial data for a ticker.")
    parser.add_argument("ticker", help="Stock ticker symbol (e.g. NVDA)")
    parser.add_argument("--refresh", action="store_true", help="Force re-fetch from SEC API")
    args = parser.parse_args()

    ticker = args.ticker.upper()
    print(f"Importing {ticker}...")

    try:
        company = import_company(ticker, save=True, refresh=args.refresh)
    except UnsupportedFilingError as e:
        print(f"Error: {e}")
        sys.exit(1)
    except EdgarClientError as e:
        print(f"SEC API error: {e}")
        sys.exit(1)

    annual = [p for p in company.periods if p.form == "10-K"]
    quarterly = [p for p in company.periods if p.form == "10-Q"]

    print(f"  Company:   {company.company_name}")
    print(f"  CIK:       {company.cik}")
    print(f"  Annual periods (10-K):    {len(annual)}")
    print(f"  Quarterly periods (10-Q): {len(quarterly)}")
    print(f"  Saved: data/processed/{ticker}.json")
    print(f"  Cache: data/raw/{ticker}/company_facts.json")


if __name__ == "__main__":
    main()
