---
id: DOC-PLAN-FINANCIAL-ANALYSIS-PIPELINE
type: plan
item: 10.16 Development plan
status: superseded
date: 2026-06-03
scope: Historical implementation plan for the EDGAR import and Monte Carlo pipeline; superseded by docs/ARCHITECTURE.md.
---
# Financial Analysis Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Python pipeline that ingests SEC EDGAR filings via the XBRL JSON API, populates a time-series financial schema, persists it as human-readable JSON, and runs a Monte Carlo stochastic DCF valuation.

**Architecture:** EDGAR's `edgar.py` handles CIK lookup. `xbrl_api.py` fetches structured financial facts from `data.sec.gov/api/xbrl/companyfacts/`. `importer.py` orchestrates these into `CompanyFinancials` and auto-saves via `store.py`. `store.py` provides a session-cached load, per-metric time series queries, and cross-ticker comparison. `dcf.py`'s `StochasticDCF` loads from store (no live API call needed at analysis time).

**Tech Stack:** Python 3.12, dataclasses, numpy (Monte Carlo), json (persistence), pytest + unittest.mock (tests), urllib (HTTP)

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `financial_analysis/edgar.py` | Existing | CIK lookup, raw file download, rate limiting |
| `financial_analysis/xbrl_api.py` | Create | Fetch & extract SEC XBRL JSON facts |
| `financial_analysis/schema.py` | Create | `PeriodFinancials` + `CompanyFinancials` dataclasses |
| `financial_analysis/store.py` | Create | Save/load JSON, session cache, `get_series`, `compare` |
| `financial_analysis/importer.py` | Create | Orchestrate CIK → XBRL API → schema → auto-save |
| `financial_analysis/dcf.py` | Create | Monte Carlo stochastic DCF model |
| `scripts/import_financials.py` | Create | CLI entrypoint for importer |
| `tests/test_edgar.py` | Create | Unit tests for edgar.py pure functions |
| `tests/test_xbrl_api.py` | Create | Unit tests for xbrl_api.py (mocked HTTP) |
| `tests/test_schema.py` | Create | Unit tests for schema dataclasses |
| `tests/test_store.py` | Create | Unit tests for store (tmp_path fixture, no real data/ writes) |
| `tests/test_importer.py` | Create | Unit tests for importer (mocked API) |
| `tests/test_dcf.py` | Create | Unit tests for StochasticDCF |
| `context.json` | Modify | Add stochastic/MC direction note |
| `README.md` | Modify | Document full pipeline |

---

## Task 1: Smoke-test the existing EDGAR import (Ticket 1 close-out)

**Files:**
- Run: `scripts/import_edgar.py` (existing)
- Inspect: `data/raw/PANW/` (existing)

- [ ] **Step 1: Run the EDGAR importer against PANW**

```bash
python -m scripts.import_edgar PANW 2024-07-31 --output-dir data/raw
```

Expected output includes:
```
Ticker: PANW
CIK: 1327567
Selected filings: <some number > 0>
Downloaded files: <some number > 0>
```

- [ ] **Step 2: Verify a downloaded file is real XBRL content**

Open any `.txt` file from `data/raw/PANW/` in a text editor. Confirm it starts with `<SEC-DOCUMENT>` or contains `<XBRL` or `<XML` tags — not an HTTP error page.

- [ ] **Step 3: Commit smoke test result as a note**

```bash
git add data/raw/PANW/
git commit -m "chore: confirm EDGAR download produces real XBRL content for PANW"
```

---

## Task 2: Write unit tests for edgar.py pure functions

**Files:**
- Create: `tests/__init__.py`
- Create: `tests/test_edgar.py`

- [ ] **Step 1: Create tests directory and write failing tests**

`tests/__init__.py` — empty file.

