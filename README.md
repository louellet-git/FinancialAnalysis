# FinancialAnalysis

This repo helps two persons make investment decisions and trade options. It has two
parts:

- **Valuation.** A Python pipeline gets company financial data from SEC EDGAR, adds
  market data (prices, earnings surprises, consensus estimates, option chains), and
  calculates a range of values for each share with a Monte Carlo DCF. Then it compares
  the range with the market price.
- **Options trading research.** A backtester and a paper trader for SPY/QQQ option
  strategies.

The repo gives analysis only. It does not send orders to a broker. It is not investment
advice.

Agents: read `CLAUDE.md` first. Open work: `PLANNING/board/`.

## Start

```bash
pip install -r requirements.txt -r requirements-dev.txt
python -m pytest                       # 469 tests, no network
cargo test --manifest-path rust/mapcheck/Cargo.toml   # 45 Rust tests
```

Run all commands from the repo root.

## From a ticker to a decision

```bash
# 1. Get the data. Output: data/processed/IBM_financials.xlsx (nine tabs)
python -m scripts.analyze IBM

# 2. Get the value range and the verdict against a price (the price is an input)
python -m scripts.verdict IBM --price 228.53

# 3. Make the one-page HTML report. Output: data/processed/IBM_dcf.html
python -m scripts.visualize_dcf IBM

# 4. Examine how the mechanical model performed on past years
python -m scripts.backtest IBM
```

Step 2 is available only for a ticker that has `analysis/<TICKER>/driver_model.py`.
On 2026-11-19 these are IBM (reviewed) and JNJ (first pass, not reviewed).

The verdict is the percentage of simulated values that are below the price:

| Chance that the price is too high | Verdict |
|---|---|
| Less than 25% | Strong opportunity |
| 25% to less than 40% | Opportunity |
| 40% to 60% | No edge |
| More than 60% to 75% | Overpriced |
| More than 75% | Strongly overpriced |

Limits that you must know before you use a verdict:

- The P10 and P90 values are too narrow (backlog item T-DCF-1). The chance value is
  thus too sure.
- A verdict is only as good as its driver model. The JNJ model has no review.
- The IBM report is from July 2026. Its watch list is not graded against the Q2 and Q3
  results (board ticket `ibm-q2-grading`).

## All valuation commands

```bash
python -m scripts.analyze IBM [--refresh] [--no-market] [--no-options] [--no-estimates] [--out custom.xlsx]
python -m scripts.import_financials IBM [--refresh]      # JSON store only, no xlsx
python -m scripts.import_edgar IBM 2026-07-01 [--output-dir data/raw] [--lookback-months 18] [--lookahead-months 6]
python -m scripts.run_dcf PANW [--simulations 10000] [--years 10] [--growth 0.15] [--wacc 0.10] [--terminal-growth 0.03] [--seed 7]
python -m scripts.backtest IBM [--horizon 3] [--simulations 10000] [--seed 7]
python -m scripts.visualize_dcf IBM [--price 195.40] [--out report.html]
python -m scripts.verdict IBM --price 228.53 [--simulations 10000] [--seed 7] [--rf-date today] [--rf-base-date 2026-07-16] [--rating A-] [--json]
python -m financial_analysis.bonds [snapshot|spread|yield|over10y|moodys|hqm|check] [--rating A-] [--json]
python -m financial_analysis.rates [snapshot|yield|curve|history|nyfed|inflation|fred|check] [--json]
python -m scripts.mapping_audit IBM JNJ    # each input value with its source
python -m scripts.mapping_audit --panel    # mapping summary for 32 tickers
python -m analysis.IBM.dcf_drivers_run     # IBM version 4 headline numbers (seed 7)
python -m analysis.IBM.dcf_scenarios       # IBM version 3 (historical)
python -m analysis.JNJ.first_pass_run      # JNJ first pass
```

`scripts.run_dcf` prints an enterprise value for each share. It does not subtract net
debt. Do not compare this number with a share price.

Remote data has a disk cache in `data/raw/TICKER/`. Option chains and consensus
estimates are saved as dated snapshots, because yfinance gives only current data. A run
on the same day reads the disk; `--refresh` gets new data.

## Standard input variables

Companies use different XBRL tag names for the same line. For example, interest expense
is `InterestExpense` for IBM, `InterestExpenseNonoperating` for Microsoft and
`InterestExpenseDebt` for Chevron. The registry `financial_analysis/xbrl_tags.py` gives
each line one standard variable name (132 variables). The models read only these names.
The registry data is in `financial_analysis/standard_variables.toml`; Python and Rust
read this one file.

