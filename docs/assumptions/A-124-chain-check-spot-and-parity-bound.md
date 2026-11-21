# A-124: spot S of the American checks, and the parity bound with no dividend

Updated: 2026-11-21. Commit: not yet made.

## Choice
For American options the checks intrinsic, bound and parity use the `spot_price` of the file as S. S is a last price. It has no bid and no ask and no time of the quote.
- Intrinsic: a quote with ask < max(S - K, 0) (call) or K - S (put) is a break and leaves the other checks.
- Bound: bid above S (call) or K (put) is a break and leaves the other checks.
- Parity upper bound: `bid C - ask P > S - K df` is a break. The bound has no dividend term.
With no spot, these three checks do not run (no message in the table).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Static arbitrage check | `rust/src/market/option_arb.rs:287`, `318-342` | `spot`, checks 6 and 7 |
| Static arbitrage check | `rust/src/market/option_arb.rs:386-400` | check 5 (parity) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Forward from the parity of the mids in place of S | Not known. |
| A spot with a time stamp from the same minute | Not known. The Cboe file has no time. |

## Why this choice
A stock has no bid and no ask, so S is a last price of the same file (code comment, `option_arb.rs:13-14`).
`CHECK_USAGE` says only the upper parity bound is tested, because the lower bound needs the dividends. A reason for the missing dividend term in the upper bound: not known.
No person's decision on S is recorded.

## Effect on the result
A stale S makes false intrinsic and bound breaks. IBM 2026-11-19 (`--rate 0.04`): 43 quotes have an ask below the intrinsic value, 0 have a bid above S or K; the largest is 63.97 USD (a put, 2028-01-21).
No test of the same file with another S is recorded.

## Who decided and when
Ticket `T-20261120-arb-1b-chain-check-depth`, item 3 (bounds call <= S and put <= K), owner ticket of 2026-11-20. The use of the last price: agent choice.

## When to examine it again
When a file has a time stamp and a quote of the stock at the same time; when a break of these checks is used as a finding.
