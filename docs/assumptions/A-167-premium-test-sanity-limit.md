# A-167: sanity limit of the bridge in the premium test

Updated: 2026-11-21. Code: branch econ/premium-test-cmd. Model document: `docs/models/premium-macro-test.md`.

## Choice
Before the primary test, the correlation of `s2_rates` with the change of the 10-year zero yield (zero10)
over the kept weeks must be below -0.5 (no unit). If not, the command stops and gives no test result.

## Where used
| Model | File | Parameter name |
|---|---|---|
| premium macro test | `rust/src/macro_test/extras.rs`, line 9 | `SANITY_MAX_CORR` |
| premium macro test | `rust/src/macro_test/extras.rs`, `sanity()` | the check |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| A stricter limit (for example -0.8) | Not known. The design gives no other value. |
| No check | A bridge defect would enter the test as a result. The design requires the check (rule 7.2). |

## Why this choice
Design rule 7.2 (frozen at 55f2e92): the rates part of the bridge must move against the 10-year zero. A rise
in the yield lowers the value. If the check fails, the bridge has a defect and the test stops. Fundamental
Research finds the defect. The value -0.5 itself: no separate reason is stated.

## Effect on the result
Not known. No sensitivity run is recorded. A value above the limit stops the test. A value below the limit
has no effect on the test. To test: change `SANITY_MAX_CORR`, rebuild, run again.

## Who decided and when
The Head of Economics, in the design (drafted and frozen 2026-11-21 at 55f2e92, after two review cycles of
the Head of Quality and Risk).

## When to examine it again
If the check fails at a run, or if the bridge changes.
