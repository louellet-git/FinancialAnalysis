# Cycle models: Sahm rule, term-spread probit, financial conditions

Run date: 2026-11-21. Store copy: `data/raw` (store `louellet-git/fa-data`, commit 58e54a2). Role: context for the other heads. It is not a model input, not a forecast of this repo, and not a trade signal.

## Main finding

- NY Fed probit: the probability of a recession in Oct 2027 (12 months ahead) is 12.4% (NY Fed, "Probability of US Recession Predicted by Treasury Spread", `Prob_Rec.pdf`, "Updated 05-Nov-2026", newyorkfed.org/medialibrary/media/research/capital_markets/Prob_Rec.pdf). The input is the Oct 2026 spread of 0.98 pp: the 10-year yield minus the 3-month bill, monthly average. The NY Fed gives a probability and no threshold.
- Our reproduction on FRED `T10Y3M` (10-year constant maturity minus 3-month constant maturity) gives 13.4% for Oct 2026 and 16.2% for Nov 2026 (14 days only). The spread is narrower in Nov: the daily T10Y3M is 0.60 pp on 2026-11-20.
- The Sahm rule is 0.00 pp (trigger 0.50 pp). The NFCI is -0.478 (looser than average). STLFSI4 is below zero. Each model gives a value; none is at a stress level.
- Labor and activity data are strong. Payroll growth is 137 thousand a month (3-month average). Claims are 197,750 (4-week average). GDP grew 3.37% (annualized) in Q3 2026.
- The Baa-Aaa spread is 0.47 pp. In the 1,294 months since 1919-01, 3.9% (50 months) have the same or a lower value (a fact of the data, not a forecast).
- All values are the latest values in the store copy. They are not real-time vintages. Later revisions can change them.

## Data

The files are in `data/raw/macro/fred/` (FRED graph CSV, downloaded 2026-11-21; `SOURCE.md`). "Data date" is the date of the observation, not the run date. Rates and spreads are in percent points (pp), not decimals. The NFCI is the exception: the store has no NFCI file, so its value is a published number (see section 1). The NY Fed figures come from the `Prob_Rec.pdf` named above.
Computed with a script in the scratchpad, from the files only. No `fa` command, no download.

## 1. Published models

| Indicator | Value | Data date | File | Unit | 3 months before | 12 months before |
|---|---|---|---|---|---|---|
| Sahm rule, real-time (published) | 0.00 | 2026-10 | `SAHMREALTIME.csv` | pp | -0.03 (2026-07) | n/a for 2025-10 (no row); 0.23 (2025-09), 0.43 (2025-11) |
| Sahm rule, own calculation from UNRATE | 0.00 | 2026-10 | `UNRATE.csv` | pp | -0.03 (2026-07) | n/a (see below) |
| Term-spread probit, NY Fed, Oct 2026 (10Y minus 3M bill, 0.98 pp) | 12.4% for Oct 2027 | 2026-10 | `Prob_Rec.pdf` | probability, 12 months ahead | n/a | n/a |
| Same, our reproduction, Oct 2026 (T10Y3M monthly mean 0.905 pp) | 13.4% | 2026-10 | `T10Y3M.csv` | probability, 12 months ahead | 14.8% (2026-08; spread 0.808); 15.9% (2026-07; spread 0.733) | 27.9% (2025-10; spread 0.084) |
| Same, Nov 2026, 14 daily values to 2026-11-20 (mean 0.713 pp) | 16.2% | 2026-11-01 to 2026-11-20 | `T10Y3M.csv` | probability | n/a | 26.4% (2025-11; spread 0.157) |
| T10Y3M, last daily value | 0.60 | 2026-11-20 | `T10Y3M.csv` | pp | 0.808 (2026-08 mean) | 0.157 (2025-11 mean) |
| NFCI | -0.478 | 2026-11-13 | published value, Chicago Fed (chicagofed.org/research/data/nfci/about) or FRED (fred.stlouisfed.org/series/NFCI) | index, 0 = average, positive = tighter | not in the store | not in the store |
| STLFSI4 | -0.866 (file -0.8655, rounded to 3 decimals) | 2026-11-13 | `STLFSI4.csv` | index, 0 = average | -0.826 (2026-08-14) | -0.500 (2025-11-14) |

