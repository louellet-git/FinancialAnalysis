# A-128: tolerance of the VX settlement check (0.005 points)

Updated: 2026-11-21. Commit: not yet made.

## Choice
A monthly VX contract fails the check when the settle of the Cboe daily file differs from the settle of its Cboe history file (same date) by more than 0.005 index points.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Futures check | `rust/src/futures/check.rs:21-22`, `53` | `VX_TOLERANCE` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| 0.00005 (the rounding of 4 digits) | Not known. |
| Exact equality | Not known. |

## Why this choice
The code comment says: "Settlement prices in the two Cboe files are given to 4 digits." A rounding of 4 digits is 0.00005, which is 100 times smaller than 0.005.
So the comment does not explain the value. The reason for 0.005: not known (author choice, no study).
The test uses a difference of 0.006 (fails) and 0.004 (passes) (`check.rs:214-215`).

## Effect on the result
A difference between 0.00005 and 0.005 passes. For a VX price near 20, that is a relative difference up to 0.025%. No run with another tolerance is recorded.

## Who decided and when
Not known. The check is from the commit `c6adfa7` ("Futures data from free sources"); no person is named in it.

## When to examine it again
When a check shows a difference between 0.00005 and 0.005 that is a data fault; when the files change their precision.
