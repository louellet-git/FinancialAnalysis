# FinancialAnalysis

Python pipeline that pulls public-company financials from SEC EDGAR, enriches them with market data, and prepares the groundwork for a stochastic Monte Carlo DCF model.

## What it does

```
scripts/analyze.py TICKER
    ├── SEC EDGAR XBRL API  →  PeriodFinancials[]   (income stmt / balance sheet / cash flow)
    ├── yfinance            →  PricePoint[]          (daily OHLCV, full history)
    └── yfinance            →  EarningsSurprise[]    (EPS estimate vs actual, ~25 quarters)
                ↓
    data/processed/TICKER_financials.xlsx  (6 tabs)
```

All remote data is disk-cached under `data/raw/TICKER/` — re-runs are instant.

## Quick start

```bash
pip install -r requirements.txt

# Export NVDA financials + market data to xlsx
python -m scripts.analyze NVDA

# Skip market data (SEC data only, faster)
python -m scripts.analyze NVDA --no-market

# Force re-fetch everything from remote APIs
python -m scripts.analyze NVDA --refresh

# Import only (no xlsx export)
python -m scripts.import_financials PANW
```

Output: `data/processed/TICKER_financials.xlsx`

## xlsx structure

| Tab | Contents |
|-----|----------|
| Income Statement | Revenue, gross profit, operating income, net income, EPS, SBC, D&A, … |
| Balance Sheet | Cash, receivables, assets, liabilities, equity, shares outstanding, … |
| Cash Flow | Operating CF, CapEx, FCF, financing flows, … |
| Raw | All other unmapped XBRL concepts (100–200 rows per period) |
| Prices | Daily OHLCV from yfinance (~6,800 rows for NVDA) |
| Estimates | Earnings surprises — actual vs estimated EPS per quarter |

All financial tabs use a long format: one row per `(period, metric)`.

## Data sources

| Data | Source | Auth |
|------|--------|------|
| Financial statements | SEC EDGAR XBRL JSON API | None (public) |
| Daily stock prices | yfinance | None |
| Earnings surprises | yfinance | None |
| Analyst estimates (forward) | FMP API (optional, requires paid plan) | `FMP_API_KEY` in `.env` |

## Environment

Create `.env` in the project root if using FMP:

```
FMP_API_KEY=your_key_here
```

The `.env` file is gitignored and never committed.

## Supported tickers

Any US-GAAP filer on SEC EDGAR. Examples verified: NVDA, PANW, WMT, TGT, DUOL.

ETFs (e.g. CIBR) and IFRS filers (e.g. SPOT) are not yet supported — the pipeline raises a clear error rather than silently producing bad data.

## Project layout

```
financial_analysis/
    cache.py        — SEC EDGAR disk cache
    edgar.py        — CIK lookup and filing downloads
    fmp.py          — FMP API client (analyst estimates, IFRS stub)
    importer.py     — XBRL → PeriodFinancials orchestration
    market_data.py  — yfinance price and earnings loader
    schema.py       — PeriodFinancials, CompanyFinancials, PricePoint, EarningsSurprise
    store.py        — JSON persistence and in-memory session cache
    xbrl_api.py     — SEC XBRL JSON API client
    xbrl_tags.py    — XBRL concept registry (TagSpec, sign_flip, fallbacks)

scripts/
    analyze.py          — full pipeline → xlsx (primary entrypoint)
    import_financials.py — import only, no xlsx

data/
    raw/TICKER/         — remote data cache (gitignored)
    processed/          — JSON and xlsx outputs (gitignored)

PLANNING/tickets.md     — all tickets with acceptance criteria and smoke test results
context.json            — project summary for agent orientation
```

## Roadmap

- [x] Ticket 1 — SEC EDGAR filing retrieval
- [x] Ticket 2 — XBRL JSON API client
- [x] Ticket 3 — Time-series financial schema
- [x] Ticket 3.5 — JSON data store with query helpers
- [x] Ticket 4 — Financial statement importer (33 metrics, annual + quarterly)
- [x] Ticket 4.5 — xlsx export (4-tab long-format)
- [x] Ticket 4.6 — Market data enrichment (prices + earnings surprises)
- [ ] Ticket 5 — Stochastic DCF scaffold (Monte Carlo, P10/P50/P90)
- [ ] Ticket 6 — README update *(this file)*
