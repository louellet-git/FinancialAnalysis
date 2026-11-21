# Model: cost of debt by rating

Updated: 2026-11-21. Commit: a66a70d (step 3), fix commit to follow.

## Purpose
Give the market cost of debt of a credit rating at a date, and the `kd_spread` that the
DCF driver engine needs (`kd = rf + kd_spread`). A person uses it with
`fa verdict --rf-date D --rating R`, or reads it with `fa bonds`.

## Inputs
| Input | Source (variable, file or command) | Note |
|---|---|---|
| HQM 10-year spot yield | Treasury HQM files, `fa bonds hqm get` (`rust/src/hqm.rs`) | Monthly average. Percent in the file, decimal in the code. Used from the publication day of the month |
| UST 10Y par yield, daily | Treasury files (`fa rates`) | The close of the date gives `rf`; the monthly mean gives the basis |
| Step of a rating row | `DEFAULT_SPREAD` in `rust/src/bonds.rs` (Damodaran, January 2026) | Fixed table, A-200 |
| Rating | `--rating` (S&P/Fitch `A-`, Moody's `A3`) | Each notch with a table row has its own step; a notch with no row takes its letter grade |

## Method (short)
1. `rating_bucket` maps the rating to a row of the table (AAA ... C; D is refused).
2. `step = default spread of the row - default spread of the reference bucket A`.
3. `HQM month` = the latest month that is published on or before the date (the first weekday after
   the 5th of the next month) and ends at most 70 days before it. An older month is an error.
4. `basis = HQM 10Y (month) - mean UST 10Y (the same month)`.
5. `cost of debt = UST 10Y (date) + basis + step`. `kd_spread = basis + step`.
   So `rf + kd_spread` is the cost of debt again, by construction.
6. Std, bounds and correlations of the driver stay as in the model file.

Units: decimals (0.0602 = 6.02%). The HQM rate is a spot rate (zero rate); we assume semi-annual
compounding (trap #65, not checked in the full Treasury paper). The UST 10Y is a par yield; the
spot-against-par basis is about 10 bp at 10 years (a review estimate, not measured by us). The
Damodaran spreads are over the par T-bond.

## Outputs
`fa bonds`: a step, a cost of debt and a spread over the 10Y for each row, and the HQM curve.
`fa bonds step|yield|over10y --rating R`. `fa verdict ... --rating R`: the `credit` block (HQM
spot, UST mean, basis, step, `kd_spread`, pre-tax and after-tax cost of debt) and the value range.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-200 | Step table | AAA -0.38, AA -0.23, A+ -0.08, A 0, A- +0.11, BBB +0.33, BB+ +0.60, BB +1.06, B +2.43, CCC +8.07 (points) | Damodaran default spreads, January 2026, less A |
| A-201 | Reference bucket | A | HQM 10Y minus UST 10Y is 0.88 over 36 months; A is 0.78 |
| A-202 | Maturity | 10 years | Same as the `rf` of the engine |
| A-203 | Dates and basis | Month used from the first weekday after the 5th; cap 70 days; basis inside one month | No look-ahead; average against average |

## Limits
- Limit of use. Investment grade (AAA to BBB-) is supported. BB and B are indicative: the table
  spread is lower than the market spread in stress. CCC, CC and C are not for use: the table is
  not a yield that a person can buy.
- The steps are fixed until the next January table. In a credit stress the real spreads move more.
- The HQM curve is a monthly average. A month is used from the first weekday after the 5th of the next
  month; the evidence for this lag is one observation (A-203). A bank holiday is not counted.
- The cost of debt follows the UST close of the date, and the basis is fixed inside a month.
- The ICE BofA and Moody's series are not used (their terms do not permit a stored copy). The old
  output of `oas`, `moodys` and `fa bonds check` left with them.
- The model gives a level at a date. It does not give a forecast and it does not give a trade.
- No valuation model of the repo reads this cost of debt without `--rating`: each model has a
  fixed `kd_spread`. The IBM headline `fa dcf drivers IBM` does not change.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/bonds.rs`, `rust/src/verdict_rates.rs`, `rust/src/hqm.rs` |
| Model or parameter file | The `kd_spread` driver of `analysis/<TICKER>/driver_model.toml` is replaced only by `--rating` |
| Command | `fa bonds hqm get`, `fa bonds`, `fa verdict IBM --price 228.53 --rf-date today --rating BBB` |
| Tests | `rust/src/bonds.rs` (steps, buckets, ids), `rust/src/rates_tests.rs` (cost of debt, spread, stale month, verdict context) |

Backtest: none applies as a forecast test, because the value is a market input at a date,
not a forecast. Two tests apply instead. (1) Cut-date test (no look-ahead): a later HQM month
changes nothing at an earlier date (`bonds::tests`, poison month 99%; and a fixture test at
2026-10-15 that gives September). (2) End-to-end check of the cost of debt against a second source:
the kd of A at the 15th of each month from 2023-12 to 2026-11 (36 dates, HQM months 2023-11 to
2026-10) minus the ICE BofA A index effective yield of the same date (read from a local cache,
no ICE number is written here): the kd of A is a little above the index on average, within
about a third of a point on each date. Part of the gap is the duration of the index (about 6 to 7
years) against 10 years. The step table alone agrees
with ICE for AAA to BBB within about 0.1 points and is lower for high yield (A-200).

## Review status
Not reviewed by the Fundamental head. The QA gate (cycle 1, REVISE) and the Quant review
(AGREE WITH NOTES) found the look-ahead, the monthly-average bias and the notches; this version
corrects them. The Fundamental head examines the steps before a valuation report uses `--rating`.
