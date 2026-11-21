# A-141: fixed skew of the model price of the option bots

Updated: 2026-11-21. Commit: not yet made.

## Choice
The volatility of a strike is the ATM volatility times a multiplier (`rust/src/trading/pricing.rs`):
- `z = ln(K / S) / (ATM * sqrt(T))`, limited to -6 to +6. T is the variance-clock time (A-146).
- multiplier = 1 + (-0.12) * z + 0.03 * z^2, then limited to 0.6 to 2.5.
- Constants: `SKEW_SLOPE` = -0.12, `SKEW_CURVE` = 0.03, `MULT_FLOOR` = 0.6, `MULT_CAP` = 2.5 (`pricing.rs`).
The same shape applies on each day and for each expiry. Puts (z below 0) cost more than calls.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Option bot backtest (`docs/models/option-bot-backtest.md`) | `rust/src/trading/pricing.rs` | `SKEW_SLOPE`, `SKEW_CURVE`, `MULT_FLOOR`, `MULT_CAP` |
| Strike choice by delta | `rust/src/trading/engine.rs` | `strike_for_delta` (delta uses this volatility) |
| Paper trader (model legs) | `rust/src/trading/paper.rs` | `price_signals` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Flat volatility (no skew) | Not known. No record |
| Skew fitted to a real chain (the repo has a smile fit, `docs/models/heston-bates.md`) | Not known. No historical chains of SPY or QQQ in the cache |
| Skew that depends on the day or on the expiry | Not known |

## Why this choice
Author choice, no study. No file gives the source of -0.12 or 0.03. The test `skew_and_spread_rules` in
`pricing.rs` only checks that puts are richer than calls. The notes of the bots call the skew "synthetic"
and "fixed" (`s10_vix_regime.md`, `s11_vrp_condor.md`): a real skew for a 28-session 32-delta strike "can be
much steeper" (`s11_vrp_condor.md`). The skew adds to the skew that VIX already has (A-140).

## Effect on the result
Not known. No run with another skew is on file. The credit-spread and condor bots (s05, s10, s11, s18, s21)
sell the wing, so their price depends on it.

## Who decided and when
Not known. The values came from the Python pricer (`pricing.rs`).
Rust port: commit e692700 (2026-11-19).

## When to examine it again
When a real SPY or QQQ chain history exists. Before a trade of a bot that sells or buys a wing.
Check `fa options check` and the Heston fit on a stored SPY chain for a real slope at the same delta.
