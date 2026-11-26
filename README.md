# FinancialAnalysis

This repo helps two persons make investment decisions and trade options. It has two
parts:

- **Valuation.** A Rust pipeline (the programs `mapcheck` and `fa`) gets company
  financial data from SEC EDGAR, adds market data (prices, earnings surprises, consensus
  estimates, option chains), and calculates a range of values for each share with a
  Monte Carlo DCF. Then it compares the range with the market price.
- **Options trading research.** A backtester and a paper trader for SPY/QQQ option
  strategies.

The repo gives analysis. A program may send orders to the Alpaca paper account only (controls in
`docs/QUALITY-STANDARDS.md`, section 8). No program sends a live order. It is not investment
advice.

Agents: read `CLAUDE.md` first. Open work: `PLANNING/board/`. The agent teams: `docs/TEAMS.md`. The head sessions: `chiefs/ORIENTATION.md`. The quality standards: `docs/QUALITY-STANDARDS.md`.

## Start

```bash
cargo build --release --manifest-path rust/mapcheck/Cargo.toml   # the Rust program that makes the standard variables (necessary)
cargo build --release --manifest-path rust/Cargo.toml            # the `fa` program
cargo test --manifest-path rust/mapcheck/Cargo.toml    # all mapcheck tests, no network (it prints the counts)
cargo test --manifest-path rust/Cargo.toml --locked    # all fa tests, no network (it prints the counts) (`fa` crate: DCF, backtest, report, rates, bonds, company data, market data, xlsx file, models, futures, option trading, bond research, data sync)
```

The repo has no Python code.

Run all commands from the repo root.

## First run on your computer

Do these steps one time on a new computer. The commands run from the root of the repo.

1. Build the two programs (the first build takes some minutes):

   ```bash
   cargo build --release --manifest-path rust/mapcheck/Cargo.toml
   cargo build --release --manifest-path rust/Cargo.toml
   ```

2. Copy `.env.example` to `.env` (the file is not in git; do not commit it). The keys in it are optional: with no key, the sources with no key are used. `fa sync` prints which key would add which source.

   - bash: `cp .env.example .env`
   - PowerShell: `Copy-Item .env.example .env`

3. Optional: set `SEC_USER_AGENT` (a name and a contact, for example `FinancialAnalysis your-name your-email`) if you want to use your own contact. The SEC asks for a contact in each request. The file `sec_contact.toml` of the repo gives the project contact, and the variable replaces it. It is an environment variable; `fa sync` does not read it from `.env`. If there is no variable and no good file, `fa sync` skips the SEC data sets and says so.

   - bash: `export SEC_USER_AGENT="FinancialAnalysis your-name your-email"`
   - PowerShell: `$env:SEC_USER_AGENT = "FinancialAnalysis your-name your-email"`

4. Look at what is old (no network request), then bring the data up to date. The sync runs on request only.

   - bash: `rust/target/release/fa sync --status IBM MSFT` then `rust/target/release/fa sync IBM MSFT`
   - PowerShell: `.\rust\target\release\fa.exe sync --status IBM MSFT` then `.\rust\target\release\fa.exe sync IBM MSFT`

   Use `fa sync --dry-run` to see the requests and the estimated time first. Use `--tickers-file rust/mapcheck/universe/sp500.txt` for the S&P 500 (about 500 requests to Nasdaq: some tens of minutes; add `--max-minutes N` to limit a run).
   Ctrl+C stops the run with all files in a correct state. Run the command again to continue with what is old.

5. A second run on the same day makes no request for a data set that is current. The summary has one line for each data set and one line for each host, the list of what stays old, and the command to run again.
   A host that refuses the program (HTTP 429 or HTTP 403) or does not answer is stopped for that run; the other hosts go on.

## From a ticker to a decision

