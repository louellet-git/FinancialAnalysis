# Long-history test of the market-level DCF: pre-registration

Date: 2026-11-21. Owner: Head of Economics. For the test of ticket T-20261121-dcf-engine-backtest.

This file fixes the test before any run. It is context for the other heads. It is not a model input and not a trade signal. A head that wants a macro input writes an assumption record. The board owns status.

## 1. Question and rules of change

Question: does the premium P1 forecast the later real excess return of the US stock market over 120 months? Yes or no, from the rule in section 4.

Rule of change: a rule that changes after the first result needs a new dated record in this file, with the reason.

Fixed choices (no change without a record):
1. Phase: the mean slope over all 120 phases, plus a January check.
2. Growth window: expanding, from 1871-01 to the last ended year before t.
3. A window belongs to the period of its start month.
4. No pooling of countries into one N.
5. The headline premium is P1.

## 2. Data and terms

| Series | Address | Dates | Use | Terms |
|---|---|---|---|---|
| Shiller `ie_data.xls` (P, D, E, CPI, GS10 before 1953-04) | https://shillerdata.com/ | 1871-01 to 2026-11 | test | The page states no licence. Disclaimer: "The user of this information assumes the entire risk of any use made of the information provided herein." The store removes the copy if the author asks. Cite: Robert J. Shiller, U.S. stock market data, https://shillerdata.com/. |
| TB3MS | https://fred.stlouisfed.org/series/TB3MS | 1934-01 to 2026-10 | test | Public domain, citation requested. |
| CPIAUCNS | https://fred.stlouisfed.org/series/CPIAUCNS | 1913-01 to 2026-10 | test | Public domain, citation requested. |
| GS10 | https://fred.stlouisfed.org/series/GS10 | 1953-04 to 2026-10 | test | Public domain, citation requested. |
| M13002US35620M156NNBR (NBER commercial paper, New York, via FRED) | https://fred.stlouisfed.org/series/M13002US35620M156NNBR | 1857-01 to 1971-12 | test | Copyrighted, citation required. The NBER site refuses bots: use the FRED copy. |
| BAA, AAA (Moody's) | https://fred.stlouisfed.org/series/BAA | 1919-01 to 2026-10 | context only, not in the test | Copyrighted, citation required. |
| GSW curve (Federal Reserve Board) | https://www.federalreserve.gov/data/nominal-yield-curve.htm | 1961 on | context only | Public data. |
| Damodaran implied ERP | https://pages.stern.nyu.edu/~adamodar/ | 1960 on | context only | Terms to check. |
| Bank of England, "A millennium of macroeconomic data" | https://www.bankofengland.co.uk/statistics/research-datasets | about 1700 on | context only | Terms to check. |
| Jorda-Schularick-Taylor (JST) | https://www.macrohistory.net/database/ | 1870 on | secondary test | Terms to check before any store copy. Name the release at download (the latest release then). |

Cite FRED series as: [source], [series name] [ID], retrieved from FRED, Federal Reserve Bank of St. Louis. The store files `macro/shiller/SOURCE.md` and `macro/fred/SOURCE.md` hold the full citations.

Traps:
- Shiller before 1926 is a reconstruction of Cowles type, not a traded price.
- Shiller D and E are annual values, interpolated to months.
- CPI before 1913 is annual, interpolated.
- Survivorship: the index of each date had the survivors of that date. Later the S&P 500 list is a list of survivors. The US is also a winner market.
- Shiller's file is a later revision. This is a limit.

## 3. Variables

All rates are decimals per year, unless the text says percent.

**Outcome X(t,H)**, H = 120 (or 60 for the secondary test).
- TR(s) = (P(s) + D(s)/12) / P(s-1) - 1, where P is the monthly average of daily closes.
- B(s) = (1 + y/100)^(1/12) - 1, where y is the annual percent rate of month s-1 (known at the start of month s).
- G_TR and G_B are the products of (1 + TR) and (1 + B) for s = t+1 to t+H. c = CPI_t / CPI_(t+H).
- RT = (G_TR * c)^(12/H) - 1. RB = (G_B * c)^(12/H) - 1. X = RT - RB.
- The factor c measures the realized outcome. It is not a predictor. So the 1-month CPI lag does not apply to it. The lag protects predictors only.

**Bill rate.** TB3MS from 1934-01. Before 1934, the NBER commercial paper rate (M13002US35620M156NNBR). Two traps:
- TB3MS is on a discount basis. This is a little below a bond-equivalent yield.
- Commercial paper is a private rate with credit risk. So RB is biased up and X is biased down before 1934.
- The joined series has a step at 1934-01.

**Premium P1** = DY(t) + g(t) - r10(t).
- DY(t) = the sum of the last 12 monthly dividends (D/12 each month), divided by P(t).
- g(t) = the annualized real dividend growth over the expanding window from 1871-01 to the last ended year before t. The window has at least 20 years. So the sample starts 1891-01. The test needs no constant for g.
- r10(t) = GS10(t) less the trailing 10-year CPI change, annualized: (CPI_(t-1) / CPI_(t-121))^(1/10) - 1. CPI_(t-1) is the CPI known at month t (the 1-month lag). Units: percent per year. The engine uses the same definition (Head of Fundamental Research, 2026-11-21).
- Before 1953-04, GS10 is the Shiller long rate column. After that, GS10 is the FRED series.
- P1 is a premium over the 10-year real yield. X is over bills. So the intercept and the slope absorb the term premium.

**Premium P2** = 1/CAPE10 - r10(t). Rebuild CAPE10 from raw P, E and CPI. Never use the published CAPE column. Rebuild all yields from raw P, D, E, CPI and GS10.

**Engine premium.** To build by the Head of Fundamental Research, ticket T-20261121-dcf-engine-backtest, branch claude/dcf-engine-backtest.

## 4. Primary test (one)

H0: P1 has no forecasting power for X(t,120).

- Sample: monthly t from 1891-01 to 2016-11. This is the last start month for H = 120. For H = 60 it is 2021-11.
- Model: X(t,120) = a + b * P1(t) + e.
- Inference: fit b in each of the 120 non-overlapping phases (the phase is the month of t; each phase takes every 120th month). Take the mean of the 120 slopes. The p-value comes from a moving-block bootstrap: block length 120 months, 10,000 draws, seed 20261121.
- Fixed-phase check: report phase January alone.
- Out of sample: an expanding window. At month t the fit uses only pairs (s, X(s,120)) with s + 120 <= t. The first forecast comes when 20 years of complete pairs exist. The benchmark is the expanding historical mean of X. OOS R2 = 1 - SSE(model) / SSE(mean).
- Rejection of H0: one-sided p < 0.05 (b > 0) AND OOS R2 > 0.

The same rule, sample and inference apply to the engine premium.

Engine failure (confirmed by the Head of Fundamental Research, 2026-11-21): the engine premium fails if it does not meet the rejection rule, or if its OOS R2 is below the OOS R2 of P1.

## 5. Look-ahead rules

1. Shiller P is the monthly average of daily closes. Treat it as known at the end of month t.
2. CPI before 1913 is annual (interpolated). Use only CPI of years that have ended.
3. Earnings lag 12 months before 1936-01 and 3 months from 1936-01.
4. D and E: use only annual values of years that ended before t (as in the current file).
5. Never use the published CAPE column.

If a run cannot follow a rule, it states the look-ahead that remains.

## 6. Periods and secondary tests

Periods are monetary regimes. The sources for each break are the Federal Reserve History essays (https://www.federalreservehistory.org/). The periods use no return data.

| Period | Dates | Break at the end |
|---|---|---|
| P-1 | 1871-01 to 1913-12 | Classical gold standard. Federal Reserve Act, 1913-12. |
| P-2 | 1914-01 to 1933-12 | War, gold exchange standard. Gold devaluation 1933-34. |
| P-3 | 1934-01 to 1951-02 | Managed rates, yield peg from 1942. Gold Reserve Act 1934-01 starts it. Treasury-Fed Accord, 1951-03, ends it (Federal Reserve History, "The Treasury-Fed Accord"). |
| P-4 | 1951-03 to 1971-07 | The Accord to the end of gold convertibility, 1971-08. |
| P-5 | 1971-08 to 1979-09 | High inflation. Volcker change, 1979-10. |
| P-6 | 1979-10 to 2008-11 | Disinflation. Zero bound, 2008-12. |
| P-7 | 2008-12 to 2022-02 | Zero bound, QE. First hike, 2022-03. |
| P-8 | 2022-03 on | Unscored: no complete window. |

- A window belongs to the period of its start month.
- Count the independent 120-month windows in each period. Run no regression in a period with fewer than 3. Report such a period as a description.
- Alternative partition (pre-declared): equal 30-year blocks from 1891-01.

Secondary tests are descriptive, or use a Holm correction over the secondary set:
- H = 60;
- P2;
- the sub-periods and the alternative partition;
- JST: P1 only (JST has no earnings), each country alone, no pooling into one N.

Two meanings of overlap, kept apart in each result:
- Overlapping windows: windows of 120 months that start in adjacent months. The phase method handles them.
- Shared information: a slow premium and a long horizon give a high but false fit. The OOS R2 and the bootstrap handle it.

## 7. Open points

| Point | Owner |
|---|---|
| Terms of JST, Damodaran, Bank of England before any store copy | Head of Technology |
| Terms of the BAA and AAA copy (context only) | Head of Technology |
| The engine premium build | Head of Fundamental Research |
| Confirm the engine failure rule | Head of Fundamental Research |

## 8. Limits and what we did not do

- About 12 independent 120-month windows exist in the sample (1891-2016). One result has little power.
- The Shiller file is a later revision. The pre-1926 index is a reconstruction.
- The bill series joins two different rates at 1934-01.
- Survivorship stays in the US data (section 2).
- We did not run `fa`, download data, or test any number. No value in this file is a measured result.
