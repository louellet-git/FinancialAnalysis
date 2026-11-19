"""Performance statistics with an in-sample / out-of-sample split.

The split is chronological (default: first 70% of bars in-sample). Bots must be TUNED
on in-sample only; the out-of-sample columns are the honest read.
"""
from __future__ import annotations

import math
from typing import Dict, List, Optional

import numpy as np
import pandas as pd

from trading.engine import BacktestResult, Trade


def _equity_stats(equity: pd.Series) -> Dict[str, float]:
    if len(equity) < 2:
        return {"return_pct": 0.0, "cagr_pct": 0.0, "max_dd_pct": 0.0, "sharpe": 0.0}
    daily = equity.groupby(equity.index.date).last()
    rets = daily.pct_change().dropna()
    total = equity.iloc[-1] / equity.iloc[0] - 1
    years = max((equity.index[-1] - equity.index[0]).days / 365.25, 1 / 365.25)
    cagr = (equity.iloc[-1] / equity.iloc[0]) ** (1 / years) - 1 if equity.iloc[-1] > 0 else -1.0
    dd = (equity / equity.cummax() - 1).min()
    sharpe = float(rets.mean() / rets.std() * math.sqrt(252)) if len(rets) > 1 and rets.std() > 0 else 0.0
    return {"return_pct": 100 * total, "cagr_pct": 100 * cagr, "max_dd_pct": 100 * dd, "sharpe": sharpe}


def _trade_stats(trades: List[Trade]) -> Dict[str, float]:
    if not trades:
        return {"trades": 0, "win_rate_pct": 0.0, "profit_factor": 0.0, "avg_pnl": 0.0}
    pnl = np.array([t.pnl for t in trades])
    wins, losses = pnl[pnl > 0].sum(), -pnl[pnl < 0].sum()
    return {
        "trades": len(trades),
        "win_rate_pct": 100 * float((pnl > 0).mean()),
        "profit_factor": float(wins / losses) if losses > 0 else float("inf") if wins > 0 else 0.0,
        "avg_pnl": float(pnl.mean()),
    }


def split_point(result: BacktestResult, is_fraction: float = 0.7) -> Optional[pd.Timestamp]:
    if len(result.equity) == 0:
        return None
    return result.equity.index[int(len(result.equity) * is_fraction)]


def summarize(result: BacktestResult, is_fraction: float = 0.7) -> Dict[str, float]:
    eq = result.equity
    cut = split_point(result, is_fraction)
    out: Dict[str, float] = {"strategy": result.strategy}
    out["start"] = str(eq.index[0].date()) if len(eq) else ""
    out["end"] = str(eq.index[-1].date()) if len(eq) else ""
    out["oos_start"] = str(cut.date()) if cut is not None else ""
    for prefix, e, tr in (
        ("all_", eq, result.trades),
        ("is_", eq[eq.index < cut] if cut is not None else eq, [t for t in result.trades if cut is None or t.entry_time < cut]),
        ("oos_", eq[eq.index >= cut] if cut is not None else eq.iloc[0:0], [t for t in result.trades if cut is not None and t.entry_time >= cut]),
    ):
        for k, v in {**_equity_stats(e), **_trade_stats(tr)}.items():
            out[prefix + k] = v
    out["final_equity"] = float(eq.iloc[-1]) if len(eq) else result.risk.starting_equity
    out["rejected"] = sum(result.rejections.values())
    out["pdt_blocks"] = result.pdt_blocks
    return out


def format_summary(row: Dict[str, float]) -> str:
    return (
        f"{row['strategy']}: {row['start']}→{row['end']}  final ${row['final_equity']:,.0f}\n"
        f"  ALL  ret {row['all_return_pct']:+.1f}%  CAGR {row['all_cagr_pct']:+.1f}%  maxDD {row['all_max_dd_pct']:.1f}%  "
        f"Sharpe {row['all_sharpe']:.2f}  trades {row['all_trades']}  win {row['all_win_rate_pct']:.0f}%  PF {row['all_profit_factor']:.2f}\n"
        f"  OOS (from {row['oos_start']})  ret {row['oos_return_pct']:+.1f}%  maxDD {row['oos_max_dd_pct']:.1f}%  "
        f"Sharpe {row['oos_sharpe']:.2f}  trades {row['oos_trades']}  win {row['oos_win_rate_pct']:.0f}%  PF {row['oos_profit_factor']:.2f}\n"
        f"  rejected entries {row['rejected']}  PDT-blocked exits {row['pdt_blocks']}"
    )
