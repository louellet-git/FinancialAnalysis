import json
from pathlib import Path
from unittest.mock import patch

import pytest

from financial_analysis.cache import load_company_facts


SAMPLE_FACTS = {"entityName": "NVIDIA CORP", "cik": 1045810, "facts": {"us-gaap": {}}}


def test_cache_miss_fetches_and_writes(tmp_path):
    with patch("financial_analysis.cache.get_cik_for_ticker", return_value="1045810"), \
         patch("financial_analysis.cache.fetch_company_facts", return_value=SAMPLE_FACTS) as mock_fetch:
        result = load_company_facts("NVDA", cache_dir=tmp_path, refresh=False)
    assert result == SAMPLE_FACTS
    mock_fetch.assert_called_once_with("1045810")
    cache_file = tmp_path / "NVDA" / "company_facts.json"
    assert cache_file.exists()
    assert json.loads(cache_file.read_text(encoding="utf-8")) == SAMPLE_FACTS


def test_cache_hit_returns_disk_without_fetch(tmp_path):
    cache_file = tmp_path / "NVDA" / "company_facts.json"
    cache_file.parent.mkdir(parents=True)
    cache_file.write_text(json.dumps(SAMPLE_FACTS), encoding="utf-8")
    with patch("financial_analysis.cache.fetch_company_facts") as mock_fetch:
        result = load_company_facts("NVDA", cache_dir=tmp_path, refresh=False)
    assert result == SAMPLE_FACTS
    mock_fetch.assert_not_called()


def test_refresh_flag_bypasses_cache(tmp_path):
    cache_file = tmp_path / "NVDA" / "company_facts.json"
    cache_file.parent.mkdir(parents=True)
    stale = {"entityName": "STALE"}
    cache_file.write_text(json.dumps(stale), encoding="utf-8")
    fresh = {"entityName": "FRESH", "facts": {"us-gaap": {}}}
    with patch("financial_analysis.cache.get_cik_for_ticker", return_value="1045810"), \
         patch("financial_analysis.cache.fetch_company_facts", return_value=fresh):
        result = load_company_facts("NVDA", cache_dir=tmp_path, refresh=True)
    assert result["entityName"] == "FRESH"


def test_corrupt_cache_falls_back_to_fetch(tmp_path):
    cache_file = tmp_path / "NVDA" / "company_facts.json"
    cache_file.parent.mkdir(parents=True)
    cache_file.write_text("NOT VALID JSON", encoding="utf-8")
    with patch("financial_analysis.cache.get_cik_for_ticker", return_value="1045810"), \
         patch("financial_analysis.cache.fetch_company_facts", return_value=SAMPLE_FACTS):
        result = load_company_facts("NVDA", cache_dir=tmp_path, refresh=False)
    assert result == SAMPLE_FACTS
    assert json.loads(cache_file.read_text(encoding="utf-8")) == SAMPLE_FACTS


def test_ticker_uppercased_in_path(tmp_path):
    with patch("financial_analysis.cache.get_cik_for_ticker", return_value="1045810"), \
         patch("financial_analysis.cache.fetch_company_facts", return_value=SAMPLE_FACTS):
        load_company_facts("nvda", cache_dir=tmp_path, refresh=False)
    assert (tmp_path / "NVDA" / "company_facts.json").exists()
