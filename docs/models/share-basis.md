# Model: share basis of a past price (split factor)

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Purpose
Put the share count of the SEC facts on the same date as the split-adjusted closes of the data store. Without
this, a value per share that divides by a filed count is wrong by the split factor when a split came after the
filing. It is a data rule inside the automatic model, not a valuation model.

## Inputs
| Input | Source | Note |
|---|---|---|
| Share counts | The SEC facts (standard variables of `mapcheck`) | As filed, with the filing date of each fact |
| Split table | `<cache>/_corporate/splits.csv` (`fa splits get`) | Alpaca corporate actions, cleaned (`splits_rules.rs`) |
| Last date of the price file | `<T>/prices.meta.json` `last_date`, else the last row of `prices.json` | The date `to` |

## Method (short)
1. For a fact filed on `s`, the factor is the product of the factors of the splits with `s < ex_date <= to`
   (`split_factor`, `rust/src/market/splits.rs`).
2. The model multiplies each share count by that factor in one place: `YearData::from_period`.
3. The filing date is used, not the period end: a filing after a split has the counts already adjusted (ASC 260).
4. A split with `status = pending` has an ex date after the last date of the price file. The price file is
   not adjusted for it, so it is not in the factor.
5. A row whose `sec_check` is not `match` is not confirmed by an SEC fact. A person can drop a row in
   `rust/src/market/splits_corrections.csv`.
6. A missing table: a warning in `fa dcf auto`, `screen`, `implied`, `attribute`. An error in `fa dcf returns`
   and `fa portfolio backtest`.

## Outputs
The share counts on the basis of the price file, for each fiscal year of each ticker. A check report for a
backtest or a study (`share_basis.rs`).

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-017 | Share basis rule | Filing date, `s < ex_date <= to` | Ticket `T-20261121-split-basis-fix`; the person who chose the filing date rule: not known |
| A-016 | Guard of the share count | 0.5x to 2x, 0.01x to 100x | A scale error of a filer; used in `fa dcf attribute` |

## Limits
- A spin-off and a class change are not splits. One class of several gives a ratio of 2 to 100 (A-016).
- `fa dcf pit` has no factor: it scores revenue and free cash flow.
- The classic engine, `verdict`, `report`, `serve` and `analyze` have no factor (curated shares).
- Alpaca has no split before 2016. The SEC cover-count steps add some rows. Terms for a public copy of the
  Alpaca data: NOT read.
- Results made before the rule (the backtests, the return test, the implied run of 2026-11-20) have a split look-ahead.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/dcf/share_basis.rs`, `rust/src/market/splits.rs`, `splits_rules.rs` |
| Model or parameter file | `<cache>/_corporate/splits.csv`, `rust/src/market/splits_corrections.csv` |
| Command | `fa splits get` (table); the commands of the automatic model use it |
| Tests | `cargo test --manifest-path rust/Cargo.toml --locked` (tests in `splits.rs`, `auto_tests.rs`) |

## Review status
Not reviewed. Trap: `docs/traps/2026-11-21-split-adjusted-prices-against-filed-shares.md`.
