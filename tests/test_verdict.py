import numpy as np
import pytest

from financial_analysis.verdict import chance_price_too_high, verdict_band


def test_chance_price_too_high_counts_paths_below_price():
    assert chance_price_too_high(np.array([1.0, 2.0, 3.0, 4.0]), 2.5) == 0.5


def test_path_equal_to_price_is_not_counted_as_too_high():
    assert chance_price_too_high(np.array([2.5, 3.0]), 2.5) == 0.0


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
