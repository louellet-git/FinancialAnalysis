# Economics view engine: pre-registration (DRAFT 2)

Date: 2026-11-21 (draft 2, after the review of Quality and Risk, cycle 1). Owner: Head of Economics. Ticket: T-20261121-economics-view-engine.

This file is a draft. It is not frozen until Quality and Risk passes it (rule 8). It sets the design before any build or run. It is context for the other heads. It is not a model input and not a trade signal. A head that wants a macro input writes an assumption record. The board owns status. This file has no data value.

## 1. Question and aim

Words of the owner (through Fundamental Research, 2026-11-21): "For attribution, there are numbers in the valuation that are a direct result of the math, but there are some that are implied. The implied components require the view of the economists." and "The economists require their own engine in that regard."

The question: can the economists give, for each date, the equity risk premium (ERP) and the long-run growth that they expect, from data that do not come from the stock prices that the attribution explains?

Terms of `fa dcf attribute` (`rust/src/dcf/auto_attribute.rs`):
- The attribution window is the span from D1 to D2 (`--from D1 --to D2`).
- The plug is the part of the price change that is left after facts, time and rates. In the PREMIUM VIEW it is the change of the implied premium. In the GROWTH VIEW it is the change of the implied growth shift at the model premium.

What a reader decides with the output: how much of the plug of a window to read as a market-wide view, and how much as a company effect.

Version 1 is layer A only. Layer A is the level from the SPF. The engine writes one file. `fa macro view` (planned) writes it. The planned flag `--view-file F` of `fa dcf attribute` (Fundamental Research) reads it. The flag name does not clash with the "common set" of `auto_attribute.rs` (the tickers that have both views).

Layer B, later: a separate measurement file, named "market-implied common part (measurement)", after the premium test (design 55f2e92, R1, R2). It is not the view of the economists, because it is fitted to dS1 from the attributed prices on dVIX. If built: out-of-sample coefficients only, no intercept, its share reported apart, and a sum of weekly fitted changes has no mean reversion.

The design gives a reading of the market. It does not show a cause. It does not give a price target.

## 2. Terms and sources

SPF is the Survey of Professional Forecasters of the Philadelphia Fed.
- Page: https://www.philadelphiafed.org/surveys-and-data/real-time-data-research/survey-of-professional-forecasters
- Documentation PDF (last update 2026-07-28, read 2026-11-21): https://www.philadelphiafed.org/-/media/frbp/assets/surveys-and-data/survey-of-professional-forecasters/spf-documentation.pdf
- Release dates (the PDF names this file; not downloaded): https://www.philadelphiafed.org/-/media/frbp/assets/surveys-and-data/survey-of-professional-forecasters/spf-release-dates.txt

Series. All are medians over the forecasters, in percentage points per year, and all are in first-quarter surveys. The data files are the workbooks of the same page, for example `MedianGrowth.xlsx`. Which workbook holds which series: not checked.

| Series | Definition (documentation PDF) | First survey |
|---|---|---|
| RGDP10 | 10-year annual-average growth of real GDP (chain-weighted; fixed-weighted before 1996) | 1992:Q1 |
| CPI10 | 10-year annual-average headline CPI inflation, from Q4 of the year before the survey to Q4 of the year that is 10 years after the survey year (40 quarters) | 1991:Q4 |
| STOCK10 | "annual-average rate of return to equities (S&P 500)" over 10 years. Arithmetic or geometric, total or price: not checked. The PDF does not say | 1992:Q1 |
| BOND10 | Before 2014:Q1: "annual average over the next 10 years of the return to 10-year Treasury bonds". From 2014:Q1: the "annual average yield on 10-year constant-maturity Treasury bonds" | 1992:Q1 |
| BILL10 | 10-year annual-average return to 3-month Treasury bills | 1992:Q1 |

The 2014 change of the BOND10 question is a possible break (a return, then a yield). It does not touch a window from 2019, because every survey from 2014:Q1 uses the yield question. BOND10 is not in the primary ERP (rule 1).

