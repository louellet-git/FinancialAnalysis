# Model: paper trader of the option bots

Updated: 2026-11-21. Commit: not yet made.

## Purpose
WARNING: `fa trade paper` never sends an order to a broker. The account is a simulation. The only order code in
the repo is the paper broker of `fa algo` (`rust/src/trading/algo/paper.rs`, `docs/models/algo-engine.md`): limit orders
to the Alpaca PAPER account only, under the controls of `docs/QUALITY-STANDARDS.md` section 8. This command does not use it. The fills are model fills (`rust/src/trading/paper.rs`). Only the entry price of a new signal can come from
a real quote, and that quote is about 15 minutes old.

The command runs each bot again from a warm-up window, on the data up to now. It shows the paper account,
and the entry price of the signals that the bot queued on the last bar. It is a forward test: the bars
after the paper start are bars that the bot did not see when it was designed. It is for the two owners
of the repo, to watch a bot before any real trade.

## Inputs
| Input | Source | Note |
|---|---|---|
| Bars and volatility indexes | data cache; refreshed from Yahoo unless `--no-refresh` | Same loader as the backtest (`docs/models/option-bot-backtest.md`) |
| Live option chain | None now. `Chains::get` refuses (T-20261121-cboe-off) | The Cboe client is removed. Use `--no-live` |
| Paper start | `data/processed/trading/paper/<bot>.json`, field `paper_start` | The first run saves it: `--start D` or today (New York date) |
| Clock | System time, converted to New York time (`Now`, `paper.rs`) | |
| Account rules | `RiskConfig::default()`, `--equity`, `--max-risk`, `--risk` | No option for the commission: 0.65 |

## Method (short)
1. Read or save the paper start of the bot (`paper.rs`).
2. Load the bars from a warm-up window before the start: 2 years for daily bots, 120 days for hourly
   bots, 30 days for 5-minute bots (`paper.rs`). The bot start date, if any, sets a later limit.
3. Run the backtest engine on these bars. An entry that would fill before the paper start is dropped
   (`engine.rs`, `trade_start`). So the account starts empty at the paper start.
4. Show the equity, the open positions, the last 5 closed trades and the rejections.
5. Price the entries that the bot queued on the last bar (`paper.rs`). A leg that the live chain lists
   gets the chain bid and ask (long at the ask, short at the bid). A leg not in the chain gets the model
   price at the time now. With `--no-live`, all legs get the model price at the last bar close.
   The `source` text of each signal says which one was used.
6. Append a snapshot to the state file and keep the last 500 (`paper.rs`).

## Outputs
A text report for each bot. A state file for each bot (not in git). The trades and the open positions in
the report come from the model price (the engine), not from the live chain. Only the line "SIGNAL (next bar)"
can come from the chain.

## Assumptions
The model price of the legs has the records of `docs/models/option-bot-backtest.md` (A-140 to A-148).
| Record | Name | Value | One line |
|---|---|---|---|
| A-149 | Paper signal price and warm-up | chain bid and ask, model for a missing leg; warm-up 2 years, 120 days, 30 days | Rule for the live price and the data window |

## Limits
- The account does not use the live price. A fill in the account is a model fill at the next bar open.
  The live price shows only what the signal would cost now.
- The live chain is refused: the Cboe page prohibits automatic downloads. The paper trader has no Alpaca adapter yet.
- The signal price is not the next-bar-open price that the engine will use.
- The run depends on the schedule of the person. The engine runs again from the bars, so a missed run is not a missed trade.
  But the live signal price of the missed run is lost.
- Warm-up is a window, so a bot with a long indicator can have too few bars at the start.
- The state file holds `paper_start` and snapshots only. The account is recomputed at each run.
- Backtest: none applies, because the paper trader is a forward test. The backtest is `docs/models/option-bot-backtest.md`.
- A result is a forward test of a few weeks of data at most. It does not show an edge.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/trading/paper.rs`, `chain.rs`, and the engine in `engine.rs` |
| Parameter file | None. State: `data/processed/trading/paper/<bot>.json` |
| Command | `fa trade paper [BOT]... [--start D] [--no-refresh] [--no-live] [--equity N] [--max-risk N] [--risk F]` |
| Tests | `cargo test --manifest-path rust/Cargo.toml --locked` (the tests use a fixed chain and no network) |

## Review status
Not reviewed by the steps of `docs/QUALITY-STANDARDS.md`. A first chain check of the live data:
`trading/research/arb-01-chain-check-2026-11-19.md` (stored chains of IBM and JNJ, not of SPY).
