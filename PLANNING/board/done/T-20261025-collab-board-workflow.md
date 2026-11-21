---
id: T-20261025-collab-board-workflow
title: Decide the fate of pull request #18 (board gate, CI)
area: planning
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 5
branch: claude/filed-date-sparse
created: 2026-10-25
acceptance: Pull request #18 is closed, or the parts that the owner wants are merged.
merged: pull request 101 (branch claude/filed-date-sparse)
---
Owner decision, 2026-11-19: the ISO/IEC/IEEE 15289 document standard is not used. Documents use ASD-STE100 only.
The owner has not decided about built-in hooks. Until the owner decides, the hooks and the gate scripts of pull request #18 stay out of `main`.
The CI job that runs `pytest` is in `main` (`.github/workflows/tests.yml`). The remaining parts of pull request #18 are the document standard and the hooks.

Closed on 2026-11-21: pull request #18 was closed on 2026-11-19 without a merge, so the acceptance is met. Its parts (Python scripts, hooks, guards) do not agree with the current rules (no Python code; no hook and no guard in the heads system).
