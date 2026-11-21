# A-130: scope of the futures check (monthly VX, one date, one ZQ contract)

Updated: 2026-11-21. Commit: not yet made.

## Choice
- Dates: `--date D`, else the last 5 weekdays before today (`run`). Holidays are not removed.
- VX: only monthly contracts (symbol `VX/..`) of the daily file. A contract with no history file is left out and not counted. The check ends at the first date with at least one compared contract.
- ZQ: one contract (month of the first date) and one comparison. The other dates are not used.
- A check with nothing compared is an error ("no date had a row in both Cboe files").

## Where used
| Model | File | Parameter name |
|---|---|---|
| Futures check | `rust/src/futures/check.rs:148-164` | `run` |
| Futures check | `rust/src/futures/check.rs:42`, `61-76` | `is_monthly` filter, `check_vx` |
| Futures check | `rust/src/futures/check.rs:122-145`, `163` | `check_zq(repo, days[0])` |
| Cboe file | `rust/src/futures/cboe.rs:34-36` | `is_monthly` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Compare all 5 dates for VX | Not known. |
| Weekly VX contracts | Not known. |
| Compare the whole ZQ strip | Not known. |

## Why this choice
Not known. The module header says there is no second source for the CME prices (ES, NQ, Treasury, SOFR), so only these two checks exist (#76).
The `run` function builds 5 dates but VX uses the first one with data and ZQ the first date: author choice, no study.

## Effect on the result
A pass covers a few prices: the monthly VX contracts of one date and one ZQ contract. It is a spot check. No count of compared prices is recorded here; the command prints `compared`.

## Who decided and when
Not known. The check is from commit `c6adfa7`; the pure functions are from commit `eae3818`. No person is named.

## When to examine it again
When the futures data are used in a signal or a trade; when a second source for the CME prices exists.
