# A-101: Weight 1 / (dirty price x modified duration)

Updated: 2026-11-21. Commit: not yet made.

## Choice
Each security has the weight `1 / (dirty price x modified duration)` in the least squares
fit of clean prices. A price error times the weight is a yield error. The residual unit is bp.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Treasury fit | `rust/src/fit.rs` lines 86-106 (`make_inst`) | `Inst.weight` |
| Duration step | `rust/src/fit.rs` line 91 | `h = 1e-4` (yield step of the duration) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Equal weight on price | A long bond would dominate because its price moves more (`fit.rs` lines 16-20). |
| Bid-ask weight | Not in the code. Not known if it was tested. |

## Why this choice
Comment in `fit.rs` lines 16-20: each security is one yield observation. The comment names
the weight 1/duration of Bliss. No other source in the repo. Test
`weight_turns_price_error_into_yield_error` checks the first-order link.

## Effect on the result
Not known. No run with another weight.

## Who decided and when
Not known. Ticket `T-20261120-arb-2-rates-recon` (2026-11-20).

## When to examine it again
When the residual of long bonds looks larger than the residual of short notes without a
reason, or when a bid-ask weight is wanted.
