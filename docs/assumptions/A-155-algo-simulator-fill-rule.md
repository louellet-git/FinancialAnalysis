# A-155: fill rule of the `fa algo` simulator

Updated: 2026-11-25. Commit: not yet made.

## Choice
A buy fills at the mid plus 1.0 times the half spread (the ask). A sell fills at the bid. Latency 250 ms (the order
fills on the first input that is 250 ms or more after the decision). Fee 0.005 USD a share. No partial fill, no queue.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Algo engine | `rust/src/trading/algo/cli.rs` | `--spread-share` (1.0), `--latency-ms` (250), `--fee` (0.005) |
| Algo engine | `rust/src/trading/algo/engine.rs` | `Config.spread_share`, `Config.latency_ms`, `Config.fee` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Fill at the mid (`--spread-share 0`) | Too good: a market order pays the spread |
| Random partial fills like the Alpaca paper account | The simulator is for a fixed answer on a replay |

## Why this choice
The Alpaca paper account fills a marketable order at the ask or bid (`chiefs/louellet-git/quant/alpaca-paper-facts-2026-11-25.md`, section 1).
The simulator copies that rule, so the journal can compare the two fills. The latency and the fee are conventions, not measured.

## Effect on the result
Not measured. With `--broker alpaca-paper --send` the journal shows the broker fill next to the simulator fill: this is the measurement.

## Who decided and when
Quant, 2026-11-21 (PR 131); kept 2026-11-25.

## When to examine it again
After the first paper fills: compare the two fills in `data/processed/algo/<date>/demo-orders.jsonl`.
