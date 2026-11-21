# A-202: the cost of debt is taken at 10 years

Updated: 2026-11-21. Commit: not yet made.

## Choice
The cost of debt uses the HQM spot yield at 10 years (`DEFAULT_MATURITY = 10`). The
`kd_spread` of the verdict is this cost of debt minus the Treasury 10Y par yield of the same date.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Cost of debt | `rust/src/bonds.rs` | `DEFAULT_MATURITY`, `corporate_yield()`, `spread_over_10y()` |
| Verdict | `rust/src/verdict_rates.rs` | `credit_context()` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| 7 or 20 years (HQM has 1 to 100) | The engine `rf` is the Treasury 10Y; a spread over the 10Y must use the same horizon |
| HQM par yield | The Treasury par file has 2, 5, 10 and 30 years only, and it is not read |

## Why this choice
The engine prices debt as `kd = rf + kd_spread` with `rf` the Treasury 10Y, as before.
The same maturity keeps the definition. The reason does not use the price. Limits: the HQM
rate is a spot rate (zero rate) and a monthly average; the Treasury 10Y is a par yield of one
day. The basis between a spot and a par yield is small at 10 years (a few basis points).
The latest HQM month can be up to 70 days old; an older month gives an error.

## Effect on the result
At the date 2026-11-18, the HQM 10Y of the latest month is used. A different maturity
changes the cost of debt by the slope of the HQM curve (a few tenths of a point from 7 to 20 years).

## Who decided and when
The Technology head, 2026-11-21, as the unchanged default of the old `fa bonds` code.

## When to examine it again
If the engine takes a `rf` of a different maturity.
