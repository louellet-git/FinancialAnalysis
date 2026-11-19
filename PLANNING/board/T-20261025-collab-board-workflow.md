---
id: T-20261025-collab-board-workflow
title: Decide the fate of pull request #18 (board gate, CI)
area: planning
status: backlog
owner: louellet-git
branch:
created: 2026-10-25
acceptance: Pull request #18 is closed, or the parts that the owner wants are merged.
---
Owner decision, 2026-11-19: the ISO/IEC/IEEE 15289 document standard is not used. Documents use ASD-STE100 only.
The owner has not decided about built-in hooks. Until the owner decides, the hooks and the gate scripts of pull request #18 stay out of `main`.
The CI job that runs `pytest` is in `main` (`.github/workflows/tests.yml`). The remaining parts of pull request #18 are the document standard and the hooks.
