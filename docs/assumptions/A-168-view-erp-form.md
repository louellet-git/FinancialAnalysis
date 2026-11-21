# A-168: the ERP form of the economics view, STOCK10 minus GSW10

Updated: 2026-11-21. Code: branch econ/view-engine-cmd. Model document: `docs/models/economics-view-engine.md`.

## Choice
ERP_A(t) = median STOCK10 of the survey of t minus GSW10 (column SVENY10) at the release date. Unit: percentage
points per year. The command divides the change by 100 for the file.

## Where used
| Model | File | Parameter name |
|---|---|---|
| economics view engine | `rust/src/macro_test/view.rs` | `Level::erp_points()`, `levels()` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| STOCK10 minus BOND10 | BOND10 is a return before 2014 and a yield from 2014:Q1. It is a difference of two answers (design rule 1.3, description only). |
| STOCK10 minus BILL10 | Design rule 1.7 keeps it as a description only. |

## Why this choice
GSW10 is a Treasury price. It does not come from the stock prices of the attribution (design rule 5.1).
The design fixes the primary form before any run (rules 1.2 and 1.7). The findings do not change it after a run.

## Effect on the result
Not known. No run and no sensitivity number exist. The units of STOCK10 (return form not known) and of GSW10
(continuously compounded) may differ. See A-173.

## Source
`analysis/macro/2026-11-21-view-engine-design.md`, rules 1.2, 1.3, 1.7 and 5, frozen at 3982324.

## Who decided and when
The Head of Economics, in the design (2026-11-21), passed by the Head of Quality and Risk (PASS-WITH-NOTES, 3982324).

## When to examine it again
After the first run of record, or when the return form of STOCK10 is known, or when a variant (rule 1.7) differs a lot from the primary form.
