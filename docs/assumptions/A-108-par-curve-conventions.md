# A-108: Conventions of the bootstrapped par curve

Updated: 2026-11-21. Commit: not yet made.

## Choice
`fa recon bonds` bootstraps the Treasury par curve to a zero curve with these rules:
- Tenors up to 6 months are bills. Their par yield is a bond-equivalent yield, simple
  interest, `D = 1 / (1 + y days/365)`. Tenors of 1 year and more are coupon bonds, with coupon dates at
  6, 12, 18, ... calendar months.
- Time is `days / 365`. The zero rate is continuous.
- Interpolation: monotone convex (Hagan and West 2006) on the forward rates; the forward rate
  is flat after the last node.
- Settlement is T+1 business day. Federal Reserve holidays are not business days.
- Observed price is the FedInvest end-of-day clean price.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Curve | `rust/src/curve.rs` lines 6-32, 384-397 (`BillBasis::Bey365`), 399-460 | `bootstrap` |
| Settlement | `rust/src/recon.rs` line 439 | `add_business_days(day, 1)` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Bill basis money-market (ACT/360), discount, semi-annual | Tested with `fa recon bills`: the mean difference is larger (see Effect). They stay in the code as test options. |
| Settlement T+2 | Tested: it gives the smallest mean difference (see Effect). Not adopted. |

## Why this choice
The Treasury says it builds its par curve with monotone convex interpolation. The Treasury
does not publish its program, so the curve is a re-creation, not a copy (`curve.rs` lines 19-24).
Regular way settlement for Treasury securities is T+1 (`curve.rs` line 25).

## Effect on the result
Mean of observed minus theoretical bill yield, 47 bills, 2026-11-18 (`rates_recon_step2_2026-11-20.md`,
section 3.1): baseline -7.1 bp; money-market basis -12.6 bp; settlement T+2 +1.0 bp; nodes from the
bill-rate table -1.7 bp. The note does not say which basis is correct. Not known: why the T+2 case is closest.

## Who decided and when
Not known. The convention list is a comment in `curve.rs`. Ticket `T-20261120-arb-2-rates-recon` (2026-11-20).

## When to examine it again
When the bill difference of the baseline is explained, or when the Treasury publishes its method.
