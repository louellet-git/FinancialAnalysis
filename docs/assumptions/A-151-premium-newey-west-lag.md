# A-151: Newey-West lags and the block t of the premium

Updated: 2026-11-21. Commit: not yet made.

## Choice
30-day premium: Newey-West (Bartlett) t with 20 lags. The 21-day windows overlap, so
neighbour days are linked. Block t: every 21st day (the horizon), plain t. 1-day premium: 5 lags,
block step 1 (so the block t is the plain t). The lag of the 1-day premium is a constant in code,
with no option. The long-run variance uses the whole series, with zero deviations outside the mask.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Variance premium test | `rust/src/trading/premium.rs` | `--lag` (default 20 in `command`), `--lags` (`print_lags`); `row`; literal 5 in `report` |
| Variance premium test | `rust/src/trading/stats.rs` | `nw_mean_t`, `t_mean` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Lag 10, 30, 63, 126 | Shown as a check, not chosen (see Effect) |
| Lag equal to the horizon minus 1 | Not known. 20 is that value for horizon 21 |

## Why this choice
Author choice, no study. The research note says "the overlap of the 21-day windows asks for 20 lags
or more". It gives no source for the 5 lags of the 1-day premium.

## Effect on the result
Gate row (2016 to 2026, outside the windows), t_NW by lag: 10: 2.26, 20: 1.97, 30: 1.87, 63: 1.77,
126: 1.80 (`trading/research/alpha-1-stage1.md`, "Lag table"). The lag moves the t by about 0.5
and crosses the bar t = 2. Command: `fa trade premium --lag N`; one table of all lags: `fa trade premium --lags 10,20,30,63,126` (run on 2026-11-21: the same values). No sensitivity of the 5 lags is known.

## Who decided and when
Not known. The note of 2026-11-21 states 20 lags. No file names a person.

## When to examine it again
When the horizon changes (the lag must follow it), or when the premium is tested with another
method (for example a block bootstrap).
