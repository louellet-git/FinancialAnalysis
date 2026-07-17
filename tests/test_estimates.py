import json
from datetime import date
from unittest.mock import patch, MagicMock, PropertyMock

import pandas as pd
import pytest

from financial_analysis.estimates import (
    fetch_estimates,
    latest,
    list_snapshots,
    load_snapshot,
    save_snapshot,
)
from financial_analysis.schema import ConsensusEstimates


# ---------------------------------------------------------------------------
# Synthetic yfinance consensus (no network in tests)
# ---------------------------------------------------------------------------

def _revenue_df():
    return pd.DataFrame(
        {
            "numberOfAnalysts": [12.0, 11.0, 18.0, 19.0],
            "avg": [16.4e9, 16.9e9, 66.5e9, 69.1e9],
            "low": [16.1e9, 16.5e9, 65.8e9, 67.2e9],
            "high": [16.8e9, 17.4e9, 67.3e9, 71.0e9],
            "growth": [0.031, float("nan"), 0.042, 0.039],  # NaN → None
        },
        index=["0q", "+1q", "0y", "+1y"],
    )


def _earnings_df():
    return pd.DataFrame(
        {
            "numberOfAnalysts": [13.0, 12.0, 17.0, 18.0],
            "avg": [2.63, 2.75, 10.95, 11.65],
            "low": [2.51, 2.60, 10.70, 11.10],
            "high": [2.75, 2.90, 11.20, 12.30],
        },
        index=["0q", "+1q", "0y", "+1y"],
    )


def _price_targets():
    return {"current": 285.5, "low": 190.0, "high": 340.0, "mean": 271.3, "median": 275.0}


def _mock_ticker(revenue=None, earnings=None, targets=None):
    ticker = MagicMock()
    type(ticker).revenue_estimate = PropertyMock(return_value=revenue)
    type(ticker).earnings_estimate = PropertyMock(return_value=earnings)
    type(ticker).analyst_price_targets = PropertyMock(return_value=targets)
    return ticker


def _full_mock_ticker():
    return _mock_ticker(_revenue_df(), _earnings_df(), _price_targets())


# ---------------------------------------------------------------------------
# Fetching
# ---------------------------------------------------------------------------

def test_fetch_creates_dated_snapshot_file(tmp_path):
    with patch("financial_analysis.estimates.yf") as mock_yf:
        mock_yf.Ticker.return_value = _full_mock_ticker()
        snap = fetch_estimates("TEST", cache_dir=tmp_path)

    assert snap.ticker == "TEST"
    assert snap.snapshot_date == date.today()
    path = tmp_path / "TEST" / "estimates" / f"{date.today().isoformat()}.json"
    assert path.exists()
    json.loads(path.read_text(encoding="utf-8"))  # valid JSON despite NaN in source


def test_fetch_parses_all_three_surfaces(tmp_path):
    with patch("financial_analysis.estimates.yf") as mock_yf:
        mock_yf.Ticker.return_value = _full_mock_ticker()
        snap = fetch_estimates("TEST", cache_dir=tmp_path)

    assert set(snap.revenue_estimate) == {"0q", "+1q", "0y", "+1y"}
    assert snap.revenue_estimate["0y"]["avg"] == 66.5e9
    assert snap.earnings_estimate["+1y"]["avg"] == 11.65
    assert snap.analyst_price_targets["mean"] == 271.3


def test_fetch_cleans_nan_to_none(tmp_path):
    with patch("financial_analysis.estimates.yf") as mock_yf:
        mock_yf.Ticker.return_value = _full_mock_ticker()
        snap = fetch_estimates("TEST", cache_dir=tmp_path)

    assert snap.revenue_estimate["+1q"]["growth"] is None


def test_fetch_absent_coverage_saves_empty_snapshot(tmp_path):
    """Small-caps / delisted: all three surfaces empty or raising — no exception."""
    ticker = MagicMock()
    type(ticker).revenue_estimate = PropertyMock(return_value=pd.DataFrame())
    type(ticker).earnings_estimate = PropertyMock(side_effect=Exception("no data"))
    type(ticker).analyst_price_targets = PropertyMock(return_value={})
    with patch("financial_analysis.estimates.yf") as mock_yf:
        mock_yf.Ticker.return_value = ticker
        snap = fetch_estimates("TINY", cache_dir=tmp_path)

    assert snap.revenue_estimate == {}
    assert snap.earnings_estimate == {}
    assert snap.analyst_price_targets == {}
    path = tmp_path / "TINY" / "estimates" / f"{date.today().isoformat()}.json"
    assert path.exists()


