# Architecture — code-verified module map

Verified against the code on 2026-07-17 (branch `docs-overhaul`, based on origin/main
`9751523`). Every claim below was checked by reading the named module. When this doc and
the code disagree, the code governs.

## Data flow

```
SEC EDGAR (companyfacts JSON API)      Yahoo chart     Cboe delayed quotes     FMP (key)
        |                                  |                  |                   |
   rust/mapcheck (Rust: fetch, map,   rust/src/market/   rust/src/market/    rust/src/market/
   check, store file; import.rs)      prices.rs          options.rs          fmp.rs, consensus.rs
        |                                  |                  |                   |
        +--> data/processed/TICKER.json    +------------------+-------------------+
        |    (statements only)                                |
        v                                                     v
   rust/src/company.rs (typed company)          data/raw/TICKER/ (cache files)
        |                                                     |
        +-----------------------+-----------------------------+
                                v
                  rust/src/analyze.rs  (fa analyze)  --> data/processed/TICKER_financials.xlsx (9 tabs)

   data/processed/TICKER.json also goes to: fa dcf classic, fa dcf scenarios, fa backtest and fa report
   (all Rust: rust/src/dcf/, rust/src/report/). data/raw/TICKER/ (prices, option snapshots,
   earnings dates) goes to fa report too. analysis/TICKER/driver_model.toml goes to fa dcf drivers,
   fa verdict and fa report.
```

The market data is attached to the in-memory company by `fa analyze` and written to the xlsx
file. It is **not** in the JSON store: the store has the periods only. The cache files
under `data/raw/TICKER/` (`options/`, `estimates/`, `prices.json`, `yf_earnings.json`) are the
durable market-data record. Their format is the format of the earlier Python code. `fa report`
reads them.

## The two DCF engines

(The engines are Rust code: see the section `rust/src/dcf/`. The Python engines were deleted on
2026-11-19. The text below describes the calculation; the class names are the names of the
old Python classes.)

The repo has two distinct Monte Carlo DCF engines. They coexist deliberately; neither
replaces the other.

### 1. Classic scenario engine — `rust/src/dcf/classic.rs` (`run_classic`, was `StochasticDCF`)

Samples the **top-level valuation items directly**: revenue growth, FCF margin, WACC,
and terminal growth, each an independent normal draw per path (10,000 sims, 10-year
horizon by default). FCF margin is derived from the last 5 annual periods when not
pinned. Gordon terminal value with a 0.001 floor on the `wacc − tg` denominator (which
lets near-degenerate paths dominate the tails — one reason the driver engine exists).
Minimum-history guard: an error unless ≥ 4 annual periods have positive FCF.
`rust/src/dcf/scenarios.rs` supplies bear/base/bull assumption presets and a
per-ticker net-debt/shares registry for it. Note: the IBM entries in `scenarios.rs` are
the **v3** aggregate framework (base P50 $141 family), kept as the aggregate-scenario
presets; the current v4 numbers live only in the driver engine's register (see
docs/HIDDEN-ASSUMPTIONS.md).

### 2. Driver-based engine — `rust/src/dcf/driver.rs` (`DriverDcf`, was `DriverDCF`)

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

Per-ticker parameterization lives in `analysis/<TICKER>/driver_model.toml` (IBM, JNJ, ANT):
drivers with written rationales, the correlation matrix, segment bases, net debt, shares.
The rationale strings are normative over DRIVERS.md prose (see CLAUDE.md). The older Python copies
`driver_model.py` are deleted (2026-11-19).

## The backtest harness

`run_backtest(company, horizon, n, seed)` in `rust/src/dcf/backtest.rs` (`fa backtest TICKER`;
the report uses the same function) — walk-forward vintages, one per clean fiscal-year-end + 90 days:

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
- `clean_annuals` requires both revenue and OCF.
- Scope: revenue/FCF projections only — no WACC, no terminal value, no price.

The HTML report (`fa report`) renders it as section 5 ("Backtest — mechanical vintages"): coverage +
per-horizon bias chips, revenue/FCF hedgehog charts, vintage scorecard. Calibration to
date: IBM 14/18 realized outcomes in the P10–P90 band (78% vs 80% nominal); DUOL 0/6 —
mechanical extrapolation fails on growth-phase tickers, so treat their bands as
decorative until a driver build exists.

## `financial_analysis/` and `sec_contact.toml` (2026-11-20)

The repo has no Python code. The engines, the scenarios, the backtest, the store, the schema,
the market-data readers and the report are Rust code (`rust/src/dcf/`, `rust/src/company.rs`,
`rust/src/market/`, `rust/src/report/`). The folder `financial_analysis/` has data only.

| File | What it is |
|---|---|
| `financial_analysis/standard_variables.toml` | The registry of standard variables (data, 1,385 lines): for each variable its alias tags in a fixed sequence, fallback formulas, unit, sign, core flag and memo flag; the rollups; the containment rules; the excluded tags with reasons. The Rust program `rust/mapcheck` uses it to make the variables. |
| `sec_contact.toml` (repo root) | The SEC contact (key `user_agent`): a project address, public on purpose. `mapcheck::http::sec_user_agent` reads it. The environment variable `SEC_USER_AGENT` replaces it. |

## Standard variables and `rust/mapcheck/` (2026-11-19)

Filers use different XBRL tags for the same line. The registry
`financial_analysis/standard_variables.toml` gives each line one standard variable name;
the engines and the analysis code read only these names.

The Rust program `rust/mapcheck` is the only implementation of this work (owner
decision of 2026-11-19: the language of the repo is Rust). `fa import` runs the mapping and writes the store file
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
| `http.rs`, `sec.rs` | HTTP with a delay between requests; the SEC ticker list, company facts and filing files, and the cache folders. The contact text for the SEC comes from `mapcheck::http::sec_user_agent`: the environment variable `SEC_USER_AGENT` if it is set, else the file `sec_contact.toml` (searched in the folder of `FA_REPO`, in the current folder and its parents, and in the repo root known at build time). A missing, empty or bad file is an error that names the file. |
| `store.rs` | Writes `data/processed/<TICKER>.json` for the valuation code (`fa`). |
| `filing.rs` | Filing-level XBRL, first part: reads `MetaLinks.json` and the instance document of the latest 10-K, builds the three statements as trees from the calculation data of the filer, and gives capex, depreciation and amortization, and total debt from the filing lines. It keeps the three fiscal years of the 10-K and the facts with one segment member. |
| `supplement.rs` | Gives a value from the 10-K for a critical input (revenue, capex, share count) that the company facts do not have: company-tag capex lines, a standard tag with segment members, share classes. The source text of the value starts with `filing`. |
| `crosscheck.rs` | Compares the standard variable with the value from the filing lines for capex, depreciation and amortization, and total debt. No outside data source. |
| `cli.rs`, `main.rs` | The command line: `check` (default), `fetch`, `import`, `values`, `filing`, `crosscheck`. |

Results, the list of checks and the limits: `rust/mapcheck/README.md`.

The crate `rust/` (the `fa` command) is a different program. The valuation, the report, the
rates, the bonds and the option trading are Rust code in it. It starts no Python code.

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

### `rust/src/dcf/` — the DCF engines in Rust (2026-11-19)

