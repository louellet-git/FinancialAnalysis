---
id: T-20261120-delete-merged-branches
title: Delete the merged branches of the remote repo
area: planning
status: backlog
owner: louellet-git
head: owner
order: 1
branch:
created: 2026-11-20
acceptance: `git branch -r --merged origin/main` lists only `origin/main`, and no branch with work that is not in `main` was deleted.
---
The remote repo has about 48 branches. Most are merged. Delete each branch that is fully in `main`. For a
branch that is not merged, ask its owner before the deletion.

This is necessary before step 2 of `T-20261025-keep-private-frameworks-out-of-the-public-repo` (the
history rewrite of `main`).

State, 2026-11-20 (Head of Technology). The repo deletes a branch after its merge (48 branches became 17). The
git proxy of a cloud session refuses a branch delete (HTTP 403), so this is work for the owner now. On
GitHub, delete these 6 branches; each one is fully in `main` and no head uses it:
`claude/agent-teams`, `claude/bank-utility-study`, `claude/bond-recon`, `claude/dcf-conventions-study`,
`claude/report-rust`, `claude/volatility-study`. Two of them came back on 2026-11-20 because a session pushed
all its local branches: a head pushes only its own branch (`git push origin <branch>`).
The 9 branches that are not merged belong to the work of the heads. Keep them.
