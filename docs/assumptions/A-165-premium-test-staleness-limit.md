# A-165: staleness limit of the daily series

Updated: 2026-11-21. Code: branch econ/premium-test-cmd.

## Choice
For VIX, DFII10 and T10YIE: the last non-missing value on or before the anchor of the week, if it is at
most 3 calendar days old. An older value gives no value (missing, never 0).
NFCI is no input after R2 (2026-11-21).

## Where used
| Model | File | Parameter name |
|---|---|---|
| premium macro test | `rust/src/macro_test/data.rs`, line 13 | `MAX_AGE_DAYS` |
| premium macro test | `rust/src/macro_test/data.rs`, `lookup_daily()`, `lookup_exact()` | the two rules |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| No limit (carry any value) | A stale level makes a false change. |
| Fill a missing level with 0 | A "." is missing, never 0 (design rule 4.3). |

## Why this choice
Design rule 4, data rules 2 to 5 (frozen at 55f2e92). DFII10 and T10YIE use the VIX rule: a reading choice
in `mod.rs` (choice 3, accepted 2026-11-21).

## Effect on the result
Not known. The output counts the carried values and the weeks without a value for each series. To test:
change `MAX_AGE_DAYS`, rebuild, compare the counts and the answer.

## Who decided and when
The Head of Economics, in the design (drafted and frozen 2026-11-21 at 55f2e92, after two review cycles of the Head of Quality and Risk).

## When to examine it again
If the counts of dropped weeks are large, or a data source changes its dates.
