---
id: T-20261121-token-calculator
title: Estimate the tokens of a ticket from the measured tickets (a calculator)
area: planning
status: backlog
owner: louellet-git
requested_by: louellet-git
head: staff
order: 9
story: E6.S5
branch:
created: 2026-11-21
acceptance: One command gives, for a new ticket, a token estimate with a range (weighted tokens and share of a week) from the actual numbers of past tickets of the same kind, and a test on tickets that it did not see shows the error of the estimate.
---
Owner word, 2026-11-21: agents "always way over guess" the time and the tokens of work, "and it's really a
guess". The person wants "a calculator of sorts" that uses the data to estimate the tokens of a ticket.
"This is a project on its own, and we can only/should only really work on it once we have enough data."

**On hold.** It starts when `T-20261121-usage-log` has the actual numbers of some weeks. The Head of Staff
says when; the person agrees to the start.

Dependency: the table "The data that the calculator needs" of `T-20261121-usage-log`. Each row of it must
be collected by a script, with no tokens and no step of a head, before this ticket starts.

Notes for the start (not a design):
- The data: the actual tokens of each closed ticket, by head, agent type and model (`T-20261121-usage-log`).
- Possible inputs of the estimate: the head, the area and the story of the ticket, the kind of work (study,
  engine code, data, document), the number of gates, the number of review cycles.
- Start with the simplest form: the median and the range of past tickets of the same kind. Add a part only
  if the test shows a smaller error.
- No estimate written by an agent from its own judgment goes into the data.
