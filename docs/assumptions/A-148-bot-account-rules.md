# A-148: account and risk rules of the option bot backtest

Updated: 2026-11-21. Commit: not yet made.

## Choice
Default account (`rust/src/trading/engine.rs:41-57`, `RiskConfig::default`; the doc comment at `engine.rs:10`
says it is "the paper account of the owner"):
| Rule | Value |
|---|---|
| Starting equity | $2,000 (`cli.rs:115`) |
| Maximum loss for one trade | 3% of equity (`risk_per_trade`) |
| Maximum open positions | 3 |
| Sum of open maximum losses | 12% of equity |
| Daily loss limit (no new entry below) | 6% of the equity at the start of the day |
| Maximum units of one entry | 20 |
| Pattern day trader (PDT) rule | on: below $25,000, at most 3 same-day round trips in 5 sessions |
| Structures | defined risk only (`structures.rs:30`) |
`--max-risk N` replaces the 3%: a cap of N dollars at the starting equity, scaled down with equity. The open
risk limit is 3 times that fraction, at most 50% (`engine.rs:62-72`). Example: N = 250 gives 12.5% and 37.5%.
The maximum loss of a trade is the structure maximum loss plus both commissions (`engine.rs:547`).
An entry that does not fit is rejected, and the reason is counted (`engine.rs:520-592`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Option bot backtest (`docs/models/option-bot-backtest.md`) | `rust/src/trading/engine.rs` | `RiskConfig` |
| Option bot backtest | `rust/src/trading/cli.rs` | `--equity`, `--risk`, `--max-risk` |
| Paper trader (`docs/models/paper-trader.md`) | `rust/src/trading/paper.rs` | same options, default 2000 |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| $250 per trade (`--max-risk 250`) | Run as "Account B" in the leaderboard. Only s05 improves; most others get worse |
| A larger account (no PDT limit) | Not the account of the owner |

## Why this choice
The values model the paper account of the owner. The source of each number (3%, 3 positions, 12%, 6%, 20 units):
author choice, no study. The PDT rule is a US rule for margin accounts below $25,000 (a rule of the broker
system, not a file of this repo). The account size $2,000: not known where it is recorded.

## Effect on the result
Large, for the strategy choice: at the default $60 maximum loss most credit spreads do not fit, and the
bots s11 and s21 reject entries (`s11_vrp_condor.md`, `s21_bollinger_fade.md`). The leaderboard
(2026-10-11) shows that $250 per trade turns losers into -80% to -95% results (Account B).

## Who decided and when
Not known by name. "The paper account of the owner" (`engine.rs:10`). Commit b57fe03 and e692700 (2026-11-19).

## When to examine it again
When the real account size changes, or the owner picks a broker. Before a real trade.