- Each variable has a list of tags with the same meaning. The first tag that has a
  value is used.
- If no tag has a value, a formula over other variables is used (for example
  `total_debt = debt_current + long_term_debt`).
- Each value has a recorded source: the tag, or the formula. The xlsx file shows it in
  the `source` column. `python -m scripts.mapping_audit TICKER` prints it.
- An annual row is always a full fiscal year. A quarterly row is always one quarter.
- Free cash flow is operating cash flow less `capex_total`. `capex_total` is equipment
  (`capex`) and capitalized software. Purchased intangibles are not included. This is
  rule R2 in `PLANNING/RULES.md`, with the reasons.
- Tags with no standard variable stay in the `Raw` tab.

Before you use a variable in a model, run the audit for the ticker and read the source
of the value. The audit also shows where the data does not agree. Total debt is the
most frequent problem: on the 32-ticker panel the two estimates of total debt disagree
for 5 companies, because companies tag debt in ways that overlap.

Limits: the SEC data source has no segment data and no company-specific tags. Banks,
insurers and REITs have a different statement structure; the audit marks them.

### Test of the mapping on many tickers (Rust)

`rust/mapcheck` is a Rust program that maps and checks many companies in parallel:
472 companies in 2.8 seconds. It answers four questions for each company: can the DCF
use the inputs, was a value lost, was a value counted two times, and do the statements
agree. A second command compares the Rust result with the Python result.

```bash
cargo build --release --manifest-path rust/mapcheck/Cargo.toml
python -m scripts.fetch_facts --tickers-file rust/mapcheck/universe/batch50.txt   # get the data
rust/mapcheck/target/release/mapcheck --tickers-file rust/mapcheck/universe/top500.txt
rust/mapcheck/target/release/mapcheck IBM --detail
python -m scripts.mapping_parity --sample 120                                     # Rust against Python
python -m scripts.mapping_crosscheck --tickers-file rust/mapcheck/universe/batch50.txt   # against Yahoo Finance
```

Result on 2026-11-19, 472 of the 500 largest SEC registrants: 312 are US companies
with a standard statement structure. Of these, 48 pass, 240 have a warning that a
person must examine, and 24 fail (15 of these have no standard tag for capital
expenditure). Read `rust/mapcheck/README.md` for the checks and the results.

Two results that you must know before you add variables in a model:

- For 166 of the 312 companies, a variable is a detail inside a different line (for
  example income taxes payable inside accrued liabilities). If you add the two, you
  count the amount two times. `mapcheck TICKER --detail` shows these as `nested`.
- For 47 of the 312 companies, the estimates of total debt disagree.

A third test compares the core inputs with Yahoo Finance, an independent source
(`scripts.mapping_crosscheck`). Result for the 50 new tickers on 2026-11-19: eight inputs
agree for 98% to 100% of the companies (pre-tax income, tax, net income, operating
cash flow, cash, total assets, total liabilities, equity, share-based compensation),
revenue for 96%, share count for 94%, total debt for 83%, capital expenditure for 75%
and depreciation and amortization for 74%. Do not use the last three inputs for a
ticker before you examine them.

The share count of some filers has no scale (McDonald's reports 716.4 for 716.4 million
shares). The importer corrects a factor of 1,000 or 1,000,000 with net income divided
by EPS, and records the correction in the source.

### The statements of a filing (filing-level data)

The SEC source that the importer uses has standard tags only, and it does not say which
line is a part of which line. Each 10-K has this information. `scripts.filing_tree` gets
the latest 10-K of a ticker and prints the three statements as the company structures
them: each line with its parent, its sign, its value, and its tag. Company-specific
tags are included.

```bash
python -m scripts.filing_tree VZ                    # the three statements
python -m scripts.filing_tree VZ --statement cash   # one statement
python -m scripts.filing_tree IBM COP --focus       # only the capex, depreciation and debt lines
```

Use `--focus` before you trust capital expenditure, depreciation and amortization, or
total debt for a ticker. It shows the lines of the filing and a value from these lines.
Examples: IBM capital expenditure is $1.09B in the standard variable and $1.74B in the
filing (IBM has a company tag for software); the ConocoPhillips capex has only a company
tag ($12.55B).

Comparison with Yahoo Finance for the 50 new tickers (2026-11-19):

