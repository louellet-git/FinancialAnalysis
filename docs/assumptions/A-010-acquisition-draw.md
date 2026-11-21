# A-010: The acquisition charge is a draw

Updated: 2026-11-21. Commit: not made (the orchestrator makes it).
Status: an option (`--rules acq_cv=1`). Default off. Not adopted: the blind score failed (see "Effect").

## Choice
Default: `acq_cv` off (`--rules acq_cv=0`).
With `acq_cv=1`, the charge of A-002 is a draw. Year 1: mean = the charge of A-002 (w times the mean spend ratio),
standard deviation = the population standard deviation of the own spend ratios of the last 5 years (the whole spend,
not w times it). Later years: the shock is that size over the charge of year 1, times the charge of the year, so it falls
with the fade. The floor is 0 and the cap is 1. No number is tuned.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model | `rust/src/dcf/auto.rs` | `Rules.acq_cv` (default `false`), `acq_sd_of` |

## Alternatives considered
Half 0 (`fa dcf pit --cut 2015 --half 0`), free cash flow, "all" row, below P10 / inside / above P90, T+1:
| Alternative | T+1 | T+3 | Why not chosen |
|---|---|---|---|
| Fixed charge (T1 rule) | 18.6/70.3/11.1 | 15.6/72.3/12.1 | The defect. |
| Shock = cv of the charge (w times smaller) | 17.4/73.7/8.9 | 14.0/75.7/10.3 | The band stays thin. |
| Whole-spend size, w floored at 0.25 | 15.7/75.6/8.7 | 11.4/78.2/10.3 | One more number. |
| Whole-spend size, no floor (this record) | 15.3/75.8/8.9 | 11.0/78.6/10.4 | Chosen: no extra number. |
Not tried: a heavier tail than the normal (lumpy deals); a rule for the revenue band. The half-0 tries were 8 runs
(5 diagnoses, 3 variants of the rule).

## Why this choice
The margin is not the cause: after the cut its one-step z has 6.5% below -1.28 and 7.9% above +1.28 (sd 1.07). The real
free cash flow subtracts the cash paid for businesses; the model charged a fixed ratio. Free cash flow below P10 at T+1:
10.9% with target-year spend below 0.5% of revenue, 58.4% with spend above 5%. Command: `--diag fcf`.

## Effect on the result
Blind score, half 1, one run, "all" row, T+1 / T+2 / T+3.
Free cash flow, before 16.8/72.0/11.2, 15.6/73.0/11.3, 14.4/73.2/12.3; after 13.9/76.9/9.2, 11.2/79.6/9.3, 10.6/78.3/11.1
(passes the T1 rule). Revenue 5.5/85.0/9.5, 5.3/84.0/10.7, 3.8/87.6/8.6 (not changed by the rule): fails at T+3
(inside above 85%, below P10 under 5%). So T1 FAILS on half 1. The full-sample T1 and the T2 to T4 runs were not made.
If they are made later they are not blind (designed after the first T1 failed; half 0 fitted the fix).
The mean charge is the same. IBM version 4 numbers do not change (the driver model does not use this rule).

## Who decided and when
Valuation quant measured it, ticket `T-20261121-blind-projection-test`, 2026-11-21. The Head of Fundamental Research
decides after the measurement.

## When to examine it again
When a rule for the revenue band at T+3 exists, or a lumpy (jump) model of the spend is tested on half 0.
