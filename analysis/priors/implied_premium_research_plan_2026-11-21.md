# Research plan: what is in the higher implied premium (2026-11-21)

For: the Head of Fundamental Research. Question of the owner (Luc), 2026-11-21: "Can we break down higher implied
risk premium? Is there any research work that can be done on this assumption for understanding it/quantifying it
better". Status: a research plan. No code, no new run. A market-implied value is a measurement, not an input of the
model. The value at the means is not the expected value.

## 1. Short answer

Yes, in part. The plug of `fa dcf attribute` is one number that holds four things: a change of the premium that is
common to all names, a part that is specific to one name, growth news that the model cannot see, and model
conventions. Data in the store can split the first two now, and can compare the common part with VIX and credit
spreads. The growth part needs a history of analyst forecasts. The store has one snapshot only. **First item: 4.1
(common part against specific part).** It uses the commands that exist, the data that exists, and it is necessary
for items 4.2 and 4.3.

Fact for the scale (`analysis/priors/data/sp100_week_2026-11-20.csv`, columns `premium1`, `premium2`): in the week
2026-11-13 to 11-20, the implied premium went up for 64 of 64 S&P 100 names with a premium view. The median change is
+0.79 points (P10 +0.44, P90 +1.38). The value-weighted mean change is +0.83 points. So most of the change is common.

## 2. Literature (each citation confirmed by a web search on 2026-11-21)

| Source | The point that matters here | What it adds to our split |
|---|---|---|
| Damodaran, *Equity Risk Premiums: Determinants, Estimation and Implications* (annual paper; 2022 edition, pages.stern.nyu.edu) | Implied premium = the rate that makes the index price equal to expected cash flows. | An outside market series. Source of the model premium 4.41% (`docs/models/auto-path-model.md`, line 51). |
| Claus and Thomas (2001), *J. Finance* 56(5) 1629-1666 | Analyst forecasts give a US premium of about 3% or less, 1985 to 1998. | A forecast-based premium, outside our DCF. |
| Gebhardt, Lee and Swaminathan (2001), *J. Accounting Research* 39(1) 135-176 | Implied cost of capital per firm; industry, book/market, forecast growth and forecast dispersion explain about 60% of its cross-section. | Firm traits that explain the specific part (item 4.1). |
| Easton (2004), *Accounting Review* 79(1) 73-95 | PE- and PEG-based implied returns have a downward bias. | Known bias 1: the method. |
| Easton and Sommers (2007), *J. Accounting Research* | Analyst optimism biases the implied return up. | Known bias 2: the forecasts. |
| Campbell and Shiller (1988), *Rev. Financial Studies* 1(3) 195-228 | The log dividend-price ratio is a linear sum of expected future returns and expected future dividend growth. | The identity behind "premium or growth": our two views are its two terms. |
| Campbell (1991), *Economic Journal* 101(405) 157-179 | Cash-flow news gives about one third to one half of the variance of unexpected market returns. | A VAR split of a price change into cash-flow news and discount-rate news. A benchmark for item 4.3. |
| Campbell and Vuolteenaho (2004), *AER* 94(5) 1249-1275 | A beta has a cash-flow part and a discount-rate part. Growth and large stocks have mostly the discount-rate part. | Large growth names (the S&P 10) carry mostly the discount-rate part. |
| Martin (2017), *QJE* 132(1) 367-433 | SVIX from index option prices gives a lower bound of the expected market excess return. Mean about 5%. | A premium from option prices that does not use our DCF (item 4.4). It needs the negative correlation condition; option horizon only. |
| Bollerslev, Tauchen and Zhou (2009), *Rev. Financial Studies* 22(11) 4463-4492 | Variance risk premium = implied variance less realized variance. It predicts quarterly returns. | A second option measure, with data in the store (item 4.4). |
| Gilchrist and Zakrajsek (2012), *AER* 102(4) 1692-1720 | A credit spread = expected default + an excess bond premium. The excess bond premium holds the predictive power. | The risk-bearing part of credit. An explanatory variable for item 4.2. |

## 3. Data inventory (store `/home/user/fa-data-split`, commit f634633)