`tests/test_edgar.py`:
```python
from datetime import date
from pathlib import Path
from unittest.mock import MagicMock, patch
import pytest

from financial_analysis.edgar import (
    _parse_date,
    _zero_pad_cik,
    _safe_filename,
    _quarter_for_date,
    _quarter_range,
    _master_index_url,
    _select_index_entries,
    get_cik_for_ticker,
    EdgarClientError,
)


def test_parse_date_iso():
    assert _parse_date("2024-07-31") == date(2024, 7, 31)


def test_parse_date_compact():
    assert _parse_date("20240731") == date(2024, 7, 31)


def test_parse_date_slash():
    assert _parse_date("07/31/2024") == date(2024, 7, 31)


def test_parse_date_none():
    assert _parse_date(None) is None


def test_parse_date_empty():
    assert _parse_date("") is None


def test_zero_pad_cik():
    assert _zero_pad_cik("1327567") == "0001327567"


def test_zero_pad_cik_already_padded():
    assert _zero_pad_cik("0001327567") == "0001327567"


def test_safe_filename_replaces_slashes():
    assert _safe_filename("edgar/data/123/filing.htm") == "edgar_data_123_filing.htm"


def test_safe_filename_allows_dots_and_dashes():
    assert _safe_filename("file-name.xml") == "file-name.xml"


def test_quarter_for_date_q1():
    assert _quarter_for_date(date(2024, 1, 15)) == 1


def test_quarter_for_date_q4():
    assert _quarter_for_date(date(2024, 12, 31)) == 4


def test_quarter_range_single():
    result = list(_quarter_range(date(2024, 1, 1), date(2024, 3, 31)))
    assert result == [(2024, 1)]


def test_quarter_range_crosses_year():
    result = list(_quarter_range(date(2024, 4, 1), date(2025, 1, 1)))
    assert (2024, 4) in result
    assert (2025, 1) in result


def test_master_index_url():
    url = _master_index_url(2024, 3)
    assert url == "https://www.sec.gov/Archives/edgar/full-index/2024/QTR3/master.idx"


def test_select_index_entries_filters_by_cik():
    entries = [
        {"cik": "1327567", "form": "10-K", "date_filed": date(2024, 9, 12), "filename": "a.txt", "company_name": "PANW"},
        {"cik": "9999999", "form": "10-K", "date_filed": date(2024, 9, 12), "filename": "b.txt", "company_name": "OTHER"},
    ]
    result = _select_index_entries(entries, "1327567", date(2024, 7, 31))
    assert len(result) == 1
    assert result[0]["cik"] == "1327567"


def test_select_index_entries_filters_by_form():
    entries = [
        {"cik": "1327567", "form": "10-K", "date_filed": date(2024, 9, 12), "filename": "a.txt", "company_name": "PANW"},
        {"cik": "1327567", "form": "8-K",  "date_filed": date(2024, 9, 12), "filename": "b.txt", "company_name": "PANW"},
    ]
    result = _select_index_entries(entries, "1327567", date(2024, 7, 31), forms=["10-K"])
    assert len(result) == 1
    assert result[0]["form"] == "10-K"


def test_get_cik_for_ticker_not_found():
    payload = b'{"0": {"ticker": "AAPL", "cik_str": "320193"}}'
    with patch("financial_analysis.edgar._http_get", return_value=payload):
        with pytest.raises(EdgarClientError, match="CIK not found"):
            get_cik_for_ticker("PANW")


def test_get_cik_for_ticker_found():
    payload = b'{"0": {"ticker": "PANW", "cik_str": "1327567"}}'
    with patch("financial_analysis.edgar._http_get", return_value=payload):
        assert get_cik_for_ticker("PANW") == "1327567"
```

- [ ] **Step 2: Run tests — expect them to pass (edgar.py already exists)**

```bash
pytest tests/test_edgar.py -v
```

Expected: all green. If any fail, fix `edgar.py` before continuing.

- [ ] **Step 3: Commit**

```bash
git add tests/__init__.py tests/test_edgar.py
git commit -m "test: add unit tests for edgar.py pure functions"
```

---

## Task 3: Implement xbrl_api.py (Ticket 2)

**Files:**
- Create: `financial_analysis/xbrl_api.py`
- Create: `tests/test_xbrl_api.py`

- [ ] **Step 1: Write failing tests**

`tests/test_xbrl_api.py`:
```python
from unittest.mock import patch
import json
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
                        {"accn": "0001327567-24-000015", "end": "2024-07-31", "val": 8030000000, "form": "10-K", "filed": "2024-09-12"},
                        {"accn": "0001327567-23-000024", "end": "2023-07-31", "val": 6893000000, "form": "10-K", "filed": "2023-09-11"},
                        {"accn": "0001327567-24-000008", "end": "2024-04-30", "val": 1980000000, "form": "10-Q", "filed": "2024-06-07"},
                    ]
                }
            }
        }
    }
}


def test_extract_annual_series_returns_only_10k():
    result = extract_annual_series(SAMPLE_FACTS, "us-gaap/Revenues")
    assert all(e["form"] == "10-K" for e in result)
    assert len(result) == 2


def test_extract_annual_series_sorted_ascending():
    result = extract_annual_series(SAMPLE_FACTS, "us-gaap/Revenues")
    assert result[0]["end"] < result[1]["end"]


def test_extract_annual_series_missing_concept_returns_empty():
    result = extract_annual_series(SAMPLE_FACTS, "us-gaap/NonExistentConcept")
    assert result == []


def test_extract_annual_series_missing_namespace_returns_empty():
    result = extract_annual_series(SAMPLE_FACTS, "nonexistent/Revenues")
    assert result == []


def test_extract_quarterly_series_returns_only_10q():
    result = extract_quarterly_series(SAMPLE_FACTS, "us-gaap/Revenues")
    assert all(e["form"] == "10-Q" for e in result)
    assert len(result) == 1


def test_fetch_company_facts_calls_correct_url():
    payload = json.dumps(SAMPLE_FACTS).encode()
    with patch("financial_analysis.xbrl_api._http_get", return_value=payload) as mock_get:
        result = fetch_company_facts("1327567")
        called_url = mock_get.call_args[0][0]
        assert "CIK0001327567" in called_url
        assert result["entityName"] == "Palo Alto Networks Inc"
```

