"""CLI: investor verdict for a ticker with a driver model.

Runs analysis/<TICKER>/driver_model.py through the driver engine and prints
the verdict layer (financial_analysis/verdict.py): quartiles, chance the price
is too high, verdict band, convergence-aware returns, chance of losing money,
and the price-implied mean of each stochastic driver. No network, no files
written; the price and dividend yield are inputs because the JSON store holds
no market data (docs/HIDDEN-ASSUMPTIONS.md #6).

Usage:
    python -m scripts.verdict IBM --price 212.40 --dividend-yield 0.0313
"""
import argparse
import importlib
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import numpy as np

from financial_analysis.driver_dcf import DriverDCF
from financial_analysis.verdict import (
    annual_return,
    chance_of_loss,
    chance_price_too_high,
    price_implied_mean,
    verdict_band,
)

CLOSURE_RATES = (0.15, 0.30)  # Capozza & Israelsen 2010: 15-30% of the gap per year
HORIZONS = (1, 3, 5)


def main() -> None:
    parser = argparse.ArgumentParser(description="Investor verdict over the driver engine.")
    parser.add_argument("ticker", help="Ticker with analysis/<TICKER>/driver_model.py")
    parser.add_argument("--price", type=float, required=True, help="Market price per share")
    parser.add_argument("--dividend-yield", type=float, default=0.0)
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
    price, dy = args.price, args.dividend_yield

    p10, p25, p50, p75, p90 = np.percentile(paths, [10, 25, 50, 75, 90])
    too_high = chance_price_too_high(paths, price)
    print(f"\n{ticker} verdict - {args.simulations:,} sims, seed {args.seed}, price ${price:,.2f}, "
          f"dividend yield {dy:.2%}")
    print(f"  P10 ${p10:,.2f} | P25 ${p25:,.2f} | P50 ${p50:,.2f} | P75 ${p75:,.2f} | P90 ${p90:,.2f}")
    print(f"  Chance the price is too high: {too_high:.1%} -> {verdict_band(too_high)}")

    print("\n  Return per year (gap closes k per year, dividends included):")
    for k in CLOSURE_RATES:
        for label, value in (("P10", p10), ("P50", p50), ("P90", p90)):
            cells = "  ".join(f"{h}y {annual_return(value, price, k, h, dy):+6.1%}" for h in HORIZONS)
            print(f"    k={k:.2f} {label}: {cells}")
    print("\n  Chance of losing money (dividends included):")
    for k in CLOSURE_RATES:
        cells = "  ".join(f"{h}y {chance_of_loss(paths, price, k, h, dy):5.1%}" for h in HORIZONS)
        print(f"    k={k:.2f}: {cells}")

    print("\n  What the price requires (one driver at a time, others at base):")
    for d in bundle["model"].stochastic_drivers:
        implied = price_implied_mean(bundle, d.name, price, seed=args.seed)
        shown = f"{implied:.4f}" if implied is not None else "not reachable within bounds"
        print(f"    {d.name:<18} base {d.mean:.4f} -> {shown}")


if __name__ == "__main__":
    main()
