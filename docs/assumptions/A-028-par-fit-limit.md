# A-028: limit of the par fit (30 years; 5.0 bp)

Updated: 2026-11-21. Commit: 2e70201 (branch `claude/premium-series`).
Model document: `docs/models/implied-and-attribute.md`.

## Choice
The `par` curve is the NSS fit to the Treasury par yields of 8 tenors: 1, 2, 3, 5, 7, 10, 20 and 30 years
(`PAR_TENORS`). The fit is not used beyond 30 years. The model needs the 10-year zero yield and the curve to 30 years
only. Fit-quality limit: `PAR_FIT_MAX_RMSE_BP` = 5.0 bp (root-mean-square error of the fit, unit: basis point).
A fit above the limit is not used. This limit is being added to the code by another change. It is a rule of the
code that will be there.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Attribution, premium series | `rust/src/dcf/auto.rs` | `PAR_TENORS`, `par_fit`, `PAR_FIT_MAX_RMSE_BP` (to be added) |

## Alternatives considered
Not known. No other limit was tested in a file that was read.

## Why this choice
The tenors are the columns of the Treasury par yield file. Bills under 1 year are not in the fit. The value
5.0 bp has no source in any file that was read. It is a convention of the code change.

## Effect on the result
Not known. No sensitivity number exists. Check on 2026-11-13, 10-year zero: par 5.129%, FedInvest 5.169%, GSW
5.135% (`PLANNING/board/T-20261121-sp100-week-attribution.md`).

## Who decided and when
Head of Fundamental Research, 2026-11-21 (the 5.0 bp value: not known if a person or the code change set it).

## When to examine it again
When a week has a fit above the limit, or when the par file adds a tenor.
