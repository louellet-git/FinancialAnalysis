"""CLI: investor verdict for a ticker with a driver model.

Runs analysis/<TICKER>/driver_model.py through the driver engine and prints
the quartiles, the chance the price is too high and the verdict band
(financial_analysis/verdict.py). No files written; the price is an input
because the JSON store holds no market data (docs/HIDDEN-ASSUMPTIONS.md #6).

Without --rf-date there is no network access and the model's own rf is used.
With --rf-date the rf driver mean is set to the U.S. Treasury 10-year par
yield on that date (financial_analysis/rates.py), and the output shows the
inflation split of that rate and a terminal-growth check. Only rf changes:
the growth drivers are nominal and stay at the model values.

With --rating (needs --rf-date) the kd_spread driver mean is set to the ICE BofA
index effective yield for that rating minus the UST 10Y on the same date
(financial_analysis/bonds.py), so the mean pre-tax cost of debt is the market
yield for the rating.

Usage:
    python -m scripts.verdict IBM --price 212.40
    python -m scripts.verdict IBM --price 212.40 --rf-date today --rf-base-date 2026-07-16
    python -m scripts.verdict JNJ --price 200 --rf-date today --rating AAA
    python -m scripts.verdict IBM --price 212.40 --rf-date today --json
"""
import argparse
import importlib
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import numpy as np

from financial_analysis.driver_dcf import DriverDCF
from financial_analysis.verdict import chance_price_too_high, verdict_band


def rates_context(model, rf_date: str, base_date: str | None = None):
    """(model with Treasury rf, dict of rate and inflation facts for the output)."""
    from financial_analysis import rates

    obs = rates.treasury_yield("10Y", on=rates.parse_date_arg(rf_date))
    model_rf = model.driver("rf").mean
    infl = rates.inflation_snapshot(obs.date)
    tg = model.driver("terminal_growth").mean
    fwd = infl["forward_5y5y"].value
    ctx = {
        "rf": obs.to_dict(),
        "model_rf_mean": model_rf,
        "rf_change_vs_model": round(obs.value - model_rf, 6),
        "real_10y": infl["real_10y"].value,
        "breakeven_10y": infl["breakeven_10y"].value,
        "forward_5y5y": fwd,
        "cpi_12m": infl["cpi_12m"].to_dict(),
        "core_cpi_12m": infl["core_cpi_12m"].to_dict(),
        "terminal_growth_mean": tg,
        "terminal_growth_real_vs_5y5y": round(tg - fwd, 6),
        "base": None,
    }
    if base_date:
        bd = rates.parse_date_arg(base_date)
        b_nom = rates.treasury_yield("10Y", on=bd)
        b_infl = rates.inflation_snapshot(b_nom.date)
        ctx["base"] = {
            "date": b_nom.date.isoformat(),
            "nominal_10y": b_nom.value,
            "nominal_change": round(obs.value - b_nom.value, 6),
            "real_change": round(ctx["real_10y"] - b_infl["real_10y"].value, 6),
            "breakeven_change": round(ctx["breakeven_10y"] - b_infl["breakeven_10y"].value, 6),
        }
    return rates.with_risk_free(model, obs), ctx


def credit_context(model, rating: str, rf_date: str, rf: float):
    """(model with the market kd_spread for ``rating``, dict of credit facts)."""
    from financial_analysis import bonds, rates

    on = rates.parse_date_arg(rf_date)
    spread = bonds.spread_over_10y(rating, on)
    index_yield = bonds.corporate_yield(rating, spread.date)
    oas = bonds.credit_spread(rating, spread.date)
    model_spread = model.driver("kd_spread").mean
    tax = model.driver("tax_rate").mean
    kd = round(rf + spread.value, 6)
    ctx = {
        "rating": rating,
        "index": bonds.rating_bucket(rating),
        "kd_spread": spread.to_dict(),
        "model_kd_spread": model_spread,
        "kd_spread_change_vs_model": round(spread.value - model_spread, 6),
        "index_yield": index_yield.value,
        "oas": oas.value,
        "moodys_aaa": bonds.moodys_yield("Aaa", spread.date).value,
        "moodys_baa": bonds.moodys_yield("Baa", spread.date).value,
        "kd_pretax": kd,
        "kd_after_tax": round(kd * (1 - tax), 6),
        "tax_rate": tax,
    }
    return bonds.with_kd_spread(model, spread), ctx


def _pp(x: float) -> str:
    return f"{x * 100:+.2f} pp"


