---
id: T-20261121-board-check
title: Check the board rules with one script (no model tokens)
area: planning
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1.5
story: E6.S5
branch:
created: 2026-11-21
acceptance: One command reads git only and prints, for each head, the rule faults of the list below with the ticket and the reason; it changes no file, it needs no network other than `git fetch`, and a fixed-answer test covers each check.
---
Owner word, 2026-11-21 (through the Head of Staff): the monitoring is "100% script-based", it "doesn't
require the heads remembering", and it is "lean, non-invasive, not token heavy". The past fault was a system
that choked the heads. So this command only reports. It never stops work, and no head has to run it.

The Head of Staff did these checks by hand on 2026-11-21, and each one found a fault:
1. A ticket that is `in-progress` or `review` with no branch, or with a branch that the remote does not have.
2. Two or more tickets in progress on one branch.
3. A ticket that is on a work branch and not on `main` (the rule "ticket first").
4. A head with more than 3 tickets in progress (a count to show, not a limit).
5. An open ticket with no place in `EPICS.md`, and a ticket name in `EPICS.md` with no file.
6. A ticket with no `head`, `order` or `acceptance`; a new ticket with no `story`.
7. A ticket whose pull request is merged and whose file is not in `done/`.

Limits: read-only; the output is one short table; a head sees it only if it wants to. Add a check only after
a fault was seen by hand. The form (a Rust command of `fa`, or a small shell script) is the choice of the
Head of Technology.
