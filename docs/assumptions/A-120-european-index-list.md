# A-120: list of European index options in the chain check

Updated: 2026-11-21. Commit: not yet made.

## Choice
12 symbols are European (cash settled): SPX, SPXW, XSP, NDX, NDXP, XND, RUT, RUTW, VIX, VIXW, DJX, XEO.
A chain of one of them gets no calendar, parity, intrinsic or bound test, and the strike filter is on by default (A-121).
Every other symbol is American and gets all seven checks. OEX is American and is not in the list.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Static arbitrage check (`docs/models/options-static-arbitrage-check.md`) | `rust/src/market/option_arb.rs` | `EUROPEAN_INDEXES`, `is_european` |
| Static arbitrage check | `rust/src/market/options_cli.rs` | `american`, `otm` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Test the type from the file (no list) | Not known. |
| Treat all symbols as American | Not known. |

## Why this choice
Ticket `T-20261120-arb-1b-chain-check-depth`, item 4: "an explicit list of the European indexes (SPX, XSP, NDX, RUT, VIX)".
The code comment says that a test keeps the list equal to the index names of `option_history::index_info` (`option_arb.rs`).
The other symbols (SPXW, NDXP, XND, RUTW, VIXW, DJX, XEO) are not in the ticket: author choice, no study.
The VIX options settle on a VIX future, not on the index, so the VIX checks are untested (`option_arb.rs`).

## Effect on the result
Not known. No run compares the two settings.

## Who decided and when
Owner ticket of 2026-11-20 (list of 5 names); the longer list is a choice of the agent `volatility-derivatives-quant`, date not known.

## When to examine it again
When a new index or an ETF option with European exercise enters the store; when a VIX check is wanted.
