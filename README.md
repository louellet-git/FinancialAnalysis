# FinancialAnalysis

This repo helps two persons make investment decisions and trade options. It has two
parts:

- **Valuation.** A Rust pipeline (the programs `mapcheck` and `fa`) gets company
  financial data from SEC EDGAR, adds market data (prices, earnings surprises, consensus
  estimates, option chains), and calculates a range of values for each share with a
  Monte Carlo DCF. Then it compares the range with the market price.
- **Options trading research.** A backtester and a paper trader for SPY/QQQ option
  strategies.

The repo gives analysis only. It does not send orders to a broker. It is not investment
advice.

Agents: read `CLAUDE.md` first. Open work: `PLANNING/board/`.

## Start

```bash
cargo build --release --manifest-path rust/mapcheck/Cargo.toml   # the Rust program that makes the standard variables (necessary)
cargo build --release --manifest-path rust/Cargo.toml            # the `fa` program
cargo test --manifest-path rust/mapcheck/Cargo.toml    # 89 Rust tests, no network
cargo test --manifest-path rust/Cargo.toml --locked    # 343 Rust tests (325 + 17 + 1), no network (`fa` crate: DCF, backtest, report, rates, bonds, company data, market data, xlsx file, models, futures, option trading, bond research)
```

No Python test remains. The only Python code that remains is the legacy SEC client with its
callers; see "Standard input variables".

Run all commands from the repo root.

## From a ticker to a decision

```bash
# 1. Get the data. Output: data/processed/IBM_financials.xlsx (nine tabs)
cargo run --release --bin fa -- analyze IBM

# 2. Get the value range and the verdict against a price (the price is an input)
cargo build --release --manifest-path rust/Cargo.toml   # one time: the `fa` program
rust/target/release/fa verdict IBM --price 228.53

# 3. Make the one-page HTML report. Output: data/processed/IBM_dcf.html
rust/target/release/fa report IBM [--price 228.53]

# 4. Examine how the mechanical model performed on past years
rust/target/release/fa backtest IBM
```

Step 2 is available only for a ticker that has `analysis/<TICKER>/driver_model.toml`.
On 2026-11-19 these are IBM (reviewed), JNJ and ANT (first passes, not reviewed).

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
- A verdict is only as good as its driver model. The JNJ and ANT models have no review.
- The IBM report is from July 2026. Its watch list is not graded against the Q2 and Q3
  results (board ticket `ibm-q2-grading`).

## All valuation commands

```bash
fa analyze IBM [--refresh] [--no-market] [--no-options] [--no-estimates] [--out custom.xlsx]
fa import IBM [--refresh] [--periods 12]                 # JSON store only, no xlsx
fa report IBM [--price 195.40] [--out report.html]       # one-page HTML report (see "The company report")
```

The valuation engines, the backtest, the report, the verdict, the rates and the bonds
are Rust code (the `fa` program). The random numbers are the same as numpy's, so the numbers are
the same as the numbers that the Python engines gave. The driver models are the files
`analysis/<TICKER>/driver_model.toml`.

```bash
fa dcf drivers IBM [--seed 7] [--sims 10000] [--price 228.53] [--json]        # headline, hand DCF check, attribution
fa dcf drivers ANT --price 83 --price 105 --sensitivity beta=1.1,1.7          # P50 for each value of one driver mean
fa dcf drivers ANT --implied-price 83 --implied-driver g_total --range 0,0.8  # the mean that gives the price
fa dcf drivers IBM --set beta=1.0 [--shift-bounds] --flat sbc_ratio=0.0254 --ramp sbc_ratio=0.0254:0.045:10 \
    --corr rf,g_software=-0.15 --net-debt 5e10 --shares 9.5e8 --years 10 --year1-revenue NAME=X   # overrides
fa dcf classic JNJ [--sims 10000] [--years 10] [--growth 0.15] [--wacc 0.10] [--terminal-growth 0.03] [--seed 7] [--data-dir D]
fa dcf classic IBM --margin-mean 0.145 --margin-std 0.02 --net-debt 54e9 --shares 948675228   # classic engine, value per share
fa dcf scenarios IBM [--data-dir D] [--json]      # bear, base, bull; value per share after net debt
fa dcf models                                     # tickers that have a driver model file
fa backtest IBM [--horizon 3] [--simulations 10000] [--seed 7] [--data-dir D]   # vintage backtest, no network
fa dcf calibrate [--cache DIR] [--out FILE] [--no-bands]   # AR(1) parameters of growth and margins from the company cache, band coverage, no network
fa verdict IBM --price 228.53 [--simulations 10000] [--seed 7] [--rf-date today] [--rf-base-date 2026-07-16] [--rating A-]
fa rates [snapshot|yield|curve|history|nyfed|inflation|fred|check] [--date D]   # JSON
fa bonds [snapshot|spread|yield|over10y|moodys|hqm|check] [--rating A-] [--date D]   # JSON
```

