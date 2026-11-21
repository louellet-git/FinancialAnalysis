# JST test of the market-level DCF: pre-registration

Date: 2026-11-21. Owner: Head of Economics. Follows the US design `analysis/macro/2026-11-21-long-history-design.md` and its findings `analysis/macro/2026-11-21-long-history-findings.md`.

This file fixes the test before any run and before any value of the JST file is read. It is context for the other heads. It is not a model input and not a trade signal. A head that wants a macro input writes an assumption record. The board owns status. No JST data value is in this file.

## 1. Purpose and question

Purpose: an out-of-sample test of the US result. The US findings show that the engine premium and the baseline P1 pass the frozen rule on US data. That is one country and about 12 independent 10-year windows. The owner asked for data "as far back as possible" and called the work "a model calibration/expansion exercise". Other countries are the next out-of-sample data.

The US design (section 6) said that JST is out of the scored set and that each country stays alone. This record replaces that plan for JST. It is a separate test with its own rules. The US design keeps its rules.

Question: does the engine premium forecast the later 10-year real excess return of the stock markets of 15 countries, pooled within country? Yes or no, from the rule in section 5.

## 2. Data and terms

| Item | Value |
|---|---|
| Source | Jordà-Schularick-Taylor Macrohistory Database, release R6, file `JSTdatasetR6.xlsx`, https://www.macrohistory.net/database/ |
| Download | 2026-11-21. SHA-256 prefix c1bb91fe56ea50d4 |
| Local copy | Only at `/home/user/jst-local`. The copy is never in a repo. |
| Licence | CC BY-NC-SA 4.0: non-commercial use |
| Permission | The person answered on 2026-11-21, 14:37 UTC (`RECORD.md` of economics): yes to a local copy with results only in the repo. A live trade that rests on a JST result needs the permission of the JST team first. |
| Cite | Jordà, Schularick and Taylor (2017), "Macrofinancial History and the New Business Cycle Facts", NBER Macroeconomics Annual 2016, vol. 31. For rates of return also: Jordà, Knoll, Kuvshinov, Schularick and Taylor (2019), "The Rate of Return on Everything, 1870-2015", QJE 134(3), 1225-1298. |

What the Head of Economics looked at before this design: the column names, and the coverage (first year, last year and count of non-missing values per country) for the equity, bill, long-rate and CPI columns. No value was read.

Columns used (annual, nominal, local currency): `eq_tr`, `eq_capgain`, `eq_div_rtn`, `bill_rate`, `ltrate`, `cpi`.

Columns not used:
- `eq_dp`: its timing is not confirmed in the R6 documentation.
- Every `*_interp` column: the values are interpolated.

Countries with equity data (15): Australia, Belgium, Denmark, Finland, France, Germany, Italy, Japan, Netherlands, Norway, Portugal, Spain, Sweden, Switzerland, UK. Canada and Ireland have no equity data. They are out. The USA also has data. It is not in the primary test, because it is the in-sample result. It is shown only as a check of JST against the Shiller result. Coverage ends 2020 for all countries.

## 3. Rules (fixed 2026-11-21, before any run)

All rates in formulas are decimals per year. Percent is used only in the text. Year Y means the end of year Y.

**R-1. Inputs.**
- Price index P_Y: the cumulative product of (1 + `eq_capgain`).
- Dividends of year Y: D_Y = `eq_div_rtn`_Y x P_(Y-1).
- Bill return of year Y = `bill_rate`_Y / 100.
- Inflation comes from `cpi`.

**R-2. Point in time.** At the end of year Y, the known values are:
- the dividends of years up to Y-1 (a one-year lag);
- the CPI of year Y-1, `cpi`_(Y-1);
- the long rate `ltrate`_Y / 100;
- the price P_Y.

**R-3. Engine premium.** The engine is the same as record R1 of the US design. It has no country tuning.
- L = 2% real. N = 10.
- g0 = the real growth of the known dividend over the last 10 known years, clamped to [-10%, +20%]. g fades in a line from g0 to L.
- rr = `ltrate`_Y / 100 - ((`cpi`_(Y-1) / `cpi`_(Y-11))^(1/10) - 1).
- Real values use the known CPI.
- The premium is the root of V = the real price, by bisection on [-0.10, 0.30], with r > L.
- Baseline P1 = D_(Y-1) (real) / P_Y (real) + g(Y) - rr. Here g is the end-point real growth of the known dividend from the first year of the country. The window has at least 20 years.

**R-4. Outcome.** X(Y,10) = the annualized real total return of years Y+1 to Y+10, less the annualized real bill return of the same years. Both use the deflator `cpi`_(Y+10) / `cpi`_Y. The outcome is not a predictor, so no lag applies to it.

**R-5. Entry and gaps.**
1. A country-year has a predictor only when 10 known years of D exist (20 for P1).
2. A pair needs every input and all 10 outcome years. There is no fill, no trim and no interpolated column.
3. The out-of-sample forecast of a country starts when that country has 20 complete pairs.
4. Every series ends in 2020. So the last start year is 2020 - 10 = 2010.

