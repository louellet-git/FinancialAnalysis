# A-016: guard of the share count in the market value of a list

Updated: 2026-11-21. Commit: 90ed737 (base).
Model document: `docs/models/share-basis.md`.

## Choice
`fa dcf attribute` weights a list by the market value at D1 = price x shares. The share count is guarded
(`guarded_shares`). The reference count is the count at the end of the period (`shares_outstanding`),
else the count of the year before.
- Count from 0.5 to 2 times the reference (`SHARES_RATIO`): no note.
- Count from 0.01 to 100 times the reference (`SHARES_SCALE`) but outside 0.5 to 2: the count stays, with a note.
  Reason in the code: a split after the reference date (BKNG 25 for 1) or one class of several gives 2 to 100.
- Count outside 0.01 to 100 times the reference: a scale error of the filer (the unit is wrong, 1000 times).
  The reference count replaces it.
The strategy `value-rank-weekly` uses the same band 0.01x to 100x as a data check (A-009, item 5).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Attribution of a list | `rust/src/dcf/auto_attribute.rs` | `SHARES_RATIO` (0.5, 2.0), `SHARES_SCALE` (0.01, 100.0), `guarded_shares` |
| Weekly trade rule | `rust/src/portfolio/strategies.rs` | the same band as a data check (A-009, item 5) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| No guard | A filer that gives the count in the wrong unit would give a market value 1000 times too large |
| Narrow band (0.5 to 2) as a replacement | A true split or a share class gives a ratio from 2 to 100 and is a true count (code comment) |

## Why this choice
The code comment above. The band limits (0.01, 100, 0.5, 2) have no other source in any file read.
The guard is not a split correction: the splits are on the basis of A-017, applied before the guard.

## Effect on the result
No sensitivity number is known. The guard changes the weight of a ticker in the list change only.

## Who decided and when
Not known by name. Code of `fa dcf attribute` (ticket `T-20261120-valuation-move-attribution`, 2026-11-20).

## When to examine it again
When the table of share classes exists (shares by class). When a flagged ticker
is a true count that the guard replaced.
