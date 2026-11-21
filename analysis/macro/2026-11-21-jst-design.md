# JST test of the market-level DCF: pre-registration

Date: 2026-11-21. Owner: Head of Economics. Follows the US design `analysis/macro/2026-11-21-long-history-design.md` and its findings `analysis/macro/2026-11-21-long-history-findings.md`.

This file fixes the test before any run and before any JST value is read. It is context for the other heads. It is not a model input and not a trade signal. The board owns status. No JST value is in this file.

## 1. Purpose, question, H0

Purpose: a cross-country replication in the same period of the US result. The countries share wars, crises and booms, so this is not "out of sample in time". This record replaces the plan of the US design (section 6) that JST stays out of the scored set. The US design keeps its rules.

Question: does the engine premium forecast the later 10-year real excess return of the stock markets of 15 countries, pooled within country? Yes or no, by the rule of section 5.

H0: the engine premium has no forecasting power for X (the slope is zero or negative). The statistic and the bootstrap are in R-6.

Effective sample: at most floor((2010 - first index year + 1) / 10) calendar windows of 10 years (about 13), times a small cross-section factor. This gives about 25 to 50 effective country-windows at most. The 15 countries are not 15 independent tests.

## 2. Data and terms

- Source: Jordà-Schularick-Taylor Macrohistory Database, release R6, 2022 (the R6 documentation says the update is "in the year 2022"), file `JSTdatasetR6.xlsx`, https://www.macrohistory.net/database/. Licence CC BY-NC-SA 4.0.
- Download: 2026-11-21. SHA-256 prefix c1bb91fe56ea50d4. Local copy only at `/home/user/jst-local`; never in a repo.
- Permission: the person, 2026-11-21 14:37 UTC (`RECORD.md` of economics): yes to a local copy with results only in the repo. A live trade that rests on a JST result needs the permission of the JST team first.
- Cite: Jordà, Schularick and Taylor (2017), NBER Macroeconomics Annual 2016, vol. 31. Also Jordà, Knoll, Kuvshinov, Schularick and Taylor (2019), QJE 134(3), 1225-1298.

Order of events (2026-11-21, UTC, machine clock):
1. 14:37: the permission.
2. 14:37:49: the ticket commit 04efd60.
3. 14:38:00 and 14:38:01: the download of `JSTdatasetR6.xlsx` and of the R6 returns documentation.
4. 14:39:47: the first design commit ccd862e.

The download came before the design. Before the design, only the column names and the coverage counts (first year, last year, non-missing count per country, for the used columns) were opened. No value was read.

Columns used (annual, nominal, local currency):
- `eq_tr`, `eq_capgain`, `eq_div_rtn`: decimals per year.
- `bill_rate`: decimal per year. If the documentation of the column says percent, divide by 100. The unit check (R-0) decides.
- `ltrate`: percent per year. Divide by 100.
- `cpi`: index.

Not used: `eq_dp` (timing not confirmed) and every `*_interp` column (interpolated values).

Countries (15, those with equity data): Australia, Belgium, Denmark, Finland, France, Germany, Italy, Japan, Netherlands, Norway, Portugal, Spain, Sweden, Switzerland, UK. Canada and Ireland have no equity data. The USA is not in the primary test (it is the in-sample result). It is a descriptive check of JST against Shiller. Coverage ends 2020.

## 3. Rules (fixed 2026-11-21, before any run)

All rates in formulas are decimals per year.

**R-0. Convention check.** It runs before scoring. It reads no outcome X and no premium.
1. For each country-year: |`eq_tr` - (`eq_capgain` + `eq_div_rtn`)| <= 0.0005.
2. Unit check: the median absolute `bill_rate` and the median absolute `eq_tr` are below 1 (decimals).
3. A country that fails check 1 in more than 5% of its years leaves the primary. The list of these countries goes in record R1 (section 9) before any scoring, with the commit of the reader code.

**R-1. Inputs and segments.**
- Price index P_Y: the cumulative product of (1 + `eq_capgain`). A missing year of `eq_capgain` ends a segment. A new segment starts at the next year with a value, with a new base. A ratio or a growth rate uses one segment only.
- Dividend D_Y = `eq_div_rtn`_Y x P_(Y-1).
- Real values: real P_(Y-1) = P_(Y-1) / `cpi`_(Y-1). Each D_k is deflated with the `cpi` of its own year k (as in R1 of the US design). g0 uses the real D of the last 10 known years.

**R-2. Timing (primary: the safe variant).** The documentation does not settle the timing of price, long rate and CPI. So at index year Y every predictor input is dated Y-1 or earlier:
- price P_(Y-1); long rate `ltrate`_(Y-1);
- dividends up to D_(Y-2); CPI up to `cpi`_(Y-2) for the inflation term;
- outcome window: years Y+1 to Y+10. Year Y is skipped on purpose. The extra lag is a deliberate choice.

Sensitivity (pre-registered, not scored): the variant as first written, with inputs at Y and Y-1 (P_Y, `ltrate`_Y, D up to D_(Y-1), `cpi`_(Y-1)) and the outcome Y+1 to Y+10. Reading: a primary pass with a failing sensitivity is reported as it is. A primary fail with a passing sensitivity is not a pass.

