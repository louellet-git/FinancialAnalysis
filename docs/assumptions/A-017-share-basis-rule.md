# A-017: share basis of a past price (split factor by the filing date)

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
The model multiplies each share count of the SEC facts by the product of the factors of the splits with
`filing date of the fact < ex_date <= last date of the price file` (`to`). The price files of the store are
adjusted for splits up to the last date of each file; the SEC counts are as filed. The filing date is used,
not the period end: a filing issued after a split shows the counts already adjusted (ASC 260).
- The split table: `<cache>/_corporate/splits.csv`, source Alpaca corporate actions, cleaned by
  `rust/src/market/splits_rules.rs`. A row with `sec_check` other than `match` is not confirmed by the SEC facts.
- A spin-off and a class change are not splits.
- A missing table: a warning in `fa dcf auto`, `screen`, `implied`, `attribute`; an error in `fa dcf returns`
  and `fa portfolio backtest`. `fa dcf pit` has no factor (it scores revenue and cash flow).
- The classic engine has no factor: its shares come from a curated file (the curator uses the post-split basis).

## Where used
| Model | File | Parameter name |
|---|---|---|
| All automatic commands | `rust/src/dcf/share_basis.rs` (`Basis`, `to_for`, `load_or_warn`) | the rule |
| One place of use | `rust/src/dcf/auto.rs` | `YearData::from_period` |
| Split table and factor | `rust/src/market/splits.rs` | `split_factor` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Period end in place of the filing date | A filing after a split already has adjusted counts: the factor would count the split twice |
| No factor | The value per share is wrong by the split factor (trap file) |

## Why this choice
Trap `docs/traps/2026-11-21-split-adjusted-prices-against-filed-shares.md`: a past price (adjusted) against
filed shares gives a wrong market value (BKNG 25 for 1, 2026-04-06). The reason for the filing date: ASC 260
(as stated in `docs/models/auto-path-model.md`). Terms for a public copy of the Alpaca data: NOT read (`splits.rs`).

## Effect on the result
The effect is the split factor for each company with a split after the filing. No run of the whole list
before and after is recorded in a file that was read. The backtests before this rule have a split
look-ahead (A-009 header: INVALID until the reruns).

## Who decided and when
Ticket `T-20261121-split-basis-fix` (2026-11-21, owner `louellet-git`). The person that chose the filing date
rule: not known.

## When to examine it again
When the reruns of the backtests are done. When the table has a row with `sec_check` other than `match`
for a ticker of a list. When the store has unadjusted prices.
