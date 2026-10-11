"""Backtest the SPY/QQQ option bots and print a leaderboard.

    python -m scripts.trade_backtest --fetch            # download/merge market data
    python -m scripts.trade_backtest                    # all bots, leaderboard
    python -m scripts.trade_backtest s01_orb_0dte -v    # one bot, trade list

All option prices are SYNTHETIC (see trading/pricing.py). Outputs land in
data/processed/trading/ (gitignored).
"""
from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

import pandas as pd

from trading.data import load_market, prefetch
from trading.engine import Backtester, RiskConfig
from trading.metrics import format_summary, summarize
from trading.registry import discover

OUT_DIR = Path(__file__).resolve().parent.parent / "data" / "processed" / "trading"

LEADERBOARD_COLS = [
    "strategy", "interval", "start", "end", "final_equity",
    "all_return_pct", "all_max_dd_pct", "all_sharpe", "all_trades", "all_win_rate_pct", "all_profit_factor",
    "oos_return_pct", "oos_max_dd_pct", "oos_sharpe", "oos_trades", "oos_win_rate_pct", "oos_profit_factor",
    "rejected", "pdt_blocks",
]


def run_one(cls, md_cache: dict, risk: RiskConfig, start=None, end=None, is_fraction=0.7):
    strat = cls()
    key = (tuple(strat.symbols), strat.interval, start or strat.start, end)
    if key not in md_cache:
        md_cache[key] = load_market(strat.symbols, strat.interval, start=start or strat.start, end=end)
    result = Backtester(strat, md_cache[key], risk=risk).run()
    row = summarize(result, is_fraction)
    row["interval"] = strat.interval
    return result, row


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("strategies", nargs="*", help="bot names (default: all)")
    ap.add_argument("--fetch", action="store_true", help="download/merge market data and exit")
    ap.add_argument("--include-templates", action="store_true", help="also run _template bots")
    ap.add_argument("--start"), ap.add_argument("--end")
    ap.add_argument("--equity", type=float, default=2000.0)
    ap.add_argument("--max-risk", type=float, metavar="DOLLARS",
                    help="risk up to this many $ per trade (e.g. 250); overrides --risk")
    ap.add_argument("--risk", type=float, default=0.03, help="max loss per trade, fraction of equity")
    ap.add_argument("--commission", type=float, default=0.65, help="$ per contract per leg")
    ap.add_argument("--is-fraction", type=float, default=0.7)
    ap.add_argument("-v", "--verbose", action="store_true", help="print each bot's trades")
    args = ap.parse_args(argv)

    if args.fetch:
        for k, v in prefetch().items():
            print(f"{k:>14}: {v} rows")
        return 0

    bots = discover(include_private=args.include_templates)
    names = args.strategies or list(bots)
    missing = [n for n in names if n not in bots]
    if missing:
        print(f"unknown bots: {missing}\navailable: {list(bots)}", file=sys.stderr)
        return 2

    if args.max_risk:
        risk = RiskConfig.with_max_risk(args.max_risk, args.equity, commission_per_contract=args.commission)
    else:
        risk = RiskConfig(starting_equity=args.equity, risk_per_trade=args.risk,
                          commission_per_contract=args.commission)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    md_cache, rows = {}, []
    for n in names:
        t0 = time.time()
        try:
            result, row = run_one(bots[n], md_cache, risk, args.start, args.end, args.is_fraction)
        except Exception as exc:  # one broken bot must not sink the leaderboard
            print(f"[{n}] FAILED: {exc!r}", file=sys.stderr)
            continue
        rows.append(row)
        print(format_summary(row) + f"  ({time.time() - t0:.1f}s)")
        if result.rejections:
            print(f"  rejections: {result.rejections}")
        trades = result.trades_frame()
        trades.to_csv(OUT_DIR / f"{n}_trades.csv", index=False)
        result.equity.to_csv(OUT_DIR / f"{n}_equity.csv", header=["equity"])
        if args.verbose and len(trades):
            with pd.option_context("display.width", 200, "display.max_rows", 60):
                print(trades[["entry_time", "exit_time", "structure", "units", "entry_net", "exit_net", "pnl", "exit_reason"]].tail(60))
        print()

    if rows:
        board = pd.DataFrame(rows)[LEADERBOARD_COLS].sort_values("oos_sharpe", ascending=False)
        tag = f"_risk{int(args.max_risk)}" if args.max_risk else ""
        board.to_csv(OUT_DIR / f"leaderboard{tag}.csv", index=False)
        with pd.option_context("display.width", 250, "display.max_columns", 30, "display.float_format", "{:.2f}".format):
            print("LEADERBOARD (sorted by out-of-sample Sharpe; option prices are model-priced)")
            print(board.to_string(index=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
