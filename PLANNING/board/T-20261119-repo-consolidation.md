---
id: T-20261119-repo-consolidation
title: Consolidate the open branches into one branch
area: planning
status: review
owner: louellet-git
branch: claude/admiring-allen-f5aaeq
created: 2026-11-19
acceptance: Main contains the verdict band, the JNJ first pass and the trading package, the tests pass, and the status is in one place (this board).
---
Contains pull requests #16 and #17, the trading branch, the removal of generated and private files, and new status documents. Pull request #15 is superseded (the handoff file is deleted).
From pull request #18 this branch takes only: a CI job that runs `pytest`, `.env.example` and `.gitattributes`. Branches `session-close` (pull request #15) and `claude/upbeat-volta-xr0dyg` have no content to keep: the board replaces them.
