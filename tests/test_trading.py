"""Trading harness: pricing, structures, engine risk rules, no-lookahead. No network."""
from datetime import date

import numpy as np
import pandas as pd
import pytest

from trading.data import NY, MarketData
from trading.engine import Backtester, RiskConfig
from trading.metrics import summarize
from trading.pricing import SyntheticPricer, VolSnapshot, bs_price, year_fraction
from trading.registry import discover
from trading.strategy import ExitRules, Strategy
from trading.structures import butterfly, iron_condor, long_put, vertical, Leg, Structure


def _md(closes, interval="1d", start="2024-01-02", vol=0.15):
    if interval == "1d":
        idx = pd.bdate_range(start, periods=len(closes), tz=NY)
    else:
        idx = pd.date_range(f"{start} 09:30", periods=len(closes), freq="5min", tz=NY)
    c = np.asarray(closes, float)
    bars = pd.DataFrame({"open": c, "high": c, "low": c, "close": c, "volume": 1}, index=idx)
    v = pd.DataFrame({k: vol for k in ("atm_open", "atm_close", "short_open", "short_close")}, index=idx)
    return MarketData(interval, {"SPY": bars}, {"SPY": v}, pd.Series(0.04, index=idx), {"SPY": bars})


def test_put_call_parity():
    S, K, t, r, v = 500, 505, 0.1, 0.04, 0.2
    c, p = bs_price("C", S, K, t, r, v), bs_price("P", S, K, t, r, v)
    assert c - p == pytest.approx(S - K * np.exp(-r * t), abs=1e-9)


def test_year_fraction_0dte_and_weekend():
    fri_noon = pd.Timestamp("2026-10-09 12:00", tz=NY)
    assert year_fraction(fri_noon, date(2026, 10, 9)) == pytest.approx(240 / (252 * 390))
    # Friday noon -> Monday close: 240 min today + 1 full session (weekend skipped)
    assert year_fraction(fri_noon, date(2026, 10, 12)) == pytest.approx((240 + 390) / (252 * 390))
    assert year_fraction(fri_noon, date(2026, 10, 8)) == 0.0


def test_skew_puts_richer_and_spread_bounds():
    p = SyntheticPricer()
    vs = VolSnapshot(0.15, 0.12, 0.04)
    now = pd.Timestamp("2026-10-09 10:00", tz=NY)
    put = p.quote("SPY", "P", 480, date(2026, 11, 20), now, 500, vs)
    call = p.quote("SPY", "C", 520, date(2026, 11, 20), now, 500, vs)
    assert put.iv > call.iv
    for q in (put, call):
        assert 0.02 - 1e-12 <= q.ask - q.bid <= 0.10 + 1e-12


def test_structures_max_loss_and_defined_risk():
    exp = date(2026, 10, 16)
    cs = vertical("SPY", exp, "C", 500, 505)
    assert cs.name == "call_debit_spread" and cs.max_loss(2.0) == pytest.approx(2.0)
    pcs = vertical("SPY", exp, "P", 490, 495)
    assert pcs.name == "put_credit_spread" and pcs.max_loss(-1.5) == pytest.approx(3.5)
    ic = iron_condor("SPY", exp, 480, 485, 515, 520)
    assert ic.max_loss(-1.0) == pytest.approx(4.0)
    assert butterfly("SPY", exp, "C", 495, 500, 505).max_loss(1.2) == pytest.approx(1.2)
    assert long_put("SPY", exp, 500).max_loss(3.0) == pytest.approx(3.0)
    with pytest.raises(ValueError):
        Structure("naked_call", "SPY", exp, (Leg("C", 500, -1),))
    with pytest.raises(ValueError):
        Structure("naked_put", "SPY", exp, (Leg("P", 500, -1),))


class _BuyOnce(Strategy):
    name = "t_buy_once"

    def __init__(self, structure_fn, exits=ExitRules()):
        self.fn, self.exits, self.done = structure_fn, exits, False

    def on_bar(self, ctx):
        if not self.done:
            ctx.open(self.fn(ctx), self.exits)
            self.done = True


def test_engine_sizes_by_risk_and_settles_at_expiry():
    md = _md([500.0] * 3 + [520.0] * 7)
    strat = _BuyOnce(lambda ctx: vertical("SPY", ctx.expiry(4), "C", 503, 504))
    res = Backtester(strat, md, RiskConfig(starting_equity=2000, risk_per_trade=0.03)).run()
    assert len(res.trades) == 1
    t = res.trades[0]
    assert t.exit_reason == "expiry" and t.exit_net == pytest.approx(1.0)
    # sized so worst case (debit + both commissions) <= 3% of $2,000
    assert t.max_loss <= 60 + 1e-9 and t.units >= 1
    assert res.equity.iloc[-1] == pytest.approx(2000 + t.pnl)


def test_entry_fills_next_bar_open_not_signal_bar():
    md = _md([500.0, 510.0, 510.0, 510.0])
    strat = _BuyOnce(lambda ctx: vertical("SPY", ctx.expiry(3), "C", 515, 516))
    res = Backtester(strat, md).run()
    assert res.trades[0].entry_time.date() == md.index[1].date()


def test_too_risky_trade_is_rejected():
    md = _md([500.0] * 5)
    strat = _BuyOnce(lambda ctx: vertical("SPY", ctx.expiry(3), "C", 480, 500))  # ~$20 wide
    res = Backtester(strat, md).run()
    assert not res.trades and res.rejections == {"too_risky_for_account": 1}


class _DayTrader(Strategy):
    name = "t_day_trader"
    interval = "5m"

    def on_bar(self, ctx):
        if not ctx.positions and not ctx.pending_entries:
            ctx.open(vertical("SPY", ctx.expiry(0), "C", 500, 501), ExitRules(max_hold_bars=1))


def test_pdt_limits_same_day_round_trips():
    md = _md([500.0] * 40, interval="5m")
    res = Backtester(_DayTrader(), md).run()
    day_trades = [t for t in res.trades if t.exit_reason != "expiry"]
    assert len(day_trades) == 3 and res.pdt_blocks > 0
    assert res.trades[-1].exit_reason == "expiry"


class _Peeker(Strategy):
    name = "t_peeker"

    def on_bar(self, ctx):
        assert len(ctx.bars()) == ctx.i + 1
        assert ctx.bars().index[-1] == ctx.ts


def test_context_has_no_lookahead():
    Backtester(_Peeker(), _md(list(range(400, 420)))).run()


def test_metrics_split_and_registry():
    md = _md([500.0] * 20)
    res = Backtester(_BuyOnce(lambda ctx: vertical("SPY", ctx.expiry(3), "C", 500, 501)), md).run()
    row = summarize(res)
    assert row["all_trades"] == 1 and row["oos_start"]
    assert "baseline_weekly_call_spread" in discover(include_private=True)