**R-6. Primary test.** The headline is the engine premium. P1 gets the same rule.
- Model: the pooled within-country slope b of X on the premium, with country intercepts.
- Statistic: the mean of b over the 10 non-overlapping phases (phase = Y mod 10).
- Inference: a circular block bootstrap of calendar years. The blocks have 10 years. All countries use the same years together, so the cross-country correlation stays. K = 10,000. Seed 20261121. Generator PCG64.
- One-sided p, from centring: p = (1 + #{b* - b_hat >= b_hat}) / (K + 1).
- Out of sample: at year t, the forecast for country i = its expanding mean of X + b(t) x (premium_i(t) - its expanding mean of the premium). b(t) is the pooled within slope from the pairs with s + 10 <= t. The benchmark is the expanding mean of X of country i. OOS R2 = 1 - SSE(model) / SSE(benchmark), pooled over all forecasts.
- Rejection of H0 (the premium has no forecasting power): p < 0.05 AND OOS R2 > 0.
- Engine failure: the engine fails if it does not meet this rule, or if its OOS R2 is below the OOS R2 of P1.

**R-7. Secondary tests.**
- Each country alone, for the engine: the same statistic with 10 phases and the year bootstrap. Holm correction, m = 15, family-wise 0.05, one-sided.
- Descriptive only (no p-value, not in the family): P1 for each country; the 30-year blocks of start years (1883-1912, 1913-1942, 1943-1972, 1973-2010); the US check.

**R-8. Change rule.** A change after the first result needs a new dated record in this file. The record states that it was written after the result. It names the run of record and each superseded run.

## 4. How the rules follow the US design

| Item | US design | This design |
|---|---|---|
| Frequency | monthly | annual |
| Horizon | 120 months | 10 years |
| Phases | 120 | 10 |
| Bootstrap block | 120 months | 10 years |
| Units | one country | 15 countries, pooled within country |
| Lag of D and CPI | step rule of section 5 | one year (R-2) |
| Dividend | Shiller D | `eq_div_rtn` x the price of the year before |

The bootstrap draws calendar years, not country-years. So a war year hits all countries in the same draw.

## 5. Decision rule

The reading of the result is fixed now:
- Pass: the engine meets R-6 and does not fail the engine-failure rule.
- Fail: it does not.
- Mixed: a pass in the primary test with no rejection in the country family (R-7). We then report that the pooled result does not hold in each country. We do not call it a clean pass.

The Holm family is the only scored secondary set. The engine headline is not the best of many tests.

## 6. Limits and what we did not do

- Annual data. The timing of `eq_capgain` and `ltrate` (end of year or average of the year) is not confirmed in the R6 documentation. The one-year lag of D and CPI (R-2) is the protection. It does not remove a timing error of the price or of the long rate.
- Survivorship: the 15 countries are rich countries today. This is selection on success. The sample leaves out the countries that did not reach this level.
- Shared shocks, mostly wars, make the countries dependent. The 15 countries are not 15 independent tests. The year bootstrap keeps this link, but the effective sample is smaller than the count of pairs.
- At most about 13 independent 10-year windows exist for each country (1870 to 2020). The p-values are coarse.
- Hyperinflation years stay in the sample with no special rule. A real value in these years can be very large or very small. We did not choose a rule before the run, to keep the test the same for each country.
- The dividend is built from a yield and a price index. It is not a reported dividend level.
- The US is checked against Shiller only as a description. A match does not prove the JST data.
- We did not read any value, run any command, or test any number. No value in this file is a measured result.
- A live trade that rests on a JST result needs the permission of the JST team first (section 2).

## 7. Work split

- Fundamental Research builds the reader and the scoring in `fa dcf market`, in Rust. The code is new for JST and follows R-1 to R-7. The engine code of the US run does not change.
- Economics wrote this design.
- Quality and Risk freezes the design (commit recorded in section 9) and audits the run. The audit includes a hand recalculation of a sample of country-years.
- The Head of Technology holds the data. The JST file stays at `/home/user/jst-local`. It does not go into the store `louellet-git/fa-data` or any repo.

## 8. Open points

| Point | Owner |
|---|---|
| Freeze of this design (commit and verdict) | Head of Quality and Risk |
| The reader and the scoring for the JST file in `fa dcf market` | Head of Fundamental Research |
| The timing of `eq_capgain` and `ltrate` in the R6 documentation | Head of Economics (ask the JST team, or read the paper; result goes in a dated record) |
| The permission of the JST team before any live trade that rests on a result | the person |
| Whether the results may be used for a decision: this file sets no input | the person |

## 9. Dated records after the freeze

None. The design is not yet frozen. The Head of Quality and Risk adds the freeze commit here. Each later record is dated and is written before any scoring, or it says that it was written after a result (R-8).
