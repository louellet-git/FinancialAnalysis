# A-173: no compounding conversion of STOCK10 or GSW10

Updated: 2026-11-21. Code: branch econ/view-engine-cmd. Model document: `docs/models/economics-view-engine.md`.

## Choice
The command does not convert STOCK10 or GSW10. GSW10 (SVENY10) is continuously compounded. The return form of STOCK10 (arithmetic or geometric, total or price) is not known. ERP_A can differ from a like-for-like premium by a compounding term.

## Where used
| Model | File | Parameter name |
|---|---|---|
| economics view engine | `rust/src/macro_test/view.rs` | `Level::erp_points()`, `levels()` (`g * 100.0`) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Convert GSW10 to an annual yield, or STOCK10 to a continuous rate | The return form of STOCK10 is not known. A conversion would be a guess. |

## Why this choice
The SPF documentation does not say the return form (design section 2 and record R3 item 3). The design names this as a condition of R1: resolve it, or state it as a limit in a dated record. R3 states it as a limit.

## Effect on the result
Record R3 gives an example: about 0.23 points at a 7% return (7% simple annual is ln(1.07) = 6.77% continuous). A change between two releases is less affected than the level. No run exists.

## Source
`analysis/macro/2026-11-21-view-engine-design.md`, rule 7, record R1 and record R3 item 3 (uncommitted text at the time of this record); `docs/HIDDEN-ASSUMPTIONS.md` #58.

## Who decided and when
The Head of Economics, record R3 (2026-11-21, about 18:50 UTC).

## When to examine it again
When the Head of Technology or the Head of Economics finds the return form of STOCK10, or before layer B.
