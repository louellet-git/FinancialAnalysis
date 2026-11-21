# A-132: spread filter of the Heston and Bates fit (1 vol point, 5 for VIX)

Updated: 2026-11-21. Commit: not yet made.

## Choice
A quote stays in the fit when the ask volatility minus the bid volatility is at most 0.01 (1 volatility point). For a symbol that starts with VIX the limit is 0.05.
Only out-of-the-money quotes with bid above 0 and ask above bid are tested. A series with fewer than 5 quotes (`MIN_QUOTES`) leaves the fit with all its quotes.
`--max-spread V` replaces the value (decimal vol).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Heston, Bates | `rust/src/volmodels/chain.rs:18-26` | `Filters.max_spread` |
| Heston, Bates | `rust/src/volmodels/chain.rs:14`, `105-113` | `MIN_QUOTES`, the test of the spread |
| Heston, Bates | `rust/src/volmodels/cli.rs:74` | `--max-spread` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| No spread filter (weights only) | Not known. |
| A wider limit for stocks | Not known. A stock chain has few quotes under 1 point. |

## Why this choice
Ticket `T-20261120-vol-1-heston-bates`, data section (from `analysis/priors/volatility_study_2026-11-20.md`, A.7): "spread at most 1 vol point (5 for VIX options)".
The study was not read for this record; the number comes from the ticket.
The minimum of 5 quotes: author choice, no study.

## Effect on the result
It sets the number of quotes. IBM 2026-11-19: 40 of 1700 quotes stay, and only 4 LEAPS series (393 to 757 days), so the short end is absent (`docs/models/heston-bates.md`, Limits).
The fit says nothing about the expiries that the filter removes. No run with another limit is recorded.

## Who decided and when
Owner ticket of 2026-11-20 (vol-1).

## When to examine it again
When a chain has too few quotes for a fit; when the fit is run on a stock with a short expiry.
