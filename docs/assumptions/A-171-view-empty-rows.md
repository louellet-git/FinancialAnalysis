# A-171: EMPTY rows for windows with no release

Updated: 2026-11-21. Code: branch econ/view-engine-cmd. Model document: `docs/models/economics-view-engine.md`.

## Choice
A window with no release has erp_change and growth_change EMPTY and the source "no release". The command does not write 0. The same holds when no survey is out at D1.

## Where used
| Model | File | Parameter name |
|---|---|---|
| economics view engine | `rust/src/macro_test/view.rs` | `view_row()`, `SOURCE_NONE` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Write 0 | 0 claims "no change in the view". The survey did not publish a new value (design section 4). |

## Why this choice
The reader of Fundamental Research reports that plug as "not covered by the economics view" (design rules 2.3 and 3.5).

## Effect on the result
Not known. About 2% of the weekly windows have a release (design rule 2.6, an estimate, not a count from data). So most windows are EMPTY.

## Source
`analysis/macro/2026-11-21-view-engine-design.md`, rules 2.3, 2.4, 2.6 and 3.5, frozen at 3982324.

## Who decided and when
The Head of Economics, in the design (2026-11-21).

## When to examine it again
When layer B or a second source gives a value between releases.