**Sahm rule.** The publisher rule (FRED, C. Sahm): the 3-month mean of the unemployment rate, less the lowest 3-month mean in the prior 12 months. The 12-month low in my calculation uses 9 of the 12 windows, because `UNRATE.csv` has no 2025-10 row. A value of 0.50 pp or more is the trigger. The own calculation from `UNRATE.csv` is 0.00 pp for 2026-10: the mean of Aug to Oct 2026 is 4.133%, and it equals the 12-month low (also 4.133%). It agrees with `SAHMREALTIME` for the latest month. The last trigger months in `SAHMREALTIME` are 2024-07 to 2024-09.

The two series differ in the past, and the reason is vintage. `SAHMREALTIME` uses the unemployment rate as first published. `UNRATE.csv` has the revised rate. For 2024-07 the published value is 0.53 and my value is 0.50; for 2024-09 they are 0.50 and 0.53. Both reach 0.50 in 2024-07. `UNRATE.csv` has no row for 2025-10 (the household survey gap), so my rule skips the means that need that month. This does not change the latest value. It makes 12-month comparisons n/a.

**Term-spread probit.** Formula: P(recession in 12 months) = Phi(-0.5333 - 0.6330 x spread), spread in pp. The coefficients -0.5333 and -0.6330 are in the NY Fed `Prob_Rec.pdf`. For a spread of 0.98 pp the formula gives 12.43%. The PDF gives 12.4158% for Oct 2027.

The NY Fed model uses the 10-year yield minus the 3-month bill, as a monthly average. FRED `T10Y3M` is the 10-year constant maturity minus the 3-month constant maturity. So 13.4% is our reproduction, not the NY Fed figure. The NY Fed figure is the reference.

One gap is open. The NY Fed spread for Oct 2026 is 0.98128 pp. The monthly mean of `T10Y3M` is 0.905 pp (13.4%). The store GS10 minus TB3MS for Oct 2026 is 1.08 pp. So the gap against the NY Fed 0.98 pp is not explained: the averaging or the data can differ. I did not check it.

The Nov 2026 value uses part of a month. At the last daily spread (0.60 pp) the formula gives 18.1%. That is arithmetic only, not a published value.

**Financial conditions.** NFCI of -0.478 means conditions are looser than the average of its history (Chicago Fed). The store has no NFCI file, so I give no change against 3 or 12 months before. STLFSI4 of -0.866 is below zero (below-average stress). It is lower than 12 months before (-0.500) and than 3 months before (-0.826). Both indexes are re-estimated by their publishers, so past values can change.

## 2. Labor and activity (latest values)

| Indicator | Value | Data date | File | Unit | 3 months before | 12 months before |
|---|---|---|---|---|---|---|
| PAYEMS, 3-month average of the monthly change | +137 | 2026-10 (Aug to Oct) | `PAYEMS.csv` | thousand jobs a month | +28 (to 2026-07) | -44.7 (to 2025-10) |
| PAYEMS, level | 159,293 | 2026-10 | `PAYEMS.csv` | thousand | 158,882 | 158,408 |
| UNRATE | 4.1 | 2026-10 | `UNRATE.csv` | percent | 4.1 | n/a (no row 2025-10); 4.5 (2025-11) |
| ICSA, 4-week average | 197,750 | week to 2026-11-14 | `ICSA.csv` | claims | 204,250 (2026-08-15) | 224,750 (2025-11-15) |
| CES0500000003, 12-month change | +3.12 | 2026-10 | `CES0500000003.csv` | percent (level $38.00 an hour) | +3.21 (2026-07) | +3.92 (2025-10) |
| GDPC1, annualized q/q | +3.37 | 2026 Q3 (obs. 2026-07-01) | `GDPC1.csv` | percent | +2.22 (2026 Q2) | +3.88 (2025 Q3) |
| GDPC1, y/y | +2.07 | 2026 Q3 | `GDPC1.csv` | percent | +2.19 (2026 Q2) | +2.60 (2025 Q3) |
| PCEPILFE, 12-month change | +3.12 | 2026-09 | `PCEPILFE.csv` | percent (index 130.902) | +3.04 (2026-06) | +2.82 (2025-09) |
| RSAFS, 12-month change, nominal | +6.82 | 2026-10 | `RSAFS.csv` | percent (level 748,220 million $) | +4.53 (2026-07) | +2.73 (2025-10) |

