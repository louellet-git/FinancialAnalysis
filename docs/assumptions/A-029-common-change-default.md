# A-029: common change of the plug (default: the list median)

Updated: 2026-11-21. Commit: 2e70201 (branch `claude/premium-series`).
Model document: `docs/models/implied-and-attribute.md`.

## Choice
`fa dcf attribute --common` sets the common change c of a view. The default is `median`: the median of
(x2 - x1) over the tickers of the run with that view (a decimal). The plug of each ticker splits into a common part
(x1 to x1 + c) and a company part (the rest). The two add exactly to the plug.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Attribution | `rust/src/dcf/auto_attribute.rs` | `--common`, `--common-list`, `--common-file` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| `vw`: the mean weighted by price x shares at D1 | Kept as an option and a comparator. The design fixes the median as default |
| `file`: the economics view (premium and G of A-001 together, both views) | Needs a file from the Head of Economics. An option. An empty field means "not covered" |
| No split (c = 0) | The company part is then the whole plug. It is the result for one ticker with no list |

## Why this choice
Owner request, 2026-11-21: the implied premium and growth are part of the attribution. The median is robust and
has the same form as the S1 of the list (median-based). Design: `PLANNING/board/T-20261121-premium-company-part.md`,
design changes 1 to 3. No other source is known.

## Effect on the result
Week 2026-11-13 to 11-20, S&P 100, 65 names with both values: the median change is +0.78 points of premium (ticket
text; the run is not a result of record).

## Who decided and when
Head of Fundamental Research, 2026-11-21, after the owner request. The economics view: agreed with the Head of
Economics, 2026-11-21 (design change 3).

## When to examine it again
When the Economics engine gives its first file. When the test on the weekly history has its result.

## Limit of the default (added 2026-11-21, gate note)
The default c is the median of the same price changes that the split explains. The company part is thus a difference
from the list, not news about the company. The common step uses the beta and the equity weight of D1; the company part
holds their change from D1 to D2.
