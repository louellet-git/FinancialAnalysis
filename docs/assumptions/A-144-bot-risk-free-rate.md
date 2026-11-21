# A-144: risk-free rate of the model price of the option bots

Updated: 2026-11-21. Commit: not yet made.

## Choice
The rate in Black-Scholes is the close of `^IRX` (the 13-week Treasury bill index of Yahoo, in percent)
of the trading day before the bar, divided by 100. A date with no value takes the last value. No `^IRX`
file, or no value yet: 0.04 (`DEFAULT_RATE`). Code: `rust/src/trading/data.rs`.
One rate is used for all expiries (no term structure). It is a continuous rate in `bs_price`
(`rust/src/trading/pricing.rs`).
The day before is on purpose: no look-ahead (`data.rs`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Option bot backtest (`docs/models/option-bot-backtest.md`) | `rust/src/trading/data.rs` | `MarketData.rate`, `DEFAULT_RATE` |
| Option bot backtest | `rust/src/trading/pricing.rs` | `VolSnapshot.rate`, `bs_price`, `bs_delta` |
| Paper trader (model legs) | `rust/src/trading/paper.rs` | `price_signals` (`md.rate[last]`) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Treasury par curve of the date for the expiry (the `fa rates` data) | Not known. No record. The backtest cache has no curve |
| A fixed rate for all dates | Not known |

## Why this choice
Author choice, no study. The only free daily short-rate series in the bar cache is `^IRX`. The code does not convert the index to a continuous rate
(not known if this was a decision).
The default 0.04 has no source in a file.

## Effect on the result
Not known. No run with another rate is on file. The rate only moves the discount and the forward. For an expiry of
5 sessions the effect on a price is small. For the 40-day condor (s11) and for the 30-session spreads (s08)
it is larger, but no number is known.

## Who decided and when
Not known. Python code, then the Rust port (commit e692700, 2026-11-19).

## When to examine it again
When a bot with a long expiry is a candidate for a trade. When the rate level changes much from 4%.
