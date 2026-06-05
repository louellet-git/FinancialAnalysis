import json
from datetime import date
from pathlib import Path
from unittest.mock import patch, MagicMock

import pandas as pd
import pytest

from financial_analysis.market_data import load_prices
from financial_analysis.schema import PricePoint


def _make_df():
    idx = pd.to_datetime(["2024-01-02", "2024-01-03"])
    return pd.DataFrame(
        {"Open": [100.0, 101.0], "High": [105.0, 106.0], "Low": [99.0, 100.0],
         "Close": [103.0, 104.0], "Volume": [1_000_000, 1_100_000]},
        index=idx,
    )


def test_load_prices_fetches_and_caches(tmp_path):
    df = _make_df()
    with patch("financial_analysis.market_data.yf") as mock_yf:
        mock_ticker = MagicMock()
        mock_ticker.history.return_value = df
        mock_yf.Ticker.return_value = mock_ticker

        result = load_prices("NVDA", cache_dir=tmp_path)

    assert len(result) == 2
    assert isinstance(result[0], PricePoint)
    assert result[0].close == 103.0
    assert result[0].volume == 1_000_000
    cache_file = tmp_path / "NVDA" / "prices.json"
    assert cache_file.exists()


def test_load_prices_uses_cache(tmp_path):
    cache_file = tmp_path / "NVDA" / "prices.json"
    cache_file.parent.mkdir(parents=True)
    cache_file.write_text(json.dumps([
        {"date": "2024-01-02", "open": 100.0, "high": 105.0, "low": 99.0, "close": 103.0, "volume": 1_000_000}
    ]), encoding="utf-8")

    with patch("financial_analysis.market_data.yf") as mock_yf:
        result = load_prices("NVDA", cache_dir=tmp_path)
        mock_yf.Ticker.assert_not_called()

    assert len(result) == 1
    assert result[0].date == date(2024, 1, 2)


def test_load_prices_empty_df_returns_empty(tmp_path):
    with patch("financial_analysis.market_data.yf") as mock_yf:
        mock_ticker = MagicMock()
        mock_ticker.history.return_value = pd.DataFrame()
        mock_yf.Ticker.return_value = mock_ticker

        result = load_prices("FAKE", cache_dir=tmp_path)

    assert result == []


def test_load_prices_refresh_ignores_cache(tmp_path):
    cache_file = tmp_path / "NVDA" / "prices.json"
    cache_file.parent.mkdir(parents=True)
    cache_file.write_text(json.dumps([
        {"date": "2024-01-02", "open": 100.0, "high": 105.0, "low": 99.0, "close": 103.0, "volume": 1_000_000}
    ]), encoding="utf-8")

    df = _make_df()
    with patch("financial_analysis.market_data.yf") as mock_yf:
        mock_ticker = MagicMock()
        mock_ticker.history.return_value = df
        mock_yf.Ticker.return_value = mock_ticker

        result = load_prices("NVDA", cache_dir=tmp_path, refresh=True)
        mock_yf.Ticker.assert_called_once_with("NVDA")

    assert len(result) == 2
