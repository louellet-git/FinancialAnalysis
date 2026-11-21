# A-134: forward of the Heston and Bates fit (5 strikes)

Updated: 2026-11-21. Commit: not yet made.

## Choice
The forward of a series is the median of `K + (C - P) / df` over the 5 strikes nearest to the strike where call and put mids are equal (`forward_from_parity_blind_n`, 5 strikes).
Mids come from live bids and asks (ask above bid, bid above 0). A series with no forward leaves the fit.
The fit keeps out-of-the-money quotes only: a call with K < F or a put with K >= F is dropped (no 1% band).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Heston, Bates (`docs/models/heston-bates.md`) | `rust/src/volmodels/chain.rs` | `PARITY_STRIKES` (5) |
| Heston, Bates | `rust/src/volmodels/chain.rs` | the out-of-the-money rule |
| Forward | `rust/src/market/option_iv.rs` | `forward_from_parity_blind_n` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| 3 strikes (`PARITY_STRIKES` of `option_iv.rs`, used by the chain check) | Not known. See A-126. |
| Spot and a dividend yield | Not known. The fit has no dividend input. |

## Why this choice
Ticket `T-20261120-vol-1-heston-bates`, data section: "Forward of each series from put-call parity (median of 5 strikes)".
The ticket gives no reason for 5. The 3 of the chain check differs: the same chain gets two forwards. Reason: not known.

## Effect on the result
The forward moves the implied volatility of every quote and so the loss. No sensitivity to 3 or 5 strikes is recorded.

## Who decided and when
Owner ticket of 2026-11-20 (vol-1).

## When to examine it again
When the check and the fit must have one forward (A-126); when a chain has few strikes near the money.
