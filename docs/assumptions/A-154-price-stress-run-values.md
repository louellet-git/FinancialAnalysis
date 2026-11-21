# A-154: model price stress values of the backtest

Updated: 2026-11-21. Commit: not yet made.

## Choice
Runs of s05_ic_0dte with a worse or a better model price: spread multiplier 2, skew multiplier 1.5 and 0.5,
and the mid fill (no spread, a reference only). Defaults: 1, 1, false. The skew multiplier scales the slope
and the curve. The floor 0.6 and the cap 2.5 of the multiplier do not scale. The knobs are process-wide values
set once by `set_price_knobs`.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Variance premium test | `rust/src/trading/pricing.rs` | `set_price_knobs`, `SPREAD_MULT`, `SKEW_MULT`, `MID_FILL` |
| Variance premium test | `rust/src/trading/cli.rs` | `--spread-mult`, `--skew-mult`, `--mid-fill` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Spread x3 or more | Not known |
| A spread based on real quotes | No real quotes in the repo. It is stage 2 of the ticket |

## Why this choice
The ticket text names "spread x2, skew +-50%" and a fill at the bid or ask
(`PLANNING/board/T-20261121-alpha-1-pipeline.md`). The default fill is already the worse side. So the
mid fill was added as a better reference. The sizes are a choice of the ticket, with no study.

## Effect on the result
Spread x2: the OOS mean trade P&L falls from $19.42 to $4.91 and t(P&L) from 1.35 to 0.52. Skew x1.5 and
x0.5 change t(P&L) to 1.30 and 1.18 (`trading/research/alpha-1-stage1.md`, stage 1b). Command:
`fa trade backtest --bot s05_ic_0dte --spread-mult 2 --trials 20`.

## Who decided and when
The ticket owner request of 2026-11-21 (the file `PLANNING/board/T-20261121-alpha-1-pipeline.md`).
The person who chose the values: not known.

## When to examine it again
When real chain copies give the true half-spread and skew (stage 2).
