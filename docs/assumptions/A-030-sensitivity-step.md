# A-030: step of the premium sensitivity (h = 0.001)

Updated: 2026-11-21. Commit: 2e70201 (branch `claude/premium-series`).
Model document: `docs/models/implied-and-attribute.md`.

## Choice
Column `dur_premium1` of `fa dcf attribute --out` is D = -(ln V(x1 + h) - ln V(x1 - h)) / (2 h), with
h = 0.001 (`DURATION_STEP`; a decimal, 0.1 point of premium). x1 is the premium that the price of D1 implies. V is the
value at the means with the facts, curve, beta and price weight of D1. Unit: d ln V for a change of 1.00 of the
premium. The column is empty when a value at x1 +- h does not exist.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Attribution | `rust/src/dcf/auto_attribute.rs` | `DURATION_STEP`, `premium_duration` |

## Alternatives considered
Not known. The design text (central difference at x1 +- 0.1 point) names no other step.

## Why this choice
The design of the ticket says: "two values at x1 +- 0.1 point". No source for the size is known. It is a
convention.

## Effect on the result
Not known. No test of other steps exists.

## Who decided and when
Head of Fundamental Research, 2026-11-21 (`PLANNING/board/T-20261121-premium-company-part.md`, design item 1(a)).

## When to examine it again
When a value of D changes by a large amount for a smaller step, or when the value is not smooth near x1.
