---
id: DOC-COLLABORATION
type: procedure
item: 10.10 Configuration management procedure
status: current
date: 2026-10-26
scope: How the two owners and their Claude sessions change this repo in parallel, and what the gates enforce.
---
# Collaboration procedure

Input: a goal in one area. Output: the work merged to main, its branch deleted, its ticket
`done`; pushing to a branch is never finished. Everything is jointly owned, "just maybe not
by two at the same time": every piece of work starts from a filled ticket on
`PLANNING/board/` (format: its [README](../PLANNING/board/README.md)), one active
(`in-progress` or `review`) ticket per area and per branch. Worktrees: `CLAUDE.md`
§Orchestration norms. Participants: the ticket's owner (a person and their session); either
person ("Either person", the owner's ruling) may merge and sign off.

**Board-only** means exactly `PLANNING/board/T-*.md` and `PLANNING/board/README.md`; any other
file there fails `board.py check`.

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
where `/delegate` armed `.delegation/`; a worktree without it skips the delegation commit gate
(`vendor/delegation-framework/scripts/commit-gate.py`).

1. `git fetch origin && git switch --detach origin/main`.
2. Create or edit the ticket (as Start step 2, or its `status`).
3. `/skeptic-review` it: one Haiku skeptic, lens `correctness` (owner: "Panel per ticket").
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
   passed | `gh pr merge --merge`. Squash and rebase leave no merge commit, so a human sets
   `done`.
6. Delete the branch from its own checkout: `git push origin --delete <branch>` (the hook
   denies a delete from detached HEAD).
7. Set `done`: the ticket's owner, or whoever merged with the owner's say-so, by §Ticket
   commits. Never in the PR head (a head change re-triggers review).

A head change after review returns to steps 1 and 3. Locally use the `gh` form (the part after
`|` in steps 2, 3 and 5).

## Errors

- **Push to main rejected** (not a fast-forward) or area now taken: redo §Ticket commits from
  step 1 on the fresh `origin/main`, `python scripts/board.py check`, pick another area if taken.
- **Hook denial:** its message names the fix. **Board check failure:** fix the file the check
  names ([README](../PLANNING/board/README.md)).
- **Conflict with main:** on the branch `git fetch origin && git merge --no-commit
  origin/main`, resolve, `python -m pytest -q`, run `/review-panel` on the integrated tree,
  then one `git commit` (the resolution is new code). `merge=union` works only in local git
  (GitHub's server merge did not, observed once); check the unioned rows' order.
- **Branch predates the board:** merge `origin/main` as above; if no ticket names it, create one
  (§Start).
- **Dead or resumed session:** `git fetch origin`, `python scripts/board.py status`, `git
  switch <the ticket's branch>`. Push the work branch at each milestone (cloud containers die).
- **Abandoned claim** (PR closed unmerged): the hook denies releasing another branch's ticket
  from detached `origin/main`. Delete the remote branch if the work is dropped (Finish step 6),
  then in the branch's worktree `git switch -C <branch> origin/main`, set the ticket
  `backlog` with `branch:` blank, commit, `git push origin HEAD:main` (no panel runs in a
  worktree).
- **Main-guard reverted a push, or its job says "NOT reverted" or "could not decide":** open a
  PR; fix main by hand in the last two cases.

## Enforcement and limits

Hook `scripts/board_gate.py` (PreToolUse; `.claude/settings.json`), per call and per edited
path, in the repo targeted: first `git fetch origin main` (10 s; a failure is ignored), then
tickets are read from `origin/main`.

| Action | Allowed when |
|---|---|
| Edit, Write, MultiEdit, NotebookEdit | the current branch has exactly one active ticket; board-only paths, `.delegation/` and paths outside the repo are free; main or detached HEAD is denied |
| `git commit`, `gh pr create` on a branch | the branch (PR: the head) has its active ticket |
| `git commit` on main or detached | it carries only board-only paths (staged, `-a` or pathspec) and the board passes `check` |
| `git push` to a branch | any push to the current branch, which has its ticket (force and delete included); denied when aimed at another branch, and `--all` and `--mirror` |
| `git push` to main | fast-forward, board-only, `board.problems()` passes on the pushed tree; an existing ticket changes only if its branch is the current branch or blank, or the change is only `status` to `done`, its branch is absent on origin, and a merge commit on `origin/main` has a subject ending `/<branch>`; new tickets are allowed; deleting main is denied |
| GitHub MCP `create_or_update_file`, `push_files`, `delete_file`, `create_branch` | the branch is the current branch with an active ticket; never main |
| GitHub MCP `create_pull_request` | the head has an active ticket |

Targets per call: `cd`, `pushd`, `Set-Location`, `-C`, `--git-dir`, `--work-tree`, `GIT_DIR`,
`GIT_WORK_TREE`, with `~` and `$VARS` expanded and `$PWD`/`$OLDPWD` resolved against the session
cwd; an unresolvable target denies. `GIT_DIR=`/`GIT_WORK_TREE=` prefixes bind to their own
command; `export` and bare assignments persist.

CI (`.github/workflows/`):

- `tests.yml`, every push to main and PR: pytest (`PLANNING/` excluded), `board.py check`,
  `doc_check.py check`. On a PR also: the head branch has an active ticket on the base board,
  and the PR sets no ticket `done`.
- `main-guard.yml`: a push whose tip is a merged PR's merge commit is ok (only the tip is
  checked); a board-only push runs `board.py check` and fails the run if it fails, with no
  revert. Any other push is undone by one tree-restore commit authored `main-guard` (no force
  push); the run then fails, as it does if the revert fails or `gh api` fails ("could not
  decide; NOT reverted").
- `branch-cleanup.yml`: deletes the head branch when its PR merges (no ticket check); daily,
  deletes branches merged into main, never main, an open PR head or a not-done ticket's branch.

Limits. The owner kept the free plan ("Keep repo on github as is. Free version."): no branch
protection or required checks, so CI-green before merge is a session rule. Strictness
(owner, provisional, 2026-10-25): "Not very tight constraints, keep it loose for now."

- A merged branch left on origin blocks the `done` push; the sweep spares a not-done ticket's
  branch; slash-named ref deletion is unverified. The check matches any merge subject ending
  `/<branch>`.
- Squash and rebase merges block the `done` edit.
- The other-branch ticket rule is hook-only; main-guard checks paths only.
- "Always ask first" is not enforceable.
- Unseen by the hook: Bash file writes; git inside scripts, `source`, `eval "$(...)"`
  indirection, make, aliases or an IDE; `gh pr merge`; `gh api`; the web UI; backtick command
  substitution; `--push-option=value` (and `-o value`, misread as a refspec, so denied); tag
  pushes; `--pathspec-from-file`, `-i` and `--amend` on a commit; GitHub MCP tools beyond those
  gated.
- Subshell scoping `( cd x && ... )` is not modelled: the `cd` is treated as persisting.
- Pushes to other remotes get the same verdicts, judged against origin's board; the other
  remote's own state is not checked.
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
- Live state: `PLANNING/handoff/<first name>.md`, lowercase (`luc.md`), named as your human
  says (ask if unsaid). Write only your own file; read the other's.
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
(403): use `mcp__github__*` (owner `louellet-git`, repo `FinancialAnalysis`). The delegation
framework is vendored (`vendor/delegation-framework/`, installed under `.claude/`): each
session runs `/delegate`, which arms its own untracked `.delegation/`.
