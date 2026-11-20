# A-003: Floor on the growth of the base year

Updated: 2026-11-20. Commit: not made (the orchestrator makes it).

## Choice
When the organic revenue growth of the base year is below 0%, the automatic model uses 0% in its place for two
things only: the company mean of the growth and the first step of the growth process. The older growth values and the
shock size keep the real values. The floor value is 0%. A base year with growth of 0% or more is not changed.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model | `rust/src/dcf/auto.rs` | `Rules.last_floor` (default `Some(0.0)`; `--rules last_floor=-1` is off) |

## Alternatives considered
Half 0 (2,834 company-years), `fa dcf pit --half 0 --rules last_floor=X`. Median error of the revenue P50 at T+3:
| Alternative | All company-years | Decline class | Why not chosen |
|---|---|---|---|
| Floor 0% (chosen) | -0.0% | +6.5% | No fitted number. |
| Floor +3% | -0.4% | +4.9% | A positive floor is a parameter that the test would choose. It is not ruled out: it needs a study out of sample. |
| Floor +6% | -1.0% | +3.4% | The same. |
| Off (`last_floor=-1`) | +0.7% | +10.8% | The error that this record corrects. |
| Phi = 0 for the growth (full cache; off: +11.3%) | | +9.8% | It fixes T+1 only (T+2: +6.1%). |
| A recovery term estimated on the panel | | | The gaps are in a few base years with a common shock (table below). A term estimated on them fits those years. It needs a decision of the owner. |

The table above and the numbers of "Effect" below are on the defaults before pull request 77 (`long_run` off, `acq_w` off).
On the merged defaults (`long_run = 0.040`, `acq_w`, `acq_fade`; `fa dcf pit`, seed 7, 2,000 paths) the numbers are the
same to 0.1 point for the decline class: half 0 floor 0 -> T+3 decline +6.5%, off +10.9%; half 1 +0.0 / +5.1 / +7.4% against
+2.0 / +7.8 / +11.8%; `--cut 2015` +1.2 / +8.6 / +13.7% against +3.1 / +12.7 / +17.9%. All company-years, error T+1 / T+2 / T+3:
half 0 -0.7 / +0.2 / +0.7% off, -1.0 / -0.4 / -0.0% on (the floor makes T+1 slightly worse); full panel -0.4 / +0.7 / +1.7% off,
-0.7 / +0.1 / +1.0% on. Files (merged defaults, full panel, in sample): `analysis/priors/auto_pit_floor_before_2026-11-20.txt`
(`--rules last_floor=-1`) and `auto_pit_floor_after_2026-11-20.txt` (default).

## Why this choice
The next year after a decline year is not a decline. In the full panel (the scoring half too; the value 0 is not
fitted) the median growth of T+1 is +2.7% (decline 0 to 5%), +3.2%, +5.9% and +7.1% (decline above 20%). A negative
value in the company mean treats the decline as a rate that stays. The floor says only that it does not.
Command: `fa dcf pit --diag decline`.

## Effect on the result
Decline class, median error of the revenue P50 at T+1 / T+2 / T+3, half 1 (the scoring half): +2.0 / +7.8 / +11.8% before,
+0.0 / +5.1 / +7.4% after. With `--cut 2015`: +3.1 / +12.7 / +17.9% before, +1.2 / +8.5 / +13.6% after.
The error that stays, by base year (full panel, after the floor; real growth less model growth, points of growth):
| Base year | 2012 | 2013 | 2014 | 2015 | 2016 | 2017 | 2018 | 2019 | 2020 | 2021 | 2022 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| T+2 | +1.7 | -5.4 | -1.4 | +3.8 | +4.9 | -1.5 | -0.1 | +16.3 | +8.7 | +6.9 | +1.9 |
| T+3 | -6.3 | -3.2 | +4.6 | +3.5 | -2.4 | -1.8 | +11.8 | +6.9 | -0.8 | +4.7 | +2.3 |
The largest gaps are T+2 of 2019 and 2020 (+16.3, +8.7; T+1 of 2020: +12.9) and T+3 of 2018 and 2019 (+11.8, +6.9).
The years 2012 and 2013 overshoot. Side effect: for revenue above $100B (184 company-years, `--cut 2015`) the share
inside the band at T+3 is 97.3%, before 95.1%.

## Who decided and when
Valuation quant, ticket `T-20261120-dcf-engine-accuracy`, 2026-11-20. The Head of Quality and Risk held the value 0%.

## When to examine it again
When the table is made again with a longer panel, or when a study out of sample of a positive floor or of a
recovery term exists.
