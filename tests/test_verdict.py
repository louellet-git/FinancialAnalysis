from dataclasses import replace

import numpy as np
import pytest

from financial_analysis.driver_dcf import DriverDCF, SegmentSpec
from financial_analysis.verdict import (
    annual_return,
    chance_of_loss,
    chance_price_too_high,
    price_implied_mean,
    total_return,
    verdict_band,
)
from tests.test_driver_dcf import _standard_model

SEED = 7
N = 2_000


def _bundle() -> dict:
    return {
        "model": _standard_model(),
        "segments": [
            SegmentSpec("software", base_revenue=30e9, growth_driver="g_software"),
            SegmentSpec("infra", base_revenue=20e9, growth_driver="g_infra"),
        ],
        "net_debt": 50e9,
        "shares": 1e9,
    }


def test_chance_price_too_high_counts_paths_below_price():
    assert chance_price_too_high(np.array([1.0, 2.0, 3.0, 4.0]), 2.5) == 0.5


@pytest.mark.parametrize("p, expected", [
    (0.20, "Strong opportunity"),
    (0.25, "Opportunity"),
    (0.40, "No edge"),
    (0.60, "No edge"),
    (0.61, "Overpriced"),
    (0.75, "Overpriced"),
    (0.76, "Strongly overpriced"),
])
def test_verdict_band_cutoffs(p, expected):
    assert verdict_band(p) == expected


def test_full_convergence_returns_the_whole_gap():
    assert total_return(150.0, 100.0, k=1.0, years=1) == pytest.approx(0.5)


def test_no_convergence_returns_only_dividends():
    assert total_return(150.0, 100.0, k=0.0, years=2, dividend_yield=0.03) == pytest.approx(1.03 ** 2 - 1)


def test_half_closure_halves_the_log_gap():
    # price 100, value 400: half the log gap closed -> price 200
    assert total_return(400.0, 100.0, k=0.5, years=1) == pytest.approx(1.0)


def test_annual_return_annualizes_total_return():
    # after 2 years a quarter of the log gap remains: 400 * (100/400) ** 0.25 = 282.84
    assert annual_return(400.0, 100.0, k=0.5, years=2) == pytest.approx(2.828427 ** 0.5 - 1, rel=1e-5)


def test_non_positive_value_is_floored_not_nan():
    assert total_return(-5.0, 100.0, k=1.0, years=1) == pytest.approx(0.01 / 100.0 - 1)


def test_chance_of_loss_counts_dividends():
    # 50 -> 0.5 x 1.2 loses; 90 -> 0.9 x 1.2 gains; 150 gains
    assert chance_of_loss(np.array([50.0, 90.0, 150.0]), 100.0, k=1.0, years=1,
                          dividend_yield=0.2) == pytest.approx(1 / 3)


def test_price_implied_mean_recovers_known_driver_mean():
    bundle = _bundle()
    target = 0.22
    model = bundle["model"]
    shifted = replace(model, drivers=[
        replace(d, mean=target) if d.name == "core_margin" else d for d in model.drivers
    ])
    price = DriverDCF(shifted, bundle["segments"], net_debt=50e9, shares=1e9).run(N, SEED).per_share_p50
    assert price_implied_mean(bundle, "core_margin", price, n_simulations=N, seed=SEED) == pytest.approx(target, abs=1e-4)


def test_price_implied_mean_is_none_when_unreachable():
    assert price_implied_mean(_bundle(), "core_margin", 1e6, n_simulations=N, seed=SEED) is None


def test_price_implied_mean_rejects_deterministic_driver():
    with pytest.raises(ValueError, match="deterministic"):
        price_implied_mean(_bundle(), "beta", 100.0)
