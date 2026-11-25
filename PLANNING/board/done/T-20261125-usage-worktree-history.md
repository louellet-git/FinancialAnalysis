---
id: T-20261125-usage-worktree-history
title: Resolve a removed worktree from the session's own record of its creation
area: planning
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
story: E6.S5
branch: claude/usage-worktree-history
created: 2026-11-25
acceptance: `fa usage record` gives a line whose worktree is already removed the branch with which the session made that worktree (`git worktree add <path> -b <branch>` in its own transcript, the latest one before the line); a fixed-answer test; on this session the "no ticket" share falls below 79%.
merged: pull request 246 (branch claude/usage-worktree-history)
---
Owner word (2026-11-25, through the Head of Staff): "make sure you're thinking about what can and should be
automated from a holistic point of view." No head has to remember a step: the transcript already records each
worktree that the session made, with its branch (87 such commands in this session). Nothing new is written.

Result (2026-11-25, this session): "no ticket" fell from about 79% to about 59.5% of the tokens. The rest is
work in folders with no ticket branch (the home folder, the store clone, the-heads): coordination and store
writes. The match also takes a parent folder of a removed worktree.

QA gate: PASS-WITH-NOTES. Limits in the code comment: the `git -C` form and a path in a shell variable are not found.
