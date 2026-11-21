# A-145: no dividend yield in the model price of the option bots

Updated: 2026-11-21. Commit: not yet made.

## Choice
The dividend yield of SPY and QQQ is 0. `bs_price` and `bs_delta` have no dividend input
(`rust/src/trading/pricing.rs:41-71`). The forward of the model is spot times exp(rate * T), with no
dividend. The model option is European: no early exercise, no assignment.
This choice is not written as a decision in any file of the repo. The code hides it. The leaderboard lists
it as a limit: "early assignment, dividends and real expiry calendars before 2022 are not modeled"
(`trading/research/LEADERBOARD.md`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Option bot backtest (`docs/models/option-bot-backtest.md`) | `rust/src/trading/pricing.rs` | `bs_price`, `bs_delta` (no `q`) |
| Paper trader (model legs) | `rust/src/trading/paper.rs` | `price_signals` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| A yield input for each ETF (a dividend of SPY or QQQ) | Not known. No record |
| Forward from put-call parity | The backtest has no option quote to take it from |

## Why this choice
Not known. The Python pricer had the same form. Source: author choice, no study.
The repo has an estimate of an implied dividend for single stocks (`trading/research/arb-01-chain-check-2026-11-19.md`,
"Implied dividend": IBM 0.9% to 2.3%, JNJ 1.7% to 2.1% from 120 days). It has no estimate for SPY or QQQ.

## Effect on the result
Not known. No run with a yield is on file. The sign is known: with a positive yield, the model gives the
calls too high a price and the puts too low a price. A call debit spread on a 30-session expiry is
the most exposed (s08, s14, s14b). No number.

## Who decided and when
Not known. Python code, then the Rust port (commit e692700, 2026-11-19).

## When to examine it again
Before a trade of a bot with an expiry of more than 5 sessions. When a real chain history of SPY or QQQ
exists. Report to a person or a review: decide whether to add a yield.