| Input | Standard variables agree | Filing lines agree |
|---|---|---|
| Capital expenditure | 73% | 92% (78% and 14% more with purchased intangibles) |
| Total debt | 80% | 90% |
| Depreciation and amortization | 73% | 81% |

Limits: the standard variables do not use the filing values yet (board ticket
`filing-values-in-variables`). The command finds the lines by their tag names, thus a
line can be missing or incorrect; read the printed lines. Facts with a segment (60% to
70% of the facts in a filing) are counted but not used.

## Interest rates and inflation

`financial_analysis/rates.py` gets US rates from the publishers. All values are
decimals (0.0507 is 5.07%). Each value has the date of its observation.

```bash
python -m financial_analysis.rates                 # snapshot and a cross-check against FRED
python -m financial_analysis.rates yield --tenor 10Y --date 2026-07-16 --json
python -m financial_analysis.rates inflation
```

The driver models keep their own `rf`. To use the Treasury 10-year rate of a date, add
`--rf-date` to `scripts.verdict`:

```bash
python -m scripts.verdict IBM --price 231.09 --rf-date today --rf-base-date 2026-07-16
```

The output shows these items:

- The Treasury 10-year rate and the model rate.
- The rate as real rate plus breakeven inflation (nominal yield minus TIPS yield).
- With `--rf-base-date`, the change since that date as real change plus inflation change.
- The 5y5y forward inflation and the 12-month CPI change.
- The terminal growth as 5y5y inflation plus real growth.

Only `rf` changes. The growth drivers are nominal and keep the model values. If the
inflation part of a rate change is large, examine the growth drivers too.

### Corporate bonds and the cost of debt

The engine uses `kd = rf + kd_spread`. `financial_analysis/bonds.py` gives market
values for `kd_spread`.

```bash
python -m financial_analysis.bonds                       # all ratings, Moody's, HQM curve, check
python -m financial_analysis.bonds over10y --rating A-   # the market kd_spread for a rating
```

`over10y` is the yield of the ICE BofA index for the rating minus the Treasury 10-year
yield on the same date. This is the definition of `kd_spread` in the engine. The rating
can be S&P (`A-`) or Moody's (`A3`). The index has no notches, thus `A-` uses the `A`
index.

To use the market spread in a verdict, add `--rating` to a run with `--rf-date`:

```bash
python -m scripts.verdict JNJ --price 200 --rf-date today --rating AAA
```

The output shows the market spread, the model spread and the pre-tax and after-tax cost
of debt. The driver models do not change. Read `docs/HIDDEN-ASSUMPTIONS.md` #45 before
you use `--rating` for IBM.

### Rust interface

The `rust/` crate calls the Python code and gives typed results. Python stays the only
implementation.

```bash
cd rust
cargo run --bin fa -- rates yield --tenor 10Y
cargo run --bin fa -- verdict IBM --price 231.09 --rf-date today --rating A-
cargo run --bin fa -- bonds over10y --rating A-
cargo run --bin fa -- run scripts.analyze IBM     # any Python entry point
cargo run --example dcf_rates -- IBM 231.09 A-    # library example
```

Set `FA_PYTHON` to select the Python interpreter and `FA_REPO` to select the repository.

## The two DCF engines

