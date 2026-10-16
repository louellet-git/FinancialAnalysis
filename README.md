# FinancialAnalysis

A **valuation factory**: Python pipeline that pulls public-company financials from SEC
EDGAR, enriches them with market data (prices, earnings surprises, forward consensus,
option chains), and runs Monte Carlo DCF valuations through two engines — a classic
scenario engine and a driver-based engine — feeding an adversarial review pipeline and
a single-page HTML company report. Flagship analysis: IBM (`analysis/IBM/`, v4.1
report).

Agent onboarding: **CLAUDE.md** (norms + normativity map) → `docs/INDEX.md` →
`docs/ARCHITECTURE.md` → `docs/HIDDEN-ASSUMPTIONS.md`.

## Pipeline commands

All commands below were verified against the code on 2026-07-17 (flags read from each
script's argparse; defaults shown). Run from the repo root.

```bash
pip install -r requirements.txt   # numpy, openpyxl, plotly, yfinance, python-dotenv, requests

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

# 6. Single-page interactive company report → data/processed/TICKER_dcf.html
python -m scripts.visualize_dcf IBM [--price 195.40] [--out report.html]

# 7. IBM v4 driver-engine run: headline + loci + disclosure runs (seed 7)
python -m analysis.IBM.dcf_drivers_run

# 8. IBM v3 aggregate framework (historical, kept reproducible)
python -m analysis.IBM.dcf_scenarios

# 8b. Investor verdict over the driver engine (no network; price is an input)
python -m scripts.verdict IBM --price 212.40 --dividend-yield 0.0313

# 9. Test suite — 302 tests, no network (~12s); pytest not in requirements.txt
python -m pytest
```

(Two shell scripts also live in `scripts/`: `memo-review-reminder.sh`, a .docx
review-gate hook, and `setup-officecli.sh`, the officecli check/installer — neither is
a pipeline command.)

All remote data is disk-cached under `data/raw/TICKER/` — re-runs are instant; option
chains and forward consensus are saved as **dated snapshots** (yfinance serves current
data only, so history accumulates from the first pull; a same-day re-run reads disk,
`--refresh` overwrites).

## The two DCF engines

**Classic scenario engine** — `financial_analysis/dcf.py` (`StochasticDCF`). Samples
revenue growth, FCF margin, WACC, and terminal growth directly (independent normal
draws, 10k sims × 10y default), Gordon terminal value, P10/P50/P90 + per-share.
FCF margin derives from the last 5 annual periods when not pinned; only annual
(10-K) periods feed the model; a minimum-history guard raises unless ≥ 4 annual periods
have positive FCF. Bear/base/bull presets and the per-ticker net-debt/shares registry
live in `financial_analysis/scenarios.py` (the IBM entries there implement the **v3**
framework — deliberate; see docs/HIDDEN-ASSUMPTIONS.md #13).

**Driver-based engine** — `financial_analysis/driver_dcf.py` (`DriverDCF`). Top-level
items (revenue, FCFF margin, WACC, terminal value) are deterministic vetted formulas;
all randomness lives in named *drivers*, each declaring stochastic-vs-deterministic
with a written rationale. One draw per stochastic driver per path (held constant across
years), jointly sampled through a validated correlation matrix; per-path WACC with the
cost of debt derived from the same path's risk-free draw; terminal growth jointly
guarded at `min(tg, WACC − 2%)` per path (pinned fraction reported); percentiles only —
no mean/std. ±1σ per-driver attribution runs under the same seed. Per-ticker registers:
`analysis/<TICKER>/driver_model.py` (IBM is the only one). Full mechanics:
docs/ARCHITECTURE.md.

**Backtest harness** — `financial_analysis/backtest.py`. Walk-forward vintages at each
fiscal-year-end + 90 days; each vintage sees only periods with `filed_date <= as_of`
(true point-in-time — conservative, because companyfacts filed dates come from later
filings' comparative columns), derives mechanical parameters, projects revenue/FCF
+1..+3y, and scores against actuals as currently reported. Known input defects are
surfaced as data flags, not fixed. Operating projections only — no WACC, no terminal
value, no price. A "Backtest — mechanical vintages" section of the HTML report (vintage
table + hedgehog chart, merged in PR #9) renders it in `visualize_dcf` output.

## xlsx structure (nine tabs)

`scripts/analyze.py` writes all nine sheets on every run (skipped groups are
header-only):

| Tab | Contents |
|-----|----------|
| Income Statement | ~25 mapped metrics per period (revenue → EPS, SBC, D&A) |
| Balance Sheet | ~24 mapped metrics + RPO disclosure rows (`statement=disclosure`) |
| Cash Flow | OCF, capex, FCF, buybacks, dividends, debt flows + derived FCF/margin |
| Raw | All unmapped us-gaap USD concepts (100–200 rows per period) |
| Prices | Daily OHLCV from yfinance (full history) |
| Estimates | Historical earnings surprises (yfinance) + FMP estimates when available |
| Estimates Fwd | Dated forward consensus snapshot: revenue/EPS estimates by period, analyst price targets |
| Options | Full option chain, one row per contract |
| Options Summary | Per-expiry ATM IV, implied move (ATM straddle ÷ spot), 90/110 skew, put/call ratios + chain-wide `ALL` row |

Financial tabs use long format: one row per `(period, metric)`. The JSON store
(`data/processed/TICKER.json`) persists **statements only** — market data lives in the
xlsx and the dated caches under `data/raw/TICKER/`.

## Data sources

| Data | Source | Auth |
|------|--------|------|
| Financial statements | SEC EDGAR companyfacts XBRL JSON API | None (public; User-Agent set in `edgar.py`) |
| Daily prices, earnings surprises, forward consensus, option chains (current snapshot) | yfinance | None |
| Analyst estimates / option history backfill | FMP (v3 legacy endpoints unavailable on free plan since Aug 2025) / Polygon | `FMP_API_KEY` in `.env` (gitignored) |

Supported: any US-GAAP filer on EDGAR (verified: NVDA, PANW, WMT, TGT, DUOL, IBM,
MSFT, AAPL). ETFs and IFRS filers raise a clear `UnsupportedFilingError` instead of
producing bad data. Segment-level revenue is NOT available from companyfacts (the API
strips dimensional facts) — see PLANNING/tickets.md Ticket 7.

## Tests

```bash
python -m pytest        # 302 tests, all passing as of 2026-07-17 (~12s)
```

No network calls in the suite (HTTP mocked, `tmp_path` fixtures). `pytest` itself is
not in `requirements.txt` — install separately.

## Project layout

```
CLAUDE.md                    agent orientation: norms + normativity map (read first)
context.json                 machine-readable project summary for agents
README.md                    this file
THOUGHTS                     raw idea scratchpad
requirements.txt             runtime deps (no pytest)

financial_analysis/          the package — 16 modules; see docs/ARCHITECTURE.md
    edgar.py xbrl_api.py cache.py xbrl_tags.py importer.py schema.py store.py
    market_data.py estimates.py options.py fmp.py
    dcf.py scenarios.py      classic engine + presets/capital registry
    driver_dcf.py            driver-based engine
    backtest.py              point-in-time vintage backtest

scripts/                     CLI entry points (analyze, import_financials, import_edgar,
                             run_dcf, backtest, visualize_dcf) + 2 shell hooks
tests/                       302 tests across 17 modules
analysis/IBM/                flagship analysis: driver_model.py (v4 register),
                             dcf_drivers_run.py, dcf_scenarios.py (v3), DRIVERS.md,
                             REVIEW.md, DRIVER-REVIEW.md, research-ledger.md, report/
docs/                        ARCHITECTURE.md, HIDDEN-ASSUMPTIONS.md, INDEX.md,
                             visualizer design brief + accepted design spec,
                             superpowers/ (historical plans/specs)
PLANNING/                    reusable-dcf-vision.md (standing initiative), RULES.md,
                             tickets.md, token-ledger.md, HANDOFF.md (live state)
.claude/agents/              vendored review pipeline: valuation-analyst/-skeptic,
                             memo-reviewer + rubric, report-auditor (FRAMEWORK.md)
data/raw/ data/processed/    caches and outputs (gitignored)
```