def print_rates(ctx: dict) -> None:
    rf = ctx["rf"]
    print(f"  rf = UST 10Y {rf['value']:.2%} on {rf['date']} (U.S. Treasury); "
          f"model mean was {ctx['model_rf_mean']:.2%} ({_pp(ctx['rf_change_vs_model'])})")
    print(f"     = real 10Y {ctx['real_10y']:.2%} + breakeven inflation {ctx['breakeven_10y']:.2%}")
    b = ctx["base"]
    if b:
        print(f"  Since {b['date']} (10Y {b['nominal_10y']:.2%}): nominal {_pp(b['nominal_change'])}"
              f" = real {_pp(b['real_change'])} + inflation {_pp(b['breakeven_change'])}")
    cpi, core = ctx["cpi_12m"], ctx["core_cpi_12m"]
    print(f"  Inflation: 5y5y forward {ctx['forward_5y5y']:.2%}; CPI-U 12-month {cpi['value']:.2%}, "
          f"core {core['value']:.2%} (month {cpi['date'][:7]}, BLS)")
    if cpi["note"]:
        print(f"     note: {cpi['note']}")
    print(f"  Terminal growth mean {ctx['terminal_growth_mean']:.2%} = 5y5y inflation "
          f"{ctx['forward_5y5y']:.2%} + real {ctx['terminal_growth_real_vs_5y5y']:.2%}")
    print("  Only rf changed. The growth drivers are nominal and keep the model values.")


def print_credit(c: dict) -> None:
    s = c["kd_spread"]
    print(f"  kd_spread = {c['index']} index yield {c['index_yield']:.2%} - UST 10Y = {s['value']:+.2%} "
          f"on {s['date']} ({s['source']}); model was {c['model_kd_spread']:+.2%} "
          f"({_pp(c['kd_spread_change_vs_model'])})")
    if s["note"]:
        print(f"     note: {s['note']}")
    print(f"     {c['index']} OAS {c['oas']:.2%}; Moody's Aaa {c['moodys_aaa']:.2%}, Baa {c['moodys_baa']:.2%}")
    print(f"     pre-tax kd mean {c['kd_pretax']:.2%}; after tax ({c['tax_rate']:.0%}) {c['kd_after_tax']:.2%}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Investor verdict over the driver engine.")
    parser.add_argument("ticker", help="Ticker with analysis/<TICKER>/driver_model.py")
    parser.add_argument("--price", type=float, required=True, help="Market price per share")
    parser.add_argument("--simulations", type=int, default=10_000)
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--rf-date", help="YYYY-MM-DD or 'today': use the Treasury 10Y on that date as rf")
    parser.add_argument("--rf-base-date", help="YYYY-MM-DD: split the 10Y change since this date "
                                               "into real rate and inflation (needs --rf-date)")
    parser.add_argument("--rating", help="credit rating, S&P (A-) or Moody's (A3): use the market "
                                         "kd_spread for it (needs --rf-date)")
    parser.add_argument("--json", action="store_true", help="machine output")
    args = parser.parse_args()
    if not (math.isfinite(args.price) and args.price > 0):
        parser.error(f"--price must be a positive number, got {args.price}")
    if args.rf_base_date and not args.rf_date:
        parser.error("--rf-base-date needs --rf-date")
    if args.rating and not args.rf_date:
        parser.error("--rating needs --rf-date")

    ticker = args.ticker.upper()
    try:
        build = importlib.import_module(f"analysis.{ticker}.driver_model").build
    except ModuleNotFoundError:
        print(f"No driver model for {ticker}: expected analysis/{ticker}/driver_model.py", file=sys.stderr)
        sys.exit(1)
    bundle = build()
    model, ctx, credit = bundle["model"], None, None
    if args.rf_date:
        model, ctx = rates_context(model, args.rf_date, args.rf_base_date)
    if args.rating:
        model, credit = credit_context(model, args.rating, args.rf_date, ctx["rf"]["value"])
    paths = DriverDCF(model, bundle["segments"], net_debt=bundle["net_debt"],
                      shares=bundle["shares"]).run(args.simulations, args.seed).per_share_paths

    p10, p25, p50, p75, p90 = (float(v) for v in np.percentile(paths, [10, 25, 50, 75, 90]))
    too_high = float(chance_price_too_high(paths, args.price))
    band = verdict_band(too_high)
    if args.json:
        print(json.dumps({
            "ticker": ticker, "simulations": args.simulations, "seed": args.seed, "price": args.price,
            "p10": p10, "p25": p25, "p50": p50, "p75": p75, "p90": p90,
            "chance_price_too_high": too_high, "verdict": band, "rates": ctx, "credit": credit,
        }))
        return
    print(f"\n{ticker} verdict - {args.simulations:,} sims, seed {args.seed}, price ${args.price:,.2f}")
    if ctx:
        print_rates(ctx)
    if credit:
        print_credit(credit)
    print(f"  P10 ${p10:,.2f} | P25 ${p25:,.2f} | P50 ${p50:,.2f} | P75 ${p75:,.2f} | P90 ${p90:,.2f}")
    print(f"  Chance the price is too high: {too_high:.1%} -> {band}")


if __name__ == "__main__":
    main()
