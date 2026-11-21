# A-106: Cost of one leg in the rich and cheap measure

Updated: 2026-11-21. Commit: not yet made.

## Choice
Cost (bp of yield) of one leg = the larger of (a) the yield change of one price step and
(b) the bid-ask. One price step is 1/64 of a point for notes and bonds (the code moves the
price by 1/128 each way). A bill has a floor of 1 bp. If the file has no bid-ask, the bid-ask
is the median of the file values in the same maturity bucket (global median if the bucket
has none). The `SELL` column is taken as the bid side.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Rich and cheap (`docs/models/treasury-fit-rich-cheap.md`) | `rust/src/richcheap.rs` | `file_spread`, `tick_bp`, `tick.max(1.0)` |
| Assumed bid-ask | `rust/src/richcheap.rs` | `ba_source = "assumed"` |
| Buckets | `rust/src/recon.rs` | `bucket` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Round trip (two legs) | Not in the code. Step 2 note says the round trip is about twice the cost. |
| Repo, borrow, tax, impact | Not in the data (`richcheap.rs` ). |

## Why this choice
Author choice, no study. The comment in `richcheap.rs` says `SELL` equals `END OF DAY` for most
notes. The median fill is a convention.

## Effect on the result
Not known. For the 10 largest lasting residuals the one-leg cost is 0.85 to 2.50 bp against
a residual of 5 to 15 bp (`rates_recon_step2_2026-11-20.md`, section 3.4).

## Who decided and when
Not known. Ticket `T-20261120-arb-2-rates-recon` (2026-11-20).

## When to examine it again
When a true quote or a repo rate enters the store.
