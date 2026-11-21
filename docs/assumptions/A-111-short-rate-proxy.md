# A-111: The 1-year GSW zero yield is the short-rate series

Updated: 2026-11-21. Commit: not yet made.

## Choice
`fa models` calibrates to a rate history. The default series is `gsw1y` (column `SVENY01`
of the GSW file, continuous zero yield, decimal). The 1-year zero yield is a proxy of the
short rate, not the overnight rate. Other choices: `gsw2y` to `gsw30y`, `SVENY..`, `csv:PATH`.

## Where used
| Model | File | Parameter name |
|---|---|---|
| `fa models` (`docs/models/short-rate-models.md`) | `rust/src/ratemodels/cli.rs` | `--series` default `gsw1y` |
| Module note | `rust/src/ratemodels/mod.rs` | "a proxy of the short rate" |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| SOFR or EFFR | Not known. |
| A 3-month bill rate | Not known. |

## Why this choice
The GSW file is a long daily history (title in `forwards.rs`: "1961 to the Present"). The start of the 1Y column is not checked.
The code gives no reason for 1 year. Author choice, no study.

## Effect on the result
Not known. A 1-year yield has a term premium and an expected path of the short rate.
So kappa, theta and sigma are for the 1-year yield, not for the overnight rate. The code has no test of another series.

## Who decided and when
Not known. Ticket `T-20261119-stochastic-dcf-drivers` (2026-11-19).

## When to examine it again
When the parameters go into a DCF or a price, and when an overnight series is cached.