```bash
# 1. Get the data. Output: data/processed/IBM_financials.xlsx (nine tabs)
cargo run --release --bin fa -- analyze IBM

# 2. Get the price history, then the value range and the verdict (without --price: the last close of the cache)
cargo build --release --manifest-path rust/Cargo.toml   # one time: the `fa` program
rust/target/release/fa prices get IBM                   # daily prices into data/raw/IBM/prices.json (Tiingo, Yahoo, Nasdaq, Alpaca; Alpaca last)
rust/target/release/fa verdict IBM [--price 228.53]

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
fa dcf drivers ANT --model driver_model_paths.toml --price 83                  # path engine: processes for each year (see docs/ARCHITECTURE.md)
fa dcf drivers ANT --model driver_model_paths.toml --pset process.growth.half_life=3 --implied-price 83 --implied-param process.growth.target --shift-bounds
fa dcf drivers IBM --set beta=1.0 [--shift-bounds] --flat sbc_ratio=0.0254 --ramp sbc_ratio=0.0254:0.045:10 \
    --corr rf,g_software=-0.15 --net-debt 5e10 --shares 9.5e8 --years 10 --year1-revenue NAME=X   # overrides
fa dcf classic JNJ [--sims 10000] [--years 10] [--growth 0.15] [--wacc 0.10] [--terminal-growth 0.03] [--seed 7] [--data-dir D]
fa dcf classic IBM --margin-mean 0.145 --margin-std 0.02 --net-debt 54e9 --shares 948675228   # classic engine, value per share
fa dcf scenarios IBM [--data-dir D] [--json]      # bear, base, bull; value per share after net debt
fa dcf models                                     # tickers that have a driver model file
fa backtest IBM [--horizon 3] [--simulations 10000] [--seed 7] [--data-dir D]   # vintage backtest, no network
fa dcf calibrate [--cache DIR] [--out FILE] [--no-bands]   # AR(1) parameters of growth and margins from the company cache, band coverage, no network
fa dcf auto IBM [--cache DIR] [--price P] [--explain] [--write FILE]   # automatic path model of a ticker from the cache and the parameter table: a screen, not a reviewed valuation; no network
fa dcf screen --tickers-file rust/mapcheck/universe/sp500.txt [--cache DIR] [--out FILE] [--jobs N] [--prices FILE]   # one row for each ticker (value range, refusals), parallel over tickers
fa dcf pit [--cache DIR] [--cut YEAR] [--rules K=V,..] [--out FILE]   # point-in-time test of the automatic model: real revenue and free cash flow against the bands
fa dcf assumptions [--md]   # the register of the assumptions of the automatic model: id, rule key, default (read from the code), state, record, code place
fa dcf assumptions --effect K=V[,K=V] --tickers-file F --cache DIR --as-of D [--cut Y] [--half H]   # the screen and the point-in-time test with the default rules and with the rules: one block for a record
fa dcf returns [--cache DIR] [--rules K=V,..] [--lag DAYS] [--out FILE]   # the point-in-time model value (P50 / price) against the later 1-year and 3-year returns
fa portfolio backtest --strategy value-rank|momentum --start D [--end D] [--rebalance monthly|quarterly] [--cost-bp N] [--tickers-file F] [--cache DIR] [--out FILE] [--hypothesis FILE] [--trades FILE] [--poison-check D]   # portfolio backtest (a run with a committed hypothesis file writes an add-only run log, trading/runs/): weights on each rebalance date, daily value, cost, SPY and an equal-weight line of the list as references (the ticker list is today's list: the result is too good)
fa dcf screen ... --implied [--rules K=V,..]      # also the cost of capital that gives the price; --rules: an alternative rule (list: fa dcf auto --help)
fa dcf implied [--tickers-file FILE | TICKER ...] [--cache DIR] [--as-of D] [--groups FILE] [--prices FILE] [--rules K=V,..] [--out FILE] [--jobs N]   # what the price implies: cost of capital, growth, margin; medians by size, sector, industry; the premium of the whole list; no random part
fa dcf attribute [--tickers-file FILE | TICKER ...] --from D1 --to D2 [--cache DIR] [--rules K=V,..] [--out FILE] [--jobs N]   # where the price went between two dates: facts, rates, premium (or growth), rest; no random part. fa dcf implied also has --price-date D
fa prices get IBM MSFT [--tickers-file rust/mapcheck/universe/sp500.txt] [--source tiingo|yahoo|nasdaq] [--full]   # price history into data/raw (network)
fa prices show IBM [--from D] [--to D]            # rows of the cache
fa prices last IBM JNJ                            # last close, date, source; warns when older than 5 weekdays
fa prices check MSFT --sources yahoo,nasdaq       # compare two sources on the common dates (network)
fa estimates get IBM MSFT [--tickers-file F] [--endpoints forecast,surprise,targets,momentum] [--momentum] [--refresh]   # consensus EPS, price targets, surprises into data/raw (Nasdaq site, no key; network)
fa commodity get gold [--cache DIR]   # World Bank Pink Sheet monthly file into data/raw/commodity/worldbank (2 requests; CC BY 4.0; network)
fa commodity show gold [--from YYYY-MM]   # monthly gold price, USD per troy ounce, 1960 to the last month of the file (cache only)
fa estimates show IBM [--price P]                 # consensus by period, revisions, forward P/E, target against the last close, next report date
fa earnings calendar [IBM...] [--tickers-file F] [--from D] [--days 60]   # Nasdaq earnings calendar scan, next report date of each ticker (network)
fa earnings dates IBM JNJ [--tolerance 1]         # past report dates against the SEC 8-K list (mapcheck fetch --submissions TICKER)
fa fund holdings URA --from 2023-07-31       # SEC N-PORT history of an ETF into data/raw/URA/nport (network; the fields: currency, exchange rate, net assets, monthly returns, flows)
fa fund attribution URA [--from D]               # layers 1 and 2 of an ETF: net assets, price against NAV return, the parts of the return and a named remainder; reads the cache, no request
fa sync [IBM MSFT...] [--tickers-file F] [--only A,B] [--skip A,B] [--force] [--dry-run] [--max-minutes N] [--bond-days N]   # bring all old data sets up to date, one queue for each host; --bond-days N asks for the FedInvest prices of the last N market days (network)
fa usage show --week [--since RFC3339] [--head NAME]   # tokens and USD at list price for each ticket since the last weekly reset (Wednesday 16:00 UTC); with --head: the share, the use and the rest from PLANNING/board/EPICS.md
fa sync --status [IBM...]                         # one line for each data set: current, old, no data; no request
fa board check [--json]                         # the rule faults of the ticket board, for each head; reads git only (run `git fetch` first), changes no file, exit 0
fa verdict IBM [--price 228.53] [--simulations 10000] [--seed 7] [--rf-date today] [--rf-base-date 2026-07-16] [--rating A-]
fa rates [snapshot|yield|curve|history|nyfed|inflation|fred|check] [--date D]   # JSON
fa bonds [snapshot|step|yield|over10y|hqm] [--rating A-] [--date D]   # JSON
fa bonds hqm get [--force]                        # the Treasury HQM files into data/raw/bonds/treasury_hqm/
fa bonds hqm-compare --xls FILE... --fred-dir DIR # the Treasury HQM files against FRED HQMCB*YR
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
keeps the cache file. The forward consensus (EPS, price targets, ratings), the surprises and the earnings dates come from the Nasdaq site API (no key): `fa estimates get` and `fa earnings calendar` write the cache, `fa analyze` reads it (see `docs/HIDDEN-ASSUMPTIONS.md`, #215 to #229). Revenue estimates need `FMP_API_KEY`.

### Golden cases (`fa selftest golden`)

`rust/tests/golden/cases.toml` lists commands with the SHA-256 of their expected output. The author and the reviewer
of a change to the engine run it (not in CI: the store is private).

```bash
rust/target/release/fa selftest golden                       # run all cases; exit 1 on a difference (first 20 lines)
rust/target/release/fa selftest golden --case dcf_drivers_ibm
rust/target/release/fa selftest golden --update "why the output changes" --case NAME   # or --all
rust/target/release/fa selftest golden --record-inputs --case NAME   # write the input list of a case
```

Each case lists the input files that it reads, each with its SHA-256 (`fa selftest golden --record-inputs --case NAME`
writes the list with `strace`, Linux only; without it, write the list by hand and add a `note`). A listed input that is
missing or has another hash makes the case SKIPPED, with the file named; it is never a pass. An input that the list
misses can only make the case FAIL (the output changes), never pass: then add the file to the list. A run where all cases
are skipped says "0 of N cases ran" and exits 0. The `store` commit is information only. A mask hides only lines that vary (timing lines); any other mask needs a `why`. The command never downloads:
it runs only commands of an allow list that read the cache, and a case whose command asks for the network FAILS and names the request. `--update` needs a reason; it rewrites the input hashes and the expected text together, and the reason and the date go
into the case file, and the expected text goes into `rust/tests/golden/NAME.txt`, so the pull request shows the change.
A case that runs longer than the deadline (default 30 minutes, `--timeout-mins N`) is killed and FAILS. The expected texts are public: a case must not print raw option quotes (the options case uses `--top 0`).
`fa selftest golden --help` has the details.

## Standard input variables

Companies use different XBRL tag names for the same line. For example, interest expense
is `InterestExpense` for IBM, `InterestExpenseNonoperating` for Microsoft and
`InterestExpenseDebt` for Chevron. The registry `financial_analysis/standard_variables.toml`
gives each line one standard variable name (132 variables). The models read only these
names.

The Rust program `rust/mapcheck` makes the standard variables. It is the only
implementation: no Python code does this work. `fa import`, `fa analyze` and `fa report`
get their data from it.

The Rust program `mapcheck filing` reads the filing statements from the SEC data. The SEC contact
is in the file `sec_contact.toml` (a project address, public on purpose); the environment variable
`SEC_USER_AGENT` replaces it.

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
$M fetch --top 2000 --write-list rust/mapcheck/universe/top2000.txt   # get data (the SEC contact: sec_contact.toml)
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
values for `kd_spread`. The cost of debt of a rating is the Treasury HQM 10-year spot yield
plus a fixed step for the rating row, with the HQM basis (`docs/models/cost-of-debt.md`).

```bash
fa bonds hqm get               # the Treasury HQM files into data/raw/bonds/treasury_hqm/
fa bonds                       # all ratings (HQM + step), HQM curve
fa bonds over10y --rating A-   # the market kd_spread for a rating
```

`over10y` is the cost of debt of the rating (UST 10-year of the date + the HQM basis + the step of
the rating row) minus the Treasury 10-year yield on the date. This is the definition of `kd_spread` in the
engine. The rating can be S&P (`A-`) or Moody's (`A3`). Each notch with a row in the
Damodaran table has its own step (`A-` +0.11 points); a notch with no row takes its letter grade.
The bucket `A` is the HQM reference bucket (step 0).

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
| Bonds | The cost of debt and the step for each rating (HQM + step), the HQM corporate curve. | `fa bonds` (Rust) |
| Option bots | The text of `trading/research/LEADERBOARD.md`. | file |

Limits:

- The server accepts connections from this computer only (127.0.0.1).
- The Monte Carlo DCF runs only for a ticker with `analysis/<TICKER>/driver_model.toml`.
  The page shows the percentiles as a range chart. It does not show a histogram.
- A ticker that is not in `data/raw/` needs the SEC. The SEC contact comes from `sec_contact.toml` (the variable
  `SEC_USER_AGENT` replaces it; see `rust/mapcheck/README.md`). A fresh clone has no `data/raw/`,
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
rust/target/release/fa prices intraday SPY --interval 5m --from 2020-01-01   # 5-minute bars since a date (Alpaca, free key)
rust/target/release/fa options history SPY --from 2024-03-01 --to 2024-03-29 # option surface of past dates from daily closes (Alpaca)
rust/target/release/fa options check IBM --date 2026-11-19 [--rate R] [--time HHMM] [--min-size USD] [--iv-mark POINTS] [--compare D2] [--otm | --all-strikes]      # no-arbitrage checks of a stored chain at executable prices (bid and ask); upper parity bound only; without --rate the Treasury curve can be downloaded (one file), give --rate R for no network
rust/target/release/fa arb run --symbol SPX --date 2026-11-20 --rate 0.04 [--time HHMM] [--out PATH]      # the register checks (arbitrage/register/) on one stored snapshot: one table (id, class, status, tested, pass, break, not tested, dropped legs) and a JSON file; cache only, never downloads; exit 0 with breaks
rust/target/release/fa options snapshot SPY --source alpaca                  # option chain of today (Alpaca, free key); SPX and VIX too (both roots; level and volatility from put-call parity)
rust/target/release/fa options archive [SPX SPY VIX] [--keep-days N] [--min-share X] [--force]    # copy the live Alpaca chains with the time (fa options snapshot --source alpaca skips when a live Alpaca file of today exists unless --refresh; file <date>T<hhmm>.json and a gzip copy), refuse a half chain (below 80% of the last Alpaca file; --force or --min-share X to take a real shrink), and fetch the Cboe VX settlement file; readers take --date D (last file of the date) and --time HHMM
rust/target/release/fa vol fit heston --symbol IBM --date 2026-11-19  # fit Heston (or bates) to a stored snapshot with bid and ask; error by expiry in vol points
rust/target/release/fa trade backtest                 # all bots, leaderboard (Rust, 1 second)
rust/target/release/fa trade backtest --bot s14b_spy_trend_calls -v
rust/target/release/fa trade backtest --max-risk 250  # larger risk for each trade
rust/target/release/fa trade paper                    # one paper-trade step for each bot
rust/target/release/fa algo replay --strategy demo --date 2026-11-18   # algo engine on stored bars; live: fa algo run --strategy demo (needs the Alpaca keys; own simulator, no order; a pipe test outside the trading sleeves; run one Alpaca command at a time). PowerShell, from the REPO folder: $env:APCA_API_KEY_ID="..."; $env:APCA_API_SECRET_KEY="..."; .\rust\target\release\fa.exe algo run --strategy demo; stop and flatten: New-Item "$PWD\data\processed\algo\STOP" (the run prints the full path of this file; remove it before the next run). A run or a replay writes under `data/processed/algo/` of the repo folder, which git ignores
```

