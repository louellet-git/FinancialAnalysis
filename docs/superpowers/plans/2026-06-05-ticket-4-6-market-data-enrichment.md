---
id: DOC-PLAN-MARKET-DATA-ENRICHMENT
type: plan
item: 10.16 Development plan
status: superseded
date: 2026-06-05
scope: Historical implementation plan for the price and estimates enrichment; superseded by docs/ARCHITECTURE.md.
---
# Ticket 4.6 — Market Data Enrichment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add historical stock prices (yfinance) and analyst earnings forecasts/surprises (FMP API) to the existing pipeline, with two new xlsx tabs and a disk-backed cache matching the existing pattern.

**Architecture:** Three new/modified modules — `market_data.py` (yfinance), `fmp.py` (FMP REST client), schema additions; `analyze.py` integrates both and writes Prices + Estimates tabs.

**Tech Stack:** Python 3.12, yfinance, python-dotenv, openpyxl (existing), dataclasses (existing)

**SECURITY:** FMP API key is stored only in `.env` (gitignored). Never print, log, or commit it.

---

### Task 1: Schema additions

**Files:**
- Modify: `financial_analysis/schema.py`
- Test: `tests/test_schema_market.py` (new)

Context: `schema.py` currently has `PeriodFinancials` and `CompanyFinancials`. We're adding three new dataclasses and three new list fields on `CompanyFinancials`. All imports needed: `from typing import Optional, List`; `from datetime import date`; `from dataclasses import dataclass, field` — all already present.

- [ ] **Step 1: Write the failing test**

Create `tests/test_schema_market.py`:

```python
from datetime import date
from financial_analysis.schema import (
    PricePoint, EarningsEstimate, EarningsSurprise, CompanyFinancials,
)


def test_price_point_fields():
    p = PricePoint(date=date(2024, 1, 2), open=100.0, high=105.0, low=99.0, close=103.0, volume=1_000_000)
    assert p.close == 103.0
    assert p.volume == 1_000_000


def test_earnings_estimate_optional_fields():
    e = EarningsEstimate(period_end=date(2024, 3, 31), period_type="quarterly")
    assert e.estimated_eps is None
    assert e.number_of_analysts is None


def test_earnings_surprise_fields():
    s = EarningsSurprise(
        date=date(2024, 4, 24),
        actual_eps=2.50,
        estimated_eps=2.30,
        surprise=0.20,
        surprise_pct=0.087,
    )
    assert s.surprise == 0.20


def test_company_financials_market_defaults():
    c = CompanyFinancials(ticker="NVDA", cik="1045810", company_name="NVIDIA")
    assert c.prices == []
    assert c.estimates == []
    assert c.surprises == []
```

- [ ] **Step 2: Run test to verify it fails**

```
pytest tests/test_schema_market.py -v
```
Expected: ImportError or AttributeError — `PricePoint` not defined yet.

- [ ] **Step 3: Add new dataclasses and CompanyFinancials fields to schema.py**

In `financial_analysis/schema.py`, after the existing imports, add before `PeriodFinancials`:

```python
@dataclass
class PricePoint:
    date: date
    open: float
    high: float
    low: float
    close: float
    volume: int


@dataclass
class EarningsEstimate:
    period_end: date
    period_type: str  # "annual" or "quarterly"
    estimated_eps: Optional[float] = None
    estimated_revenue: Optional[float] = None
    number_of_analysts: Optional[int] = None


@dataclass
class EarningsSurprise:
    date: date
    actual_eps: float
    estimated_eps: float
    surprise: float
    surprise_pct: float
```

Then extend `CompanyFinancials` by adding these three fields after `periods`:

```python
    prices: List[PricePoint] = field(default_factory=list)
    estimates: List[EarningsEstimate] = field(default_factory=list)
    surprises: List[EarningsSurprise] = field(default_factory=list)
```

- [ ] **Step 4: Run test to verify it passes**

```
pytest tests/test_schema_market.py -v
```
Expected: 4 tests PASS.