| Data | File | Last date | Note |
|---|---|---|---|
| VIX close (Cboe) | `vix/VIX_History.csv` | 2026-11-19 | No 2026-11-20 row. Also `trading/_VIX_1d.csv`, `_VXN_1d.csv` (to 11-18), `_VIX_5m.csv` (2026-08-27 to 11-18). |
| Moody's Baa less 10-year Treasury, daily | `bonds/fred_BAA10Y.csv` | 2026-11-18 | From 1986-01-02. Monthly `macro/fred/BAA.csv`, `GS10.csv`: to 2026-10-01. |
| ICE BofA OAS (BBB, B, ...) | `bonds/fred_BAMLC0A4CBBB.csv` and others | 2026-11-18 | 786 rows only. `macro/fred/SOURCE.md` says ICE limits these series: check the terms before a study. |
| NFCI, STLFSI4 (weekly); T10Y3M (daily) | `macro/fred/` | 2026-11-13; T10Y3M 2026-11-20 | |
| Treasury nominal and real par yields | `rates/treasury_nominal_202[56].csv`, `treasury_real_202[56].csv` | 2026-11-20 | 2025 and 2026 only. Breakeven = nominal less real (par). |
| GSW nominal curve | `rates/gsw_feds200628.csv` | 2026-11-13 | No GSW real (TIPS) curve in the store. |
| Shiller monthly | `macro/shiller/shiller_monthly.csv` | 2026-11 | 1871 on. The last two rows are estimates (column `estimate`). |
| Consensus estimates (Nasdaq) | `<TICKER>/estimates/<date>.json` | 2026-11-19 | One snapshot for 503 tickers; NVDA also 2026-11-20. Fields: mean, low, high, analysts, `up`, `down`. The file does not say the window of `up`/`down`. |
| Option chains | `SPX/options/2026-11-20.json` | 2026-11-20 | One snapshot, SPX and SPXW, 57 expiries to 2031-12-19, bid and ask (Alpaca indicative feed), forward from put-call parity. `IBM/options`, `JNJ/options`: 2026-11-19. |
| VX futures settlements | `futures/cboe_settle_2026-11-19.csv` | 2026-11-19 | Six expiries show the same price 24.5357. Check this before use. |
| SPY bars | `trading/SPY_5m.csv`, `SPY_1d.csv` | 2026-11-18 | 5-minute bars from 2026-08-26 only; daily from 1993. |

**Missing:** a history of analyst forecasts and of their revisions; a history of option chains; a daily implied premium
series of Damodaran; an excess bond premium series; a TIPS curve before 2025; VIX and credit spreads for 2026-11-20.

## 4. First measurements: the S&P 10, each Friday

Premium: `premium1` of `analysis/priors/data/sp10_2m/w_<next Friday>.csv` (the start date of each weekly file); for
2026-11-20, `premium2` of `w_2026-11-20.csv`. "Fixed 5" = AAPL, MSFT, GOOG, META, WMT, the names with a premium view on
every date. "All" = each name with a value in that file. The weight is `value1` (price x shares at the start; for
11-20, `price2` x `shares`). GOOGL has no weight (GOOG carries Alphabet). Zero: the 10-year zero of the par fit (the
`curve` line of each `w_*.txt`). VIX: `vix/VIX_History.csv`. Baa-10y: `bonds/fred_BAA10Y.csv`. Real and nominal:
the 10-year par yields of the Treasury files.

| Friday | Fixed 5 VW | Fixed 5 median | All VW (n) | All median | 10y zero | VIX | Baa-10y | Real 10y | Breakeven 10y |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 09-18 | 0.90% | 0.93% | 0.90% (5) | 0.93% | 4.99% | 14.81 | 1.40 | 2.68 | 2.33 |
| 09-25 | 0.52% | 0.82% | 0.52% (5) | 0.82% | 5.15% | 14.87 | 1.46 | 2.83 | 2.34 |
| 10-02 | 0.51% | 0.71% | 0.51% (5) | 0.71% | 5.27% | 15.31 | 1.47 | 2.92 | 2.36 |
| 10-09 | 0.49% | 0.32% | 0.49% (5) | 0.32% | 5.23% | 14.84 | 1.45 | 2.91 | 2.33 |
| 10-16 | 0.55% | 0.13% | 0.21% (6) | -0.27% | 5.08% | 19.99 | 1.46 | 2.74 | 2.35 |
| 10-23 | 0.73% | 0.05% | 0.40% (6) | 0.05% | 4.98% | 19.45 | 1.47 | 2.66 | 2.34 |
| 10-30 | 0.90% | 0.59% | 0.53% (6) | 0.41% | 4.92% | 19.42 | 1.53 | 2.60 | 2.34 |
| 11-06 | 1.34% | 1.46% | 1.34% (5) | 1.46% | 4.85% | 25.64 | 1.56 | 2.63 | 2.23 |
| 11-13 | 0.94% | 1.11% | 0.30% (6) | 0.54% | 5.13% | 19.30 | 1.47 | 2.91 | 2.24 |
| 11-20 | 1.41% | 1.45% | 1.17% (6) | 1.24% | 4.90% | 28.44 (11-19) | 1.52 (11-18) | 2.77 | 2.15 |

