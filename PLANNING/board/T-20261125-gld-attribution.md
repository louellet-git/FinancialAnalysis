---
id: T-20261125-gld-attribution
title: GLD layers 1 and 2: ounces for each share x the gold price, less the fee, against the GLD price
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
story: E3.S5
order: 2
branch:
created: 2026-11-25
acceptance: `fa fund attribution GLD` prints, for each quarter end of 3 years, the NAV from the trust's own 10-Q and 10-K (ounces, shares, the gold price that the trust used, the fee) against the GLD price, and a monthly view with the gold series of T-20261123-commodity-series (approximate, a monthly average); the remainder is named; fixed-answer tests; one real run.
---
Owner word (2026-11-25, through the Head of Staff): "All three layers this week". The URA part is
T-20261125-fund-attribution (done). Needs the gold series of T-20261123-commodity-series for the monthly view.

Gate note (Quality and Risk, 2026-11-26, PR 261): the World Bank gold is a monthly average (before 2025-06: the London
afternoon fixing). The layer 1 identity of GLD must say "monthly average against a month-end NAV"; the quarter ends
use the price of the trust's own 10-Q or 10-K.