- [ ] **Step 5: Run full test suite to check for regressions**

```
pytest tests/ -v
```
Expected: all existing tests still pass.

- [ ] **Step 6: Commit**

```bash
git add financial_analysis/schema.py tests/test_schema_market.py
git commit -m "feat: add PricePoint, EarningsEstimate, EarningsSurprise to schema (Ticket 4.6)"
```

---

### Task 2: market_data.py — yfinance price loader

**Files:**
- Create: `financial_analysis/market_data.py`
- Test: `tests/test_market_data.py` (new)

Context: Mirrors the pattern in `financial_analysis/cache.py` — check disk cache first, fetch if missing or refresh=True, write cache as JSON, return typed objects. yfinance's `download()` returns a pandas DataFrame with a MultiIndex if `group_by='column'`, or a simple index. Use `yf.Ticker(ticker).history(period="max")` — simpler API, single ticker, returns DatetimeIndex with columns Open/High/Low/Close/Volume. The cache file is `data/raw/TICKER/prices.json`.

- [ ] **Step 1: Write failing tests**

Create `tests/test_market_data.py`:

```python
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
    # Cache was written
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
```

- [ ] **Step 2: Run tests to verify they fail**

```
pytest tests/test_market_data.py -v
```
Expected: ModuleNotFoundError — `financial_analysis.market_data` not found.

- [ ] **Step 3: Create financial_analysis/market_data.py**

```python
from __future__ import annotations
import json
import logging
from datetime import date
from pathlib import Path

import yfinance as yf

from financial_analysis.schema import PricePoint

logger = logging.getLogger(__name__)


def load_prices(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> list[PricePoint]:
    path = cache_dir / ticker.upper() / "prices.json"

    if not refresh and path.exists():
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
            return [
                PricePoint(
                    date=date.fromisoformat(r["date"]),
                    open=r["open"],
                    high=r["high"],
                    low=r["low"],
                    close=r["close"],
                    volume=int(r["volume"]),
                )
                for r in raw
            ]
        except (json.JSONDecodeError, KeyError):
            logger.warning("Corrupt price cache for %s — re-fetching", ticker)

    t = yf.Ticker(ticker.upper())
    df = t.history(period="max")

    if df.empty:
        logger.warning("yfinance returned empty DataFrame for %s", ticker)
        return []

    records = []
    for ts, row in df.iterrows():
        records.append({
            "date": ts.strftime("%Y-%m-%d"),
            "open": round(float(row["Open"]), 6),
            "high": round(float(row["High"]), 6),
            "low": round(float(row["Low"]), 6),
            "close": round(float(row["Close"]), 6),
            "volume": int(row["Volume"]),
        })

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(records, indent=2), encoding="utf-8")

    return [
        PricePoint(
            date=date.fromisoformat(r["date"]),
            open=r["open"],
            high=r["high"],
            low=r["low"],
            close=r["close"],
            volume=r["volume"],
        )
        for r in records
    ]
```

- [ ] **Step 4: Run tests to verify they pass**

```
pytest tests/test_market_data.py -v
```
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add financial_analysis/market_data.py tests/test_market_data.py
git commit -m "feat: add market_data.py yfinance price loader with disk cache (Ticket 4.6)"
```

---

### Task 3: fmp.py — FMP API client + IFRS stub

**Files:**
- Create: `financial_analysis/fmp.py`
- Test: `tests/test_fmp.py` (new)

Context: FMP base URL is `https://financialmodelingprep.com/api/v3`. Endpoints: `/analyst-estimates/{ticker}` returns list of objects with `date` (fiscal period end), `estimatedEpsAvg`, `estimatedRevenueAvg`, `numberAnalystEstimatedEps`. `/earnings-surprises/{ticker}` returns list with `date` (announcement date), `actualEarningResult`, `estimatedEarning`. API key comes from `.env` via `python-dotenv`. Load once at module level using `dotenv.load_dotenv()`. Cache files: `data/raw/TICKER/estimates.json` and `data/raw/TICKER/surprises.json`.

