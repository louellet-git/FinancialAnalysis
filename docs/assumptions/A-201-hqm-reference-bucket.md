# A-201: the HQM curve is the bucket A

Updated: 2026-11-21. Commit: a66a70d (step 3), fix commit to follow.
## Choice
The Treasury HQM corporate curve has the reference bucket A (`REFERENCE_BUCKET = "A"`). The step
of A is 0. The steps of the other rows are measured from the default spread of A (0.78).
Model document: `docs/models/cost-of-debt.md`.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Cost of debt | `rust/src/bonds.rs` | `REFERENCE_BUCKET` |
| HQM curve check | `rust/src/recon.rs` | default spread of A (one rule) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| AA as the reference (0.55) | The HQM level over the UST would then be 0.33 above AA. That is too much for a spot-against-par basis |
| The mean of AAA, AA, A (0.58) | Same reason. The weights of the HQM blend are not in the files that we read |

## Why this choice
Evidence, not a guess. The mean of HQM 10Y minus the UST 10Y monthly mean is 0.88 points over 36
months (2023-11 to 2026-10; minimum 0.78, maximum 1.22; `fa bonds over10y --rating A` on the 15th of
each month; the same number as a hand check of HQM10 minus FRED GS10). The Damodaran default
spread of A is 0.78. So HQM is at or above A, not below it. The 0.10 difference is the size of
the spot-against-par basis that a review gave (about 10 bp at 10 years; we did not measure it).
A as the reference fits this. AA does not.
Claim "HQM is a blend of AAA, AA and A bonds": Treasury methodology papers, listed on
`https://home.treasury.gov/data/treasury-coupon-issues-and-corporate-bond-yield-curve/corporate-bond-yield-curve`.
We did not read them. The choice does not rest on this claim, but on the 0.88 above.

## Effect on the result
A reference of AA would move every step by +0.23 points. The cost of debt of A itself does not
change (it is the HQM level). Effect on the IBM value: small (see the model document).

## Who decided and when
The Technology head, 2026-11-21, under the decision of the users of ticket
`T-20261121-fred-local`. Open to a review by the Fundamental head.

## When to examine it again
If Treasury publishes the rating weights of the HQM blend, or if the Damodaran table changes
so that the 0.88 is no longer near the A spread.
