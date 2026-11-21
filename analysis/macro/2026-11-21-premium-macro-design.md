# Implied premium and macro risk measures: pre-registration

Date: 2026-11-21. Owner: Head of Economics. For the ticket T-20261121-economics-premium-macro.

This file fixes the test before any run. It is context for the other heads. It is not a model input and not a trade signal. A head that wants a macro input writes an assumption record. The board owns status. No weekly series value is in this file, except the disclosure in rule 9.

## 1. Question and aim

Question of the person (relayed by Fundamental Research, 2026-11-21): "Can we break down higher implied risk premium? ... understanding it/quantifying it better."

Our part is the market level. The question for this design: what share of the weekly change of the market-implied premium do the macro risk measures explain out of sample? The answer is a number, and yes or no by the rule in rule 5.

The design names the share that the macro measures explain. The rest stays unnamed. A residual is not a cause. The share is a statistical link, not a proof of cause.

Stock-level work is the part of Fundamental Research. It is not in this file.

## 2. Terms

- The premium is the premium of the engine of `fa dcf implied`: the premium at which the median of value/price over the list is 1. Unit: percentage points per year.
- The list is the S&P 500 list (primary) or the S&P 100 list (secondary). Both are today's lists, so survivorship stays (rule 8).
- A week w ends on a Friday. A change is the value of Friday w minus the value of Friday w-1, or the last value on or before each Friday.

## 3. Fixed rules

Each rule is fixed on 2026-11-21 before any run. Fundamental Research gives the series at a point in time.

### Rule 1. Series

One value each Friday, from 2018-01-05.

| Id | Definition | Command |
|---|---|---|
| S1 | The LEVEL of the premium that the S&P 500 list implies | `fa dcf implied --tickers-file <list> --price-date <Friday>` |
| S2 | The weekly premium part (points) of the value-weighted bridge, the "all" line. The rates part and the no-model part stay beside it | `fa dcf attribute --from <Friday-1> --to <Friday>` |

The S&P 100 list gives a second S1 and a second S2.

### Rule 2. Outcome of the primary test

y_w = S1(w) - S1(w-1), in percentage points, S&P 500 list. S2 and the S&P 100 series are secondary.

### Rule 3. Sample

- From 2019-01-04 to the last Friday with all series.
- 2018 is out. The beta is 1 for every name until about 2018-11 (Fundamental Research).
- The curve is GSW for the history. It is the par fit for the last weeks, because GSW lags about one week.
- Fundamental Research states the switch week. The findings report it.

### Rule 4. Regressors

Each regressor is a weekly change: Friday close to Friday close, or the last value on or before the Friday.

| Name | Definition | Source |
|---|---|---|
| dVIX | Change of the Cboe VIX close (index points) | store `vix/VIX_History.csv` |
| dVRP | Change of VRP = (VIX/100)^2 minus the realized variance of SPY daily log returns over the last 21 trading days x 252 (variance, decimal) | store, VIX file and SPY prices |
| dBAA10Y | Change of FRED BAA10Y, daily (percentage points) | store `bonds/fred_BAA10Y.csv` |
| dREAL10 | Change of FRED DFII10, the 10-year TIPS yield, daily (percentage points) | not in the store yet |
| dBEI10 | Change of FRED T10YIE, daily (percentage points) | not in the store yet |
| dNFCI | Change of the Chicago Fed NFCI, weekly (index points); the value of the week that ends on or before the Friday, as published | store `macro/fred/NFCI.csv` |

- DFII10 and T10YIE are a data request to the Head of Technology.
- If a series is not in the store before the freeze, the design drops that regressor and says so in the findings. No substitute is used.
- NFCI is revised. We use the latest values, not vintages. This is a limit.

### Rule 5. Primary statistic and inference

The primary statistic is the out-of-sample (OOS) R2 of an expanding-window OLS of y on the regressors.

- The first fit uses the first 104 weeks. Each later week uses only earlier weeks.
- Benchmark 1: zero change (no change in the premium).
- Benchmark 2: the expanding mean of y.
- OOS R2 = 1 - SSE(model) / SSE(benchmark), for each benchmark.

Inference: a circular block bootstrap of weeks.
1. Block length: 8 weeks. K = 10,000. Seed 20261121.
2. The null is no link: resample the y blocks independent of the regressor blocks.
3. Calculate the OOS R2 on each pseudo-sample.
4. The p-value is the share of pseudo-sample OOS R2 values at or above the observed value, with the (1 + count) / (K + 1) form.

