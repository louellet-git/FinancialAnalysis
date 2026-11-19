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
python -m pytest                       # 331 tests, no network
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
python -m scripts.verdict IBM --price 228.53 [--simulations 10000] [--seed 7]
python -m analysis.IBM.dcf_drivers_run     # IBM version 4 headline numbers (seed 7)
python -m analysis.IBM.dcf_scenarios       # IBM version 3 (historical)
python -m analysis.JNJ.first_pass_run      # JNJ first pass
```

`scripts.run_dcf` prints an enterprise value for each share. It does not subtract net
debt. Do not compare this number with a share price.

Remote data has a disk cache in `data/raw/TICKER/`. Option chains and consensus
estimates are saved as dated snapshots, because yfinance gives only current data. A run
on the same day reads the disk; `--refresh` gets new data.

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
| Income Statement | Approximately 25 metrics for each period |
| Balance Sheet | Approximately 24 metrics, and RPO rows (`statement=disclosure`) |
| Cash Flow | Operating cash flow, capex, free cash flow, buybacks, dividends, debt flows |
| Raw | All us-gaap USD concepts that have no mapping |
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

The pipeline supports US-GAAP filers on EDGAR. ETFs and IFRS filers cause an
`UnsupportedFilingError`. Segment revenue is not available from companyfacts
(`PLANNING/tickets.md`, Ticket 7).

## Layout

```
CLAUDE.md                 rules for agents (read first)
README.md                 this file
requirements.txt          runtime packages
requirements-dev.txt      pytest

financial_analysis/       valuation package (import, store, engines, backtest, verdict)
analysis/IBM/             IBM model (version 4), reviews, published report
analysis/JNJ/             JNJ first pass (not reviewed)
trading/                  option strategy backtester, paper trader, 22 bots, research notes
scripts/                  command-line entry points
tests/                    331 tests

docs/                     ARCHITECTURE.md, HIDDEN-ASSUMPTIONS.md, INDEX.md, design documents
PLANNING/board/           open work, one file for each ticket
PLANNING/tickets.md       backlog without tickets
PLANNING/                 reusable-dcf-vision.md (design), RULES.md, token-ledger.md,
                          tickets-closed.md, proposals/
data/                     caches and outputs (not in git)
```
