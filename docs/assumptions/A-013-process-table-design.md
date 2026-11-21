# A-013: design of the process table (panel rules and cells)

Updated: 2026-11-21. Commit: 90ed737 (base).
Model document: `docs/models/process-calibration.md`.

## Choice
`fa dcf calibrate` makes the table of the AR(1) parameters of growth and margins from the company panel.
Rules of the panel (`rust/src/dcf/panel.rs`):
- Two fiscal years are in sequence only if their end dates are 350 to 380 days apart (`YEAR_DAYS`).
- End dates 14 days or less apart are one year (`SAME_YEAR_DAYS`); the latest filing stays.
- A growth jump has an absolute growth above 60% (`JUMP_GROWTH`). The revenue of the year before is $1B or
  more (`JUMP_MIN_REVENUE`).
- A jump is `acq_likely` when the cash paid for businesses is 10% or more of the revenue change (`ACQ_SHARE`).
- Two jumps that undo each other within 20% are a `glitch`.
Rules of the estimate (`rust/src/dcf/calibrate.rs`):
- Growth outside -80% to +300% (`GROWTH_LIMITS`) and a margin outside -1 to +1 (`MARGIN_LIMIT`) are not used.
- Size buckets of the revenue of the year before: below $1B, 1-10B, 10-30B, 30-100B, above 100B
  (`BUCKET_LIMITS`). The persistence `phi` is the within estimate with the Nickell correction.
- A cell needs 150 observations and 25 companies (`MIN_OBS`, `MIN_UNITS`). Else the model uses the
  cell of the size for all templates, then the template for all sizes, then all.
- Fade cells: start growth above 20%, 30% or 50%; look-ahead 1, 2, 3, 5 and 10 years (`FADE_THRESHOLDS`, `FADE_HORIZONS`).
No cell has a sector (the cache had no SIC code).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model, tests, portfolio strategies | `analysis/priors/process_priors_2026-11-19.toml` | the table (996 companies, 14,305 company-years) |
| Calibration | `rust/src/dcf/panel.rs`, `calibrate.rs`, `calib_report.rs`, `calib_bands.rs` | the constants above |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Pooled estimate of `phi` | Biased up; the within estimate with the Nickell correction is the table value (the file keeps both) |
| Cells by sector | Not possible on 2026-11-19: no SIC code (`docs/HIDDEN-ASSUMPTIONS.md`, #170, update 2026-11-20) |
| Other jump or bucket limits | Not known. No other value was tested |

## Why this choice
The estimators are in the module text of `calibrate.rs`. The limits 60%, $1B, 10%, 20%, 350 to 380 days,
the bucket limits, 150 and 25 have no source in any file read; they are conventions of the first design
(commit 86b637a, 2026-11-19, ticket `T-20261119-dcf-process-calibration`). Known traps: #170 to #174
(`docs/HIDDEN-ASSUMPTIONS.md`): no sector, restated years, 53-week years, the jump rule removes real growth,
the size bucket selects. Survivorship biases the growth level up (`calib_report.rs`, `SURVIVORSHIP`).

## Effect on the result
No sensitivity to the limits is known. The table feeds each value of the automatic model.
Point-in-time test with this table: `analysis/priors/auto_pit_new_2026-11-20.txt` (A-014).

## Who decided and when
Not known by name. The design is in commit 86b637a (2026-11-19). No review.

## When to examine it again
When the cache has the SIC code of each filer (cells by sector). When a new table is estimated
(`fa dcf calibrate`): the table file has a date in its name.
