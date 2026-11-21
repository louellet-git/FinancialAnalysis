# Long-history test of the market-level DCF: pre-registration

Date: 2026-11-21. Owner: Head of Economics. For the test of ticket T-20261121-dcf-engine-backtest.

This file fixes the test before any run. It is context for the other heads. It is not a model input and not a trade signal. A head that wants a macro input writes an assumption record. The board owns status.

## 1. Question and rules of change

Question: does the premium P1 forecast the later real excess return of the US stock market over 120 months? Yes or no, from the rule in section 4.

Rule of change: a rule that changes after the first result needs a new dated record in this file, with the reason.

Fixed choices (no change without a record):
1. Phase: the mean slope over all 120 phases, plus a January check.
2. Growth window: expanding, from 1871 to the last ended year known at t (section 3, g(t); section 5).
3. A window belongs to the period of its start month.
4. No pooling of countries into one N.
5. The headline premium is P1.

Reason for P1 over P2 (fixed 2026-11-21; no result was used for this choice):
- P1 uses dividends. Dividends are the cash flow paid to the holders. They are also the cash flow of the engine (the engine uses dividends only).
- P2 uses reported earnings. The accounting rules for earnings changed over the 150 years. Earnings also need a longer publication lag.

## 2. Data and terms

| Series | Address | Dates | Use | Terms |
|---|---|---|---|---|
| Shiller `ie_data.xls` (P, D, E, CPI, GS10 before 1953-04) | https://shillerdata.com/ (checked 2026-11-21) | 1871-01 (checked 2026-11-21) to 2026-11 (end date not checked: the page gives no end date) | test | The page states no licence (checked 2026-11-21). Disclaimer: "The user of this information assumes the entire risk of any use made of the information provided herein." (checked 2026-11-21). The store removes the copy if the author asks. Cite: Robert J. Shiller, U.S. stock market data, https://shillerdata.com/. |
| TB3MS | https://fred.stlouisfed.org/series/TB3MS | 1934-01 to 2026-10 | test | Public domain, citation requested. |
| CPIAUCNS | https://fred.stlouisfed.org/series/CPIAUCNS | 1913-01 to 2026-10 | test | Public domain, citation requested. |
| GS10 | https://fred.stlouisfed.org/series/GS10 | 1953-04 to 2026-10 | test | Public domain, citation requested. |
| M13002US35620M156NNBR (NBER commercial paper, New York, via FRED) | https://fred.stlouisfed.org/series/M13002US35620M156NNBR | 1857-01 to 1971-12 | test | Copyrighted, citation required. The NBER site refuses bots: use the FRED copy. |
| BAA, AAA (Moody's) | https://fred.stlouisfed.org/series/BAA | 1919-01 to 2026-10 | context only, not in the test | Copyrighted, citation required. |
| GSW curve (Federal Reserve Board) | https://www.federalreserve.gov/data/nominal-yield-curve.htm (checked 2026-11-21) | 1961 on (checked 2026-11-21) | context only | The page has no terms section. It links to a model disclaimer and to the site policies. It says the data is a staff research product, not an official statistical release (checked 2026-11-21). |
| Damodaran implied ERP | https://pages.stern.nyu.edu/~adamodar/New_Home_Page/datafile/implpr.html (checked 2026-11-21) | annual; the table starts 1960, the implied premium (DDM) starts 1961 (checked 2026-11-21) | context only | The page states no terms and no copyright (checked 2026-11-21). Terms unknown. |
| Bank of England, "A millennium of macroeconomic data" | https://www.bankofengland.co.uk/statistics/research-datasets (checked 2026-11-21) | some series from the 13th century; version 3.1 ends 2016 (checked 2026-11-21) | context only | The page states no terms and no licence (checked 2026-11-21). Terms unknown. |
| Jorda-Schularick-Taylor (JST) | https://www.macrohistory.net/database/ (checked 2026-11-21) | annual, 1870 on, 18 economies; latest release R6 (checked 2026-11-21) | context only; out of the scored set (section 6) | Creative Commons Attribution-NonCommercial-ShareAlike 4.0 (checked 2026-11-21). Name the release at download (the latest release then). |

Store rule (fixed 2026-11-21): a source with unknown terms gets no store copy until its terms are known.

Cite FRED series as: [source], [series name] [ID], retrieved from FRED, Federal Reserve Bank of St. Louis. The store files `macro/shiller/SOURCE.md` and `macro/fred/SOURCE.md` hold the full citations.

Traps:
- Shiller before 1926 is a reconstruction of Cowles type, not a traded price.
- Shiller D and E are annual values, interpolated to months.
- CPI before 1913 is annual, interpolated.
- Survivorship: the index of each date had the survivors of that date. Later the S&P 500 list is a list of survivors. The US is also a winner market.
- Shiller's file is a later revision. This is a limit.

## 3. Variables

Units (fixed 2026-11-21): all rates in the formulas are decimals per year (for example GS10/100, TB3MS/100). Percent is used only in the text.

**Known values at t** (fixed 2026-11-21; the rules are in section 5):
- D_k(t) is the dividend known at t. E_k(t) is the earnings known at t. Both are step values of section 5, rule 3.
- CPI_k(t) is the CPI known at t (section 5, rule 4).

**Outcome X(t,H)**, H = 120 (or 60 for the secondary test).
- TR(s) = (P(s) + D(s)/12) / P(s-1) - 1, where P is the monthly average of daily closes.
- B(s) = (1 + y/100)^(1/12) - 1, where y is the annual percent rate of month s-1 (known at the start of month s).
- G_TR and G_B are the products of (1 + TR) and (1 + B) for s = t+1 to t+H. c = CPI_t / CPI_(t+H).
- RT = (G_TR * c)^(12/H) - 1. RB = (G_B * c)^(12/H) - 1. X = RT - RB.
- The factor c measures the realized outcome. It is not a predictor. So the 1-month CPI lag does not apply to it. The lag protects predictors only.
- Outcome deflator (fixed 2026-11-21): Shiller CPI before 1913-01, FRED CPIAUCNS from 1913-01. For a window that crosses 1913-01, c is the product of the monthly CPI ratios, and each ratio comes from one series.

**Bill rate.** TB3MS from 1934-01. Before 1934, the NBER commercial paper rate (M13002US35620M156NNBR). Traps:
- TB3MS is on a discount basis. This is a little below a bond-equivalent yield.
- Commercial paper is a private rate with credit risk. So RB is biased up and X is biased down before 1934.
- The joined series has a step at 1934-01.

**Premium P1** = DY(t) + g(t) - r10(t).
- DY(t) = D_k(t) / P(t). D_k(t) is the annual dividend of section 5, rule 3 (a step value, no interpolation across t).
- g(t) (fixed 2026-11-21) is the end-point growth of the real annual dividend:
  g(t) = (Dr(Y_last) / Dr(1871))^(1/(Y_last - 1871)) - 1.
  Dr(Y) = the December D of year Y, divided by the December CPI of year Y (the step values of section 5). Y_last is the last ended year known at t (section 5, rule 3).
- The window has at least 20 years: Y_last - 1871 >= 20. By section 5, rule 3, Y_last = 1891 first at 1893-01 (December of Y-2). So the sample starts 1893-01. The test needs no constant for g. (Decision of the Head of Economics, 2026-11-21: the start moved from 1891-01 to 1893-01 because the C2 step rule gives an 18-year window at 1891-01.)
- r10(t) = GS10(t)/100 - ((CPI_k(t) / CPI_k(t-120))^(1/10) - 1). The engine uses the same definition (Head of Fundamental Research, 2026-11-21).
- Before 1953-04, GS10 is the Shiller long rate column. After that, GS10 is the FRED series.
- P1 is a premium over the 10-year real yield. X is over bills. So the intercept and the slope absorb the term premium.

**Premium P2** = 1/CAPE10 - r10(t). Rebuild CAPE10 from raw P, E and CPI. E uses the step and lag rule of section 5, rule 3. Never use the published CAPE column. Rebuild all yields from raw P, D, E, CPI and GS10.

**Engine premium.** To build by the Head of Fundamental Research, ticket T-20261121-dcf-engine-backtest, branch claude/dcf-market-level.

## 4. Primary test (one)

H0: P1 has no forecasting power for X(t,120).

- Sample end rule (fixed 2026-11-21): the last start month = (the last month for which each series of the test has a value) minus H. At the download of 2026-11-21, CPIAUCNS and GS10 end at 2026-10. So the last start month is 2016-10 for H = 120 and 2021-10 for H = 60. Each run records the latest data month that it used.
- Sample: monthly t from 1893-01 to 2016-10 (H = 120).
- Model: X(t,120) = a + b * P1(t) + e.
- Statistic: fit b with OLS in each of the 120 non-overlapping phases (the phase is the month of t; each phase takes every 120th month). The statistic b_hat is the mean of the 120 slopes.
- Bootstrap (fixed 2026-11-21): a circular block bootstrap (Politis and Romano 1992).
  1. Keep the monthly pairs (P1(t), X(t,120)) together.
  2. Resample circular blocks of 120 consecutive start months. Join the blocks to a pseudo-sample of the same length as the sample.
  3. Calculate the same statistic b*_k on each pseudo-sample.
  4. Use K = 10,000 pseudo-samples and the seed 20261121.
- p-value: the null comes from centring. One-sided (b > 0): p = (1 + #{k: b*_k - b_hat >= b_hat}) / (K + 1).
- The sample has about 12 blocks of 120 months. So the p-value is coarse.
- Fixed-phase check: report phase January alone.
- Out of sample: an expanding window. At month t the fit uses only pairs (s, X(s,120)) with s + 120 <= t. The first forecast comes when 20 years of complete pairs exist. The benchmark is the expanding historical mean of X. OOS R2 = 1 - SSE(model) / SSE(mean).
- Rejection of H0: one-sided p < 0.05 (b > 0) AND OOS R2 > 0.

The same rule, sample and inference apply to the engine premium.

Engine failure (confirmed by the Head of Fundamental Research, 2026-11-21): the engine premium fails if it does not meet the rejection rule, or if its OOS R2 is below the OOS R2 of P1.

## 5. Look-ahead rules

Fixed 2026-11-21:
1. Shiller P is the monthly average of daily closes. Treat it as known at the end of month t.
2. Never use the published CAPE column.
3. D known at t is a step value: the Shiller December D of the last ended year. Use the same rule as the engine:
   - before 1936-01: the December D of year Y-2 (Y is the year of t);
   - from 1936-01: the December D of year Y-1 from April of year Y, else of year Y-2.
   No interpolation across t. E (for P2 only) uses the same step and lag rule.
4. CPI known at t:
   - before 1913-01: the Shiller December CPI of the last ended year (a step value);
   - from 1913-01: FRED CPIAUCNS of month t-1.
5. The outcome deflator is not a predictor (section 3). It uses Shiller CPI before 1913-01 and CPIAUCNS from 1913-01.

Remaining look-ahead: before 1926, Shiller interpolates the annual D, E and CPI linearly to months. So a December value can carry some information of the next annual figure. The size is small. No test corrects it.

If a run cannot follow a rule, it states the look-ahead that remains.

## 6. Periods and secondary tests

Periods are monetary regimes. The periods use no return data. Each break has one source (addresses checked 2026-11-21).

| Period | Dates | Break at the end |
|---|---|---|
| P-1 | 1871-01 to 1913-12 | Classical gold standard. Federal Reserve Act, 1913-12 (https://www.federalreservehistory.org/essays/federal-reserve-act-signed). |
| P-2 | 1914-01 to 1933-12 | War, gold exchange standard. Gold devaluation 1933-34. |
| P-3 | 1934-01 to 1951-02 | Managed rates, yield peg from 1942. The Gold Reserve Act, 1934-01, starts it (https://www.federalreservehistory.org/essays/gold-reserve-act). The Treasury-Fed Accord, 1951-03, ends it (https://www.federalreservehistory.org/essays/treasury-fed-accord). |
| P-4 | 1951-03 to 1971-07 | The Accord to the end of gold convertibility, 1971-08 (https://www.federalreservehistory.org/essays/gold-convertibility-ends). |
| P-5 | 1971-08 to 1979-09 | High inflation. Volcker change, 1979-10 (https://www.federalreservehistory.org/essays/anti-inflation-measures). |
| P-6 | 1979-10 to 2008-11 | Disinflation. Zero bound, 2008-12 (FOMC statement, 2008-12-16: https://www.federalreserve.gov/newsevents/pressreleases/monetary20081216b.htm). |
| P-7 | 2008-12 to 2022-02 | Zero bound, QE. First hike, 2022-03 (FOMC statement, 2022-03-16: https://www.federalreserve.gov/newsevents/pressreleases/monetary20220316a.htm). |
| P-8 | 2022-03 on | No complete window. |

Window count (fixed 2026-11-21):
- A window belongs to the period of its start month.
- The number of independent windows in a period = floor(number of start months in the period / 120).
- By this count, no regime period has 3 or more windows. P-6 has 350 start months: 350/120 = 2.92, so 2.
- So the results for the regime periods P-1 to P-8 are descriptive only. They have no p-value and are not in the Holm family.
- The 30-year blocks (3 windows each) are the only partition with a scored test.

Holm family (fixed 2026-11-21, before any run): the scored secondary set for P1 has m = 6 tests. Each test uses the statistic and the bootstrap of section 4. Holm corrects the 6 p-values at a family-wise level of 0.05, one-sided.

| Test | Premium | H | Sample |
|---|---|---|---|
| S1 | P1 | 60 | 1893-01 to 2021-10 |
| S2 | P2 | 120 | 1893-01 to 2016-10 |
| B1 | P1 | 120 | 1893-01 to 1922-12 |
| B2 | P1 | 120 | 1923-01 to 1952-12 |
| B3 | P1 | 120 | 1953-01 to 1982-12 |
| B4 | P1 | 120 | 1983-01 to 2016-10 |

- The secondary tests of the engine are a separate family. The Head of Fundamental Research lists them before the engine run.
- Descriptive only (no p-value): the monetary-regime periods P-1 to P-8, and JST.
- JST: to be pre-registered before the JST download. It is out of the scored set: the data is annual and has no dividend level. Each country stays alone, with no pooling into one N.

Two meanings of overlap, kept apart in each result:
- Overlapping windows: windows of 120 months that start in adjacent months. The phase method handles them.
- Shared information: a slow premium and a long horizon give a high but false fit. The OOS R2 and the bootstrap handle it.

## 7. Open points

| Point | Owner |
|---|---|
| Terms of Damodaran and Bank of England before any store copy (the pages state no terms) | Head of Technology |
| Store copy of JST under CC BY-NC-SA 4.0 | Head of Technology |
| Terms of the BAA and AAA copy (context only) | Head of Technology |
| The engine premium build | Done: record R1 (section 9) |

## 8. Limits and what we did not do

- About 12 independent 120-month windows exist in the sample (1893-01 to 2016-10). One result has little power. The bootstrap p-value is coarse.
- The Shiller file is a later revision. The pre-1926 index is a reconstruction.
- Before 1926, the linear interpolation of Shiller leaves a small look-ahead (section 5).
- The Working effect: P is a monthly average of daily prices. So the measured returns have a small extra autocorrelation (Working 1960).
- The bill series joins two different rates at 1934-01.
- Survivorship stays in the US data (section 2).
- We did not run `fa`, download data, or test any number. No value in this file is a measured result.

## 9. Dated records after the freeze

The design was fixed at commit 2ffd392 (PASS-WITH-NOTES, Head of Quality and Risk, 2026-11-21). Each
record below was written before any later return was computed and before any scoring.

### R1. Engine premium and engine commit (2026-11-21, 04:20 UTC)

Given by the Head of Fundamental Research.

- **Engine commit:** branch `claude/dcf-market-level`, commit a6c0175 (2026-11-21 04:19:48 UTC). Data:
  store `louellet-git/fa-data` 8952a5a. Command: `fa dcf market --shiller macro/shiller/shiller_monthly.csv
  --cpi macro/fred/CPIAUCNS.csv --out FILE`. Model document: `docs/models/market-level-dcf.md`.
- **Engine premium (primary):** erp_implied(t) solves V(t) = the real price of month t. All values are real.
  - V(t) = sum over k = 1..N of D_k / (1 + r)^k + D_N (1 + L) / (r - L) / (1 + r)^N, with r = rr(t) + erp.
  - D_k = D_known_real(t) x product over j <= k of (1 + g_j); g_j = g0 + (L - g0)(j - 1)/(N - 1).
  - g0 = the real growth of the known December dividend over the last 10 known years, clamped to
    [-10%, +20%]. L = 2.0% real. N = 10.
  - D known at t (year Y): before 1936, December of Y-2; from 1936, December of Y-1 from April, else Y-2.
    Each December dividend is deflated with its own December CPI.
  - CPIK(t), the CPI known at t: the Shiller December CPI of Y-1 before 1913-01 (Shiller December 1912
    at 1913-01); CPIAUCNS of t-1 from 1913-02. This follows section 5 (C2).
  - rr(t) = gs10(t)/100 - ((CPIK(t)/CPIK(t-120))^(1/10) - 1). The same definition as r10 of P1.
  - Root by bisection on [-0.10, 0.30], with r > L.
- **Secondary family of the engine (Holm, m = 5, family-wise 0.05, one-sided):** erp_implied with
  L = 1%; with L = 3%; with N = 5; with N = 20; and erp_gordon = D_known_real(1 + L)/P + L - rr with L = 2%.
  The same statistic, bootstrap and sample as the primary test.
- **Two families:** the P1 family (m = 6) and the engine family (m = 5) are corrected separately. The headline
  of each is its primary test (P1; the engine premium at L = 2%, N = 10), not the best of the eleven tests.
- **Disclosure (the words of the Head of Fundamental Research):** "Before the scoring, the engine ran three
  times: before the pre-1936 lag fix, after it, and after the C2 CPI change (a6c0175). Only the decade
  medians and the P10 / P90 of erp_implied were seen. No later return was computed, and nothing was scored."

### R2. A missing CPI month (2026-11-21)

CPIAUCNS has no value for 2025-10 in the store copy (8952a5a): the value was not published. Rule: a
month with no CPI value gives no predictor value at the month that needs it (the engine marks 2025-11
`no_cpi`), and a window whose outcome deflator needs that month (the end month 2025-10, start 2015-10
for H = 120 and 2020-10 for H = 60) gives no outcome. These months drop out of the sample. No other
value is filled. The count of dropped months is reported in the findings.
