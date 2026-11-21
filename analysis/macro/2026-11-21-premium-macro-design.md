# Implied premium and macro risk measures: pre-registration

Date: 2026-11-21 (version 3, after review). Owner: Head of Economics. Ticket: T-20261121-economics-premium-macro.

This file sets the test before any run. It is context for the other heads. It is not a model input and not a trade signal. A head that wants a macro input writes an assumption record. The board owns status. No weekly series value is in this file, except the disclosure in rule 9.

The Head of Economics drafted all rules below on 2026-11-21. Quality and Risk freezes them (rule 12).

## 1. Question and aim

Question of the person (from Fundamental Research, 2026-11-21): "Can we break down higher implied risk premium? ... understanding it/quantifying it better."

Our part is the market level. The question: do the macro risk measures explain more of the weekly change of the market-implied premium, out of sample, than a mechanical base model? The answer is yes or no (rule 5), with two numbers.

A person uses the answer to decide how to read a weekly premium move. The move is either a market-wide risk move, with the macro measures as evidence, or only a residual.

The design is a statistical link. It does not show a cause. No result is read as a cause. Stock-level work belongs to Fundamental Research and is not in this file.

## 2. Terms

- S1 is the premium of `fa dcf implied`. It is the premium at which the median of value/price over the list is 1 (`rust/src/dcf/auto_implied.rs`, lines 14 and 494). Unit: percentage points per year.
- S1 sets the median to 1 by design. So a large part of the change of S1 is mechanical (rule 5).
- The list is the S&P 500 list (primary), `rust/mapcheck/universe/sp500.txt`. The S&P 100 list is secondary. Fundamental Research names its file (open point).
- Both lists are today's lists, so survivorship stays (rule 8).
- A week ends on a Friday. The anchor of a week is the last equity trading day on or before that Friday. This is the S1 price date.
- Labels of `fa dcf attribute` (`rust/src/dcf/auto_attribute.rs`, lines 57-70): facts, time, rates, plug, residual.
- "Plug" names only the part of the bridge. For the level, this file writes "premium" or "S1".
- "No model" means "residual".
- "The total line of the bridge" is the change of the whole list, value-weighted.

## 3. Fixed rules

### Rule 1. Series

One value each Friday, from 2018-01-05.

| Id | Definition | Command |
|---|---|---|
| S1 | The level of the premium that the S&P 500 list implies | `fa dcf implied --tickers-file <list> --price-date <Friday>` |
| S2 | The weekly plug part (points) of the value-weighted bridge. The rates part and the residual part stay beside it | `fa dcf attribute --from <Friday-1> --to <Friday>` |

The S&P 100 list gives a second S1 and a second S2. Steps of S1 at earnings dates are not an effect of the macro measures. These steps come from company filings, not from the macro measures. Facts change on those dates.

### Rule 2. Outcome

y = S1(w) - S1(w-1), in percentage points, S&P 500 list. S2 and the S&P 100 series are secondary descriptions.

### Rule 3. Sample

- The sample starts on 2019-01-04 and ends on the last Friday with all series.
- 2018 is excluded. The beta is 1 for every name until about 2018-11 (Fundamental Research).
- Training uses the first 104 kept weeks from 2019-01-04. A kept week is a week that is not dropped (rule 4, data rules).
- The first out-of-sample (OOS) week is the 105th kept week, about 2021-01.
- March 2020 is in training only. The OOS window has about 310 weeks.
- The findings give the count of kept weeks after the drops.
- The curve of S1 is GSW for the history and the par fit for the last weeks. GSW lags about one week.
- Fundamental Research states the curve and its date for each Friday. It also states the switch week from GSW to the par fit.
- The switch week drops from all models. The findings name it.

### Rule 4. Regressors, models and data rules

Each change is a weekly change on the anchor of the week.

