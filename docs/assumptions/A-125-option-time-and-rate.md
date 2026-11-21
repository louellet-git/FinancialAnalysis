# A-125: time to expiry and rate in the option checks and the option history

Updated: 2026-11-21. Commit: not yet made.

## Choice
- Time: `t` = calendar days from the snapshot date to the expiry, divided by 365. No clock time, no day-count by instrument.
- Rate: `--rate R` (one flat decimal rate), else the Treasury par curve of the snapshot date, linear in `t`, flat outside the first and last tenor.
- Discount factor: `df = exp(-r t)`. The curve gives a par yield and the code uses it as a continuous rate.
- The rate is asked for `max(t, 1/365)` years (`option_arb.rs`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Static arbitrage check (`docs/models/options-static-arbitrage-check.md`) | `rust/src/market/option_arb.rs` | `t`, `r`, `df` |
| Option history | `rust/src/market/option_history.rs` | `t`, `df` |
| Rate function | `rust/src/market/options_cli.rs` | `rate_fn`, `interpolate_curve` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Time in minutes from the snapshot time | Written in the ticket of the Heston fit, but the files have no clock time. See A-136. |
| A zero-coupon or SOFR curve in place of the Treasury par curve | Not known. |

## Why this choice
Convention. Day count 365 and the par yield as a continuous rate: author choice, no study. The reason for the Treasury curve: not known.

## Effect on the result
It moves `df` and so the forward, the implied volatility and the parity bound. The snapshot has no clock time, so for a short series the error in `t` is large.
The implied dividend of the 1-day IBM series of 2026-11-19 is -55%, so the output is not usable for a series that short.
No sensitivity number to `r` is recorded.

## Who decided and when
Not known. The tickets do not state the day count or the curve.

## When to examine it again
When a snapshot has a time stamp (Alpaca `<date>T<hhmm>`); when a rate source of the rates area is chosen for the options.
