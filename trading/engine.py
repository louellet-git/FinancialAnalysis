"""Event-driven options backtester with account-level risk rules.

One strategy = one isolated account. Per bar:
  1. fill entries queued on the previous bar at THIS bar's open (sized by risk rules)
  2. mark open positions at close; fire ExitRules exits (PDT-checked)
  3. settle positions expiring today at the session's last bar (intrinsic)
  4. call ``strategy.on_bar(ctx)``
  5. record equity (mid marks)

Risk rules (``RiskConfig``) default to the user's paper account: $2,000, 3% of equity
max loss per trade, defined-risk structures only, pattern-day-trader limit enforced
(< $25k: max 3 same-day round trips per 5 sessions; a blocked exit is retried each bar
and otherwise the position rides to expiry — expiration is not a day trade).
"""
from __future__ import annotations

import math
from collections import deque
from dataclasses import dataclass, field
from datetime import date
from typing import Dict, List, Optional

import numpy as np
import pandas as pd

from trading.data import MarketData, bar_close_time, bar_open_time
from trading.pricing import OptionPricer, Quote, SyntheticPricer, VolSnapshot, year_fraction
from trading.strategy import ExitRules, Strategy
from trading.structures import MULTIPLIER, Structure


@dataclass
class RiskConfig:
    """Account rules. ``with_max_risk(250)`` gives the "risk up to $250/trade" mode."""

    starting_equity: float = 2_000.0
    risk_per_trade: float = 0.03        # max loss per trade as fraction of equity
    max_risk_dollars: Optional[float] = None  # hard $ cap on top of the fraction
    max_open_positions: int = 3
    max_total_risk: float = 0.12        # sum of open max-loss / equity
    daily_loss_limit: float = 0.06      # no new entries once down this much today
    max_units: int = 20
    commission_per_contract: float = 0.65
    pdt: bool = True
    pdt_equity_threshold: float = 25_000.0
    pdt_max_day_trades: int = 3         # per rolling 5 sessions

    @classmethod
    def with_max_risk(cls, dollars: float, starting_equity: float = 2_000.0, **kw) -> "RiskConfig":
        """Per-trade max loss of ``dollars`` at the starting equity, scaling down (never
        up past ``dollars``) with equity; total open risk allows ~3 such trades (<= 50%)."""
        frac = dollars / starting_equity
        return cls(starting_equity=starting_equity, risk_per_trade=frac, max_risk_dollars=dollars,
                   max_total_risk=min(0.5, 3 * frac), **kw)


@dataclass
class Position:
    id: int
    structure: Structure
    units: int
    entry_time: pd.Timestamp
    entry_net: float            # per share per unit: +debit paid / -credit received
    max_loss: float             # $ for the whole position incl. commissions
    exits: ExitRules
    tag: str = ""
    bars_held: int = 0
    mark: float = 0.0           # mid net value per share per unit

    @property
    def premium(self) -> float:
        return abs(self.entry_net)

    def pnl_at(self, net_value: float) -> float:
        """$ P&L before exit commission if liquidated at ``net_value``."""
        return (net_value - self.entry_net) * MULTIPLIER * self.units


@dataclass
class Trade:
    id: int
    strategy: str
    symbol: str
    structure: str
    tag: str
    units: int
    entry_time: pd.Timestamp
    exit_time: pd.Timestamp
    entry_net: float
    exit_net: float
    max_loss: float
    pnl: float                  # $ net of all commissions
    exit_reason: str


@dataclass
class _Pending:
    structure: Structure
    exits: ExitRules
    tag: str
    max_units: Optional[int]


