# A-156: order caps of the paper broker

Updated: 2026-11-25. Commit: not yet made.

## Choice
Step A: 1 share in an order and in the position; at most 6 orders in 60 seconds; at most 10 orders a day; an open order that is not
filled after 30 seconds is cancelled; at most 3 API errors in a row; the Alpaca requests of the process stay inside 120 a minute (the data poll plus the order calls).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Algo engine | `rust/src/trading/algo/paper.rs` | `LIMITS.max_qty`, `LIMITS.orders_per_min`, `LIMITS.orders_per_day`, `ORDER_TTL_MS`, `MAX_API_ERRORS` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| More than 1 share | Step A tests the pipe, not the size |
| 20 or more orders a minute | One order each 10 seconds is far above what a one-minute rule makes: a hit is a bug |

## Why this choice
10 orders a day and 5% of the capital for one order are in `docs/QUALITY-STANDARDS.md` section 8 (control 6). The rate cap of 6 a minute and
the size cap are set by the ticket `T-20261121-algo-paper-orders`. Alpaca documents 200 requests a minute for each account and no order rate limit.

## Effect on the result
None on a model value. A cap that is hit refuses an order, and the journal says why.

## Who decided and when
The ticket and the Head of Quality and Risk, 2026-11-21; step A values 2026-11-25.

## When to examine it again
Before step B (an option) and after the first week of paper results.
