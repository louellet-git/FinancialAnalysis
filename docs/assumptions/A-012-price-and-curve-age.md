# A-012: how old a price or a curve row may be

Updated: 2026-11-21. Commit: 90ed737 (base).
Model document: `docs/models/implied-and-attribute.md`.

## Choice
Four limits on the age of a data point. Unit: days.
- Past price (`fa dcf implied --price-date`, `fa dcf attribute`): the last close on or before the date.
  It is at most 10 days older (`PRICE_GAP_DAYS` = 10). Older: no price, no value.
- A close in the portfolio backtest is at most 10 days old (`STALE_DAYS` = 10). Older: the ticker is not bought.
- `fa dcf attribute` flags a GSW curve row that is more than 3 days older than its date (`CURVE_LAG_DAYS` = 3).
- The portfolio strategies: a price for a held or new ticker is at most 7 days old (`FRESH` = 7).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic model, implied, attribute | `rust/src/dcf/auto.rs` | `PRICE_GAP_DAYS`, `close_on_or_before` |
| Attribution | `rust/src/dcf/auto_attribute.rs` | `CURVE_LAG_DAYS` |
| Portfolio backtest | `rust/src/portfolio/mod.rs`, `strategies.rs` | `STALE_DAYS`, `FRESH` |

## Alternatives considered
Not known. No other value was tested.

## Why this choice
The code says: "a halted ticker has no price" (`auto.rs`). The values 10, 3 and 7 have no source in any
file that was read. They are conventions.

## Effect on the result
No sensitivity number is known. A larger gap gives a price for a halted ticker and a stale market value.

## Who decided and when
Not known. `PRICE_GAP_DAYS` came with the commit b5a33bf (2026-11-20, `fa dcf attribute`,
`fa dcf implied --as-of`).

## When to examine it again
When a ticker with a long halt is in a list. When the data store adds point-in-time prices.
