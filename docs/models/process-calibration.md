# Model: calibration of the process table (`fa dcf calibrate`)

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Purpose
`fa dcf calibrate` estimates, from the company cache, the AR(1) parameters of revenue growth and of the margins,
the growth fade of fast growers, and the shock correlations. It writes the table that the automatic path model
reads (`docs/models/auto-path-model.md`). It also tests four band methods for revenue. It is for the head of
fundamental research and for any person who makes a new table.

## Inputs
| Input | Source | Note |
|---|---|---|
| Company facts | The cache `data/raw/<T>/company_facts.json[.gz]` | The standard variables of `mapcheck`. No network |
| Template of the filer | `mapcheck` (`standard`, `bank`, `insurer`, `reit`, `fund`) | Today's template |
| Date, cut | `--date` (default 2026-11-19), `--cut YEAR` | `--cut`: the facts are today's facts |

## Method (short)
1. Panel (`panel.rs`): one row for each company and fiscal year; chain only years 350 to 380 days apart;
   flag a growth jump (above 60%, revenue of the year before at $1B or more) and a pair of jumps that undo
   each other (A-013).
2. Series: growth, growth less the year mean, operating margin, free cash flow margin. Growth outside -80% to
   +300% and a margin outside -1 to +1 are not used. A jump is not in the sample.
3. For each group (size bucket of the revenue of the year before, and template) fit
   `y_t = mu + phi (y_{t-1} - mu) + sigma e_t` by four estimates of `phi`: pooled, within, within with the
   Nickell correction (the table value) and Anderson-Hsiao. Standard errors are clustered by company.
4. The spread of the company means (`tau`) is the spread of the within means less the noise of a mean of `n`
   values (`mean_variance_factor`).
5. Shock correlations (growth with margin) by a bootstrap over companies (`--reps`, default 200).
6. Fade table: for a start year with growth above 20%, 30% or 50% (organic), the P10, P50 and P90 of the
   growth 1, 2, 3, 5 and 10 years later, by size bucket.
7. Band test: P10 to P90 bands of revenue for T+1 to T+3 by four methods (legacy, AR(1) pooled, AR(1) within
   with Nickell, same with the group mean), with the parameters from all years and from the years to 2015.
8. Lookup: a cell needs 150 observations and 25 companies, else the next cell (size, template, all).

## Outputs
`analysis/priors/process_priors_2026-11-19.toml` (996 companies, 14,305 company-years, 13,228 growth
observations): `[[cell]]` for size and template, `[[fade]]` for the fast growers. The screen text has the
estimates, the survivorship note, the fade table and the band coverage. `--table FILE --lookup REVENUE,TEMPLATE`
prints the parameters of one company.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-013 | Design of the process table | 60% jump, $1B, buckets 1/10/30/100B, 150 obs, 25 companies | Conventions of the first design (commit 86b637a); no source |
| A-014 | Test design | train end 2015, 3 years ahead | Band test and the point-in-time test |

## Limits
- No sector cell. The size bucket is the revenue of the year before, so it selects (peaks): `HIDDEN-ASSUMPTIONS.md` #170, #174.
- Survivorship: the cache has companies that exist now. The growth level is biased up (`SURVIVORSHIP` in `calib_report.rs`).
- A restated year has the value of the latest filing. A 53-week year stays (#171, #172).
- The jump rule removes real growth and keeps some mergers (#173).
- The table is in sample unless `--cut` is used. The facts of `--cut` are today's facts.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/dcf/panel.rs`, `calibrate.rs`, `calib_bands.rs`, `calib_report.rs` |
| Model or parameter file | `analysis/priors/process_priors_2026-11-19.toml` |
| Command | `fa dcf calibrate --cache data/raw --out analysis/priors/process_priors_2026-11-19.toml --reps 200 --draws 2000` |
| Tests | `rust/src/dcf/calibrate_tests.rs`; `cargo test --manifest-path rust/Cargo.toml --locked` |

## Review status
Not reviewed (no skeptic, reviewer or auditor). The table was tested by the point-in-time test
(2026-11-19, 2026-11-20; `analysis/priors/auto_pit_*.txt`) and investigated on 2026-11-20
(`analysis/priors/dcf_investigation_2026-11-20.md`). Only one table file is in the repo: `process_priors_2026-11-19.toml`.
