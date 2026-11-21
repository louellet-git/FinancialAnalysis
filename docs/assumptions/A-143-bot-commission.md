# A-143: commission of the option bots

Updated: 2026-11-21. Commit: not yet made.

## Choice
$0.65 for each contract, on each side (`rust/src/trading/engine.rs:51`, `RiskConfig::default`).
`fa trade backtest --commission N` changes it (`rust/src/trading/cli.rs:116`).
- The commission of a structure is $0.65 times the legs of one unit (`contracts_per_unit`) times the units
  (`engine.rs:452`). A $1 vertical costs $1.30 at entry. A round trip of it costs $2.60.
- The entry commission is paid at the fill. The exit commission is paid at an exit by rule. A position
  that expires pays no exit commission (`engine.rs:478-486`, `:640`).
- The sizing counts both commissions in the maximum loss of the trade (`engine.rs:547`, `2.0 * commission`),
  also for a position that expires.
- `fa trade paper` has no commission option: it uses the default of 0.65 (`rust/src/trading/paper.rs:126-127`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Option bot backtest (`docs/models/option-bot-backtest.md`) | `rust/src/trading/engine.rs` | `RiskConfig.commission_per_contract` |
| Option bot backtest | `rust/src/trading/cli.rs` | `--commission` (default 0.65) |
| Paper trader | `rust/src/trading/paper.rs` | `RiskConfig::default().commission_per_contract` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| A different broker rate (zero commission, or a rate with a fee for each order) | Not known. No record |
| A regulatory and exchange fee on top | Not known. The model has no such fee |

## Why this choice
Author choice, no study. $0.65 for each contract is a usual retail rate (general knowledge, not a file of the repo).
No file names the broker of the owner. The notes use it as a fact (`s01_orb_0dte.md`: "$0.65 x 2 legs x 2
sides = $2.60 per $1-wide round trip"). Source: not known. The fee schedule of the real broker: not known.

## Effect on the result
Large for $1-wide spreads on a $2,000 account. For s11: commissions off only moves -20.7% to -20.4%, and with
the spreads off the result is +98.3% (`trading/research/s11_vrp_condor.md`, "Cost decomposition"). The leaderboard
says that several real effects "are too small to survive $1-wide option spreads after $0.65/leg commission"
(`trading/research/LEADERBOARD.md`). You can run `fa trade backtest --commission 0` to see the effect for a bot.

## Who decided and when
Not known. Python code, then Rust port (commit e692700, 2026-11-19).

## When to examine it again
When the owner chooses a broker, or the broker changes the fee. Before any real trade.
