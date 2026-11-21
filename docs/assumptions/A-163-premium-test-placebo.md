# A-163: placebo shifts and the limit of 4 of 80

Updated: 2026-11-21. Code: branch econ/premium-test-cmd. Model document: `docs/models/premium-macro-test.md`.

## Choice
The block dVIX (one series since R2) shifts circularly in time, as one block, by every shift from 13 to 52 weeks in both
directions: 80 runs. y, ret_w, ret_w^2 and dzero10 stay in place. The main result "yes" stands only if at most 4
of the 80 runs give "yes" (5% of 80). Otherwise: "no share shown".

## Where used
| Model | File | Parameter name |
|---|---|---|
| premium macro test | `rust/src/macro_test/design.rs`, lines 11 to 13 | `PLACEBO_MAX_YES`, `SHIFT_MIN`, `SHIFT_MAX` |
| premium macro test | `rust/src/macro_test/design.rs`, `placebo()` | the loop over shifts |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Circular block bootstrap as the primary p | The design says it is not the primary p (rule 5.8). |
| Shifts shorter than 13 weeks | Not known. The shifts keep the block away from its own weeks. |
| A limit other than 5% | Not known. |

## Why this choice
Design rule 5.6 and 5.7 (frozen at 55f2e92; the block was (dVIX, dBAA10Y, dNFCI) until R1 and (dVIX, dNFCI) until R2, 2026-11-21: the Chicago Fed terms do not permit a stored copy of NFCI). The range and the limit come from review condition C2 of cycle 2: every shift from 13 to 52 weeks, and at most 5% placebo "yes".

## Effect on the result
Not known. No sensitivity run is recorded. To test: change the constants, rebuild, run the same command.
The output prints the placebo count and the gain range, min, median and max.

## Who decided and when
The Head of Economics, in the design (drafted and frozen 2026-11-21 at 55f2e92, after two review cycles of the Head of Quality and Risk).

## When to examine it again
If the placebo count is 3 to 6 of 80 at the first run of record, or if the block changes.