def test_fetch_same_day_uses_cached_snapshot(tmp_path):
    save_snapshot(ConsensusEstimates(
        ticker="TEST", snapshot_date=date.today(),
        analyst_price_targets={"mean": 42.0},
    ), cache_dir=tmp_path)

    with patch("financial_analysis.estimates.yf") as mock_yf:
        snap = fetch_estimates("TEST", cache_dir=tmp_path)

    mock_yf.Ticker.assert_not_called()
    assert snap.analyst_price_targets == {"mean": 42.0}


def test_fetch_refresh_overwrites_same_day_snapshot(tmp_path):
    save_snapshot(ConsensusEstimates(
        ticker="TEST", snapshot_date=date.today(),
        analyst_price_targets={"mean": 42.0},
    ), cache_dir=tmp_path)

    with patch("financial_analysis.estimates.yf") as mock_yf:
        mock_yf.Ticker.return_value = _full_mock_ticker()
        snap = fetch_estimates("TEST", cache_dir=tmp_path, refresh=True)

    assert snap.analyst_price_targets["mean"] == 271.3
    reloaded = load_snapshot("TEST", cache_dir=tmp_path)
    assert reloaded.analyst_price_targets["mean"] == 271.3


# ---------------------------------------------------------------------------
# Persistence / loading
# ---------------------------------------------------------------------------

def _saved(tmp_path, day, mean=100.0):
    save_snapshot(ConsensusEstimates(
        ticker="TEST", snapshot_date=day, analyst_price_targets={"mean": mean},
    ), cache_dir=tmp_path)


def test_round_trip_preserves_structure(tmp_path):
    snap = ConsensusEstimates(
        ticker="TEST",
        snapshot_date=date(2026, 7, 17),
        revenue_estimate={"0y": {"avg": 66.5e9, "growth": None}},
        earnings_estimate={"0y": {"avg": 10.95}},
        analyst_price_targets={"mean": 271.3},
    )
    save_snapshot(snap, cache_dir=tmp_path)
    loaded = load_snapshot("TEST", cache_dir=tmp_path, snapshot_date=date(2026, 7, 17))
    assert loaded == snap


def test_list_snapshots_sorted_and_ignores_junk(tmp_path):
    _saved(tmp_path, date(2026, 7, 17))
    _saved(tmp_path, date(2026, 7, 15))
    (tmp_path / "TEST" / "estimates" / "notes.json").write_text("{}", encoding="utf-8")
    assert list_snapshots("TEST", cache_dir=tmp_path) == [date(2026, 7, 15), date(2026, 7, 17)]


def test_load_snapshot_defaults_to_most_recent(tmp_path):
    _saved(tmp_path, date(2026, 7, 15), mean=100.0)
    _saved(tmp_path, date(2026, 7, 17), mean=110.0)
    snap = load_snapshot("TEST", cache_dir=tmp_path)
    assert snap.snapshot_date == date(2026, 7, 17)
    assert snap.analyst_price_targets["mean"] == 110.0


def test_load_snapshot_missing_raises(tmp_path):
    with pytest.raises(FileNotFoundError):
        load_snapshot("TEST", cache_dir=tmp_path)
    with pytest.raises(FileNotFoundError):
        load_snapshot("TEST", cache_dir=tmp_path, snapshot_date=date(2026, 1, 1))


def test_latest_returns_none_when_never_pulled(tmp_path):
    assert latest("TEST", cache_dir=tmp_path) is None


def test_latest_returns_most_recent(tmp_path):
    _saved(tmp_path, date(2026, 7, 15), mean=100.0)
    _saved(tmp_path, date(2026, 7, 17), mean=110.0)
    snap = latest("TEST", cache_dir=tmp_path)
    assert snap is not None
    assert snap.snapshot_date == date(2026, 7, 17)
