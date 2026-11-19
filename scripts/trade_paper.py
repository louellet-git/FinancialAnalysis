"""Forward paper-trade the SPY/QQQ option bots (no broker orders, ever).

    python -m scripts.trade_paper                       # all bots, one step
    python -m scripts.trade_paper s05_ic_0dte s08_trend_regime
    python -m scripts.trade_paper --no-refresh --no-live

Run it on a schedule during market hours (every 5 min for 5m bots, hourly for 1h,
after the close for 1d). Each bot's paper account starts on its first run; see
trading/paper.py.
"""
from __future__ import annotations

import argparse
import sys

from trading.engine import RiskConfig
from trading.paper import step
from trading.registry import discover


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("bots", nargs="*")
    ap.add_argument("--start", help="paper start date for bots without saved state (default today)")
    ap.add_argument("--no-refresh", action="store_true", help="use cached data only")
    ap.add_argument("--no-live", action="store_true", help="price signals synthetically, skip live chain")
    ap.add_argument("--equity", type=float, default=2000.0)
    ap.add_argument("--max-risk", type=float, metavar="DOLLARS",
                    help="risk up to this many $ per trade (e.g. 250); overrides --risk")
    ap.add_argument("--risk", type=float, default=0.03)
    args = ap.parse_args(argv)

    bots = args.bots or list(discover())
    risk = (RiskConfig.with_max_risk(args.max_risk, args.equity) if args.max_risk
            else RiskConfig(starting_equity=args.equity, risk_per_trade=args.risk))
    rc = 0
    for b in bots:
        try:
            print(step(b, risk, refresh=not args.no_refresh, start=args.start, live_quotes=not args.no_live).text())
        except Exception as exc:
            print(f"== {b}: FAILED {exc!r}", file=sys.stderr)
            rc = 1
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
