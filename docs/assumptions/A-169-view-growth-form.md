# A-169: the growth form of the economics view, RGDP10 plus CPI10

Updated: 2026-11-21. Code: branch econ/view-engine-cmd. Model document: `docs/models/economics-view-engine.md`.

## Choice
Growth_A(t) = median RGDP10 + median CPI10 of the survey of t. Nominal. Unit: percentage points per year.
The reader of Fundamental Research maps growth_change to G of A-001 only. It never maps to revenue growth or to the shift of the growth path.

## Where used
| Model | File | Parameter name |
|---|---|---|
| economics view engine | `rust/src/macro_test/view.rs` | `Level::growth_points()` |
| terminal growth (reader, planned) | `docs/assumptions/A-001-terminal-growth.md` | G |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| RGDP10 + PCE10 | PCE10 starts in 2007:Q1. Design rule 1.7 keeps it as a description only. A-001 uses PCE inflation in G, and CPI10 is CPI. The gap is a limit. |
| Product (1 + real)(1 + inflation) - 1 | The sum is the usual approximation (design rule 1.4). |

## Why this choice
The design fixes it before any run (rules 1.4, 1.5 and 3.2). The mapping to G follows the confirmation of the Head of Fundamental Research (rule 3, 2026-11-21).

## Effect on the result
Not known. No run exists. L moves only where G binds (G below the 10-year zero yield; design rule 3.2).

## Source
`analysis/macro/2026-11-21-view-engine-design.md`, rules 1.4, 1.5, 1.7 and 3.2, frozen at 3982324.

## Who decided and when
The Head of Economics, in the design (2026-11-21); the mapping confirmed by the Head of Fundamental Research (2026-11-21).

## When to examine it again
After the first run of record, or when A-001 changes the form of G.