`FMPClientError` is a custom exception for HTTP errors and missing API key.

The analyst-estimates endpoint returns periods going forward (future estimates) and backward. The `date` field format is `"YYYY-MM-DD"`. The `period` field on each record is `"annual"` or `"quarter"` — map `"quarter"` → `"quarterly"`.

- [ ] **Step 1: Write failing tests**

Create `tests/test_fmp.py`:

```python
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
```

- [ ] **Step 2: Run tests to verify they fail**

```
pytest tests/test_fmp.py -v
```
Expected: ModuleNotFoundError — `financial_analysis.fmp` not found.

- [ ] **Step 3: Create financial_analysis/fmp.py**

```python
from __future__ import annotations
import json
import logging
import os
from datetime import date
from pathlib import Path

import requests
from dotenv import load_dotenv

from financial_analysis.schema import (
    CompanyFinancials,
    EarningsEstimate,
    EarningsSurprise,
)

load_dotenv()

logger = logging.getLogger(__name__)
_BASE = "https://financialmodelingprep.com/api/v3"


class FMPClientError(Exception):
    pass


def _fmp_get(endpoint: str) -> list[dict]:
    api_key = os.environ.get("FMP_API_KEY")
    if not api_key:
        raise FMPClientError("FMP_API_KEY not set in environment — add it to .env")
    url = f"{_BASE}/{endpoint}"
    resp = requests.get(url, params={"apikey": api_key}, timeout=30)
    if resp.status_code != 200:
        raise FMPClientError(f"FMP API returned HTTP {resp.status_code} for {url}")
    return resp.json()


def load_estimates(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> list[EarningsEstimate]:
    path = cache_dir / ticker.upper() / "estimates.json"

    if not refresh and path.exists():
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
            return _parse_estimates(raw)
        except (json.JSONDecodeError, KeyError):
            logger.warning("Corrupt estimates cache for %s — re-fetching", ticker)

    raw = _fmp_get(f"analyst-estimates/{ticker.upper()}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(raw, indent=2), encoding="utf-8")
    return _parse_estimates(raw)


def _parse_estimates(raw: list[dict]) -> list[EarningsEstimate]:
    out = []
    for r in raw:
        period_str = r.get("period", "")
        period_type = "quarterly" if period_str == "quarter" else "annual"
        try:
            out.append(EarningsEstimate(
                period_end=date.fromisoformat(r["date"]),
                period_type=period_type,
                estimated_eps=r.get("estimatedEpsAvg"),
                estimated_revenue=r.get("estimatedRevenueAvg"),
                number_of_analysts=r.get("numberAnalystEstimatedEps"),
            ))
        except (KeyError, ValueError) as exc:
            logger.warning("Skipping malformed estimate record: %s", exc)
    return out


def load_surprises(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> list[EarningsSurprise]:
    path = cache_dir / ticker.upper() / "surprises.json"

    if not refresh and path.exists():
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
            return _parse_surprises(raw)
        except (json.JSONDecodeError, KeyError):
            logger.warning("Corrupt surprises cache for %s — re-fetching", ticker)

    raw = _fmp_get(f"earnings-surprises/{ticker.upper()}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(raw, indent=2), encoding="utf-8")
    return _parse_surprises(raw)


def _parse_surprises(raw: list[dict]) -> list[EarningsSurprise]:
    out = []
    for r in raw:
        try:
            actual = float(r["actualEarningResult"])
            estimated = float(r["estimatedEarning"])
            surprise = actual - estimated
            surprise_pct = surprise / abs(estimated) if estimated != 0 else 0.0
            out.append(EarningsSurprise(
                date=date.fromisoformat(r["date"]),
                actual_eps=actual,
                estimated_eps=estimated,
                surprise=round(surprise, 6),
                surprise_pct=round(surprise_pct, 6),
            ))
        except (KeyError, ValueError, ZeroDivisionError) as exc:
            logger.warning("Skipping malformed surprise record: %s", exc)
    return out


def load_fmp_statements(ticker: str) -> CompanyFinancials:
    raise NotImplementedError(
        f"IFRS via FMP not yet implemented for {ticker}. "
        "This stub exists to wire the fallback path for future development."
    )
```

