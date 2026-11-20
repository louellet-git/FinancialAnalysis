---
id: T-20261120-ask-chain-copy-schedule
title: May a scheduled task copy the option chains each market day?
area: ask
status: backlog
owner: louellet-git
requested_by: louellet-git
head: owner
order: 2
branch:
created: 2026-11-20
acceptance: The person chooses an option in the session "FA-L - Head of Technology" or "FA-L - Head of Quant Research".
---
From the Head of Quant Research, filtered by the Head of Technology (2026-11-20). Ticket
`T-20261120-vol-7-chain-copy`.

The question: each market day that nobody copies loses its bid and ask for good (no free source keeps
past chains with bid and ask). The arbitrage checks and the out-of-sample test of the volatility models
need many days.

Options:
1. A scheduled task of the Claude Code service, kept by the Head of Technology, that runs
   `fa options archive` at about 15:45 New York time on each market day and puts the files in the
   store. Recommended by the Head of Quant Research. Note of the Head of Technology: it is an exception
   to your rule "nothing runs between two commands", and each run starts a cloud session (a small
   token cost each market day; about 10 MB in the store each day).
2. A person or a head runs it by hand on the days that someone works. No cost, but days are lost.
3. A job on a computer of the person (none is known now).

Recommendation of the Head of Technology: option 1 for a test of 20 market days, then look at the cost
and the use.
