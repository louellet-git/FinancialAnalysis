# A-170: the release date sets the step of the economics view

Updated: 2026-11-21. Code: branch econ/view-engine-cmd. Model document: `docs/models/economics-view-engine.md`.

## Choice
The survey of t is the latest first-quarter survey with a release date on or before t. The layer is a step function of the release date, not of the survey quarter. A window D1 to D2 is covered when a release date falls after D1 and on or before D2.

## Where used
| Model | File | Parameter name |
|---|---|---|
| economics view engine | `rust/src/macro_test/view.rs` | `survey_at()`, `view_row()` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| The survey quarter | The answer deadline is before the release. The release date is the date when the public knows the value (design section 2). |

## Why this choice
The layer is then point in time by the public release date. The release date of the 2026 Q1 survey is 2026-03-06, not the first date, because a government shutdown delayed it (design record R2).

## Effect on the result
Not known. A window that holds a release gets one full year of change in one step. The date can move a window in or out of the covered set.

## Source
`analysis/macro/2026-11-21-view-engine-design.md`, rules 1.1, 1.6 and 2.2, section 2, record R2, frozen at 3982324.

## Who decided and when
The Head of Economics, in the design (2026-11-21).

## When to examine it again
When the Head of Technology finds a release date that the release calendar does not confirm, or when SPF changes its release practice.
