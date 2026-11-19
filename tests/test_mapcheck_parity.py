"""The Rust mapper (rust/mapcheck) and the Python importer must give the same result.

The test uses synthetic company facts (no network). It runs only if the Rust program
is built: cargo build --release --manifest-path rust/mapcheck/Cargo.toml
"""
import json
import subprocess

import pytest

from scripts.mapping_parity import BINARY, run

pytestmark = pytest.mark.skipif(not BINARY.exists(), reason="the Rust program mapcheck is not built")


def _f(start, end, val, form="10-K", filed="2025-02-01"):
    entry = {"end": end, "val": val, "form": form, "filed": filed, "accn": "x", "fy": 2024, "fp": "FY"}
    if start:
        entry["start"] = start
    return entry


def _years(values, form="10-K"):
    """One full-year fact for each (year, value)."""
    return [_f(f"{y}-01-01", f"{y}-12-31", v, form, f"{y + 1}-02-01") for y, v in values]


def _balances(values):
    return [_f(None, f"{y}-12-31", v, "10-K", f"{y + 1}-02-01") for y, v in values]


def _acme() -> dict:
    usd = {
        # Revenue: one restated year, a quarter value in the 10-K, and quarters.
        "Revenues": _years([(2022, 900.0), (2023, 950.0), (2024, 1000.0)]) + [
            _f("2023-01-01", "2023-12-31", 955.0, "10-K/A", "2025-03-01"),      # restated: latest filing wins
            _f("2024-10-01", "2024-12-31", 270.0, "10-K", "2025-02-02"),        # Q4 in the 10-K
            _f("2024-01-01", "2024-03-31", 230.0, "10-Q", "2024-05-01"),
            _f("2024-04-01", "2024-06-30", 240.0, "10-Q", "2024-08-01"),
            _f("2024-01-01", "2024-06-30", 470.0, "10-Q", "2024-08-01"),        # year to date
            _f("2024-07-01", "2024-09-30", 260.0, "10-Q", "2024-11-01"),
        ],
        "CostOfRevenue": _years([(2022, 500.0), (2023, 520.0), (2024, 560.0)]),
        "NetIncomeLoss": _years([(2022, 90.0), (2023, 95.0), (2024, 100.0)]),
        "ProfitLoss": _years([(2024, 104.0)]),
        "IncomeTaxExpenseBenefit": _years([(2024, 25.0)]),
        "IncomeLossFromContinuingOperationsBeforeIncomeTaxesDomestic": _years([(2024, 80.0)]),
        "IncomeLossFromContinuingOperationsBeforeIncomeTaxesForeign": _years([(2024, 49.0)]),
        "Depreciation": _years([(2024, 40.0)]),
        "InterestPaidNet": _years([(2024, 12.0)]),
        # Cash flow: year-to-date values only in the 10-Q.
        "NetCashProvidedByUsedInOperatingActivities": _years([(2022, 180.0), (2023, 190.0), (2024, 200.0)]) + [
            _f("2024-01-01", "2024-03-31", 40.0, "10-Q", "2024-05-01"),
            _f("2024-01-01", "2024-06-30", 95.0, "10-Q", "2024-08-01"),
            _f("2024-01-01", "2024-09-30", 150.0, "10-Q", "2024-11-01"),
        ],
        "PaymentsToAcquirePropertyPlantAndEquipment": _years([(2022, 45.0), (2023, 48.0), (2024, 50.0)]),
        "PaymentsForProceedsFromOtherInvestingActivities": _years([(2024, -7.0)]),   # negate
        "PaymentsOfDividendsCommonStock": _years([(2024, 30.0)]),
        # Balance sheet, with one debt item in two tags (same value).
        "CashAndCashEquivalentsAtCarryingValue": _balances([(2023, 280.0), (2024, 300.0)]),
        "ShortTermInvestments": _balances([(2024, 100.0)]),
        "Assets": _balances([(2023, 1400.0), (2024, 1500.0)]),
        "Liabilities": _balances([(2024, 600.0)]),
        "StockholdersEquity": _balances([(2024, 880.0)]),
        "MinorityInterest": _balances([(2024, 20.0)]),
        "LiabilitiesAndStockholdersEquity": _balances([(2024, 1500.0)]),
        "ShortTermBorrowings": _balances([(2024, 64.0)]),
        "LongTermDebtAndCapitalLeaseObligationsCurrent": _balances([(2024, 64.0)]),
        "LongTermDebtNoncurrent": _balances([(2024, 548.0)]),
        "FiniteLivedIntangibleAssetsNet": _balances([(2024, 70.0)]),
        # No standard variable: stays in raw_extras.
        "AssetRetirementObligation": _balances([(2024, 80.0)]),
        "SomeFlowDetail": _years([(2024, 3.0)]) + [_f("2024-10-01", "2024-12-31", 1.0, "10-K", "2025-03-01")],
    }
    gaap = {name: {"units": {"USD": entries}} for name, entries in usd.items()}
    gaap["WeightedAverageNumberOfDilutedSharesOutstanding"] = {"units": {"shares": _years([(2024, 100.0)])}}
    gaap["EarningsPerShareDiluted"] = {"units": {"USD/shares": _years([(2024, 1.0)])}}
    return {"cik": 1, "entityName": "ACME CORP", "facts": {"us-gaap": gaap, "dei": {}}}


