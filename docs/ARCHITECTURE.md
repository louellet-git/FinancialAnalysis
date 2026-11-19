# Architecture — code-verified module map

Verified against the code on 2026-07-17 (branch `docs-overhaul`, based on origin/main
`9751523`). Every claim below was checked by reading the named module. When this doc and
the code disagree, the code governs.

## Data flow

```
SEC EDGAR (companyfacts JSON API)          yfinance
        │                                     │
   edgar.py (CIK) ──► xbrl_api.py        market_data.py (prices, surprises)
        │                  │             estimates.py   (forward consensus snapshots)
        ▼                  ▼             options.py     (option-chain snapshots)
   cache.py ──► importer.py + xbrl_tags.py    │
                     │                        │
                     ▼                        │
              schema.py (PeriodFinancials)    │
                     │                        │
                     ▼                        ▼
              store.py (data/processed/TICKER.json — statements ONLY)
                     │
        ┌────────────┼───────────────────────────────┐
        ▼            ▼                               ▼
   dcf.py       driver_dcf.py                  backtest.py
 (classic MC)  (driver-based MC engine)     (point-in-time vintages)
        │            │
        │      analysis/IBM/driver_model.py (per-ticker register)
        ▼            ▼
  scenarios.py   scripts/visualize_dcf.py ──► data/processed/TICKER_dcf.html
        │
  scripts/analyze.py ──► data/processed/TICKER_financials.xlsx (9 tabs)
```

Market data (prices, estimates, options) is attached to the in-memory
`CompanyFinancials` by `scripts/analyze.py` and written to the xlsx, but it is **never
persisted to the JSON store** — `store.save()` serializes `periods` only
(`financial_analysis/store.py`). Dated per-day snapshots under `data/raw/TICKER/`
(options/, estimates/, prices.json) are the durable market-data record.

## The two DCF engines

The repo has two distinct Monte Carlo DCF engines. They coexist deliberately; neither
replaces the other.

### 1. Classic scenario engine — `financial_analysis/dcf.py` (`StochasticDCF`)

Samples the **top-level valuation items directly**: revenue growth, FCF margin, WACC,
and terminal growth, each an independent normal draw per path (10,000 sims, 10-year
horizon by default). FCF margin is derived from the last 5 annual periods when not
pinned. Gordon terminal value with a 0.001 floor on the `wacc − tg` denominator (which
lets near-degenerate paths dominate the tails — one reason the driver engine exists).
Minimum-history guard: raises `ValueError` unless ≥ 4 annual periods have positive FCF.
`financial_analysis/scenarios.py` supplies bear/base/bull assumption presets and a
per-ticker net-debt/shares registry for it. Note: the IBM entries in `scenarios.py` are
the **v3** aggregate framework (base P50 $141 family), kept as the aggregate-scenario
presets; the current v4 numbers live only in the driver engine's register (see
docs/HIDDEN-ASSUMPTIONS.md).

### 2. Driver-based engine — `financial_analysis/driver_dcf.py` (`DriverDCF`)

Never samples a top-level item. Every top-level item (revenue, FCFF margin, WACC,
terminal value) is a **deterministic, unit-tested formula** over underlying *drivers*,
and the drivers are the only stochastic objects. In plain language:

- **Parameter-MC, one draw per path.** Each of the 10,000 paths draws one value per
  stochastic driver (e.g. one software growth rate) and holds it constant across all 10
  projection years. There is no year-to-year dynamics (no AR(1) mean reversion — that is
  ticket T-DCF-1, disclosed in the module docstring). This widens far-year dispersion
  and understates the left tail; the report calls the tails decorative.
- **Joint sampling.** Stochastic drivers are drawn as correlated standard normals (full
  pairwise correlation matrix, Cholesky-factorized, validated positive-definite — never
  silently repaired), then transformed to each driver's marginal as `mean + std·z`.
  Truncated drivers (`dist="truncnorm"`) are **clipped after correlation** — disclosed
  as piling probability mass on the bounds; reviewers must check bounds sit > ~2σ from
  the mean.
- **Per-path WACC.** WACC is computed per path from CAPM drivers (`rf`, `beta`, `erp`,
  `kd_spread`, `tax_rate`, `equity_weight`), and the after-tax cost of debt is *derived*
  from the same path's risk-free draw: `(rf + spread)·(1 − tax)`. Deriving kd from rf
  keeps credit spreads non-negative by construction (a fixed kd against a floating rf
  produced negative-spread paths — DRIVER-REVIEW finding, fixed).