- [ ] **Step 4: Run tests to verify they pass**

```
pytest tests/test_fmp.py -v
```
Expected: 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add financial_analysis/fmp.py tests/test_fmp.py
git commit -m "feat: add fmp.py FMP API client with disk cache and IFRS stub (Ticket 4.6)"
```

---

### Task 4: Update requirements.txt

**Files:**
- Modify: `requirements.txt`

Context: `requirements.txt` currently has `numpy>=1.26` and `openpyxl>=3.1`. Adding yfinance and python-dotenv.

- [ ] **Step 1: Update requirements.txt**

```
numpy>=1.26
openpyxl>=3.1
yfinance>=0.2
python-dotenv>=1.0
requests>=2.31
```

Note: `requests` is added explicitly since `fmp.py` uses it directly. yfinance already pulls it transitively, but explicit is better.

- [ ] **Step 2: Install new dependencies**

```
pip install yfinance>=0.2 python-dotenv>=1.0 requests>=2.31
```

- [ ] **Step 3: Run full test suite to confirm nothing broke**

```
pytest tests/ -v
```
Expected: all tests PASS.

- [ ] **Step 4: Commit**

```bash
git add requirements.txt
git commit -m "chore: add yfinance, python-dotenv, requests to requirements (Ticket 4.6)"
```

---

### Task 5: Update scripts/analyze.py — market data integration

**Files:**
- Modify: `scripts/analyze.py`

Context: `analyze.py` currently imports company via `importer.import_company`, loads from store, and writes 4-tab xlsx. We're adding:
1. Import `load_prices` from `market_data`, `load_estimates` + `load_surprises` from `fmp`
2. After store load, attach market data to `company`
3. `--no-market` flag skips market data (graceful degradation)
4. Two new tabs in `_flatten`: Prices (columns: ticker, date, open, high, low, close, volume) and Estimates (long-format rows for each metric)
5. IFRS fallback: catch `UnsupportedFilingError`, try `load_fmp_statements`, handle `NotImplementedError`
6. Update output message from "4 tabs" to "6 tabs" (or "4 tabs" when `--no-market`)

The Prices tab columns: `ticker, date, open, high, low, close, volume` (7 columns, different from the financial COLUMNS constant).

The Estimates tab uses a long format similar to the financial tabs: `ticker, period_end_or_date, period_type, metric, value, unit`. Rows come from both `company.estimates` and `company.surprises`.

Write these as separate column lists since they differ from the main COLUMNS.

- [ ] **Step 1: Add imports and new column constants**

At the top of `scripts/analyze.py`, add these imports after the existing ones:

```python
from financial_analysis.market_data import load_prices
from financial_analysis.fmp import FMPClientError, load_estimates, load_surprises, load_fmp_statements
```

Add new column constants after `_DERIVED_METRICS`:

```python
PRICE_COLUMNS = ["ticker", "date", "open", "high", "low", "close", "volume"]
ESTIMATE_COLUMNS = ["ticker", "period_end", "period_type", "metric", "value", "unit"]
```

- [ ] **Step 2: Update _flatten to include Prices and Estimates tabs**

Replace the `_flatten` function:

```python
def _flatten(company: CompanyFinancials) -> dict[str, list[dict[str, Any]]]:
    tabs: dict[str, list[dict]] = {
        "Income Statement": [],
        "Balance Sheet": [],
        "Cash Flow": [],
        "Raw": [],
        "Prices": [],
        "Estimates": [],
    }

    for p in company.periods:
        base = {
            "ticker": p.ticker,
            "cik": company.cik,
            "company_name": company.company_name,
            "period_end": p.period_end.isoformat(),
            "form_type": p.form,
            "filed_date": p.filed_date.isoformat(),
        }

        for metric, statement in STATEMENT_MAP.items():
            if metric in _DERIVED_METRICS:
                continue
            val = getattr(p, metric, None)
            if val is None:
                continue
            tabs[_TAB_FOR[statement]].append({
                **base,
                "statement": statement,
                "metric": metric,
                "value": val,
                "unit": _unit_for(metric),
            })

        for metric, statement in _DERIVED_METRICS.items():
            val = getattr(p, metric, None)
            if val is not None:
                tabs[_TAB_FOR[statement]].append({
                    **base,
                    "statement": statement,
                    "metric": metric,
                    "value": val,
                    "unit": _unit_for(metric),
                })

        for concept, val in (p.raw_extras or {}).items():
            if val is not None:
                tabs["Raw"].append({
                    **base,
                    "statement": "raw",
                    "metric": concept,
                    "value": val,
                    "unit": "USD",
                })

    # Prices tab
    for pp in company.prices:
        tabs["Prices"].append({
            "ticker": company.ticker,
            "date": pp.date.isoformat(),
            "open": pp.open,
            "high": pp.high,
            "low": pp.low,
            "close": pp.close,
            "volume": pp.volume,
        })

    # Estimates tab — long format
    for est in company.estimates:
        for metric, val in [
            ("estimated_eps", est.estimated_eps),
            ("estimated_revenue", est.estimated_revenue),
        ]:
            if val is not None:
                tabs["Estimates"].append({
                    "ticker": company.ticker,
                    "period_end": est.period_end.isoformat(),
                    "period_type": est.period_type,
                    "metric": metric,
                    "value": val,
                    "unit": "USD/share" if "eps" in metric else "USD",
                })
        if est.number_of_analysts is not None:
            tabs["Estimates"].append({
                "ticker": company.ticker,
                "period_end": est.period_end.isoformat(),
                "period_type": est.period_type,
                "metric": "number_of_analysts",
                "value": est.number_of_analysts,
                "unit": "count",
            })

    for surp in company.surprises:
        for metric, val in [
            ("actual_eps", surp.actual_eps),
            ("estimated_eps", surp.estimated_eps),
            ("surprise", surp.surprise),
            ("surprise_pct", surp.surprise_pct),
        ]:
            tabs["Estimates"].append({
                "ticker": company.ticker,
                "period_end": surp.date.isoformat(),
                "period_type": "surprise",
                "metric": metric,
                "value": val,
                "unit": "USD/share" if "eps" in metric or metric == "surprise" else "ratio",
            })

    return tabs
