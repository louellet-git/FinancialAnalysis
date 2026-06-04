import argparse
from datetime import datetime
from pathlib import Path

from financial_analysis.edgar import EdgarClient


def parse_date(value: str) -> datetime.date:
    for fmt in ("%Y-%m-%d", "%Y%m", "%Y%m%d"):
        try:
            return datetime.strptime(value, fmt).date()
        except ValueError:
            continue
    raise ValueError(f"Invalid date format: {value}. Use YYYY-MM-DD, YYYYMM, or YYYYMMDD")


def main() -> None:
    parser = argparse.ArgumentParser(description="Import SEC filings for a ticker by using SEC bulk full-index files.")
    parser.add_argument("ticker", help="Company ticker symbol, e.g. PANW")
    parser.add_argument("valuation_date", help="Valuation date for the target period")
    parser.add_argument(
        "--output-dir",
        default="data/raw",
        help="Base directory for downloaded filing assets",
    )
    parser.add_argument(
        "--lookback-months",
        type=int,
        default=18,
        help="How many months before the valuation date to consider when selecting filings",
    )
    parser.add_argument(
        "--lookahead-months",
        type=int,
        default=6,
        help="How many months after the valuation date to consider when selecting filings",
    )
    args = parser.parse_args()

    valuation_date = parse_date(args.valuation_date)
    output_root = Path(args.output_dir).resolve()
    client = EdgarClient()

    result = client.download_for_ticker(
        args.ticker,
        valuation_date,
        output_root,
        lookback_months=args.lookback_months,
        lookahead_months=args.lookahead_months,
        allowed_extensions=[".xml", ".xbrl", ".xsd", ".htm", ".html", ".txt", ".zip"],
    )

    print(f"Ticker: {result['ticker']}")
    print(f"CIK: {result['cik']}")
    print(f"Valuation date: {result['valuation_date']}")
    print(f"Selected filings: {len(result['selected_filings'])}")
    print(f"Downloaded files: {len(result['downloaded_files'])}")
    print(f"Files saved under: {output_root / args.ticker.upper()}")


if __name__ == "__main__":
    main()