`fa trade backtest --help` shows all options (`--out FILE` writes the leaderboard as CSV,
`--trades-dir DIR` writes the trades of each bot). `fa trade paper --help` shows the paper
options (`--no-refresh`, `--no-live`, `--start D`, `--max-risk N`).

The code is in `rust/src/trading/`. It has 22 bots. Each bot has a research note in
`trading/research/`. The results are in `trading/research/LEADERBOARD.md`.

Data sources (free, no key, no login). `fa trade fetch` gets the bars from the Yahoo
chart endpoint. It waits 2 seconds between two requests. If Yahoo answers HTTP 429, the
command waits, tries one more time, and then stops; the cache files stay as they are.
`fa trade paper` prices with the model (`--no-live`; add `--no-refresh` to read only the cached bars, with no Yahoo download). It refuses a live option chain: the Cboe
page prohibits automatic downloads, and the Alpaca chain has no adapter yet (T-20261121-cboe-off). Yahoo is not a source of option chains: it refuses a program that has no browser
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

The table of all data sets with their sources, keys and commands is `docs/DATA-SOURCES.md`.

| Data | Source | Key |
|------|--------|-----|
| Financial statements | SEC EDGAR companyfacts XBRL API | None |
| Daily prices | Tiingo (key), Yahoo chart endpoint, Nasdaq site API, Alpaca (free keys; last: IEX close, history from 2020-07-27) | `TIINGO_API_KEY`; `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY` (none for Yahoo and Nasdaq) |
| Intraday bars (1 and 5 minutes), option chain snapshots, option closes of past dates | Alpaca Market Data API (free plan) | `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY` |
| Option chains | Alpaca indicative feed (Cboe delayed quotes are not read) | Alpaca keys |
| Analyst estimates and earnings surprises | Financial Modeling Prep | `FMP_API_KEY` in `.env` or the environment |
| Forward consensus snapshot, earnings dates | No source in Rust: cache files of the earlier Python code | None |
| Treasury par yield curves, nominal and real (TIPS) | U.S. Treasury | None |
| SOFR, EFFR, FOMC target range | Federal Reserve Bank of New York | None |
| CPI-U, headline and core | Bureau of Labor Statistics, API v1 | None |
| Zero-coupon yields and forward rates (fitted curve, GSW) | Federal Reserve Board, `feds200628.csv` | None |
| Cross-check of the rates and CPI | FRED (St. Louis Fed): the API with a key, else the CSV with no key | `FRED_API_KEY` (optional) |
| Rating step table (default spread of each rating) | A. Damodaran, January 2026 (fixed table in the code) | None |
| HQM corporate bond yield curve | U.S. Treasury (files) | None |

