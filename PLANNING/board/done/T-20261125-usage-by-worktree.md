---
id: T-20261125-usage-by-worktree
title: Give the tokens of a head session to the ticket of the worktree of each line
area: planning
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
story: E6.S5
branch: claude/usage-by-worktree
created: 2026-11-25
acceptance: `fa usage record` gives a line of a head session the ticket of the branch of its `cwd` (a live worktree), else the branch of the line; a fixed-answer test; on this session the "no ticket" share falls.
merged: pull request 241 (branch claude/usage-by-worktree)
---
A head works in many worktrees of one session, and the `gitBranch` of a line is the branch of the start
folder, so all lines were "no ticket". Each line also has its `cwd`. The Head of Staff accepted this proposal
(2026-11-25): about 40 lines and one test, one reviewer. Limit: a worktree that is already removed gives its
lines the branch of the line (count before a worktree is removed).

Result (2026-11-25, this session): "no ticket" fell from 100% to about 79% of the tokens. Most of this session's
worktrees were removed before the count, so their lines keep the old rule (the limit above). Note: the lines
in the main checkout get the ticket of its branch claude/admiring-allen-f5aaeq, which the board gives to an old
ticket (T-20261119-jnj-pipeline-defects); that is the board field, not a code fault.

QA gate: PASS-WITH-NOTES. Order after the note: the line's own branch first when the board knows it (it was right at the time of the line); the branch of its cwd only when the line's branch has no ticket (a reused worktree moves).