**R-3. Engine premium.** The same as record R1 of the US design. No country tuning.
- V = sum over k = 1..N of D_k / (1 + r)^k + D_N (1 + L) / (r - L) / (1 + r)^N, with r = rr + erp. D_k = D_known_real x product over j <= k of (1 + g_j). g_j fades in a line from g0 to L.
- L = 2%. N = 10. g0 is clamped to [-10%, +20%]. D_known_real = real D_(Y-2).
- rr = `ltrate`_(Y-1) / 100 - ((`cpi`_(Y-2) / `cpi`_(Y-12))^(1/10) - 1).
- erp is the root of V = real P_(Y-1), by bisection on [-0.10, 0.30], with r > L.
- If there is no root, or the root is at an endpoint, or r <= L, the pair is dropped. The findings give the count of dropped pairs for each country before the score is read.
- Baseline P1 = real D_(Y-2) / real P_(Y-1) + g - rr. g is the end-point real growth of the dividend over a window of at least 20 years, inside one segment.

**R-4. Outcome.** X(Y,10) = RT - RB. RT is the annualized real total return of years Y+1 to Y+10 (`eq_tr`). RB is the annualized real bill return of the same years (`bill_rate`). Both use the deflator `cpi`_(Y+10) / `cpi`_Y, as written. Limit: `cpi` is an annual index that may be a year average, while prices are year-end. The deflator can be misaligned with the returns. We do not correct this.

**R-5. Entry and gaps.**
1. A predictor needs all its inputs (P_(Y-1), 10 years of D, the CPI years) inside one segment. P1 needs 20 years of D.
2. First index year of a country = the later of (first year of D + 11) and (first year of `cpi` + 12), inside its first segment. The first year of D is the first year of `eq_capgain` + 1. The first index year of the test is the smallest of these. R1 gives the number.
3. An outcome window needs all 10 years. It can cross a segment break only if `eq_tr`, `bill_rate` and `cpi` exist for each year, because the outcome uses returns, not levels.
4. No fill, no trim, no interpolated column.
5. A country starts its out-of-sample forecast with 20 complete pairs. This is a count of pairs, not a span.
6. Series end in 2020, so the last index year is 2010.

**R-6. Primary test.** The headline is the engine premium. P1 has the same rule.

In phase k (k = Y mod 10, calendar years; in a bootstrap pseudo-sample, the position mod 10 in the pseudo calendar):

b_k = sum_i sum_t (x_it - xbar_ik)(y_it - ybar_ik) / sum_i sum_t (x_it - xbar_ik)^2

x is the premium and y is X. The means are within country i and within phase k. So each country weighs by its count of pairs. A country enters a phase only with at least 2 pairs in it. The statistic b_hat is the mean of b_k over the 10 phases. There are no time effects: common world shocks stay in the slope. The slope uses within-country time variation, and this includes the shared shocks.