Other terms:
- S1 is the premium of `fa dcf implied` (percentage points per year): the premium at which the median of value/price over the list is 1. The comparison with ERP_A is at a market beta of 1.
- GSW10 is the 10-year zero yield of the GSW curve (`fa rates`, store copy), the last row on or before the release date. It is a Treasury price, not a stock price. Its compounding unit: not checked.
- The long-run growth L of A-001 (`docs/assumptions/A-001-terminal-growth.md`) is L = min(10-year nominal zero yield, G). G = the longer-run FOMC SEP real growth + the longer-run SEP PCE inflation (4.0% on 2026-09-16).
- The release date is the date of the public release of a survey. Facts of the survey about the deadline: the answer deadline is before the release (mid-quarter), so the release date is the conservative date for the step. The release date of the 2026 Q1 survey, 2026-03-06, is to check against the release calendar.
- Store copy of the SPF medians: Technology keeps it with a hash and a date, because the panel changes and the values can be revised. We commit no series. The SPF terms are open. The owner allowed a private copy.
- `spf-release-dates.txt` (planned, Head of Technology; private store).

## 3. Fixed rules

### Rule 1. Layer A: the level, one value each year

1. For a date t, the survey of t is the latest first-quarter survey with a release date on or before t. The layer is point in time by the release date.
2. Primary ERP form: ERP_A(t) = median STOCK10 of the survey of t minus GSW10 at the release date. Unit: percentage points per year.
3. Forecaster premium (description only, not in the file): median STOCK10 minus median BOND10 of the same survey. It is a difference of two answers, and BOND10 is a return before 2014.
4. Primary growth form: Growth_A(t) = median RGDP10 + median CPI10 of the same survey. Unit: percentage points per year. It is the usual approximation of (1 + real)(1 + inflation) - 1.
5. Use of the growth. RGDP10 + CPI10 maps to L only (rule 3). It never maps to revenue growth. A-001 uses PCE inflation in G, and CPI10 is CPI. The gap between the two is a limit.
6. Between two releases, both values stay flat. The layer is a step function of the release date.
7. Variants, for description only. Each one gives a second value in the findings and does not go into the file:
   - ERP form: STOCK10 - BOND10 (rule 1.3); STOCK10 - BILL10.
   - Growth form: RGDP10 + PCE10 (PCE10 starts in 2007:Q1).
   The primary forms are fixed now. The findings do not change the primary form after a run.

### Rule 2. Rows

For an attribution window D1 to D2 (the engine matches a row on D2):

1. erp_change = ERP_A(D2) - ERP_A(D1), in decimals per year. growth_change = Growth_A(D2) - Growth_A(D1), in decimals per year. The engine divides the SPF points by 100. A change of 0.40 points is written 0.0040.
2. A window is a "release window" if a survey release date falls after D1 and on or before D2.
3. A window with no release has a row with erp_change and growth_change EMPTY (the missing-value marker of the file contract), source "no release". The engine does not write 0, because 0 would claim "no change in the view". The reader of Fundamental Research reports that plug as "not covered by the economics view" (like the "covered part only" rule of `auto_attribute.rs`).
4. If no survey is out at D1, the window has no release row. It has the EMPTY row of rule 2.3.
5. The output gives the count of covered windows and of all windows.
6. Reading of the coverage. The 10-year questions are in Q1 surveys only: about 8 releases from 2019 in about 400 weekly windows (about 2%). A window with a release takes a full year's change in one week. So layer A is mainly a yearly level check. It is not a weekly view.

### Rule 3. Mapping to the attribution (confirmed by the Head of Fundamental Research, 2026-11-21)

1. The common step applies both changes together, in both views (premium view and growth view). For each stock with a value: premium = premium of D1 + erp_change, and G = G + growth_change.
2. growth_change maps to G of A-001 (L = min(10-year zero yield, G)). It does not map to the shift of the growth path of years 1 to 10 (the free variable of the growth view). So L moves only where G binds (G is below the 10-year zero yield).
3. The common part is that one value change (ln units, as the plug). The company part is the rest of the plug. So common + company = plug exactly.
4. growth_change acts on every stock with a value, not only on the growth view set. The plug is split once, so nothing counts twice.
5. For an EMPTY row, or a stock with no value, the whole plug is "not covered by the economics view". It is not split.

### Rule 4. Output