Rust form of the two Python engines, the scenarios, the verdict and the backtest (ticket
`T-20261119-dcf-engine-rust`). The Python engine files are deleted. The random numbers are the
numpy stream, so each published number stays the same (`docs/HIDDEN-ASSUMPTIONS.md` #120 to #126).

**Files.** `nprandom.rs` (`SeedSequence`, `PCG64`, ziggurat normal; tables in
`ziggurat_tables.rs`), `stats.rs` (percentile, Cholesky, mean, std), `classic.rs`
(`StochasticDCF`), `driver.rs` (`DriverDCF`, drivers, correlations, attribution),
`scenarios.rs` (presets, capital structure), `verdict.rs`, `model.rs` (a driver model as a
TOML file, `Overrides`, hand DCF, bisection), `analysis.rs` (runs with overrides,
sensitivity, implied value, the fade check, the commitment check, the equity-weight fixed point),
`backtest.rs` (point-in-time vintage backtest), `cli.rs` (`fa dcf`, `fa backtest`), `tests.rs`
and `runner_tests.rs`. Outside `dcf/`: `rust/src/rates.rs` (Treasury curves, New York Fed rates,
BLS CPI, FRED, inflation, cross-checks), `rust/src/bonds.rs` (ICE BofA, Moody's, HQM, ratings),
`rust/src/verdict_rates.rs` (market `rf` and `kd_spread` in a driver model); the `Fa` struct in
`lib.rs` keeps the method names and calls them.

**Data.** `analysis/<TICKER>/driver_model.toml` (IBM, JNJ, ANT): drivers with their
rationale text, correlations, segments, net debt, shares, horizon. The fixtures of
the tests are in `rust/tests/fixtures/dcf/`. They are the recorded output of the Python
engines (numpy 2.5.3) and of the three Python runners (deleted 2026-11-19; the fixtures are
their record). `rust/tests/fixtures/backtest/` and `rust/tests/fixtures/rates/` are the same kind
for the backtest, the rates, the bonds and the verdict with market rates; the Treasury, New York
Fed and BLS files there are saved answers, the FRED series are made up (see
`docs/HIDDEN-ASSUMPTIONS.md` #128). `THIRD-PARTY.md` has the license
text for the numpy tables.

**Speed.** One run of 10,000 paths, 10 years and the attribution of 8 drivers takes
about 30 ms on 4 threads (60 ms on 1 thread). The z rows are made once; the paths run
in parallel. Parallel paths do not change a result.

**Commands.** `fa dcf drivers|classic|scenarios|models`, `fa backtest`, `fa verdict`, `fa rates`, `fa bonds`. See the README.

### `rust/src/dcf/paths.rs` — the path engine (2026-11-19)

Ticket `T-20261119-stochastic-dcf-drivers`. A model file with `engine = "paths"` runs here. The old
engine (`driver.rs`) and its three model files do not change. The old engine draws each driver one
time for each path. The path engine draws the parameters one time for each path and then moves the
processes year by year with shocks. Traps: `docs/HIDDEN-ASSUMPTIONS.md` #155 to #169.

**Files.** `process.rs` (the model file: `PathsSpec`, `ParamSpec`, `ProcessSpec`, `--pset`),
`paths.rs` (`PathEngine`: compile, validate, `simulate`, `deterministic`, `attribution`,
`variance_shares`), `pathrate.rs` (`RateFactor`: Hull-White on a GSW curve, Vasicek, CIR),
`paths_cli.rs` (the output of `fa dcf drivers --model`), `paths_tests.rs` (tests, speed table).

**Model file.** `[[segments]]` (base revenue, pin of year 1, name of the growth process);
`[process.<name>]` with a role (`growth`, `margin`, `sbc`, `charge`, `s2c`); `[reinvestment]`, `[tax]`,
`[discount]`, `[rates]`, `[terminal]`, `[failure]`; `[[corr]]` (drawn parameters) and
`[[shock_corr]]` (yearly shocks). A number is fixed (`0.05`) or drawn (`{ mean, std, lo, hi }`). The
name of a number is its TOML path (`process.growth.target`).

**Process.** The value of year `t` is `center(t) + u(t) + drift(t)`, cut at `lo` and `hi`.
Center, shape `ou`: `target + (start - target) phi^(t-1)`, `phi = exp(-ln 2 dt / half_life)`.
Center, shape `ramp`: linear from `start` to `target` in `ramp_years`. Shock: `u(1) = 0`,
`u(t) = rho u(t-1) + max(sigma, sigma_level |center|) e(t)`. With `rho` not set it is `phi` (then
`x(t) = target + phi (x(t-1) - target) + sigma e(t)`, the Ornstein-Uhlenbeck step) and 0 for `ramp`.
`half_life = inf` and `sigma = 0` give a constant: the limit that gives the old engine.

**Cash flow.** Growth of each segment gives revenue. Option A (`reinvestment.mode = "margin"`):
`FCFF = revenue (margin - SBC - charge)`. Option B (`"sales_to_capital"`): the margin is the
operating margin; `FCFF = EBIT - tax - max(revenue change, 0) / s2c`; the tax is paid on positive
EBIT after the loss pool (`tax.opening_loss`, `tax.loss_limit_share`).

**Discount rate.** `k = w (rf + beta erp) + (1 - w) (rf + spread) (1 - tax)` as in the old engine.
`rf` of year `t` comes from the rate factor at the start of year `t`; the discount factor is the
product of `1 / (1 + k)`. `rates.model`: `fixed` (a number or a draw, as the old engine),
`hullwhite` (drift from the six GSW numbers in `[rates.curve]`), `vasicek`, `cir`. `rates.mode`:
`long` (zero yield of `tenor` years from the closed form of the model), `long_flat` (the same, with the
expected value fixed at the level of year 1) or `short` (short rate plus `spread`). The rate shock
has a correlation with the other shocks (`[[shock_corr]]`, name `rate`).

**Terminal value.** Gordon with the cost of capital of the end of the horizon. Terminal growth: the
caps `cap_by_process`, `rf_margin`, and the guard `k - 2%` (T-DCF-2). Option B: `tg / ROIC` of NOPAT is
reinvested (`terminal.roic`, a number, a draw or `"wacc"`). **Failure.** `[failure]`: yearly hazard,
recovery value, `sampled` or `expected`. Off when the table is absent.

**Random numbers.** One stream for each path (`ratemodels::rng`): the same result with any number of
threads. The normals of the parameters are correlated by a Cholesky factor of `[[corr]]`; the yearly
shocks by a Cholesky factor of `[[shock_corr]]`. A matrix that is not positive definite is refused.

**Commands.** `fa dcf drivers TICKER --model driver_model_paths.toml [--price P] [--pset NAME=V]
[--sensitivity NAME=V1,V2] [--implied-price P --implied-param NAME] [--no-shocks] [--freeze GROUP]
[--time] [--json]`. The output: headline, value at the means, attribution (P50 change at minus and plus
one std of each drawn number), variance by group, the mean path by year with the revenue P10, P50 and
P90, sensitivities, implied values. Speed (release build, ten years, Hull-White, three shock channels):
10,000 paths 12 ms on 1 thread and 3 ms on 4 threads; 100,000 paths 123 ms and 33 ms.

**Files of ticker models.** `analysis/ANT/driver_model_paths.toml` (first pass, not reviewed),
`analysis/IBM/driver_model_paths.toml` (a comparison file; the record stays `driver_model.toml`).

### `rust/src/dcf/` — calibration of the growth and margin processes (2026-11-19, branch `claude/dcf-calibration`)

Ticket `T-20261119-dcf-process-calibration`. The command `fa dcf calibrate` estimates, from the company cache, the
parameters that a path engine needs: the speed of the return to a long-run value, the long-run value, and the size of the
shocks, for revenue growth and for two margins. It also measures how often a later real revenue is inside a P10 to P90 band.
It uses no network. The output has no random part except a fixed-seed bootstrap and fixed-seed draws.

| File (`rust/src/dcf/`) | Function |
|---|---|
| `panel.rs` | The annual panel of the cache (`build_panel`): one row for each company and fiscal year, from the mapping of `mapcheck`. The rules of a clean growth series (`clean_series`, tested on plain values): one row for each fiscal year, revenue above zero, consecutive years only (350 to 380 days), a jump flag, an acquisition flag, a reversal flag. Each rule has a counter (`RuleCounts`). Size buckets. |
| `calibrate.rs` | The AR(1) estimators (`fit_ar1`): pooled, within (company mean), within with the Nickell correction, Anderson-Hsiao. Standard errors that are robust to correlation inside a company. The shock correlation with a bootstrap over companies. The table (`ProcessTable`, TOML) and `priors_for(revenue, template, sector)`. |
| `calib_report.rs` | The command, the text output, the growth fade of fast growers by starting level, the list of very large fast growers. |
| `calib_bands.rs` | The calibration of the revenue bands: four methods, in sample and out of sample. |
| `calibrate_tests.rs` | Tests on synthetic panels with known parameters. |

**Model.** `y_t = mean + phi (y_{t-1} - mean) + sigma e_t` for revenue growth, operating margin and free cash flow margin.
`phi` is the value of the within estimate with the Nickell correction. The other three estimates stay in the table. The
growth that is used has no jump (more than 60% for a company with $1B or more of revenue) and is between -80% and +300%.

**Table.** `analysis/priors/process_priors_2026-11-19.toml` has one cell for each size bucket and template (the sizes are
`<1B`, `1-10B`, `10-30B`, `30-100B`, `>100B`, `all`), with `phi`, its standard error, the half-life, the mean, the
standard deviation of the shock, the number of observations and companies, and the other estimates. `priors_for` takes the
cell of the size and the template if it has 150 observations and 25 companies. If not, it takes the cell of the size, then
the cell of the template, then all. The cache has the SIC code in `<T>/sec_8k_earnings.json` (`mapcheck::sec::company_sic`), and `mapcheck::sec::ff12_sector` gives the Fama-French 12 sector. The priors do not use them yet, so no cell has a sector.

**Results of the run of 2026-11-19** (996 companies, 14,305 company-years, 13,228 growth observations; the panel takes 7 seconds
and the whole command about 15 seconds on 4 threads):

- Growth, all sizes: `phi` = 0.27 (pooled), 0.02 (within), 0.11 (within with the Nickell correction), 0.16 (Anderson-Hsiao). The mean is
  9%, the shock is 18%, and the standard deviation of the company means is 14%. Growth is close to a company mean plus
  noise. The half-life of the deviation from the company mean is under one year.
- Operating margin: `phi` = 0.72 (pooled), 0.48 (within with the correction); the half-life is about one year.
  The standard deviation of the company means is 16%.
- The shock correlation of growth with operating margin is +0.25 (standard error 0.03). The correlation with the free cash
  flow margin is about 0.
- Band coverage (P10 to P90, target 80%): the rule of `backtest.rs` gives 60% at one year and 68% at three years (too narrow). The AR(1) with the pooled
  values gives 81% to 83% in sample and 80% to 82% out of sample (parameters from the years up to 2015). The fast growers
  and the very large companies are not well described: see `docs/HIDDEN-ASSUMPTIONS.md` #176, #177 and #182.

**Survivorship.** The cache has the companies that exist now. The companies that failed or were bought are not in it. The
growth mean is too high, the shock size and the lower tail are too low, and the persistence of margins is too high. The
fade of fast growers is too slow. The command prints this text with each run. The size of the bias is not measured.

**Limits.** The priors do not use the sector yet. No GDP series in the repo cache: the growth less the mean growth of the fiscal year
replaces it. A company with fewer than three growth values has no weight in the within estimates. The traps are #170 to #184
of `docs/HIDDEN-ASSUMPTIONS.md`.

### `rust/src/dcf/` — the automatic model (2026-11-19, branch `claude/dcf-auto`)

Ticket `T-20261119-dcf-auto-model`. The commands `fa dcf auto`, `fa dcf screen` and `fa dcf pit` make a path model for a ticker from
data, run it, and test it. No hand input and no network. An automatic model is a screen, not a reviewed valuation. Traps: `docs/HIDDEN-ASSUMPTIONS.md` #185 to #199.

| File (`rust/src/dcf/`) | Function |
|---|---|
| `auto.rs` | The builder `build`. Input: the standard variables of the company (`AutoInputs`, `YearData`), the parameter table with its fade cells, the market (`Market`: GSW curve, price, beta). Output: the model file (TOML, `engine = "paths"`), one row for each number with source and rule (`ExplainRow`), the warnings. It refuses (`Refusal`) with a reason. The helpers (`ar1_fit`, `own_sigma`, `tax_rate`, `regression_beta`, `load_curve`) are public and tested with a hand calculation. |
| `auto_cli.rs` | `fa dcf auto TICKER` (map from the cache, build, print, run with `paths_cli::run`, price-implied values) and `fa dcf screen` (one row for each ticker; rayon over tickers). |
| `auto_pit.rs` | `fa dcf pit`: the point-in-time test. `model_at` makes the model from the fiscal years up to the base year only. |
| `auto_tests.rs` | Unit tests: each rule on small data with a fixed answer, the refusals, the point-in-time property, the screen on 1 and on 4 threads. |
| `calibrate.rs` (added) | `FadeCell`, `FadeFile`, `slow_half_life`, `level_spread_ratio`: the fade table as data. |
| `calib_report.rs` (added) | `fade_cells`: `fa dcf calibrate --out FILE` now writes the `[[fade]]` cells after the AR(1) cells. |
| `paths.rs` (added) | `simulate_one_thread` (a run on the calling thread, with detail), `simulate_serial` is public. The results are the same as `simulate`. |

**The rules** (`auto.rs`; each number is printed by `fa dcf auto TICKER --explain`):

| Input | Source (standard variables, files) | Rule |
|---|---|---|
| Base revenue | `revenue` of the latest fiscal year | The trailing four quarters are not used (see #194). |
| Growth: company mean | `revenue` of 5 fiscal years in sequence; the table cell of the size bucket (`priors_for`) | Mean of the last 4 growth values, shrunk to the cell mean with `w = tau^2 / (tau^2 + sigma^2 mvf)`. A jump with cash for businesses, or one that reverses, is left out. |
| Growth: year 1 | the same | AR(1): `mean + phi (last - mean)`; standard deviation `sqrt((1 - phi)^2 var_mean + sigma^2)`. The value and the long-run mean are correlated. |
| Growth: long run | L = the lower of the 10-year zero yield of the GSW curve (`data/raw/rates`) and 4.0% (`LONG_RUN_GROWTH`, FOMC SEP 2026-09-16; record A-001, 2026-11-20); fade cells of the table | The company mean fades linearly to L by year 10 if it is above it (a mean below L stays). The ramp equals an exponential fade with the half-life of the fade table (k = 3 to 10). |
| Growth: shocks | table cell, own history, fade cells | `phi` and `sigma` of the cell; `sigma` of the company from its own residuals (weight 4 years); at least `kappa` times the level (`kappa` from the fade table). |
| Margin | `operating_cash_flow`, `capex_total` (else `capex`), `interest_paid` (else `interest_expense`), `interest_income`, `revenue` | FCFF before SBC and before interest `= (OCF - capex + (1 - t) (interest - interest income)) / revenue` (the interest income is out since 2026-11-20: the cash that earns it is in the bridge). The history is shrunk as the table margin (after interest); the mean after-tax interest ratio is added. A `phi` with a standard error above 0.08 is replaced by the `phi` of all sizes. |
| SBC | `sbc_total` / `revenue`, last 3 years | Constant, a cost of the model. |
| Tax | `income_tax_expense`, `income_before_tax` | The median effective rate of 5 years if there are 3 values, each between 5% and 40%, in a range of 10 points; else 21%. |
| Net debt | `total_debt`, `cash_and_short_term_investments` (else `cash_and_equivalents` + `short_term_investments`), `long_term_investments`, latest balance sheet | Stock-and-flow rule of the IBM review F15 (2026-11-20). Pensions, leases and minority interests are in a memo line. |
| Shares | `shares_outstanding`, `shares_diluted`, `shares_basic`, latest period | The count at the end plus the dilution of the period (2026-11-20); the weighted diluted count if the end count is not within 85% to 110% of it. |
| Timing | model date (`--as-of`), end of the base fiscal year, date of the balance sheet | The discount starts on the model date (`[discount] stub_years`); the part of year 1 before the balance sheet date is out (`year1_share`) (2026-11-20). |
| Beta | `<cache>/<T>/prices.json`, `<cache>/trading/SPY_1d.csv` | Weekly returns of 5 years, `2/3 raw + 1/3`; else 1.0 with a warning. |
| Equity risk premium | repo convention | 4.41%, standard deviation 0.75%, lower bound 2%. |
| Risk-free rate | GSW file of the cache | Hull-White (`a` 0.14, `sigma` 0.0075), `long_flat`, 10-year zero yield; else a fixed 4.2% with a warning. |
| Equity weight | `--price`, else the last close of the price cache | `E / (E + debt)`; else 1.0 with a warning. |
| Acquisition charge | `acquisitions_net_of_cash` / `revenue`, 5 years (a year with no value is 0), `w` of the growth mean | Cost of the model: `w` x the mean ratio (`acq_w`, record A-002, 2026-11-20). It falls linearly to zero along the drift ramp when the company mean is above L (`acq_fade`); else constant. |
| Terminal growth, horizon | one long-run growth L (record A-001) | Mean L (the same number as the fade level), standard deviation 0.75%, the caps of the engine; 10 years. `long_run=off` gives the mean 2.25% of the old model. |

**Templates and data quality.** Only `standard` has a model. `bank`, `insurer`, `reit`, `fund`, `foreign`, `new`, `non-usd` and `no-revenue` give "no model for this template". A mapping
status of FAIL, a necessary variable with no value, or fewer than 3 fiscal years in sequence give no value and name the reason. A mapping status of WARN runs, and the warnings that touch a model input are printed.

**`fa dcf auto TICKER [--price P] [--explain] [--write FILE] [--cache DIR] [--table FILE] [--as-of D] [--seed S] [--sims N]`.** It prints the banner "Automatic model: a screen, not a reviewed valuation.",
the numbers of the model, the warnings, then the output of `fa dcf drivers TICKER --model FILE` (headline, value at the means, attribution, mean path), and with a price what the price implies: the long-run
growth, the target margin and the half-life of the fade. `--write FILE` writes the model; `fa dcf drivers TICKER --model FILE` runs it again with the same numbers. Exit code: 0 a value, 3 no value, 2 an error.

**`fa dcf screen [--tickers-file FILE | TICKER ...] [--cache DIR] [--out FILE] [--jobs N] [--prices FILE] [--sims N]`.** One row for each ticker. Stage 1 maps the company facts (parallel over tickers), stage 2 builds
the models, stage 3 simulates them. Each ticker runs on one thread, so the result does not depend on `--jobs` (a test compares 1 and 4 threads). The default is 4,000 paths. No price is downloaded.
`--prices FILE` has lines `ticker,price`.

**The two defaults of 2026-11-20** (records A-001 and A-002; #271). `long_run = 0.040`, `acq_w = 1` and `acq_fade = 1` (the charge falls to zero with the fade of the growth when the company mean is above L).
Data of 2026-11-19 (data store commit 41116d0), `fa dcf screen` of `sp500.txt` (4,000 paths, seed 7), old model `--rules long_run=off,acq_w=0` and new model: median P50 / price 0.4895 to 0.4745
(430 tickers; the 55 stable names 0.523 to 0.511; L alone 0.5605; charge alone 0.4245; constant charge `acq_fade=0` with L 0.427); P50 at or below 0: 38 to 39 (constant charge: 50).
Median terminal value share 0.611 to 0.685. `fa dcf pit`: revenue error unchanged; error of the free cash flow after acquisitions, T+1 / T+2 / T+3, -1.7 / -1.4 / -0.6 to -0.4 / -0.1 / +0.6% of
revenue; its coverage P10 to P90 is 1 point lower (76.3 / 77.4 / 77.8% against 77.3 / 78.0 / 78.4%). `fa dcf returns`: rank correlation of P50 / price with the return, 1 year +0.062 to +0.058, 3 years +0.128 to +0.126
(means over the years; standard errors 0.036 to 0.045). IBM P50 (`fa dcf auto IBM`, 2026-11-19) $132.87 to $95.84. Files: `analysis/priors/auto_pit_{old,old_afteracq,new,constcharge}_2026-11-20.txt`,
`auto_returns_{old,new}_2026-11-20.txt`; the screen CSV is not kept (the command makes it again).

**`fa dcf pit [--cut YEAR] [--rules K=V,..] [--paths N] [--out FILE]`.** The point-in-time test (see #199 for its limits).

**The investigation of 2026-11-20** (ticket `T-20261120-dcf-investigation`; findings `analysis/priors/dcf_investigation_2026-11-20.md`;
traps #260 to #270).

| Part | Function |
|---|---|
| `Rules` (`auto.rs`) | Alternatives for measurement and for the owner: `rf`, `erp`, `beta_weight`, `equity_weight_one`, `terminal_growth`, `terminal_rf`, `years`, `long_cap`, `long_run` (default 0.040; `off` or 0: the old model; `terminal_growth=X` also turns it off), `fade_years`, `growth_w`, `growth_phi`, `margin_w`, `margin_last`, `sbc_cost`, `acq_charge`, `acq_f`, `acq_mult`, `acq_w` (default 1; 0: no charge; `acq_charge` and `acq_f` win), `acq_fade` (default 1; 0: constant charge), `off` for `long_run` and the flags only, `bridge_lti`, `bridge_leases`, `no_interest_income`, `shares_latest`, `stub`, `mid_year`. `--rules K=V,..` of `fa dcf auto`, `screen`, `pit` and `returns` sets them (`auto_pit::parse_rules`). |
| `[discount]` (`process.rs`, `paths.rs`) | `stub_years`, `year1_share`, `mid_year`. Not set: the end-of-year rule of before (the hand models do not change). |
| `fa dcf screen --implied` (`auto_cli.rs`) | `implied_discount_rate`: the cost of capital of year 1 that gives the price (a search on the premium). New CSV columns: `model_rate`, `implied_rate`, `sbc_ratio`, `net_debt`, `shares`, `beta`, `equity_weight`. |
| `fa dcf returns` (`auto_returns.rs`) | The P50 / price of the point-in-time model (date: end of the fiscal year + 100 days) against the later 1-year and 3-year price returns: rank correlations by year and pooled, quintiles inside each year, size buckets, the years as clusters. About 10 s for 420 companies. Result: `analysis/priors/auto_returns_2026-11-20.txt`. |
| `filer_template` (`rust/mapcheck/src/checks.rs`) | Also domestic and split deposit lines, deposit-taking lenders, lenders with no current-assets total, real estate at gross less depreciation, sales-type leases of real estate (#264). |

Screen of `sp500.txt` before and after the four corrections (2026-11-19 data, 4,000 paths): median P50 / price 0.480 and 0.489; P50 at or
below 0: 10% and 9%; median P90 / P10 10.6 and 10.7; 438 and 430 values with a price (8 REITs are now refused). The point-in-time
bands change by less than 0.2 points (`analysis/priors/auto_pit_2026-11-20.txt`).

**Results of the run of 2026-11-19** (cache of 1,249 tickers; the full cache is not in git).

- Screen of the 1,003 tickers of `rust/mapcheck/universe/sp500.txt` and `next500.txt`: 844 have a value and 159 have none (bank 48, REIT 40, insurer 38,
  new registrant 12, history too short 7, mapping status FAIL 7, no revenue 5, fund 1, currency other than USD 1). On 4 threads: mapping 8.5 to 10.6 s,
  models 0.05 s, simulation of 4,000 paths for each ticker 1.4 to 2.0 s. On 1 thread: 32 s, 0.15 s and 3.9 s. The rows are the same on 1 and on 4 threads.
- Point-in-time test (`fa dcf pit`, 6,128 company-years of 725 companies, 2,000 paths for each model). Share of the real values inside the P10 to P90 band (target 80%):

| Rule set | Revenue T+1 / T+2 / T+3 | Revenue, growth above 50% (270 company-years) | Free cash flow T+1 / T+2 / T+3 | Free cash flow, margin range above 10 points |
|---|---|---|---|---|
| Table values only (`own_sigma_nu=0,level_kappa=0`) | 82.0 / 80.0 / 83.0 | 47.8 / 44.1 / 50.7 | 86.3 / 85.9 / 85.1 | 74.7 / 74.4 / 73.4 |
| With the shock size of the company | 84.4 / 82.5 / 86.0 | 76.7 / 75.2 / 80.7 | 86.9 / 86.2 / 85.6 | 79.5 / 78.5 / 78.1 |
| With the level rule from the fade table | 87.5 / 85.1 / 87.3 | 81.1 / 79.3 / 80.7 | 86.9 / 86.6 / 85.4 | 76.3 / 75.7 / 74.0 |
| Both (the rules of the model) | 86.1 / 83.8 / 86.9 | 83.7 / 82.2 / 84.4 | 87.1 / 86.2 / 85.6 | 79.8 / 78.7 / 78.3 |
| Both, table from the years up to 2015, base years 2015 and later (4,713 company-years) | 84.8 / 83.3 / 86.8 | 82.5 / 85.1 / 87.3 | 82.7 / 80.8 / 78.9 | 75.5 / 73.3 / 71.5 |

  The result files are `analysis/priors/auto_pit_2026-11-19.txt` and `analysis/priors/auto_pit_cut2015_2026-11-19.txt`. Systematic errors that stay (#199): the bands are a little too wide
  for companies with a stable margin (free cash flow coverage 95%); a decline in the base year is followed by more growth than the model gives (the real revenue at T+3 is 11% above the P50);
  the table from before 2016 gives revenue that is 6% too low at T+3 for the years after 2015.
- Efficiency: the standard error of the P50 of the screen is 3.3%, 2.6%, 1.8%, 0.9% and 0.6% of the P50 for 1,000, 2,000, 4,000, 10,000 and 40,000 paths (the mean of 5 tickers over 20 seeds). The screen uses 4,000 paths.
  The search of an implied value uses the same random numbers for each step. Antithetic numbers are not made.

## Company data, market data and the xlsx file (`rust/src/company.rs`, `market/`, `analyze.rs`, 2026-11-19)

These parts are Rust code only. The Python files `scripts/analyze.py`, `scripts/import_financials.py`,
`estimates.py`, `fmp.py`, `xbrl_tags.py`, `schema.py`, `store.py`, `market_data.py` and `options.py` are deleted.

| Command | Function |
|---|---|
| `fa import TICKER...` | Imports the SEC data (cache first) and writes `data/processed/<TICKER>.json`. Calls `mapcheck::import::import_ticker`, the same function as `mapcheck import`. |
| `fa analyze TICKER [--refresh] [--out F] [--no-market] [--no-options] [--no-estimates]` | Imports, gets the market data, writes the nine-sheet xlsx file. |

| Module (`rust/src/`) | Function |
|---|---|
| `company.rs` | Typed company: periods with one value for each registry variable, the source texts and the unmapped tags. `Company::from_store` reads the content of the store file in memory, `Company::load` reads the file. Free cash flow and its margin. `series` gives the values of one metric, oldest first (a variable, `free_cash_flow` or `fcf_margin`). |
| `market/mod.rs` | Rounding as Python `round(x, 6)`, JSON with `NaN` (written by Python), key order of a JSON object, the HTTP client with 2 seconds between requests. |
| `market/prices.rs` | The older Yahoo loader (adjusted, as `yfinance`); cache `data/raw/<T>/prices.json`; HTTP 429 keeps the cache. `fa analyze` and `fa report` use it. New code uses the price layer below. |
| `market/options.rs` | Cboe delayed quotes to a snapshot `data/raw/<T>/options/<date>.json`; snapshot files; `compute_chain_metrics`. |
| `market/fmp.rs` | FMP estimates and surprises (the source of revenue estimates); key from `FMP_API_KEY`; caches `estimates.json`, `surprises.json`. |
| `market/consensus.rs` | The consensus snapshot files (Nasdaq source: see the section on estimates below) and `yf_earnings.json` of the Python code (read only). |
| `analyze.rs` | `flatten` (the rows of the nine sheets), `write_xlsx` (crate `rust_xlsxwriter`), the command. |

`rust_xlsxwriter` is the Rust port of the library `XlsxWriter`; it writes xlsx files only and has no network code.
Fixtures from the Python functions are in `rust/tests/fixtures/market/` and `rust/tests/fixtures/analyze/`.
The traps are #140 to #146 of `docs/HIDDEN-ASSUMPTIONS.md`.

## The price layer (`market/price_*.rs`, `fa prices`, 2026-11-19, branch `claude/price-history`)

The person does not give the price. The program gets the daily price history of a ticker from
more than one source, keeps it in the cache, and each command that needs a price reads the cache.

| Module (`rust/src/market/`) | Function |
|---|---|
| `price_source.rs` | The trait `PriceSource` (`name`, `adjusts_dividends`, `fetch(ticker, from, to)`), the row type `Row` (open, high, low, close, volume, and where the source has them: adjusted close, dividend, split) and three sources: Tiingo (key in `TIINGO_API_KEY` or `.env`, in the header `Authorization: Token`), the Yahoo chart endpoint, the Nasdaq site API. Each parser is a function of the reply bytes. `sources()` gives the order: Tiingo (with a key), Yahoo, Nasdaq. |
| `price_cache.rs` | `data/raw/<T>/prices.json` (the form that the older readers use) and `prices.meta.json` (source, adjusted, updated, first and last date, rows). `update_ticker` (incremental update, overlap check, full history again, no mixing of two sources, HTTP 429 rule). `last_close(cache, ticker)` and `history(cache, ticker)` for other modules. |
| `price_check.rs` | The cross-check of two sources on the common dates, and the check of the adjusted series against the dividend and split events of one source. |
| `price_cli.rs` | `fa prices get`, `show`, `last`, `check`; `cached_price` (the price of a command that has no `--price`). |

Commands: `fa prices get TICKER... [--tickers-file F] [--source S] [--full]`, `fa prices show TICKER [--from D] [--to D]`,
`fa prices last TICKER...`, `fa prices check TICKER... [--sources A,B] [--tolerance T]`. `fa verdict TICKER` without `--price`
uses the last close of the cache and prints which price and date it used.

Rules. One request each 2 seconds (one client for all sites). On HTTP 429 the program waits 60 seconds one time, then stops that source for
the run and keeps the cache. A failed source never changes or shortens a cache file (a full history replaces the file only if it has
90% of the cached rows or more). The cache keeps one series from one source: the record says if the closes are adjusted for splits
and dividends (Tiingo, Yahoo) or for splits only (Nasdaq). A different source replaces the whole file or does not change it. An
incremental update gets the days from 10 calendar days before the last row, replaces the cached rows of these days, and gets the full
history again if the overlap differs by more than 0.02% (a split or a dividend changed the adjusted history).
The mapping of the sources is in `docs/HIDDEN-ASSUMPTIONS.md`, #200 to #214. `mapcheck/src/http.rs` has `get_with_headers`.

## Estimates and earnings dates (`market/nasdaq_est.rs`, `estimates_*.rs`, `earnings_dates.rs`, 2026-11-19, branch `claude/estimates-earnings`)

Source: the Nasdaq site API (vendor Zacks; no key; not a published API, see `docs/HIDDEN-ASSUMPTIONS.md` #215 to #229). SEC filing list for the check of the past dates.
No revenue estimates in this source; FMP gives them with a key.

| Module (`rust/src/market/`) | Function |
|---|---|
| `nasdaq_est.rs` | The URLs, the parsers of each endpoint (functions of the reply bytes), the text number forms, the symbol rule (`BRK-B` to `BRK.B`), the request layer `Client` (counts requests, waits 60 s at a block, stops the source at the second block), and the functions that put the replies into a `ConsensusEstimates` snapshot. |
| `consensus.rs` | The snapshot type (the Yahoo layout, plus `source`, `endpoints`, `period_ends`, `ratings`, `consensus_change`), `forward_pe`, `eps_growth`, `eps_next_four_quarters`, `merge_revenue` (FMP revenue by month). |
| `estimates_cache.rs` | `update_ticker` (one request for each endpoint that today's files do not have), the files `estimates/<date>.json` and `earnings_surprises.json`, `surprises_for_sheet`. |
| `earnings_dates.rs` | `earnings_dates.json` (past and next dates), the calendar cache `_calendar/earnings-<date>.json` and its rule, the scan, `estimate_next`, `resolve_next`, and the comparison with the SEC file (`compare_dates`). |
| `estimates_cli.rs` | `fa estimates get|show`, `fa earnings calendar|dates`. |
| `rust/mapcheck/src/sec.rs` | `earnings_filings` (form 8-K with item 2.02 of `filings.recent`), `fetch_earnings_filings` (it saves also `sic` and `sic_description` of the filer), `company_sic` (reads them from the cache, no network), `ff12_sector` (SIC code to Fama-French 12 sector); the command `mapcheck fetch --submissions TICKER...` writes `<T>/sec_8k_earnings.json`. |

Commands: `fa estimates get TICKER... [--tickers-file F] [--endpoints LIST] [--momentum] [--refresh]`, `fa estimates show TICKER [--price P]`,
`fa earnings calendar [TICKER...] [--tickers-file F] [--from D] [--days N]`, `fa earnings dates TICKER... [--tolerance N]`.

Rules. One request each 2 seconds (the shared client). Default 3 requests for each ticker (forecast, surprise, targets); `--momentum` adds a fourth. One calendar request for each weekday of the scan.
HTTP 429, HTTP 403 or a reply that is not JSON: wait 60 seconds one time, then stop the source for the run; the cache stays. A failed source never deletes cache data. The files of today are not asked again.
A snapshot of the Python code for the same date is never replaced. The source order for `fa analyze`: Nasdaq files for EPS, targets, surprises and dates; FMP for revenue estimates when a key exists.
`fa analyze` and `fa report` make no request to this source; they read the cache.

Read-only functions for other modules: `consensus::forward_pe(price, eps)` and `ConsensusEstimates::eps_growth()`. The DCF model files and `dcf/auto.rs` are not changed;
`fa estimates show` does not print the growth start of `fa dcf auto` (no cheap function to call).

## Keyed data sources (`market/alpaca.rs`, `fred_api.rs`, `option_history.rs`, 2026-11-19, branch `claude/keyed-data-sources`)

Owner request: solid data sources that a person syncs whenever the person wants. No process runs all the time. So each data set needs a source
that gives the PAST on request through a published API. All loaders of this section follow the official documentation (read on 2026-11-19).
No key was available when the loaders were written. On 2026-11-20 the owner made a free Alpaca paper key and the loaders of Alpaca ran against the real server (results below; the facts of the table are corrected). Tiingo ran on the same day (#206, #284). The FRED API has not run. The tests use text that follows the documentation and, for Alpaca, real reply text. The table of all data sets is
`docs/DATA-SOURCES.md`.

### Alpaca Market Data API (free plan)

Facts from the documentation. "Confirmed" means that a page states it.

| Fact | State | URL |
|---|---|---|
| (a) The free plan has the feed IEX for stocks and the feed "indicative" for options. 200 requests each minute. Stock history: the pages say "Since 2016" and "7+ years"; MEASURED 2026-11-20: the daily history starts 2020-07-27 (1,589 rows), the intraday history starts at the same time (a week of 2016: 0 bars; a week of August 2020: 390 bars). The close is the IEX close (MSFT 2026-11-18: 508.505 at Alpaca, 508.29 official at Nasdaq) and the volume is the IEX volume. The history limit of the free plan is "latest 15 minutes": the page does not say that a request for this window fails. The reference says that without real-time access `end` defaults to 15 minutes ago. The program never asks for the latest 15 minutes (it leaves `end` out when the window reaches today) | confirmed, except the exact failure | `https://docs.alpaca.markets/docs/about-market-data-api`, `https://docs.alpaca.markets/reference/stockbars`, `https://alpaca.markets/data` |
| (b) Indicative option feed: free; "quotes are not actual OPRA quotes", trades delayed 15 minutes. The chain snapshot has bid, ask, last trade, implied volatility and the greeks. Historical option bars exist "since February 2024" (`GET /v1beta1/options/bars`, up to 100 contracts in a request, 10,000 points in a page). Historical option quotes: no page found | bars and snapshots confirmed; quotes not confirmed | `https://docs.alpaca.markets/docs/historical-option-data`, `https://docs.alpaca.markets/reference/optionbars`, `https://docs.alpaca.markets/reference/optionchain` |
| (c) Index options (SPX, VIX). No page says that they are covered | MEASURED 2026-11-20: they ARE on the free plan. Chain `SPX`: 10,412 quotes, 20 expiries to 2031-12-19; chain `SPXW` (weekly root): 21,634 quotes; `VIX`: 1,010 quotes; `VIXW`: 500 quotes. Bars of `SPX`, `SPXW` and `VIX` contracts of 2024-06-07 exist. No index level, and no implied volatility in the chain (#280). The list of expired index contracts is short (#281) | `https://docs.alpaca.markets/docs/real-time-option-data`, `https://docs.alpaca.markets/docs/options-trading-overview` |
| (d) Account. Paper accounts can call the Market Data API with their own keys. No page says that the free data needs a funded account. Whether Alpaca asks for a document when a person opens a paper account is not in the pages read | paper keys confirmed; funding rule not stated | `https://docs.alpaca.markets/docs/authentication` |
| (e) Headers `APCA-API-KEY-ID` and `APCA-API-SECRET-KEY` (or HTTP Basic). Data address `https://data.alpaca.markets` (`/v2/stocks/bars`, `/v1beta1/options/bars`, `/v1beta1/options/snapshots/{underlying}`). The list of option contracts is on the trading address `https://paper-api.alpaca.markets/v2/options/contracts` | confirmed | `https://docs.alpaca.markets/docs/authentication`, `https://docs.alpaca.markets/reference/get-options-contracts` |
| (f) Terms for stored data and automated use. The data pages do not state them. The disclosure page lists the agreements "NASDAQ OMX Global Subscriber Agreement" and "NYSE Market Data Display Services Agreement" by title only; they were not read | not confirmed | `https://alpaca.markets/disclosures` |

Modules:

| Module (`rust/src/market/`) | Function |
|---|---|
| `keys.rs` | Reads a key from the environment, else `.env`. `scrub_all` and `scrub_api_key_param` take key values out of a text. `urlenc`. |
| `alpaca.rs` | The client (keys in headers, 0.4 s between requests, request counter, paging by `next_page_token`), the parsers (bars, contracts, chain snapshots), the daily rows, the intraday rows, and the price source `Alpaca`. |
| `price_source.rs`, `price_cache.rs` | The trait `PriceSource` got `batch_size`, `fetch_many` and `detail`. `price_cache::update_many` updates many tickers with one request for each group of tickers (the overlap rule, the "no mixing" rule and the 90% rule stay). `prices.meta.json` has the new field `detail` (`feed=iex adjustment=split`). |
| `intraday.rs` | `fa prices intraday`: the range goes in calendar months; each month is one request; the file is written after each month. |
| `option_iv.rs` | Black-76 price, the implied volatility (bisection), the forward from put-call parity (with a spot, or without one: `forward_from_parity_blind`). |
| `option_history.rs` | The trait `OptionHistorySource`, the builder `build_range` (selection, forward, implied volatility, snapshot of each date), the Alpaca source, the index functions (roots, level, symbols from the rules, the VIX history file) and `fill_from_mids` (forward, level and volatility of a live index chain). |
| `options_cli.rs` | `fa options snapshot` and `fa options history`. |
| `fred_api.rs` | The FRED API loader (see below). |

Commands and rules:

1. **Daily prices.** `fa prices get TICKER... --source alpaca`. Alpaca is the LAST source when no source is forced (order: Tiingo, Yahoo, Nasdaq, Alpaca; #284): its close is the IEX close and its history starts 2020-07-27. It is the check source and the fallback. The call is `GET /v2/stocks/bars` with `timeframe=1Day`, `adjustment=split`, `feed=iex`,
   100 symbols in a request, `limit=10000`, and paging. The series is split adjusted and not dividend adjusted (the same series as the Nasdaq source). The cache record says
   `adjusted: splits` and `detail: feed=iex adjustment=split`. The bar of today stays out until 16:20 New York time.
2. **Intraday bars.** `fa prices intraday SYMBOL --interval 5m|1m --from D [--to D] [--feed iex|sip]` writes `data/raw/trading/<SYMBOL>_5m.csv` (or `_1m.csv`) in the form of
   `fa trade fetch`. The file keeps the regular session and complete bars. A stopped run continues from the last cached day when the cache starts at `--from`.
3. **Option chain now.** `fa options snapshot SYMBOL --source alpaca` writes `data/raw/<T>/options/<date>.json` with `source: alpaca`, `feed: indicative`, `quote_kind: indicative`. For SPX and VIX
   the chain has two roots, the monthly and the weekly root (`SPX,SPXW`, `VIX,VIXW`); the roots are the default because a person who asks for the SPX chain expects the weekly options too (`--roots A,B` sets
   them). An index has no price at Alpaca and the chain has no implied volatility. The program takes the forward of each expiry from put-call parity of the mids of bid and ask, sets the level of the
   underlying to the forward of the nearest expiry (1 to 10 days; 7,085.49 for SPX on 2026-11-20, SPY 706.8 times 10 is 7,068), and sets the implied volatility (Black-76; index options are European) of each
   out-of-the-money or near-the-money quote that has none. The rate is the Treasury par curve (`--rate R` for a flat rate). Measured 2026-11-20: SPX 32,046 quotes, 33 requests, 15,605 volatilities;
   VIX 1,510 quotes, 3 requests, 629 volatilities. The SPX volatility of the 2026-12-18 expiry near the money is 24.7 to 25.5%; the SPY one is 24.6 to 26.0%.
4. **Option surface of past dates.** `fa options history SYMBOL --date D` (or `--from D --to D`). Steps for each month of the range: (1) the level of the underlying (the close for a stock; for an index see below);
   (2) the contract list with an expiry from the first day to the last day plus `--max-days`, and a strike in the moneyness range of the lowest and the highest level (status `active` and `inactive`);
   (3) the daily bars of the contracts in groups of 500 (1,000 worked on 2026-11-20) for the whole month; (4) for each date: the filter (a close, a volume above zero, the expiry after the date, the selection), the forward of each
   expiry from put-call parity, the implied volatility, and the snapshot file. The file has `quote_kind: close`. The plan (contracts, requests) is printed to stderr before the closes are asked for; `fa options history ... --plan` prints it and gets no closes.
   Default selection: expiry up to 120 days, strike 0.9 to 1.1 times the level. The rate is the Treasury par curve of the date (linear in the time to expiry); `--rate R` gives one flat rate.
   **Index (SPX, VIX).** Level of SPX: the SPY close times 10 (a proxy: it picks the strike range only; the forward of each expiry comes from parity, and the level of the snapshot is the parity forward of the nearest expiry).
   Level of VIX: the close of the Cboe file `data/raw/vix/VIX_History.csv` (`fa sync --only vix`); the options of the VIX are on the VIX future of their expiry, so the parity forward is the future (12.95 for the
   expiry 5 days away on 2024-06-07, the index was 12.22). `--level SYMBOL:RATIO` gives another proxy (not tested for NDX and RUT). A missing level is an error text. The list of expired index contracts holds the last
   six months only (#281), so for an older expiry the program makes the symbols from the rules (SPXW on each weekday, SPX on the third Friday, strikes every 5 points; VIX on Wednesdays, every 0.5 point) and asks for
   their bars; a symbol that does not exist has no bar.
5. **Rules for the files.** A snapshot with real bid and ask is never replaced by closes. A file of closes is replaced only with `--refresh`. The new fields `source`, `feed`, `quote_kind` and `note` are
   optional: a file of the older form (Cboe) has none of them, and the older readers read both forms.
6. **Keys.** `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY` from the environment, else `<repo>/.env`. They go in headers only. Each error text has them taken out (tests).

Request counts. The first two lines are measured with fixtures (a fake server with the paging rule of the documentation). The other lines are MEASURED with the real server on 2026-11-20 unless the text says "estimate".

| Job | Requests | Time |
|---|---|---|
| 1,000 tickers, daily update (7 rows each) | 10 (fixture test) | 4 s |
| 1,000 tickers, full history (about 1,590 rows each since 2020-07-27) | about 160 (fixture rule: 10 groups of 100 symbols, about 16 pages each; estimate) | about 1 minute |
| One year of 5-minute bars of one symbol | 12 (one for each month); 19,656 rows at most (estimate) | 5 s |
| SPY chain now | 14 (12,500 quotes) | 6 s |
| SPX chain now (roots SPX and SPXW) | 33 (32,046 quotes) | 18 s |
| VIX chain now (roots VIX and VIXW) | 3 (1,510 quotes) | 2 s |
| One past date of SPY, default selection (120 days, 0.9 to 1.1) | 35: 1 (closes of the underlying) + 3 (contract list: 15,220 contracts) + 31 (bars, 500 contracts in a request); 2,038 contracts had a close | 17 s |
| The same, selection 45 days and 0.95 to 1.05 | 3 + 6 = about 9 (2,952 contracts; 1,196 had a close) | 15 s with groups of 100 (33 requests) |
| The same, selection 365 days and 0.8 to 1.2 | 136 (63,498 contracts: 9 + 127) | about 1 minute |
| One past date of SPX, 45 days and 0.95 to 1.05 | 15 (6,848 symbols from the rules; 2,724 had a close) | 8 s |
| One past date of SPX, default selection | about 75 (36,808 symbols) | |
| One past date of VIX, 0.7 to 1.5 | 2 (782 symbols; 139 had a close) | 2 s |
| One year of dates, one date at a time (SPY, default) | about 252 x 35 = 8,800 (estimate) | 45 minutes at 200 each minute |

The first estimate of the first version (65 to 125 requests for one date) was wrong by a factor of 5 to 10 for the selection with 365 days and 0.8 to 1.2 (136 requests with groups of 500, 635 with groups of 100). The list of
expired contracts is filtered by expiry and strike only: most listed contracts had no trade on the date (SPY: 1,196 of 2,952; 2,038 of 15,220). The list gives `close_price`, `close_price_date` and `open_interest`, but
they do not tell which contracts traded on a date (1,076 of 1,080 contracts with a bar had a `close_price`, and 2,990 of 3,148 contracts had one), so the list cannot reduce the number of closes to ask for. What reduces the
requests: groups of 500 contracts in a request, and a narrower selection. The contract count for a date is not the limit of the pages of closes: a request returns up to 10,000 bars.

### FRED API with a key

Official documentation: `https://fred.stlouisfed.org/docs/api/fred/series_observations.html` (endpoint `https://api.stlouisfed.org/fred/series/observations`; parameters `series_id`,
`api_key`, `file_type=json`, `observation_start`, `limit` up to 100,000, `offset`), `.../api_key.html` (32 lowercase characters; a key for each application; each user has an own key; the account is
at `fredaccount.stlouisfed.org`), `.../errors.html` (120 requests each minute, then HTTP 429 and a possible block; an error is `{"error_code", "error_message"}`), `.../terms_of_use.html`
(no number for storage; no application that replaces the FRED site; third-party series need the permission of the owner for more than personal use).
With `FRED_API_KEY` in the environment or in `.env`, `Rates::fred_text` asks the API first and turns the reply into the CSV text of the keyless address, so the cache files
(`data/raw/bonds/fred_<ID>.csv`) and the parsers stay as they were. If the API fails, the program tries the keyless address. The key is a URL parameter: the program takes the key value
and each `api_key=` value out of each error text and log line (tests). The code does not know how the API writes a missing value; it skips `.` and the empty text.

### Massive (the former Polygon.io), paid: the plan for the owner's decision

Nothing here is built, and no key was used. The pages below were read on 2026-11-19. Prices and plans come from the research file (pricing pages that change).

| Fact | State | URL |
|---|---|---|
| Option chain snapshot `GET /v3/snapshot/options/{underlyingAsset}`: `limit` default 10, maximum 250; greeks, implied volatility, open interest, `day` bar; Starter and higher (15 minutes late); no past dates (it is a snapshot). The page has no `I:SPX` example | index underlying not confirmed | `https://massive.com/docs/rest/options/snapshots/option-chain-snapshot` |
| Option aggregates `GET /v2/aggs/ticker/{optionsTicker}/range/{multiplier}/{timespan}/{from}/{to}`: one ticker in each request; `limit` up to 50,000; ticker form `O:...`; records from 2014-06-02; depth by plan: Basic 2 years, Starter 2 years, Developer 4 years, Advanced and Business all | confirmed | `https://massive.com/docs/rest/options/aggregates/custom-bars` |
| Index aggregates `GET /v2/aggs/ticker/I:SPX/range/...`: fields `o h l c t`; depth "all history", records from 2023-02-14; the page does not list which index tickers a plan has (SPX and VIX not named) | SPX, VIX not confirmed | `https://massive.com/docs/rest/indices/aggregates/custom-bars` |
| Contract list `GET /v3/reference/options/contracts`: `as_of` (a past date), `expired=true`, `underlying_ticker`, `expiration_date`, `strike_price`, `limit` up to 1,000. The page does not say if SPX, SPXW, VIX or VIXW are listed | index options not confirmed | `https://massive.com/docs/rest/options/contracts/all-contracts` |
| Flat files: S3 address `https://files.massive.com`, bucket `flatfiles`, prefix `us_options_opra`, gzip CSV, one file for each day; options day aggregates have `ticker, volume, open, close, high, low, window_start, transactions`; "Included in select Options plans": Starter, Developer, Advanced, Business (not Basic); depth as the plan; the file of a day is ready about 11:00 New York time the next day. The exact key of the day file is not on the pages read | partly confirmed | `https://massive.com/docs/flat-files/quickstart`, `https://massive.com/docs/flat-files/options/day-aggregates` |
| Request limit: "Five requests a minute per asset class you have not subscribed to, and no limit on the ones you have" | confirmed | `https://massive.com/knowledge-base/article/what-is-the-request-limit-for-massives-restful-apis` |
| VIX options (root VIX, VIXW) | not confirmed | no page found |
| Terms for stored data after a cancelled plan, and for automated use | not confirmed (the terms page names "Individual Use" and "Business Use" products and refers to other documents) | `https://massive.com/legal/terms` |

Plan for one year of SPX surfaces (the trait `OptionHistorySource` fits both forms: `bars_per_request` is 1 for the REST form and very large for the flat-file form):

1. **Flat files (best; 253 requests).** Download one file for each trading day (`us_options_opra/day_aggs_v1/YYYY/MM/YYYY-MM-DD.csv.gz`; the path is not confirmed) with an S3 client: 252 files. Keep the rows
   of the roots SPX and SPXW (the ticker form `O:SPXW...` is not confirmed). Get the underlying close with one request `GET /v2/aggs/ticker/I:SPX/range/1/day/{from}/{to}` (limit 50,000). Use `build_range`: the
   source answers `closes` from the files, and the forward comes from put-call parity (SPX has no tradable spot; the parity forward is the right forward). Plan needed: Options Starter ($29 each month; 2 years)
   or Developer ($79; 4 years), and an Indices plan for `I:SPX` (Indices Starter, $49). All numbers: research file; read the pricing page again before a payment.
2. **REST (not advised; at least 100,000 requests).** For each date: `GET /v3/reference/options/contracts?underlying_ticker=SPX&expired=true&as_of=D&limit=1000` (about 3 to 6 pages), then one `aggs` request for each contract
   for the whole year. With a selection (365 days, 0.8 to 1.2 of the spot) the year has more than 100,000 different SPX and SPXW contracts (an estimate: the daily expiries of SPXW alone are 252 each year with about 300 strikes). The paid plans have no request limit, but 100,000 requests take
   hours.
3. **Test before a payment, with the free key.** Options Basic (free, 5 requests each minute for each asset class) has reference data and 2 years of aggregates. With this key the owner can check for no money if the contract
   list has `SPX`, `SPXW`, `VIX` and `VIXW`, and if `O:SPXW...` has aggregates. Then the owner knows the one fact that decides the purchase: index options coverage.

### Tests and counts

`cargo test --manifest-path rust/Cargo.toml --locked`: 473 Rust tests (455 + 17 + 1; 2 more are ignored) when this section was written; the later count is in the next section. 36 tests were new (alpaca 10, intraday 3, option_iv 3, option_history 5, options_cli 4, fred_api 6,
keys 3, price_cache 2). The traps are #230 to #244 of `docs/HIDDEN-ASSUMPTIONS.md`.

## `fa sync`: one command for all cached data sets (`rust/src/sync/`, `rust/src/hostnet.rs`, 2026-11-20, branch `claude/data-sync`)

`fa sync [TICKER...] [--tickers-file F] [--only A,B] [--skip A,B] [--force] [--dry-run] [--max-minutes N]` and `fa sync --status [TICKER...]`.
The sync runs on request only. It brings the old data sets up to date and leaves the current ones. It calls the functions of the existing data commands; it has no copy of their logic and changes no cache rule.
The table of the data sets, with their names, maximum ages and hosts, is in `docs/DATA-SOURCES.md`. The traps are #245 to #259 in `docs/HIDDEN-ASSUMPTIONS.md`.

| Module | Function |
|---|---|
| `sync/dataset.rs` | The 13 data sets (`sec`, `filings`, `prices`, `intraday`, `estimates`, `earnings`, `options`, `rates`, `gsw`, `fred`, `futures`, `vix`, `bonds`), their maximum ages (one table), hosts and the request pauses of the hosts (`HOSTS`). |
| `sync/config.rs` | The optional file `sync.toml`: ages, the default list of tickers, the option symbols, the intraday symbols. A wrong key is an error. |
| `sync/calendar.rs` | Market days: weekends and the holidays of the New York Stock Exchange. A maximum age of 1 day counts market days. |
| `sync/ledger.rs` | `data/raw/_sync/checked.json`: the date of the last good check of each item (for an item that has no record in its own cache). Not data; it can be deleted. |
| `sync/fresh.rs` | Old or current, from the cache files and the ledger. No request. `old_data_warning` is the one function of the age check that the value commands use. |
| `sync/plan.rs` | The plan: the items to run, the counts (current, old, no data), the requests for each host, the estimate of the time (the longest queue), the hints for missing keys. |
| `sync/exec.rs` | The executor: one thread and one queue for each host, all hosts at the same time; a stable order; the stop flag and the time limit; the summary of each data set. It does not know the network (a test gives a fake runner). |
| `sync/runner.rs` | The real runner: one job for each data set. |
| `sync/interrupt.rs` | Ctrl+C (Unix signal, Windows console handler). |
| `hostnet.rs` | One request slot for each host group: the pause of the host, never two requests to one host at the same time, the counts, the stop of a host for the run. |

Flow. `fa sync` reads the config, the ticker list, the keys and the SEC contact (`sec_user_agent`). It makes the plan from the cache files (no request). `--status` and `--dry-run` print the plan and end. Otherwise the executor starts
one thread for each host. A thread runs the items of its host one after the other. At the end the summary has one line for each data set and one line for each host, the list of what stays old, and the command to run again.

Rules.

- Source order and fallbacks are the order of the existing code. The price cache keeps its source. A missing key is not an error; the summary says which key adds which source (the value of a key is never printed).
- A block (HTTP 429, HTTP 403, a block page) stops that host for the run. Two connection failures in a row (timeout, closed connection) stop it too. The cache stays; the other hosts go on. A failure of one ticker does not stop the others.
- The time limit of one request is 30 seconds, 60 seconds for the hosts with large files, and 90 seconds for the Atlanta Fed (`hostnet::timeout_of`). The sync has no wait after a block: the old 60-second waits of the price and the estimates clients are 0 in the sync.
- Exit code 0 when no data set failed as a whole. A block of a fallback source alone, a data set that is "not available yet" (FedInvest at the end of the day), a skipped data set, and a stop by Ctrl+C or `--max-minutes` do not set the code 1.
- Without a SEC contact (error of `sec_user_agent`: no variable and no good file), `sec`, `filings` and the SEC part of `earnings` are skipped with one line.
- The value commands do not start a sync: `fa dcf auto`, `fa verdict` and `fa estimates show` print one line `warning: old data for TICKER: ...; run: fa sync TICKER` when their data is older than its maximum age.

Tests. They use a fake runner and temp folders (no network): the plan from a cache state (old, current, weekend, holiday), `--only`, `--skip`, `--force`, a blocked host, a failed ticker, the stop after N failures, the pause of each host,
that two hosts overlap in time, the summary counts and the exit code, "not available yet", the config file, the continue after a stop, keys, a missing SEC contact (the status line names the place, never the contact), and that `--status` makes no request.

## The company report (`rust/src/report/`, `fa report`, 2026-11-19)

`fa report TICKER [--price P] [--out FILE]` writes one HTML page. It is the Rust form of
`scripts/visualize_dcf.py` (deleted). The page has the same sections, in the same order, and the same
texts, tables and numbers as the Python page. Each plotly figure has the same data arrays and the
same layout keys.

| Module (`rust/src/report/`) | Function |
|---|---|
| `mod.rs` | The command, the inputs (company file, price history, option snapshot, earnings dates, driver model), the sections (header, price and valuation, fundamentals, aggregate DCF, driver model, backtest, options market, footer), the tables, the chips, the page. `make_report` is the function that the viewer calls. |
| `figs.rs` | The 19 figure builders. Each gives the `data` and the `layout` of one plotly figure as JSON (`serde_json`). The stratified path sample uses `Pcg64::choice_set` (numpy `Generator.choice` without replacement, Floyd's algorithm). |
| `fmt.rs` | The number formats of Python (`{:,.1f}`, `{:+.0%}`, `{:,.4g}`, `{:g}`, `round`, `html.escape`), so a table cell has the same text. |
| `style.css` | The style sheet of the Python page (the "Ratings-desk blueprint" tokens). |
| `tests.rs` | The parity tests with the fixtures of `rust/tests/fixtures/report/`. |

**Charts and network.** The page does not carry the plotly.js library. It loads version 4.1.1 (the
version that the Python library plotly 7.1.0 inlined) from `https://cdn.plot.ly/plotly-4.1.1.min.js`.
The page needs the network when a person opens it. Without the network, a chart shows a note. The Python page was
self-contained (about 3.5 MB of library in each file) and worked offline. An option that inlines a
local copy is an open point (board ticket `report-rust`). The page does not carry the plotly Python
template either; the figures set `autotypenumbers: "strict"`, `automargin`, `ticks` and the title
standoff themselves (the template values that change the picture). The bars have no template outline.

**Parity.** The fixtures were recorded from the Python script before the Rust code was written, for IBM,
JNJ, KO (no price history, no driver model) and KO with a short price history and an empty option
chain. The tests compare the body of the page (each text, cell and chip, with a marker for each
figure) and each figure (each number within 1e-9 relative; an array of more than 200 numbers by its
digest and 40 sampled values). The words that name a Python command or file are other words in the Rust
page (list: `TEXT_CHANGES` in `tests.rs`). `docs/HIDDEN-ASSUMPTIONS.md` #147 to #151 list the traps.

**Viewer.** `/api/report/make`, `/api/report/info` and `/report/file` call `make_report`. The JSON fields
are the same as before.

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

`cargo test --manifest-path rust/Cargo.toml --locked`: 591 Rust tests (573 + 17 + 1; 2 more are ignored) (the `fa` crate).
`cargo test --manifest-path rust/mapcheck/Cargo.toml`: 100 Rust tests. The suites make no network calls and start no
Python program; HTTP is replaced by a function that the test gives, and files are in a temp folder.
No Python test remains (the last 211 Python tests were deleted on 2026-11-19 after each case had a Rust test).
