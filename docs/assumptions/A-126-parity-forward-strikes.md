# A-126: parity forward of the option checks (3 strikes, median, 0.5 S to 2 S)

Updated: 2026-11-21. Commit: not yet made.

## Choice
The forward of one series is the median of `K + (C - P) / df` over 3 strikes.
- The 3 strikes are the nearest to the spot that have both a call price and a put price (all positive).
- The prices are mids of bid and ask (live chain) or closes (history).
- The result must be above 0.5 S and below 2 S. Otherwise the code does not use it.
- No spot (index chain): the guess for S is the strike with the least `|C - P|`; then the same rule (`forward_from_parity_blind`).
- Without a forward, the option history uses `S / df` (no dividend, flag `from_parity: false`). The check marker uses `S / df` too.
The Heston and Bates fit uses 5 strikes (A-134), so one chain can have two forwards.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Implied volatility, forward (`docs/models/options-static-arbitrage-check.md`) | `rust/src/market/option_iv.rs` | `PARITY_STRIKES`, `forward_from_parity_n` |
| Static arbitrage check | `rust/src/market/option_arb.rs` | forward for the filter, the dividend and the marker |
| Option history | `rust/src/market/option_history.rs` | `forward_from_parity`, `fill_from_mids` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| 5 strikes (Heston fit) | Not known for the check. The fit ticket names 5. |
| Forward from a dividend forecast | Not known. |

## Why this choice
The code says the parity of American options is only an inequality, so the forward is an estimate (`option_iv.rs`).
3 strikes and the factor 2: author choice, no study. Test: `put_call_parity_forward` shows the median ignores one bad pair among three.
Hidden-assumptions entries #237 and #280 describe the limits.

## Effect on the result
It moves the filter (A-121), the marker (A-122), the implied dividend and the volatility of the history. No sensitivity number to the strike count is recorded.

## Who decided and when
Not known. The constant is in the first version of `option_iv.rs` (header, #236); no person is named.

## When to examine it again
When the check and the fit must use one forward; when a chain has a large borrow cost or a large dividend before the expiry.
