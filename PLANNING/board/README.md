# Board

The board is the one list of work that a person does now or does next.
One ticket is one file: `PLANNING/board/<id>.md`.
Work without a ticket is in the backlog: `PLANNING/tickets.md`.

## Ticket format

```
---
id: T-20261119-example
title: Short title that starts with a verb
area: engine
status: backlog
owner: louellet-git
head: technology
order: 3
branch:
created: 2026-11-19
acceptance: One sentence that a person can check.
---
Free text.
```

| Field | Rule |
|---|---|
| `id` | `T-YYYYMMDD-<slug>`. The file name is the id. |
| `area` | `engine`, `importer`, `trading`, `docs`, `planning`, or `ticker:<TICKER>`. |
| `status` | `backlog`, `in-progress`, `review`, or `done`. |
| `owner` | A GitHub login. |
| `head` | The head that is responsible for the ticket: `quant`, `fundamental`, `technology` or `quality` (`chiefs/ORIENTATION.md`). `owner` means that only a person can do the work. Each ticket has this field. |
| `order` | The position of the ticket in the queue of its head. The lowest number is first. A person can change it. |
| `branch` | The git branch. Necessary when the status is `in-progress` or `review`. |
| `acceptance` | One sentence that a person can check. |

| Status | Meaning |
|---|---|
| `backlog` | Planned. No person has the ticket. |
| `in-progress` | A person works on the branch of the ticket. |
| `review` | A pull request is open. |
| `done` | The pull request is merged. Delete the ticket file in the subsequent change. |

## Rules

1. Before you start work, set the ticket to `in-progress` with your login and branch.
   Push this change first. Then the other person can see that the ticket is yours.
2. Use one branch for one ticket. Start the branch from `origin/main`.
3. Merge to `main` only through a pull request.
4. A new ticket has a `head` and an `order`. If the work is in the areas of two heads, make two tickets.
5. No tool enforces these rules. Each person and each agent obeys them.
