# A-136: time to expiry of the Heston and Bates fit (calendar days / 365)

Updated: 2026-11-21. Commit: not yet made.

## Choice
`t` = calendar days from the snapshot date to the expiry, divided by 365. A series with 0 days or fewer is dropped. The snapshot has no clock time, so a series of 1 to 2 days has a large error in `t`.
`--expiry-max DAYS` drops series beyond a limit.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Heston, Bates (`docs/models/heston-bates.md`) | `rust/src/volmodels/chain.rs` | `days`, `t` |
| Option checks | `rust/src/market/option_arb.rs` | the same rule (A-125) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Minutes from the snapshot time (ticket text) | The stored file has no clock time for Cboe files. For Alpaca files (`<date>T<hhmm>`) the time exists; the code does not use it. `--time HHMM` only selects the file. |
| 252 trading days | Not known. |

## Why this choice
The ticket `T-20261120-vol-1-heston-bates` says "Time in minutes from the snapshot time". The code uses days. The difference between ticket and code is not explained in any file: not known.

## Effect on the result
Large for expiries of 1 to 2 days (model document, Limits). No sensitivity number is recorded.

## Who decided and when
Not known.

## When to examine it again
When the fit runs on a file with a time stamp; when short expiries (under 7 days) matter.
