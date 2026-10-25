---
id: DOC-BOARD-README
type: procedure
item: 10.10 Configuration management procedure
status: current
date: 2026-10-25
scope: Format, lifecycle and rules of the scrum board PLANNING/board/, enforced by scripts/board.py; audience both owners and their agents.
---
# Scrum board

One ticket per file: `PLANNING/board/<id>.md`, id `T-YYYYMMDD-<slug>` (slug: lowercase
`a-z0-9` and hyphens). Date plus slug keeps two people from colliding. The filename
must equal the id. `python scripts/board.py check` enforces everything below; on a
violation it prints the failing ticket and rule: fix the ticket file, re-run.

## Format

Front matter between `---` lines, one `key: value` per line; the body is free text.

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

| Key | Value |
|---|---|
| area | `engine`, `importer`, `docs`, `planning`, or `ticker:<TICKER>` (`ticker:[A-Z]{1,5}(\.[A-Z])?`) |
| status | `backlog`, `in-progress`, `review`, `done` |
| owner | GitHub login |
| branch | git branch doing the work; empty only in `backlog` |
| created | `YYYY-MM-DD` |
| acceptance | one checkable sentence |

## Lifecycle

Steps, commands and gates: `docs/COLLABORATION.md`.

| Status | Meaning | Set by |
|---|---|---|
| `backlog` | planned, unclaimed, no branch | `board.py new` |
| `in-progress` | work on the ticket's branch | the create commit on main |
| `review` | PR open | the branch's last commit |
| `done` | merged, branch deleted | only the finish commit on main, after merge |

## Rules

- At most one `in-progress` or `review` ticket per area and per branch, counted on `origin/main`.
- A session never changes another owner's ticket; ask that person.
- `python scripts/board.py branch <name>` prints the active ticket id for a branch (exit 1 if none).
