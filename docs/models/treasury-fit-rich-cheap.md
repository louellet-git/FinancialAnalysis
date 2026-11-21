# Model: Fitted Treasury curve and the rich and cheap measure

Updated: 2026-11-21. Commit: not yet made.

## Purpose
WARNING: a residual is a model residual, not a trade. The data have no repo rate, borrow cost, tax or
executable quote (`docs/HIDDEN-ASSUMPTIONS.md` #112, `rust/src/richcheap.rs` lines 94-98).

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

## Limits
- A candidate is a security to study. It is not an arbitrage and not a trade (`richcheap.rs` `LIMITS_ALL`).
- Do not use prices as executable quotes: they are end-of-day bid prices.
- The residual is a price gap plus model error. Fit RMSE is 1.46 to 2.50 bp (mean 1.90) over 58 days; 2.77 bp for 10Y to 20Y on 2026-11-18.
- The residual of a security is mostly a fixed level for that security. After the own mean is removed it lasts about 1 to 3 days (note step 2, section 3.3).
- The lasting rich bonds are the original 30-year bonds of 2036 to 2040: a group effect. The cause is not tested (`fa recon explain` was not run: the New York Fed files are not in the store).
- Bills of 6 months and more price 4 to 12 bp rich against the note curve. The sample is 3 dates (hypothesis, not proved).
- `fa recon bonds` HQM test: a flat spread does not fit the term structure. The result is a convention gap (-29 bp at 1Y to +51 bp at 30Y).
- The data are one year (2026) and 58 days of prices. No other regime.
- Network: `fa recon bonds` gets FedInvest prices (the step 1 note records a download by mistake, defect D5). `forwards::load` (GSW file, used by `rich-cheap` for its comparison table) downloads when the cache is older than 12 hours (`rust/src/forwards.rs` lines 243-252). Check the cache before you run.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/fit.rs`, `rust/src/curve.rs`, `rust/src/recon.rs`, `rust/src/richcheap.rs`, `rust/src/history.rs`, `rust/src/billdiag.rs` |
| Model or parameter file | none (the values are in the code) |
| Command | `fa recon bonds\|bills\|rich-cheap\|history\|explain [--date D] [--out FILE]` |
| Tests | `rust/src/fit.rs`, `rust/src/recon.rs`, `rust/src/richcheap.rs` (`cargo test --manifest-path rust/Cargo.toml --lib fit`, `recon`, `richcheap`) |
| Notes | `analysis/priors/rates_recon_2026-11-20.md`, `analysis/priors/rates_recon_step2_2026-11-20.md` |

## Review status
Not reviewed. The two notes are investigation notes of ticket `T-20261120-arb-2-rates-recon`
(agent `rates-credit-quant`; gate: auditor). Their numbers come from `awk` scripts that the auditor must run again.
