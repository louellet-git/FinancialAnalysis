# A-123: tick rule and size bands of the chain check

Updated: 2026-11-21. Commit: not yet made.

## Choice
- Tick of a price: 0.01 USD below 3 USD, else 0.05 USD. The tick of a break is by the highest price among its legs.
- Size bands of a break, USD for one share: below 0.10, 0.10 to 1, 1 to 10, 10 and above.
Both change the display only: the ticks of a break and its band. They do not change which quote pairs break.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Static arbitrage check (`docs/models/options-static-arbitrage-check.md`) | `rust/src/market/option_arb.rs` | `tick_of` |
| Static arbitrage check | `rust/src/market/option_arb.rs` | `BAND_NAMES`, `band_of` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Tick by the class of the option (penny pilot or not) | Not known. The code comment says the rule is an assumption, not a rule of each class. |
| Other band limits | Not known. |

## Why this choice
Tick: the penny pilot rule (0.01 below 3 USD, else 0.05), as the code comment says. The code comment says it is not the rule for each class.
Bands: ticket `T-20261120-arb-1b-chain-check-depth`, item 1: "below 0.10, 0.10 to 1, 1 to 10, above 10 USD". The size floor `--min-size` has default 0.

## Effect on the result
None on the breaks. A break of 0.10 USD is 10 ticks below 3 USD and 2 ticks above 3 USD, so a reader can judge the same break in two ways. The ticks are not the same size.

## Who decided and when
Bands: owner ticket of 2026-11-20. Tick rule: agent `volatility-derivatives-quant`, date not known.

## When to examine it again
When the check lists SPX or another class with other ticks; when a person reads the ticks as a tradability test (they are not one).
