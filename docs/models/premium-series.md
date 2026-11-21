# Model: the weekly premium series (`fa dcf premium-series`)

Updated: 2026-11-21. Commit: 2e70201 (branch `claude/premium-series`).

Backtest: none applies, because the command measures a past premium and does not forecast. Its point-in-time checks apply: each row uses only data public on its anchor. The poison test and the truncation test on 3 dates are a condition of the Economics design (rule 7). They have not run (not known).

## Purpose
The command writes one CSV row for each Friday: the premium that a list of tickers implies (S1), the median
weekly return of the list (ret_w) and, with `--bridge`, the "all" line of the full bridge (S2). The Head of
Economics uses the file for a test of macro risk measures. The design is
`analysis/macro/2026-11-21-premium-macro-design.md` on the branch `econ/premium-macro-design` (rules 1 to 4 and 7;
frozen by Quality and Risk, not known if done). The file has no macro data. It is a measurement, never an input
of the model. The value is the value at the means, not the expected value.

## Inputs
| Input | Source | Note |
|---|---|---|
| List | `--tickers-file` | A list of today's names (survivorship) |
| Facts | `map_inputs_pit` | Facts filed on or before the anchor |
| Closes, beta | the cache; SPY is the trading calendar | Last close on or before the anchor |
| Curve | GSW file, else the Treasury par file | The curve rule below |
| Model | `fa dcf auto` builder and the table | As for `fa dcf implied`; `--rules` changes the builder |

## Method (short)
1. Anchor of a Friday: the last date on or before it with a SPY close (A-033). An error when that date is 7 days
   or more before the Friday.
2. Curve of an anchor (A-027): the GSW row if it is on or before the anchor and at most 3 days older. Else the par
   fit of the last par row on or before the anchor. The first week with the par fit is the switch week. It is
   printed, and Economics drops it from all models (design, rule 3).
3. S1: the premium for which the median of value / price of the list is 1 (the search of `fa dcf implied`,
   `list_premium`; range in A-015). It is a decimal for one year. It is empty when the search has no solution in its
   range; a bound is not a measurement.
4. ret_w: the median over the list of close(anchor) / close(previous anchor) - 1. A ticker with no close at an
   anchor, or with a close older than the SPY close of that anchor (A-031), is out. `n_ret` is the count that stays.
5. S2 (with `--bridge`): the line "all" of the full bridge of `fa dcf attribute` (premium first) from the previous
   anchor to the anchor: facts, time, rates, premium, growth, residual, nomodel (shares of the market value of the list
   at the previous anchor) and s2_change (the change of the list, value-weighted). The bridge has the rules of
   `docs/models/implied-and-attribute.md`.
6. The first row has no previous week: `ret_w`, `n_ret` and the `s2_*` columns are empty.
7. `--per-ticker FILE` (rule of the code, being built; ticket item R5): a second CSV with one row for each week and
   ticker: the bridge line of the ticker in points of the list, the price change and the weight. The main CSV does
   not change.

## The CSV contract with Economics
`HEADER` in `rust/src/dcf/auto_premium.rs` is fixed:
`friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change`.
- Rates and returns are decimals (0.0232 is 2.32%). A missing value is an empty field.
- `curve_source` is `gsw` or `par`. `zero10` is the 10-year zero yield of that curve.
- `n_value`: tickers with a value and a price (the tickers of the median). `n_list`: tickers in the list.
- A change of the header is a change of the contract. Tell Economics first.
- The result does not depend on `--jobs`. Each row is written when it is done.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-027 | Curve rule | GSW at most 3 days older, else par | Switch week printed |
| A-028 | Par fit limit | 30 years; 5.0 bp | Quality limit of the par fit |
| A-031 | Stale-price rule | close older than the SPY close | A ticker is out of ret_w |
| A-033 | Anchor rule | last SPY close on or before the Friday; 7 days | SPY is the calendar |
| A-015 | Search range of the premium | -4% to 20% | S1 search |
| A-012 | Age of a price and of a curve row | 10 days (price); 3 days (GSW row) | Past close; the curve rule |

## Limits
- The list has today's names (survivorship). The premium is a residual: a growth or cash-flow error of the model
  enters it (design, rule 8).
- S1 sets the median of value / price to 1 by design. A large part of its weekly change is mechanical.
- The GSW curve and the par fit have different fits. The switch week can show a step in `zero10` and in S1.
- A list premium or a ratio is not a signal without a review (`HIDDEN-ASSUMPTIONS.md` #260).
- Coverage gap (checked on 2026-11-21, store 0340bc1). A ticker with the mapping status FAIL at an anchor has no value
  at that anchor: it is not in S1, and with no share count it is not in `s2_change`. Examples: BRK-B, V and NEE have no
  value on 2020-03-20 and on 2022-06-17. BRK-B and V have a value on 2026-11-20; NEE has none. COP has a value on
  2022-06-17; on 2020-03-20 its share count has a scale defect (1,123,536). The columns `n_value` and `n_ret` of each
  row show the coverage of that week.
- The series run of record. S1 uses the parameter table `process_priors_2026-11-19.toml`, which is fitted on all years.
  A weekly S1 from 2019 thus uses later priors: this is a look-ahead in the table. The run of record uses a table cut
  before its first year (`--table`), or its header states this look-ahead.
- The switch week. `s2_rates` and `s2_premium` mix the GSW curve at the previous anchor and the par curve at the
  current anchor. Week to 2026-11-20: rates +5.23 with the mix, +5.08 with the par curve at both dates.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/dcf/auto_premium.rs`, `rust/src/dcf/auto_attribute.rs` (the bridge, `stale_note`) |
| Command | `fa dcf premium-series --tickers-file FILE --start D1 --end D2 --out FILE [--bridge] [--per-ticker FILE]` |
| Tests | `rust/src/dcf/auto_premium_tests.rs` |

## Review status
Gate of the Head of Quality and Risk on 2026-11-21 (a993e68, then ee82144): PASS-WITH-NOTES. Point-in-time tests on
main: `analysis/priors/data/premium_series_pit_main/result.md` (Quality: PASS-WITH-NOTES). Run of record of the S&P 100
list from 2019-01-04: `analysis/priors/data/premium_series_sp100_2019_record_main-e2557b6/`, with the label "parameters
in-sample" (the parameter table is fitted on all years; no cut table exists).
