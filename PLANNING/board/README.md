---
id: DOC-BOARD-README
type: specification
item: 10.60 System requirements specification
status: current
date: 2026-10-25
scope: Ticket format and status meanings of the scrum board PLANNING/board/, enforced by scripts/board.py.
---
# Scrum board ticket specification

```
---
id: T-20261025-example
title: Short imperative title
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-10-25
acceptance: One checkable sentence.
---
```

| ID | Requirement |
|---|---|
| TK1 | One ticket per file `PLANNING/board/<id>.md`; the filename equals the id. |
| TK2 | id is `T-YYYYMMDD-<slug>`, slug lowercase `a-z0-9` and hyphens (date plus slug keeps two people from colliding); unique on the board. |
| TK3 | Front matter sits between `---` lines, one `key: value` per line, with all eight keys above, each non-empty except `branch`; the body is free text. |
| TK4 | `area` is `engine`, `importer`, `docs`, `planning`, or `ticker:<TICKER>` (`ticker:[A-Z]{1,5}(\.[A-Z])?`). |
| TK5 | `status` is `backlog`, `in-progress`, `review` or `done`. |
| TK6 | `owner` is a GitHub login. |
| TK7 | `branch` is the git branch doing the work; required when status is `in-progress` or `review`. |
| TK8 | `created` is `YYYY-MM-DD`. |
| TK9 | `acceptance` is one checkable sentence. |
| TK10 | At most one `in-progress` or `review` ticket per area and per branch (claimed on `origin/main`: `docs/COLLABORATION.md`). |

| Status | Meaning |
|---|---|
| `backlog` | planned, unclaimed, no branch |
| `in-progress` | work on the ticket's branch |
| `review` | PR open |
| `done` | merged, branch deleted |

Constraint: transitions and ownership of tickets are in `docs/COLLABORATION.md`.

Assurance: `python scripts/board.py check` enforces TK1–TK5, TK7, TK8 and TK10 (TK6 and
TK9 by review); on a violation it prints the failing ticket and rule: fix the ticket file,
re-run. `python scripts/board.py branch
<name>` prints the active ticket id for a branch (exit 1 if none).
