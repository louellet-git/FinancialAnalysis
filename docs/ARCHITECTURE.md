# Architecture — code-verified module map

Verified against the code on 2026-07-17 (branch `docs-overhaul`, based on origin/main
`9751523`). Every claim below was checked by reading the named module. When this doc and
the code disagree, the code governs.

## Data flow

```
SEC EDGAR (companyfacts JSON API)          yfinance
        │                                     │
   rust/mapcheck (Rust: fetch, map,      market_data.py (prices, surprises)
   check, store file)                    estimates.py   (forward consensus snapshots)
        ▼                                options.py     (option-chain snapshots)
   importer.py (runs mapcheck, reads the store file)
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
- `_clean_annuals` requires both revenue and OCF.
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
| `dcf.py` | 157 | Classic engine (above): `DCFAssumptions`, `DCFResult`, `StochasticDCF`, `derive_fcf_margin`. Annual (10-K/10-K/A) periods only. |
| `driver_dcf.py` | 713 | Driver-based engine (above): `Driver`, `Item`, `DriverModel`, `SegmentSpec`, `DriverDCF`, `DriverDCFResult` (incl. `driver_draws` per-path values for the path explorer), vetted item formulas `revenue_paths`, `fcff_margin`, `wacc_capm`, `terminal_value`, registered in `ITEMS`. |
| `edgar.py` | 272 | SEC HTTP client (User-Agent, 0.15s rate limit), CIK lookup from company_tickers.json, full-index filing downloads (`EdgarClient.download_for_ticker`), `EdgarClientError`. |
| `estimates.py` | 193 | yfinance **forward** consensus (revenue/EPS estimates, price targets) → `ConsensusEstimates`; persisted as dated snapshots `data/raw/TICKER/estimates/YYYY-MM-DD.json` (yfinance has no history — history accumulates from first pull; empty snapshots saved so absence is dated evidence). `fetch_estimates`, `list_snapshots`, `load_snapshot`. |
| `fmp.py` | 120 | Optional FMP API client (`FMP_API_KEY` in `.env`): analyst estimates/surprises, IFRS statements stub (`NotImplementedError`). Free-plan v3 endpoints unavailable since Aug 2025 — pipeline degrades to yfinance. |
| `importer.py` | 100 | No mapping logic. Runs the Rust program `mapcheck import` (it gets the SEC data if it is not in the cache) and reads the store file. Raises `UnsupportedFilingError` for non-us-gaap filers and `MapcheckError` if the Rust program is not built. |
| `market_data.py` | 136 | yfinance daily OHLCV (`load_prices`, cached at `data/raw/TICKER/prices.json`) and earnings surprises (`load_earnings_surprises`). |
| `options.py` | 375 | yfinance option-chain snapshots (dated, one per day, `data/raw/TICKER/options/YYYY-MM-DD.json`) + derived per-expiry metrics: ATM IV, straddle-mid implied move, 90/110 skew, put/call volume & OI ratios (`fetch_option_chain`, `compute_chain_metrics`, `list_snapshots`, `load_snapshot`). |
| `rates.py` | 677 | US rates and inflation from the publishers, with no API key. U.S. Treasury par yield curves: nominal 1M to 30Y from 1990, real (TIPS) 5Y to 30Y from 2003. New York Fed SOFR, EFFR, OBFR, TGCR, BGCR and the FOMC target range. BLS CPI-U, headline and core. Breakeven inflation and 5y5y forward inflation use the FRED formulas. `cross_check` and `cross_check_inflation` compare each value with FRED. `with_risk_free` sets the `rf` driver mean of a `DriverModel`. Values are decimals. Cache: `data/raw/rates/`. CLI with `--json`: `python -m financial_analysis.rates`. |
| `bonds.py` | 300 | US corporate bond data, with no API key, through the FRED CSV: ICE BofA option-adjusted spread and effective yield for each rating (AAA to CCC; FRED keeps only 3 years), Moody's seasoned Aaa and Baa yields (daily from 1983 and 1986), and the U.S. Treasury HQM corporate spot curve (monthly from 1984). `rating_bucket` maps S&P and Moody's ratings to an index. `spread_over_10y` = index yield minus UST 10Y, the engine's `kd_spread`. `with_kd_spread` sets that driver mean. A value more than 7 days old (70 days for HQM) raises an error. `cross_check_bonds` compares Moody's minus UST 10Y with FRED AAA10Y and BAA10Y. Cache: `data/raw/bonds/`. CLI with `--json`: `python -m financial_analysis.bonds`. |
| `scenarios.py` | 125 | Bear/base/bull presets (`DEFAULT_SCENARIOS` generic mature-large-cap; `_SCENARIO_OVERRIDES["IBM"]` = v3 framework), `CAPITAL_OVERRIDES` (IBM: net debt $54.0B, 948,675,228 diluted shares), `capital_structure_for` (derives net debt from the latest 10-K: standard variable `total_debt` − cash; shares_outstanding falling back to diluted). |
| `schema.py` | 187 | Dataclasses: `PeriodFinancials` (132 standard variables with the same names as the registry, `sources` = the tag or formula behind each value, reserved `segment_revenue`, `raw_extras`; derived `free_cash_flow = OCF − abs(capex_total)` (rule R2: equipment and capitalized software) and `fcf_margin`), `CompanyFinancials`, `PricePoint`, `EarningsEstimate`, `EarningsSurprise`, `OptionQuote`, `OptionChainSnapshot`, `ConsensusEstimates`. |
| `store.py` | 105 | JSON persistence `data/processed/TICKER.json` (**periods only** — no market data), in-memory session cache, `get_series`/`compare` query helpers with metric-name validation. |
| `standard_variables.toml` | 1,385 | The registry of standard variables (data): for each variable its alias tags in a fixed sequence, fallback formulas, unit, sign, core flag and memo flag; the rollups; the containment rules; the excluded tags with reasons. The Rust program `rust/mapcheck` uses it to make the variables. |
| `xbrl_tags.py` | 135 | Loads the registry file for the Python code that stays (schema fields, xlsx writer): `ALL_TAGS`, the four statement dicts, `CORE_FIELDS`, `ROLLUPS`, `WITHIN`, `STATEMENT_MAP`, `UNIT_MAP`. It does no mapping. |
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

## Root-level analysis entry points

| Entry point | What it does |
|---|---|
| `python -m analysis.IBM.dcf_scenarios` | v3 aggregate framework console report: FCFF v3 margins, paired (growth, f) scenarios, WACC grid, f-locus. Historical — superseded by the driver run for headline numbers, kept reproducible. |
| `python -m analysis.IBM.dcf_drivers_run` | v4 driver-engine runner: headline P10/P50/P90 + attribution, E/V fixed-point iteration, f-locus (g_software mean moves WITH f), SBC-drift locus, beta locus, supply-shock correlation and τ=0 disclosure runs, implied-reinvestment check. Seed 7, 10k sims, 10y. |
| `python -m analysis.JNJ.first_pass_run` | JNJ first pass (unreviewed register): headline, engine-vs-hand-DCF cross-check at means, f / beta / rf loci and one stacked case. Seed 7, 10k sims. |

## Standard variables and `rust/mapcheck/` (2026-11-19)

Filers use different XBRL tags for the same line. The registry
`financial_analysis/standard_variables.toml` gives each line one standard variable name;
the engines and the analysis code read only these names.

The Rust program `rust/mapcheck` is the only implementation of this work (owner
decision of 2026-11-19: the language of the repo is Rust). The Python importer has no
mapping logic; it runs `mapcheck import` and reads the store file
`data/processed/<TICKER>.json`.

1. **Period.** A row exists for each real fiscal year and quarter, and each value is the
   fact of the correct length.
2. **Alias.** For each variable, the first tag in its list that has a fact gives the
   value. A tag is in a list only if its scope is the same as the variable.
3. **Formula.** If no tag has a fact, a formula over other variables gives the value
   (for example `total_debt = debt_current + long_term_debt`). In a sum, two terms with
   the same value count one time.
4. **Source.** Each value records its tag or its formula (`PeriodFinancials.sources`, the
   `source` column of the xlsx file, `mapcheck values TICKER`).
5. **Checks.** `mapcheck [TICKER ...]` does the checks for one ticker or for all tickers
   in the cache, in parallel.
6. **Filing statements.** `mapcheck filing TICKER` reads the latest 10-K and prints the
   statements as the filer structures them, company tags included.

| Module (`rust/mapcheck/src/`) | Function |
|---|---|
| `registry.rs` | Loads the registry file (put into the program at build time) and checks its rules: one tag in one variable; no formula that mixes memo and line variables; no memo variable in a rollup. |
| `facts.rs` | The companyfacts data and the selection of facts by period length: an annual value covers a full fiscal year, a quarterly value one quarter (or a year-to-date difference); the latest filing wins; end dates of one period that differ by some days become one period. |
| `mapper.rs` | The standard variables of one period: alias tags, formulas, sources, the share-count scale rule, tags with no variable. |
| `checks.rs` | The checks: core inputs, signs, identities, rollups (overlap, nested details, lines with no variable), total-debt estimates, share count, capex and software, period sequence. |
| `run.rs`, `summary.rs` | One ticker, and all tickers in parallel (rayon); the JSON report; the text summary, the ticker list, the detail and the values listing. |
| `http.rs`, `sec.rs` | HTTP with a delay between requests; the SEC ticker list, company facts and filing files, and the cache folders. The contact text for the SEC comes from the environment variable `SEC_USER_AGENT`. |
| `store.rs` | Writes `data/processed/<TICKER>.json` for the Python valuation code. |
| `filing.rs` | Filing-level XBRL, first part: reads `MetaLinks.json` and the instance document of the latest 10-K, builds the three statements as trees from the calculation data of the filer, and gives capex, depreciation and amortization, and total debt from the filing lines. It keeps the three fiscal years of the 10-K and the facts with one segment member. |
| `supplement.rs` | Gives a value from the 10-K for a critical input (revenue, capex, share count) that the company facts do not have: company-tag capex lines, a standard tag with segment members, share classes. The source text of the value starts with `filing`. |
| `crosscheck.rs` | Compares the standard variable with the value from the filing lines for capex, depreciation and amortization, and total debt. No outside data source. |
| `cli.rs`, `main.rs` | The command line: `check` (default), `fetch`, `import`, `values`, `filing`, `crosscheck`. |

Results, the list of checks and the limits: `rust/mapcheck/README.md`.

The crate `rust/` (the `fa` command) is a different program: it calls the Python code
and gives typed results. It does not move a part to Rust.

## `fa recon bonds`: bond values from our rates against market prices (2026-11-19, branch `claude/bond-recon`)

The command `fa recon bonds [--date D] [--out FILE]` (crate `rust/`) finds the
difference between the value that our rates give and the market value of simple
bonds. It lists differences. It does not explain them. Open questions are in
`docs/HIDDEN-ASSUMPTIONS.md` (#60 to #67).

Steps for one date:

1. Get the Treasury par curve of the date (`Fa::curve`, Python code).
2. Bootstrap the par yields to zero rates (`rust/src/curve.rs`).
3. Get the end-of-day prices of Treasury bills, notes and bonds (`rust/src/prices.rs`).
4. Price each instrument on the curve (`rust/src/recon.rs`).
5. Print the summary and write one CSV row for each instrument. A second file
   `<name>_curve.csv` has the curve nodes.
6. Compare the HQM corporate spot curve with our zero curve plus the ICE spread.

| Module (`rust/src/`) | Function |
|---|---|
| `curve.rs` | The only code for curve mathematics: dates, business days, par to zero, monotone convex interpolation, discount factors, prices and yields of bills and coupon bonds. |
| `prices.rs` | The FedInvest end-of-day prices: a session with the web form of TreasuryDirect, the CSV parser, the cache `data/raw/prices/`. |
| `recon.rs` | The filter, the pricing of each instrument, the summary, the files, the corporate curve rows. |

Conventions (each one is in the header of `curve.rs`):

- **Par yields.** Semi-annual bond-equivalent yields. A par yield is the coupon rate of a bond that prices to 100 on the curve date.
- **Nodes.** The tenors up to 6 months are bills with simple interest: 28, 42, 56, 91, 119 and 182 days (the 4, 6, 8, 13, 17 and 26 week bills that the Treasury uses as inputs). The tenors of 1 year and more are par coupon bonds with coupons at 6, 12, 18, ... calendar months from the curve date.
- **Time.** Actual days divided by 365. A zero rate is continuously compounded.
- **Interpolation.** Monotone convex on the forward rates (Hagan and West, 2006). The Treasury says that it uses this method since 2021-12-06. The Treasury does not publish its program. Our curve is a re-creation. After 30 years the forward rate is flat. The coupon nodes are solved by sweeps until each par bond prices to 100 (error below 1e-13).
- **Settlement.** T+1 business day. Saturday, Sunday and the Federal Reserve holidays are not business days.
- **Price.** The FedInvest END OF DAY column: a clean price per 100. A bill has no accrued interest. A note or a bond has accrued interest of actual/actual (days between the two coupon dates). The coupon dates come from the maturity date, backward in steps of 6 months. A maturity at the end of a month gives coupon dates at the end of the month.
- **Yield.** For a note or a bond: semi-annual, with a fractional first period; simple interest when one coupon is left. For a bill: bond-equivalent yield, actual/365, quadratic formula above 182 days.
- **Difference.** Observed minus theoretical. A positive price difference means that the market price is above the price from our curve.
- **Filter.** The set has market-based bills, notes and bonds that have a price, have 7 days or more to maturity and have no call date. TIPS and floating-rate notes are not in the set.
- **Resolution.** The source gives notes and bonds in 64ths of a point. The column `resolution_bp` is the yield change of one such step. A yield difference that is smaller than this column has no information. This is large for instruments with a short life.
- **Corporate curve.** No free official source of prices of single corporate bonds is known (see #65). Thus the comparison is at curve level: the HQM spot rate (semi-annual) against our Treasury zero rate (semi-annual) plus the mean ICE OAS of AAA, AA and A. It shows how our spread data relate to the Treasury HQM curve. It does not show a bond price difference.

The tests (`cargo test --manifest-path rust/Cargo.toml --locked`) use small samples and hand calculations: a par bond prices to 100 on a coupon date, a bill, a bond between coupon dates, the reprice of each par node, a flat curve. They do not use the network.
## Bond research: bill tests, fitted curve, rich and cheap, history (2026-11-19, branch `claude/bond-arb`)

Three commands continue `fa recon bonds` (crate `rust/`). They give candidates for study.
They do not give arbitrage and they send no order. Traps: `docs/HIDDEN-ASSUMPTIONS.md` #69, #100 to #106 and #107 to #115.

| Command | Question | Module (`rust/src/`) |
|---|---|---|
| `fa recon bills [--date D]` | Which change makes the bill difference of `recon bonds` go to 0? | `billdiag.rs` |
| `fa recon rich-cheap [--date D] [--out F]` | Which securities are far from a curve that is fitted to the securities? | `fit.rs`, `richcheap.rs` |
| `fa recon history [--from D] [--to D] [--out F]` | Does a difference last? | `history.rs` |
| `fa recon explain [--date D] [--out F]` | Which known features explain a difference? What stays? | `refdata.rs`, `explain.rs` |

**Bill tests.** The command changes one thing at a time: the basis of the bill nodes, the
settlement day, the price field, the curve input. It prints the mean bill difference (bp)
for each change. The Treasury publishes "Daily Treasury Bill Rates" (coupon equivalent of
the on-the-run bills). The command compares them with the bill nodes of the par curve.
`curve.rs` has `bootstrap_with` and `BillBasis` for the test; `bootstrap` is unchanged.

**Fitted curve (`fit.rs`).** Nelson-Siegel-Svensson curve (6 parameters), fitted to the clean
prices of the notes and bonds of one date. Weight of a security: 1 / (dirty price x modified
duration), so each security is one yield observation. Levenberg-Marquardt with a numeric
Jacobian; the start is a grid of tau1 and tau2 with least-squares betas. Three passes leave out
the securities that are more than 4 robust sigma from the curve (MAD). The fit set is: notes and
bonds with a life of 3 months or more, no call date, a price, a coupon. Left out: bills, TIPS,
FRN, callable securities, securities under 3 months, and the outliers. We chose NSS because
the Federal Reserve Board (GSW) uses the same family: its curve is an independent check. A
spline would follow the local errors that we want to see.

**Residual (`richcheap.rs`).** `res_bp` = observed yield - fitted yield. Positive = cheap.
`z` = the residual against the median of the 20 securities nearest in maturity, in robust
sigma (MAD, floor 0.3 bp). The cost is the larger of: the bid-ask of the file (BUY - SELL, in
yield), else the median of the file in the same maturity bucket; and one price step of the
source (1/64). A candidate has |res_bp| larger than the cost and larger than the fit RMSE of the
day, |z| of 2 or more, and a life of 3 months or more. Groups (first rule that fits):
bill; under 3 months; recent issue (life within 0.35 year below 2, 3, 5, 7, 10, 20 or 30 years and
a coupon within 0.5 point of the yield); old 30-year bond (a bond that matures before 2040);
high or low coupon (1.25 points or more from the median coupon of the 20 neighbours); near a
coupon date (next coupon in 10 days or less); other. The output says, for each group, what the
data cannot show.

**History (`history.rs`).** Fits each cached date (parallel with rayon) and keeps the residual of
each note and bond of 6 months or more. It measures: pooled autocorrelation at 1, 5 and 10
business days (raw, and within = mean of the security removed); the half-life from the lag
coefficient; the residual k days after a large value (the 10 percent most cheap and most rich of a
date). It compares our curve with the GSW curve on each date that has a GSW row.

**Data.** The price files are `data/raw/prices/fedinvest_<date>.csv`. `fa recon bonds --date D` downloads
a missing file (the HTTP client was corrected on 2026-11-19, hidden assumption 69). The bill-rate file is
`data/raw/rates/treasury_bill_rates_<year>.csv` (downloaded once). The outputs go to `data/recon/`
(not committed).

**Tests.** Fixed answers: the bill price from a discount rate and the coupon equivalent (against the
Treasury table); the NSS curve by hand; recovery of known NSS parameters from synthetic prices
(to 0.01 bp); one planted bad price is left out; the weight turns a price error into a yield
error; planted rich and cheap securities are found with the right sign; autocorrelation,
half-life and reversal of a series with a known decay; a date gap is not a lag.

**Explain the residuals (`refdata.rs`, `explain.rs`).** `fa recon explain` joins free public data to the
residual of each note and bond (327 securities of 6 months or more on 2026-11-18), runs a cross-section
regression, and prints the residual before and after it. It makes the fit for each cached date (rayon) to get
the mean residual, so it needs the 50 price files. Run time: about 10 s with a warm cache.

| Source | Endpoint | Cache (`data/raw/`) | Gives |
|---|---|---|---|
| NY Fed secured rates | `markets.newyorkfed.org/api/rates/secured/{sofr,bgcr,tgcr}/search.json?startDate&endDate` | `nyfed/secured_<type>_<from>_<to>.json` | General collateral rates by date |
| NY Fed SOMA, Treasury | `.../api/soma/asofdates/list.json`, `.../api/soma/tsy/get/asof/<date>.json` | `nyfed/soma_*.json` | Par held and `percentOutstanding` by CUSIP, weekly |
| Fiscal Data, MSPD table 5 | `api.fiscaldata.treasury.gov/services/api/fiscal_service/v1/debt/mspd/mspd_table_5` | `fiscaldata/mspd_table_5_<month end>.json` | Amount outstanding and amount stripped by CUSIP (thousands of dollars) |
| Fiscal Data, auctions | `.../v1/accounting/od/auctions_query` (notes and bonds, 8 fields) | `fiscaldata/auctions_notes_bonds_<date>.json` | First issue date, original term, reopenings |

The client name is `FinancialAnalysis fa (personal research)`. The wait between two requests is 2 seconds.
The Fiscal Data page says that the data is free, without restriction, and needs no key. The NY Fed
documentation page does not show text without JavaScript; the endpoints were tested by their answers (#108).

Features of one security: coupon minus the par rate of the same maturity on the fitted curve (percentage
points); age since the first issue (years); on-the-run (the CUSIP of the latest auction of its original term);
SOMA share (%); stripped share (%); log of the amount outstanding ($bn); original 30-year bond with 5 to 15
years left; original 30-year bond with less than 5 years left; original 20-year bond; deliverable into a front
Treasury future. The original term comes from the auction record and is rounded to the standard term
(a "29-Year 9-Month" bond is a 30-year bond). The regression is ordinary least squares with an intercept.
The output gives the classical t and a robust (HC1) t. Regression 1 uses the residual of the date. Regression
1b adds the years to maturity. Regression 2 uses the mean residual of all cached dates. Regression 3 leaves
out the two original 30-year groups. Then the command prints: the candidates before and after regression 1;
the 12 largest residuals after it; the nine old 30-year bonds (2036 to 2039) with SOMA share, stripped share,
amount, residual, carry, and a comparison with the note of the nearest maturity; every security with a SOMA
share of 60 % or more; and a test that fits the curve again without the old 30-year bonds.

Futures baskets (front delivery month = the first quarterly month after the date; remaining term from the
first day of that month, rounded down): ZT 2-year: notes with an original term of 5 years 3 months or less,
at least 1 year 9 months left from the first day and at most 2 years from the last day. ZF 5-year: the same
original limit, at least 4 years 2 months left. ZN 10-year: notes, at least 6 years 6 months and less than
8 years left (3-month rounding). TN Ultra 10-year: original 10-year notes, 9 years 5 months to 10 years left.
ZB bond: bonds, at least 15 and less than 25 years left (3-month rounding). UB Ultra bond: bonds, at least
25 years left. The sources are in #113.

**Tests.** The parsers (secured rates, SOMA, MSPD table 5, auctions) use small fixture text. The tests also
check the original-term rounding, the on-the-run rule, the month arithmetic, the front month, each basket
rule, the par rate of a flat curve, and the regression on a planted line (exact coefficients, dropped zero
column, singular matrix, classical standard error against the formula).

### `rust/src/forwards.rs` (2026-11-19)

This module is Rust only. It does not call Python. It uses `mapcheck::http::Client`.

1. `fa forwards [--date D]` gives the zero-coupon yields, the instantaneous forward
   rates, the one-year forward rates and the par yields of the Federal Reserve Board
   dataset of Gürkaynak, Sack and Wright (GSW). Each rate is a decimal. The output has
   the source, the URL, the date of the row and a note.
2. The module reads the published values. It does not compute a curve. The curve
   mathematics is in another part of the repo.
3. The file goes to the cache `data/raw/rates/gsw_feds200628.csv`. A cached file is
   fresh for 12 hours. A new file goes to the cache only after it parses. If the
   download fails and a cache exists, the program uses the old file and prints a warning.
4. `fa forwards check [--days N]` compares the GSW par yields with the Treasury par
   yields (exit 1 on mismatch). It does not use FRED (hidden assumption 59).

### `rust/src/futures/` (2026-11-19)

This module is Rust only. It does not call Python. It uses `mapcheck::http::Client`, with
one shared client and 0.5 s between two requests. The User-Agent text is
`FinancialAnalysis fa (personal research)`.

| File | Function |
|---|---|
| `mod.rs` | Output types, the cache (`load`), and the commands `quote`, `history`, `curve` |
| `facts.rs` | Table of contract facts for each root; reads symbols (`ES`, `ESZ26`) |
| `yahoo.rs` | Yahoo chart endpoint: CME and CBOT contracts (ES, NQ, ZT, ZF, ZN, TN, ZB, UB, SR3, SR1, ZQ) |
| `cboe.rs` | Cboe VIX futures: settlement file of a date and history file of a contract |
| `cftc.rs` | CFTC Commitments of Traders, Traders in Financial Futures |
| `mpt.rs` | Atlanta Fed Market Probability Tracker; a small zip and xlsx reader |
| `check.rs` | Cross-checks (exit 1 on mismatch) |
| `cli.rs` | Arguments of `fa futures ...` |

1. Each value has a date, a source, a URL and a price type. Rates are decimals. Prices
   stay in the units of the exchange. For SR3, SR1 and ZQ the program adds
   `implied_rate = (100 - price) / 100`.
2. The cache is `data/raw/futures/`. A new file goes to the cache only after it parses. A
   file that holds the day that the caller needs is used at any age; other files are fresh
   for 1 hour (Yahoo), 6 hours (Cboe, NY Fed) or 12 hours (CFTC, Atlanta Fed). If a
   download fails and an old file parses, the program uses the old file and prints a warning.
3. Source choice for each kind of data: VIX futures from Cboe (settlement price, a file of
   the exchange); CME and CBOT futures from Yahoo (daily close that Yahoo gives, and the last
   trade); positions from the CFTC; the policy path from the Atlanta Fed. The CME website
   is not used: it refuses automated access (hidden assumption 76).
4. Know the limits before you use a value: hidden assumptions 70 to 78.

### `rust/src/ratemodels/` — stochastic short-rate models (2026-11-19)

Rust only. The module has models, a calibration to a rate history, and a parallel
simulation. It feeds the Monte Carlo DCF (ticket `T-20261119-stochastic-dcf-drivers`).
The library is the main interface. The commands `fa models ...` are thin wrappers.
Rates are decimals per year. Time is in years. All parameters are annualized.

**Files.** `models.rs` (trait `RateModel` and the models), `sim.rs` (`simulate_paths`,
`simulate_map`), `rng.rs` (xoshiro256++, Box-Muller), `data.rs` (`Series`, `Freq`,
`sample`, `Span`), `calib.rs` (`calibrate`), `rolling.rs` (`rolling`, `frequencies`,
`compare`), `curve.rs` (GSW Svensson curve), `math.rs` (Nelder-Mead, Hessian, OLS),
`cli.rs`, `tests.rs`. `forwards.rs` has the accessor `Dataset::history(column)`.

**Models.** Each model gives the step, and the closed forms that exist (mean and variance
of r(t), stationary mean and variance, zero-coupon bond price P(0,t)). The bond price is
`E[exp(-integral of r)]` in the model itself. There is no market price of risk.

| Model | Equation | Scheme | Closed forms |
|---|---|---|---|
| `Vasicek` | dr = k (th - r) dt + s dW | exact Gaussian step | mean, variance, stationary, P(0,t) |
| `Cir` | dr = k (th - r) dt + s sqrt(r) dW | Andersen QE, psi_c = 1.5; r >= 0; each step has the exact mean and variance | mean, variance, stationary, P(0,t); `feller_ratio` = 2 k th / s^2 |
| `Ckls` | dr = k (th - r) dt + s r^g dW | g = 0: exact step; g > 0: Milstein with full truncation (weak order 1) | mean for each g; variance, stationary, P(0,t) for g = 0 and g = 1/2 |
| `HullWhite` | dr = (th(t) - a r) dt + s dW | r = x + alpha(t); x is exact OU | mean, variance, P(0,t); reprices the curve |
| `HoLee` | dr = th(t) dt + s dW | Hull-White with a = 0 | same |

Hull-White: `alpha(t) = f(0,t) + s^2 B(t)^2 / 2`, `B(t) = (1 - exp(-a t))/a`. The forward
curve `f(0,t)` and the discount factors come from `ZeroCurve`: the Svensson formula with
the six parameters of one GSW day. The curve matches the published zero yields (SVENY01..30)
to 5e-7.

**Simulation.** Paths run in parallel with rayon. Path p uses the stream
`Rng::for_path(seed, p)`, so the result is the same with any number of threads. Each task
has one buffer for a path. There is no allocation in the time loop. `simulate_map` reduces
each path at once (small memory). `trapezoid` gives the integral of r, so `exp(-integral)`
is the discount factor (error of order dt^2).

**Data and sampling.** `sample(series, start, end, freq)` takes the observations of an
interval and, for weekly and monthly data, the last observation of each period. The step
`dt` is the mean calendar spacing in years. The default series is the GSW 1-year zero yield
(continuously compounded), a proxy of the short rate. A Treasury 3-month history is not built.

**Calibration methods** (`calibrate(kind, method, sample, gamma)`).

| Method | What it estimates, and from what |
|---|---|
| `ml` | Vasicek: the exact conditional Gaussian likelihood (closed form: the AR(1) regression). CIR: the exact noncentral chi-square density (Poisson mixture of gamma densities), optimized with Nelder-Mead. CKLS: a quasi-likelihood (exact mean, variance s^2 r^(2g) (1 - exp(-2 k dt))/(2 k)), s profiled out. Ho-Lee: Gaussian random walk. Standard errors: numeric Hessian. |
| `ols` | Least squares of r(i+1) on r(i). Exact map: `k = -ln(b)/dt`, `th = a/(1 - b)`. Robust (HC0) standard errors, delta method. CIR: s from the mean of e^2 / v(r), with the exact conditional variance v. |
| `gmm` | CKLS (1992) moment conditions, exactly identified. The mean conditions are least squares. Gamma comes from a one-dimensional root search. Standard errors by the sandwich formula. |
| `realvol` | Drift from least squares. The volatility matches a realized-volatility estimator: `close` (all changes), `rolling:N` (last N changes), `ewma:L` (weight L^age). |

Realized volatility of a rate: the standard deviation of the changes over one sampling
step, in rate units per square root of a year (0.0100 = 100 bp per sqrt(year)). The model
sigma is set from the residuals of the fitted mean, divided by the conditional variance
of the model with sigma = 1. For Vasicek, sigma is in rate units. For CIR and CKLS, the
volatility of the rate at level r is `sigma r^gamma`.

Each fit has the parameters with standard errors, the log-likelihood (with its kind), the
AIC, the t-statistic of mean reversion and a flag. A fit with t below 2.86 (Dickey-Fuller
5%) is flagged "not different from zero". A sample with a constant series, fewer than 30
observations, rates at or below 0 for CIR and CKLS, or an AR(1) coefficient of 1 or more
gives an error with the reason.

**Studies.** `rolling(window, step)`: one calibration for each window, in parallel; output
is a table (JSON, or CSV with `--out`); a window that fails has the error in its row.
`frequencies`: the same interval at daily, weekly and monthly sampling.
`compare`: Vasicek, CIR, CKLS and Ho-Lee on one interval, with AIC.

**Commands.** See `fa help` and the README. The tests are in `tests.rs`; they run with
`cargo test --manifest-path rust/Cargo.toml --locked`.

## Options trading (`rust/src/trading/`, `fa trade ...`)

A research tool for SPY/QQQ option strategies. It is Rust code only. It does not use
`financial_analysis/`. The Python package `trading/` and the scripts `trade_backtest.py` and
`trade_paper.py` are deleted (2026-11-19). The folder `trading/research/` stays: it has the
notes of the bots and `LEADERBOARD.md`.

Commands:

| Command | Function |
|---|---|
| `fa trade fetch` | Downloads the bars and merges them into the CSV cache. |
| `fa trade backtest` | Runs the 22 bots on the cache and prints a leaderboard. |
| `fa trade paper` | One forward paper-trade step for each bot. It sends no broker orders. |

| Module | Function |
|---|---|
| `fetch.rs` | The bars of SPY, QQQ, ^VIX, ^VXN, ^VIX1D (1d, 1h, 5m) and ^IRX (1d) from the Yahoo chart endpoint. It merges each reply into the CSV file of the cache (see below). It reuses the HTTP client and the reply reader of `rust/src/futures/` (`futures::download`, `yahoo::parse_raw`). |
| `chain.rs` | The live option chain of one underlying: the Cboe delayed quotes (JSON). It gives bid and ask for the options that the chain lists. |
| `paper.rs` | The paper trader. It refreshes the data, runs each bot again with entries blocked before the paper start, and prices the new signals on the live chain. State: `data/processed/trading/paper/<bot>.json`. |
| `data.rs` | Reads the CSV files one time. Aligns the bars of all symbols, the vol indexes (VIX, VXN, VIX1D) and the rate on one bar index. A timestamp is an integer. |
| `calendar.rs` | Dates, business days, and the New York time offset with the daylight saving rules (checked against every date of the cache, from 1960). |
| `pricing.rs` | The model pricer: Black-Scholes, skew, spread and the variance clock. `erf` comes from the `libm` crate. |
| `structures.rs` | Vertical, iron condor and butterfly. A structure that is not defined-risk is an error. |
| `engine.rs` | The bar loop, the account (`Ctx`), the risk rules, the pattern-day-trader rule, the exit rules and the settlement at expiry. A bot sees data up to the current bar only. `run_from` blocks entries before a start time (paper trading) and returns the entries that the bot queued on the last bar. |
| `metrics.rs` | Statistics, and the split into in-sample and out-of-sample data. |
| `strategy.rs` | `Strategy` (the trait of a bot) and `ExitRules`. |
| `bots/` | One file for each bot (s01 to s21, s14b is in the s14 file) and the template `baseline`. `bots/mod.rs` is the registry. |
| `cli.rs` | The backtest command. It loads each market one time and runs the bots in parallel (rayon). |

The cache is `$TRADING_DATA_DIR`, else `data/raw/trading/` (not in git). One file for each
symbol and interval, for example `SPY_1h.csv` and `_VIX_1d.csv` (`^` is `_`). Each row has the
New York time with its offset (`2026-11-18 15:30:00-05:00`; a daily bar is at 00:00), and open,
high, low, close and volume. The merge rule: a fresh row replaces the cached row with the same
time, and the file never loses a row. Yahoo gives 730 days of hourly bars and 60 days of
5-minute bars, so the cache keeps history that the source has dropped. A bar that has not
ended is not written. A row with seconds is the live price row of Yahoo, not a bar, and the
program drops it.

All backtest option prices are model prices. No free source of historical option quotes
exists. A live chain exists for the present only, thus only the paper trader uses it.

Parity with the earlier Python code (checked 2026-11-19 before the Python code was deleted):
the trades are the same for the 22 bots and the template, with the 3% risk account and with
`--max-risk 250`; each price and each profit and loss differs by less than 2e-10 dollars.
A Rust refresh of the cache gives the same text as the Python cache for each old row (see the
ticket `option-backtest-rust`).

## Tests

`python -m pytest` from the repo root: 349 tests, all pass on 2026-11-19 (4 s).
19 test modules are in `tests/`. `cargo test --manifest-path rust/mapcheck/Cargo.toml`: 86 Rust
tests. The suites make no network calls; HTTP is mocked and
files use `tmp_path`. Install `pytest` with `pip install -r requirements-dev.txt`.