def _week53() -> dict:
    """A 52/53-week filer, with two end dates for one fiscal year."""
    revenue = [
        _f("2022-10-31", "2023-10-29", 100.0, "10-K", "2023-12-01"),
        _f("2023-10-30", "2024-11-03", 110.0, "10-K", "2024-12-10"),   # 53 weeks
        _f("2023-10-29", "2024-11-02", 110.0, "10-K", "2025-12-10"),   # the same year, one day different
    ]
    net_income = [
        _f("2022-10-31", "2023-10-29", 9.0, "10-K", "2023-12-01"),
        _f("2023-10-30", "2024-11-03", 10.0, "10-K", "2024-12-10"),
    ]
    gaap = {
        "RevenueFromContractWithCustomerExcludingAssessedTax": {"units": {"USD": revenue}},
        "NetIncomeLoss": {"units": {"USD": net_income}},
        "Assets": {"units": {"USD": [_f(None, "2024-11-03", 500.0, "10-K", "2024-12-10")]}},
    }
    return {"cik": 2, "entityName": "WEEK53 INC", "facts": {"us-gaap": gaap}}


@pytest.fixture
def cache(tmp_path):
    for ticker, facts in (("ACME", _acme()), ("WEEK", _week53())):
        folder = tmp_path / ticker
        folder.mkdir()
        (folder / "company_facts.json").write_text(json.dumps(facts), encoding="utf-8")
    return tmp_path


def test_rust_and_python_give_the_same_values(cache):
    diffs, periods, compared = run(["ACME", "WEEK"], cache)
    assert diffs == []
    assert periods == 3 + 3 + 2      # ACME: 3 years and 3 quarters; WEEK: 2 years
    assert compared > 60


def test_rust_program_reports_the_synthetic_company(cache, tmp_path):
    out = tmp_path / "report.jsonl"
    result = subprocess.run(
        [str(BINARY), "--cache", str(cache), "--as-of", "2025-03-01", "--out", str(out), "ACME"],
        capture_output=True, text=True, check=True,
    )
    assert "mapcheck: 1 tickers" in result.stdout
    report = json.loads(out.read_text(encoding="utf-8").splitlines()[0])
    assert report["ticker"] == "ACME" and report["annual_rows"] == 3 and report["quarterly_rows"] == 3
    assert report["status"] in ("PASS", "WARN")           # no Fail on a consistent company
    assert ["us-gaap/AssetRetirementObligation", 80.0] in report["unmapped"]
    codes = {f["code"] for f in report["findings"]}
    # One debt item has two tags (the IBM pattern): the two estimates of total debt
    # disagree, and the framework reports it.
    assert "total_debt_estimates" in codes


def test_strict_mode_gives_exit_code_1_on_a_fail(cache):
    # WEEK has no cash flow and no shares: critical inputs have no value.
    result = subprocess.run(
        [str(BINARY), "--cache", str(cache), "--as-of", "2025-03-01", "--strict", "WEEK"],
        capture_output=True, text=True,
    )
    assert result.returncode == 1
    assert "core_missing_critical" in result.stdout
