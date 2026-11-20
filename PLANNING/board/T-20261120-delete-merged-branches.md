---
id: T-20261120-delete-merged-branches
title: Delete the merged branches of the remote repo
area: planning
status: in-progress
owner: louellet-git
head: technology
order: 1
branch: claude/delete-merged-branches
created: 2026-11-20
acceptance: `git branch -r --merged origin/main` lists only `origin/main`, and no branch with work that is not in `main` was deleted.
---
The remote repo has about 48 branches. Most are merged. Delete each branch that is fully in `main`. For a
branch that is not merged, ask its owner before the deletion.

This is necessary before step 2 of `T-20261025-keep-private-frameworks-out-of-the-public-repo` (the
history rewrite of `main`).