Rule: the macro measures explain a share of the premium changes out of sample only if the OOS R2 is above 0 against both benchmarks AND the bootstrap p is below 0.05. If the rule fails, the answer is "no share shown". It is not "zero share".

### Rule 6. Description (no test, no p-value)

1. The in-sample R2 and the coefficients with Newey-West errors (lag 8).
2. The same for S2 (points) and for the S&P 100.
3. The R2 inside the stress windows of `analysis/macro/2026-11-21-stress-windows.md` (2019 onward), against the other weeks.
4. A variance split of S2 against its rates part and its no-model part.

A description is not a result. It does not change the answer of rule 5.

### Rule 7. Sanity check (before the primary is read)

The rates part of S2 must move with the weekly change of the 10-year zero yield. The correlation must be below -0.5. A rise in the yield lowers the value, so the sign is negative.

If the correlation is not below -0.5, the bridge has a defect. The test stops. Fundamental Research finds the defect. No primary result is read before this check passes.

### Rule 8. Traps

- The premium is a residual. A growth or cash-flow error of the model enters it. Chen, Da and Zhao (2013), "What Drives Stock Price Movements?", RFS 26(4).
- The VIX mixes risk and risk aversion. Bekaert, Hoerova and Lo Duca (2013), JME 60(7). So a link with dVIX does not say which of the two moved.
- The excess bond premium (Gilchrist and Zakrajsek 2012, AER 102(4)) is monthly and revised. It is out of a weekly test. A monthly descriptive check can follow, if Technology adds the series.
- Most premium predictors fail out of sample. Goyal and Welch (2008), RFS 21(4). This is the reason for the OOS rule.
- Survivorship: the lists are today's names.
- NFCI revisions (rule 4).
- The regressors share information. The coefficients can be unstable. The design reports the share of the set, not a share for each regressor.

### Rule 9. Disclosure

Before this design, the Head of Economics saw one weekly attribution and the Oct-Nov 2026 split.

The week 2026-11-13 to 2026-11-20 (from Fundamental Research): S&P 100 -9.40%; rates +4.93 points; premium -10.71; growth -1.68; no model -2.03.

These weeks are in the sample. No other weekly value was seen. So the test is not fully blind for these weeks. One week of about 400 has a small weight, but the Oct-Nov 2026 weeks are in the stress period, and the stress description (rule 6) can carry this.

### Rule 10. Change rule

A change after the first result needs a new dated record in this file. The record says that it follows a result. It names the run of record and each superseded run.

### Rule 11. Work split

- Fundamental Research produces S1 and S2, after the merge of the Technology fix `claude/pit-supplement-fill`. It gives the commit.
- Economics runs the regressions. The program is in Rust, the main language of the repo. The team of the head that owns the code area writes it, or Fundamental Research writes it in `fa dcf`. This is an open point. The owner is both heads.
- Quality and Risk freezes this design and audits the run.

### Rule 12. Open points

| Point | Owner |
|---|---|
| The request for DFII10 and T10YIE in the store | Head of Technology |
| The code owner of the regression | Fundamental Research and Economics |
| The switch week from GSW to the par fit | Fundamental Research |
| The freeze of this design | Quality and Risk |

## 4. Two readings of the result

Each result has a second reading. The findings give both.

- A high OOS R2 for the credit and VIX measures can mean that the premium moves with risk aversion. It can also mean that the model price reacts to the same rates and spread data that the regressors hold. The sanity check of rule 7 covers only the rates part.
- A low OOS R2 can mean that the premium moves for other reasons. It can also mean that the weekly noise of the model premium hides a real link. The test cannot tell these two apart.
- A market-implied series (BEI10, the VIX) is the price of the market. It is not a forecast of this repo.

## 5. Limits and what we did not do

- About 400 weeks. The first 104 weeks only train. The OOS window is short, and the p-value is coarse.
- The sample holds a few large episodes. A few weeks can drive the OOS R2.
- The premium is a model quantity. It changes with the engine and the list. A change of the engine commit needs a new run.
- The weekly step of S1 has noise from the model (the median of value/price over a list).
- NFCI is not point in time. DFII10 and T10YIE can be missing.
- Survivorship stays in both lists.
- The test shows a link, not a cause. The unexplained share is a residual and not a named cause.
- We did not run any command, read any weekly value, or test any number. No value in this file is a result. The only values are in rule 9.

## 6. Dated records after the freeze

None yet. Quality and Risk adds the freeze commit here. Fundamental Research adds the commit of S1 and S2, the switch week, and the list of regressors that the store has. Each record is dated and written before the first run, or it says that it follows a result (rule 10).
