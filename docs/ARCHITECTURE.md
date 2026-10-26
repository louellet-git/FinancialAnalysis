---
id: DOC-ARCHITECTURE
type: description
item: 10.58 System architecture description
status: current
date: 2026-10-25
scope: Code-verified module map, data flow and engine mechanics; the code governs on disagreement.
---
# Architecture

Context: the valuation factory in README.md; agent orientation in CLAUDE.md.

## Data flow

```
SEC EDGAR (companyfacts JSON API)          yfinance
        │                                     │
   edgar.py (CIK) ──► xbrl_api.py        market_data.py (prices, surprises)
        │                  │             estimates.py   (forward consensus snapshots)
        ▼                  ▼             options.py     (option-chain snapshots)
   cache.py ──► importer.py + xbrl_tags.py    │      fmp.py (optional FMP estimates/surprises) ──┐
                     │                        └──────────────────────────────────────────────────┤
                     ▼                                                                           │
              schema.py (PeriodFinancials)                                                       │
                     │                                                                           │
                     ▼                                                                           │
              store.py (data/processed/TICKER.json — statements ONLY) ───────────────────────────┤
                     │                                                                           │
            ┌────────┴────────────────────────────┐                                              │
            ▼                                     ▼                                              │
        dcf.py                            backtest.py         analysis/IBM/driver_model.py       │
      (classic MC)                (point-in-time vintages)  (per-ticker register; no store read) │
            │                                     │                 │                            │
            │                                     │                 ▼                            │
            │                                     │           driver_dcf.py                      │
            │                                     │       (driver-based MC engine)               │
            ▼                                     │                 │                            │
      scenarios.py ──► scripts/visualize_dcf.py ◄───┴─────────────────┘         scripts/analyze.py
                     │                                                                           │
                     ▼                                                                           ▼
              TICKER_dcf.html                                      TICKER_financials.xlsx (9 tabs)
```

## The two DCF engines

Two Monte Carlo DCF engines coexist; neither replaces the other.

### 1. Classic scenario engine — `financial_analysis/dcf.py` (`StochasticDCF`)

Samples the **top-level valuation items directly**: revenue growth, FCF margin, WACC,
and terminal growth, each an independent normal draw per path (WACC floored at 1%, FCF
margin clipped to [0, 1]; 10,000 sims, 10-year horizon by default). FCF margin is derived from the last 5 annual rows when not
pinned (phantom rows can shrink the window; HIDDEN-ASSUMPTIONS #2). Gordon terminal value; its denominator floor (one reason the driver engine
exists) is HIDDEN-ASSUMPTIONS #15. Minimum-history guard: HIDDEN-ASSUMPTIONS #16.
`scenarios.py` IBM entries are the v3 aggregate framework: HIDDEN-ASSUMPTIONS #13.

### 2. Driver-based engine — `financial_analysis/driver_dcf.py` (`DriverDCF`)

Every top-level item (revenue, FCFF margin, WACC, terminal value) is a
**deterministic, unit-tested formula** over *drivers*; the drivers are the only
stochastic objects.

