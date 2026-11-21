# A-112: Sampling rules of a calibration sample

Updated: 2026-11-21. Commit: not yet made.

## Choice
- A sample needs 30 observations or more after resampling.
- Weekly and monthly samples take the last observation of each period (week from Monday).
- The step `dt` = (last day - first day) / 365.25 / (n - 1), in years. It is the mean calendar spacing.
- A warning is given if the largest gap is more than 6 x the median gap (the fit assumes equal spacing).
- Default frequency: daily.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Sample | `rust/src/ratemodels/data.rs` lines 9, 123-170 | `MIN_OBS`, `dt`, `max > 6 * median` |
| Default | `rust/src/ratemodels/cli.rs` line 117 | `--freq` daily |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Business-day count for `dt` | Not known. |
| Mean of the period instead of the last value | Not known. |

## Why this choice
Author choice, no study. The code has no source for 30 or for 6. The step 365.25 is a convention.

## Effect on the result
Not known as a number. `fa models frequencies` gives the same interval at daily, weekly and
monthly sampling, so a person can see the effect of the frequency on the parameters.

## Who decided and when
Not known. Ticket `T-20261119-stochastic-dcf-drivers` (2026-11-19).

## When to examine it again
When the fit has a calendar gap, or when the parameters change much between frequencies.
