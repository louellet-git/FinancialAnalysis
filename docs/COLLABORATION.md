---
id: DOC-COLLABORATION
type: procedure
item: 10.10 Configuration management procedure
status: current
date: 2026-10-26
scope: How the two owners and their Claude sessions change this repo in parallel, and what the gates enforce.
---
# Collaboration procedure

Related: ticket format `PLANNING/board/README.md`; worktrees `CLAUDE.md` §Orchestration norms;
doc review `docs/DOC-STANDARD.md`; handoffs `PLANNING/handoff/README.md`.

Input: a goal in one area. Output: the work merged to main, its branch deleted, its ticket
`done`; pushing to a branch is never finished. Everything is jointly owned, "just maybe not
by two at the same time": every piece of work starts from a filled ticket on
`PLANNING/board/`, one active (`in-progress` or `review`) ticket per area and per branch.
Participants: the ticket's owner (a person and their session); either person ("Either
person", the owner's ruling) may merge and sign off.

## Start

1. `git fetch origin`; in your own worktree or cloud clone run `python scripts/board.py check`.
2. Pick an area with no active ticket (`python scripts/board.py status`). Create a filled
   ticket (`python scripts/board.py new ... --status in-progress`, naming your branch) or move
   your own `backlog` ticket to `in-progress` with its branch.
3. Commit and push it (§Ticket commits). The push to `origin/main` is the claim; nothing
   counts before it.
4. In your worktree `git switch -c <branch>`, the name on the ticket. Never push to, rewrite
   or delete the other person's branch.

## Ticket commits

A ticket commit (create, claim, done) takes only the ticket file and happens in the checkout
where `/delegate` armed `.delegation/`; a worktree without it skips the commit gate.

1. `git fetch origin && git switch --detach origin/main`.
2. Create or edit the ticket (`board.py new ...`, or its `status`).
3. `/skeptic-review` it: one Haiku skeptic, lens `board-ticket` (owner: "Panel per ticket").
   The all-PLEASED `.delegation/review-artifact.json` binds HEAD, not content.
4. `git add PLANNING/board/<id>.md && git commit -m "board: <id>" && git push origin HEAD:main`.

## Finish and merge

1. Your session's `/review-panel` reaches all-PLEASED.
2. Set your ticket to `review` in the branch's last commit; open a normal PR:
   `create_pull_request` head `<branch>`, base `main`, title `<area>: <goal>`, body the
   template | `gh pr create --title "<area>: <goal>" --body-file
   .github/pull_request_template.md`.
3. CI green: `pull_request_read` method `get_check_runs` | `gh pr checks --watch`. A failing
   check is a stop.
4. A frontier or accepted-with-reservations run (vision ✦6a) needs either person's explicit
   sign-off before merge ("Either of you"), recorded in the PR template's Sign-off; other PRs
   need no human review.
5. Merge it yourself: `merge_pull_request` merge_method `merge`, `expectedHeadSha` the head CI
   passed | `gh pr merge --merge`. Either person may merge.
6. Delete the branch from its own checkout: `git push origin --delete <branch>` (the hook
   denies a delete from detached HEAD; `branch-cleanup.yml` normally has deleted it already).
7. Set `done`: the ticket's owner, or whoever merged with the owner's say-so, by §Ticket
   commits from detached `origin/main`. Never in the PR head: CI rejects `done` heads and a
   head change re-triggers review.

A head change after review returns to steps 1 and 3.

## Errors

- **Push to main rejected** (not a fast-forward) or area now taken: redo §Ticket commits from
  step 1 on the fresh `origin/main`, `python scripts/board.py check`, pick another area if taken.
- **Hook denial:** its message names the fix. **Board check failure:** it prints the ticket
  and rule (board README); fix the file.