The pipeline supports US-GAAP filers on EDGAR. ETFs and IFRS filers cause an
`UnsupportedFilingError`. Segment revenue is not available from companyfacts
(`PLANNING/tickets.md`, Ticket 7).

## Layout

```
CLAUDE.md                 rules for agents (read first)
README.md                 this file
sec_contact.toml         the SEC contact (project address; SEC_USER_AGENT replaces it)
rust/mapcheck/            Rust: SEC data to standard variables, checks, filing statements
.env.example              names of the optional API keys
.github/workflows/        CI: runs the Rust tests on a pull request that changes code (not for a change of documents only)

financial_analysis/       the registry standard_variables.toml (data)
analysis/IBM/             IBM model (version 4), reviews, published report
analysis/JNJ/             JNJ first pass (not reviewed)
trading/                  research notes of the 22 option bots, LEADERBOARD.md (the code is in rust/src/trading/)
rust/                     Rust: `fa` command (the valuation, the report, the rates, the option trading: fetch, backtest, paper)

docs/                     ARCHITECTURE.md, HIDDEN-ASSUMPTIONS.md, INDEX.md, design documents
PLANNING/board/           open work, one file for each ticket
PLANNING/tickets.md       backlog without tickets
PLANNING/                 reusable-dcf-vision.md (design), RULES.md, token-ledger.md,
                          tickets-closed.md, proposals/
data/                     caches and outputs (not in git)
```
