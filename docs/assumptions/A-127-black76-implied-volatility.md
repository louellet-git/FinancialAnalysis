# A-127: Black-76 as the implied volatility model, for American options

Updated: 2026-11-21. Commit: not yet made.

## Choice
The implied volatility is the Black-76 volatility (a European model with the forward F): `C = df (F N(d1) - K N(d2))`.
It is used for American stock and ETF options and for European indexes. A bisection finds it between 1e-4 (`MIN_VOL`) and 5.0 (`MAX_VOL`, 500%), to a width of 1e-12, with at most 200 steps.
No volatility is returned when the price is at or below the discounted intrinsic value (+1e-12) or at or above the upper bound.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Implied volatility (`docs/models/options-static-arbitrage-check.md`) | `rust/src/market/option_iv.rs` | `black76`, `implied_vol`, `MIN_VOL`, `MAX_VOL` |
| Static arbitrage check | `rust/src/market/option_arb.rs` | the marker (A-122) |
| Option history | `rust/src/market/option_history.rs` | `usable`, `implied_volatility` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| American model (tree or PDE) | Not known. Not in the code. |
| Volatility of the data source (Alpaca `iv`) | The index chains have none; the code keeps a given value and fills the empty ones only (`option_history.rs`). |

## Why this choice
The header of `option_iv.rs` states it: a first version with known errors, `docs/HIDDEN-ASSUMPTIONS.md` #236. The early-exercise premium and the dividend make the volatility of an in-the-money American option too high.
So the history and the fill give a volatility only for out-of-the-money quotes and quotes within 1% of the forward. The 1% band: author choice, no study. The bounds 1e-4 and 5.0: author choice.
Index options are European, so Black-76 is the right model for them (#280).

## Effect on the result
The size of the error is not sized. It marks quotes in the check (A-122) and fills the volatility of the stored history. The marker of the check does not use the 1% band: it can mark deep in-the-money American quotes (`render` note, `option_arb.rs`).

## Who decided and when
Not known. The entries #236, #237 and #280 of `docs/HIDDEN-ASSUMPTIONS.md` describe it; no person and no date are named.

## When to examine it again
When an American model is added; when a result depends on the volatility of an in-the-money quote; when a volatility above 5.0 is needed.
