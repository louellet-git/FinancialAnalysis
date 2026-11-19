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
python -m pytest                       # 349 tests, no network
cargo build --release --manifest-path rust/mapcheck/Cargo.toml   # the Rust program (necessary)
cargo test --manifest-path rust/mapcheck/Cargo.toml    # 86 Rust tests
cargo test --manifest-path rust/Cargo.toml --locked    # 185 Rust tests (`fa` crate: rates, models, futures, option trading, bond research)
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
`InterestExpenseDebt` for Chevron. The registry `financial_analysis/standard_variables.toml`
gives each line one standard variable name (132 variables). The models read only these
names.

The Rust program `rust/mapcheck` makes the standard variables. It is the only
implementation: no Python code does this work. `scripts.analyze` and the other Python
commands get their data from it.

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
implementation of these parts. The option backtester (`fa trade backtest`) and `fa forwards`
are Rust code and call no Python code.

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
cargo run --bin fa -- run scripts.analyze IBM     # any Python entry point
cargo run --release --bin fa -- recon bonds --date 2026-11-17   # bond values from our rates vs market prices
cargo run --release --bin fa -- recon bills --date 2026-11-17   # test each cause of the bill difference
cargo run --release --bin fa -- recon rich-cheap --date 2026-11-17   # fitted curve, rich and cheap candidates after cost
cargo run --release --bin fa -- recon history                   # the difference over the cached dates: persistence, half-life
cargo run --release --bin fa -- recon explain --date 2026-11-18   # regression of the difference on SOMA share, stripping, age, size, futures; the old 30-year bonds
cargo run --release --bin fa -- serve             # local viewer in the browser
cargo run --example dcf_rates -- IBM 231.09 A-    # library example
```

Set `FA_PYTHON` to select the Python interpreter and `FA_REPO` to select the repository.

## Local viewer

`fa serve` starts a local web server and opens a page in the default browser. The page
shows the stock data, the Monte Carlo DCF, the company report, the rates, the bonds and the
option bots in one place.

You must have Rust (`cargo`), Python 3 and the Python packages. Install them one time.
Then build the two Rust programs.

Windows PowerShell, from the repo root:

```powershell
python -m venv .venv
.venv\Scripts\python.exe -m pip install -r requirements.txt
cargo build --release --manifest-path rust/mapcheck/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
rust\target\release\fa.exe serve
```

macOS or Linux, from the repo root:

```bash
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
cargo build --release --manifest-path rust/mapcheck/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
rust/target/release/fa serve
```

The program prints the address (`http://127.0.0.1:8765/`). If the port is in use, the
program tries the next port. Options: `--port N` sets the port. `--no-open` does not
open the browser. Push Ctrl+C to stop the server.

The view "Company report" shows the report of `python -m scripts.visualize_dcf` inside the
page (one file for a ticker, with the plotly library, about 6 MB). Select a ticker, type a
price if you want, and push "Show the report". The program makes the report with the Python
command and saves it in `data/reports/` (not in git). It makes the report again only when no
file exists for the ticker and the price, or when you push "Make it again". The page shows
the time of the file and a link to open it in a new tab. Without a price, the Python command
reads the price from the network. If it cannot, the report has no price marker and the page
shows a warning. The ticker needs saved company data (`data/processed/<TICKER>.json`).

The program finds Python in this sequence: `FA_PYTHON`, then `.venv` in the repo
(`.venv\Scripts\python.exe` on Windows, `.venv/bin/python` on macOS and Linux), then
`python` (Windows) or `python3` (macOS and Linux).

| View | What it shows | Source |
|---|---|---|
| Stock | The standard variables of the latest fiscal years with value and source, the status (PASS, WARN, FAIL, N/A), the company type and the findings. | `mapcheck` library, cache `data/raw/` |
| Monte Carlo DCF | P10 to P90 of the value for each share, the chance that the price is too high, and the verdict. Optional: the market rates of today and a credit rating. | `scripts.verdict` |
| Rates | The nominal and real Treasury curves, SOFR, EFFR, breakevens, the 5y5y forward and CPI. | `financial_analysis.rates` |
| Bonds | The ICE BofA yield and spread for each rating, Moody's Aaa and Baa, the HQM corporate curve. | `financial_analysis.bonds` |
| Option bots | The text of `trading/research/LEADERBOARD.md`. | file |

Limits:

- The server accepts connections from this computer only (127.0.0.1).
- The Monte Carlo DCF runs only for a ticker with `analysis/<TICKER>/driver_model.py`.
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
requirements.txt          runtime packages
requirements-dev.txt      pytest
rust/mapcheck/            Rust: SEC data to standard variables, checks, filing statements
.env.example              names of the optional API keys
.github/workflows/        CI: runs pytest and the Rust tests on each pull request

financial_analysis/       valuation package (import, store, engines, backtest, verdict)
analysis/IBM/             IBM model (version 4), reviews, published report
analysis/JNJ/             JNJ first pass (not reviewed)
trading/                  research notes of the 22 option bots, LEADERBOARD.md (the code is in rust/src/trading/)
scripts/                  command-line entry points
rust/                     Rust: `fa` command (calls the Python code; has the option trading: fetch, backtest, paper)
tests/                    349 Python tests (legacy code)

docs/                     ARCHITECTURE.md, HIDDEN-ASSUMPTIONS.md, INDEX.md, design documents
PLANNING/board/           open work, one file for each ticket
PLANNING/tickets.md       backlog without tickets
PLANNING/                 reusable-dcf-vision.md (design), RULES.md, token-ledger.md,
                          tickets-closed.md, proposals/
data/                     caches and outputs (not in git)
```
