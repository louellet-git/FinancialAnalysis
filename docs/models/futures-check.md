# Model: Futures cross-check

Updated: 2026-11-21. Commit: not yet made.

## Purpose
The check compares a futures price with a second source, and says if the two agree inside a tolerance.
Two checks exist: VIX futures (two Cboe files) and the 30-day fed funds future (Yahoo against the NY Fed EFFR).
A pass does not prove a price of another contract. CME prices (ES, NQ, Treasury, SOFR) have no second source (`docs/HIDDEN-ASSUMPTIONS.md` #76).

## Inputs
| Input | Source | Note |
|---|---|---|
| VX settle, one date | Cboe daily settlement file | Monthly contracts only (`VX/` symbol). |
| VX settle, history | Cboe history file of each monthly contract | Row of the same date. |
| ZQ price | Yahoo bar of the contract of the month of the date | Not a CME settlement price. |
| EFFR | NY Fed `markets.newyorkfed.org`, 7 days before the month to the date | Cache file used when younger than 6 hours, or when the date is more than 3 days old (`check.rs:140-142`). |

## Method (short)
1. `run` builds the dates: `--date D`, else the last 5 weekdays before today (`check.rs:148-164`).
2. VX: for each monthly contract of the daily file, read its history file and the row of the date. A contract with no file is left out. Mismatch when `|diff| > 0.005` points (`compare_vx`, `check.rs:34-59`).
3. `check_vx` goes through the dates and stops at the first date that has at least one compared contract (`check.rs:61-76`).
4. ZQ: the implied rate is `100 - price`, in decimal, rounded to 1e-8.
5. ZQ: the expected rate is the mean EFFR over the calendar days of the month. A day with no published rate has the last published rate (`expected_average`, `check.rs:97-103`).
6. ZQ: mismatch when `|implied - expected| > 0.0005` (5 bp) (`compare_zq`, `check.rs:107-120`). One comparison for the first date only (`check.rs:163`).
7. The command exits with 1 on a mismatch.

## Outputs
For each check: the text of the check, the source name, the number compared, and the list of mismatches (label, ours, theirs).

## Assumptions
Each choice has one record in `docs/assumptions/`.

| Record | Name | Value | One line |
|---|---|---|---|
| A-128 | VX tolerance | 0.005 index points | Cboe settle against Cboe history. |
| A-129 | ZQ tolerance and month average | 5 bp; last rate carried | A target change in the month breaks it. |
| A-130 | Scope of the check | Monthly VX; one date; one ZQ contract | Few points are compared. |

## Limits
- A pass shows that two files agree. It does not show that a price is right. Both VX files are from Cboe: one fault can be in both.
- ZQ compares one daily price with the average of a whole month. Days after the date use the last published rate. A target change in the month (FOMC) can fail the check with no error in the price.
- The ZQ price is a Yahoo bar, not a CME settlement price. The Yahoo field is not known to be the settle.
- A missing Yahoo bar of the date is an error, not a mismatch (`check.rs:129-134`).
- No check for the weekly VX contracts and for the quarterly rate futures.
- `fa futures check` can download a missing file. Run it with the data store in `data/raw` to avoid a request.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/futures/check.rs` (`compare_vx`, `compare_zq`, `check_vx`, `check_zq`) |
| Command | `fa futures check [--date D]` |
| Tests | `rust/src/futures/check.rs`, module `tests` (`cargo test --manifest-path rust/Cargo.toml --lib futures::check`) |

## Review status
Not reviewed. The trap `docs/HIDDEN-ASSUMPTIONS.md` #76 states the limit of the two checks.