class Context:
    """What a strategy sees. Everything is as of the current bar close."""

    def __init__(self, engine: "Backtester"):
        self._e = engine
        self.state: dict = {}

    # --- time ------------------------------------------------------------
    @property
    def i(self) -> int:
        return self._e.i

    @property
    def ts(self) -> pd.Timestamp:
        return self._e.index[self._e.i]

    @property
    def now(self) -> pd.Timestamp:
        return bar_close_time(self.ts, self._e.md.interval)

    @property
    def today(self) -> date:
        return self.ts.date()

    @property
    def is_first_bar_of_day(self) -> bool:
        return self._e.first_of_day[self._e.i]

    @property
    def is_last_bar_of_day(self) -> bool:
        return self._e.last_of_day[self._e.i]

    @property
    def minutes_to_close(self) -> float:
        return (self.now.normalize() + pd.Timedelta(hours=16) - self.now).total_seconds() / 60.0

    # --- market data (no lookahead) ---------------------------------------
    def bars(self, symbol: Optional[str] = None) -> pd.DataFrame:
        """Bars up to and including the current one."""
        return self._e.md.bars[symbol or self._e.symbols[0]].iloc[: self._e.i + 1]

    def today_bars(self, symbol: Optional[str] = None) -> pd.DataFrame:
        start = self._e.day_start[self._e.i]
        return self._e.md.bars[symbol or self._e.symbols[0]].iloc[start: self._e.i + 1]

    def daily(self, symbol: Optional[str] = None) -> pd.DataFrame:
        """Completed daily bars strictly before today (for intraday strategies)."""
        d = self._e.md.daily[symbol or self._e.symbols[0]]
        return d[d.index < self.ts.normalize()]

    def spot(self, symbol: Optional[str] = None) -> float:
        return float(self._e.md.bars[symbol or self._e.symbols[0]]["close"].iat[self._e.i])

    def vol_history(self, symbol: Optional[str] = None) -> pd.DataFrame:
        """Vol per bar up to now (decimals): atm_* = VIX/VXN, short_* = VIX1D-based."""
        return self._e.md.vol[symbol or self._e.symbols[0]].iloc[: self._e.i + 1]

    def vol(self, symbol: Optional[str] = None) -> VolSnapshot:
        return self._e.vol_at(symbol or self._e.symbols[0], self._e.i, "close")

    # --- options helpers ---------------------------------------------------
    def expiry(self, sessions_ahead: int = 0) -> date:
        """Expiry date ``sessions_ahead`` trading days from today (0 = 0DTE).

        Assumes daily expirations (true for SPY/QQQ since 2022; earlier history had
        only M/W/F or weekly expiries — synthetic pricing does not model that).
        """
        return np.busday_offset(self.today, sessions_ahead, roll="forward").astype(object)

    def quote(self, symbol: str, cp: str, strike: float, expiry: date) -> Quote:
        return self._e.pricer.quote(symbol, cp, strike, expiry, self.now, self.spot(symbol), self.vol(symbol))

    def strike_for_delta(self, symbol: str, cp: str, target_delta: float, expiry: date,
                         increment: float = 1.0) -> float:
        """Strike whose |delta| is closest to ``target_delta`` (e.g. 0.30)."""
        spot = self.spot(symbol)
        d = lambda k: abs(self.quote(symbol, cp, k, expiry).delta)
        # |delta| is monotone in strike (falls for calls, rises for puts): bisect.
        lo, hi = spot * 0.5, spot * 1.5
        for _ in range(22):
            m = 0.5 * (lo + hi)
            if (d(m) > target_delta) == (cp == "C"):
                lo = m
            else:
                hi = m
        k0 = round(0.5 * (lo + hi) / increment) * increment
        cands = [round(k0 + j * increment, 2) for j in (-1, 0, 1) if k0 + j * increment > 0]
        return min(cands, key=lambda k: abs(d(k) - target_delta))

    def price(self, structure: Structure) -> Dict[str, float]:
        """Net per-share prices now: entry (pay ask/receive bid), mid, liquidation."""
        return self._e.net_prices(structure, self._e.i, "close")

    # --- account -----------------------------------------------------------
    @property
    def positions(self) -> List[Position]:
        return list(self._e.positions.values())

    @property
    def equity(self) -> float:
        return self._e.equity_now()

    @property
    def risk_budget(self) -> float:
        """Max $ loss the engine will allow on a new trade right now (incl. commissions)."""
        e = self._e
        budget = e.equity_now() * e.risk.risk_per_trade
        if e.risk.max_risk_dollars is not None:
            budget = min(budget, e.risk.max_risk_dollars)
        return budget

    @property
    def commission_per_contract(self) -> float:
        return self._e.risk.commission_per_contract

    @property
    def day_trades_left(self) -> int:
        """Same-day round trips still allowed under PDT in the rolling 5 sessions."""
        e = self._e
        if not e.risk.pdt or e.equity_now() >= e.risk.pdt_equity_threshold:
            return 10**9
        window_start = np.busday_offset(self.today, -4, roll="backward").astype(object)
        used = sum(1 for d in e._day_trades if d >= window_start)
        return max(e.risk.pdt_max_day_trades - used, 0)

    @property
    def pending_entries(self) -> int:
        return len(self._e.pending)

    # --- orders ------------------------------------------------------------
    def open(self, structure: Structure, exits: ExitRules = ExitRules(), tag: str = "",
             max_units: Optional[int] = None) -> None:
        """Queue an entry for the next bar's open. Sizing is the engine's job."""
        self._e.pending.append(_Pending(structure, exits, tag, max_units))

    def close(self, position: Position, reason: str = "signal") -> bool:
        """Close at this bar's close. Returns False if PDT-blocked."""
        return self._e.try_close(position, reason)