- **Degenerate-draw guard (T-DCF-2).** Effective terminal growth per path is
  `min(tg_draw, wacc_path − 0.02)`, so the Gordon denominator is at least 2% on every
  path — no 0.001-clip absurdities. The fraction of paths where the guard binds is
  reported (`tg_pinned_fraction`; 0.0009 for IBM at seed 7). Results expose percentiles
  only — mean/std are deliberately absent (fat tails make moments misleading).
- **Attribution.** `run()` re-runs the full valuation per stochastic driver with its
  mean shifted ±1σ under the **same seed** (identical z-draws, correlations intact) and
  reports the P50 per-share delta — the tornado chart and per-driver review inputs.
- **Deterministic drivers** become constant arrays, or per-year `schedule`s (e.g. the
  SBC ramp 2.54%→3.5%); `mean` must equal `schedule[0]`. Ambiguity (stochastic with
  std 0, deterministic with bounds, non-PSD matrices, duplicate names) is rejected at
  construction, not repaired.
- **Required drivers.** `ROLE_DRIVERS` names 12 mandatory drivers (core_margin,
  margin_mix_drift, sbc_ratio, ma_ratio, ma_recurrence, rf, beta, erp, kd_spread,
  tax_rate, equity_weight, terminal_growth) plus one growth driver per revenue segment.
  `SegmentSpec` can pin year-1 growth to known actuals (draws apply from year 2).

Per-ticker parameterization lives in `analysis/<TICKER>/driver_model.py` (IBM is the
only instance): drivers with written rationales, the correlation matrix, segment bases,
net debt, shares, exposing `build()`. The rationale strings in `driver_model.py` are
normative over DRIVERS.md prose (see CLAUDE.md).

## The backtest harness

`run_backtest(company, horizon=3, n=10_000, seed=7)` in `financial_analysis/backtest.py`
— walk-forward vintages, one per clean fiscal-year-end + 90 days:

- **Point-in-time estimates**: a vintage sees only periods with `filed_date <= as_of`.
  Because the importer's `filed_date` is the filing date of the (often later) 10-K whose
  comparative column supplied the figure, vintages are *conservative* — they may see
  fewer periods than a real-time observer did, never more. The leakage direction is safe.
- **Mechanical parameters, no judgment**: trailing revenue CAGR + YoY std, FCF-margin
  mean/std over ≤5 clean annuals (stds floored at 0.01); mirrors `StochasticDCF`'s
  sampling structure (one growth + one margin draw per path).
- **Asymmetric actuals, disclosed**: actuals join from the store *as currently reported*
  (restatements included) — how forecasts are judged in practice.
- **DATA_FLAGS, not fixes**: vintages whose parameter window touches IBM FY2019/FY2020
  carry the recast-mismatch flag; inputs are marked, never silently repaired.
