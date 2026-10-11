"""Forward paper trading. Never sends broker orders.

Each bot gets a paper account that starts on its first ``step`` (the start date is
persisted). On every step we refresh data (the cache merges, so intraday history keeps
growing), replay the bot from a warm-up window with entries blocked before the paper
start, and report:

- the paper account (equity, open positions, trades since start) — synthetic fills on
  data the bot never saw while it was being designed, i.e. a true forward test;
- the bot's NEW signals queued on the latest bar, priced against the LIVE option chain
  (real bid/ask from yfinance, synthetic fallback) so you can see what it wants now.

State/ledger: ``data/processed/trading/paper/<bot>.json`` (gitignored).
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional

import pandas as pd

from trading.data import NY, load_market
from trading.engine import Backtester, BacktestResult, RiskConfig
from trading.pricing import LiveChainPricer
from trading.registry import discover
from trading.structures import MULTIPLIER

PAPER_DIR = Path(__file__).resolve().parent.parent / "data" / "processed" / "trading" / "paper"
WARMUP = {"1d": pd.DateOffset(years=2), "1h": pd.DateOffset(days=120), "5m": pd.DateOffset(days=30)}


@dataclass
class PaperReport:
    bot: str
    paper_start: pd.Timestamp
    as_of: pd.Timestamp
    result: BacktestResult
    signals: List[dict]

    def text(self) -> str:
        r, eq = self.result, self.result.equity
        start_eq = r.risk.starting_equity
        trades = [t for t in r.trades if t.entry_time >= self.paper_start]
        lines = [
            f"== {self.bot}  (paper since {self.paper_start.date()}, data through {self.as_of})",
            f"   equity ${eq.iloc[-1]:,.2f}  ({(eq.iloc[-1] / start_eq - 1) * 100:+.2f}%)  "
            f"closed trades {len(trades)}  realized ${sum(t.pnl for t in trades):+,.2f}",
        ]
        for p in r.open_positions:
            lines.append(f"   OPEN  {p.structure.describe()} x{p.units}  entry {p.entry_net:+.2f}  "
                         f"mark {p.mark:+.2f}  uP&L ${p.pnl_at(p.mark):+,.2f}")
        for t in trades[-5:]:
            lines.append(f"   DONE  {t.symbol} {t.structure} x{t.units} {t.entry_time:%m-%d %H:%M}→"
                         f"{t.exit_time:%m-%d %H:%M} ${t.pnl:+,.2f} ({t.exit_reason})")
        for s in self.signals:
            lines.append(f"   SIGNAL (next bar) {s['structure']}  live entry {s['entry']:+.2f}/unit "
                         f"(max loss ${s['max_loss_per_unit']:,.0f}/unit; {s['source']})")
        if r.rejections:
            lines.append(f"   rejections: {r.rejections}")
        return "\n".join(lines)


def _state_path(bot: str) -> Path:
    return PAPER_DIR / f"{bot}.json"


def paper_start_for(bot: str, start: Optional[str] = None) -> pd.Timestamp:
    """Persisted paper start; first call fixes it (today unless ``start`` is given)."""
    path = _state_path(bot)
    if path.exists():
        return pd.Timestamp(json.loads(path.read_text())["paper_start"])
    ts = pd.Timestamp(start or pd.Timestamp.now(tz=NY).date().isoformat(), tz=NY)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"paper_start": ts.isoformat(), "snapshots": []}, indent=1))
    return ts


def step(bot: str, risk: Optional[RiskConfig] = None, refresh: bool = True,
         start: Optional[str] = None, live_quotes: bool = True) -> PaperReport:
    cls = discover(include_private=True)[bot]
    strat = cls()
    paper_start = paper_start_for(bot, start)
    data_start = paper_start - WARMUP[strat.interval]
    if strat.start:
        data_start = max(data_start, pd.Timestamp(strat.start, tz=NY))
    md = load_market(strat.symbols, strat.interval, start=data_start.date().isoformat(), refresh=refresh)
    bt = Backtester(strat, md, risk=risk or RiskConfig(), trade_start=paper_start)
    result = bt.run()

    signals = []
    if bt.pending:
        pricer = LiveChainPricer() if live_quotes else None
        now = pd.Timestamp.now(tz=NY)
        for o in bt.pending:
            s = o.structure
            spot, vol = bt._spot[s.symbol][1][-1], bt.vol_at(s.symbol, len(md.index) - 1, "close")
            entry, source = 0.0, "synthetic"
            for leg in s.legs:
                if pricer is not None:
                    q = pricer.quote(s.symbol, leg.cp, leg.strike, s.expiry, now, spot, vol)
                    source = "live chain"
                else:
                    q = bt.pricer.quote(s.symbol, leg.cp, leg.strike, s.expiry, bt._time(len(md.index) - 1, "close"), spot, vol)
                entry += leg.qty * (q.ask if leg.qty > 0 else q.bid)
            signals.append({"structure": s.describe(), "entry": entry, "tag": o.tag, "source": source,
                            "max_loss_per_unit": s.max_loss(entry) * MULTIPLIER})

    report = PaperReport(bot, paper_start, md.index[-1] if len(md.index) else paper_start, result, signals)
    path = _state_path(bot)
    state = json.loads(path.read_text())
    state["snapshots"].append({
        "run_at": pd.Timestamp.now(tz=NY).isoformat(), "as_of": str(report.as_of),
        "equity": float(result.equity.iloc[-1]) if len(result.equity) else None,
        "open": [p.structure.describe() for p in result.open_positions], "signals": signals,
    })
    state["snapshots"] = state["snapshots"][-500:]
    path.write_text(json.dumps(state, indent=1, default=str))
    return report
