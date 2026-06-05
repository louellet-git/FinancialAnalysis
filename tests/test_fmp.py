import json
import os
from datetime import date
from pathlib import Path
from unittest.mock import patch, MagicMock

import pytest

from financial_analysis.fmp import (
    FMPClientError,
    load_estimates,
    load_surprises,
    load_fmp_statements,
)
from financial_analysis.schema import EarningsEstimate, EarningsSurprise


_ESTIMATES_FIXTURE = [
    {
        "symbol": "NVDA",
        "date": "2024-01-28",
        "period": "quarter",
        "estimatedEpsAvg": 4.59,
        "estimatedRevenueAvg": 20148000000,
        "numberAnalystEstimatedEps": 30,
    }
]

_SURPRISES_FIXTURE = [
    {
        "symbol": "NVDA",
        "date": "2024-02-21",
        "actualEarningResult": 4.93,
        "estimatedEarning": 4.59,
    }
]


def _mock_response(data):
    m = MagicMock()
    m.status_code = 200
    m.json.return_value = data
    m.raise_for_status.return_value = None
    return m


def test_load_estimates_fetches_and_caches(tmp_path, monkeypatch):
    monkeypatch.setenv("FMP_API_KEY", "test_key")
    with patch("financial_analysis.fmp.requests.get", return_value=_mock_response(_ESTIMATES_FIXTURE)):
        result = load_estimates("NVDA", cache_dir=tmp_path)

    assert len(result) == 1
    assert isinstance(result[0], EarningsEstimate)
    assert result[0].estimated_eps == 4.59
    assert result[0].period_type == "quarterly"
    assert result[0].number_of_analysts == 30

    cache_file = tmp_path / "NVDA" / "estimates.json"
    assert cache_file.exists()


def test_load_estimates_uses_cache(tmp_path, monkeypatch):
    monkeypatch.setenv("FMP_API_KEY", "test_key")
    cache_file = tmp_path / "NVDA" / "estimates.json"
    cache_file.parent.mkdir(parents=True)
    cache_file.write_text(json.dumps(_ESTIMATES_FIXTURE), encoding="utf-8")

    with patch("financial_analysis.fmp.requests.get") as mock_get:
        result = load_estimates("NVDA", cache_dir=tmp_path)
        mock_get.assert_not_called()

    assert len(result) == 1


def test_load_surprises_fetches_and_caches(tmp_path, monkeypatch):
    monkeypatch.setenv("FMP_API_KEY", "test_key")
    with patch("financial_analysis.fmp.requests.get", return_value=_mock_response(_SURPRISES_FIXTURE)):
        result = load_surprises("NVDA", cache_dir=tmp_path)

    assert len(result) == 1
    assert isinstance(result[0], EarningsSurprise)
    assert result[0].actual_eps == 4.93
    assert result[0].surprise == pytest.approx(0.34, abs=0.01)


def test_load_estimates_missing_key_raises(tmp_path, monkeypatch):
    monkeypatch.delenv("FMP_API_KEY", raising=False)
    with pytest.raises(FMPClientError, match="FMP_API_KEY"):
        load_estimates("NVDA", cache_dir=tmp_path)


def test_load_fmp_statements_raises_not_implemented():
    with pytest.raises(NotImplementedError):
        load_fmp_statements("SPOT")