| Name | Definition | Source |
|---|---|---|
| dr10_w | Change of the nominal 10-year zero yield (percentage points), from the curve file and the date that S1 used on that anchor | `data/raw/rates/gsw_feds200628.csv` (GSW; https://www.federalreserve.gov/data/nominal-yield-curve.htm); par fit for the last weeks |
| ret_w | The median of the weekly price returns of the list names, on the S1 price dates, from the prices that S1 uses | Fundamental Research gives this series with S1 |
| dVIX | Change of the Cboe VIX close (index points) | `data/raw/vix/VIX_History.csv` |
| dBAA10Y | Change of BAA10Y, daily (percentage points) | `data/raw/bonds/fred_BAA10Y.csv`; https://fred.stlouisfed.org/series/BAA10Y |
| dNFCI | Change of the Chicago Fed NFCI (index points), weekly | `data/raw/macro/fred/NFCI.csv`; https://fred.stlouisfed.org/series/NFCI |

The store root is `data/raw/`. The FRED terms of BAA10Y and NFCI: see `data/raw/macro/fred/SOURCE.md` and `data/raw/bonds/` (store). The auditor checks `NFCI.csv` at the run.

Why ret_w: S1 makes the median of value/price equal to 1. So the mechanical price term of S1 is the median return of the list. The SPY return is a description only. It is not a regressor.

Models:

- M0 (mechanical): dS1 on an intercept, ret_w, ret_w squared and dr10_w. The square is in M0 because VIX is not linear in the return. A rise of VIX in falls must not win by that alone. The 2-year zero change is not added.
- M1 (test): M0 plus dVIX, dBAA10Y and dNFCI.

All models have an intercept.

Descriptive block next to M0 (not in the test):

- dREAL10 (FRED DFII10, 10-year TIPS yield; https://fred.stlouisfed.org/series/DFII10) and dBEI10 (FRED T10YIE, 10-year breakeven). The curve of S1 partly holds both, so they are not inputs to the test.
- Both series are in the store since commit a30e47c: `data/raw/macro/fred/DFII10.csv` and `data/raw/macro/fred/T10YIE.csv`.
- dVRP is not used. It moves closely with dVIX by construction.

NFCI: the Chicago Fed publishes NFCI weekly, on Wednesday (https://www.chicagofed.org/research/data/nfci/about). The value for the week to Friday F comes out on the next Wednesday. It is a weekly average. This is acceptable for a same-week break-down. It is not a forecast. NFCI is revised, and we use the latest values, not vintages. This is a limit.

Data rule for the "." days (FRED files use "." for no value):

1. The anchor of each week is the last equity trading day on or before the Friday.
2. For VIX, BAA10Y and the prices, take the last value that is not "." on or before the anchor. These three series follow the same steps.
3. A "." is missing. It is never 0.
4. For these three series, a value older than 3 calendar days gives no value.
5. For NFCI, use the value dated on the Friday of the week (its weekly date). The 3-day limit does not apply to NFCI.
6. If week w has no value for a series, the changes of weeks w and w+1 both drop. The week w is then not kept for the models.
7. A change never spans two weeks.
8. The expanding window and the Newey-West variance use the kept weeks in time order.
9. The findings give the counts of dropped weeks, the count of kept weeks and the counts of carried values.
10. The loader gets a test with a fixed answer, before the run.

T10YIE has the note "Copyrighted, citation required". The repo commits results only: R2, coefficients, p and counts. It never commits the weekly series, a table of it, or a plot. Cite as "FRED, series T10YIE, accessed <date>".

### Rule 5. Statistic and inference

There is one inferential test: Clark-West, M1 against M0. The test is one-sided. All other outputs are descriptions.

1. Fit both models by expanding-window OLS on the kept weeks. The first fit uses the first 104 kept weeks. Each later week uses only earlier kept weeks.
2. OOS R2 of a model = 1 - SSE(model) / SSE(zero change).
3. Report the OOS R2 of M0. Label it "mechanical".
4. Report the gain of M1 over M0: 1 - SSE(M1) / SSE(M0).
5. The primary p is the Clark-West p-value, with a Newey-West variance of lag 8. T is the count of OOS weeks. For T of about 310, T^(1/3) is about 7. Round up to 8.
6. Placebo. Shift only dVIX, dBAA10Y and dNFCI, together as one block, circularly in time. Keep y, ret_w, ret_w squared and dr10_w in place, so M0 stays aligned. Use every shift from 13 to 52 weeks, in both directions (80 runs). Apply the same "yes" rule to each run.
7. The real "yes" stands only if at most 5% of the 80 placebo runs give a "yes" (at most 4 runs). If more runs give a "yes", the answer is "no share shown".
8. The circular block bootstrap is not the primary p.

The "yes" rule: the answer is "yes" only if the Clark-West p is below 0.05 AND the gain is at least 0.05 (5%). If the rule fails, the answer is "no share shown". It is not "zero share".

The effect size is part of the answer. Newey-West t-values of single coefficients are descriptions. We do not read them as tests.

### Rule 6. Description (no test, no p-value)

1. The in-sample R2 and the coefficients of M0 and M1, with Newey-West errors (lag 8).
2. The same for S2 (points) and for the S&P 100.
3. The R2 inside the stress windows of `analysis/macro/2026-11-21-stress-windows.md` (2019 onward), against the other weeks.
4. A variance split of S2 into its rates part and its residual part.
5. The M0 results with dREAL10 and dBEI10 added (descriptive block).
6. The share of the total SSE that comes from the 5 weeks with the largest errors. Heavy tails can drive the fit.
7. The SPY weekly return next to ret_w.

A description is not a result. It does not change the answer of rule 5.

### Rule 7. Checks before the run

1. Identity check. The r10 value is read from the same curve file and the same date that S1 used on that anchor. It comes from the output of Fundamental Research. Economics does not derive it again. Report the R2 of M0, in sample and OOS, before any primary result is read.
2. Sanity check of the bridge. The rates part of S2 must move with the weekly change of the 10-year zero yield. The correlation must be below -0.5. A rise in the yield lowers the value. If the check fails, the bridge has a defect, and the test stops. Fundamental Research finds the defect.
3. Poison test and truncation test of S1, on 3 dates: one in 2019-2020, one in 2022 and one in 2025. Fundamental Research runs both tests. The skill `point-in-time-test` (`.claude/skills/point-in-time-test/SKILL.md`) gives the method.
   - The poison test puts a false value in data that is dated after the test date. S1 must not change.
   - The truncation test cuts all data after the test date. S1 must stay the same.
4. No primary result is read before all checks pass.

### Rule 8. Traps

- The premium is a residual. A growth or cash-flow error of the model enters it. Chen, Da and Zhao (2013), RFS 26(4).
- The VIX mixes risk and risk aversion. Bekaert, Hoerova and Lo Duca (2013), JME 60(7). A link with dVIX does not say which one moved.
- The excess bond premium (Gilchrist and Zakrajsek 2012, AER 102(4)) is monthly and revised. It is not in a weekly test.
- Most premium predictors fail out of sample. Goyal and Welch (2008), RFS 21(4). This is the reason for the OOS rule.
- Survivorship: the lists are today's names.
- NFCI has revisions and a publication lag (rule 4).
- The regressors share information. Coefficients can be unstable. The design reports the gain of the set, not a share for each regressor.

### Rule 9. Disclosure

Before this design, the Head of Economics saw two outputs. The first is the split of 2026-10-06 to 2026-11-19. The second is the attribution of 2026-11-13 to 2026-11-20. So the window 2026-10-06 to 2026-11-20 is "seen".

The week 2026-11-13 to 2026-11-20 (from Fundamental Research) has these values. The total is the percent price change of the S&P 100 list: -9.40%. The parts are in points of percent: rates +4.93, plug -10.71, growth (facts) -1.68 and residual -2.03.

The parts sum to 4.93 - 10.71 - 1.68 - 2.03 = -9.49 points. The total is -9.40%. The total minus the parts is +0.09 points. This difference is not rounding. The findings give its source (the time part or another line of the bridge) from the bridge output.

Pre-declared sensitivity run: the same procedure without the seen weeks (2026-10-06 to 2026-11-20). The answer of rule 5 uses all weeks. The sensitivity run is a description. If the sensitivity run changes the answer, the findings say "the answer depends on the seen weeks".

### Rule 10. Change rule

A change after the first result needs a new dated record in this file. The record says that it follows a result. It names the run of record and each superseded run.

### Rule 11. Work split

- Fundamental Research produces S1, S2 and ret_w, after the merge of the Technology fix `claude/pit-supplement-fill`. It gives the commit.
- Economics runs the regressions. The regression program and its command are an open point. The owners are Fundamental Research and Economics. They name both before the run.
- Quality and Risk freezes this design and audits the run.

### Rule 12. Open points

| Point | Owner |
|---|---|
| The name and owner of the regression program and command | Fundamental Research and Economics |
| The curve and date of S1 for each Friday; the switch week | Fundamental Research |
| The series ret_w, with S1 | Fundamental Research |
| The file of the S&P 100 list | Fundamental Research |
| The poison and truncation tests of S1 | Fundamental Research |
| The check of `NFCI.csv` at the run | Auditor |
| The freeze of this design | Quality and Risk |

## 4. Two readings of the result

Each result has a second reading. The findings give both.

- A gain of M1 over M0 can mean that the premium moves with risk aversion. It can also mean that the model price reacts to the same rates and spread data that the regressors hold. The check in rule 7 covers only the rates part.
- A small gain can mean that the premium moves for other reasons. It can also mean that the weekly noise of the model premium hides a real link. The test cannot tell these two apart.
- A large M0 share means that S1 moves with the price and the yield by design. It is not a market view.
- A market-implied series (the VIX, T10YIE) is the price of the market. It is not a forecast of this repo.

## 5. Limits and what we did not do

- The OOS window has about 310 weeks. This is short.
- A few large episodes can drive the fit. Rule 6 reports the top 5 weeks.
- The premium is a model quantity. It changes with the engine and the list. A change of the engine commit needs a new run.
- The weekly step of S1 has noise from the model (the median of value/price over a list).
- NFCI is not point in time.
- Survivorship stays in both lists.
- The window 2026-10-06 to 2026-11-20 is seen (rule 9).
- We did not run any command, read any weekly value, or test any number. No value in this file is a result. The only values are in rule 9.

## 6. Dated records after the freeze

Each record is dated. Each record was written before the first run, or it says that it follows a result (rule 10).

**Freeze.** The design is frozen at commit 55f2e92 (Head of Quality and Risk, PASS-WITH-NOTES, 2026-11-21, about 15:42 UTC).

**R1. BAA10Y is excluded (2026-11-21, about 16:20 UTC, before any run).** The Moody's Baa series (FRED BAA10Y) has no lawful daily source for the store: the FRED terms prohibit storage, and Moody's publishes no free daily series (Head of Technology, 2026-11-21). A monthly HQM spread does not fit a weekly test. So M1 = M0 + dVIX + dNFCI, and the placebo block is (dVIX, dNFCI). No other rule changes. No value of S1, of y or of a regressor was seen for this choice. The terms of the other inputs (Head of Technology, 2026-11-21): VIX comes from Cboe and stays in the private store; NFCI comes from the Chicago Fed, and its terms are read before the run (open, owner Head of Technology); DFII10 and T10YIE come from the Treasury curves (public domain). A limit that follows from R1: the macro block has no credit spread, so a "no share shown" says nothing about credit risk.

**R2. dNFCI is excluded (2026-11-21, about 17:15 UTC, before any run).** The legal notice of the Chicago Fed (chicagofed.org/utilities/legal-notices, read by the Head of Technology on 2026-11-21) does not permit a copy of its content in the store, except with written permission. The FRED copy of NFCI has the same source. So the store keeps no NFCI file. The decision: M1 = M0 + dVIX, and the placebo block is dVIX only. The alternative, a request for written permission to the Chicago Fed, was not chosen: it is an outward action of the owner, and it delays the test. No other rule changes. No value of S1, of y or of a regressor was seen for this choice. A limit that follows from R2: the macro block has no financial-conditions index and no credit spread. A "no share shown" says nothing about those two risks. Rules 4, 5.6, 8 and 12 and section 5 still name NFCI; R2 replaces those parts.

Fundamental Research adds its record (the commit of S1, S2 and ret_w, the curve dates, the switch week) before the first run.
