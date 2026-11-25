---
id: T-20261121-daily-history
title: Decide on a stored daily history of estimates and the SPX chain
area: data
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 3
branch:
created: 2026-11-21
acceptance: The owner decides yes or no from the terms table; if yes, a daily task writes the snapshots to a private place, and the first two days are in it.
---
The Head of Fundamental Research asks for one snapshot each day of (a) the consensus estimates
(`fa estimates get`, S&P 100 at least) and (b) the SPX chain (`fa options snapshot SPX --source alpaca`).
A history cannot be made later.

Terms (read 2026-11-21): Nasdaq gives a licence for personal, non-commercial use; the clause on
storage was not found. Alpaca and OPRA: a non-professional may not give the data to another person;
the clause on storage was not found. Cboe and Yahoo do not permit a stored copy without consent.
Thus a daily history is a risk that only the owner can accept. Ask the owner after the store is
private, with the terms table.
