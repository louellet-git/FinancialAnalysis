---
id: T-20261125-usage-by-worktree
title: Give the tokens of a head session to the ticket of the worktree of each line
area: planning
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
story: E6.S5
branch: claude/usage-by-worktree
created: 2026-11-25
acceptance: `fa usage record` gives a line of a head session the ticket of the branch of its `cwd` (a live worktree), else the branch of the line; a fixed-answer test; on this session the "no ticket" share falls.
---
A head works in many worktrees of one session, and the `gitBranch` of a line is the branch of the start
folder, so all lines were "no ticket". Each line also has its `cwd`. The Head of Staff accepted this proposal
(2026-11-25): about 40 lines and one test, one reviewer. Limit: a worktree that is already removed gives its
lines the branch of the line (count before a worktree is removed).
