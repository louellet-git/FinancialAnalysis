# Architecture — code-verified data flow and module map

Verified against the code on 2026-07-17 (main lineage after the factory-builds and
handoff merges, PRs #7/#8; full test suite run the same day). Where a
document and the code disagree, the code governs. Companion docs: `docs/HIDDEN-ASSUMPTIONS.md`
(trap registry), root `CLAUDE.md` (agent onboarding + normativity map), `docs/INDEX.md`
(full doc catalogue).

## Data flow

```
SEC EDGAR companyfacts (data.sec.gov/api/xbrl/companyfacts/CIK##########.json)
        │  edgar.py (CIK lookup, rate-limited HTTP) + xbrl_api.py (fetch + dedupe)
        ▼
cache.py → data/raw/TICKER/company_facts.json          (disk cache; --refresh overwrites)
        │  importer.py: xbrl_tags.py TagSpec registry → PeriodFinancials
        ▼
store.py → data/processed/TICKER.json                  (periods ONLY — no prices/options/estimates)
        │
        ├── dcf.py  StochasticDCF        (aggregate engine)  ──┐
        ├── driver_dcf.py  DriverDCF     (driver engine)     ──┼── scripts + analysis runners
        └── backtest.py  run_backtest    (vintage harness)   ──┘
        ▼
outputs: data/processed/TICKER_financials.xlsx (9 tabs)  ·  TICKER_dcf.html (visualizer)
         analysis/IBM/report/*.docx (report pipeline)

yfinance (side channel, never persisted into TICKER.json):
  prices        → data/raw/TICKER/prices.json                    (full history, cached)
  earnings      → data/raw/TICKER/yf_earnings.json               (surprises)
  option chain  → data/raw/TICKER/options/YYYY-MM-DD.json        (dated snapshots — current chain only;
                                                                  history accumulates from first run)
  fwd consensus → data/raw/TICKER/estimates/YYYY-MM-DD.json      (dated snapshots, estimates.py)
FMP (optional, paid key in .env) → fmp.py estimates/surprises; IFRS support is a stub.
```

Branch note (2026-07-17): the backtest harness, `DriverDCFResult.driver_draws`, and the
MC path-explorer visualizer section landed via branch `factory-builds` (commits
`8153323`, `d226dab`), merged to main as PR #7 the same day. If you are on an older
branch that predates that merge, `python -m scripts.backtest` will not exist — check
`git log --all -- <path>` before concluding something is missing.

## Module map (one line each, verified)

| Module | What it actually does |
|---|---|
| `financial_analysis/cache.py` | Disk cache for companyfacts JSON under `data/raw/TICKER/`; `refresh=True` re-fetches; corrupt cache self-heals |
| `financial_analysis/edgar.py` | CIK lookup (`company_tickers.json`), rate-limited `_http_get` with a personal User-Agent, bulk full-index filing downloads (`EdgarClient`, used by `scripts/import_edgar.py` only) |
| `financial_analysis/xbrl_api.py` | companyfacts fetch + `extract_series`; `_deduplicate` keeps the **latest-filed** entry per `end` date (source of the filed_date lag — see HIDDEN-ASSUMPTIONS #3) |
| `financial_analysis/xbrl_tags.py` | `TagSpec` concept registry (concepts, fallbacks, `sign_flip`, unit) for ~50 mapped fields incl. `dividends_paid`, `interest_income`, `remaining_performance_obligation` (RPO, per rule R1) |
| `financial_analysis/importer.py` | Orchestrates facts → `PeriodFinancials`; merges concept fallbacks per period; collects unmapped us-gaap USD concepts into `raw_extras`; documents that companyfacts has NO segment-axis data (Ticket 7) |
| `financial_analysis/schema.py` | Dataclasses: `PeriodFinancials` (IS/BS/CF/disclosure slots + `raw_extras`), `CompanyFinancials`, `PricePoint`, `EarningsSurprise`, `OptionChainSnapshot`, `ConsensusEstimates`; `free_cash_flow = OCF − abs(capex)` (capex stored sign-flipped) |
| `financial_analysis/store.py` | JSON persistence of **periods only** + in-session cache (`_CACHE`); `get_series`/`compare` query helpers; prices/options/estimates are NOT round-tripped |
| `financial_analysis/market_data.py` | yfinance daily OHLCV + earnings-surprise loader, disk-cached |
| `financial_analysis/options.py` | yfinance option-chain **dated snapshots** + derived metrics (ATM IV, implied move = ATM straddle/spot, 90/110 skew, put/call ratios); `list_snapshots`/`load_snapshot` |
| `financial_analysis/estimates.py` | yfinance forward consensus (revenue/EPS estimates, price targets) as dated snapshots; absence degrades to empty dicts, and the empty snapshot is still saved as dated evidence |
| `financial_analysis/fmp.py` | Optional FMP client (`FMP_API_KEY` in `.env`); analyst estimates/surprises; IFRS statements raise `NotImplementedError` |
| `financial_analysis/dcf.py` | **Aggregate engine**: `StochasticDCF` — independent normal draws of growth/margin/WACC/tg |
| `financial_analysis/scenarios.py` | Bear/base/bull presets + per-ticker registries (`_SCENARIO_OVERRIDES`, `CAPITAL_OVERRIDES`); seed 7; **IBM entries are v3-era values** (see below) |
| `financial_analysis/driver_dcf.py` | **Driver engine**: `Driver`/`DriverModel`/`SegmentSpec`/`DriverDCF` — correlated driver sampling, vetted item formulas, T-DCF-2 terminal guard |
| `financial_analysis/backtest.py` | Point-in-time vintage backtest of mechanical operating projections (revenue/FCF only; no WACC, no TV) |
| `scripts/analyze.py` | Full pipeline → 9-tab xlsx (primary entrypoint); `--refresh/--no-market/--no-options/--no-estimates/--out` |
| `scripts/import_financials.py` | Import + save JSON only, no xlsx |
| `scripts/import_edgar.py` | Bulk raw-filing download via full-index (archival fallback path; rarely used) |
| `scripts/run_dcf.py` | CLI for the aggregate engine on stored data (no network) |
| `scripts/visualize_dcf.py` | Single-page interactive HTML company report (plotly inlined, offline-viewable); sections degrade gracefully |
| `scripts/backtest.py` | CLI: vintage table with P10/P50/P90 vs actuals, error %, band coverage, data flags |
| `scripts/setup-officecli.sh` | SessionStart hook target: CHECK-ONLY officecli presence (installs only with explicit `--install`) |
| `scripts/memo-review-reminder.sh` | PostToolUse(Bash) hook: fires (exit 2) whenever a command produces a `.docx` — demands a memo-reviewer pass |
| `analysis/IBM/dcf_scenarios.py` | v3 console report (aggregate engine): scenarios, f-locus, WACC grid, F19 check — **derives margins live from the store** (see drift warning below) |
| `analysis/IBM/driver_model.py` | IBM v4 driver register — **normative assumption source** (rationale strings govern) |
| `analysis/IBM/dcf_drivers_run.py` | v4 runner: headline, E/V fixed point, f/SBC/beta loci, disclosure runs, implied-reinvestment check |

## The TWO engines — do not conflate them

**1. Aggregate engine — `dcf.py` `StochasticDCF` (+ `scenarios.py` presets).**
Samples four top-level items directly and independently (normal draws): revenue growth,
FCF margin, WACC, terminal growth. One draw per path held constant for all 10 years.
Gordon terminal value with a raw `clip(wacc − tg, 0.001)` denominator (the clip that
motivated T-DCF-2). Reports mean/std (the driver engine deliberately does not). Margin
derived from the last 5 annual periods when unset. Min-history guard: ≥4 annual periods
with positive FCF. Used by `run_dcf.py`, the visualizer's scenario panels, and the IBM
v3 record.

**`scenarios.py` registry status (flagged honestly):** the IBM `_SCENARIO_OVERRIDES`
(margins 9.2/14.5/16.1%) and `CAPITAL_OVERRIDES` (net debt $54.0B, 948,675,228 diluted
shares) are **v3-era values frozen 2026-07-16**, while `analysis/IBM/driver_model.py` is
the consolidated v4 register. They are kept for the visualizer's aggregate panels and the
v3 record; the v4 driver run is the normative IBM valuation.

**Store-drift warning (verified 2026-07-17):** `analysis/IBM/dcf_scenarios.py` derives
FCFF margins from the *current* `data/processed/IBM.json` at each run. The p1-importer
re-import changed derived margins (mean 17.8% → 18.4%), so a fresh run now prints
Bear $43 / Base $149 / Bull $243 — the *published, signed-off* v3 record remains
**$37 / $141 / $232** (README, REVIEW.md). Neither is a bug: the record is frozen, the
script is live. Quote the record for v3 history; quote the engine for current numbers.

**2. Driver engine — `driver_dcf.py` `DriverDCF` (normative for IBM v4).**
No top-level item is ever sampled; every item is a vetted deterministic formula over
underlying *drivers*, and only drivers are stochastic. Mechanics, plainly:

- **Parameter-MC, not process-MC.** One joint draw per path: the 8 stochastic drivers
  of the IBM model (g_software, g_consulting, g_infrastructure, core_margin, ma_ratio,
  rf, erp, terminal_growth) are drawn once as a correlated 8-dim vector
  (Cholesky-factored full pairwise correlation matrix, `mean + std·z`), and each value
  is **held constant across all 10 projection years of that path**. There is no
  year-to-year dynamics — no AR(1), no mean reversion (ticket T-DCF-1, the single
  largest known distortion). Deterministic drivers are constants or per-year
  `schedule`s ((years,1) columns that broadcast against the (n,) draws — e.g. the
  sbc_ratio ramp and margin_mix_drift).
- **Clip-after-correlation (disclosed).** `dist="truncnorm"` bounds are applied by
  clipping *after* the correlated transform — probability mass piles on the bounds and
  realized correlations are slightly distorted. Deliberate; acceptable only while
  bounds sit ≳2σ from means (the module docstring tells reviewers to check).
- **Per-path WACC.** CAPM per path from the drawn rf and erp with deterministic beta,
  weights and tax rate; **kd is derived per path** as `(rf + kd_spread) × (1 − tax)` so
  credit spreads cannot go negative when rf floats. **Flat term structure**: one WACC
  per path discounts years 1–10 and the terminal value.
- **Terminal guard (T-DCF-2).** Effective tg per path = `min(tg_draw, wacc − 2%)` — the
  Gordon denominator is ≥2% on every path; the binding frequency is reported as
  `tg_pinned_fraction` (0.0009 at seed 7). Percentiles only — mean/std deliberately
  absent from `DriverDCFResult` (fat tails make moments misleading).
- **Attribution.** ±1σ mean-shift per stochastic driver with the **same seed** (identical
  z-draws, correlations intact) → path-comparable P50 deltas; feeds the tornado.
- **Year-1 pins.** `SegmentSpec.year1_growth` pins year 1 to known actuals (IBM: the
  Q2-26 warning — SW +5%, Consulting 0%, Infra −1.69% on the haircut base, Financing 0%);
  draws compound from year 2.
- **`driver_draws`**: the result exposes the per-path post-clip draws for every
  stochastic driver (deterministic drivers excluded), powering the path-explorer
  visualizer section. Defaults empty for backward compatibility.

IBM v4 headline (reproduced 2026-07-17, `python -m analysis.IBM.dcf_drivers_run`, seed 7,
10k sims): **P10 $83.84 / P50 $151.84 / P90 $258.03**, TV/EV at means 64.1%, tg pinned
0.09% — **conditional on f = MA_RECURRENCE = 0.50** (see CLAUDE.md and
HIDDEN-ASSUMPTIONS #7).

## The backtest harness

`run_backtest(company, horizon=3, n=10_000, seed=7)` — walk-forward vintages, one per
clean fiscal-year-end + 90 days:

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

## The xlsx (always 9 tabs)

`write_xlsx` always creates all nine sheets — Income Statement, Balance Sheet, Cash Flow,
Raw, Prices, Estimates, Estimates Fwd, Options, Options Summary — skipped data just
leaves a sheet empty (header row only). The console "N tabs" message counts populated
groups and can read 4–9; the workbook sheet count does not vary. Financial tabs are long
format, one row per (period, metric); RPO rows appear on the Balance Sheet tab with
`statement=disclosure`.

## Review pipeline (L5/L7 of the factory)

`.claude/agents/`: `valuation-analyst` (drafts, REVIEW.md dispositions bind it) ↔
`valuation-skeptic` (reproduces every number before judging) → `memo-reviewer`
(craft rubric, target=good; reviews a COPY because officecli mutates .docx on open) →
`report-auditor` (claim→source taxonomy FILING/MARKET/PRESS/MODEL/ASSUMPTION/
INTERPRETATION; softens over-claims; PASS required). Review scopes per the vision doc:
**frontier** (full fan-out, new mappings), **charter** (every run, assumptions-as-package),
**exception** (validator/staleness/provenance triggers — the RPO re-look was the first).
