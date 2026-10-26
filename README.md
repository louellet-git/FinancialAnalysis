---
id: DOC-README
type: description
item: 10.67 User documentation
status: current
date: 2026-10-25
scope: Overview of the pipeline, its CLI commands, outputs and layout.
---
# FinancialAnalysis

A **valuation factory**: Python pipeline that pulls public-company financials from SEC
EDGAR, enriches them with market data (prices, earnings surprises, forward consensus,
option chains), and runs Monte Carlo DCF valuations through two engines — a classic
scenario engine and a driver-based engine (mechanics: docs/ARCHITECTURE.md) — feeding an
adversarial review pipeline and a single-page HTML company report. Rules: PLANNING/RULES.md; open work: the board, PLANNING/board/ (`python scripts/board.py status`); unticketed backlog: PLANNING/tickets.md. Flagship analysis: IBM (`analysis/IBM/`, v4.1
report `analysis/IBM/report/IBM-valuation-2026-07.md`).

## Pipeline commands

Run from the repo root. `--help` on each argparse-based `scripts/` entry point lists its flags; the `analysis/IBM` runs take no flags.

```bash
pip install -r requirements.txt   # runtime deps

# 1. Full pipeline: import + market data → data/processed/TICKER_financials.xlsx
python -m scripts.analyze IBM
python -m scripts.analyze IBM --refresh        # force re-fetch from remote APIs
python -m scripts.analyze IBM --no-market      # SEC statements only
python -m scripts.analyze IBM --no-options     # skip the option-chain snapshot
python -m scripts.analyze IBM --no-estimates   # skip forward consensus estimates
python -m scripts.analyze IBM --out custom.xlsx

# 2. Import only (JSON store, no xlsx)
python -m scripts.import_financials IBM [--refresh]

# 3. Bulk filing download (archival fallback path; companyfacts is primary)
python -m scripts.import_edgar IBM 2026-07-01 [--output-dir data/raw] [--lookback-months 18] [--lookahead-months 6]

# 4. Classic Monte Carlo DCF on stored data (no network)
python -m scripts.run_dcf PANW [--simulations 10000] [--years 10] [--growth 0.15] [--wacc 0.10] [--terminal-growth 0.03] [--seed 7]

# 5. Point-in-time vintage backtest of operating projections (no network)
python -m scripts.backtest IBM [--horizon 3] [--simulations 10000] [--seed 7]

# 6. Single-page interactive company report → data/processed/TICKER_dcf.html (price: yfinance last price unless --price)
python -m scripts.visualize_dcf IBM [--price 195.40] [--out report.html]

# 7. IBM v4 driver-engine run: headline, E/V fixed point, loci, disclosure, reinvestment check (seed 7)
python -m analysis.IBM.dcf_drivers_run

# 8. IBM v3 aggregate framework (historical; output drifts from the v3 record, HIDDEN-ASSUMPTIONS #24)
python -m analysis.IBM.dcf_scenarios

# 9. Test suite — no network; pytest is in requirements-dev.txt
python -m pytest
```

Remote data is disk-cached under `data/raw/TICKER/`; re-runs read the cache unless `--refresh`. Option and consensus snapshot rules: HIDDEN-ASSUMPTIONS #9.

## xlsx structure (nine tabs)

`scripts/analyze.py` writes every sheet on every run (HIDDEN-ASSUMPTIONS #17):

| Tab | Contents |
|-----|----------|
| Income Statement | ~23 mapped metrics per period (revenue → EPS, SBC, D&A) |
| Balance Sheet | ~24 mapped metrics + RPO disclosure rows (`statement=disclosure`) |
| Cash Flow | OCF, capex, FCF, buybacks, dividends, debt flows + derived FCF/margin |
| Raw | All unmapped us-gaap USD concepts (100–200 rows per period) |
| Prices | Daily OHLCV, full history (source: Data sources) |
| Estimates | Earnings surprises and FMP estimates (sources: Data sources) |
| Estimates Fwd | Dated forward consensus snapshot: revenue/EPS estimates by period, analyst price targets |
| Options | Full option chain, one row per contract |
| Options Summary | Per-expiry ATM IV, implied move (ATM straddle ÷ spot), 90/110 skew, put/call ratios + chain-wide `ALL` row |

Financial tabs use long format: one row per `(period, metric)`. The JSON store persists
statements only (docs/ARCHITECTURE.md); market data lives in the xlsx and the caches above.

## Data sources

| Data | Source | Auth |
|------|--------|------|
| Financial statements | SEC EDGAR companyfacts XBRL JSON API | None (public; User-Agent set in `edgar.py`) |
| Daily prices, earnings surprises, forward consensus, option chains (current snapshot) | yfinance | None |
| Analyst estimates; earnings surprises (override yfinance when returned) | FMP (v3 legacy endpoints unavailable on free plan) | `FMP_API_KEY` in `.env` (gitignored) |

Any US-GAAP EDGAR filer (verified: NVDA, PANW, WMT, TGT, DUOL, IBM, MSFT, AAPL). ETFs
and IFRS filers raise `UnsupportedFilingError`. Segment revenue is unavailable:
PLANNING/tickets.md Ticket 7. Option history: none (HIDDEN-ASSUMPTIONS #9); upgrade path FMP or Polygon (`financial_analysis/options.py` docstring).

## Project layout

```
CLAUDE.md                    agent orientation: norms + normativity map (read first)
context.json                 machine-readable project summary for agents
README.md                    this file
THOUGHTS                     raw idea scratchpad (see CLAUDE.md, normativity item 8)
requirements.txt             runtime deps
requirements-dev.txt         test deps (pytest)

financial_analysis/          the package; see docs/ARCHITECTURE.md
scripts/                     CLI entry points and tooling; see docs/ARCHITECTURE.md
tests/                       test suite (no network; see docs/ARCHITECTURE.md §Tests)
analysis/IBM/                driver_model.py (v4 register),
                             dcf_drivers_run.py, dcf_scenarios.py (v3), DRIVERS.md,
                             REVIEW.md, DRIVER-REVIEW.md, research-ledger.md, report/,
                             README.md (v3, superseded; net-debt bridge)
docs/                        COLLABORATION.md (two-person workflow), DOC-STANDARD.md,
                             ARCHITECTURE.md, HIDDEN-ASSUMPTIONS.md,
                             visualizer design brief (docs/visualizer-design-brief.md) + accepted design spec (docs/design-specs/2026-07-16-ratings-desk-blueprint.md),
                             superpowers/ (historical plans/specs)
PLANNING/                    reusable-dcf-vision.md (standing initiative), RULES.md,
                             tickets.md, tickets-closed.md, token-ledger.md, board/ (tickets),
                             handoff/ (state)
.github/workflows/           CI (tests, main guard, branch cleanup)
data/raw/ data/processed/    caches and outputs (gitignored)
```
