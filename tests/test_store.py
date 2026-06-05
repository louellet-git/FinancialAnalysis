import json
from datetime import date
from pathlib import Path
import pytest

from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.store import save, load, get_series, compare, clear_cache


def _make_company() -> CompanyFinancials:
    return CompanyFinancials(
        ticker="TEST",
        cik="0000000001",
        company_name="Test Corp",
        periods=[
            PeriodFinancials(
                ticker="TEST", period_end=date(2022, 12, 31),
                form="10-K", filed_date=date(2023, 2, 15),
                revenue=1_000_000_000.0,
                operating_cash_flow=200_000_000.0,
                capex=50_000_000.0,
            ),
            PeriodFinancials(
                ticker="TEST", period_end=date(2023, 12, 31),
                form="10-K", filed_date=date(2024, 2, 14),
                revenue=1_200_000_000.0,
                operating_cash_flow=250_000_000.0,
                capex=60_000_000.0,
            ),
        ],
    )


# --- save ---

def test_save_creates_json_file(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    assert (tmp_path / "TEST.json").exists()


def test_save_creates_data_dir_if_missing(tmp_path):
    nested = tmp_path / "sub" / "dir"
    save(_make_company(), data_dir=nested)
    assert (nested / "TEST.json").exists()


def test_save_json_is_pretty_printed(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    raw = (tmp_path / "TEST.json").read_text()
    assert "\n" in raw  # pretty-printed, not one line


def test_save_json_uses_iso_dates(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    parsed = json.loads((tmp_path / "TEST.json").read_text())
    assert parsed["periods"][0]["period_end"] == "2022-12-31"
    assert parsed["periods"][0]["filed_date"] == "2023-02-15"


def test_save_json_contains_ticker_and_name(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    parsed = json.loads((tmp_path / "TEST.json").read_text())
    assert parsed["ticker"] == "TEST"
    assert parsed["company_name"] == "Test Corp"
    assert parsed["cik"] == "0000000001"


# --- load ---

def test_load_roundtrip_preserves_all_fields(tmp_path):
    company = _make_company()
    save(company, data_dir=tmp_path)
    clear_cache()
    loaded = load("TEST", data_dir=tmp_path)
    assert loaded.ticker == "TEST"
    assert loaded.cik == "0000000001"
    assert loaded.company_name == "Test Corp"
    assert len(loaded.periods) == 2
    assert loaded.periods[0].revenue == 1_000_000_000.0
    assert loaded.periods[0].period_end == date(2022, 12, 31)


def test_load_returns_cached_instance_on_second_call(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    clear_cache()
    first = load("TEST", data_dir=tmp_path)
    second = load("TEST", data_dir=tmp_path)
    assert first is second


def test_load_ticker_is_case_insensitive(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    clear_cache()
    loaded = load("test", data_dir=tmp_path)
    assert loaded.ticker == "TEST"


def test_load_missing_file_raises_file_not_found(tmp_path):
    clear_cache()
    with pytest.raises(FileNotFoundError):
        load("MISSING", data_dir=tmp_path)


# --- get_series ---

def test_get_series_returns_date_value_pairs(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    clear_cache()
    series = get_series("TEST", "revenue", data_dir=tmp_path)
    assert series == [
        (date(2022, 12, 31), 1_000_000_000.0),
        (date(2023, 12, 31), 1_200_000_000.0),
    ]


def test_get_series_sorted_ascending(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    clear_cache()
    series = get_series("TEST", "revenue", data_dir=tmp_path)
    dates = [d for d, _ in series]
    assert dates == sorted(dates)


def test_get_series_free_cash_flow_uses_derived_property(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    clear_cache()
    series = get_series("TEST", "free_cash_flow", data_dir=tmp_path)
    assert series[0][1] == 200_000_000.0 - 50_000_000.0
    assert series[1][1] == 250_000_000.0 - 60_000_000.0


def test_get_series_fcf_margin_uses_derived_property(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    clear_cache()
    series = get_series("TEST", "fcf_margin", data_dir=tmp_path)
    expected = (200_000_000.0 - 50_000_000.0) / 1_000_000_000.0
    assert abs(series[0][1] - expected) < 1e-9


def test_get_series_invalid_metric_raises_value_error(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    clear_cache()
    with pytest.raises(ValueError, match="not a valid metric"):
        get_series("TEST", "nonexistent_field", data_dir=tmp_path)


# --- compare ---

def test_compare_returns_dict_keyed_by_ticker(tmp_path):
    save(_make_company(), data_dir=tmp_path)
    clear_cache()
    result = compare(["TEST"], "revenue", data_dir=tmp_path)
    assert "TEST" in result
    assert len(result["TEST"]) == 2


def test_compare_multiple_tickers(tmp_path):
    c1 = _make_company()
    c2 = CompanyFinancials(
        ticker="OTHER", cik="0000000002", company_name="Other Corp",
        periods=[
            PeriodFinancials(
                ticker="OTHER", period_end=date(2023, 12, 31),
                form="10-K", filed_date=date(2024, 2, 1),
                revenue=500_000_000.0,
            )
        ],
    )
    save(c1, data_dir=tmp_path)
    save(c2, data_dir=tmp_path)
    clear_cache()
    result = compare(["TEST", "OTHER"], "revenue", data_dir=tmp_path)
    assert set(result.keys()) == {"TEST", "OTHER"}


# --- new fields round-trip ---

def test_save_and_load_preserves_new_fields(tmp_path):
    from financial_analysis.store import save, load, clear_cache
    clear_cache()
    p = PeriodFinancials(
        ticker="TEST", period_end=date(2024, 1, 28),
        form="10-K", filed_date=date(2024, 2, 1),
        revenue=60_000_000_000,
        cash_and_equivalents=7_000_000_000,
        repurchases_common_stock=-9_500_000_000,
        raw_extras={"us-gaap/SomeUnmappedConcept": 999_000_000},
    )
    company = CompanyFinancials(ticker="TEST", cik="0001", company_name="Test Co", periods=[p])
    save(company, data_dir=tmp_path)
    clear_cache()
    loaded = load("TEST", data_dir=tmp_path)
    lp = loaded.periods[0]
    assert lp.cash_and_equivalents == 7_000_000_000
    assert lp.repurchases_common_stock == -9_500_000_000
    assert lp.raw_extras["us-gaap/SomeUnmappedConcept"] == 999_000_000
