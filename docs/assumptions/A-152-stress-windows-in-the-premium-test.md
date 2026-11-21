# A-152: the stress windows and the gate subset of the premium test

Updated: 2026-11-21. Commit: not yet made.

## Choice
13 stress windows (12 closed, 1 open) are read from a file at each run: `--stress-file PATH` (default
`analysis/macro/stress-windows.csv`, read by `parse_windows` of `fa portfolio calibrate`). The gate subset is
"from the first window start (2016-01-05) to the last day of the data, signal date outside every window
(inclusive)". The block has no end date of its own: a day after the last window end is outside every
window and stays in the block (before 2026-11-21 the end was fixed at 2026-11-19, and later days went to the
row "1993 to 2015"; finding P1). The windows come from one rule on the VIX
close: trigger 25 or more, start at the last close below 20, end at the first close below 20, join when
the gap is 10 trading days or less, list when 3 or more closes are at 25 or more. The code has no
window before 2016. A second view, "clean outside", also drops days with a window in the next 21 days.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Variance premium test | `rust/src/trading/premium.rs` | `parse_stress`, `in_window`, `block_start`, `clean`, `w16` in `report`; a test holds the 13 old values and compares them with the file |
| Source of the list | `analysis/macro/2026-11-21-stress-windows.md` and `analysis/macro/stress-windows.csv` | the rule |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Other levels (25, 20, 10 days, 3 closes) | The file says these are conventions and other levels give other dates (edge case 2025) |
| All days of 2016 to 2026, no window choice | Shown as a second row (t_NW 1.64) |
| Windows before 2016 (2000, 2008) | Not in the file. The years 1993 to 2015 are shown as information |

## Why this choice
Author choice, no study. The windows were defined with hindsight, after the stress was known, so
"outside windows" is a choice made with the future. The "clean outside" row drops days by a later
fact: it uses the future and is not a result. The list is no longer a copy: a change of the file
changes the run. The labels of the rows follow the file and the data (the last window is now "2026-11 (open)").

## Effect on the result
Outside windows t_NW 1.97, all 2016 to 2026 t_NW 1.64, "clean outside" t_NW 13.49 (biased)
(`trading/research/alpha-1-stage1.md`, stage 1a). The 2020 window is 13 months long and
covers the recovery (`analysis/macro/2026-11-21-stress-windows.md`).

## Who decided and when
The rule: the stress file of 2026-11-21 (the author is not named). The use of the windows as the gate
subset: the ticket `PLANNING/board/T-20261121-alpha-1-pipeline.md`. The window levels: not known.

## When to examine it again
When the stress file changes (the test of the 13 windows then fails on purpose) or when a window rule is set
before the data are seen (stage 2 of the ticket).
