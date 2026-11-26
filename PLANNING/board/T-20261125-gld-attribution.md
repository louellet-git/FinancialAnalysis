---
id: T-20261125-gld-attribution
title: GLD layers 1 and 2: ounces for each share x the gold price, less the fee, against the GLD price
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
story: E3.S5
order: 2
branch: claude/gld-monthly
created: 2026-11-25
acceptance: `fa fund attribution GLD` prints, for each quarter end of 3 years, the NAV from the trust's own 10-Q and 10-K (ounces, shares, the gold price that the trust used, the fee) against the GLD price, and a monthly view with the gold series of T-20261123-commodity-series (approximate, a monthly average); the remainder is named; fixed-answer tests; one real run.
---
Owner word (2026-11-25, through the Head of Staff): "All three layers this week". The URA part is
T-20261125-fund-attribution (done). Needs the gold series of T-20261123-commodity-series for the monthly view.

Part 1 (2026-11-26, pull request 262): the quarter-end part. `fa fund attribution GLD [--fetch]`: 13 filings (10-Q and
10-K, 2023-09-30 to 2026-09-30, 15 requests). Layer 1a: ounces per share x the trust's price - accrued fee per share
against the reported NAV per share: within 0.004 USD in all 13 (gold in settlement included). Layer 1b: premium level
-1.19% to +1.64%. Layer 2: NAV return = gold + fee drag (-0.093 to -0.103 pp) + rest (-0.015 to +0.015 pp). Open: the
monthly view (waits for the owner's answer on the World Bank series); `fa sync` does not read this data set yet.

Gate note (Quality and Risk, 2026-11-26, PR 261): the World Bank gold is a monthly average (before 2025-06: the London
afternoon fixing). The layer 1 identity of GLD must say "monthly average against a month-end NAV"; the quarter ends
use the price of the trust's own 10-Q or 10-K.

Part 2 start (2026-11-26): the monthly view, now unblocked (owner decision: the World Bank gold series for all years).
