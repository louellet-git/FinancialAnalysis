# FinancialAnalysis

Python pipeline that pulls public-company financials from SEC EDGAR, enriches them with market data, and runs a stochastic Monte Carlo DCF valuation.

## What it does

```
scripts/analyze.py TICKER
    ├── SEC EDGAR XBRL API  →  PeriodFinancials[]     (income stmt / balance sheet / cash flow)
    ├── yfinance            →  PricePoint[]           (daily OHLCV, full history)
    ├── yfinance            →  EarningsSurprise[]     (EPS estimate vs actual, ~25 quarters)
    └── yfinance            →  OptionChainSnapshot    (full option chain + derived metrics)
                ↓
    data/processed/TICKER_financials.xlsx  (8 tabs)
```

All remote data is disk-cached under `data/raw/TICKER/` — re-runs are instant.

## Quick start

```bash
pip install -r requirements.txt

# Export NVDA financials + market data to xlsx
python -m scripts.analyze NVDA

# Skip market data (SEC data only, faster)
python -m scripts.analyze NVDA --no-market

# Skip the option-chain snapshot (keep prices and earnings)
python -m scripts.analyze NVDA --no-options

# Force re-fetch everything from remote APIs
python -m scripts.analyze NVDA --refresh

# Import only (no xlsx export)
python -m scripts.import_financials PANW

# Run a Monte Carlo DCF on already-imported data (no live API call)
python -m scripts.run_dcf PANW --seed 7
```

Output: `data/processed/TICKER_financials.xlsx`

## Stochastic DCF

`financial_analysis/dcf.py` runs a vectorized Monte Carlo DCF. It samples revenue
growth, FCF margin, WACC, and terminal growth from normal distributions, projects
free cash flow over a 10-year horizon, adds a Gordon-growth terminal value, and
discounts everything to present value across (by default) 10,000 simulations.

```python
from financial_analysis import store
from financial_analysis.dcf import DCFAssumptions, StochasticDCF

company = store.load("PANW")              # from data/processed/PANW.json — no SEC call
result = StochasticDCF(company, DCFAssumptions(seed=7)).run()
print(result.median_value, result.p10_value, result.p90_value)
print(result.median_per_share)            # None if shares outstanding unavailable
```

- **FCF margin** is derived from the last 5 annual periods when not supplied explicitly.
- Only annual (10-K) periods feed the model; quarterly rows are ignored.
- **Minimum-history guard:** raises `ValueError` unless at least 4 annual periods
  have positive FCF. This blocks meaningless valuations of recently-IPO'd or
  loss-making companies (e.g. SpaceX, which has no 10-K on file yet).

## xlsx structure

| Tab | Contents |
|-----|----------|
| Income Statement | Revenue, gross profit, operating income, net income, EPS, SBC, D&A, … |
| Balance Sheet | Cash, receivables, assets, liabilities, equity, shares outstanding, … |
| Cash Flow | Operating CF, CapEx, FCF, financing flows, … |
| Raw | All other unmapped XBRL concepts (100–200 rows per period) |
| Prices | Daily OHLCV from yfinance (~6,800 rows for NVDA) |
| Estimates | Earnings surprises — actual vs estimated EPS per quarter |
| Options | Full option chain, one row per contract (strike, bid/ask, volume, OI, IV, ITM flag) |
| Options Summary | Per-expiry derived metrics — ATM IV, implied move (ATM straddle ÷ spot), 90/110 skew, put/call ratios — plus a chain-wide `ALL` row |

All financial tabs use a long format: one row per `(period, metric)`.

## Data sources

| Data | Source | Auth |
|------|--------|------|
| Financial statements | SEC EDGAR XBRL JSON API | None (public) |
| Daily stock prices | yfinance | None |
| Earnings surprises | yfinance | None |
| Option chains (current snapshot only) | yfinance | None |
| Option chain history (backfill) | FMP / Polygon (optional, requires paid plan) | API key in `.env` |
| Analyst estimates (forward) | FMP API (optional, requires paid plan) | `FMP_API_KEY` in `.env` |

### Option-chain history

yfinance only serves the **current** option chain — there is no free historical
endpoint. The pipeline therefore persists every pull as a dated snapshot at
`data/raw/TICKER/options/YYYY-MM-DD.json` (one per calendar day; a same-day
re-run reads from disk, `--refresh` overwrites). History accumulates from the
first time you run a ticker. Use `financial_analysis.options.list_snapshots(ticker, cache_dir)`
and `load_snapshot(ticker, cache_dir, snapshot_date=None)` (None → latest) to
query accumulated snapshots. If backfilled history is required, FMP or Polygon
paid plans are the upgrade path.

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
    options.py      — yfinance option-chain snapshots + derived metrics (ATM IV, implied move, skew, P/C)
    schema.py       — PeriodFinancials, CompanyFinancials, PricePoint, EarningsSurprise, OptionQuote, OptionChainSnapshot
    store.py        — JSON persistence and in-memory session cache
    xbrl_api.py     — SEC XBRL JSON API client
    xbrl_tags.py    — XBRL concept registry (TagSpec, sign_flip, fallbacks)

    dcf.py          — Monte Carlo stochastic DCF (P10/P50/P90, per-share)

scripts/
    analyze.py          — full pipeline → xlsx (primary entrypoint)
    import_financials.py — import only, no xlsx
    run_dcf.py          — Monte Carlo DCF on imported data

data/
    raw/TICKER/         — remote data cache (gitignored)
    raw/TICKER/options/ — dated option-chain snapshots, YYYY-MM-DD.json (gitignored)
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
- [x] Ticket 5 — Stochastic DCF scaffold (Monte Carlo, P10/P50/P90)
- [x] Ticket 6 — README update *(this file)*
- [ ] Future — options valuation (Black-Scholes, real options on growth); more cybersecurity tickers
