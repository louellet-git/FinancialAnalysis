# Model: algo engine and its paper broker (`fa algo`)

Updated: 2026-11-25. Commit: not yet made.

## Purpose
`fa algo` runs one rule on live quotes (or a replay) with one engine. It tests the pipe: data, decision, order, fill, journal.
The rule is `demo`, a mean reversion on one-minute closes. It is not a strategy with an edge. No result is used for a decision.
With `--broker alpaca-paper --send` the engine also sends LIMIT orders of 1 share to the Alpaca PAPER account. Without `--send`
it is a dry run: it prints and journals the order that it would send.

## Inputs
| Input | Source | Note |
|---|---|---|
| Quote | Alpaca data host, latest quote, feed IEX (free plan) | IEX is not the NBBO. A quote older than 10 seconds is skipped |
| Market clock | `GET /v2/clock` on the paper host constant | The run ends at once when the market is closed. If the clock cannot be read, the run does not start |
| Account, positions, orders | Paper host, only with `--send` | Read before each order |
| 20-day dollar volume | Alpaca daily bars (IEX volume) | Only for the liquidity limit |
| Keys | Data: `APCA_API_KEY_ID`, `APCA_API_SECRET_KEY`. Orders: `ALPACA_PAPER_KEY_ID`, `ALPACA_PAPER_SECRET_KEY` only | On the VM both pairs can hold the same paper key. The order code reads only its own pair |

## The simulator
It fills in memory: a buy at the ask, a sell at the bid of the first input 250 ms after the decision, fee 0.005 USD a share (A-155).
It has no partial fill, no queue, no market impact. The loss stop is approximate.

## The paper broker (`rust/src/trading/algo/paper.rs`)
- Host: the constant `https://paper-api.alpaca.markets`. No variable changes it. No redirect is followed.
- Before the first order: the account check. The Alpaca account object has no field "paper". The check is the host constant (a live key does
  not work there) plus status `ACTIVE` or `PAPER_ONLY`, no block, USD. The run log says "paper: yes" or "paper: no", never the account number.
- The sleeve stop (10% below the peak) refuses a run at its start. Orders: limit, day, 1 share, long only, no margin, no option (A-156, A-157). The paper limits of `chiefs/louellet-git/quality/proposal-sleeve-limits.md`
  are the constant `LIMITS`; a limit that cannot be checked refuses the order.
- Stops (section 8, control 4): the kill file `data/processed/algo/STOP`, the 2% daily loss, a broker stop (3 API errors) and the end time
  (`--until`) cancel all open orders, KEEP the position, write the log line (position kept: quantity and average entry price, no account number)
  and send no new order. A person closes a position. The run prints "position kept: N shares; a person closes it". The next run refuses to start
  while the account holds the symbol. The simulator keeps its own flatten, for its own P&L only: it never touches the broker position.
- Journal: `data/processed/algo/<date>/demo-orders.jsonl` (git ignores it): the intent before each call, the broker reply, and a `paper_pair` line with the
  order, the broker fill and the simulator fill of the same decision.
- Host machine and the clock: order runs only on the laptop or the VM, never in a cloud session.

## Limits of use
- A paper fill is not a live fill: no queue, no market impact, no regulatory fee, no depth. IEX quotes are not the NBBO.
- Step A: SPY shares only. Options are not allowed (section 8, control 6).
- The sleeve capital is the equity of the paper account. The "write-up" limits (exit price before entry, spread and fees in a write-up) are for a person, not for this code.
- A run is a pipe test and a cost measurement. It does not show an edge.

## Backtest: none applies, because the demo rule is a pipe test with no claim of return, and the paper fills are a forward measurement of costs.

## Records
| Record | Name | Value |
|---|---|---|
| A-155 | Simulator fill rule | ask or bid, 250 ms, 0.005 USD a share |
| A-156 | Order caps | 1 share, 6 a minute, 10 a day, 30 s time to live |
| A-157 | Limit price | at the touch, day order |

## Code and command
`rust/src/trading/algo/` (`engine.rs`, `paper.rs`, `cli.rs`). `fa algo run --strategy demo --symbol SPY --broker alpaca-paper [--send]`.
Tests: `cargo test --manifest-path rust/Cargo.toml --locked algo`.