```

- [ ] **Step 3: Update write_xlsx to use correct columns per tab**

Replace `write_xlsx`:

```python
def write_xlsx(company: CompanyFinancials, out_path: Path) -> int:
    wb = openpyxl.Workbook()
    wb.remove(wb.active)

    tabs = _flatten(company)
    total_rows = 0

    tab_columns = {
        "Prices": PRICE_COLUMNS,
        "Estimates": ESTIMATE_COLUMNS,
    }

    for tab_name, rows in tabs.items():
        ws = wb.create_sheet(tab_name)
        cols = tab_columns.get(tab_name, COLUMNS)
        ws.append(cols)
        for row in rows:
            ws.append([row.get(c, "") for c in cols])
            total_rows += 1

    out_path.parent.mkdir(parents=True, exist_ok=True)
    wb.save(str(out_path))
    return total_rows
```

- [ ] **Step 4: Update main() to fetch market data and wire IFRS fallback**

Replace `main()`:

```python
def main() -> None:
    parser = argparse.ArgumentParser(description="Export financial statements to xlsx.")
    parser.add_argument("ticker", help="Stock ticker symbol (e.g. NVDA)")
    parser.add_argument("--refresh", action="store_true", help="Force re-fetch from SEC API")
    parser.add_argument("--out", type=Path, help="Output file path (default: data/processed/TICKER_financials.xlsx)")
    parser.add_argument("--no-market", action="store_true", help="Skip stock prices and analyst estimates")
    args = parser.parse_args()

    ticker = args.ticker.upper()
    out_path = args.out or Path(f"data/processed/{ticker}_financials.xlsx")
    cache_dir = Path("data/raw")

    print(f"Importing {ticker}...")
    try:
        import_company(ticker, save=True, refresh=args.refresh)
    except UnsupportedFilingError as e:
        print(f"XBRL import failed ({e}). Trying FMP IFRS fallback...")
        try:
            load_fmp_statements(ticker)
        except NotImplementedError:
            print("IFRS via FMP not yet implemented. Exiting.")
            sys.exit(1)
    except EdgarClientError as e:
        print(f"SEC API error: {e}")
        sys.exit(1)

    store.clear_cache()
    company = store.load(ticker)

    if not args.no_market:
        print("Fetching stock prices (yfinance)...")
        try:
            company.prices = load_prices(ticker, cache_dir=cache_dir, refresh=args.refresh)
            print(f"  {len(company.prices)} price points loaded")
        except Exception as e:
            print(f"  Warning: price fetch failed — {e}")

        print("Fetching analyst estimates (FMP)...")
        try:
            company.estimates = load_estimates(ticker, cache_dir=cache_dir, refresh=args.refresh)
            company.surprises = load_surprises(ticker, cache_dir=cache_dir, refresh=args.refresh)
            print(f"  {len(company.estimates)} estimate records, {len(company.surprises)} surprise records")
        except FMPClientError as e:
            print(f"  Warning: FMP fetch failed — {e}")

    n_tabs = 4 if args.no_market else 6
    print(f"Writing {out_path} ...")
    total = write_xlsx(company, out_path)
    print(f"Done: {out_path} ({total} rows across {n_tabs} tabs)")


