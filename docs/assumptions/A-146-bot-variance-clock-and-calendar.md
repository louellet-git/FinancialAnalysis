# A-146: variance clock and calendar of the option bots

Updated: 2026-11-21. Commit: not yet made.

## Choice
Time to expiry T is a variance time in years, not calendar time (`rust/src/trading/pricing.rs`):
- 1 trading year = 252 sessions of 390 minutes (09:30 to 16:00, New York time) (`SESSION_MINUTES`, `TRADING_DAYS`, `pricing.rs`).
- One overnight (also a weekend or a holiday weekend) has a weight of 0.25 of a session of 390 minutes
  (`OVERNIGHT_WEIGHT`, `pricing.rs`). So the overnight is 20% of a close-to-close day (0.25 / 1.25).
- The session has a U shape. The first 30 minutes have a weight of 2.0. The last 30 minutes have a weight of 1.5.
  The middle weight is lower. The weights add up to 390 (`OPEN_W`, `CLOSE_W`, `EDGE`, `cum_weight` in `pricing.rs`).
- A close-to-close day is exactly 1/252 (test `variance_clock` in `pricing.rs`).
Calendar (`rust/src/trading/calendar.rs`):
- Business days are Monday to Friday. There is no holiday calendar. A holiday counts as a session.
- An option exists for each weekday expiry, with no date limit (`Ctx::expiry`, `engine.rs`).
  Real daily SPY and QQQ expiries exist from 2022 (`LEADERBOARD.md`: "real expiry calendars before 2022 are not modeled").
- The wall time is New York time with a US daylight saving rule (`calendar.rs`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Option bot backtest (`docs/models/option-bot-backtest.md`) | `rust/src/trading/pricing.rs` | `year_fraction`, `cum_weight` |
| Option bot backtest | `rust/src/trading/calendar.rs`, `engine.rs` | `busday_count`, `busday_offset`, `Ctx::expiry` |
| Paper trader | `rust/src/trading/paper.rs` | `Now`, `ny_offset` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Calendar time (days / 365) | The leaderboard says the options did not decay overnight. It also says the volatility of 0DTE after the morning was too high |
| A holiday calendar | Not known. No record |

## Why this choice
The variance clock was added after the first leaderboard of 2026-10-11 (`trading/research/LEADERBOARD.md`,
"Harness changes"), with "overnight about 20% of daily variance". The source of the 20% and of the weights 2.0
and 1.5: author choice, no study.

## Effect on the result
The change from calendar time to the variance clock cut the backtest return of s05 and s18 by about half
(`LEADERBOARD.md`; s05 from +168% to +84%). No number for the weights or for the holiday rule.

## Who decided and when
Not known by name. Commit ec78ccf (2026-10-11), "variance-clock pricing". Rust port: e692700 (2026-11-19).

## When to examine it again
When a bot is a candidate for a trade (0DTE bots depend most on it). When a holiday calendar is added.
