"""The Python importer only runs the Rust program and reads its store file.

The mapping rules have their tests in Rust (cargo test --manifest-path
rust/mapcheck/Cargo.toml). These tests run only if the Rust program is built.
"""
import json
from datetime import date

import pytest

from financial_analysis import importer, store
from financial_analysis.importer import MapcheckError, UnsupportedFilingError, import_company
from financial_analysis.schema import CompanyFinancials

needs_rust = pytest.mark.skipif(not importer.mapcheck_binary().exists(), reason="the Rust program mapcheck is not built")


def _f(start, end, val, form="10-K", filed="2025-02-01"):
    entry = {"end": end, "val": val, "form": form, "filed": filed, "accn": "0000000001-25-000001"}
    if start:
        entry["start"] = start
    return entry


def _cache(tmp_path, facts: dict, ticker: str = "ACME"):
    folder = tmp_path / "raw" / ticker
    folder.mkdir(parents=True)
    (folder / "company_facts.json").write_text(json.dumps(facts), encoding="utf-8")
    return tmp_path / "raw"


def _acme() -> dict:
    fy = ("2024-01-01", "2024-12-31")
    usd = {
        "Revenues": [_f(*fy, 1000.0), _f("2024-04-01", "2024-06-30", 240.0, "10-Q", "2024-08-01")],
        "CostOfRevenue": [_f(*fy, 560.0)],
        "NetCashProvidedByUsedInOperatingActivities": [_f(*fy, 300.0)],
        "PaymentsToAcquirePropertyPlantAndEquipment": [_f(*fy, 50.0)],
        "PaymentsToDevelopSoftware": [_f(*fy, 30.0)],
        "Assets": [_f(None, "2024-12-31", 1500.0)],
        "SomeCompanyDetail": [_f(*fy, 3.0)],
    }
    return {"cik": 1, "entityName": "ACME CORP",
            "facts": {"us-gaap": {name: {"units": {"USD": entries}} for name, entries in usd.items()}}}


@needs_rust
def test_import_company_returns_the_values_of_the_rust_program(tmp_path):
    company = import_company("acme", save=False, cache_dir=_cache(tmp_path, _acme()))
    assert isinstance(company, CompanyFinancials)
    assert (company.ticker, company.company_name, company.cik) == ("ACME", "ACME CORP", "1")
    annual = [p for p in company.periods if p.form == "10-K"]
    quarterly = [p for p in company.periods if p.form == "10-Q"]
    assert len(annual) == 1 and len(quarterly) == 1
    p = annual[0]
    assert p.period_end == date(2024, 12, 31) and p.filed_date == date(2025, 2, 1)
    assert p.revenue == 1000.0 and p.gross_profit == 440.0
    assert p.sources["gross_profit"] == "= revenue - cogs_total"
    assert p.capex == -50.0 and p.capex_total == -80.0          # rule R2
    assert p.free_cash_flow == 220.0
    assert p.raw_extras == {"us-gaap/SomeCompanyDetail": 3.0}
    assert p.total_debt is None and p.segment_revenue is None


@needs_rust
def test_import_company_saves_the_store_file_only_when_asked(tmp_path, monkeypatch):
    monkeypatch.setattr(store, "_DEFAULT_DATA_DIR", tmp_path / "processed")
    cache = _cache(tmp_path, _acme())
    import_company("ACME", save=False, cache_dir=cache)
    assert not (tmp_path / "processed" / "ACME.json").exists()
    import_company("ACME", save=True, cache_dir=cache)
    assert (tmp_path / "processed" / "ACME.json").exists()
    store.clear_cache()


@needs_rust
def test_n_periods_counts_fiscal_years(tmp_path):
    facts = {"cik": 1, "entityName": "ACME", "facts": {"us-gaap": {"Revenues": {"units": {"USD": [
        _f(f"{y}-01-01", f"{y}-12-31", float(y)) for y in range(2010, 2025)]}}}}}
    company = import_company("ACME", n_periods=5, save=False, cache_dir=_cache(tmp_path, facts))
    assert [p.period_end.year for p in company.periods] == [2020, 2021, 2022, 2023, 2024]


@needs_rust
def test_ifrs_filer_is_not_supported(tmp_path):
    facts = {"cik": 2, "entityName": "SPOTIFY AB", "facts": {"ifrs-full": {}}}
    with pytest.raises(UnsupportedFilingError):
        import_company("SPOT", save=False, cache_dir=_cache(tmp_path, facts, "SPOT"))


def test_missing_rust_program_gives_a_clear_error(tmp_path, monkeypatch):
    monkeypatch.setenv(importer.BINARY_ENV, str(tmp_path / "no-such-program"))
    with pytest.raises(MapcheckError, match="cargo build"):
        import_company("ACME", save=False, cache_dir=tmp_path)
