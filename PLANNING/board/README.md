# Board

The shape of the work is epic → story → ticket: `EPICS.md` (the Head of Staff keeps it).

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
requested_by: louellet-git
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
| `area` | `engine`, `importer`, `trading`, `docs`, `planning`, or `ticker:<TICKER>`, or `ask` (a question or a permission request for the person; `chiefs/ORIENTATION.md`, section 5). |
| `status` | `backlog`, `in-progress`, `review`, or `done`. |
| `owner` | A GitHub login. |
| `head` | The head that is responsible for the ticket: `quant`, `fundamental`, `technology`, `economics`, `quality` or `staff` (`chiefs/ORIENTATION.md`). `owner` means that only a person can do the work. Each ticket has this field. |
| `order` | The position of the ticket in the queue of its head. The lowest number is first. A person can change it. |
| `requested_by` | The GitHub login of the person who asked for the work. A head offers only the tickets of its own person (`chiefs/ORIENTATION.md`, section 4). No field means `louellet-git`. |
| `story` | The story of the ticket in `EPICS.md`, for example `E1.S3`. Each new ticket has it. An older ticket has its place in `EPICS.md` only. |
| `branch` | The git branch. Necessary when the status is `in-progress` or `review`. |
| `acceptance` | One sentence that a person can check. |

| Status | Meaning |
|---|---|
| `backlog` | Planned. No person has the ticket. |
| `in-progress` | A person works on the branch of the ticket. |
| `review` | A pull request is open. |
| `done` | The pull request is merged. In the subsequent change, move the ticket file to `PLANNING/board/done/` (`git mv`) with `status: done` and a line `merged: <pull request>`. Never delete a ticket. |

## Rules

1. Before you start work, set the ticket to `in-progress` with your login and branch.
   Push this change first. Then the other person can see that the ticket is yours.
2. Use one branch for one ticket. Start the branch from `origin/main`.
3. Merge to `main` only through a pull request.
4. A new ticket has a `head`, an `order` and a `requested_by`. If the work is in the areas of two heads, make two tickets.
5. The queues are the files at the top of `PLANNING/board/`. The folder `done/` keeps the closed
   tickets as a record; a head does not take a ticket from it.
6. No tool enforces these rules. Each person and each agent obeys them.
