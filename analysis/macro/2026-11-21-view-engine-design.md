# Economics view engine: pre-registration (DRAFT)

Date: 2026-11-21 (draft 1). Owner: Head of Economics. Ticket: T-20261121-economics-view-engine.

This file is a draft. It is not frozen until Quality and Risk passes it (rule 9). It sets the design before any build or run. It is context for the other heads. It is not a model input and not a trade signal. A head that wants a macro input writes an assumption record. The board owns status. This file holds no data value.

## 1. Question and aim

Words of the owner (through Fundamental Research, 2026-11-21): "For attribution, there are numbers in the valuation that are a direct result of the math, but there are some that are implied. The implied components require the view of the economists." and "The economists require their own engine in that regard."

The question: can the economists give, for each date, the market equity risk premium (ERP) and the long-run growth that they expect, from data that do not come from the stock prices that the attribution explains?

The engine writes one file. `fa dcf attribute --common file --common-file F` of Fundamental Research reads it. The company plug then splits into "common (economics view)" and "company (residual)".

The file has this CSV header: `date,erp_change,growth_change,source`. The values are decimals. For example, 0.01 means one percentage point. Each value is the change over the attribution window. The engine matches a row on the end date D2 of the window.

The design gives a reading of the market. It does not show a cause. It does not give a price target.

## 2. Terms

- SPF is the Survey of Professional Forecasters of the Philadelphia Fed. Source: https://www.philadelphiafed.org/surveys-and-data/real-time-data-research/survey-of-professional-forecasters.
- First-quarter surveys only ask the 10-year questions. They have done so since 1992:Q1. The forecasts are 10-year annual averages, in percentage points.
- RGDP10 is the forecast of real GDP growth. PROD10 is the forecast of productivity growth.
- STOCK10 is the forecast of the return on S&P 500 stocks.
- BOND10 is the forecast of the 10-year constant-maturity Treasury yield.
- BILL10 is the forecast of the 3-month bill return.
- The question of BOND10 changed in 2014:Q1. This is a possible structural break.
- Median means the median of the forecasters, as the Philadelphia Fed publishes it.
- S1 is the premium of `fa dcf implied` (percentage points per year). It is the premium at which the median of value/price over the list is 1.
- The Head of Economics read the facts about SPF in the documentation PDF of the Philadelphia Fed on 2026-11-21.
- The release dates of 2026 are: Q1 on 2026-03-06, Q2 on 2026-05-15, Q3 on 2026-08-14 and Q4 on 2026-11-16 (file `spf-release-dates.txt`).
- The 2026 first-quarter survey is the latest one that is out. Its release date is 2026-03-06.

## 3. Fixed rules

### Rule 1. Layer A: the level, one value each year

1. Let t be a date. Let the survey of t be the latest first-quarter survey that was released on or before t. This makes the layer point in time by the release date.
2. ERP_A(t) = median STOCK10 - median BOND10 of the survey of t. Unit: percentage points per year.
3. Growth_A(t) = median RGDP10 of the survey of t. Unit: percentage points per year.
4. Unit match. The growth engine of Fundamental Research uses real growth L. RGDP10 is real growth. So the units match, and the engine makes no inflation step.
5. ERP_A has a nominal return and a nominal yield. Both hold the same expected inflation. So the difference is a premium over the 10-year Treasury yield.
6. Between two releases, both values stay flat. The layer is a step function of the release date.
7. A second ERP form, for description only: STOCK10 - BILL10. It does not go into the output file.
8. The break of BOND10 in 2014. The findings report the values of 2013 and of 2014 side by side. The design does not adjust for the break. This is a limit.

### Rule 2. Layer B: weekly changes, conditional

1. The test of the premium (`analysis/macro/2026-11-21-premium-macro-design.md`, frozen at commit 55f2e92, record R1) decides if layer B exists.
2. Layer B exists if and only if that test gives a "yes".
3. If the test gives a "yes", the weekly ERP change of layer B is the fitted macro part of model M1. The macro part is the coefficient of dVIX times dVIX plus the coefficient of dNFCI times dNFCI.
4. The fit uses only data from before the week. The window expands with time.
5. If the test gives "no share shown", layer B is zero. The view then changes only at survey releases.
6. Growth has no layer B. No weekly series in this design holds a long-run growth view.

### Rule 3. Output

For an attribution window D1 to D2:

1. erp_change = ERP(D2) - ERP(D1), in decimals.
2. growth_change = Growth(D2) - Growth(D1), in decimals.
3. ERP(t) = ERP_A(t) plus the sum of the layer B changes since the last survey release.
4. If no survey is out at D1, the row for that window is absent. The engine does not fill it.
5. The column source is "spf" when layer B is zero. It is "spf+macro" when layer B is on.
6. Most windows contain no survey release. In those windows, erp_change comes from layer B only, or it is zero. The findings say how many windows have a release.