if __name__ == "__main__":
    main()
```

- [ ] **Step 5: Run full test suite**

```
pytest tests/ -v
```
Expected: all tests PASS (analyze.py is not directly unit-tested — it will be validated by smoke test).

- [ ] **Step 6: Commit**

```bash
git add scripts/analyze.py
git commit -m "feat: add Prices/Estimates xlsx tabs and market data integration to analyze.py (Ticket 4.6)"
```

---

### Task 6: Smoke test and ticket close

**Goal:** Run the full pipeline live, verify 6-tab output, and close Ticket 4.6.

- [ ] **Step 1: Run the smoke test**

```
python -m scripts.analyze NVDA
```

Expected output (approximately):
```
Importing NVDA...
Fetching stock prices (yfinance)...
  5000+ price points loaded
Fetching analyst estimates (FMP)...
  N estimate records, M surprise records
Writing data/processed/NVDA_financials.xlsx ...
Done: data/processed/NVDA_financials.xlsx (XXXXX rows across 6 tabs)
```

- [ ] **Step 2: Verify cache hit on second run**

```
python -m scripts.analyze NVDA
```
Second run should be instant (no API calls). Print timestamps or note that it runs in under 1 second.

- [ ] **Step 3: Verify --no-market flag**

```
python -m scripts.analyze NVDA --no-market
```
Expected: "Done: ... (N rows across 4 tabs)" — same row count as before for the financial tabs.

- [ ] **Step 4: Open the xlsx and verify tabs**

Verify:
- 6 tabs exist: Income Statement, Balance Sheet, Cash Flow, Raw, Prices, Estimates
- Prices tab: 5000+ rows with OHLCV columns
- Estimates tab: analyst estimate and surprise rows for NVDA

- [ ] **Step 5: Update PLANNING/tickets.md**

Mark Ticket 4.6 as closed with smoke test results. Add a new ticket entry before Ticket 5:

```markdown
## Ticket 4.6 — Market Data Enrichment ✅ CLOSED
- **Status:** Closed 2026-06-05. Smoke test passed.
- **Results:**
  - [fill in actual counts from smoke test output]
```

- [ ] **Step 6: Commit**

```bash
git add PLANNING/tickets.md
git commit -m "docs: close Ticket 4.6 — market data enrichment smoke test passed"
```