- **Parameter-MC, one draw per path.** Each of the 10,000 paths draws one value per
  stochastic driver (e.g. one software growth rate) and holds it constant across all 10
  projection years. No year-to-year dynamics (HIDDEN-ASSUMPTIONS #12).
- **Joint sampling.** Stochastic drivers are drawn as correlated standard normals (full
  pairwise correlation matrix, Cholesky-factorized, validated positive-definite), then transformed to each driver's marginal as `mean + std·z`.
  Truncated drivers: HIDDEN-ASSUMPTIONS #14.
- **Per-path WACC.** WACC is computed per path from CAPM drivers (`rf`, `beta`, `erp`,
  `kd_spread`, `tax_rate`, `equity_weight`), and the after-tax cost of debt is *derived*
  from the same path's risk-free draw: `(rf + spread)·(1 − tax)`. kd moves with rf on
  every path (spread fixed, see analysis/IBM/DRIVERS.md kd_spread row).
- **Degenerate-draw guard (T-DCF-2, analysis/IBM/REVIEW.md).** Per path, `tg_eff = min(tg_draw, wacc_path - 0.02)`;
  pinned share in `tg_pinned_fraction` (0.0009 for IBM, seed 7).
  Results expose percentiles only (fat tails make moments misleading).
- **Attribution.** `run()` re-runs the full valuation per stochastic driver with its
  mean shifted ±1σ under the **same seed** (identical z-draws, correlations intact) and
  reports the P50 per-share delta — the tornado chart and per-driver review inputs.
- **Deterministic drivers** become constant arrays, or per-year `schedule`s (e.g. the
  SBC ramp 2.54%→3.5%); `mean` must equal `schedule[0]`. Ambiguity (stochastic with
  std 0, deterministic with bounds, duplicate names) is rejected at construction; non-PSD
  matrices fail at first `sample()`. Nothing is repaired.
- **Required drivers.** `ROLE_DRIVERS` names 12 mandatory drivers (core_margin,
  margin_mix_drift, sbc_ratio, ma_ratio, ma_recurrence, rf, beta, erp, kd_spread,
  tax_rate, equity_weight, terminal_growth) plus one growth driver per revenue segment.
  `SegmentSpec` can pin year-1 growth to known actuals (draws apply from year 2).

Per-ticker parameterization lives in `analysis/<TICKER>/driver_model.py` (IBM is the
only instance): drivers with written rationales, the correlation matrix, segment bases,
net debt, shares, exposing `build()`. Precedence: CLAUDE.md §Normativity map, item 2.

## Backtest harness

`run_backtest(company, horizon=3, n=10_000, seed=7)` in `financial_analysis/backtest.py`
— walk-forward vintages, one per clean fiscal-year-end + 90 days:

- **Point-in-time estimates**: a vintage sees only periods with `filed_date <= as_of`; vintages are conservative (HIDDEN-ASSUMPTIONS #4).
- **Mechanical parameters, no judgment**: trailing revenue CAGR + YoY std, FCF-margin
  mean/std over ≤5 clean annuals (stds floored at 0.01); mirrors `StochasticDCF`'s
  sampling structure (one growth + one margin draw per path).
- **Asymmetric actuals, disclosed**: actuals join from the store *as currently reported*
  (restatements included) — how forecasts are judged in practice.
- **DATA_FLAGS, not fixes**: IBM FY2019/FY2020 windows carry the recast flag (HIDDEN-ASSUMPTIONS #1).
- Phantom rows are filtered by `_clean_annuals` (HIDDEN-ASSUMPTIONS #2).
- Scope: revenue/FCF projections only — no WACC, no terminal value, no price.

The HTML report renders it as the "Backtest — mechanical vintages" section (layout:
docs/visualizer-design-brief.md §6). Calibration: IBM 14/18 realized outcomes in the P10–P90
band (78% vs 80% nominal); DUOL 0/6 — mechanical extrapolation fails on growth-phase tickers,
so treat their bands as decorative until a driver build exists.

## `financial_analysis/` modules

| Module | What it does |
|---|---|
| `__init__.py` | Re-exports `EdgarClient`. |
| `backtest.py` | Vintage backtest of revenue/FCF projections; see Backtest harness. Skipped vintages are recorded with reasons. |
| `cache.py` | Disk cache for SEC companyfacts JSON at `data/raw/TICKER/company_facts.json`; re-fetches on `refresh` or corrupt cache. |
| `dcf.py` | Classic engine (above): `DCFAssumptions`, `DCFResult`, `StochasticDCF`, `derive_fcf_margin`. Annual (10-K/10-K/A) periods only. |
| `driver_dcf.py` | Driver-based engine (above): `Driver`, `Item`, `DriverModel`, `SegmentSpec`, `DriverDCF`, `DriverAttribution`, `DriverDCFResult` (incl. `driver_draws` per-path values for the path explorer), vetted item formulas `revenue_paths`, `fcff_margin`, `wacc_capm`, `terminal_value`, registered in `ITEMS`. |
| `edgar.py` | SEC HTTP client (User-Agent, 0.15s rate limit), CIK lookup from company_tickers.json, full-index filing downloads (`EdgarClient.download_for_ticker`), `EdgarClientError`. |
| `estimates.py` | yfinance **forward** consensus (revenue/EPS estimates, price targets) → `ConsensusEstimates`; persisted as dated snapshots `data/raw/TICKER/estimates/YYYY-MM-DD.json` (history: HIDDEN-ASSUMPTIONS #9). `fetch_estimates`, `list_snapshots`, `load_snapshot`. |
| `fmp.py` | Optional FMP API client (`FMP_API_KEY` in `.env`): analyst estimates/surprises, IFRS statements stub (`NotImplementedError`). Free-plan v3 endpoints are unavailable; the pipeline falls back to yfinance. |
| `importer.py` | companyfacts → `PeriodFinancials`: maps concepts via `xbrl_tags` (first-wins merge across fallback concepts per period_end, sign flips), collects unmapped us-gaap USD concepts into `raw_extras`, best-effort `filed_date` from revenue-concept `filed` fields, raises `UnsupportedFilingError` for non-us-gaap filers. `segment_revenue` is None (HIDDEN-ASSUMPTIONS #5). |
| `market_data.py` | yfinance daily OHLCV (`load_prices`, cached at `data/raw/TICKER/prices.json`) and earnings surprises (`load_earnings_surprises`). |
| `options.py` | yfinance option-chain snapshots (dated, `data/raw/TICKER/options/YYYY-MM-DD.json`; history: HIDDEN-ASSUMPTIONS #9) + derived per-expiry metrics: ATM IV, straddle-mid implied move, 90/110 skew, put/call volume & OI ratios (`fetch_option_chain`, `compute_chain_metrics`, `list_snapshots`, `load_snapshot`). |
| `scenarios.py` | Bear/base/bull presets (`DEFAULT_SCENARIOS` generic mature-large-cap; `_SCENARIO_OVERRIDES` for IBM and DUOL), `CAPITAL_OVERRIDES` (IBM, DUOL), `capital_structure_for` (derives net debt from latest 10-K: long_term_debt + short-term debt (first of ShortTermBorrowings, LongTermDebtAndCapitalLeaseObligationsCurrent in raw_extras; else 0) − cash; shares_outstanding falling back to diluted). |
| `schema.py` | Dataclasses: `PeriodFinancials` (~60 statement fields + `remaining_performance_obligation`, reserved `segment_revenue`, `raw_extras`; derived `free_cash_flow` (sign trap: HIDDEN-ASSUMPTIONS #7) and `fcf_margin`), `CompanyFinancials`, `PricePoint`, `EarningsEstimate`, `EarningsSurprise`, `OptionQuote`, `OptionChainSnapshot`, `ConsensusEstimates`. |
| `store.py` | JSON persistence `data/processed/TICKER.json`; in-memory session cache; `get_series`/`compare` query helpers with metric-name validation. |
| `xbrl_api.py` | companyfacts fetch + series extraction: filters by form type/unit, deduplicates by end date (HIDDEN-ASSUMPTIONS #4), returns last `n` entries. |
| `xbrl_tags.py` | The concept registry: `TagSpec` (fallback concept lists, unit, sign_flip) for income statement / balance sheet / cash flow / disclosure (RPO) fields, plus `STATEMENT_MAP` and `UNIT_MAP`. Inline comments document precedence rationales (interest_income, dividends_paid). |

## `scripts/`

Commands for the first six scripts below (the pipeline): README.md §Pipeline commands.

| Script | What it does |
|---|---|
| `analyze.py` | Full pipeline: import → attach prices/surprises/consensus/options → `data/processed/TICKER_financials.xlsx`. Writes the nine-sheet workbook (Income Statement, Balance Sheet, Cash Flow, Raw, Prices, Estimates, Estimates Fwd, Options, Options Summary; HIDDEN-ASSUMPTIONS #17). RPO rows: HIDDEN-ASSUMPTIONS #19. |
| `import_financials.py` | Import + save JSON only, no xlsx. |
| `import_edgar.py` | Bulk full-index filing download (archival fallback path — companyfacts is the primary parse path). |
| `run_dcf.py` | Classic-engine MC DCF on stored data; no network. |
| `backtest.py` | Vintage backtest console report: per-vintage P10/P50/P90 vs actuals, band coverage vs nominal 80%, signed bias by horizon, data flags, skipped vintages. No network. |
| `visualize_dcf.py` | Single-page HTML report (plotly.js inlined); six sections in `SECTION_TITLES`; panels and styling in docs/visualizer-design-brief.md. |
| `board.py` | `python scripts/board.py check \| status \| branch NAME \| new --title --area --owner --branch --acceptance [--status]`. Scrum-board checker and ticket creator over `PLANNING/board/` (rules in its README). CI use: docs/COLLABORATION.md. |
| `board_gate.py` | (PreToolUse hook, not a CLI) Board gate on edits, commits, pushes, PRs and GitHub MCP writes. Rules and limits: docs/COLLABORATION.md. |
| `main_guard.py` | `git diff --no-renames --name-only B A \| python -m scripts.main_guard <1 if A merges a PR, else 0>`. Exit 2 = revert (rule: docs/COLLABORATION.md). `revert B` (undo pushes since B; main-guard.yml) and `pr-done BASE HEAD` (fails when a PR sets a ticket to done; tests.yml). |
| `stale_branches.py` | `python -m scripts.stale_branches BRANCHES MERGED OPEN_PR_HEADS`. Selects the remote branches the daily cleanup deletes (criteria: docs/COLLABORATION.md). |
| `gitcmd.py` | Library, no CLI: parses git invocations for the hook gates (`board_gate.py`, `doc_check.py`). |
| `doc_check.py` | `python scripts/doc_check.py check \| list \| record <path> <PASS\|FAIL>`; also its hook (`--hook`), which gates git commit, merge, cherry-pick, revert, pull, am, rebase and push. Rules: docs/DOC-STANDARD.md. |
| `cloud-setup.sh` | `bash scripts/cloud-setup.sh`. Idempotent environment setup: installs requirements.txt and requirements-dev.txt (pytest). |

CI (`.github/workflows/`): `tests.yml`, `main-guard.yml`, `branch-cleanup.yml` (behavior: docs/COLLABORATION.md).

## Analysis entry points

| Entry point | What it does |
|---|---|
| `python -m analysis.IBM.dcf_scenarios` | v3 aggregate framework console report: FCFF v3 margins, paired (growth, f; f = M&A recurrence share, HIDDEN-ASSUMPTIONS #11) scenarios, WACC grid, f-locus. Not a headline source (see next row); live store-derived output drifts from the v3 record (HIDDEN-ASSUMPTIONS #24). |
| `python -m analysis.IBM.dcf_drivers_run` | v4 driver-engine runner: headline P10/P50/P90 + attribution, E/V fixed-point iteration, f-locus (HIDDEN-ASSUMPTIONS #11), SBC-drift locus, beta locus, supply-shock correlation and `tax_rate`=0 (no debt tax shield) disclosure runs, implied-reinvestment check. Loci and disclosure runs: `analysis/IBM/dcf_drivers_run.py` docstring. |

## Tests

`python -m pytest` from the repo root runs the full suite; `tests/` covers most modules and
scripts; no test file for `edgar.py`, `run_dcf.py` or `import_*.py`. No network (HIDDEN-ASSUMPTIONS #31).
