# A-035: window of the base rates (10 fiscal years, end dates 10 years apart plus or minus 45 days, public years only)

Updated: 2026-11-21. Commit: a0e2480 (branch `claude/attribution-reality`).
Model document: `docs/models/baserates-and-reality.md`.

## Choice
1. A window is a base fiscal year and the fiscal year `--years` years after it. The default is 10
   (`BASERATE_YEARS` = 10, unit: fiscal years). The own growth of `--reality` uses 5 years (`OWN_YEARS` = 5).
2. The end dates of the two years are `years` x 365.25 days apart, plus or minus 45 days
   (`END_TOLERANCE_DAYS` = 45, unit: days; a 52 or 53 week year). The code takes the base year with the nearest distance.
3. A fiscal year with no filing date is not public. It is out of each window (point in time).
4. The last window of a company is the window of its last public fiscal year. A last year that ended more than 548
   days before the date D gives no last window (`LAST_YEAR_MAX_AGE_DAYS` = 548, unit: days; the text says 18 months).
5. A year with no mapped `revenue` above 0 has no window. The code does not patch it.

## Where used
| Model | File | Parameter name |
|---|---|---|
| `fa dcf baserates` | `rust/src/dcf/auto_baserate.rs` | `BASERATE_YEARS`, `END_TOLERANCE_DAYS`, `LAST_YEAR_MAX_AGE_DAYS`, `usable_years`, `windows`, `last_window` |
| `fa dcf attribute --reality` | `rust/src/dcf/auto_reality.rs` | `OWN_YEARS`; the base rates use the values above |

## Alternatives considered
Not known. A window of 5 years is possible with `--years`. The ticket names no other tolerance or age limit.

## Why this choice
Item 1: the ticket (item R1) sets `--years 10`, the length of the explicit period of the automatic model. Item 2: the
code comment says a 52 or 53 week year. Item 3: a filing date proves that the fact was public (point in time).
Items 4 and 5: no source is known for 548 days or for the rule of no patch beyond the code comments.

## Effect on the result
Not known. No sensitivity number exists. A shorter window gives a wider spread of growth. The windows of one company
overlap, so they are not independent.

## Who decided and when
Head of Fundamental Research, 2026-11-21 (ticket item R1). Source for 45 and 548: not known.

## When to examine it again
When a first review of `--reality` looks at the windows, or when the mapping fills the old years of revenue
(ticket `T-20261121-revenue-old-tags`).
