import json
from unittest.mock import patch
import pytest

from financial_analysis.xbrl_api import fetch_company_facts, extract_annual_series, extract_quarterly_series

SAMPLE_FACTS = {
    "entityName": "Palo Alto Networks Inc",
    "facts": {
        "us-gaap": {
            "Revenues": {
                "label": "Revenues",
                "units": {
                    "USD": [
                        {"end": "2023-07-31", "val": 6893000000, "form": "10-K", "filed": "2023-09-11"},
                        {"end": "2024-07-31", "val": 8030000000, "form": "10-K", "filed": "2024-09-12"},
                        # Duplicate end date — older filing, should be dropped
                        {"end": "2024-07-31", "val": 8030000000, "form": "10-K", "filed": "2024-08-01"},
                        {"end": "2024-04-30", "val": 1980000000, "form": "10-Q", "filed": "2024-06-07"},
                        {"end": "2024-01-31", "val": 1980000000, "form": "10-Q", "filed": "2024-03-05"},
                    ]
                }
            }
        }
    }
}


# --- extract_annual_series ---

def test_extract_annual_series_returns_only_10k_entries():
    result = extract_annual_series(SAMPLE_FACTS, "us-gaap/Revenues")
    assert all(e["form"] == "10-K" for e in result)


def test_extract_annual_series_sorted_ascending_by_end_date():
    result = extract_annual_series(SAMPLE_FACTS, "us-gaap/Revenues")
    ends = [e["end"] for e in result]
    assert ends == sorted(ends)


def test_extract_annual_series_deduplicates_by_end_date():
    result = extract_annual_series(SAMPLE_FACTS, "us-gaap/Revenues")
    ends = [e["end"] for e in result]
    assert len(ends) == len(set(ends))


def test_extract_annual_series_keeps_latest_filed_on_duplicate():
    result = extract_annual_series(SAMPLE_FACTS, "us-gaap/Revenues")
    entry_2024 = next(e for e in result if e["end"] == "2024-07-31")
    assert entry_2024["filed"] == "2024-09-12"


def test_extract_annual_series_missing_concept_returns_empty():
    result = extract_annual_series(SAMPLE_FACTS, "us-gaap/NonExistentConcept")
    assert result == []


def test_extract_annual_series_missing_namespace_returns_empty():
    result = extract_annual_series(SAMPLE_FACTS, "nonexistent/Revenues")
    assert result == []


def test_extract_annual_series_malformed_concept_returns_empty():
    result = extract_annual_series(SAMPLE_FACTS, "no-slash-here")
    assert result == []


# --- extract_quarterly_series ---

def test_extract_quarterly_series_returns_only_10q_entries():
    result = extract_quarterly_series(SAMPLE_FACTS, "us-gaap/Revenues")
    assert all(e["form"] == "10-Q" for e in result)


def test_extract_quarterly_series_sorted_ascending_by_end_date():
    result = extract_quarterly_series(SAMPLE_FACTS, "us-gaap/Revenues")
    ends = [e["end"] for e in result]
    assert ends == sorted(ends)


def test_extract_quarterly_series_missing_concept_returns_empty():
    result = extract_quarterly_series(SAMPLE_FACTS, "us-gaap/NonExistent")
    assert result == []


# --- fetch_company_facts ---

def test_fetch_company_facts_calls_correct_url():
    payload = json.dumps(SAMPLE_FACTS).encode()
    with patch("financial_analysis.xbrl_api._http_get", return_value=payload) as mock_get:
        fetch_company_facts("1327567")
        called_url = mock_get.call_args[0][0]
        assert "CIK0001327567" in called_url


def test_fetch_company_facts_returns_parsed_dict():
    payload = json.dumps(SAMPLE_FACTS).encode()
    with patch("financial_analysis.xbrl_api._http_get", return_value=payload):
        result = fetch_company_facts("1327567")
        assert result["entityName"] == "Palo Alto Networks Inc"
        assert "facts" in result


def test_fetch_company_facts_zero_pads_short_cik():
    payload = json.dumps(SAMPLE_FACTS).encode()
    with patch("financial_analysis.xbrl_api._http_get", return_value=payload) as mock_get:
        fetch_company_facts("123")
        called_url = mock_get.call_args[0][0]
        assert "CIK0000000123" in called_url