Notes. The annualized q/q rate is (level / prior level)^4 - 1. The GDPC1 file does not say if the 2026 Q3 value is an advance or a later estimate. The PAYEMS changes come from the latest revised levels. The 3-month PAYEMS average is not a published BLS figure; I computed it.

## 3. Long credit context (Moody's, monthly, from 1919-01)

| Indicator | Value | Data date | File | Unit | 3 months before | 12 months before |
|---|---|---|---|---|---|---|
| Baa yield | 6.59 | 2026-10 | `BAA.csv` | percent | 6.19 (2026-07) | 5.74 (2025-10) |
| Aaa yield | 6.12 | 2026-10 | `AAA.csv` | percent | 5.76 | 5.13 |
| Baa less Aaa | 0.47 | 2026-10 | `BAA.csv`, `AAA.csv` | pp | 0.43 | 0.61 |

Over 1,294 months (1919-01 to 2026-10): 50 months (3.9%) have a spread at or below 0.47 pp; 44 months (3.4%) are strictly below. Six months equal 0.47 pp: 1946-01, 1946-02, 1946-03, 1955-12, 1964-02 and 2026-10. The median is 0.93 pp. The lowest is 0.32 pp (1966-01). The highest is 5.64 pp (1932-05). Since 1983-01, 0.8% of the months are at or below 0.47 pp. This is a statement about the data. The Baa and Aaa classes changed over a century, so the old months are not a clean match.

## What the models say together (facts)

1. Sahm rule 0.00 pp against a 0.50 pp trigger; the unemployment rate (4.1%) is below its 2025-11 value (4.5%).
2. The NY Fed probit is 12.4% (Oct 2027, 12 months ahead). Our reproduction is 13.4% (Oct 2026) and 16.2% (Nov 2026, part month). It was 27.9% a year before (Oct 2025).
3. NFCI and STLFSI4 are below zero. The Baa-Aaa spread is in the lowest 4% of its history.
4. Payroll growth, claims, real GDP and retail sales all improved against 12 months before. Core PCE inflation (3.12%) and wage growth (3.12%) are above 3% a year.

## Reading, and the other reading

- Reading: no model shows stress. The data fit an expansion with strong activity and loose credit.
- Other reading: the low readings can be a lagging view. The Sahm rule needs a rise in unemployment first. The spread (0.60 pp, falling in Nov) moves the probit up. Tight credit spreads can come before a widening, and a low percentile does not say when. Retail sales are nominal, so some of the +6.82% is price. Core PCE inflation above 3% and rising from 2.82% can limit how far rates fall; I do not forecast the effect.

## What is uncertain

- **Revisions.** All values are latest values. Real-time values were different (see 2024-07: 0.53 against 0.50). GDP, payrolls and the NFCI history revise.
- **Missing 2025-10.** `UNRATE.csv` and `SAHMREALTIME.csv` have no row for 2025-10. 12-month comparisons use the nearby months.
- **The probit is a model of the market.** It maps one spread to one probability. The 0.075 pp gap in the Oct 2026 spread (0.905 against 0.98) is not explained.
- **The Sahm rule failed in 2024.** It triggered in 2024-07 and no recession followed (industry note, source Fortune 2024-07-29, a summary read). It is a rule of thumb, not a law.
- **Data lag.** NFCI and STLFSI4 end 2026-11-13; ICSA 2026-11-14; T10Y3M 2026-11-20; BAA and AAA 2026-10; PCEPILFE 2026-09.

## What to watch next

- The Nov 2026 unemployment rate: the Sahm value moves from 0.00 toward 0.50 only if the 3-month mean rises.
- The spread: the full-month mean for Nov 2026 (the probit input).
- The NFCI and the Baa-Aaa spread for a move from the present low levels.
- Revisions to GDP and payrolls, which can change the facts above.