- `_clean_annuals` requires both revenue and OCF — this is what filters the phantom
  10-K rows (HIDDEN-ASSUMPTIONS #2).
- Scope: revenue/FCF projections only — no WACC, no terminal value, no price.

The HTML report renders it as section 6 ("Backtest — mechanical vintages"): coverage +
per-horizon bias chips, revenue/FCF hedgehog charts, vintage scorecard. Calibration to
date: IBM 14/18 realized outcomes in the P10–P90 band (78% vs 80% nominal); DUOL 0/6 —
mechanical extrapolation fails on growth-phase tickers, so treat their bands as
decorative until a driver build exists.

## `financial_analysis/` modules (verified 2026-07-17)

| Module | Lines | What it does |
|---|---|---|
| `__init__.py` | 3 | Re-exports `EdgarClient`. |
| `backtest.py` | 407 | Point-in-time vintage backtest of *operating* projections (revenue, FCF — no WACC, no TV, no price). One vintage per clean fiscal-year-end + 90-day filing lag; `vintage_periods` filters on `filed_date <= as_of` (true point-in-time); mechanical parameters (trailing CAGR, margin mean/std over ≤ 5 clean annuals, std floored 0.01, ≥ 3 periods required); projects +1..+horizon with StochasticDCF-style one-draw-per-path sampling; scores vs actuals as currently reported (asymmetry disclosed); `DATA_FLAGS` surfaces (never fixes) known input defects, e.g. IBM's FY2019–20 recast mismatch; skipped vintages are recorded with reasons. |
| `cache.py` | 28 | Disk cache for SEC companyfacts JSON at `data/raw/TICKER/company_facts.json`; re-fetches on `refresh` or corrupt cache. |
| `dcf.py` | 157 | Classic engine (above): `DCFAssumptions`, `DCFResult`, `StochasticDCF`, `derive_fcf_margin`. Annual (10-K/10-K/A) periods only. |
| `driver_dcf.py` | 713 | Driver-based engine (above): `Driver`, `Item`, `DriverModel`, `SegmentSpec`, `DriverDCF`, `DriverDCFResult` (incl. `driver_draws` per-path values for the path explorer), vetted item formulas `revenue_paths`, `fcff_margin`, `wacc_capm`, `terminal_value`, registered in `ITEMS`. |
| `edgar.py` | 272 | SEC HTTP client (User-Agent, 0.15s rate limit), CIK lookup from company_tickers.json, full-index filing downloads (`EdgarClient.download_for_ticker`), `EdgarClientError`. |
| `estimates.py` | 193 | yfinance **forward** consensus (revenue/EPS estimates, price targets) → `ConsensusEstimates`; persisted as dated snapshots `data/raw/TICKER/estimates/YYYY-MM-DD.json` (yfinance has no history — history accumulates from first pull; empty snapshots saved so absence is dated evidence). `fetch_estimates`, `list_snapshots`, `load_snapshot`. |
| `fmp.py` | 120 | Optional FMP API client (`FMP_API_KEY` in `.env`): analyst estimates/surprises, IFRS statements stub (`NotImplementedError`). Free-plan v3 endpoints unavailable since Aug 2025 — pipeline degrades to yfinance. |
| `importer.py` | 196 | companyfacts → `PeriodFinancials`: maps concepts via `xbrl_tags` (first-wins merge across fallback concepts per period_end, sign flips), collects unmapped us-gaap USD concepts into `raw_extras`, best-effort `filed_date` from revenue-concept `filed` fields, raises `UnsupportedFilingError` for non-us-gaap filers. Module docstring documents why `segment_revenue` is always None from this source (companyfacts strips dimensional facts — Ticket 7). |
| `market_data.py` | 136 | yfinance daily OHLCV (`load_prices`, cached at `data/raw/TICKER/prices.json`) and earnings surprises (`load_earnings_surprises`). |
| `options.py` | 375 | yfinance option-chain snapshots (dated, one per day, `data/raw/TICKER/options/YYYY-MM-DD.json`) + derived per-expiry metrics: ATM IV, straddle-mid implied move, 90/110 skew, put/call volume & OI ratios (`fetch_option_chain`, `compute_chain_metrics`, `list_snapshots`, `load_snapshot`). |
| `rates.py` | 677 | US rates and inflation from the publishers, with no API key. U.S. Treasury par yield curves: nominal 1M to 30Y from 1990, real (TIPS) 5Y to 30Y from 2003. New York Fed SOFR, EFFR, OBFR, TGCR, BGCR and the FOMC target range. BLS CPI-U, headline and core. Breakeven inflation and 5y5y forward inflation use the FRED formulas. `cross_check` and `cross_check_inflation` compare each value with FRED. `with_risk_free` sets the `rf` driver mean of a `DriverModel`. Values are decimals. Cache: `data/raw/rates/`. CLI with `--json`: `python -m financial_analysis.rates`. |
| `bonds.py` | 300 | US corporate bond data, with no API key, through the FRED CSV: ICE BofA option-adjusted spread and effective yield for each rating (AAA to CCC; FRED keeps only 3 years), Moody's seasoned Aaa and Baa yields (daily from 1983 and 1986), and the U.S. Treasury HQM corporate spot curve (monthly from 1984). `rating_bucket` maps S&P and Moody's ratings to an index. `spread_over_10y` = index yield minus UST 10Y, the engine's `kd_spread`. `with_kd_spread` sets that driver mean. A value more than 7 days old (70 days for HQM) raises an error. `cross_check_bonds` compares Moody's minus UST 10Y with FRED AAA10Y and BAA10Y. Cache: `data/raw/bonds/`. CLI with `--json`: `python -m financial_analysis.bonds`. |
| `scenarios.py` | 125 | Bear/base/bull presets (`DEFAULT_SCENARIOS` generic mature-large-cap; `_SCENARIO_OVERRIDES["IBM"]` = v3 framework), `CAPITAL_OVERRIDES` (IBM: net debt $54.0B, 948,675,228 diluted shares), `capital_structure_for` (derives net debt from latest 10-K: long_term_debt + short-term borrowings from raw_extras − cash; shares_outstanding falling back to diluted). |
| `schema.py` | 187 | Dataclasses: `PeriodFinancials` (~60 statement fields + `remaining_performance_obligation`, reserved `segment_revenue`, `raw_extras`; derived `free_cash_flow = OCF − abs(capex)` and `fcf_margin`), `CompanyFinancials`, `PricePoint`, `EarningsEstimate`, `EarningsSurprise`, `OptionQuote`, `OptionChainSnapshot`, `ConsensusEstimates`. |
| `store.py` | 105 | JSON persistence `data/processed/TICKER.json` (**periods only** — no market data), in-memory session cache, `get_series`/`compare` query helpers with metric-name validation. |
| `xbrl_api.py` | 74 | companyfacts fetch + series extraction: filters by form type/unit, deduplicates by end date keeping the **latest-filed** entry, returns last `n` entries. |
| `xbrl_tags.py` | 173 | The concept registry: `TagSpec` (fallback concept lists, unit, sign_flip) for income statement / balance sheet / cash flow / disclosure (RPO) fields, plus `STATEMENT_MAP` and `UNIT_MAP`. Inline comments document precedence rationales (interest_income, dividends_paid). |
| `verdict.py` | 31 | Investor verdict, read-only over driver-engine paths: chance the price is too high (share of paths valued below the price) and a five-band verdict (25%/75% outer cutoffs, Morningstar-style; 40%/60% middle cutoffs are judgment). Plan: PLANNING/proposals/2026-10-15-investor-verdict-layer.md. |

## `scripts/` (verified 2026-07-17)

| Script | CLI (verified against argparse) | What it does |
|---|---|---|
| `analyze.py` | `python -m scripts.analyze TICKER [--refresh] [--out PATH] [--no-market] [--no-options] [--no-estimates]` | Full pipeline: import → attach prices/surprises/consensus/options → write `data/processed/TICKER_financials.xlsx`. The workbook **always contains all nine sheets** (Income Statement, Balance Sheet, Cash Flow, Raw, Prices, Estimates, Estimates Fwd, Options, Options Summary); skipped groups leave header-only sheets, and the console "N tabs" count refers to populated groups. RPO rows share the Balance Sheet tab with `statement=disclosure`. |
| `import_financials.py` | `python -m scripts.import_financials TICKER [--refresh]` | Import + save JSON only, no xlsx. |
| `import_edgar.py` | `python -m scripts.import_edgar TICKER VALUATION_DATE [--output-dir data/raw] [--lookback-months 18] [--lookahead-months 6]` | Bulk full-index filing download (archival fallback path — companyfacts is the primary parse path). |
| `run_dcf.py` | `python -m scripts.run_dcf TICKER [--simulations 10000] [--years 10] [--growth 0.15] [--wacc 0.10] [--terminal-growth 0.03] [--seed N]` | Classic-engine MC DCF on stored data; no network. |
| `backtest.py` | `python -m scripts.backtest TICKER [--horizon 3] [--simulations 10000] [--seed 7]` | Vintage backtest console report: per-vintage P10/P50/P90 vs actuals, band coverage vs nominal 80%, signed bias by horizon, data flags, skipped vintages. No network. |
| `visualize_dcf.py` | `python -m scripts.visualize_dcf TICKER [--price X] [--out PATH]` | Single-page self-contained HTML report (plotly.js inlined, ~5.9k lines of output): header chips, price & valuation, fundamentals, the four aggregate-DCF panels (fan / distribution / waterfall / tornado), the driver-model section (segment history, driver table, correlation heatmap, driver-vs-aggregate compare, attribution tornado, **MC path explorer**: stratified 301-path spaghetti, 2,000-path parallel coordinates, bottom/top-10 outlier table with \|z\|≥1.5 drivers bolded), options section (IV term structure, implied move, skew, OI heatmap). Driver section renders only if `analysis/<TICKER>/driver_model.py` exposes `build()`; every section degrades to a labeled "not available" note. Styling follows docs/design-specs/2026-07-16-ratings-desk-blueprint.md. The sixth section "Backtest — mechanical vintages" (vintage table + hedgehog charts from `financial_analysis/backtest.py`, honesty labels, merged PR #9) makes SECTION_TITLES six entries. The skew panel excludes expiries under 2 days from the snapshot (`_SKEW_MIN_DTE`, PR #14) — near-expiry wing IVs are solver noise. |
| `verdict.py` | `python -m scripts.verdict TICKER --price X [--simulations 10000] [--seed 7] [--rf-date D] [--rf-base-date D] [--rating R] [--json]` | Console verdict for a ticker with `analysis/<TICKER>/driver_model.py`: P10/P25/P50/P75/P90, chance the price is too high, verdict band. It writes nothing. Without `--rf-date` it uses no network. `--rf-date` sets the `rf` mean to the Treasury 10Y of that date and prints the real and inflation parts of the rate. `--rating` (with `--rf-date`) sets the `kd_spread` mean to the market spread for the rating. `--json` gives machine output for `rust/`. |
| `trade_backtest.py` | `python -m scripts.trade_backtest [BOT ...] [--fetch] [--max-risk DOLLARS] [-v]` | Backtest of the SPY/QQQ option bots in `trading/`; prints a leaderboard. Option prices are model prices. |
| `trade_paper.py` | `python -m scripts.trade_paper [BOT ...] [--no-refresh] [--no-live] [--equity X] [--max-risk DOLLARS]` | One forward paper-trade step for each bot, with live-chain prices when available. It sends no broker orders. |

## Root-level analysis entry points

| Entry point | What it does |
|---|---|
| `python -m analysis.IBM.dcf_scenarios` | v3 aggregate framework console report: FCFF v3 margins, paired (growth, f) scenarios, WACC grid, f-locus. Historical — superseded by the driver run for headline numbers, kept reproducible. |
| `python -m analysis.IBM.dcf_drivers_run` | v4 driver-engine runner: headline P10/P50/P90 + attribution, E/V fixed-point iteration, f-locus (g_software mean moves WITH f), SBC-drift locus, beta locus, supply-shock correlation and τ=0 disclosure runs, implied-reinvestment check. Seed 7, 10k sims, 10y. |
| `python -m analysis.JNJ.first_pass_run` | JNJ first pass (unreviewed register): headline, engine-vs-hand-DCF cross-check at means, f / beta / rf loci and one stacked case. Seed 7, 10k sims. |

## `trading/` (added 2026-11-19 from branch `claude/trading-bot-spy-qqq-xl1o2o`)

A research package for SPY/QQQ option strategies. It is independent of
`financial_analysis/`; no module imports across the two packages.

| Module | Function |
|---|---|
| `data.py` | Loads bars, volatility indexes and rates from yfinance into a CSV cache that keeps history. |
| `pricing.py` | Option pricer. The default is a model price: Black-Scholes on VIX/VXN/VIX1D with skew and spread. `LiveChainPricer` uses live bid/ask for new signals. |
| `structures.py` | Option structures with a known maximum loss at entry. |
| `strategy.py` | The `Strategy` and `Context` interface that each bot implements. |
| `engine.py` | Event-driven backtester with account risk rules (size, pattern-day-trader limit). |
| `metrics.py` | Performance statistics and the in-sample / out-of-sample split. |
| `registry.py` | Finds the bots in `trading/strategies/`. |
| `paper.py` | Forward paper trading. It sends no broker orders. |
| `strategies/` | 22 bots (s01 to s21 and s14b) and a template. |
| `research/` | One note for each bot, and `LEADERBOARD.md` with the results and limits. |

Limit: no free source of historical option quotes exists, thus each backtest result is a
model result.

## Tests

`python -m pytest` from the repo root: 390 tests, all pass on 2026-11-18 (3 s).
22 test modules are in `tests/`. The suite makes no network calls; HTTP is mocked and
files use `tmp_path`. Install `pytest` with `pip install -r requirements-dev.txt`.
