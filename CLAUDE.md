---
id: DOC-CLAUDE
type: policy
item: 10.29 Life-cycle policy
status: current
date: 2026-10-25
scope: Orientation and norms for working this repo; read first.
---
# CLAUDE.md — agent orientation

Start with `docs/ARCHITECTURE.md` (code-verified module map) and
`docs/HIDDEN-ASSUMPTIONS.md` (trap registry). Every doc and its header:
`python scripts/doc_check.py list`; doc rules: `docs/DOC-STANDARD.md`.

## Collaboration

Two people work this repo in parallel. Follow `docs/COLLABORATION.md` before any work.

## What this repo is

Valuation factory: README.md. Engines: `financial_analysis/dcf.py`,
`financial_analysis/driver_dcf.py` (mechanics: docs/ARCHITECTURE.md). Design:
`PLANNING/reusable-dcf-vision.md`. Flagship report: `analysis/IBM/report/IBM-valuation-2026-07.md`
(§4 headline).

**Open work:** the board, `PLANNING/board/` (`python scripts/board.py status`); backlog not
yet ticketed: `PLANNING/tickets.md` §OPEN TICKETS.

## Normativity map — which source governs when docs disagree

1. **Engine code governs over doc prose and stale doc numbers.**
   `financial_analysis/driver_dcf.py` and `dcf.py` define what the engines actually do;
   any number in a README/report that disagrees with a fresh engine run is stale.
2. **`driver_model.py` inline rationales govern over DRIVERS.md prose.** DRIVERS.md
   says so itself ("the executable, NORMATIVE definition — its rationale strings
   govern"). DRIVERS.md is the reviewer-facing companion table.
3. **Recorded dispositions bind.** `analysis/IBM/REVIEW.md` (F1–F19) and
   `analysis/IBM/DRIVER-REVIEW.md` (19 + 4 reviews) are settled rulings — do not
   re-litigate them in later work; reopening one requires a review of the same weight
   (same weight rule as item 4).
4. **`PLANNING/RULES.md` rules are binding at their declared scope**
   (universal / archetype-bound / ticker-bound); deprecation requires a review fan-out
   of the same weight that created the rule.
5. **`scenarios.py` IBM entries are the v3 aggregate framework, kept deliberately** —
   they are NOT stale v4 values (see docs/HIDDEN-ASSUMPTIONS.md #13). v4 headline
   numbers come only from `python -m analysis.IBM.dcf_drivers_run`.
   `analysis/IBM/README.md` is superseded (v3); keep its net-debt bridge and data notes,
   which v4 still cites.
6. **Reports and `frozen`/`superseded` docs are never edited.** `analysis/IBM/report/*`
   was audited (`analysis/IBM/report/SOURCE-AUDIT.md`) against the engine on its date; do not edit published
   reports to "fix" them — supersede with a new version.
7. **Evidence and design contracts bind.** `analysis/IBM/research-ledger.md` holds the
   frozen source-tagged facts the model consumes; RPO expiry is open in `PLANNING/tickets.md`;
   `docs/visualizer-design-brief.md` and
   `docs/design-specs/2026-07-16-ratings-desk-blueprint.md` bind restyles of
   `visualize_dcf` output.
8. `README.md`, `context.json`, `docs/ARCHITECTURE.md`, `docs/HIDDEN-ASSUMPTIONS.md` are
   informative orientation (code governs); `THOUGHTS` is a raw idea scratchpad.

## Orchestration norms

Objective: an agent works this repo correctly without re-deriving its norms.
Enforcement: docs/COLLABORATION.md §Enforcement and limits (hook, CI); smoke-test and
token-ledger rules: none (session discipline).

- **Subagents do not commit.** The orchestrator (each person's manager session) reviews and
  commits their worktree output.
- **Parallel work happens in git worktrees, one branch per worktree** (`git worktree
  add ../wt-<name> -b <branch> origin/main`). **Checking out a branch in the shared
  main tree is forbidden.** Assume the main checkout has
  someone's uncommitted state at all times.
- The local `main` ref in the main tree may lag `origin/main`; branch new work off
  **`origin/main`**, not local `main`.
- Never commit `data/` churn: `data/` outputs and `__pycache__/` are gitignored and
  untracked.
- **Smoke-test policy:** see `PLANNING/tickets.md` header.
- **Token ledger:** update `PLANNING/token-ledger.md` per run (success metric #1; targets there).
- Session handoffs: `PLANNING/handoff/<first name>.md`, live state (rules in its README;
  delete items as completed).

## Review pipeline

- Doc review: `docs/DOC-STANDARD.md` (Review, Enforcement).
- Run review tiers and sign-off: `PLANNING/reusable-dcf-vision.md`, L5.

## Commands

Verified CLI list: README.md §Pipeline commands; per-module detail:
docs/ARCHITECTURE.md.
