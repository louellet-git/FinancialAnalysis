"""CLI: point-in-time vintage backtest of mechanical operating projections.

Loads financials from data/processed/{TICKER}.json (run import_financials or
analyze first). No network call is made.

Estimates are point-in-time (only periods filed by each vintage's as-of date);
actuals are joined as currently reported (restatements included). See
financial_analysis/backtest.py for the honesty mechanisms and their limits.

Usage:
    python -m scripts.backtest IBM
    python -m scripts.backtest IBM --horizon 3
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from financial_analysis import store
from financial_analysis.backtest import BacktestResult, run_backtest


def _b(v) -> str:
    return "      ." if v is None else f"{v / 1e9:7.1f}"


def _err(v) -> str:
    return "     ." if v is None else f"{v:+6.1f}"


def _band(v) -> str:
    if v is None:
        return "  ."
    return "yes" if v else " NO"


def _print_metric_table(result: BacktestResult, metric: str, flag_index: dict) -> None:
    label = "REVENUE" if metric == "revenue" else "FREE CASH FLOW"
    print(f"\n{label} ($B)")
    print(f"  {'vintage':<12}{'base':>6}{'target':>8}{'P10':>9}{'P50':>9}"
          f"{'P90':>9}{'actual':>9}{'err%':>8}{'band':>6}  flags")
    for v in result.vintages:
        for o in v.outcomes:
            if o.metric != metric:
                continue
            flags = ",".join(
                str(flag_index[f]) for f in v.params.data_flags) or "-"
            print(f"  {v.params.as_of.isoformat():<12}"
                  f"{v.params.base_fiscal_year:>6}"
                  f"{o.target_fiscal_year:>8}"
                  f"{_b(o.p10):>9}{_b(o.p50):>9}{_b(o.p90):>9}"
                  f"{_b(o.actual):>9}{_err(o.error_pct):>8}"
                  f"{_band(o.in_band):>6}  {flags}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Point-in-time vintage backtest (operating projections).")
    parser.add_argument("ticker", help="Stock ticker symbol (e.g. IBM)")
    parser.add_argument("--horizon", type=int, default=3,
                        help="Projection horizon in years (default 3)")
    parser.add_argument("--simulations", type=int, default=10_000)
    parser.add_argument("--seed", type=int, default=7)
    args = parser.parse_args()

    ticker = args.ticker.upper()
    try:
        company = store.load(ticker)
    except FileNotFoundError:
        print(f"No saved data for {ticker}. "
              f"Run: python -m scripts.import_financials {ticker}")
        sys.exit(1)

    result = run_backtest(company, horizon=args.horizon,
                          n=args.simulations, seed=args.seed)

    if not result.vintages:
        print(f"{ticker}: no usable vintages — history is too thin at every "
              "fiscal-year-end vintage date.")
        for as_of, reason in result.skipped:
            print(f"  {as_of.isoformat()}: {reason}")
        sys.exit(1)

    # Stable footnote numbering for data flags across both metric tables.
    flag_index: dict = {}
    for v in result.vintages:
        for f in v.params.data_flags:
            flag_index.setdefault(f, len(flag_index) + 1)

    print(f"\n{company.company_name} ({ticker}) -- point-in-time vintage backtest")
    print(f"  horizon {args.horizon}y | {args.simulations:,} paths | seed {args.seed}")
    print("  estimates: point-in-time (filed_date <= as-of) | "
          "actuals: as currently reported (may be restated)")

    for metric in ("revenue", "fcf"):
        _print_metric_table(result, metric, flag_index)

    inside, total = result.coverage()
    print()
    if total:
        print(f"  Coverage: {inside} of {total} realized outcomes inside the "
              f"P10-P90 band ({inside / total:.0%}; nominal 80%)")
    else:
        print("  Coverage: no realized outcomes yet — every target fiscal "
              "year is still unreported.")

    bias = result.bias_by_horizon()
    if bias:
        print("  Mean signed error (actual vs P50; + = model underestimated):")
        for h in sorted(bias):
            parts = [f"{metric} {mean:+.1f}% (n={n})"
                     for metric, (mean, n) in sorted(bias[h].items(), reverse=True)]
            print(f"    +{h}y: " + " | ".join(parts))

    if flag_index:
        print("  Data flags (inputs surfaced, not fixed):")
        for f, i in sorted(flag_index.items(), key=lambda kv: kv[1]):
            print(f"    [{i}] {f}")

    if result.skipped:
        print(f"  Skipped vintages ({len(result.skipped)} -- insufficient "
              "point-in-time history):")
        for as_of, reason in result.skipped:
            print(f"    {as_of.isoformat()}: {reason}")


if __name__ == "__main__":
    main()
