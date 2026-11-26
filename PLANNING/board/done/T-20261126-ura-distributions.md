---
id: T-20261126-ura-distributions
title: URA distribution history (ex-date, amount) for layer 1b of the fund attribution
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
story: E3.S5
order: 1
branch: claude/ura-distributions
created: 2026-11-26
acceptance: The distributions of URA for 3 years (ex-date, pay date, amount per share) are in the cache from the Nasdaq dividend history (the host and rules of our Nasdaq prices), checked against the yearly sum of the fund's SEC N-CSR where it is available; `fa fund attribution URA` uses them in layer 1b (price total return against NAV total return) and names the source; fixed-answer tests; one real run.
merged: pull request 263 (branch claude/ura-distributions)
---
Asked by the Head of Economics (2026-11-26) for its URA page, limit (c): the three large negative price-less-NAV gaps
of the quarters with December are read as the yearly distribution, but without the amounts it is an inference.
Size given: about 0.3% of the week.

Result (2026-11-26): Nasdaq has no dividend history for URA (NYSE Arca). Yahoo chart (events=div, 1 request): 20
ex-dates, 2010-12-29 to 2025-12-30; the SEC N-CSR of 2026-01-09 agrees for FY2021 to FY2025 to the cent. Layer 1b
now uses the price total return: the three December quarters go from -9.18, -3.28, -6.42 pp to -2.91, -0.87, -2.64 pp.
The rest is the premium change plus timing, not the distribution.
