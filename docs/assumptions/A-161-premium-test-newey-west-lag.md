# A-161: Newey-West lag of the premium test

Updated: 2026-11-21. Code: branch econ/premium-test-cmd.

## Choice
Lag 8 weeks, with Bartlett weights 1 - j/(lag+1). It sets the variance of the Clark-West statistic. The
descriptions (coefficient t-values) use the same lag.

## Where used
| Model | File | Parameter name |
|---|---|---|
| premium macro test | `rust/src/macro_test/stats.rs`, line 4 | `NW_LAG` |
| premium macro test | `rust/src/macro_test/design.rs` | passes `NW_LAG` to `clark_west` and `nw_se` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Lag 7 (T^(1/3) for T of about 310) | The design rounds up to 8. |
| Lag 0 (White errors) | Not known. The code tests lag 0 only as a check of the formula. |
| A data-driven lag | Not known. |

## Why this choice
Design rule 5.5: T is the count of OOS weeks. For T of about 310, T^(1/3) is about 7. Round up to 8.
This is a rule of thumb (a convention).

## Effect on the result
Not known. No sensitivity run is recorded. To test: change `NW_LAG`, rebuild, run the same command. The
lag changes the t and p only. R2 and the gain do not change.

## Who decided and when
The Head of Economics, in the design (drafted and frozen 2026-11-21 at 55f2e92, after two review cycles of the Head of Quality and Risk).

## When to examine it again
If the count of OOS weeks changes much (a long new sample), or if the p is near 0.05.
