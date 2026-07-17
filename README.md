# FinancialAnalysis

Python pipeline that pulls public-company financials from SEC EDGAR, enriches them with
market data, and runs Monte Carlo DCF valuations — an aggregate engine for quick scans
and a driver-based engine for reviewed, publication-grade work. The repo is growing into
a reusable **valuation factory** (see `PLANNING/reusable-dcf-vision.md`).

New agent? Start with `CLAUDE.md` (normativity map + operating norms), then
`docs/ARCHITECTURE.md` and `docs/HIDDEN-ASSUMPTIONS.md`.

## What it does

```
scripts/analyze.py TICKER
    ├── SEC EDGAR XBRL API  →  PeriodFinancials[]     (income stmt / balance sheet / cash flow / disclosures)
    ├── yfinance            →  PricePoint[]           (daily OHLCV, full history)
    ├── yfinance            →  EarningsSurprise[]     (EPS estimate vs actual, ~25 quarters)
    ├── yfinance            →  ConsensusEstimates     (forward revenue/EPS consensus + price targets, dated snapshot)
    └── yfinance            →  OptionChainSnapshot    (full option chain + derived metrics, dated snapshot)
                ↓
    data/processed/TICKER_financials.xlsx  (9 tabs)
```

All remote data is disk-cached under `data/raw/TICKER/` — re-runs are instant.
Financial statements also persist to `data/processed/TICKER.json` (periods only —
prices/options/estimates stay in the raw cache and the xlsx, not the JSON store).

## Quick start

```bash
pip install -r requirements.txt

# Export IBM financials + market data to xlsx
python -m scripts.analyze IBM

# Skip market data (SEC data only, faster)
python -m scripts.analyze IBM --no-market

# Skip the option-chain snapshot (keep prices and earnings)
python -m scripts.analyze IBM --no-options

# Skip the forward consensus estimates snapshot
python -m scripts.analyze IBM --no-estimates

# Force re-fetch everything from remote APIs
python -m scripts.analyze IBM --refresh

# Import only (no xlsx export)
python -m scripts.import_financials PANW

# Run a Monte Carlo DCF on already-imported data (no live API call)
python -m scripts.run_dcf PANW --seed 7

# Interactive single-page company report → data/processed/IBM_dcf.html
python -m scripts.visualize_dcf IBM

# IBM v3 scenario console report (aggregate engine — see note below)
python -m analysis.IBM.dcf_scenarios

# IBM v4 driver-based valuation (normative headline, loci, disclosure runs)
python -m analysis.IBM.dcf_drivers_run

# Point-in-time vintage backtest (revenue/FCF projections vs realized actuals)
python -m scripts.backtest IBM --horizon 3
```

## Two DCF engines

**Aggregate — `financial_analysis/dcf.py` (`StochasticDCF`).** Vectorized Monte Carlo:
samples revenue growth, FCF margin, WACC, and terminal growth from normal distributions
(independent draws, one per path), projects FCF over a 10-year horizon, adds a
Gordon-growth terminal value, discounts across (by default) 10,000 simulations.

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

**Driver-based — `financial_analysis/driver_dcf.py` (`DriverDCF`).** Nothing top-level is
sampled: revenue, margin, WACC and terminal value are vetted deterministic formulas over
underlying *drivers*, and only drivers are stochastic — drawn jointly from a full
correlated distribution (Cholesky), one draw per path, with per-path CAPM WACC (kd
derived from the rf draw), a hard terminal guard `tg ≤ WACC − 2%`, and ±1σ per-driver
attribution at fixed seed. Per-ticker driver registers with mandatory written rationales
live in `analysis/<TICKER>/driver_model.py`. Full mechanics: `docs/ARCHITECTURE.md`.

## IBM (the frontier ticker)

The reviewed, normative IBM valuation is the **v4 driver run**
(`python -m analysis.IBM.dcf_drivers_run`, seed 7, 10k sims):

> **P10 $83.84 / P50 $151.84 / P90 $258.03** · TV/EV at means 64.1% ·
> **conditional on f = 0.50** — the M&A-recurrence judgment; the published f-locus spans
> P50 $197.79 (f=0) to $101.44 (f=1). Never quote the headline without the condition.