One CSV file, one table. The unit of each column is in the table.

| Column | Meaning | Unit |
|---|---|---|
| date | D2, the end date of the window | YYYY-MM-DD |
| d1 | D1, the start date of the window | YYYY-MM-DD |
| erp_change | change of ERP_A over the window; EMPTY if no release | decimals per year |
| growth_change | change of Growth_A over the window; EMPTY if no release | decimals per year |
| source | allowed values: "spf" and "no release" | text |
| growth_parts | the two growth parts, for example "RGDP10 2.0 + CPI10 2.3" (empty if no release) | SPF points, as published |

### Rule 5. Circularity

1. No input comes from the stock prices of the attribution list. The SPF medians and GSW10 do not use those prices.
2. The forecasters can follow the market. Their opinions are a reading, not a price.
3. GSW10 at the release date is a Treasury price. It moves with the rates, and rates also enter the attribution step "rates". ERP_A is a premium over that yield, so a rate move does not change ERP_A alone.

### Rule 6. Validation and tests (pre-registered, for later)

1. Plug share. Over the covered windows, report the share of the plug that the common part explains. It is a description. It has no p-value.
2. Level check. At each survey date, compare ERP_A with S1. Report both values and the gap. A gap is not an error: the two measures differ in kind.
3. The findings give the counts of windows, release windows and covered windows.
4. A market-implied value (S1, a yield, a path) is the price of the market. It is not a forecast of this repo.
5. Backtest: none applies, because it splits an attribution and moves no value or trade rule.
6. Tests with a fixed answer, written before the first run:
   - Step and release date. The release date, not the survey quarter, sets the step.
   - Poison test. Change a later survey in the SPF store file. No earlier window moves.
   - Truncation test. Cut the file after a survey. Every window up to that survey gives the same row.
   - Unit. A change of 0.40 points gives 0.0040.
   - Split. A synthetic plug of 3.0 points, erp_change 0.0040, a known beta and a known value sensitivity. Check: the common part; common + company = plug exactly; the sign; an EMPTY row gives "not covered".

### Rule 7. Limits

- The survey gives one level a year. Between releases the level is flat.
- Each median has about 40 forecasters. The count changes by year. The panel changes, so a revised median is possible.
- The surveys lag the market. A market move shows only at the next first-quarter release.
- STOCK10 has an unknown return form (arithmetic or geometric, total or price). GSW10 is a zero yield. So ERP_A may mix two units. This is a limit of the level.
- The list of S1 has today's S&P 500 names. This is a survivorship limit of S1, not of the survey.
- A survey median is an opinion. It is not a price and not a forecast of this repo.

### Rule 8. Work split, open points, freeze

1. Economics designs and builds the engine, a Rust command `fa macro view` (planned), in `rust/src/macro_test/` or a sibling module. It reads the store copy in `data/raw`. It does not download.
2. Technology provides the SPF store file and `spf-release-dates.txt` (planned, Head of Technology; private store), after its terms check.
3. Fundamental Research reads the file with `--view-file` (planned). It confirmed rule 3.
4. Quality and Risk freezes the design and audits the build. After the freeze, a change needs a new dated record in this file. The engine does not run on the real SPF file before the freeze.

| Open point | Owner |
|---|---|
| SPF terms ("Copyright 2026. All rights reserved."); the private copy waits for the check | Head of Technology |
| Release date of each first-quarter survey, from the release calendar | Head of Technology |
| Which workbook holds each series; the return form of STOCK10; the compounding of GSW10 | Head of Technology, Head of Economics |
| The CFO Survey (Richmond Fed) is out of the design: no 10-year S&P return, and its terms forbid reproduction | none (closed) |

## 4. Two readings of the result

- A small common part can mean that the market-wide view explains little of the plug. It can also mean that one survey a year cannot follow a weekly move.
- A large common part can mean that a market-wide risk move drives the plug. It can also mean that forecasters follow the market, so the view is not independent of the price.
- A gap between ERP_A and S1 can mean that prices imply a different premium than forecasters expect. It can also mean that S1 has a model error, or that the units of ERP_A differ (rule 7).
- A flat layer A is not "no change in the view". The survey did not publish a new value.

## 5. What we did not do

