---
id: T-20261025-keep-private-frameworks-out-of-the-public-repo
title: Keep private frameworks out of the public repo
area: planning
status: backlog
owner: louellet-git
branch:
created: 2026-10-25
acceptance: No file or history object from delegation-exp or doc-review-framework is reachable from the public repo, and CI plus the gate refuse to add any.
---
Step 1 (done in branch `claude/admiring-allen-f5aaeq`): the files are removed from the tree and are in `.gitignore`.
Step 2 (owner, open): rewrite the history of `main`. Do this after all other branches are merged or deleted. Old pull-request refs (`refs/pull/*`) keep the old objects; only GitHub Support can remove them.