### Rule 4. Circularity

1. No input comes from the stock prices of the attribution list. SPF forecasts, NFCI and the Treasury curves do not hold those prices.
2. VIX comes from S&P 500 option prices. It is a market price of index options, not of the stocks. It moves with the stocks.
3. So layer B can hold a part of the price move that it should explain. This is a limit.
4. Layer B exists only if the premium test passes. That test controls for the mechanical return with the M0 model.
5. A "yes" does not remove the limit. It shows that the macro block adds to the mechanical model out of sample.
6. The survey layer has no link to the stock prices of the list, except through the opinions of the forecasters. Those opinions can follow the market.

### Rule 5. Validation (pre-registered, for later)

1. Plug share. Over the windows of the premium test sample, report the share of the plug of `fa dcf attribute` that the common part explains. Fundamental Research gives the plug. The engine gives the common part. The share is a description. It has no p-value.
2. Level check. At each survey date, compare ERP_A with the median-based implied premium S1 of Fundamental Research. Report both values and their gap. This is a description. A gap is not an error: the two measures differ in kind.
3. A reading of a gap. A gap can mean that the forecasters expect a different return than the prices imply. It can also mean that the model premium holds a growth or cash-flow error of the engine.
4. The findings give the counts of windows, of survey dates and of dropped weeks.
5. A market-implied path is the price of the market. It is not a forecast of this repo. This holds for the VIX and for S1.

### Rule 6. Limits

- The survey gives one level a year. Between releases the level is flat.
- Each median holds about 40 forecasters. The count changes by year.
- The surveys lag the market. A move in the market shows in the layer A level only at the next first-quarter release.
- The BOND10 question changed in 2014:Q1 (rule 1).
- The list of S1 holds today's S&P 500 names. This is a survivorship limit of S1. It is not a limit of the survey.
- A survey median is an opinion. It is not a market price and not a forecast of this repo.
- STOCK10 is a forecast of a return, not of a premium. The premium needs the BOND10 step, and the two answers can differ in their meaning.

### Rule 7. Work split

1. Economics designs and builds the engine. The engine is a Rust command, for example `fa macro view --as-of D` and `fa macro view --window D1 D2`. Its code goes in `rust/src/macro_test/` or in a sibling module.
2. Technology provides the SPF file. The file holds the first-quarter 10-year medians, with the release date of each survey. Technology gives it after its terms check.
3. Fundamental Research reads the output file with `--common file`.
4. Quality and Risk freezes the design and audits the build.
5. The engine does not download. It reads the store copy in `data/raw`.
6. The build has a test with a fixed answer before the first run. The test checks the step rule, the release date rule and the unit.

### Rule 8. Open points

| Point | Owner |
|---|---|
| The SPF terms. The page says "Copyright 2026. All rights reserved." The store copy waits for the terms check. | Head of Technology |
| The release date of each first-quarter survey before 2026 (the release calendar) | Head of Technology |
| The code of the command | Head of Economics |
| The CFO Survey of the Richmond Fed has no 10-year S&P return on its pages. Its site terms forbid reproduction for private or commercial gain. It is out of the design. | none (closed) |

### Rule 9. Change and freeze

1. Quality and Risk freezes this design. Until then, the Head of Economics can change this draft.
2. After the freeze, a change needs a new dated record in this file.
3. The record says if it follows a result. It names the run of record.
4. The engine does not run on the real SPF file before the freeze.

## 4. Two readings of the result

Each result has a second reading. The findings give both.

- A small common part can mean that the market-wide view explains little of the plug. It can also mean that the engine is too coarse: one survey a year cannot follow a weekly move.
- A large common part can mean that a market-wide risk move drives the plug. It can also mean that layer B holds the index move that it should explain (rule 4).
- A gap between ERP_A and S1 can mean that the prices imply a different premium than the forecasters expect. It can also mean that S1 holds a model error.
- A flat layer A is not "no change in the view". It means that the survey did not publish a new value.

## 5. What we did not do

- We did not run any command, read any data value or download any file.
- We did not read the SPF file. It is not in the store.
- We did not test any number. No value in this file is a result.
- We did not use the CFO Survey, for the reason in rule 8.
- We did not adjust the BOND10 break.
- We did not give a growth change at weekly frequency.
- We did not set a model input. This brief is context. A head that wants these values as an input writes an assumption record.

## 6. Dated records

No record yet. The first record follows the freeze.
