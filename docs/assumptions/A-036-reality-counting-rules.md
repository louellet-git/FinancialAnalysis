# A-036: counting rules of the reality check (one row for each SEC number, percentile below, cut after the common change)

Updated: 2026-11-21. Commit: not known (branch `claude/attribution-reality`, not committed).
Model document: `docs/models/baserates-and-reality.md`.

## Choice
1. One company, one row: two tickers with the same SEC number (CIK) are one company in the base rates. The first
   ticker of the list is used. A ticker with no CIK is a company of its own.
2. The percentile of a value is the share of the set that is strictly below it (`position_in`), from 0 to 100. The
   flag "above every case" means that the value is above the maximum. A set with no finite value gives no percentile.
3. `--top N` keeps the N companies with the largest market value at D1 (price x guarded share count, A-016). It runs
   after the common change (A-029) and after the list premium of `--premium-history`. So both use the full list
   before the cut. Both classes of one company stay and count as one company. Ties go to the first ticker.
4. The list premium of `--premium-history` counts only when its search ended inside the range (an exact result).
   Its percentile uses the rows of the history with an anchor before D1.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Base rates | `rust/src/dcf/auto_baserate.rs` | `base_rates`, `position_in` |
| Reality check | `rust/src/dcf/auto_reality.rs` | `history_before`, `premium_position` |
| Attribution | `rust/src/dcf/auto_attribute.rs` | `keep_top`, `cik_of` |

## Alternatives considered
Not known. A percentile with ties split in half, or an interpolated one, is not in a file.

## Why this choice
Item 1 follows the rule of one weight for one company (`one_weight_for_one_company`). Item 3: the common change is a
measure of the whole list; the ticket (item R2, state note) names the 10 largest of the S&P 100 with the median of the
S&P 100. For items 2 and 4 no source is known.

## Effect on the result
Not known. No sensitivity number exists. With ties, "strictly below" gives a lower percentile than a half-tie rule.

## Who decided and when
Head of Fundamental Research, 2026-11-21.

## When to examine it again
When a review of `--reality` examines the percentile, or when a list has many equal growths.
