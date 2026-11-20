# A-001: one long-run growth of the automatic model

Updated: 2026-11-20.

## Choice
The automatic model uses one number L for two things: the level that the company growth
fades to by year 10, and the mean of the terminal growth.
L = the lower of the 10-year zero yield of the model date and G.
G = the long-run nominal growth of the economy = the longer-run median "Change in real
GDP" + the longer-run median "PCE inflation" of the FOMC Summary of Economic Projections
(SEP). SEP of 2026-09-16: 2.0% + 2.0% = 4.0%. The standard deviation of the terminal
growth stays 0.75%. A company with a mean below L keeps its own mean.
The reviewed IBM model keeps its own 2.25% (a closed decision that does not change).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model (`docs/models/auto-path-model.md`) | `rust/src/dcf/auto.rs` | `Rules::long_run` = `Some(LONG_RUN_GROWTH)` = 0.040 |
| Reviewed IBM model | `analysis/IBM/driver_model.toml` | terminal growth driver (2.25%, not changed) |

## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| 2.25% for the terminal growth, the yield for the fade (the old model) | `--rules long_run=off` | Two numbers that do not agree: a step of 2.9 points between year 10 and year 11 |
| The 10-year zero yield for both | `--rules terminal_rf=1` | The yield is a cap and a proxy, not the growth of the economy; sources A and B of the study cap the stable growth at the growth of the economy |

## Why this choice
None of the reasons uses the price.
(a) Sources A and B of the study cap the stable growth at the growth of the economy.
(b) The panel does not support 5.13% for large companies. The 10-year test rejects 2.25%
against a higher level, but it does not select 4.0% (in the study, "no cap" was best).
(c) With L near the growth of year 10, the margin of year 10 pays for about the growth
that the terminal value uses (study 1.3: a hand calculation with an assumed ROIC of 10%
to 15%). (d) One number removes the step between year 10 and year 11.
Why the FOMC number and not the CBO 3.8%: we can read the FOMC page (Table 1, column
"Longer run", read 2026-11-20: `https://www.federalreserve.gov/monetarypolicy/fomcprojtabl20260916.htm`).
The FOMC publishes it four times a year, and ALFRED keeps each vintage. The CBO site
refuses requests: we have secondary summaries only. The sum of two medians is a convention.
The number follows the source: a new SEP value changes the default and this record with
no new review of the rule. Source: `analysis/priors/dcf_conventions_study_2026-11-20.md`.

## Limits
`fa dcf pit` and `fa dcf returns` use the constant 4.0% (SEP of 2026-09-16) at each past
base date: a small look-ahead of a constant. A point-in-time SEP series is not built.

## Effect on the result
Study (3.8%, 430 tickers, median P50 / price): old 0.489; `long_run=0.038` 0.546;
`terminal_rf=1` 0.603. Run of 2026-11-20 on the data of 2026-11-19 (data store commit
41116d0; `fa dcf screen`, `sp500.txt`, `--as-of 2026-11-19`, 4,000 paths, seed 7; old model
`--rules long_run=off,acq_w=0`): 430 tickers with a price, median ratio 0.4895 (old),
0.5605 with L alone (`acq_w=0`), 0.4745 with both defaults. They do not add.
Pit revenue error T+1 to T+3: -0.4 / +0.7 / +1.7% (old +1.6%). AAPL, 10,000 paths: P50 $122.57 to $132.89.

## Who decided and when
The Head of Fundamental Research, on the word of the owner of 2026-11-20 (record:
`chiefs/louellet-git/fundamental/RECORD.md`). The old value: IBM review, 2026-07-16.
Review of the same weight as the IBM review (`CLAUDE.md`, source rule 3): review: not done yet.

## When to examine it again
At each SEP (March, June, September, December), and when a sector study exists.