class Backtester:
    def __init__(self, strategy: Strategy, md: MarketData, risk: Optional[RiskConfig] = None,
                 pricer: Optional[OptionPricer] = None, trade_start: Optional[pd.Timestamp] = None):
        self.strategy = strategy
        self.md = md
        self.risk = risk or RiskConfig()
        self.pricer = pricer or SyntheticPricer()
        self.symbols = tuple(strategy.symbols)
        self.index = md.index
        self.trade_start = trade_start
        n = len(self.index)
        dates = np.array([t.date() for t in self.index])
        self.first_of_day = np.r_[True, dates[1:] != dates[:-1]] if n else np.array([], bool)
        self.last_of_day = np.r_[dates[1:] != dates[:-1], True] if n else np.array([], bool)
        self.day_start = np.maximum.accumulate(np.where(self.first_of_day, np.arange(n), 0)) if n else np.array([], int)
        self._spot = {s: (md.bars[s]["open"].to_numpy(float), md.bars[s]["close"].to_numpy(float)) for s in self.symbols}
        self._vol = {s: {c: md.vol[s][c].to_numpy(float) for c in md.vol[s].columns} for s in self.symbols}
        self._rate = md.rate.to_numpy(float)

        self.i = 0
        self.cash = self.risk.starting_equity
        self.positions: Dict[int, Position] = {}
        self.pending: List[_Pending] = []
        self.trades: List[Trade] = []
        self.equity_curve: List[float] = []
        self.rejections: Dict[str, int] = {}
        self.pdt_blocks = 0
        self._day_trades: deque = deque()  # dates of same-day round trips
        self._next_id = 1
        self._day_start_equity = self.cash
        self.ctx = Context(self)

    # --- pricing -------------------------------------------------------------
    def vol_at(self, symbol: str, i: int, when: str) -> VolSnapshot:
        v = self._vol[symbol]
        return VolSnapshot(v[f"atm_{when}"][i], v[f"short_{when}"][i], self._rate[i])

    def _time(self, i: int, when: str) -> pd.Timestamp:
        ts = self.index[i]
        return bar_open_time(ts, self.md.interval) if when == "open" else bar_close_time(ts, self.md.interval)

    def net_prices(self, structure: Structure, i: int, when: str) -> Dict[str, float]:
        sym = structure.symbol
        spot = self._spot[sym][0 if when == "open" else 1][i]
        vol, now = self.vol_at(sym, i, when), self._time(i, when)
        buy = mid = sell = 0.0
        for leg in structure.legs:
            q = self.pricer.quote(sym, leg.cp, leg.strike, structure.expiry, now, spot, vol)
            mid += leg.qty * q.mid
            # entry: pay ask on longs, receive bid on shorts; exit: the reverse
            buy += leg.qty * (q.ask if leg.qty > 0 else q.bid)
            sell += leg.qty * (q.bid if leg.qty > 0 else q.ask)
        return {"entry": buy, "mid": mid, "liquidation": sell}

    # --- account -------------------------------------------------------------
    def equity_now(self) -> float:
        return self.cash + sum(p.mark * MULTIPLIER * p.units for p in self.positions.values())

    def _commission(self, structure: Structure, units: int) -> float:
        return self.risk.commission_per_contract * structure.contracts_per_unit * units

    def _reject(self, why: str) -> None:
        self.rejections[why] = self.rejections.get(why, 0) + 1

    def _pdt_ok(self, pos: Position) -> bool:
        if not self.risk.pdt or pos.entry_time.date() != self.index[self.i].date():
            return True
        if self.equity_now() >= self.risk.pdt_equity_threshold:
            return True
        today = self.index[self.i].date()
        window_start = np.busday_offset(today, -4, roll="backward").astype(object)
        while self._day_trades and self._day_trades[0] < window_start:
            self._day_trades.popleft()
        return len(self._day_trades) < self.risk.pdt_max_day_trades

    def _close(self, pos: Position, net_value: float, reason: str, commission: bool = True) -> None:
        fee = self._commission(pos.structure, pos.units) if commission else 0.0
        self.cash += net_value * MULTIPLIER * pos.units - fee
        entry_fee = self._commission(pos.structure, pos.units)
        pnl = pos.pnl_at(net_value) - fee - entry_fee
        if pos.entry_time.date() == self.index[self.i].date() and reason != "expiry":
            self._day_trades.append(self.index[self.i].date())
        self.trades.append(Trade(
            pos.id, self.strategy.name, pos.structure.symbol, pos.structure.name, pos.tag, pos.units,
            pos.entry_time, self._time(self.i, "close"), pos.entry_net, net_value, pos.max_loss, pnl, reason,
        ))
        del self.positions[pos.id]

    def try_close(self, pos: Position, reason: str) -> bool:
        if pos.id not in self.positions:
            return False
        if not self._pdt_ok(pos):
            self.pdt_blocks += 1
            return False
        self._close(pos, self.net_prices(pos.structure, self.i, "close")["liquidation"], reason)
        return True

    # --- the loop --------------------------------------------------------------
    def _fill_pending(self) -> None:
        orders, self.pending = self.pending, []
        if self.trade_start is not None and self.index[self.i] < self.trade_start:
            return
        if self.i < self.strategy.warmup_bars:
            return
        today = self.index[self.i].date()
        for o in orders:
            s = o.structure
            if s.expiry < today:
                self._reject("expired_before_fill"); continue
            if year_fraction(self._time(self.i, "open"), s.expiry) <= 0:
                self._reject("expired_before_fill"); continue
            equity = self.equity_now()
            if equity < self._day_start_equity * (1 - self.risk.daily_loss_limit):
                self._reject("daily_loss_limit"); continue
            if len(self.positions) >= self.risk.max_open_positions:
                self._reject("max_open_positions"); continue
            entry = self.net_prices(s, self.i, "open")["entry"]
            loss_per_unit = s.max_loss(entry) * MULTIPLIER + 2 * self._commission(s, 1)
            if loss_per_unit <= 0:
                self._reject("nonpositive_max_loss"); continue  # arbitrage-looking quote; skip
            budget = equity * self.risk.risk_per_trade
            if self.risk.max_risk_dollars is not None:
                budget = min(budget, self.risk.max_risk_dollars)
            units = min(int(budget // loss_per_unit), self.risk.max_units, o.max_units or 10**9)
            if units < 1:
                self._reject("too_risky_for_account"); continue
            open_risk = sum(p.max_loss for p in self.positions.values())
            if open_risk + units * loss_per_unit > equity * self.risk.max_total_risk:
                self._reject("max_total_risk"); continue
            cost = entry * MULTIPLIER * units + self._commission(s, units)
            if entry > 0 and cost > self.cash:
                self._reject("insufficient_cash"); continue
            self.cash -= cost
            pos = Position(self._next_id, s, units, self._time(self.i, "open"), entry,
                           units * loss_per_unit, o.exits, o.tag)
            pos.mark = self.net_prices(s, self.i, "open")["mid"]
            self.positions[pos.id] = pos
            self._next_id += 1

    def _check_exits(self) -> None:
        today = self.index[self.i].date()
        now = self._time(self.i, "close")
        for pos in list(self.positions.values()):
            px = self.net_prices(pos.structure, self.i, "close")
            pos.mark = px["mid"]
            pos.bars_held += 1
            if pos.structure.expiry <= today and self.last_of_day[self.i]:
                continue  # settles below
            ex, liq = pos.exits, px["liquidation"]
            gain = (liq - pos.entry_net) / pos.premium if pos.premium > 1e-9 else 0.0
            reason = None
            if ex.take_profit is not None and gain >= ex.take_profit:
                reason = "take_profit"
            elif ex.stop_loss is not None and gain <= -ex.stop_loss:
                reason = "stop_loss"
            elif ex.max_hold_bars is not None and pos.bars_held >= ex.max_hold_bars:
                reason = "max_hold"
            elif ex.exit_dte is not None and int(np.busday_count(today, pos.structure.expiry)) <= ex.exit_dte:
                reason = "dte_exit"
            elif (ex.minutes_before_close is not None and pos.structure.expiry == today
                  and (now.normalize() + pd.Timedelta(hours=16) - now).total_seconds() / 60 <= ex.minutes_before_close):
                reason = "time_exit"
            if reason:
                self.try_close(pos, reason)

    def _settle_expiries(self) -> None:
        if not self.last_of_day[self.i]:
            return
        today = self.index[self.i].date()
        for pos in list(self.positions.values()):
            if pos.structure.expiry <= today:
                spot = self._spot[pos.structure.symbol][1][self.i]
                self._close(pos, pos.structure.payoff(spot), "expiry", commission=False)

    def run(self) -> "BacktestResult":
        self.strategy.on_start(self.ctx)
        for i in range(len(self.index)):
            self.i = i
            if self.first_of_day[i]:
                self._day_start_equity = self.equity_now()
            self._fill_pending()
            self._check_exits()
            self._settle_expiries()
            if i >= self.strategy.warmup_bars:
                self.strategy.on_bar(self.ctx)
            self.equity_curve.append(self.equity_now())
        return BacktestResult(
            strategy=self.strategy.name,
            equity=pd.Series(self.equity_curve, index=self.index, dtype=float),
            trades=self.trades,
            open_positions=list(self.positions.values()),
            rejections=dict(self.rejections),
            pdt_blocks=self.pdt_blocks,
            risk=self.risk,
        )


@dataclass
class BacktestResult:
    strategy: str
    equity: pd.Series
    trades: List[Trade]
    open_positions: List[Position]
    rejections: Dict[str, int]
    pdt_blocks: int
    risk: RiskConfig

    def trades_frame(self) -> pd.DataFrame:
        return pd.DataFrame([t.__dict__ for t in self.trades])
