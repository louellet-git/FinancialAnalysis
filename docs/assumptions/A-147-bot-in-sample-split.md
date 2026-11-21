# A-147: in-sample and out-of-sample split of the bot backtest

Updated: 2026-11-21. Commit: not yet made.

## Choice
The first 70% of the bars of a bot are in-sample. The last 30% are out-of-sample (OOS). The cut is by bar
count, in time order: `cut = floor(n * 0.7)`. A trade is in OOS if its entry time is at or after the cut bar
(`rust/src/trading/metrics.rs:91-100`). Default 0.7: `rust/src/trading/cli.rs:123`; option `--is-fraction F`.
This is one split in time. It is not a rolling walk-forward test. (The bot s20 refits its own model every 21
sessions, a different thing, `rust/src/trading/bots/s20_walkforward_model.rs`.)
The OOS statistics use the equity curve from the cut bar (the first OOS bar is the start of the curve).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Option bot backtest (`docs/models/option-bot-backtest.md`) | `rust/src/trading/metrics.rs` | `summarize(.., is_fraction)` |
| Option bot backtest | `rust/src/trading/cli.rs` | `--is-fraction` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Rolling walk-forward windows | Not known. No record |
| A split by date, equal for all bots | Not known. The bots have different bar sets (5-minute, hourly from 2023, daily from 2005) |
| Cross-validation with a purge | Not known |

## Why this choice
Author choice, no study. The doc comment says "the first 70% of the bars are in-sample by default" and
"a bot is TUNED on in-sample data only" (`metrics.rs:1-4`). The leaderboard says the OOS part is "never used for
tuning by the bot itself" but that "several bots admit to peeking or to running more than the ~6 allowed variants"
(`trading/research/LEADERBOARD.md`). So the rule is a rule of work, and the code cannot enforce it.

## Effect on the result
The split changes which bars are called OOS, not the trades. No number for another fraction. The cut of a
bot with hourly bars from 2023 is a short OOS span, with few trades. The notes call a count of 17 or fewer noise.

## Who decided and when
Not known. Python code, then the Rust port (commits b57fe03 and e692700, 2026-11-19).

## When to examine it again
Before a decision on a bot. When a variance count of the runs of `fa trade backtest` exists (the run log of
`trading/runs/README.md` covers `fa portfolio backtest` only). When the cache has more bars.
