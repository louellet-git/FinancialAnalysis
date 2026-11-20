---
name: ticket-workflow
description: Use at the start and at the end of each piece of work in this repo: set the board ticket, make the branch, open the pull request, add the token ledger row, and delete the ticket after the merge. Use it before the first edit.
---

# Ticket workflow

Rules: `CLAUDE.md` ("Work rules") and `PLANNING/board/README.md`. Only the orchestrator
runs the git steps. A subagent does the checks and leaves the commit to the orchestrator.

## Steps
1. Find or write the ticket `PLANNING/board/T-YYYYMMDD-<slug>.md` (fields: id, title,
   area, status, owner, requested_by, head, order, branch, created, acceptance). Work without a ticket is in
   `PLANNING/tickets.md`: make a ticket and delete the item there.
2. Set `status: in-progress`, `owner` (your GitHub login) and `branch`. Commit and push
   this change first.
3. Make the branch from `origin/main`. Use one branch for one ticket. For parallel agents:
   `git worktree add ../wt-<name> -b <branch> origin/main`.
4. Do the work in small parts. Run the real command after each part.
5. Before the pull request, run the skill `release-checks`.
6. Add one row to the ledger of your own folder (`CLAUDE.md`, "Token ledger"): run name, date, scope, tokens (write "not
   measured" if you cannot count), notes.
7. Set `status: review` and open the pull request. Merge only through it.
8. After the merge, move the ticket file to `PLANNING/board/done/` in the next change (`git mv`, `status: done`, a line `merged: <pull request>`). Never delete a ticket. Add open points to the
   board as new tickets.

## Check
- The ticket was `in-progress` before the first edit.
- The branch started from `origin/main`.
- The pull request text names the ticket, the commands that ran, and the open points.
- The ledger row exists.