- Bootstrap: circular blocks of 10 calendar years (one horizon). All countries use the same years. K = 10,000. Seed 20261121. PCG64.
- One-sided p = (1 + #{b* - b_hat >= b_hat}) / (K + 1).
- Out of sample: at year t, the forecast for country i = its expanding mean of X + b(t) x (premium_i(t) - its expanding mean of the premium). The expanding means and b(t) use only pairs with s + 10 <= t. The benchmark is the expanding mean of X of country i. OOS R2 = 1 - SSE(model) / SSE(benchmark), pooled.
- Rejection of H0: p < 0.05 AND OOS R2 > 0. If the size simulation (R-9) gives a rejection rate above 0.10, b_hat must also be above the 95th percentile of the simulated null.
- Engine failure: the engine fails if it does not meet this rule, or if its OOS R2 is below that of P1.

**R-7. Exclusions from history.** Out of the primary: every pair whose predictor years or outcome years touch a span below. The list comes from history, not from the data.

| Country | Years | Reason (history, not from the data) |
|---|---|---|
| Germany | 1914-1924 | War, hyperinflation, currency reform 1923-24 |
| Germany | 1939-1948 | War, exchange controls, currency reform 1948 |
| Japan | 1941-1949 | War; the Tokyo exchange closed 1945-1949 |
| Spain | 1936-1939 | Civil war; the exchange closed |
| Portugal | 1974-1977 | The Lisbon exchange closed after the revolution |
| Italy | 1943-1946 | War, inflation |

R1 gives the count of pairs that the list removes, before scoring. Sensitivity (not scored): the full sample with no list. If the primary passes and the full sample fails, report "depends on the war years". If the primary fails and the full sample passes, it is not a pass.

**R-8. Secondary tests.**
- Each country alone, for the engine: the mean over the 10 phases of the OLS slope of that country (a phase needs at least 2 pairs), the year block bootstrap of that country, one-sided p < 0.05 after Holm. A country enters the family only with at least 8 complete non-overlapping windows in at least one phase. m = the count of such countries, from coverage and not from values, fixed in R1 before scoring.
- Count readout (descriptive; the countries are dependent): the number of countries with a positive slope.
- Leave-one-country-out: 15 runs, descriptive. Report the minimum and maximum slope and how many of the 15 keep p < 0.05.
- P1 is the baseline: the same primary rule and the engine-failure rule.
- Descriptive only: P1 for each country; the US check; blocks of 30 years from the first index year of the test (R-5) to 2010, with the last block to 2010 (its length is stated in the findings); the US engine family (L 1% and 3%, N 5 and 20, erp_gordon), not scored.

**R-9. Fragility and size simulation.**
- Block-20 check: repeat the primary bootstrap with blocks of 20 years. A primary pass with p > 0.10 at block 20 is "fragile".
- Size simulation, before the score is read: 1,000 panels with the same countries, years and gaps. The premium is an AR(1) with rho = 0.9 per year and innovation sd 1 (the slope test is scale-free). The annual excess return has no link to the premium, but its innovations have correlation -0.7 with the premium innovations (a Stambaugh setting). The outcome of a pair is the mean of the next 10 annual returns. The countries share world shocks: the premium innovations and the return innovations each have a common factor, with a correlation of 0.5 between any two countries. Run the full primary procedure (statistic and bootstrap) on each panel, with K = 999 bootstrap draws for each panel. Seed 20261121. Report the rejection rate at 0.05 and the mean b_hat under H0 (the bias).
- If the rejection rate is above 0.10, report the primary p next to the simulated null distribution of b_hat. A pass needs b_hat above its 95th percentile.

**R-10. Change rule.** A change after the first result needs a new dated record. It states that it was written after the result, and names the run of record and each superseded run.

## 4. Disclosure

The engine was made the headline with the US findings known (US findings merged 2026-11-21, PR #129). L = 2%, N = 10, the clamp [-10%, +20%], the horizon of 10 years, the phase method and the block of 10 years are the US values with no change. A pass here is not blind to the US result. Differences from the US design: annual data, 10 phases, 15 countries pooled within country, and the lags of R-2.

## 5. Decision rule

The classes are disjoint.
- Pass: the primary meets R-6 AND at least one country rejects in the Holm family AND the block-20 check is not fragile AND the engine does not fail the engine rule.
- Mixed: the primary meets R-6, the engine does not fail the engine-failure rule, and one of these holds: no country rejects; the block-20 check is fragile. We do not call it a clean pass.
- An engine that meets R-6 but has an OOS R2 below that of P1 fails by the engine-failure rule (as in the US design): Fail.
- Fail: the primary does not meet R-6. A country rejection is then description only: "no pooled support".

The sensitivities (R-2, R-7) change the reading, not the class.

## 6. Limits and what we did not do

- The timing of `eq_capgain`, `ltrate` and `cpi` is not in the R6 documentation. The safe lag removes predictor look-ahead. It costs one year of information. It does not fix the deflator misalignment (R-4).
- Revisions and splices: JST joins sources of different type and date. A later revision can change a value, and a splice can make a step. We did not test for this.
- Survivorship: the 15 countries are rich today.
- Shared shocks make the countries dependent. The effective sample is small (section 1). The p-values are coarse.
- "Pass" needs at least one country rejection in a Holm family of m = 15, with about 13 windows a country. This is hard, so "Mixed" is the likely reading even if the pooled effect is real. "Mixed" is not a failure: it means a pooled result that no single country confirms alone. (Note of the Head of Quality and Risk at the freeze, 31ed025; no rule changed.)
- The Stambaugh bias can raise the slope under H0. The size simulation measures it with a common factor of 0.5; a stronger common factor in the real data can still make it understate the bias.
- The list of R-7 comes from history. A war year not on the list stays in.
- The dividend is a yield times a price index, not a reported dividend level.
- The US check is a description. A match does not prove the JST data.
- We did not read any value, run any command or test any number. No value in this file is a result.

## 7. Work split and open points

- Fundamental Research builds the reader and the scoring in `fa dcf market`, in Rust, following R-0 to R-9. The US engine code does not change. The reader has two tests: a poison test (a value at year Y+1 changes no predictor at Y) and a truncation test (cutting the data after year T changes no predictor at or before T). The reader commit goes in R1.
- Quality and Risk freezes this design and audits the run, with a hand recalculation of a sample of country-years.
- The Head of Technology holds the data. The file stays at `/home/user/jst-local`.
- Open: freeze (Quality and Risk); the timing of the columns in the R6 documentation (Economics: the JST team or the paper; dated record); K of the size simulation (Fundamental Research, in R1); the live-trade permission of the JST team and any use for a decision (the person).

## 9. Dated records after the freeze

None yet. Quality and Risk adds the freeze commit here. Each record is dated and written before any scoring, or it says that it follows a result (R-10).

R1 (to be filled before scoring):
1. The result of the convention check (R-0), by country.
2. The list of excluded countries.
3. m, the size of the Holm family (R-8).
4. The first index year (R-5) and the count of pairs that R-7 removes.
5. The counts of dropped pairs by country (R-3).
6. K of the size simulation.
7. The commit of the reader code and the results of the poison and truncation tests.
