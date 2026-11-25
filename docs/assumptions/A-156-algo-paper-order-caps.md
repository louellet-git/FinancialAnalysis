# A-156: order caps of the paper broker

Updated: 2026-11-25. Commit: 9803c60.

## Choice
Step A has these caps:
- 1 share in an order and in the position;
- at most 6 orders in 60 seconds, which is one order every 10 seconds;
- at most 10 orders a day;
- an open order that is not filled after 30 seconds is cancelled;
- three API errors in a row end the run;
- all Alpaca requests of the process stay inside 120 a minute (the data poll plus the order calls).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Algo engine | `rust/src/trading/algo/paper.rs` | `LIMITS.max_qty`, `LIMITS.orders_per_min`, `LIMITS.orders_per_day`, `ORDER_TTL_MS`, `MAX_API_ERRORS` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| More than 1 share | Step A tests the pipe, not the size |
| 20 orders a minute (one every 3 seconds) | A one-minute rule never needs it. A hit would show a bug |

## Why this choice
`docs/QUALITY-STANDARDS.md` section 8 (control 6) sets 10 orders a day and 5% of the capital for one order.
The ticket `T-20261121-algo-paper-orders` sets the rate cap of 6 a minute and the size cap.
Alpaca gives a limit of 200 requests a minute for each account (`chiefs/louellet-git/quant/alpaca-paper-facts-2026-11-25.md`, section 5, source RL).
The Alpaca pages that we read give no limit for orders (`docs/DATA-SOURCES.md`, section "Alpaca paper trading", read 2026-11-21).
So the rate cap is our own convention.

## Effect on the result
None on a model value. A cap that is hit refuses an order. The journal says why.

## Who decided and when
The ticket and the Head of Quality and Risk, 2026-11-21. The step A values: 2026-11-25.

## When to examine it again
Before step B (an option), and after the first week of paper results.