- **Conflict with main:** on the branch `git fetch origin && git merge --no-commit
  origin/main`, resolve, `python -m pytest -q`, run `/review-panel` on the integrated tree,
  then one `git commit` (the resolution is new code and the artifact binds HEAD, not content).
  `merge=union` works only in local git (GitHub's server merge did not, observed once); check
  the unioned rows' order.
- **Branch predates the board:** merge `origin/main` as above; if no ticket names it, create one
  (§Start).
- **Dead or resumed session:** `git fetch origin`, `python scripts/board.py status`, `git
  switch <the ticket's branch>`. Push the work branch at each milestone (cloud containers die).
- **Merged branch still on origin:** delete it by hand (Finish step 6); the hook denies the
  `done` push until it is gone.
- **Main-guard reverted a push, or its job reports "NOT reverted":** open a PR; fix main by hand
  in the second case.

## Enforcement and limits

Hook `scripts/board_gate.py` (PreToolUse; `.claude/settings.json`), per call and per edited
path, in the repo targeted: first `git fetch origin main` (10 s; a failure is ignored), then
tickets are read from `origin/main`.

| Action | Allowed when |
|---|---|
| Edit, Write, MultiEdit, NotebookEdit | the current branch has exactly one active ticket; `PLANNING/board/`, `.delegation/` and paths outside the repo are free; main or detached HEAD is denied |
| `git commit`, `gh pr create` on a branch | the branch (PR: the head) has its active ticket |
| `git commit` on main or detached | it carries only `PLANNING/board/` paths (staged, `-a` or pathspec) and the board passes `check` |
| `git push` to a branch | the destination is the current branch, which has its ticket (any push to it, force and delete included); other branches, `+force`, `--delete`, `:x`, `src:x`, `--all` (every local branch is a destination) and `--mirror` are denied |
| `git push` to main | fast-forward, board-only, `board.problems()` passes on the pushed tree; an existing ticket changes only if its branch is the current branch or blank, or its only change is `status` to `done` with its branch gone from origin; new tickets are allowed; deleting main is denied |
| GitHub MCP `create_or_update_file`, `push_files`, `delete_file`, `create_branch` | the branch is the current branch with an active ticket; never main |
| GitHub MCP `create_pull_request` | the head has an active ticket |

Targets per call: `cd`, `pushd`, `Set-Location`, `-C`, `--git-dir`, `--work-tree`, `GIT_DIR`,
`GIT_WORK_TREE`, with `~` and `$VARS` expanded; an unresolvable target denies.

CI (`.github/workflows/`):

- `tests.yml`, every push to main and PR: pytest, `board.py check`, `doc_check.py check`. On a
  PR also: the head branch has an active ticket on the base board, and the PR sets no ticket
  `done`.
- `main-guard.yml`: a push to main that touches paths outside `PLANNING/board/` and is not the
  merge of a merged PR is undone by one tree-restore commit (no force push); the run then
  fails, as it does if the revert fails.
- `branch-cleanup.yml`: deletes the head branch when its PR merges, and daily the branches
  merged into main; never main, an open PR head or a not-done ticket's branch.

Limits. The owner kept the free plan ("Keep repo on github as is. Free version."): no branch
protection or required checks, so CI-green before merge is a session rule. Strictness
(owner, provisional, 2026-10-25): "Not very tight constraints, keep it loose for now."

- The `done` push needs its branch deleted: a merged branch left on origin blocks it (the
  sweep spares a not-done ticket's branch; deleting a slash-named ref is unverified on
  GitHub), and deleting an unmerged branch allows a forged `done`.
- The other-branch ticket rule is hook-only; main-guard checks paths only.
- "Always ask first" is not enforceable.
- Unseen by the hook: Bash file writes; git inside scripts, make, aliases or an IDE; `gh pr
  merge`; `gh api`; the web UI; command substitution; push options with values; tag pushes;
  `--pathspec-from-file`, `-i` and `--amend` on a commit; GitHub MCP tools beyond those gated.
- Pushes to non-origin remotes are not checked.
- The fetch adds latency, and after a failed fetch the hook uses stale tickets.
- The hook fails open when no python runs.
- Main-guard is detective: code sits on main until reverted.
- The `done` check runs only on PRs.
- Sessions of two accounts cannot message each other, and `~/.claude` settings, memory and
  plugins miss cloud sessions: coordinate only through the board, PRs and committed files.

## Board, handoff and decisions

- Never change another owner's ticket or close another person's claim: your human asks that
  person ("Always ask first", owner).
- A change reaching into another area's files stays under your ticket: name that area in the
  PR and tell its ticket holder.
- Live state: `PLANNING/handoff/<first name>.md`, named as your human says (ask if unsaid).
  Write only your own file; read the other's.
- A handoff entry is never binding: commit a binding decision in the file that owns it.
- Union ledgers are append-only: add rows directly below the last table row, above any
  trailing prose; never edit old ones.
- A new ID (rule, ticket, finding) is the next free one on `origin/main`; on a clash the later
  merger renumbers.

## Environment

Each person creates a cloud environment in their own account: setup script `bash
scripts/cloud-setup.sh`, variable `FMP_API_KEY` (names in `.env.example`). Values are shared
privately, never committed (locally: `.env`); anyone using an environment can read them. Each
person needs write access and GitHub connected on claude.ai. `gh` fails in cloud sessions
(403): use `mcp__github__*` (owner `louellet-git`, repo `FinancialAnalysis`); locally the `gh`
form after `|`. The delegation framework is vendored (`vendor/delegation-framework/`,
installed under `.claude/`): each session runs `/delegate`, which arms its own untracked
`.delegation/`.
