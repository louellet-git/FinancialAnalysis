# A-201: the HQM curve is the bucket A

Updated: 2026-11-21. Commit: not yet made.

## Choice
The Treasury HQM corporate curve has the reference bucket A. The step of A is 0. The steps
of the other buckets are measured from the default spread of A (`REFERENCE_BUCKET = "A"`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Cost of debt | `rust/src/bonds.rs` | `REFERENCE_BUCKET` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| AA as the reference | It is a valid view of a blend of AAA, AA and A. It adds +0.23 points to the cost of debt of A and every lower bucket. Not chosen: we have no weights that favour AA |
| The mean of AAA, AA, A (0.58) | The weights of the HQM blend are not published in the files that we read |

## Why this choice
Convention. Treasury describes the HQM curve as a blend of AAA, AA and A bonds. We read no
weights. We did not measure the mix. A is the lowest bucket of the blend, so the HQM yield is a
little low for A and a little high for AAA. A is a simple choice, and an A- issuer such as
IBM gets the HQM yield with no step. The choice is a convention, not a measurement.

## Effect on the result
A change of the reference to AA moves every cost of debt by +0.23 points (A and below)
and AAA by -0.15. Effect on the IBM value: small (see the model document).

## Who decided and when
The Technology head, 2026-11-21, under the decision of the users of ticket
`T-20261121-fred-local`. Open to a review by the Fundamental head.

## When to examine it again
If Treasury publishes the rating weights of the HQM blend, or if the check against a
second source shows a gap of more than 0.25 points for A.
