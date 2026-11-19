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
