# A-137: form of the bounds of the chain check (slope without discount, buy-only legs, EPS)

Updated: 2026-11-21. Commit: not yet made.

## Choice
- Slope: call `bid C(K1) - ask C(K2) > K2 - K1`; put `bid P(K2) - ask P(K1) > K2 - K1`. The bound has no discount factor.
- Monotony and slope use all strike pairs of a series and type. Butterfly uses consecutive triples.
- A quote with bid 0 and ask above 0 is a buy leg only: it is never a sell leg, so it cannot start a break.
- A break needs a size above `EPS = 1e-9` USD.
- A quote is used when ask >= bid > 0; an ask equal to the bid is used.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Static arbitrage check | `rust/src/market/option_arb.rs:34` | `EPS` |
| Static arbitrage check | `rust/src/market/option_arb.rs:354-384` | checks 1 to 3 |
| Static arbitrage check | `rust/src/market/option_arb.rs:263-282` | buy-only rule |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Slope bound `df (K2 - K1)` for a European index | Not known. It is a tighter bound and would find more breaks. |
| Consecutive pairs only for monotony and slope | Not known. The output note says all pairs make one bad quote many breaks. |

## Why this choice
Buy-only rule: ticket `T-20261120-arb-1b-chain-check-depth`, item 3 ("its ask is a valid buy leg").
Slope with no discount: not known; the bound without `df` is the weaker one, so it is valid for American options too. This is a reading of the code, not a recorded reason.
EPS: author choice, no study.

## Effect on the result
With `df < 1` the slope check on a European index misses small breaks that the discounted bound would show. Not measured.
The all-pairs rule multiplies the number of breaks for one stale quote (IBM 2026-11-19: 5697 monotony tests in one series).

## Who decided and when
Buy-only rule: owner ticket of 2026-11-20. The rest: not known.

## When to examine it again
When the check is used for an index chain; when a break count is used as a measure of quality of a chain.
