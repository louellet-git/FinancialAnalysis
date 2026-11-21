# A-032: limit of the per-ticker bridge lines (20 tickers)

Updated: 2026-11-21. Commit: 2e70201 (branch `claude/premium-series`).
Model document: `docs/models/implied-and-attribute.md`.

## Choice
The text of the full bridge has one line for each ticker when the list has 20 weighted tickers or fewer
(`BRIDGE_TICKER_LINES` = 20, unit: tickers). A longer list has the group lines and the top contributors only.
The CSV has one row for each ticker for any size.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Attribution | `rust/src/dcf/auto_attribute.rs` | `BRIDGE_TICKER_LINES` |

## Alternatives considered
Not known. The S&P 10 list (10 tickers) has the lines; the S&P 100 list does not.

## Why this choice
Owner request, 2026-11-21 about 15:40 UTC: break down the S&P 10 so that the sum of the parts is the change of the
S&P 10. The value 20 has no source. It is a display convention.

## Effect on the result
None on a number. It changes the text only.

## Who decided and when
Head of Fundamental Research, 2026-11-21, after the owner request (`PLANNING/board/T-20261121-sp100-week-attribution.md`).

## When to examine it again
When a person needs the lines for a longer list. The CSV has them now.
