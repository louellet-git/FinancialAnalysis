# A-122: marker of a quote far from its neighbours (10 vol points)

Updated: 2026-11-21. Commit: not yet made.

## Choice
A quote gets a marker when its Black-76 implied volatility of the mid is more than 0.10 (10 volatility points) from a median.
- The median is of up to 2 neighbours below and 2 above, in strike order, in the same series and type.
- At least 2 neighbours must have a volatility.
- A quote with no bid has no mid and no marker.
- The marker is a flag. The checks still use the quote.
- `--iv-mark POINTS` sets the limit. 0 turns the marker off.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Static arbitrage check (`docs/models/options-static-arbitrage-check.md`) | `rust/src/market/option_arb.rs` (`CheckOpts` default, marker code) | `CheckOpts.iv_mark` |
| Static arbitrage check | `rust/src/market/options_cli.rs` | `--iv-mark` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Remove the marked quote | Not known. The code keeps it and counts the breaks apart. |
| Fewer or more neighbours; a mean in place of the median | Not known. |

## Why this choice
Ticket `T-20261120-arb-1b-chain-check-depth`, item 2, asks for the marker but gives no number. The help text says "a smile moves a few points between strikes; a stale quote moves more" (`options_cli.rs`).
The value 10 and the 2 + 2 neighbours: author choice, no study.
Cause: on IBM 2026-11-19 the largest breaks came from one stale quote each (ticket text).

## Effect on the result
It decides which breaks the report shows as "no marked leg". Example, `fa options check IBM --date 2026-11-19 --rate 0.04` (Cboe file, 2026-11-21 run):
427 breaks of the checks 1 to 5, 366 with a marked leg, 61 with none. Run with `--iv-mark 0` to see the other split. No other sensitivity is recorded.

## Who decided and when
Agent `volatility-derivatives-quant`, ticket of 2026-11-20; review PASS-WITH-NOTES of 2026-11-20 (see the ticket). The value itself: no person named.

## When to examine it again
When a break with no marked leg is a stale quote, or a real break has a marked leg. Also for a chain with a wide smile (short expiry, VIX).
