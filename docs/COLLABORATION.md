# Collaboration workflow

Two people work this repo in parallel, each driving their own Claude Code session (cloud
or local). Everything is jointly owned; one rule: every piece of work starts from a filled
ticket on the scrum board, `PLANNING/board/` (format, areas, statuses: its README).
Work is finished only when it is merged to main, its branch is deleted and its ticket is
`done`; pushing to a branch is never finished.

## Enforcement and limits

- **Claim = the ticket on `origin/main`.** The push is the claim; nothing counts before
  it. One active (`in-progress` or `review`) ticket per area and per branch.
- **Hook** (`scripts/board_gate.py`, per command and per edited path, in the target
  repo): no edit, commit, push or PR unless the branch has an active ticket on
  `origin/main`. A push that could update main must be board-only and a fast-forward.
  GitHub write tools (`create_or_update_file`, `push_files`, `delete_file`) cannot write
  to main.
- **Ticket commits** (create, done) need a panel first: see §Ticket commits.
- **CI** (`.github/workflows/`): `tests.yml` runs pytest, `board.py check` and the PR
  head's ticket (read from main's board); `main-guard.yml` reverts code pushed to main
  outside a merged PR; `branch-cleanup.yml` deletes merged branches on merge and daily,
  never `main`, an open PR head or an active ticket's branch.
- **Limits:** the hook misses Bash file writes, aliases and other indirection, and
  sessions without python. The free plan has no branch protection or required checks
  (repo settings unreadable, 403), so the main guard is detective (it reverts after
  landing) and CI-green before merge is a session rule.
- Sessions of two accounts cannot message each other, and `~/.claude` settings, memory
  and plugins miss cloud sessions: coordinate only through the board, PRs and committed
  files. `gh` fails in cloud sessions (403): use `mcp__github__*` (owner `louellet-git`,
  repo `FinancialAnalysis`); locally the `gh` form after `|`. Each person needs write
  access and GitHub connected on claude.ai.

## Start: ticket, then branch

1. `git fetch origin`; in your own worktree (`git worktree add --detach ../wt-<name>
   origin/main`, see CLAUDE.md) or cloud clone, run `python scripts/board.py check`.
2. Pick an area with no active ticket on `origin/main`. Create a filled ticket
   (`python scripts/board.py new ... --status in-progress`, naming your branch) or move
   your own `backlog` ticket to `in-progress` with its branch.
3. Commit and push it (§Ticket commits). If rejected, `git pull --rebase origin main`,
   re-run `check`, and pick another area if yours is now taken.
4. `git switch -c <branch>`, the name on the ticket. Never push to, rewrite or delete
   the other person's branch.

A branch that predates the board: `git merge origin/main`; if no ticket names it, create
one as above; continue. A dead or resumed session: `git fetch origin`, `python scripts/board.py
status`, `git switch <the ticket's branch>`; push the work branch at each milestone
(cloud containers die).

## Ticket commits

Create and done commits take only the ticket file and happen in the checkout where
`/delegate` armed `.delegation/` (a separate worktree skips the gate). First
`/skeptic-review` the ticket with one Haiku skeptic, lens `board-ticket`; the all-PLEASED
`.delegation/review-artifact.json` is bound to HEAD. Then `git add PLANNING/board/<id>.md
&& git commit -m "board: <id>" && git push origin HEAD:main`.

## Finish and merge

1. Your session's `/review-panel` (the vendored skeptic) reaches all-PLEASED.
2. Set your ticket to `review` in the branch's last commit and open a normal PR:
   `create_pull_request` head `<branch>`, base `main`, title `<area>: <goal>`, body the
   template | `gh pr create --title "<area>: <goal>" --body-file
   .github/pull_request_template.md`.
3. CI green: `pull_request_read` method `get_check_runs` | `gh pr checks --watch`. A
   failing check is a stop.
4. Frontier and accepted-with-reservations runs (vision ✦6a) need either person's
   explicit sign-off before merge, recorded in the PR; other PRs need no human review.
5. Merge it yourself: `merge_pull_request` merge_method `merge`, `expectedHeadSha` the
   head CI passed | `gh pr merge --merge`. Either person may merge.
6. Set `done`: `git fetch origin && git switch --detach origin/main`, edit the ticket,
   commit and push per §Ticket commits. Never in the PR head: CI rejects `done` heads
   and a head change re-triggers review.
7. Delete your branch: `git push origin --delete <branch>` (`branch-cleanup.yml` also
   does).

A head change after review returns to step 1 and step 3.

## Conflict with main

On your branch: `git fetch origin && git merge origin/main`, resolve, `python -m pytest
-q`, re-run `/review-panel` before pushing (the resolution is new code and the commit
gate does not gate merges). `merge=union` works only in local git (GitHub's server merge
did not, observed once); check the unioned rows' order.

## Board, handoff and decisions

- Never change another owner's ticket; ask that person (owner: "Always ask first").
- A change reaching into another area's files stays under your ticket: name that area in
  the PR and tell its ticket holder.
- Live state: `PLANNING/handoff/<first name>.md`, named as your human says (ask if
  unsaid). Write only your own file; read the other's.
- A handoff entry is never binding: commit a binding decision in the file that owns it.
- Union ledgers are append-only: add rows directly below the last table row, above any
  trailing prose; never edit old ones.
- A new ID (rule, ticket, finding) is the next free one on `origin/main`; on a clash the
  later merger renumbers.
- Rulings, provisional (owner, 2026-10-25): "Not very tight constraints, keep it loose
  for now."

## Environment

Each person creates a cloud environment in their own account: setup script `bash
scripts/cloud-setup.sh`, variable `FMP_API_KEY` (names in `.env.example`). Values are
shared privately, never committed (locally: `.env`); anyone using an environment can read
them. The delegation framework is vendored (`vendor/delegation-framework/`, installed
under `.claude/`): each session runs `/delegate`, which arms its own untracked
`.delegation/`.
