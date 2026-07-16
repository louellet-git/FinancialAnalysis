"""CLI: run a Monte Carlo stochastic DCF for a ticker.

Loads financials from data/processed/{TICKER}.json (run import_financials or
analyze first). No live SEC call is made at valuation time.

Usage:
    python -m scripts.run_dcf PANW
    python -m scripts.run_dcf PANW --simulations 50000 --seed 7
    python -m scripts.run_dcf PANW --wacc 0.09 --growth 0.18
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from financial_analysis import store
from financial_analysis.dcf import DCFAssumptions, StochasticDCF


def _fmt_b(v: float) -> str:
    return f"${v / 1e9:,.1f}B"


def main() -> None:
    parser = argparse.ArgumentParser(description="Monte Carlo stochastic DCF.")
    parser.add_argument("ticker", help="Stock ticker symbol (e.g. PANW)")
    parser.add_argument("--simulations", type=int, default=10_000)
    parser.add_argument("--years", type=int, default=10, help="Projection horizon")
    parser.add_argument("--growth", type=float, default=0.15, help="Mean revenue growth")
    parser.add_argument("--wacc", type=float, default=0.10, help="Mean discount rate")
    parser.add_argument("--terminal-growth", type=float, default=0.03)
    parser.add_argument("--seed", type=int, default=None, help="Seed for reproducibility")
    args = parser.parse_args()

    ticker = args.ticker.upper()
    try:
        company = store.load(ticker)
    except FileNotFoundError:
        print(f"No saved data for {ticker}. Run: python -m scripts.import_financials {ticker}")
        sys.exit(1)

    assumptions = DCFAssumptions(
        n_simulations=args.simulations,
        projection_years=args.years,
        growth_rate_mean=args.growth,
        wacc_mean=args.wacc,
        terminal_growth_mean=args.terminal_growth,
        seed=args.seed,
    )

    try:
        result = StochasticDCF(company, assumptions).run()
    except ValueError as e:
        print(f"Cannot value {ticker}: {e}")
        sys.exit(1)

    print(f"\n{company.company_name} ({ticker}) — Monte Carlo DCF")
    print(f"  {args.simulations:,} simulations · {args.years}y horizon · "
          f"growth {args.growth:.0%} · WACC {args.wacc:.0%}\n")
    print(f"  Enterprise/equity value (present value):")
    print(f"    P10 : {_fmt_b(result.p10_value)}")
    print(f"    P50 : {_fmt_b(result.median_value)}")
    print(f"    P90 : {_fmt_b(result.p90_value)}")
    print(f"    mean: {_fmt_b(result.mean_value)}  (std {_fmt_b(result.std_value)})")
    if result.median_per_share is not None:
        print(f"\n  P50 per share: ${result.median_per_share:,.2f}")
    else:
        print("\n  Per-share: N/A (no shares outstanding in annual data)")


if __name__ == "__main__":
    main()
