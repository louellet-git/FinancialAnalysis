# A-142: spread and fill rule of the option bots

Updated: 2026-11-21. Commit: not yet made.

## Choice
- Half-spread of one leg = 0.5% of the mid price, at least $0.01, at most $0.05 (per share)
  (`rust/src/trading/pricing.rs`). Bid = mid minus half-spread (not below 0). Ask = mid plus half-spread.
- Entry: long legs at the ask, short legs at the bid. Exit by a rule: long legs at the bid, short legs at
  the ask ("liquidation value") (`rust/src/trading/engine.rs`). So each round trip crosses the whole spread.
- An entry that a bot queues on a bar fills at the next bar open, at the model price of that open (`engine.rs`).
- An exit rule is tested on the bar close, and it fills at that close (`engine.rs`).
- Expiry: settle at intrinsic value, no spread, no exit commission (`engine.rs`).
- An expired option has bid = ask = intrinsic value (`pricing.rs`).
- The equity curve marks open positions at mid (`engine.rs`), not at liquidation.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Option bot backtest (`docs/models/option-bot-backtest.md`) | `rust/src/trading/pricing.rs` | `SPREAD_REL`, `SPREAD_MIN`, `SPREAD_MAX` |
| Option bot backtest | `rust/src/trading/engine.rs` | `prices_at`, `fill_pending`, `check_exits` |
| Paper trader | `rust/src/trading/paper.rs` | `price_signals` (long at ask, short at bid) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Fill at mid | Not known. Mid is not a price that one can get. The rule is more cautious |
| Fill between mid and the far side (a limit order) | Not known. No record |
| Spread that depends on the time to expiry, the moneyness or the volatility | Not known |

## Why this choice
Author choice, no study. No file gives the source of 0.5%, $0.01 and $0.05. The notes say that real spreads are wider than $0.01 to $0.05 per leg in three cases:
when the price moves fast, near the close, and for the condor (`s01_orb_0dte.md`, `s05_ic_0dte.md`, `s11_vrp_condor.md`).
They say that a real 40-session condor fill is "mid minus a few cents" per leg. The floor of $0.01 is a large share of a cheap option (`s11_vrp_condor.md`).

## Effect on the result
For s11 (SPY, $2,000, costs on) the note gives: both commissions and spreads off: +98.3%. Spreads off only:
-19.0%. Commissions off only: -20.4%. Actual: -20.7% (`s11_vrp_condor.md`, "Cost decomposition"). The leaderboard
says that round-trip friction is about 18% of a $50 debit. For the other bots: not known.

## Who decided and when
Not known. Python pricer, then the Rust port (commit e692700, 2026-11-19).

## When to examine it again
When a measure of real fills exists (a bid and ask history, or paper fills; `s02_vwap_revert_0dte.md` asks
for "realized slippage on $1-wide ATM 0DTE spreads"). Before a trade of a bot that holds for a short time.
