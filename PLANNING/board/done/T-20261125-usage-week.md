---
id: T-20261125-usage-week
title: fa usage show --week (use since the last weekly reset, in tokens and USD, against the share of the head)
area: tooling
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
story: E6.S5
order: 2
branch: claude/usage-week
created: 2026-11-25
acceptance: `fa usage show --week` lists, for each ticket and in total, the tokens and the USD at list price used since the last reset (Wednesday 16:00 UTC), next to the share of the head from the last table of PLANNING/board/EPICS.md, and the amount that remains; the total of this session is compared with the session cost of the platform, and the difference is stated; fixed-answer tests.
merged: pull request 257 (branch claude/usage-week)
---
Owner word (2026-11-25, through the Head of Staff): "the heads struggle w knowing how many tokens were their tokens
for the week". Size given: about 0.7% of the week. The Head of Staff said go on 2026-11-25.

Result (2026-11-25): `fa usage show --week --head Technology` at 23:01 UTC gave 36.40 USD since the reset; to
22:40 UTC it gave 30.11 USD against the platform session cost 29.95 USD (+0.5%). The 5-minute and 1-hour cache
writes are priced from the split in the transcript lines. Limits: the Haiku cache prices are not verified; a
model with no price shows "(no price)"; the lines of a branch that is not on the board count as "no ticket".
