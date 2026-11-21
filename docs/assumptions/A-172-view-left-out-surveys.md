# A-172: a survey with a bound date or a missing value is left out

Updated: 2026-11-21. Code: branch econ/view-engine-cmd. Model document: `docs/models/economics-view-engine.md`.

## Choice
A survey is left out when its release date is a bound (no published date), or when RGDP10, CPI10, STOCK10 or the GSW10 value at the release date is missing. The command counts the surveys left out and prints the count on stderr.

## Where used
| Model | File | Parameter name |
|---|---|---|
| economics view engine | `rust/src/macro_test/view.rs` | `levels()` (second return value) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Use the bound date | A bound is not a public date. It can use a value before the public knew it. Not recorded in the design; the design record R3.2 gives no other reason. |
| Fill the missing value | Not known. R3 does not name it. |

## Why this choice
The design was silent on this point (record R3.2, written before any run on the real SPF file). Today no Q1 row of 1992 to 2026 uses a bound (record R2).

## Effect on the result
Not known. No run exists. A left-out survey makes the layer flat for one more year.

## Source
`analysis/macro/2026-11-21-view-engine-design.md`, record R3 item 2 (uncommitted text at the time of this record), after R2.

## Who decided and when
The Head of Economics, record R3 (2026-11-21, about 18:50 UTC).

## When to examine it again
When the stderr count of left-out surveys is above 0 in a run of record.
