---
id: T-20261126-ura-distributions
title: URA distribution history (ex-date, amount) for layer 1b of the fund attribution
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
story: E3.S5
order: 1
branch: claude/ura-distributions
created: 2026-11-26
acceptance: The distributions of URA for 3 years (ex-date, pay date, amount per share) are in the cache from the Nasdaq dividend history (the host and rules of our Nasdaq prices), checked against the yearly sum of the fund's SEC N-CSR where it is available; `fa fund attribution URA` uses them in layer 1b (price total return against NAV total return) and names the source; fixed-answer tests; one real run.
---
Asked by the Head of Economics (2026-11-26) for its URA page, limit (c): the three large negative price-less-NAV gaps
of the quarters with December are read as the yearly distribution, but without the amounts it is an inference.
Size given: about 0.3% of the week.
