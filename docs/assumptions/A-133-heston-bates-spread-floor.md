# A-133: floor of the quoted spread in the loss (0.05 vol points)

Updated: 2026-11-21. Commit: not yet made.

## Choice
The loss is the sum of squares of `(model price - mid) / (vega x spread)`. The spread in this weight has a floor of 0.0005 (0.05 vol points).
A quote with a spread below the floor gets the weight of a quote with the floor spread.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Heston, Bates (`docs/models/heston-bates.md`) | `rust/src/volmodels/base.rs` | `MIN_SPREAD` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| No floor | The code comment says one very tight quote takes all the weight. |
| A higher floor | Not known. |

## Why this choice
Reason in the code comment: "It stops one very tight quote from taking all the weight."
The value 0.05 vol points: author choice, no study. The ticket does not name it (it says weight 1 / spread).

## Effect on the result
It changes the weights of the tightest quotes only. The spread filter (A-132) allows spreads up to 1 point, so the floor acts on quotes with a spread below 0.05 point.
No run with another floor is recorded.

## Who decided and when
Agent `volatility-derivatives-quant`, ticket of 2026-11-20; date and person not known.

## When to examine it again
When the loss is dominated by a few tight quotes; when the fit uses a different weight rule.
