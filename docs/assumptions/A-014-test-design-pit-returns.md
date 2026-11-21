# A-014: design of the point-in-time test and of the return test

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
`fa dcf pit` (revenue and free cash flow bands):
- Each base year T needs 5 fiscal years up to T (`NEED_BEFORE` = 5) and 3 after (`HORIZON` = 3).
- The band is P10 to P90; the target coverage is 80%. Default 2000 paths, seed 7.
- `--half`: the parity of the FNV-1a hash of the ticker. A fixed rule, written before the results.
- `--cut YEAR`: a table estimated on the years up to YEAR (out of sample). `TRAIN_END` of the band calibration: 2015.
`fa dcf returns` (value against the later price return):
- The model date is the end of the fiscal year plus 100 days (`--lag` 100). Reason in the code: a 10-K is
  due 60 to 90 days after the year end (`auto_returns.rs`, module text).
- Price returns for 1 and 3 years. The rank correlation of P50 / price with the return, by year; a year
  needs 20 observations or more (`MIN_YEAR_OBS` = 20). Default 1000 paths, seed 7.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Point-in-time test | `rust/src/dcf/auto_pit.rs` | `NEED_BEFORE`, `HORIZON`, `ticker_half` |
| Return test | `rust/src/dcf/auto_returns.rs` | `--lag`, `MIN_YEAR_OBS` |
| Band calibration | `rust/src/dcf/calib_bands.rs` | `TRAIN_END`, `HISTORY` = 4, `HORIZON` = 3 |
| Effect runs | `rust/src/dcf/assumptions.rs` | `fa dcf assumptions --effect` |

## Alternatives considered
Not known. No other lag, window or band was tested. The lag is a `--lag` option.

## Why this choice
The 80% coverage is the nominal width of P10 to P90. The 100-day lag has a stated reason (above). The
numbers 5, 3, 20 have no source in any file read. The tests are measurements: no input of the model reads them.

## Effect on the result
Results (not a sensitivity): revenue coverage P10 to P90 86.1 / 83.8 / 86.9% at T+1 / T+2 / T+3
(`analysis/priors/auto_pit_new_2026-11-20.txt`). The return test of 2026-11-20 gave a mean rank
correlation of +0.058 (t 1.6) at 1 year (`analysis/priors/auto_returns_new_2026-11-20.txt`). **That run
was before the share-basis rule** (A-017): the share counts had no split factor. Do not quote it as valid.
The limits: table in sample, survivorship, restated years, price returns (`LIMITS` in both files; #199).

## Who decided and when
Not known by name. Point-in-time test: commit 176aa8e (2026-11-19). Return test: ticket
`T-20261120-dcf-investigation`, step 3 (code header). No review.

## When to examine it again
After the share-basis rule: run `fa dcf returns` again. When a table is estimated with a cut. When a
point-in-time index list exists (survivorship).