- We did not run any command, read any data value or download any data file. We read the documentation PDF only, for the definitions of rule 2.
- We did not read the SPF store file. It is not in the store.
- We did not test any number. No value in this file is a result.
- We did not adjust the BOND10 break. It does not touch the primary form.
- We did not give a growth change at weekly frequency.
- We did not set a model input.

## 6. Dated records

Each record is dated. Each record was written before the first run, or it says that it follows a result.

**Freeze.** The design is frozen at commit 3982324 (Head of Quality and Risk, PASS-WITH-NOTES, 2026-11-21, about 17:35 UTC).

**R1. Conditions and notes of the freeze (2026-11-21, about 17:40 UTC, before any build or run).** No SPF value was seen for this record.
1. Conditions before the first run on the real SPF file (not before the build):
   - The Head of Technology checks the SPF terms ("All rights reserved"). Until then, a private copy only, and no series in any repo.
   - The release dates file exists in the private store, with its source.
   - The return form of STOCK10 (arithmetic or geometric, total or price) and the compounding of GSW10 are resolved, or a dated record states them as a limit.
   - The six tests of rule 6.6 exist and pass.
   - `fa macro view` and the flag `--view-file` of Fundamental Research pass their code gates.
2. Notes (no change of a rule):
   - Rule 2.4 is a special case of rule 2.3. Read them as one rule.
   - Rule 5.3 ("a rate move does not change ERP_A alone") is true only between releases, because GSW10 is fixed at the release date. At a release, a rate move and a survey move are both in the step of ERP_A.
   - The mix of two units (rule 7) touches the level check of rule 6.2 more than the changes.
   - The release date 2026-03-06 of the 2026 Q1 survey is checked against the release calendar before the first run.

**R2. Two conditions of R1 are met (2026-11-21, about 17:40 UTC, before any run).** Source: the Head of Technology, 2026-11-21.
1. SPF terms. The Philadelphia Fed site has no terms-of-use page. The footer says "Copyright 2026. All rights reserved. Federal Reserve Bank of Philadelphia". The owner decided on 2026-11-21 (chiefs/louellet-git/technology/RECORD.md): a private copy only, in the private store; only results go in the public repo.
2. Release dates. The file spf-release-dates.txt, linked from the SPF page: https://www.philadelphiafed.org/-/media/FRBP/Assets/Surveys-And-Data/survey-of-professional-forecasters/spf-release-dates.txt, downloaded 2026-11-21, sha256 b5dd7c4c4b9114e455c3cce4aa55f6d90283e10b6fd34d781e46f2533812f390, stored as fa-data macro/spf/spf-release-dates_2026-11-21.txt. Line 186: "2026 Q1 3/2/26** 3/6/26**" (deadline, release). The note "**" says that a government shutdown delayed the survey. So the late release date 2026-03-06 is correct. All 35 Q1 rows (1992 to 2026) have a published release date.
Still open: the return form of STOCK10, the compounding of GSW10, the six tests of rule 6.6, and the code gates.

**R3. Three choices of the build (2026-11-21, about 18:50 UTC, before any run on the real SPF file).** The design was silent on these points. No SPF value was seen for this record.
1. Windows. The command does not choose a grid of windows. The caller gives the windows in a file (`--windows FILE`, lines `d1,date`). Fundamental Research gives the windows of its attribution.
2. Missing inputs. A survey with a "bound" release date (no published date), or with a missing RGDP10, CPI10, STOCK10 or GSW10 value, is left out. The command counts each such survey and prints the count. Today no Q1 row of 1992 to 2026 uses a bound (R2).
3. Compounding (a condition of R1). GSW10 (SVENY10) is a continuously compounded zero yield (`docs/HIDDEN-ASSUMPTIONS.md` #58; the header of the Board file: "Zero-coupon yield,Continuously Compounded"). The SPF documentation does not say whether STOCK10 is arithmetic or geometric, or a total or a price return (draft 2, check D). The command does not convert either value. This is a limit: ERP_A can differ from a like-for-like premium by a compounding term, about 0.23 points at a 7% return (7% simple annual is ln(1.07) = 6.77% continuous). A change between two releases is less affected than the level.
