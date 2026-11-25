# A-157: limit price of a paper order

Updated: 2026-11-25. Commit: 9803c60.

## Choice
A buy is a limit order at the ask of the IEX quote of the decision. A sell is a limit order at the bid.
The price has 2 decimals. The order is a day order. A stop sends no order: it keeps the position.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Algo engine | `rust/src/trading/algo/paper.rs` | `PaperHook::decision` (the variable `touch`) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Market order | It has no price bound. A limit at the touch pays the same spread |
| A limit inside the spread | It may not fill, and then the simulator and the broker differ |

## Why this choice
The NBBO is at least as good as the IEX quote. Its ask is not above the IEX ask, and its bid is not below the IEX bid.
So a limit at the IEX touch is marketable on the paper account.
The paper account fills a marketable order at the best price (`alpaca-paper-facts-2026-11-25.md`, section 1).

## Effect on the result
Fills happen at the ask or the bid, as in the simulator.
If the market moves between the quote and the order, the order can stay open. The broker cancels it after 30 seconds (A-156).

## Who decided and when
The ticket `T-20261121-algo-paper-orders`, 2026-11-25.

## When to examine it again
After the first paper fills, and before the first option order.