The sixth name of "All" is AVGO on 10-16 to 10-30 (-3.12%, -2.87%, -2.95%) and NVDA on 11-13 and 11-20 (-2.20%,
+0.07%). No premium view on any date: AMZN, TSLA, LLY. NVDA and AVGO have no premium view on the other dates. A
weekly file shows a premium only when both of its dates have one: AVGO has -1.98% on 11-06 in `premium2` of
`w_2026-11-06.csv`, but no `premium1` in `w_2026-11-13.csv`. BRK-B and V have no weight (no share count).

**Co-movement (words only; 10 dates do not show a cause).**
1. From 09-18 to 11-20, the fixed-5 premium went from 0.90% to 1.41%, VIX from 14.81 to 28.44 (11-19), Baa-10y from
   1.40 to 1.52 (11-18). The breakeven went down from 2.33 to 2.15.
2. Week to week, the fixed-5 premium and VIX moved in the same direction in 5 of 9 steps. In the 4 steps where VIX
   moved by more than 5 points (to 10-16, 11-06, 11-13, and to 11-20 with the VIX of 11-19), the premium moved in the same direction.
3. The premium and the 10-year zero often move in opposite directions (11-06 to 11-13: zero +0.28 points, premium
   -0.39 points). Part of this is mechanical: the premium is the rest of the cost of capital after the zero yield. A
   lower zero at the same price gives a higher implied premium.
4. The level (0% to 1.5%) is far below the model premium 4.41%. The level holds model conventions (terminal growth,
   fade; `analysis/priors/dcf_conventions_study_2026-11-20.md`, section 1.3). The change is the better measurement.

## 5. Research proposal

**Pre-reg** = write the rule, the sample and the pass criterion before the run, and commit them (precedent:
`analysis/macro/2026-11-21-long-history-design.md`; `trading/hypotheses/README.md`; skill `point-in-time-test`).

| # | Question | Method | Data | What it quantifies | Rust work | What it cannot show | Pre-reg |
|---|---|---|---|---|---|---|---|
| 4.1 | How much of the weekly change of the implied premium is common to all names? | Run `fa dcf attribute` each week for 2 to 5 years (a shell loop of the existing command). For each week: the cross-section mean of the change (common part); the residual of each name. Report the share of the variance that is common, by sector and by duration. | SEC facts, prices, GSW curve: in the store | The common part against the specific part, in points and in percent of the variance | 2 to 3 days: a command that reads the weekly CSV files and gives the panel numbers (with tests) | Why the common part moved; the growth part stays in it | No for the description; yes for a claim about a trait (for example, duration) |
| 4.2 | Which part of the common change goes with a risk measure? | Regress the weekly common change of 4.1 on the changes of VIX, Baa-10y, BBB OAS, NFCI, real 10y and breakeven. Standard errors robust to autocorrelation. | All in the store (to 11-18 or 11-20) | The share that a risk measure explains (R squared, the size of each coefficient) | 2 days: a least-squares fit with tests | A cause. The mechanical link to the zero yield (control for it). | Yes |
| 4.3 | Which part of the plug is growth news? | (a) Start a daily `fa estimates get` for the S&P 100 now. After 12 weeks, regress the change of the specific part on the revision of the consensus EPS. (b) A Campbell (1991) VAR on the Shiller monthly data for the index. | (a) Missing: collect forward. (b) `macro/shiller/` | The part of the plug that analyst revisions explain; the share of discount-rate news in the index history | (a) 0 days to collect, 1 to 2 days to fit; (b) 3 to 5 days | Analyst forecasts have biases (Easton and Sommers 2007). The VAR result depends on the predictors. | Yes |
| 4.4 | What premium do option prices give, without our DCF? | The SVIX lower bound of Martin (2017) from the SPX chain, for 1, 3 and 12 months. The variance risk premium: VIX squared less the realized variance of SPY 5-minute bars. | One SPX snapshot (2026-11-20); daily collection is necessary for a series. Bars from 2026-08-26. | A market premium for a short horizon, independent of the model; a check of the sign of our weekly change | 2 to 3 days (SVIX integral, tests with a known price set) | The long-run premium of a DCF. One snapshot gives one number, not a change. The feed is indicative. | No for the measurement; yes for a test against 4.1 |
| 4.5 | How much of the level is model convention? | Run `fa dcf implied` on the S&P 100 with `--rules` changes: `long_run`, `terminal_rf=1`, `years`. | In the store | The change of the implied premium for each convention | 0.5 day, no code | Which convention is correct | No |

**Recommendation: item 4.1 first.** Reasons:
1. The week shows a rise for 64 of 64 names. A panel shows if this is usual or rare.
2. It needs no new data and only a small Rust command. The weekly runs use existing commands.
3. Items 4.2 and 4.3 need its common series and its residuals.
4. It keeps the rule: the premium stays a measurement. No input of the model changes.

Item 4.5 costs 0.5 day and can run in parallel. Start the daily collection of estimates (4.3a) and of the SPX chain
(4.4) now, because a history cannot be made later.
