# CLAUDE.md — agent orientation

Read this first. Then: `docs/INDEX.md` (every doc, with normativity),
`docs/ARCHITECTURE.md` (code-verified module map), `docs/HIDDEN-ASSUMPTIONS.md` (the
trap registry), `PLANNING/handoff/` (per-person session state).

## Collaboration

Two people work this repo in parallel, each with their own Claude Code session. Before
any work, follow `docs/COLLABORATION.md`: it owns the workflow, the hook and CI that
enforce it, and their limits.

## What this repo is

A **valuation factory**: a reusable, self-improving DCF pipeline (SEC EDGAR XBRL import
→ typed statement store → two Monte Carlo DCF engines → adversarial review → published
report), whose accepted review findings compound into deterministic infrastructure
(rules → validators → red-team specimens) instead of being re-paid every run. The
standing design is `PLANNING/reusable-dcf-vision.md` (approved 2026-07-16, seven
amendments + basket amendment 2026-07-17).

Flagship analysis: **IBM v4.1** (`analysis/IBM/`, report
`analysis/IBM/report/IBM-valuation-2026-07.md`): P10 $83.84 / P50 $151.84 / P90 $258.03,
conditional on f = 0.50, price at writing $212.40. IBM reported Q2 on July 22, 2026;
grading the report's five-item watch list and first bet-ledger entries is open work.

Two engines, both live (details in docs/ARCHITECTURE.md):
- `financial_analysis/dcf.py` — classic scenario MC (samples growth/margin/WACC/tg
  directly; presets in `scenarios.py`).
- `financial_analysis/driver_dcf.py` — driver-based MC (parameter-MC, one draw per
  path, per-path WACC with kd derived from rf, joint tg < WACC − 2% guard; per-ticker
  register in `analysis/<TICKER>/driver_model.py`).

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
   (same principle as RULES.md deprecation).
4. **`PLANNING/RULES.md` rules are binding at their declared scope**
   (universal / archetype-bound / ticker-bound); deprecation requires a review fan-out
   of the same weight that created the rule.
5. **`scenarios.py` IBM entries are the v3 aggregate framework, kept deliberately** —
   they are NOT stale v4 values (see docs/HIDDEN-ASSUMPTIONS.md #13). v4 headline
   numbers come only from `python -m analysis.IBM.dcf_drivers_run`.
6. **Reports are frozen artifacts.** `analysis/IBM/report/*` was audited
   (SOURCE-AUDIT.md) against the engine on its date; do not edit published reports to
   "fix" them — supersede with a new version.
7. `README.md`, `context.json`, `docs/` are informative orientation; `PLANNING/handoff/`
   is live state (delete items as completed); `THOUGHTS` is a raw idea scratchpad.

## Orchestration norms (learned the hard way)

- **Subagents do not commit.** Subagents leave work uncommitted in their worktree; the
  orchestrator (each person's own manager session) reviews, collects, and commits.
  Every commit goes through one person's orchestrator.
- **Parallel work happens in git worktrees, one branch per worktree** (`git worktree
  add ../wt-<name> -b <branch> origin/main`). **Checking out a branch in the shared
  main tree is forbidden** — on 2026-07-17 an agent avoided clobbering another agent's
  uncommitted `p1-importer` work sitting in the main checkout only because it created
  its own worktree instead of switching branches in place. Assume the main checkout has
  someone's uncommitted state at all times.
- The local `main` ref in the main tree may lag `origin/main`; branch new work off
  **`origin/main`**, not local `main`.
- Never commit `data/` churn: `data/` outputs and `__pycache__/` are gitignored and
  untracked (the NVDA/PANW xlsx and stray .pyc leftovers were untracked 2026-10-25).
- **Smoke-test policy:** a ticket is not done until real output is printed and
  confirmed consumable by the next stage (`PLANNING/tickets.md` header; `context.json`).
- **Token ledger:** update `PLANNING/token-ledger.md` per run (success metric #1;
  targets: frontier ticker ≤ 500k, repeat run ≤ 50k subagent tokens).
- Session handoffs live in `PLANNING/handoff/<first name>.md` (rules in its README).

## Review pipeline

Every governed doc follows `docs/DOC-STANDARD.md` (ISO/IEC/IEEE 15289, tailored); the
Haiku `doc-critic` agent reviews conformance and compactness, enforced by hook and CI.
(The doc-review-framework agents were removed 2026-10-25.)

The vision doc (L5) extends this to three tiers — frontier fan-outs for new
templates/mappings, a **charter review on every run** (ACCEPT required before
publication), exception reviews on validator/staleness triggers — with **either person
as the accountability terminus** (✦6a, amended 2026-10-25): frontier runs and
accepted-with-reservations runs need one person's explicit sign-off; routine repeat runs
publish on charter ACCEPT alone.

## Quick commands

```bash
python -m pytest                          # full suite, no network
python -m scripts.analyze IBM             # full import → 9-tab xlsx
python -m scripts.visualize_dcf IBM       # single-page HTML report
python -m scripts.backtest IBM            # point-in-time vintage backtest
python -m analysis.IBM.dcf_drivers_run    # v4 headline numbers (seed 7)
```

Full verified CLI list: README.md; per-module detail: docs/ARCHITECTURE.md.