The v3 aggregate-engine record (Bear $37 / Base $141 / Bull $232, `analysis/IBM/README.md`)
is frozen history: a fresh `analysis.IBM.dcf_scenarios` run derives margins from the
*current* store and now prints slightly different numbers (see
`docs/HIDDEN-ASSUMPTIONS.md` #14). The findings ledger in `analysis/IBM/REVIEW.md` and
the driver dispositions in `analysis/IBM/DRIVER-REVIEW.md` **bind future work**.

## Interactive report

`python -m scripts.visualize_dcf TICKER` writes `data/processed/TICKER_dcf.html` — a
self-contained single-page report (plotly.js inlined, opens offline): price & valuation
overlay, fundamentals, the four aggregate Monte Carlo panels (fan, distribution,
waterfall, tornado), a driver-model section when `analysis/<TICKER>/driver_model.py`
exists, an options-market section when a snapshot exists, and a vintage-backtest section.
Sections degrade gracefully. `--price X` overrides the live last-close marker (omitted
with a note when offline).

The aggregate panels use the bear/base/bull presets in `financial_analysis/scenarios.py`;
per-ticker overrides (scenario labels, pinned net debt / share counts) live in its
registries. The IBM entries there are v3-era by design. Without an override, net debt is
derived from the latest 10-K (long-term debt + short-term borrowings − cash) and shares
from shares_outstanding (falling back to diluted) — a simpler definition than the
reviewed IBM bridge (trap #15).

## xlsx structure (9 tabs, always created; empty when skipped)

| Tab | Contents |
|-----|----------|
| Income Statement | Revenue, gross profit, operating income, net income, EPS, SBC, D&A, … |
| Balance Sheet | Cash, receivables, assets, liabilities, equity, shares outstanding, … + note disclosures (RPO) |
| Cash Flow | Operating CF, CapEx, FCF, dividends, buybacks, financing flows, … |
| Raw | All other unmapped XBRL concepts (100–200 rows per period) |
| Prices | Daily OHLCV from yfinance |
| Estimates | Earnings surprises — actual vs estimated EPS per quarter |
| Estimates Fwd | Forward consensus snapshot — revenue/EPS estimates per period (0q/+1q/0y/+1y) + analyst price targets |
| Options | Full option chain, one row per contract (strike, bid/ask, volume, OI, IV, ITM flag) |
| Options Summary | Per-expiry derived metrics — ATM IV, implied move (ATM straddle ÷ spot), 90/110 skew, put/call ratios — plus a chain-wide `ALL` row |

All financial tabs use a long format: one row per `(period, metric)`.

## Data sources

| Data | Source | Auth |
|------|--------|------|
| Financial statements | SEC EDGAR XBRL JSON API | None (public) |
| Daily stock prices | yfinance | None |
| Earnings surprises | yfinance | None |
| Forward consensus estimates (current snapshot only) | yfinance | None |
| Option chains (current snapshot only) | yfinance | None |
| Option chain history (backfill) | FMP / Polygon (optional, requires paid plan) | API key in `.env` |
| Analyst estimates (FMP path) | FMP API (optional, requires paid plan) | `FMP_API_KEY` in `.env` |

### Snapshot history (options and forward estimates)

yfinance only serves the **current** option chain and consensus — there is no free
historical endpoint. The pipeline therefore persists every pull as a dated snapshot
(`data/raw/TICKER/options/YYYY-MM-DD.json`, `data/raw/TICKER/estimates/YYYY-MM-DD.json`;
one per calendar day, same-day re-runs read from disk, `--refresh` overwrites). History
accumulates from the first time you run a ticker. Query with
`financial_analysis.options.list_snapshots` / `load_snapshot` (None → latest). If
backfilled history is required, FMP or Polygon paid plans are the upgrade path.

## Environment

Create `.env` in the project root if using FMP:

```
FMP_API_KEY=your_key_here
```

The `.env` file is gitignored and never committed.

## Supported tickers

Any US-GAAP filer on SEC EDGAR. Examples verified: NVDA, PANW, WMT, TGT, DUOL, IBM,
MSFT, AAPL.

ETFs (e.g. CIBR) and IFRS filers (e.g. SPOT) are not yet supported — the pipeline raises
a clear error rather than silently producing bad data.

## Project layout

```
financial_analysis/
    cache.py        — SEC EDGAR disk cache
    edgar.py        — CIK lookup and filing downloads
    fmp.py          — FMP API client (analyst estimates, IFRS stub)
    importer.py     — XBRL → PeriodFinancials orchestration (no segment data in companyfacts — Ticket 7)
    market_data.py  — yfinance price and earnings loader
    options.py      — yfinance option-chain snapshots + derived metrics (ATM IV, implied move, skew, P/C)
    estimates.py    — yfinance forward consensus snapshots (dated)
    schema.py       — PeriodFinancials, CompanyFinancials, PricePoint, EarningsSurprise, OptionChainSnapshot, ConsensusEstimates
    store.py        — JSON persistence (periods only) + in-memory session cache
    xbrl_api.py     — SEC XBRL JSON API client (dedupe keeps latest-filed per period)
    xbrl_tags.py    — XBRL concept registry (TagSpec, sign_flip, fallbacks)

    dcf.py          — aggregate Monte Carlo DCF (P10/P50/P90, per-share)
    scenarios.py    — bear/base/bull presets + per-ticker net debt/shares registry (IBM entries are v3-era)
    driver_dcf.py   — driver-based Monte Carlo DCF engine (correlated drivers, vetted formulas)
    backtest.py     — point-in-time vintage backtest (filed_date visibility, mechanical params, DATA_FLAGS)

scripts/
    analyze.py           — full pipeline → 9-tab xlsx (primary entrypoint)
    import_financials.py — import only, no xlsx
    import_edgar.py      — bulk raw-filing download (archival fallback)
    run_dcf.py           — aggregate Monte Carlo DCF on imported data
    visualize_dcf.py     — interactive single-page company report → self-contained HTML
    backtest.py          — vintage backtest CLI (table + coverage + bias by horizon)
    setup-officecli.sh / memo-review-reminder.sh — review-gate hooks (see CLAUDE.md)

analysis/IBM/
    driver_model.py     — v4 driver register (NORMATIVE assumption source)
    dcf_drivers_run.py  — v4 runner: headline, loci, disclosure runs
    dcf_scenarios.py    — v3 console report (aggregate engine)
    README.md · REVIEW.md · DRIVERS.md · DRIVER-REVIEW.md · research-ledger.md
    report/             — IBM-valuation-2026-07.{md,docx} + SOURCE-AUDIT.md

docs/
    ARCHITECTURE.md · HIDDEN-ASSUMPTIONS.md · INDEX.md
    visualizer-design-brief.md · design-specs/ · superpowers/ (historical plans/specs)

PLANNING/
    reusable-dcf-vision.md — the valuation-factory roadmap (NORMATIVE)
    RULES.md               — rulebook (L4 ratchet)
    tickets.md             — closed Tickets 1–6 (history) + current open tickets
    HANDOFF.md             — transient live work-state
    token-ledger.md        — cost per run

data/
    raw/TICKER/          — remote data cache (gitignored)
    raw/TICKER/options/  — dated option-chain snapshots (gitignored)
    raw/TICKER/estimates/— dated consensus snapshots (gitignored)
    processed/           — JSON and xlsx outputs (gitignored; two xlsx are tracked legacy warts — never commit them)

CLAUDE.md               — agent onboarding contract (read first)
context.json            — project summary for agent orientation
```

## Roadmap

The build-out tickets that created the pipeline are all closed:

- [x] Tickets 1–6 — EDGAR retrieval, XBRL client, schema, store, importer, xlsx export,
  market data, aggregate DCF, docs (history preserved in `PLANNING/tickets.md`)

Everything forward is governed by the **valuation factory** vision
(`PLANNING/reusable-dcf-vision.md`, 7 layers, amendments ✦1–✦7). Phase state as of
2026-07-17 — **P1 in progress**:

- Done: driver engine + IBM v4 frontier run (reviewed, audited, reported); importer
  completeness pass (dividends, interest income, RPO mapped; segment-axis absence
  documented); point-in-time backtest harness; RULES.md seeded (R1); research ledger
  with expiry; token ledger.
- Pending in P1: validators, red-team corpus + injection harness, statement-coverage
  validator, charter-review wiring.
- Next: ORCL frontier run (ticker #2) under the cheapened pipeline, then the basket
  (IBM/ORCL/MSFT/ACN/CSCO) and cross-sectional gap decomposition (✦1).
- Open model tickets (T-DCF-1 AR(1) growth, T-DCF-3 derived tg, T-IMP-1 importer
  fidelity, Ticket 7 segments, …): `PLANNING/tickets.md`.
