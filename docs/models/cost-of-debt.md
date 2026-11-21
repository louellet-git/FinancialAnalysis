# Model: cost of debt by rating

Updated: 2026-11-21. Commit: not yet made.

## Purpose
Give the market cost of debt of a credit rating at a date, and the `kd_spread` that the
DCF driver engine needs (`kd = rf + kd_spread`). A person uses it with
`fa verdict --rf-date D --rating R`, or reads it with `fa bonds`.

## Inputs
| Input | Source (variable, file or command) | Note |
|---|---|---|
| HQM 10-year spot yield | Treasury HQM files, `fa bonds hqm get` (`rust/src/hqm.rs`) | Monthly average, percent in the file, decimal in the code. Blend of AAA, AA and A bonds |
| Step of a rating bucket | `DEFAULT_SPREAD` in `rust/src/bonds.rs` (Damodaran, January 2026) | Fixed table, A-200 |
| Treasury 10Y par yield of the date | `fa rates` (Treasury files) | Gives `rf` and the spread form |
| Rating | `--rating` (S&P/Fitch `A-`, Moody's `A3`) | The notch is dropped: `A-` is bucket `A` |

## Method (short)
1. `rating_bucket` maps the rating to AAA, AA, A, BBB, BB, B or CCC (CC and C go to CCC).
2. `step = default spread of the bucket - default spread of the reference bucket A`.
3. `cost of debt = HQM 10Y spot (latest month on or before D, at most 70 days old) + step`.
4. `kd_spread = cost of debt - UST 10Y of D`. The driver `kd_spread` mean takes this value.
   Then `rf + kd_spread` is the cost of debt again, by construction.
5. Std, bounds and correlations of the driver stay as in the model file.

Units: decimals (0.0602 = 6.02%). Day count and compounding: the HQM rate is a spot rate
(zero rate); we assume semi-annual compounding (trap #65, not checked in the full Treasury
paper). The UST 10Y is a par yield of one day.

## Outputs
`fa bonds`: a step, a cost of debt and a spread over the 10Y for each bucket, and the HQM
curve. `fa bonds step|yield|over10y --rating R`. `fa verdict ... --rating R`: the
`credit` block (HQM spot, step, `kd_spread`, pre-tax and after-tax cost of debt) and the
value range with the new `kd_spread`.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-200 | Step table | AAA -0.38, AA -0.23, A 0, BBB +0.33, BB +1.06, B +2.43, CCC +8.07 (points) | Damodaran default spreads, January 2026, less A |
| A-201 | Reference bucket | A | HQM blend is treated as A |
| A-202 | Maturity | 10 years | Same as the `rf` of the engine |
| A-203 | Dates | HQM month at most 70 days old; Treasury of the date | Level from HQM, spread from the Treasury of the date |

## Limits
- The steps are one number for each letter grade. They do not change with the date until
  the next January. In a credit stress the real spreads move more than the table.
- The HQM curve is a monthly average and is published after the month end.
- The spread has a small basis: a spot yield minus a par yield.
- The CCC step is a default spread of a table, not a yield that a person can buy.
- The ICE BofA and Moody's series are not used (their terms do not permit a stored copy).
  The old output of `oas`, `moodys` and `fa bonds check` left with them.
- The model gives a level at a date. It does not give a forecast and it does not give a trade.
- No valuation model of the repo reads this cost of debt without `--rating`: each model
  has a fixed `kd_spread`. The IBM headline `fa dcf drivers IBM` does not change.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/bonds.rs`, `rust/src/verdict_rates.rs`, `rust/src/hqm.rs` |
| Model or parameter file | The `kd_spread` driver of `analysis/<TICKER>/driver_model.toml` is replaced only by `--rating` |
| Command | `fa bonds hqm get`, `fa bonds`, `fa verdict IBM --price 228.53 --rf-date today --rating BBB` |
| Tests | `rust/src/bonds.rs` (steps, buckets, ids), `rust/src/rates_tests.rs` (cost of debt, spread, stale month, verdict context) |

Backtest: none applies. The value is a market input at a date, not a forecast, so a
point-in-time backtest has nothing to score. The check that applies is a comparison of the
steps with a second source: the ICE BofA OAS of 2026-11-18 (read from a local cache, not
stored) is within 0.08 points for AAA to BBB and higher for BB, B and CCC (A-200).

## Review status
Not reviewed. Built by the Technology head under the decisions of the users of ticket
`T-20261121-fred-local` (Fundamental and Economics). The Fundamental head examines the
steps before a valuation report uses `--rating`.