- [ ] **Step 2: Run tests — expect ImportError (module doesn't exist yet)**

```bash
pytest tests/test_xbrl_api.py -v
```

Expected: `ModuleNotFoundError: No module named 'financial_analysis.xbrl_api'`

- [ ] **Step 3: Implement xbrl_api.py**

`financial_analysis/xbrl_api.py`:
```python
import json
from typing import Any, Dict, List

from financial_analysis.edgar import _http_get, _zero_pad_cik

COMPANY_FACTS_URL = "https://data.sec.gov/api/xbrl/companyfacts/CIK{cik}.json"


def fetch_company_facts(cik: str) -> Dict[str, Any]:
    padded = _zero_pad_cik(cik)
    url = COMPANY_FACTS_URL.format(cik=padded)
    payload = _http_get(url)
    return json.loads(payload)


def _get_usd_entries(facts: Dict[str, Any], concept: str) -> List[Dict[str, Any]]:
    parts = concept.split("/", 1)
    if len(parts) != 2:
        return []
    namespace, name = parts
    try:
        return facts["facts"][namespace][name]["units"]["USD"]
    except KeyError:
        return []


def extract_annual_series(facts: Dict[str, Any], concept: str) -> List[Dict[str, Any]]:
    entries = [e for e in _get_usd_entries(facts, concept) if e.get("form") == "10-K"]
    return sorted(entries, key=lambda e: e["end"])


def extract_quarterly_series(facts: Dict[str, Any], concept: str) -> List[Dict[str, Any]]:
    entries = [e for e in _get_usd_entries(facts, concept) if e.get("form") == "10-Q"]
    return sorted(entries, key=lambda e: e["end"])
```

- [ ] **Step 4: Run tests — expect all green**

```bash
pytest tests/test_xbrl_api.py -v
```

- [ ] **Step 5: Smoke test against live SEC API**

```bash
python -c "
from financial_analysis.xbrl_api import fetch_company_facts, extract_annual_series
facts = fetch_company_facts('1327567')
revenue = extract_annual_series(facts, 'us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax')
if not revenue:
    revenue = extract_annual_series(facts, 'us-gaap/Revenues')
for e in revenue[-5:]:
    print(e['end'], e['val'])
"
```

Expected: 5 lines of `YYYY-MM-DD  <dollar amount>` for PANW revenue. Confirm values are plausible (billions range).

- [ ] **Step 6: Commit**

```bash
git add financial_analysis/xbrl_api.py tests/test_xbrl_api.py
git commit -m "feat: add XBRL JSON API client with annual/quarterly series extraction"
```

---

## Task 4: Implement schema.py (Ticket 3)

**Files:**
- Create: `financial_analysis/schema.py`
- Create: `tests/test_schema.py`

- [ ] **Step 1: Write failing tests**

`tests/test_schema.py`:
```python
from datetime import date
import pytest

from financial_analysis.schema import PeriodFinancials, CompanyFinancials


def test_free_cash_flow_derived_when_both_present():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31),
        form="10-K", filed_date=date(2024, 9, 12),
        operating_cash_flow=3_000_000_000.0,
        capex=200_000_000.0,
    )
    assert p.free_cash_flow == 2_800_000_000.0


def test_free_cash_flow_none_when_ocf_missing():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31),
        form="10-K", filed_date=date(2024, 9, 12),
        capex=200_000_000.0,
    )
    assert p.free_cash_flow is None


def test_free_cash_flow_none_when_capex_missing():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31),
        form="10-K", filed_date=date(2024, 9, 12),
        operating_cash_flow=3_000_000_000.0,
    )
    assert p.free_cash_flow is None


def test_company_financials_holds_multiple_periods():
    periods = [
        PeriodFinancials(ticker="PANW", period_end=date(2023, 7, 31), form="10-K", filed_date=date(2023, 9, 11)),
        PeriodFinancials(ticker="PANW", period_end=date(2024, 7, 31), form="10-K", filed_date=date(2024, 9, 12)),
    ]
    company = CompanyFinancials(ticker="PANW", cik="1327567", company_name="Palo Alto Networks Inc", periods=periods)
    assert len(company.periods) == 2
    assert company.periods[0].period_end < company.periods[1].period_end


def test_period_financials_defaults_to_usd():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31),
        form="10-K", filed_date=date(2024, 9, 12),
    )
    assert p.currency == "USD"


def test_all_financial_fields_optional():
    p = PeriodFinancials(
        ticker="PANW", period_end=date(2024, 7, 31),
        form="10-K", filed_date=date(2024, 9, 12),
    )
    for field in ("revenue", "gross_profit", "operating_income", "net_income",
                  "rd_expense", "sga_expense", "operating_cash_flow", "capex",
                  "total_assets", "total_liabilities", "equity", "shares_outstanding"):
        assert getattr(p, field) is None, f"{field} should default to None"
```

- [ ] **Step 2: Run tests — expect ImportError**

```bash
pytest tests/test_schema.py -v
```

- [ ] **Step 3: Implement schema.py**

`financial_analysis/schema.py`:
```python
from __future__ import annotations
from dataclasses import dataclass, field
from datetime import date
from typing import List, Optional


@dataclass
class PeriodFinancials:
    ticker: str
    period_end: date
    form: str
    filed_date: date
    currency: str = "USD"
    revenue: Optional[float] = None
    gross_profit: Optional[float] = None
    operating_income: Optional[float] = None
    net_income: Optional[float] = None
    rd_expense: Optional[float] = None
    sga_expense: Optional[float] = None
    operating_cash_flow: Optional[float] = None
    capex: Optional[float] = None
    total_assets: Optional[float] = None
    total_liabilities: Optional[float] = None
    equity: Optional[float] = None
    shares_outstanding: Optional[float] = None

    @property
    def free_cash_flow(self) -> Optional[float]:
        if self.operating_cash_flow is None or self.capex is None:
            return None
        return self.operating_cash_flow - self.capex


@dataclass
class CompanyFinancials:
    ticker: str
    cik: str
    company_name: str
    periods: List[PeriodFinancials] = field(default_factory=list)
```

- [ ] **Step 4: Run tests — expect all green**

```bash
pytest tests/test_schema.py -v
```

- [ ] **Step 5: Commit**

```bash
git add financial_analysis/schema.py tests/test_schema.py
git commit -m "feat: add time-series financial schema dataclasses"
```

---

## Task 5: Implement store.py (Ticket 3.5)

**Files:**
- Create: `financial_analysis/store.py`
- Create: `tests/test_store.py`

- [ ] **Step 1: Write failing tests**

`tests/test_store.py`:
```python
from datetime import date
from pathlib import Path
import json
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
                revenue=1_000_000_000.0, operating_cash_flow=200_000_000.0, capex=50_000_000.0,
            ),
            PeriodFinancials(
                ticker="TEST", period_end=date(2023, 12, 31),
                form="10-K", filed_date=date(2024, 2, 14),
                revenue=1_200_000_000.0, operating_cash_flow=250_000_000.0, capex=60_000_000.0,
            ),
        ],
    )


def test_save_creates_json_file(tmp_path):
    company = _make_company()
    save(company, data_dir=tmp_path)
    assert (tmp_path / "TEST.json").exists()


def test_save_json_is_human_readable(tmp_path):
    company = _make_company()
    save(company, data_dir=tmp_path)
    raw = (tmp_path / "TEST.json").read_text()
    parsed = json.loads(raw)
    assert parsed["ticker"] == "TEST"
    assert isinstance(parsed["periods"], list)
    assert parsed["periods"][0]["period_end"] == "2022-12-31"


def test_load_roundtrip(tmp_path):
    company = _make_company()
    save(company, data_dir=tmp_path)
    clear_cache()
    loaded = load("TEST", data_dir=tmp_path)
    assert loaded.ticker == "TEST"
    assert len(loaded.periods) == 2
    assert loaded.periods[0].revenue == 1_000_000_000.0


def test_load_returns_cached_instance(tmp_path):
    company = _make_company()
    save(company, data_dir=tmp_path)
    clear_cache()
    first = load("TEST", data_dir=tmp_path)
    second = load("TEST", data_dir=tmp_path)
    assert first is second


def test_load_missing_file_raises(tmp_path):
    clear_cache()
    with pytest.raises(FileNotFoundError):
        load("MISSING", data_dir=tmp_path)


def test_get_series_returns_sorted_pairs(tmp_path):
    company = _make_company()
    save(company, data_dir=tmp_path)
    clear_cache()
    series = get_series("TEST", "revenue", data_dir=tmp_path)
    assert series == [(date(2022, 12, 31), 1_000_000_000.0), (date(2023, 12, 31), 1_200_000_000.0)]


def test_get_series_free_cash_flow_derived(tmp_path):
    company = _make_company()
    save(company, data_dir=tmp_path)
    clear_cache()
    series = get_series("TEST", "free_cash_flow", data_dir=tmp_path)
    assert series[0][1] == 200_000_000.0 - 50_000_000.0
    assert series[1][1] == 250_000_000.0 - 60_000_000.0


def test_get_series_invalid_metric_raises(tmp_path):
    company = _make_company()
    save(company, data_dir=tmp_path)
    clear_cache()
    with pytest.raises(ValueError, match="not a valid metric"):
        get_series("TEST", "nonexistent_field", data_dir=tmp_path)


def test_compare_returns_dict_keyed_by_ticker(tmp_path):
    company = _make_company()
    save(company, data_dir=tmp_path)
    clear_cache()
    result = compare(["TEST"], "revenue", data_dir=tmp_path)
    assert "TEST" in result
    assert len(result["TEST"]) == 2
```

- [ ] **Step 2: Run tests — expect ImportError**

```bash
pytest tests/test_store.py -v
```

- [ ] **Step 3: Implement store.py**

`financial_analysis/store.py`:
```python
from __future__ import annotations
import json
from datetime import date, datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from financial_analysis.schema import CompanyFinancials, PeriodFinancials

_CACHE: Dict[str, CompanyFinancials] = {}

_PERIOD_FIELDS = {
    f for f in PeriodFinancials.__dataclass_fields__
} | {"free_cash_flow"}

_DEFAULT_DATA_DIR = Path("data/processed")


def clear_cache() -> None:
    _CACHE.clear()


def save(company: CompanyFinancials, data_dir: Path = _DEFAULT_DATA_DIR) -> Path:
    data_dir.mkdir(parents=True, exist_ok=True)
    path = data_dir / f"{company.ticker.upper()}.json"

    def _serialize_period(p: PeriodFinancials) -> dict:
        return {
            "ticker": p.ticker,
            "period_end": p.period_end.isoformat(),
            "form": p.form,
            "filed_date": p.filed_date.isoformat(),
            "currency": p.currency,
            "revenue": p.revenue,
            "gross_profit": p.gross_profit,
            "operating_income": p.operating_income,
            "net_income": p.net_income,
            "rd_expense": p.rd_expense,
            "sga_expense": p.sga_expense,
            "operating_cash_flow": p.operating_cash_flow,
            "capex": p.capex,
            "total_assets": p.total_assets,
            "total_liabilities": p.total_liabilities,
            "equity": p.equity,
            "shares_outstanding": p.shares_outstanding,
        }

    payload = {
        "ticker": company.ticker,
        "cik": company.cik,
        "company_name": company.company_name,
        "periods": [_serialize_period(p) for p in company.periods],
    }
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    _CACHE[company.ticker.upper()] = company
    return path


def load(ticker: str, data_dir: Path = _DEFAULT_DATA_DIR) -> CompanyFinancials:
    key = ticker.upper()
    if key in _CACHE:
        return _CACHE[key]

    path = data_dir / f"{key}.json"
    if not path.exists():
        raise FileNotFoundError(f"No saved data for {key} at {path}")

    raw = json.loads(path.read_text(encoding="utf-8"))

    def _to_date(s: str) -> date:
        return datetime.strptime(s, "%Y-%m-%d").date()

    periods = [
        PeriodFinancials(
            ticker=p["ticker"],
            period_end=_to_date(p["period_end"]),
            form=p["form"],
            filed_date=_to_date(p["filed_date"]),
            currency=p.get("currency", "USD"),
            revenue=p.get("revenue"),
            gross_profit=p.get("gross_profit"),
            operating_income=p.get("operating_income"),
            net_income=p.get("net_income"),
            rd_expense=p.get("rd_expense"),
            sga_expense=p.get("sga_expense"),
            operating_cash_flow=p.get("operating_cash_flow"),
            capex=p.get("capex"),
            total_assets=p.get("total_assets"),
            total_liabilities=p.get("total_liabilities"),
            equity=p.get("equity"),
            shares_outstanding=p.get("shares_outstanding"),
        )
        for p in raw["periods"]
    ]

    company = CompanyFinancials(
        ticker=raw["ticker"],
        cik=raw["cik"],
        company_name=raw["company_name"],
        periods=periods,
    )
    _CACHE[key] = company
    return company


def get_series(
    ticker: str,
    metric: str,
    data_dir: Path = _DEFAULT_DATA_DIR,
) -> List[Tuple[date, Optional[float]]]:
    if metric not in _PERIOD_FIELDS:
        valid = sorted(_PERIOD_FIELDS)
        raise ValueError(f"'{metric}' is not a valid metric. Valid fields: {valid}")
    company = load(ticker, data_dir=data_dir)
    return [(p.period_end, getattr(p, metric)) for p in company.periods]


def compare(
    tickers: List[str],
    metric: str,
    data_dir: Path = _DEFAULT_DATA_DIR,
) -> Dict[str, List[Tuple[date, Optional[float]]]]:
    return {t.upper(): get_series(t, metric, data_dir=data_dir) for t in tickers}
```

- [ ] **Step 4: Run tests**

```bash
pytest tests/test_store.py -v
```

Expected: all green.

- [ ] **Step 5: Smoke test**

```bash
python -c "
from pathlib import Path
from financial_analysis.store import save, load, get_series, compare, clear_cache
from financial_analysis.importer import import_company

company = import_company('PANW', save=False)
save(company, data_dir=Path('data/processed'))
clear_cache()
loaded = load('PANW', data_dir=Path('data/processed'))
print('Revenue series:')
for dt, val in get_series('PANW', 'revenue', data_dir=Path('data/processed')):
    print(f'  {dt}  \${val/1e9:.2f}B')
print()
print('FCF margin series:')
rev = dict(get_series('PANW', 'revenue', data_dir=Path('data/processed')))
fcf = dict(get_series('PANW', 'free_cash_flow', data_dir=Path('data/processed')))
for dt in sorted(rev):
    if rev[dt] and fcf.get(dt):
        print(f'  {dt}  {fcf[dt]/rev[dt]:.1%}')
"
```

Expected: revenue rows in billions, FCF margins in the 20–40% range for PANW.

- [ ] **Step 6: Confirm file is human-readable**

Open `data/processed/PANW.json` in any text editor. Confirm it is pretty-printed JSON with ISO date strings and numeric values — not binary.

- [ ] **Step 7: Commit**

```bash
git add financial_analysis/store.py tests/test_store.py
git commit -m "feat: add data store with JSON persistence, session cache, and query helpers"
```

---

## Task 6: Implement importer.py + auto-save (Ticket 4)

**Files:**
- Create: `financial_analysis/importer.py`
- Create: `scripts/import_financials.py`
- Create: `tests/test_importer.py`

- [ ] **Step 1: Write failing tests**

`tests/test_importer.py`:
```python
from datetime import date
from unittest.mock import patch
import pytest

from financial_analysis.importer import import_company
from financial_analysis.schema import CompanyFinancials

SAMPLE_FACTS = {
    "entityName": "Palo Alto Networks Inc",
    "facts": {
        "us-gaap": {
            "RevenueFromContractWithCustomerExcludingAssessedTax": {
                "units": {"USD": [
                    {"end": "2024-07-31", "val": 8030000000, "form": "10-K", "filed": "2024-09-12"},
                    {"end": "2023-07-31", "val": 6893000000, "form": "10-K", "filed": "2023-09-11"},
                    {"end": "2022-07-31", "val": 5502000000, "form": "10-K", "filed": "2022-09-09"},
                ]}
            },
            "NetCashProvidedByUsedInOperatingActivities": {
                "units": {"USD": [
                    {"end": "2024-07-31", "val": 3050000000, "form": "10-K", "filed": "2024-09-12"},
                    {"end": "2023-07-31", "val": 2786000000, "form": "10-K", "filed": "2023-09-11"},
                    {"end": "2022-07-31", "val": 1688000000, "form": "10-K", "filed": "2022-09-09"},
                ]}
            },
            "PaymentsToAcquirePropertyPlantAndEquipment": {
                "units": {"USD": [
                    {"end": "2024-07-31", "val": 212000000, "form": "10-K", "filed": "2024-09-12"},
                    {"end": "2023-07-31", "val": 184000000, "form": "10-K", "filed": "2023-09-11"},
                    {"end": "2022-07-31", "val": 131000000, "form": "10-K", "filed": "2022-09-09"},
                ]}
            },
        }
    }
}


def test_import_company_returns_company_financials():
    with patch("financial_analysis.importer.get_cik_for_ticker", return_value="1327567"), \
         patch("financial_analysis.importer.fetch_company_facts", return_value=SAMPLE_FACTS):
        result = import_company("PANW")
    assert isinstance(result, CompanyFinancials)
    assert result.ticker == "PANW"
    assert result.cik == "1327567"


def test_import_company_populates_periods():
    with patch("financial_analysis.importer.get_cik_for_ticker", return_value="1327567"), \
         patch("financial_analysis.importer.fetch_company_facts", return_value=SAMPLE_FACTS):
        result = import_company("PANW", n_periods=8)
    assert len(result.periods) == 3  # only 3 in sample data


def test_import_company_revenue_populated():
    with patch("financial_analysis.importer.get_cik_for_ticker", return_value="1327567"), \
         patch("financial_analysis.importer.fetch_company_facts", return_value=SAMPLE_FACTS):
        result = import_company("PANW")
    latest = result.periods[-1]
    assert latest.revenue == 8030000000


def test_import_company_fcf_derived():
    with patch("financial_analysis.importer.get_cik_for_ticker", return_value="1327567"), \
         patch("financial_analysis.importer.fetch_company_facts", return_value=SAMPLE_FACTS):
        result = import_company("PANW")
    latest = result.periods[-1]
    assert latest.free_cash_flow == 3050000000 - 212000000
```

- [ ] **Step 2: Run tests — expect ImportError**

```bash
pytest tests/test_importer.py -v
```

- [ ] **Step 3: Implement importer.py**

`financial_analysis/importer.py`:
```python
from datetime import date, datetime
from typing import Dict, List, Optional

from financial_analysis.edgar import get_cik_for_ticker
from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.xbrl_api import extract_annual_series, fetch_company_facts

_CONCEPT_MAP = {
    "revenue": [
        "us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax",
        "us-gaap/Revenues",
    ],
    "gross_profit": ["us-gaap/GrossProfit"],
    "operating_income": ["us-gaap/OperatingIncomeLoss"],
    "net_income": ["us-gaap/NetIncomeLoss"],
    "rd_expense": ["us-gaap/ResearchAndDevelopmentExpense"],
    "sga_expense": ["us-gaap/SellingGeneralAndAdministrativeExpense"],
    "operating_cash_flow": ["us-gaap/NetCashProvidedByUsedInOperatingActivities"],
    "capex": ["us-gaap/PaymentsToAcquirePropertyPlantAndEquipment"],
    "total_assets": ["us-gaap/Assets"],
    "total_liabilities": ["us-gaap/Liabilities"],
    "equity": ["us-gaap/StockholdersEquity"],
    "shares_outstanding": ["us-gaap/CommonStockSharesOutstanding"],
}


def _first_series(facts: dict, concepts: List[str]) -> List[dict]:
    for concept in concepts:
        series = extract_annual_series(facts, concept)
        if series:
            return series
    return []


def import_company(
    ticker: str,
    form: str = "10-K",
    n_periods: int = 8,
    save: bool = True,
    data_dir: Path = Path("data/processed"),
) -> CompanyFinancials:
    cik = get_cik_for_ticker(ticker)
    facts = fetch_company_facts(cik)
    entity_name = facts.get("entityName", ticker)

    # Build a dict keyed by period_end date for each financial slot
    period_data: Dict[str, dict] = {}

    for slot, concepts in _CONCEPT_MAP.items():
        for entry in _first_series(facts, concepts):
            key = entry["end"]
            if key not in period_data:
                period_data[key] = {
                    "period_end": key,
                    "filed_date": entry.get("filed", key),
                    "form": entry.get("form", form),
                }
            period_data[key][slot] = float(entry["val"])

    def _to_date(s: str) -> date:
        return datetime.strptime(s, "%Y-%m-%d").date()

    periods = []
    for key in sorted(period_data.keys())[-n_periods:]:
        d = period_data[key]
        periods.append(PeriodFinancials(
            ticker=ticker.upper(),
            period_end=_to_date(d["period_end"]),
            form=d["form"],
            filed_date=_to_date(d["filed_date"]),
            revenue=d.get("revenue"),
            gross_profit=d.get("gross_profit"),
            operating_income=d.get("operating_income"),
            net_income=d.get("net_income"),
            rd_expense=d.get("rd_expense"),
            sga_expense=d.get("sga_expense"),
            operating_cash_flow=d.get("operating_cash_flow"),
            capex=d.get("capex"),
            total_assets=d.get("total_assets"),
            total_liabilities=d.get("total_liabilities"),
            equity=d.get("equity"),
            shares_outstanding=d.get("shares_outstanding"),
        ))

    company = CompanyFinancials(
        ticker=ticker.upper(),
        cik=cik,
        company_name=entity_name,
        periods=periods,
    )
    if save:
        from financial_analysis.store import save as store_save
        store_save(company, data_dir=data_dir)
    return company
```

- [ ] **Step 4: Create scripts/import_financials.py**

`scripts/import_financials.py`:
```python
import argparse
from financial_analysis.importer import import_company


def main() -> None:
    parser = argparse.ArgumentParser(description="Import financial statements via SEC XBRL API")
    parser.add_argument("ticker", help="Company ticker, e.g. PANW")
    parser.add_argument("--periods", type=int, default=8, help="Number of annual periods to fetch")
    args = parser.parse_args()

    company = import_company(args.ticker, n_periods=args.periods)
    print(f"\n{company.company_name} ({company.ticker})  CIK: {company.cik}\n")
    header = f"{'Period':<12} {'Revenue':>14} {'Op. Income':>14} {'OCF':>14} {'CapEx':>10} {'FCF':>14}"
    print(header)
    print("-" * len(header))

    def fmt(v):
        return f"${v/1e9:.2f}B" if v is not None else "N/A"

    for p in company.periods:
        print(f"{p.period_end!s:<12} {fmt(p.revenue):>14} {fmt(p.operating_income):>14} "
              f"{fmt(p.operating_cash_flow):>14} {fmt(p.capex):>10} {fmt(p.free_cash_flow):>14}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 5: Run unit tests**

```bash
pytest tests/test_importer.py -v
```

Expected: all green.

- [ ] **Step 6: Smoke test against live SEC API**

```bash
python -m scripts.import_financials PANW
```

Expected: table with 5+ rows, revenue in billions, FCF column non-zero. If any column shows all `N/A`, the concept mapping needs adjustment — check `extract_annual_series` against the live JSON directly.

- [ ] **Step 7: Commit**

```bash
git add financial_analysis/importer.py scripts/import_financials.py tests/test_importer.py
git commit -m "feat: add importer orchestrating CIK lookup + XBRL API into CompanyFinancials"
```

---

## Task 6: Implement dcf.py — Stochastic DCF with Monte Carlo (Ticket 5)

**Files:**
- Create: `financial_analysis/dcf.py`
- Create: `tests/test_dcf.py`

- [ ] **Step 1: Write failing tests**

`tests/test_dcf.py`:
```python
from datetime import date
import numpy as np
import pytest

from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.dcf import DCFAssumptions, DCFResult, StochasticDCF


def _make_company(n_periods: int = 5) -> CompanyFinancials:
    base_revenue = 5_000_000_000.0
    base_ocf = 1_500_000_000.0
    capex = 200_000_000.0
    periods = [
        PeriodFinancials(
            ticker="PANW",
            period_end=date(2019 + i, 7, 31),
            form="10-K",
            filed_date=date(2019 + i, 9, 12),
            revenue=base_revenue * (1.2 ** i),
            operating_cash_flow=base_ocf * (1.2 ** i),
            capex=capex,
        )
        for i in range(n_periods)
    ]
    return CompanyFinancials(
        ticker="PANW", cik="1327567",
        company_name="Palo Alto Networks Inc",
        periods=periods,
    )


def test_dcf_run_returns_dcf_result():
    result = StochasticDCF(_make_company(), DCFAssumptions(n_simulations=1000)).run()
    assert isinstance(result, DCFResult)


def test_dcf_percentile_ordering():
    result = StochasticDCF(_make_company(), DCFAssumptions(n_simulations=1000)).run()
    assert result.p10_value < result.median_value < result.p90_value


def test_dcf_all_values_positive():
    result = StochasticDCF(_make_company(), DCFAssumptions(n_simulations=1000)).run()
    assert result.p10_value > 0
    assert result.median_value > 0


def test_dcf_simulations_array_shape():
    n = 500
    result = StochasticDCF(_make_company(), DCFAssumptions(n_simulations=n)).run()
    assert result.simulations.shape == (n,)


def test_dcf_uses_explicit_fcf_margin_when_provided():
    assumptions = DCFAssumptions(
        n_simulations=1000,
        fcf_margin_mean=0.30,
        fcf_margin_std=0.02,
    )
    result = StochasticDCF(_make_company(), assumptions).run()
    assert result.median_value > 0


def test_dcf_derives_fcf_margin_from_history():
    company = _make_company(n_periods=5)
    assumptions = DCFAssumptions(n_simulations=1000)
    result = StochasticDCF(company, assumptions).run()
    assert result.median_value > 0


def test_dcf_no_fcf_history_raises():
    company = CompanyFinancials(
        ticker="X", cik="0", company_name="X",
        periods=[
            PeriodFinancials(ticker="X", period_end=date(2024, 12, 31), form="10-K", filed_date=date(2025, 2, 1))
        ],
    )
    assumptions = DCFAssumptions(n_simulations=100)
    with pytest.raises(ValueError, match="FCF"):
        StochasticDCF(company, assumptions).run()
```

- [ ] **Step 2: Run tests — expect ImportError**

```bash
pytest tests/test_dcf.py -v
```

- [ ] **Step 3: Implement dcf.py**

`financial_analysis/dcf.py`:
```python
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Optional
import numpy as np

from financial_analysis.schema import CompanyFinancials


@dataclass
class DCFAssumptions:
    n_simulations: int = 10_000
    projection_years: int = 10
    growth_rate_mean: float = 0.15
    growth_rate_std: float = 0.05
    terminal_growth_mean: float = 0.03
    terminal_growth_std: float = 0.005
    wacc_mean: float = 0.10
    wacc_std: float = 0.01
    fcf_margin_mean: Optional[float] = None
    fcf_margin_std: Optional[float] = None


@dataclass
class DCFResult:
    median_value: float
    p10_value: float
    p90_value: float
    mean_value: float
    std_value: float
    simulations: np.ndarray = field(repr=False)


class StochasticDCF:
    def __init__(self, company: CompanyFinancials, assumptions: DCFAssumptions) -> None:
        self.company = company
        self.assumptions = assumptions

    def _derive_fcf_margin(self) -> tuple[float, float]:
        margins = []
        for p in self.company.periods:
            fcf = p.free_cash_flow
            if fcf is not None and p.revenue and p.revenue > 0:
                margins.append(fcf / p.revenue)
        if not margins:
            raise ValueError("FCF margin cannot be derived: no periods with both FCF and revenue data")
        arr = np.array(margins)
        return float(arr.mean()), max(float(arr.std()), 0.01)

    def _base_revenue(self) -> float:
        for p in reversed(self.company.periods):
            if p.revenue is not None:
                return p.revenue
        raise ValueError("No revenue data available in CompanyFinancials")

    def run(self) -> DCFResult:
        a = self.assumptions
        n = a.n_simulations
        rng = np.random.default_rng()

        fcf_margin_mean = a.fcf_margin_mean
        fcf_margin_std = a.fcf_margin_std
        if fcf_margin_mean is None or fcf_margin_std is None:
            fcf_margin_mean, fcf_margin_std = self._derive_fcf_margin()

        base_revenue = self._base_revenue()

        # Sample distributions — shape (n,)
        growth_rates = rng.normal(a.growth_rate_mean, a.growth_rate_std, n)
        terminal_growth = rng.normal(a.terminal_growth_mean, a.terminal_growth_std, n)
        wacc = np.clip(rng.normal(a.wacc_mean, a.wacc_std, n), 0.01, None)
        fcf_margins = np.clip(rng.normal(fcf_margin_mean, fcf_margin_std, n), 0.0, 1.0)

        # Project FCF for each year — shape (projection_years, n)
        revenues = np.zeros((a.projection_years, n))
        revenues[0] = base_revenue * (1 + growth_rates)
        for yr in range(1, a.projection_years):
            revenues[yr] = revenues[yr - 1] * (1 + growth_rates)

        fcfs = revenues * fcf_margins  # (projection_years, n)

        # Discount each year's FCF back to present
        discount_factors = np.array([
            1 / (1 + wacc) ** (yr + 1)
            for yr in range(a.projection_years)
        ])  # (projection_years, n)

        pv_fcfs = (fcfs * discount_factors).sum(axis=0)  # (n,)

        # Terminal value: Gordon Growth Model
        terminal_fcf = fcfs[-1] * (1 + terminal_growth)
        terminal_value = terminal_fcf / np.clip(wacc - terminal_growth, 0.001, None)
        pv_terminal = terminal_value / (1 + wacc) ** a.projection_years

        total_values = pv_fcfs + pv_terminal

        return DCFResult(
            median_value=float(np.percentile(total_values, 50)),
            p10_value=float(np.percentile(total_values, 10)),
            p90_value=float(np.percentile(total_values, 90)),
            mean_value=float(total_values.mean()),
            std_value=float(total_values.std()),
            simulations=total_values,
        )
```

- [ ] **Step 4: Run tests**

```bash
pytest tests/test_dcf.py -v
```

Expected: all green.

- [ ] **Step 5: Smoke test with live PANW data**

```bash
python -c "
from pathlib import Path
from financial_analysis.store import load
from financial_analysis.dcf import DCFAssumptions, StochasticDCF

# Loads from data/processed/PANW.json — no SEC API call needed
company = load('PANW', data_dir=Path('data/processed'))
result = StochasticDCF(company, DCFAssumptions()).run()
shares = company.periods[-1].shares_outstanding
print(f'P10: \${result.p10_value/1e9:.1f}B')
print(f'P50: \${result.median_value/1e9:.1f}B')
print(f'P90: \${result.p90_value/1e9:.1f}B')
if shares:
    print(f'P50/share: \${result.median_value/shares:.2f}')
"
```

Expected: three dollar amounts in plausible range for PANW market cap. No network activity should occur.

- [ ] **Step 6: Commit**

```bash
git add financial_analysis/dcf.py tests/test_dcf.py
git commit -m "feat: add stochastic DCF with Monte Carlo simulation (P10/P50/P90)"
```

---

## Task 7: Run full test suite and update README (Ticket 6)

**Files:**
- Modify: `README.md`
- Modify: `context.json`

- [ ] **Step 1: Run full test suite**

```bash
pytest tests/ -v
```

Expected: all tests green. Fix any failures before continuing.

- [ ] **Step 2: Update README.md**

Replace the Current status section with the full pipeline description, usage examples for both scripts, and smoke test commands for each stage. See `PLANNING/tickets.md` Ticket 6 acceptance criteria.

- [ ] **Step 3: Update context.json**

Add `"stochastic_direction": "Monte Carlo DCF via numpy; future: options valuation"` and `"xbrl_path": "SEC XBRL JSON API (data.sec.gov/api/xbrl/companyfacts/)"` to the JSON.

- [ ] **Step 4: Final commit**

```bash
git add README.md context.json
git commit -m "docs: update README and context for stochastic DCF pipeline"
```
