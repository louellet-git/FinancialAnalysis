# A-018: verdict bands over the chance that the price is too high

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
`fa verdict` and the report take the share of simulated values per share below the market price
(`chance_price_too_high`) and name one of five bands (`verdict_band`, `rust/src/dcf/verdict.rs`):
| Chance that the price is too high | Verdict |
|---|---|
| below 25% | Strong opportunity |
| 25% to below 40% | Opportunity |
| 40% to 60% | No edge |
| above 60% to 75% | Overpriced |
| above 75% | Strongly overpriced |

## Where used
| Model | File | Parameter name |
|---|---|---|
| Verdict and report | `rust/src/dcf/verdict.rs` | `verdict_band` (0.25, 0.40, 0.60, 0.75) |
| Documentation | `README.md` (table of the verdict) | the same table |

## Alternatives considered
Not known. The proposal (`PLANNING/proposals/2026-10-15-investor-verdict-layer.md`) names no other set of cutoffs.

## Why this choice
The proposal says: the cutoffs at 25% and 75% follow Morningstar's bull/bear calibration. No file gives a
source for the cutoffs 40% and 60%. The bands are a reading aid. The value range itself is the result.
The verdict is only as good as its model: the P10 and P90 of the driver model are too narrow (backlog item
T-DCF-1; `docs/HIDDEN-ASSUMPTIONS.md`, #12 and `README.md`), so the chance is too sure. A verdict of a first
pass (JNJ, ANT) has no review.

## Effect on the result
No sensitivity number is known. A band changes the word, not the value range.

## Who decided and when
The proposal of 2026-10-15; piece 2 (the verdict band) shipped first, ticket `T-20261025-verdict-layer`
(pull request 16). The reviewer of the cutoffs: not known.

## When to examine it again
When the tails of the driver model are corrected (T-DCF-1). When the bands are tested against later returns.
