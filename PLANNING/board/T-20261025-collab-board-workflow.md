---
id: T-20261025-collab-board-workflow
title: Collaboration workflow, board, gate, CI check and documentation standard
area: planning
status: in-progress
owner: louellet-git
branch: claude/focused-dijkstra-0senb5
created: 2026-10-25
acceptance: The collaboration workflow, board, gate, CI check and ISO/IEC/IEEE 15289 documentation standard (checker, doc-critic, retrofitted docs) land on main with tests and doc_check passing.
---

This branch also carries the documentation standard (owner, 2026-10-25: "go ahead with the engineering standards docs"); one session branch, so one ticket.

Paused 2026-10-25 (owner). Resume from here:
- Docs: 21/30 PASS (`docs/doc-reviews.tsv`). Critic round-6 fixes were mid-flight on CLAUDE.md, docs/DOC-STANDARD.md, PLANNING/tickets.md, analysis/IBM/DRIVERS.md, docs/ARCHITECTURE.md, docs/HIDDEN-ASSUMPTIONS.md, docs/visualizer-design-brief.md (partial edits committed as WIP); re-run the doc-critic on those 7. docs/COLLABORATION.md not yet retrofitted.
- Gate: round-2 design review of the board gate is NOT PLEASED (11 blockers, incl. COLLABORATION.md rewrite); not started.
- Then: land this branch via PR; tickets for the 13 merged branches; history scrub with Merlan (reminder set for 2026-11-01).
- Owner decision pending: pass a doc once only wording nits remain, or loop to zero findings.
