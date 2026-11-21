# A-149: signal price and warm-up window of the paper trader

Updated: 2026-11-21. Commit: not yet made.

## Choice
Signal price (`rust/src/trading/paper.rs:257-290`): for each leg of a signal that the bot queued on the last bar:
1. The live Cboe chain has the leg (exact strike and expiry): use its bid and ask. A quote needs bid at or
   above 0, ask above 0 and ask at or above bid (`rust/src/trading/chain.rs:quote`).
2. The chain has no such leg: use the model price at the time now (A-140 to A-146).
3. `--no-live`, or the download failed: the model price at the last bar close.
Entry = long legs at the ask, short legs at the bid (the same rule as A-142). The `source` text names the case.
The paper account itself does not use these prices: it uses the model price of the engine.

Warm-up window before the paper start (`paper.rs:164-176`): 2 years for daily bots, 120 days for hourly
bots, 30 days for 5-minute bots. The bot start date can move the window later (`paper.rs:306`).
Paper start: the date of the first run (New York date), or `--start D`, saved in the state file (`paper.rs:233`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Paper trader (`docs/models/paper-trader.md`) | `rust/src/trading/paper.rs` | `price_signals`, `warmup_start` |
| Paper trader | `rust/src/trading/chain.rs` | `Chain::quote`, `URL` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Fill the paper account with the live quote | Not known. The account runs from bars in the cache, so it has no quote at the fill |
| Yahoo option chain | Needs a cookie and a crumb and answered with HTTP 429 (tested 2026-11-19, `chain.rs:10-11`) |
| A longer warm-up for each bot | Not known. No record |

## Why this choice
The source of the chain: Cboe delayed quotes, free, no key (`chain.rs:1-8`). The reason for bid and ask:
same as A-142. The warm-up lengths: author choice, no study. A bot with an indicator longer than the window
(for example the 252-day percentile of s10, or SMA(200)) can lack bars: not known if this was checked.

## Effect on the result
Not known. The live price only informs the person. Compare the model price and the live price on a signal:
the `source` text and the entry value are in the report and the state file. No stored comparison.

## Who decided and when
Not known by name. Commit e692700 (2026-11-19), "Trading: bars, live chain and paper trading in Rust".

## When to examine it again
After the first weeks of paper signals: compare the model entry price with the live entry price of the
same signal. When Cboe changes the file.
