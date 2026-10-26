"""CLI: investor verdict for a ticker with a driver model.

Runs analysis/<TICKER>/driver_model.py through the driver engine and prints
the quartiles, the chance the price is too high and the verdict band
(financial_analysis/verdict.py). No network, no files written; the price is
an input because the JSON store holds no market data
(docs/HIDDEN-ASSUMPTIONS.md #6).

Usage:
    python -m scripts.verdict IBM --price 212.40
"""
import argparse
import importlib
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import numpy as np

from financial_analysis.driver_dcf import DriverDCF
from financial_analysis.verdict import chance_price_too_high, verdict_band


def main() -> None:
    parser = argparse.ArgumentParser(description="Investor verdict over the driver engine.")
    parser.add_argument("ticker", help="Ticker with analysis/<TICKER>/driver_model.py")
    parser.add_argument("--price", type=float, required=True, help="Market price per share")
    parser.add_argument("--simulations", type=int, default=10_000)
    parser.add_argument("--seed", type=int, default=7)
    args = parser.parse_args()

    ticker = args.ticker.upper()
    try:
        build = importlib.import_module(f"analysis.{ticker}.driver_model").build
    except ModuleNotFoundError:
        print(f"No driver model for {ticker}: expected analysis/{ticker}/driver_model.py")
        sys.exit(1)
    bundle = build()
    paths = DriverDCF(bundle["model"], bundle["segments"], net_debt=bundle["net_debt"],
                      shares=bundle["shares"]).run(args.simulations, args.seed).per_share_paths

    p10, p25, p50, p75, p90 = np.percentile(paths, [10, 25, 50, 75, 90])
    too_high = chance_price_too_high(paths, args.price)
    print(f"\n{ticker} verdict - {args.simulations:,} sims, seed {args.seed}, price ${args.price:,.2f}")
    print(f"  P10 ${p10:,.2f} | P25 ${p25:,.2f} | P50 ${p50:,.2f} | P75 ${p75:,.2f} | P90 ${p90:,.2f}")
    print(f"  Chance the price is too high: {too_high:.1%} -> {verdict_band(too_high)}")


if __name__ == "__main__":
    main()
