# A-033: anchor rule of the premium series

Updated: 2026-11-21. Commit: 2e70201 (branch `claude/premium-series`).
Model document: `docs/models/premium-series.md`.

## Choice
The anchor of a Friday is the last date on or before it with a SPY close. SPY is the trading calendar. The command
stops with an error when the anchor is 7 days or more before the Friday (`ANCHOR_MAX_AGE` = 7, unit: days). Each
number of a row uses data public on the anchor only.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Premium series | `rust/src/dcf/auto_premium.rs` | `anchor_of`, `ANCHOR_MAX_AGE` |

## Alternatives considered
Not known. The design names no other rule.

## Why this choice
Design of the Head of Economics (`analysis/macro/2026-11-21-premium-macro-design.md`, terms): a week ends on a
Friday, and the anchor is the last equity trading day on or before it. The limit of 7 days has no source. It
stops a run on a SPY series that ends too early.

## Effect on the result
Not known. No sensitivity number exists.

## Who decided and when
The Head of Economics (the anchor rule, 2026-11-21). The 7-day limit: not known.

## When to examine it again
When the series uses another calendar than SPY.
