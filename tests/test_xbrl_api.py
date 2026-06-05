import json
from unittest.mock import patch
import pytest

from financial_analysis.xbrl_api import fetch_company_facts, extract_annual_series, extract_quarterly_series, extract_series

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


# --- extract_series (generic extractor) ---

def _make_facts(concept_name: str, unit: str, entries: list) -> dict:
    return {"facts": {"us-gaap": {concept_name: {"units": {unit: entries}}}}}


def test_extract_series_annual_usd():
    facts = _make_facts("Revenues", "USD", [
        {"end": "2024-01-28", "val": 60920000000, "form": "10-K", "filed": "2024-02-21"},
        {"end": "2023-01-29", "val": 26970000000, "form": "10-K", "filed": "2023-02-24"},
        {"end": "2024-04-28", "val": 22100000000, "form": "10-Q", "filed": "2024-05-22"},
    ])
    result = extract_series(facts, "us-gaap/Revenues", form_types=["10-K"], unit="USD", n=10)
    assert len(result) == 2
    assert all(e["form"] == "10-K" for e in result)
    assert result[0]["end"] == "2023-01-29"  # sorted ascending


def test_extract_series_quarterly():
    facts = _make_facts("Revenues", "USD", [
        {"end": "2024-04-28", "val": 22100000000, "form": "10-Q", "filed": "2024-05-22"},
        {"end": "2024-01-28", "val": 60920000000, "form": "10-K", "filed": "2024-02-21"},
    ])
    result = extract_series(facts, "us-gaap/Revenues", form_types=["10-Q"], unit="USD", n=10)
    assert len(result) == 1
    assert result[0]["form"] == "10-Q"


def test_extract_series_non_usd_unit():
    facts = _make_facts("EarningsPerShareDiluted", "USD/shares", [
        {"end": "2024-01-28", "val": 1.19, "form": "10-K", "filed": "2024-02-21"},
    ])
    result = extract_series(facts, "us-gaap/EarningsPerShareDiluted", form_types=["10-K"], unit="USD/shares", n=10)
    assert len(result) == 1
    assert result[0]["val"] == 1.19


def test_extract_series_missing_concept_returns_empty():
    facts = {"facts": {"us-gaap": {}}}
    result = extract_series(facts, "us-gaap/NonExistent", form_types=["10-K"], unit="USD", n=10)
    assert result == []


def test_extract_series_respects_n_limit():
    entries = [
        {"end": f"202{i}-01-28", "val": float(i), "form": "10-K", "filed": f"202{i}-02-01"}
        for i in range(8)
    ]
    facts = _make_facts("Revenues", "USD", entries)
    result = extract_series(facts, "us-gaap/Revenues", form_types=["10-K"], unit="USD", n=3)
    assert len(result) == 3
    assert result[-1]["end"] == "2027-01-28"  # most recent 3


def test_extract_series_deduplicates_by_latest_filed():
    facts = _make_facts("Revenues", "USD", [
        {"end": "2024-01-28", "val": 60920000000, "form": "10-K", "filed": "2024-02-21"},
        {"end": "2024-01-28", "val": 61000000000, "form": "10-K", "filed": "2024-03-01"},
    ])
    result = extract_series(facts, "us-gaap/Revenues", form_types=["10-K"], unit="USD", n=10)
    assert len(result) == 1
    assert result[0]["val"] == 61000000000
