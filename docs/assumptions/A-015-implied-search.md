# A-015: search ranges and the list premium of `fa dcf implied`

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
`fa dcf implied` shows what a price implies. It is a measurement; no input of the model reads it.
- The value of a ticker is the value of one path with every parameter at its mean and every shock zero
  (`PathEngine::deterministic`). No seed. It is not the expected value and not the P50.
- Three bisection searches, each moves one number: the mean of the equity risk premium, range -4% to +20%
  (`ERP_RANGE`); the growth shift (company mean and year 1), -30 to +60 points (`GROWTH_RANGE`); the margin
  shift (start and target), -50 to +50 points (`MARGIN_RANGE`). No solution: the bound and a flag.
- The list premium: the premium for which the median of value / price of the whole list is 1. The
  value of each group at that premium is divided by the price.
- A group by industry needs 5 tickers or more (`INDUSTRY_MIN`).
- Groups: Fama-French 12 sector and the SIC code of today (`--groups` replaces them).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Implied by the price | `rust/src/dcf/auto_implied.rs` | `ERP_RANGE`, `GROWTH_RANGE`, `MARGIN_RANGE`, `INDUSTRY_MIN` |
| Attribution (the premium search) | `rust/src/dcf/auto_attribute.rs` | the same ranges |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| The P50 of the paths in place of the value at the means | The module text says: a fixed procedure with no random part. The value at the means is not the P50 |
| Other ranges | Not known |

## Why this choice
Method and first run: `analysis/priors/market_implied_2026-11-20.md` (ticket `T-20261120-dcf-market-implied`).
The ranges have no source other than that file. A `<` flag can be the floor of the rate at 0%, not the end
of the range (23 of 57 `premium<` rows of the S&P 500 list in that run).

## Effect on the result
Run of 2026-11-20 with the defaults of that day: the list premium is 2.32% for the S&P 500 list and 3.12%
for the next 500, against the 4.41% of the model (A-004). The median of value / price is below 1 for
most tickers (`docs/HIDDEN-ASSUMPTIONS.md`, #260). This run was before the share-basis rule (A-017).

## Who decided and when
Not known by name. The design is in the ticket, in commit 4d80959 (2026-11-20) and in the file above. No review.

## When to examine it again
When a range binds for many tickers. When a person wants a use of the list premium as a signal (the rule of
`docs/QUALITY-STANDARDS.md`, section 6, forbids an input from the price).