**Classic engine**, `financial_analysis/dcf.py`. It samples revenue growth, free-cash-flow
margin, WACC and terminal growth directly. The presets (bear, base, bull) and the net
debt and share count for each ticker are in `financial_analysis/scenarios.py`. The IBM
entries there are version 3 on purpose (`docs/HIDDEN-ASSUMPTIONS.md` #13).

**Driver engine**, `financial_analysis/driver_dcf.py`. The top-level items (revenue,
margin, WACC, terminal value) are fixed formulas. All randomness is in named drivers.
Each driver has a written reason. Each path uses one draw for each driver, with a
correlation matrix. WACC is calculated for each path. Terminal growth is limited to
`WACC − 2%` for each path. The model for a ticker is in
`analysis/<TICKER>/driver_model.py`.

**Backtest**, `financial_analysis/backtest.py`. For each past fiscal year end plus 90
days, it uses only the data that was filed before that date, projects revenue and free
cash flow for 1 to 3 years, and compares the projection with the actual results. It
tests operating projections only: no WACC, no terminal value, no price.

More detail: `docs/ARCHITECTURE.md`.

## Options trading research

```bash
python -m scripts.trade_backtest --fetch              # download market data
python -m scripts.trade_backtest                      # all bots, leaderboard
python -m scripts.trade_backtest s14b_spy_trend_calls -v
python -m scripts.trade_backtest --max-risk 250       # larger risk for each trade
python -m scripts.trade_paper                         # one paper-trade step for each bot
```

The package `trading/` contains 22 bots. Each bot has a research note in
`trading/research/`. The results are in `trading/research/LEADERBOARD.md`.

Limits that you must know before you trade:

- **All backtest option prices are model prices** (Black-Scholes on VIX, with skew and
  spread). No free source of historical option quotes exists.
- Of the 22 bots, three have a clear positive result out of sample:
  `s14b_spy_trend_calls` and `s14_spy_qqq_rotation` (the same idea; mostly market
  exposure with a trend filter) and `s05_ic_0dte` (3 years of data; a small number of
  trades give most of the profit). Most other bots are negative results.
- Paper-test a bot against live quotes before you use real money (board ticket
  `options-paper-test`).

The valuation part and the trading part are not connected. The valuation part has
option-chain metrics for a ticker (implied move, skew, put/call ratios, in the xlsx tabs
`Options` and `Options Summary`), but no tool changes a verdict into an option trade.

## The xlsx file (nine tabs)

| Tab | Contents |
|-----|----------|
| Income Statement | The income-statement standard variables for each period, with the source of each value |
| Balance Sheet | The balance-sheet standard variables, and RPO rows (`statement=disclosure`) |
| Cash Flow | Operating cash flow, capex, free cash flow, buybacks, dividends, debt flows |
| Raw | All us-gaap USD tags that have no standard variable |
| Prices | Daily prices from yfinance (full history) |
| Estimates | Past earnings surprises (yfinance), and FMP estimates if available |
| Estimates Fwd | Dated snapshot of consensus revenue/EPS estimates and analyst price targets |
| Options | The full option chain, one row for each contract |
| Options Summary | For each expiry: at-the-money IV, implied move, 90/110 skew, put/call ratios |

The financial tabs have one row for each `(period, metric)`. The JSON store
(`data/processed/TICKER.json`) contains statements only.

## Data sources

| Data | Source | Key |
|------|--------|-----|
| Financial statements | SEC EDGAR companyfacts XBRL API | None |
| Prices, earnings surprises, consensus, option chains | yfinance | None |
| Analyst estimates (optional) | FMP | `FMP_API_KEY` in `.env` |
| Treasury par yield curves, nominal and real (TIPS) | U.S. Treasury | None |
| SOFR, EFFR, FOMC target range | Federal Reserve Bank of New York | None |
| CPI-U, headline and core | Bureau of Labor Statistics, API v1 | None |
| Cross-check of the rates and CPI | FRED (St. Louis Fed) | None |
| Corporate bond spreads and yields for each rating | ICE BofA indexes, through FRED (last 3 years only) | None |
| Moody's seasoned Aaa and Baa yields | Moody's, through FRED | None |
| HQM corporate bond yield curve | U.S. Treasury, through FRED | None |

The pipeline supports US-GAAP filers on EDGAR. ETFs and IFRS filers cause an
`UnsupportedFilingError`. Segment revenue is not available from companyfacts
(`PLANNING/tickets.md`, Ticket 7).

## Layout

```
CLAUDE.md                 rules for agents (read first)
README.md                 this file
requirements.txt          runtime packages
requirements-dev.txt      pytest
rust/mapcheck/            Rust test framework for the mapping (cargo)
.env.example              names of the optional API keys
.github/workflows/        CI: runs pytest and the Rust tests on each pull request

financial_analysis/       valuation package (import, store, engines, backtest, verdict)
analysis/IBM/             IBM model (version 4), reviews, published report
analysis/JNJ/             JNJ first pass (not reviewed)
trading/                  option strategy backtester, paper trader, 22 bots, research notes
scripts/                  command-line entry points
rust/                     Rust interface: library and `fa` command (calls the Python code)
tests/                    469 tests

docs/                     ARCHITECTURE.md, HIDDEN-ASSUMPTIONS.md, INDEX.md, design documents
PLANNING/board/           open work, one file for each ticket
PLANNING/tickets.md       backlog without tickets
PLANNING/                 reusable-dcf-vision.md (design), RULES.md, token-ledger.md,
                          tickets-closed.md, proposals/
data/                     caches and outputs (not in git)
```
