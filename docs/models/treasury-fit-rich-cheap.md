# Model: Fitted Treasury curve and the rich and cheap measure

Updated: 2026-11-21. Commit: not yet made.

## Purpose
WARNING: a residual is a model residual, not a trade. The data have no repo rate, borrow cost, tax or
executable quote (`docs/HIDDEN-ASSUMPTIONS.md` #112, `rust/src/richcheap.rs` ).

The model fits one smooth discount curve to the end-of-day prices of the Treasury notes and bonds of one
date. The difference of each security from the curve, in yield, shows which security is rich or cheap
against the others. `fa recon bonds` is a second, simpler check: it bootstraps the Treasury par curve and
lists observed minus theoretical price for bills, notes and bonds, and one comparison with the HQM corporate curve.
For a person who studies Treasury pricing. It is not an input of a DCF.

## Inputs
| Input | Source | Note |
|---|---|---|
| Prices | `data/raw/prices/fedinvest_<date>.csv` (FedInvest end of day) | Bid-side clean price. No volume, no time of day. |
| Par curve | Treasury par yields of the date (`fa.curve`) | Only for `fa recon bonds`. |
| HQM, ICE OAS | `data/raw/bonds/fred_*.csv` | Only for the corporate comparison. |
| GSW curve | `data/raw/rates/gsw_feds200628.csv` | Comparison table of `rich-cheap`. |

## Method (short)
1. Select notes and bonds with a life of 3 months or more (A-103). Bills, TIPS, FRN, callable and zero coupon are out.
2. Each security has cash flows, accrued interest and the weight `1/(dirty x duration)` (A-101).
3. Fit an NSS curve (A-100) by Levenberg-Marquardt from 3 starts of a grid (A-104).
4. Repeat the fit with the far securities left out, 3 passes (A-102). The curve of the last fit is the result.
5. Residual = observed yield minus fitted yield, bp. Plus is cheap, minus is rich.
6. Cost of one leg: tick or bid-ask (A-106). The z-score uses the 20 nearest securities; the group is a label (A-107).
7. A candidate has a residual above cost and fit RMSE, `|z|` of 2 or more (A-105).
8. `fa recon bonds` bootstraps the par curve (A-108), prices each instrument, and compares HQM with zero plus mean OAS (A-109).
9. `fa recon history` fits each cached date and measures the autocorrelation and half-life of the residual.

## Outputs
- `rich-cheap`: a CSV (`data/recon/richcheap_<date>.csv`) with the residual, `z`, cost, group, side and candidate flag, and a summary.
- `bonds`: a CSV and a curve node file (`data/recon/bonds_<date>.csv`, `_curve.csv`) and the largest 12 differences.
- `history`: the autocorrelation at lags 1, 5, 10, the half-life, the reversal of the 10% extreme securities.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-100 | NSS family | 6 parameters | Few parameters, same family as GSW. |
| A-101 | Fit weight | 1/(dirty x duration) | One yield observation for each security. |
| A-102 | Outlier rule | 3 passes, 4 sigma, floor 0.5 bp | One bad price must not move the curve. |
| A-103 | Selection | life 0.25 y (panel 0.5 y), no bills | Coupon curve only. |
| A-104 | Start grid and tau bounds | tau 0.08 to 40.4 y | Clamp is silent. |
| A-105 | Candidate rule | above cost and RMSE, `|z|` 2 | Author choice. |
| A-106 | Cost | max(tick, bid-ask) | Bucket median if no bid-ask. |
| A-107 | z neighbours and groups | 20 securities, 1.25 pt coupon | Group is a label only. |
| A-108 | Par curve conventions | monotone convex, T+1, BEY bills | Re-creation of the Treasury method. |
| A-109 | HQM test | mean OAS of AAA, AA, A | One flat spread for all tenors. |
| A-117 | Lasting residual rule | 38 dates, 90% same sign, above cost and 1.9 bp | Author choice, no study. |

## Lasting residuals (`fa recon history --lasting`)
The command replaces the `awk` script B5 of the step 2 note. It fits each stored price day with the code of
`fit.rs` and `richcheap.rs` (no second fit), builds the panel of residuals, and lists each security that meets
the rule A-117. Input: the stored files `data/raw/prices/fedinvest_<date>.csv` only. The command makes no
request, reads no GSW or Treasury file, and writes no file. If fewer than `--min-dates` price files are in
range, it stops with a message.

| Test of a security | Default | Option |
|---|---|---|
| Dates | 38 or more | `--min-dates` |
| Days with the sign of the mean | 90% or more (a zero day does not count) | `--same-sign` |
| `|mean|` above the mean cost (A-106) | strict | none |
| `|mean|` above the fit RMSE | 1.9 bp, strict | `--min-abs-bp` |

Output: one row for each lasting security (CUSIP, maturity, days, share of the same sign, mean residual in bp,
mean cost, standard deviation, half-life, group), sorted by `|mean|`. Then the counts: lasting of all securities,
rich, cheap, and by group. The group is the label at the last date of the security.
The half-life is of the deviation from the own mean: `ln(0.5)/ln(phi)` with `phi` from a regression
through the origin of lag 1 (the function `coefficient`). It is `-` if `phi` is not in (0, 1).

Point in time: `--to D` reads no file after `D`. A later day does not change the result of an earlier end date.
Tests: fixed counts of a synthetic panel, the edges (38 and 37 dates; 36 of 40 days is exactly 90%), and a panel
with poisoned later days. A run with `--to 2026-11-10` on the full store gave the same text as a run on a store
that has only the files up to that day (52 days; 63 of 337 securities).

Result on the 58 days (2026-08-27 to 2026-11-19): 60 of 340 securities are lasting: 29 rich, 31 cheap.
Groups: old 30-year bond 9, low coupon 13, high coupon 5, recent issue 1, near coupon date 2, other 30.
The 10 largest are rich (5.1 to 14.6 bp): the original 30-year bonds of 2036 to 2040.
Against the note (section 3.4): the counts and the 10 table rows agree (mean, cost). Two differences:
- The note says 61. The script rounds the share of the same sign to 2 decimals. 91282CAU5 has 89.7% (52 of 58),
  which the script rounds to 0.90. The code uses the exact share, so it has 60 (the note says "60 with exact values").
- The half-life differs (for 912810PW2: 2.4 days here, 1.6 in the note). The script divides by the sum of squares
  of all 58 deviations; the regression through the origin leaves out the last day. The last deviation is large
  for a trending series, so the two values differ. Here 912810FT0 has `phi` above 1 and no half-life.
  The standard deviation here uses n-1, the script used n (0.87 against 0.86).

## Limits of the lasting rule
- A lasting residual is a candidate after cost, not a trade. The command sends no order.
- 58 days, one regime. The rule values are set on the same days that they score (A-117). No unit-root test was run:
  a `phi` near 1 shows that the level is close to a unit root.
- The cost is one leg. A round trip is about two legs. Repo, borrow fee and tax are not in the data.

## Limits
- A candidate is a security to study. It is not an arbitrage and not a trade (`richcheap.rs` `LIMITS_ALL`).
- Do not use prices as executable quotes: they are end-of-day bid prices.
- The residual is a price gap plus model error. Fit RMSE is 1.46 to 2.50 bp (mean 1.90) over 58 days; 2.77 bp for 10Y to 20Y on 2026-11-18.
- The residual of a security is mostly a fixed level for that security. The part that is left, after the mean of the security is removed, lasts about 1 to 3 days (note step 2, section 3.3).
- The bonds that stay rich are the original 30-year bonds of 2036 to 2040. They can be a group effect. The cause is not tested, because `fa recon explain` was not run (the New York Fed files are not in the store).
- Bills of 6 months and more price 4 to 12 bp rich against the note curve. The sample is 3 dates (hypothesis, not proved).
- `fa recon bonds` HQM test: a flat spread does not fit the term structure. The result is a convention gap (-29 bp at 1Y to +51 bp at 30Y).
- The data are one year (2026) and 58 days of prices. No other regime.
- Network: `fa recon bonds` gets FedInvest prices. The step 1 note records a download by mistake (defect D5).
- Network: `rich-cheap` uses `forwards::load` (`rust/src/forwards.rs`) for its comparison table. It downloads the GSW file when the cache is older than 12 hours. Check the cache before you run.
- Backtest: none applies, because the model is a daily check of one price set. It moves no value with history and it does not forecast.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/fit.rs`, `rust/src/curve.rs`, `rust/src/recon.rs`, `rust/src/richcheap.rs`, `rust/src/history.rs`, `rust/src/billdiag.rs` |
| Model or parameter file | none (the values are in the code) |
| Command | `fa recon bonds\|bills\|rich-cheap\|history\|explain [--date D] [--out FILE]`; `fa recon history --lasting [--to D] [--min-dates N] [--same-sign S] [--min-abs-bp B]` |
| Tests | `rust/src/fit.rs`, `rust/src/recon.rs`, `rust/src/richcheap.rs`, `rust/src/history.rs` (`cargo test --manifest-path rust/Cargo.toml --lib fit`, `recon`, `richcheap`, `history`) |
| Notes | `analysis/priors/rates_recon_2026-11-20.md`, `analysis/priors/rates_recon_step2_2026-11-20.md` |

## Review status
Not reviewed. The two notes are investigation notes of ticket `T-20261120-arb-2-rates-recon`
(agent `rates-credit-quant`; gate: auditor). Their numbers come from `awk` scripts that the auditor must run again.
