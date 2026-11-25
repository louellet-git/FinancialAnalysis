# A-157: limit price of a paper order

Updated: 2026-11-25. Commit: not yet made.

## Choice
A buy is a limit order at the ask of the IEX quote of the decision. A sell is a limit order at the bid. The price has 2 decimals. A day order.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Algo engine | `rust/src/trading/algo/paper.rs` | `PaperHook::decision` (the variable `touch`) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Market order | No price bound. A limit at the touch pays the same spread (the stop sends no order: it keeps the position) |
| A limit inside the spread | It may not fill, and the simulator and the broker then differ |

## Why this choice
The IEX touch is inside the NBBO, so a limit at the IEX ask (bid) is marketable on the paper account. The paper account fills it at the best price
(`alpaca-paper-facts-2026-11-25.md`, section 1).

## Effect on the result
Fills at the ask or bid, as the simulator does. A move of the market between the quote and the order can leave the order open: it is cancelled after 30 seconds (A-156).

## Who decided and when
The ticket `T-20261121-algo-paper-orders`, 2026-11-25.

## When to examine it again
After the first paper fills, and before the first option order.