More options of `fa dcf drivers` (each analysis of the three first passes and of the IBM
version 4 runner): `--ev-fixed-point`, `--reinvestment-check`, `--path`, `--fade`,
`--commitments`, `--keep-year1-margin`. The command for each number of the first
passes: `analysis/ANT/COMMANDS.md`, `analysis/JNJ/COMMANDS.md`; for IBM:
`analysis/IBM/README.md` (section "Reproduce").

`fa dcf classic` prints an enterprise value. It subtracts net debt only when you give
`--net-debt` and `--shares`. Do not compare an enterprise value with a share price.

Remote data has a disk cache in `data/raw/TICKER/`. Option chains are saved as dated
snapshots, because the source gives only current data. A run on the same day reads the
disk; `fa analyze --refresh` gets new data. Prices come from the Yahoo chart endpoint. Yahoo
often answers HTTP 429 (too many requests) to a cloud container: then the program says so and
keeps the cache file. The forward consensus snapshot and the earnings dates have no source in
Rust: the program reads the cache files that exist (see `docs/HIDDEN-ASSUMPTIONS.md`, #140 to #145).

## Standard input variables

Companies use different XBRL tag names for the same line. For example, interest expense
is `InterestExpense` for IBM, `InterestExpenseNonoperating` for Microsoft and
`InterestExpenseDebt` for Chevron. The registry `financial_analysis/standard_variables.toml`
gives each line one standard variable name (132 variables). The models read only these
names.

The Rust program `rust/mapcheck` makes the standard variables. It is the only
implementation: no Python code does this work. `fa import`, `fa analyze` and `fa report`
get their data from it.

One Python command remains: `python -m scripts.import_edgar IBM 2026-07-01 [--output-dir data/raw]
[--lookback-months 18] [--lookahead-months 6]`. It downloads the filing files with the
legacy SEC client in `financial_analysis/`. No Rust command makes the same download; the
Rust program `mapcheck filing` reads the filing statements from the SEC data instead. The
legacy client stays until the owner decides the new place of the SEC contact text (board
ticket `python-to-rust`). It needs the packages of
`requirements.txt` (`pip install -r requirements.txt`).

- Each variable has a list of tags with the same meaning. The first tag that has a
  value is used.
- If no tag has a value, a formula over other variables is used (for example
  `total_debt = debt_current + long_term_debt`).
- Each value has a recorded source: the tag, or the formula. The xlsx file shows it in
  the `source` column. `mapcheck values TICKER` prints it.
- An annual row is always a full fiscal year. A quarterly row is always one quarter.
- Free cash flow is operating cash flow less `capex_total`. `capex_total` is equipment
  (`capex`) and capitalized software. Purchased intangibles are not included. This is
  rule R2 in `PLANNING/RULES.md`, with the reasons.
- A share count with no scale (McDonald's reports 716.4 for 716.4 million shares) is
  corrected with net income divided by EPS, and the source records the correction.
- Tags with no standard variable stay in the `Raw` tab.

```bash
cargo build --release --manifest-path rust/mapcheck/Cargo.toml    # one time, and after a registry change

M=rust/mapcheck/target/release/mapcheck
$M values IBM                      # each variable with its value and its source
$M IBM --detail                    # the checks for one ticker
$M --tickers-file rust/mapcheck/universe/sp500.txt      # the checks for many tickers (S&P 500)
$M fetch --filings --gaps --tickers-file rust/mapcheck/universe/sp500.txt   # the 10-K of each company with a missing critical input
$M fetch --prospectus ANT          # the latest prospectus of a new registrant (424B4, or S-1/A, or S-1) into the cache
$M filing IBM --focus              # the capex, depreciation and debt lines of the latest 10-K
$M filing VZ --statement cash      # one statement of the 10-K as the company structures it
$M crosscheck --tickers-file rust/mapcheck/universe/batch50.txt   # standard variables against the filing
$M fetch --top 2000 --write-list rust/mapcheck/universe/top2000.txt   # get data (set SEC_USER_AGENT)
```

On Windows the program is `rust\mapcheck\target\release\mapcheck.exe`.

The checks answer four questions for each company: can the DCF use the inputs, was a
value lost, was a value counted two times, and do the statements agree. Result on
2026-11-19 for the S&P 500 (503 tickers, 5 seconds): 68 pass, 433 have a warning that a
person must examine, 0 fail, and 2 are new companies with no annual report. The next 500
companies of the SEC list: 20 pass, 457 have a warning, 7 fail with a known cause, 16
are not judged. 24 of the S&P 500 companies get capital expenditure, revenue or the
share count from their 10-K, because the standard SEC data has no value for them. Read
`rust/mapcheck/README.md` for the checks and the results.

Before you use a variable in a model, read these three results:

- **Details inside lines.** For 245 of the 501 S&P 500 companies, a variable is a detail
  inside a different line (for example income taxes payable inside accrued liabilities).
  If you add the two, you count the amount two times. `mapcheck TICKER --detail` shows
  these as `nested`.
- **Total debt.** For 61 of the 501 companies, the estimates of total debt disagree.
- **Three weak inputs.** Capital expenditure, depreciation and amortization, and total
  debt are less reliable than the other core inputs. `mapcheck filing TICKER --focus`
  shows the lines of the company's own filing for these three inputs. Examples: IBM
  capital expenditure is $1.09B in the standard variable and $1.74B in the filing (IBM
  has a company tag for software); the ConocoPhillips capex has only a company tag
  ($12.55B). The standard variables use the filing value only when the standard SEC
  data has no value for a critical input (ConocoPhillips: yes; IBM: no).

Limits: the SEC data source of the standard variables has no segment data and no
company-specific tags. Banks, insurers and REITs have a different statement structure;
the checks mark them. Foreign filers (form 20-F) and IFRS filers are out of scope.

## Interest rates and inflation

`fa rates` (Rust, `rust/src/rates.rs`) gets US rates from the publishers. All values are
decimals (0.0507 is 5.07%). Each value has the date of its observation. The cache is
`data/raw/rates/` (not in git). The requests use the client name `FinancialAnalysis fa
(personal research)` and wait 2 seconds between requests.

```bash
fa rates                                  # snapshot: curves, SOFR, EFFR, FOMC range, inflation
fa rates yield --tenor 10Y --date 2026-07-16
fa rates inflation
fa rates check                            # cross-check against FRED (exit 1 on a mismatch)
```

The driver models keep their own `rf`. To use the Treasury 10-year rate of a date, add
`--rf-date` to `fa verdict`:

```bash
fa verdict IBM --price 231.09 --rf-date today --rf-base-date 2026-07-16
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

The engine uses `kd = rf + kd_spread`. `fa bonds` (Rust, `rust/src/bonds.rs`) gives market
values for `kd_spread`. The data come through the FRED CSV file (see
`docs/HIDDEN-ASSUMPTIONS.md` #128 if FRED does not answer).

```bash
fa bonds                       # all ratings, Moody's, HQM curve
fa bonds over10y --rating A-   # the market kd_spread for a rating
fa bonds check                 # Moody's minus UST 10Y against FRED (exit 1 on a mismatch)
```

`over10y` is the yield of the ICE BofA index for the rating minus the Treasury 10-year
yield on the same date. This is the definition of `kd_spread` in the engine. The rating
can be S&P (`A-`) or Moody's (`A3`). The index has no notches, thus `A-` uses the `A`
index.

To use the market spread in a verdict, add `--rating` to a run with `--rf-date`:

```bash
fa verdict JNJ --price 200 --rf-date today --rating AAA
```

The output shows the market spread, the model spread and the pre-tax and after-tax cost
of debt. The driver models do not change. Read `docs/HIDDEN-ASSUMPTIONS.md` #45 before
you use `--rating` for IBM.

### Rust interface

The `rust/` crate (the `fa` program) gives typed results. The DCF engines, the scenarios,
the verdict (with or without market rates), the backtest, the rates and the bonds are Rust
code (`fa dcf ...`, `fa verdict`, `fa backtest`, `fa rates`, `fa bonds`). The option trading
(`fa trade ...`), `fa forwards`, `fa futures`, `fa models` and `fa recon` are Rust code too and
call no Python code. No `fa` command starts Python code.

```bash
cd rust
cargo run --bin fa -- rates yield --tenor 10Y
cargo run --bin fa -- verdict IBM --price 231.09 --rf-date today --rating A-
cargo run --bin fa -- bonds over10y --rating A-
cargo run --bin fa -- forwards --date 2026-11-13   # GSW zero-coupon yields and forward rates (Rust only)
cargo run --bin fa -- forwards check               # GSW against Treasury par yields, exit 1 on mismatch
cargo run --bin fa -- futures roots                 # contract facts of each root
cargo run --bin fa -- futures quote ESZ26            # last daily value and last trade (Yahoo); VXZ26 uses Cboe
cargo run --bin fa -- futures history ZQX26 --start 2026-11-01   # daily values of one contract
cargo run --bin fa -- futures curve SR3              # all listed contracts; implied rate = 100 - price
cargo run --bin fa -- futures curve VX --date 2026-11-17         # Cboe settlement of one date
cargo run --bin fa -- futures positions ES --weeks 4 # CFTC Commitments of Traders (Rust only)
cargo run --bin fa -- futures policy-path            # Atlanta Fed policy-rate path from SOFR options
cargo run --bin fa -- futures check                  # two cross-checks, exit 1 on mismatch
cargo run --release --bin fa -- models calibrate --model cir --method ml --start 2000-01-01 --freq weekly   # fit one model to GSW 1Y zero yield
cargo run --release --bin fa -- models rolling --model vasicek --method ml --window 5y --step 3m --out roll.csv  # parameters in windows
cargo run --release --bin fa -- models frequencies --model vasicek --method ml   # daily, weekly, monthly
cargo run --release --bin fa -- models compare --start 1990-01-01                # vasicek, cir, ckls, holee: log-likelihood, AIC
cargo run --release --bin fa -- models simulate --model cir --kappa 0.5 --theta 0.05 --sigma 0.1 --years 10 --paths 100000 --seed 1
cargo run --bin fa -- analyze IBM                  # import, market data, nine-tab xlsx file
cargo run --bin fa -- import IBM                   # SEC data -> data/processed/IBM.json
cargo run --bin fa -- report IBM                   # one-page HTML report -> data/processed/IBM_dcf.html
cargo run --release --bin fa -- recon bonds --date 2026-11-17   # bond values from our rates vs market prices
cargo run --release --bin fa -- recon bills --date 2026-11-17   # test each cause of the bill difference
cargo run --release --bin fa -- recon rich-cheap --date 2026-11-17   # fitted curve, rich and cheap candidates after cost
cargo run --release --bin fa -- recon history                   # the difference over the cached dates: persistence, half-life
cargo run --release --bin fa -- recon explain --date 2026-11-18   # regression of the difference on SOMA share, stripping, age, size, futures; the old 30-year bonds
cargo run --release --bin fa -- serve             # local viewer in the browser
cargo run --example dcf_rates -- IBM 231.09 A-    # library example
```

Set `FA_REPO` to select the repository.

## Local viewer

`fa serve` starts a local web server and opens a page in the default browser. The page
shows the stock data, the Monte Carlo DCF, the company report, the rates, the bonds and the
option bots in one place.

You must have Rust (`cargo`). You do not need Python. Build the two Rust programs one
time.

Windows PowerShell, from the repo root:

```powershell
cargo build --release --manifest-path rust/mapcheck/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
rust\target\release\fa.exe serve
```

macOS or Linux, from the repo root:

```bash
cargo build --release --manifest-path rust/mapcheck/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
rust/target/release/fa serve
```

The program prints the address (`http://127.0.0.1:8765/`). If the port is in use, the
program tries the next port. Options: `--port N` sets the port. `--no-open` does not
open the browser. Push Ctrl+C to stop the server.

The view "Company report" shows the report of `fa report` inside the page (see "The
company report"). Select a ticker, type a price if you want, and push "Show the report".
The program makes the report and saves it in `data/reports/` (not in git). It makes the
report again only when no file exists for the ticker and the price, or when you push "Make
it again". The page shows the time of the file and a link to open it in a new tab. Without
a price, the report uses the last close of the price history. If there is no price
history, the report has no price marker and the page shows a warning. The ticker needs
saved company data (`data/processed/<TICKER>.json`). The charts need the network: see
"The company report".

| View | What it shows | Source |
|---|---|---|
| Stock | The standard variables of the latest fiscal years with value and source, the status (PASS, WARN, FAIL, N/A), the company type and the findings. | `mapcheck` library, cache `data/raw/` |
| Monte Carlo DCF | P10 to P90 of the value for each share, the chance that the price is too high, and the verdict. Optional: the market rates of today and a credit rating. | `fa verdict` (Rust) |
| Rates | The nominal and real Treasury curves, SOFR, EFFR, breakevens, the 5y5y forward and CPI. | `fa rates` (Rust) |
| Bonds | The ICE BofA yield and spread for each rating, Moody's Aaa and Baa, the HQM corporate curve. | `fa bonds` (Rust) |
| Option bots | The text of `trading/research/LEADERBOARD.md`. | file |

Limits:

- The server accepts connections from this computer only (127.0.0.1).
- The Monte Carlo DCF runs only for a ticker with `analysis/<TICKER>/driver_model.toml`.
  The page shows the percentiles as a range chart. It does not show a histogram.
- A ticker that is not in `data/raw/` needs the SEC. Set `SEC_USER_AGENT` before you
  start the server (see `rust/mapcheck/README.md`). A fresh clone has no `data/raw/`,
  thus the first look-up of each ticker needs the SEC.
- The rates and the bonds need a network connection to the public data sites. If a site
  does not answer, the page shows the error text. A call that waits for a site can take
  more than one minute.
- The program finds the repo from the folder where you built it. If you move the repo,
  build again or set `FA_REPO`.

## The two DCF engines

**Classic engine**, `rust/src/dcf/classic.rs`. It samples revenue growth, free-cash-flow
margin, WACC and terminal growth directly. The presets (bear, base, bull) and the net
debt and share count for each ticker are in `rust/src/dcf/scenarios.rs`. The IBM
entries there are version 3 on purpose (`docs/HIDDEN-ASSUMPTIONS.md` #13).

**Driver engine**, `rust/src/dcf/driver.rs`. The top-level items (revenue,
margin, WACC, terminal value) are fixed formulas. All randomness is in named drivers.
Each driver has a written reason. Each path uses one draw for each driver, with a
correlation matrix. WACC is calculated for each path. Terminal growth is limited to
`WACC − 2%` for each path. The model for a ticker is in
`analysis/<TICKER>/driver_model.toml`.

**Backtest**, `rust/src/dcf/backtest.rs` (`fa backtest`). For each past fiscal year end plus 90
days, it uses only the data that was filed before that date, projects revenue and free
cash flow for 1 to 3 years, and compares the projection with the actual results. It
tests operating projections only: no WACC, no terminal value, no price.

More detail: `docs/ARCHITECTURE.md`.

## The company report

`fa report TICKER [--price P] [--out FILE]` writes one HTML page for a ticker (default file:
`data/processed/<TICKER>_dcf.html`). The sections, in this order: header band (price,
market cap, scenario P50 values), price and valuation, fundamentals, aggregate DCF
scenarios (the classic engine: fan, distribution, waterfall, tornado, summary table),
driver-based model (segment history, driver table, correlations, comparison with the
aggregate base, attribution, path explorer), backtest, options market, footer. A section
that has no data shows a note with the reason.

The program draws the charts with the plotly.js library, version 4.1.1 (the version that
the old Python page had inside the file). **The page loads the library from the official
CDN (`https://cdn.plot.ly/plotly-4.1.1.min.js`, a fixed version). The page needs the
network when you open it.** The old Python page had the library inside the file (about
3.5 MB) and worked offline. An option that writes a local copy of the library into the
file is an open point for the owner (board ticket `report-rust`). Without the network the
tables and texts show, and each chart shows a note.

The random numbers are the numpy numbers, with the same seeds and the same order of
draws as the old script. The driver model is the file `analysis/<TICKER>/driver_model.toml`.
Without `--price`, the report uses the last close of the price history (the cache file
`data/raw/<TICKER>/prices.json`; the program asks the Yahoo chart endpoint if there is no
cache file).

## Options trading research

```bash
cargo build --release --manifest-path rust/Cargo.toml # one time
rust/target/release/fa trade fetch                    # download the bars (Yahoo) into data/raw/trading/
rust/target/release/fa trade backtest                 # all bots, leaderboard (Rust, 1 second)
rust/target/release/fa trade backtest --bot s14b_spy_trend_calls -v
rust/target/release/fa trade backtest --max-risk 250  # larger risk for each trade
rust/target/release/fa trade paper                    # one paper-trade step for each bot
```

`fa trade backtest --help` shows all options (`--out FILE` writes the leaderboard as CSV,
`--trades-dir DIR` writes the trades of each bot). `fa trade paper --help` shows the paper
options (`--no-refresh`, `--no-live`, `--start D`, `--max-risk N`).

The code is in `rust/src/trading/`. It has 22 bots. Each bot has a research note in
`trading/research/`. The results are in `trading/research/LEADERBOARD.md`.

Data sources (free, no key, no login). `fa trade fetch` gets the bars from the Yahoo
chart endpoint. It waits 2 seconds between two requests. If Yahoo answers HTTP 429, the
command waits, tries one more time, and then stops; the cache files stay as they are.
`fa trade paper` gets the live option chain from the Cboe delayed quotes (about 15 minutes
late). Yahoo is not a source of option chains: it refuses a program that has no browser
identity (HTTP 429 at the crumb request).

Limits that you must know before you trade:

- **All backtest option prices are model prices** (Black-Scholes on VIX, with skew and
  spread). No free source of historical option quotes exists.
- Of the 22 bots, three have a clear positive result out of sample:
  `s14b_spy_trend_calls` and `s14_spy_qqq_rotation` (the same idea; mostly market
  exposure with a trend filter) and `s05_ic_0dte` (3 years of data; a small number of
  trades give most of the profit). Most other bots are negative results.
- Paper-test a bot against live quotes before you use real money (board ticket
  `options-paper-test`). The bots assume an expiry on each trading day. The live chain has
  daily expiries for the next two weeks only, then Fridays. A signal with an expiry that
  the chain does not list is priced with the model, and the report says so.

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
| Prices | Daily prices from the Yahoo chart endpoint (full history, adjusted for splits and dividends) |
| Estimates | Past earnings surprises (FMP, or the cache file of an earlier run), and FMP estimates if the key exists |
| Estimates Fwd | Dated snapshot of consensus revenue/EPS estimates and analyst price targets |
| Options | The full option chain, one row for each contract |
| Options Summary | For each expiry: at-the-money IV, implied move, 90/110 skew, put/call ratios |

The financial tabs have one row for each `(period, metric)`. The JSON store
(`data/processed/TICKER.json`) contains statements only.

## Data sources

| Data | Source | Key |
|------|--------|-----|
| Financial statements | SEC EDGAR companyfacts XBRL API | None |
| Daily prices | Yahoo chart endpoint | None |
| Option chains | Cboe delayed quotes (about 15 minutes late) | None |
| Analyst estimates and earnings surprises | Financial Modeling Prep | `FMP_API_KEY` in `.env` or the environment |
| Forward consensus snapshot, earnings dates | No source in Rust: cache files of the earlier Python code | None |
| Treasury par yield curves, nominal and real (TIPS) | U.S. Treasury | None |
| SOFR, EFFR, FOMC target range | Federal Reserve Bank of New York | None |
| CPI-U, headline and core | Bureau of Labor Statistics, API v1 | None |
| Zero-coupon yields and forward rates (fitted curve, GSW) | Federal Reserve Board, `feds200628.csv` | None |
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
requirements.txt          packages of the legacy SEC client (the only Python code)
rust/mapcheck/            Rust: SEC data to standard variables, checks, filing statements
.env.example              names of the optional API keys
.github/workflows/        CI: runs the Rust tests on each pull request

financial_analysis/       the registry standard_variables.toml; the legacy SEC client (Python)
analysis/IBM/             IBM model (version 4), reviews, published report
analysis/JNJ/             JNJ first pass (not reviewed)
trading/                  research notes of the 22 option bots, LEADERBOARD.md (the code is in rust/src/trading/)
scripts/                  import_edgar.py: the one Python command (legacy SEC client)
rust/                     Rust: `fa` command (the valuation, the report, the rates, the option trading: fetch, backtest, paper)

docs/                     ARCHITECTURE.md, HIDDEN-ASSUMPTIONS.md, INDEX.md, design documents
PLANNING/board/           open work, one file for each ticket
PLANNING/tickets.md       backlog without tickets
PLANNING/                 reusable-dcf-vision.md (design), RULES.md, token-ledger.md,
                          tickets-closed.md, proposals/
data/                     caches and outputs (not in git)
```
