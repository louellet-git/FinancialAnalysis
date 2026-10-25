---
id: DOC-SPEC-MARKET-DATA-ENRICHMENT
type: description
item: 10.15 Design description
status: superseded
date: 2026-06-05
scope: Historical design of the price and estimates enrichment; superseded by docs/ARCHITECTURE.md and the code.
---
# Ticket 4.6 — Market Data Enrichment Design Spec

**Date:** 2026-06-05  
**Status:** Approved

---

## Goal

Add historical stock prices and analyst earnings forecasts (consensus estimates + actuals-vs-estimates) to the existing financial analysis pipeline, enriching `CompanyFinancials` without breaking existing XBRL import or xlsx export.

---

## Architecture

```
scripts/analyze.py TICKER
    ├── [existing] importer.py  →  XBRL  →  PeriodFinancials[]
    │       └── on UnsupportedFilingError  →  fmp.py IFRS stub (NotImplementedError)
    ├── [new] market_data.py   →  yfinance  →  PricePoint[]
    │       └── cache: data/raw/TICKER/prices.json
    └── [new] fmp.py           →  FMP API  →  EarningsEstimate[] + EarningsSurprise[]
            └── cache: data/raw/TICKER/estimates.json + surprises.json
```

`CompanyFinancials` grows three new optional lists. `analyze.py` grows two new tabs: Prices + Estimates. All data sources are disk-cached (forever, `--refresh` to force refetch).

---

## Data Sources

| Data | Source | Auth | Free limit |
|------|--------|------|-----------|
| Daily OHLCV prices | yfinance | None | Unlimited |
| Analyst EPS/revenue estimates | FMP `/analyst-estimates/{ticker}` | `FMP_API_KEY` from `.env` | 250 calls/day |
| Earnings surprises | FMP `/earnings-surprises/{ticker}` | same | counts toward 250 |

FMP API key lives in `.env` only — never committed to git.

---

## Schema Additions (`financial_analysis/schema.py`)

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
    period_type: str          # "annual" or "quarterly"
    estimated_eps: Optional[float] = None
    estimated_revenue: Optional[float] = None
    number_of_analysts: Optional[int] = None

@dataclass
class EarningsSurprise:
    date: date                # earnings announcement date
    actual_eps: float
    estimated_eps: float
    surprise: float           # actual - estimated
    surprise_pct: float       # (actual - estimated) / abs(estimated)
```

`CompanyFinancials` gets three new fields, all defaulting to empty list:
```python
prices: List[PricePoint] = field(default_factory=list)
estimates: List[EarningsEstimate] = field(default_factory=list)
surprises: List[EarningsSurprise] = field(default_factory=list)
```

---

## New Module: `financial_analysis/market_data.py`

```python
def load_prices(ticker: str, cache_dir: Path, refresh: bool = False) -> list[PricePoint]
```

- Reads `data/raw/TICKER/prices.json` if exists and `not refresh`
- Otherwise calls `yf.download(ticker, period="max", interval="1d", auto_adjust=True)`
- Writes cache as JSON list of `{date, open, high, low, close, volume}`
- Returns `PricePoint[]` sorted ascending by date

---

## New Module: `financial_analysis/fmp.py`

```python
def load_estimates(ticker: str, cache_dir: Path, refresh: bool = False) -> list[EarningsEstimate]
def load_surprises(ticker: str, cache_dir: Path, refresh: bool = False) -> list[EarningsSurprise]
def load_fmp_statements(ticker: str) -> CompanyFinancials  # IFRS stub — raises NotImplementedError
```

- FMP base URL: `https://financialmodelingprep.com/api/v3`
- API key loaded from `.env` via `python-dotenv` (`FMP_API_KEY`)
- Same disk-cache pattern as `cache.py`: read → parse → return if fresh, else fetch → write → return
- `_fmp_get(endpoint, params)` private helper handles auth + HTTP (raises `FMPClientError` on non-200)
- `load_fmp_statements` raises `NotImplementedError("IFRS via FMP not yet implemented")`

---

## Updated Module: `scripts/analyze.py`

Two new xlsx tabs:

**Prices tab** columns: `ticker, date, open, high, low, close, volume`  
**Estimates tab** columns: `ticker, period_end, period_type, metric, value, unit`  
- Rows for each `EarningsEstimate`: one row per metric (`estimated_eps`, `estimated_revenue`)
- Rows for each `EarningsSurprise`: metric = `actual_eps`, `estimated_eps`, `surprise`, `surprise_pct`

Output line updated: "Done: {path} (N rows across 6 tabs)"

`--no-market` flag skips market data fetch (useful for offline/CI environments).

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| yfinance returns empty DataFrame (delisted/bad ticker) | `load_prices` returns `[]`, no crash |
| FMP returns 401/403 | `FMPClientError` with message including HTTP status |
| FMP returns empty list | returns `[]`, no crash |
| `.env` missing `FMP_API_KEY` | `FMPClientError("FMP_API_KEY not set in environment")` |
| Ticker is IFRS filer | `UnsupportedFilingError` still raised; `fmp.load_fmp_statements` stub exists but raises `NotImplementedError` |

---

## IFRS Stub Wiring (future-proofing only)

`analyze.py` catches `UnsupportedFilingError`, attempts `fmp.load_fmp_statements(ticker)`, and handles `NotImplementedError` by printing "IFRS via FMP not yet supported" and exiting. This wires the fallback path without implementing it.

---

## Files Changed

| File | Change |
|------|--------|
| `financial_analysis/schema.py` | Add `PricePoint`, `EarningsEstimate`, `EarningsSurprise`; extend `CompanyFinancials` |
| `financial_analysis/market_data.py` | **New** — yfinance price loader with disk cache |
| `financial_analysis/fmp.py` | **New** — FMP API client + IFRS stub |
| `scripts/analyze.py` | Add market data fetch, Prices/Estimates tabs, `--no-market` flag, IFRS fallback |
| `requirements.txt` | Add `yfinance>=0.2`, `python-dotenv>=1.0` |
| `tests/test_market_data.py` | **New** — mocked yfinance tests |
| `tests/test_fmp.py` | **New** — mocked HTTP tests for FMP client |

---

## Smoke Test (Ticket Acceptance)

```bash
python -m scripts.analyze NVDA
```

Expected:
- "Done: data/processed/NVDA_financials.xlsx (N rows across 6 tabs)"
- Prices tab: 5000+ rows (20+ years daily OHLCV)
- Estimates tab: 20+ rows (analyst estimates and surprises)
- Re-run is instant (cache hit, no API calls)
- `--no-market` flag: runs without yfinance/FMP, produces 4-tab xlsx
