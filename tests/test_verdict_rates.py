"""scripts/verdict.py --rf-date path, offline: the rates functions are patched."""
from datetime import date
from unittest.mock import patch

import pytest

from financial_analysis import rates
from financial_analysis.driver_dcf import Driver, DriverModel
from scripts.verdict import rates_context


def _obs(series, d, v, note=""):
    return rates.RateObservation(series, d, v, "test", note)


def _infl(d, real, be):
    return {"real_10y": _obs("real", d, real), "breakeven_10y": _obs("be", d, be),
            "forward_5y5y": _obs("fwd", d, 0.0215), "cpi_12m": _obs("cpi", date(2026, 9, 1), 0.0374),
            "core_cpi_12m": _obs("core", date(2026, 9, 1), 0.0256)}


def test_rates_context_sets_rf_and_splits_the_change():
    model = DriverModel(drivers=[
        Driver("rf", stochastic=True, mean=0.046, std=0.005, dist="truncnorm", lo=0.01),
        Driver("terminal_growth", stochastic=True, mean=0.0225, std=0.004),
    ])
    now, base = date(2026, 11, 18), date(2026, 7, 16)
    yields = {now: _obs("UST 10Y", now, 0.0507), base: _obs("UST 10Y", base, 0.0457)}
    with patch.object(rates, "treasury_yield", side_effect=lambda t, on: yields[on or now]), \
         patch.object(rates, "inflation_snapshot",
                      side_effect=lambda d: _infl(d, 0.0288, 0.0219) if d == now else _infl(d, 0.0235, 0.0222)):
        new, ctx = rates_context(model, "2026-11-18", "2026-07-16")
    assert new.driver("rf").mean == 0.0507
    assert ctx["rf_change_vs_model"] == pytest.approx(0.0047)
    assert ctx["terminal_growth_real_vs_5y5y"] == pytest.approx(0.0010)
    b = ctx["base"]
    assert b["nominal_change"] == pytest.approx(0.0050)
    assert b["real_change"] == pytest.approx(0.0053)
    assert b["breakeven_change"] == pytest.approx(-0.0003)


def test_credit_context_sets_kd_spread_from_rating():
    from financial_analysis import bonds
    from scripts.verdict import credit_context
    model = DriverModel(drivers=[
        Driver("kd_spread", stochastic=False, mean=-0.001),
        Driver("tax_rate", stochastic=False, mean=0.21),
    ])
    d = date(2026, 11, 17)
    with patch.object(bonds, "spread_over_10y", return_value=_obs("A - 10Y", d, 0.0067, "A- uses the A index")), \
         patch.object(bonds, "corporate_yield", return_value=_obs("A yield", d, 0.0577)), \
         patch.object(bonds, "credit_spread", return_value=_obs("A OAS", d, 0.0076)), \
         patch.object(bonds, "moodys_yield", side_effect=lambda g, on: _obs(g, d, 0.0618 if g == "Aaa" else 0.0663)):
        new, ctx = credit_context(model, "A-", "today", rf=0.0507)
    assert new.driver("kd_spread").mean == 0.0067
    assert ctx["index"] == "A"
    assert ctx["kd_spread_change_vs_model"] == pytest.approx(0.0077)
    assert ctx["kd_pretax"] == pytest.approx(0.0574)
    assert ctx["kd_after_tax"] == pytest.approx(0.0574 * 0.79)
