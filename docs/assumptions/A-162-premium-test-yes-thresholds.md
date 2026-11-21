# A-162: the "yes" thresholds of the premium test

Updated: 2026-11-21. Code: branch econ/premium-test-cmd.

## Choice
The answer is "yes" only if the one-sided Clark-West p is below 0.05 AND the gain of M1 over M0 is at
least 0.05 (5%, no unit: 1 - SSE(M1) / SSE(M0)). If not, the answer is "no share shown".

## Where used
| Model | File | Parameter name |
|---|---|---|
| premium macro test | `rust/src/macro_test/design.rs`, line 10 | `GAIN_MIN` |
| premium macro test | `rust/src/macro_test/design.rs`, line 9 | `CW_P_MAX` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| p alone | The design puts the effect size in the answer. A small gain can have a small p. |
| Another gain limit (for example 0.02 or 0.10) | Not known. The design gives no other value. |
| Block bootstrap p | The design says it is not the primary p (rule 5.8). |

## Why this choice
Design rule 5 (frozen at 55f2e92). The value 0.05 for p is a convention. The gain limit 0.05 comes from review condition V5 of cycle 1: an OOS R2 of 0.5% must not count as a "yes". The reviewer named 5% as an example and the Head of Economics took it.

## Effect on the result
Not known. No sensitivity run is recorded. The command prints the gain, t and p, so a person can read the
result against another limit without a new run. To test in code: change `GAIN_MIN`, rebuild, run again.

## Who decided and when
The Head of Economics, in the design (drafted and frozen 2026-11-21 at 55f2e92, after two review cycles of the Head of Quality and Risk).

## When to examine it again
After the first run of record, only through a new dated record (design rule 10).
