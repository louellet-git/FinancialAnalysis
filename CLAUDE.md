# CLAUDE.md — agent orientation

Read this first. Then: `docs/INDEX.md` (every doc, with normativity),
`docs/ARCHITECTURE.md` (code-verified module map), `docs/HIDDEN-ASSUMPTIONS.md` (the
trap registry), `PLANNING/BOARD.md` (the workflow and the project board, where all open
work lives).

## Work tracking: ticket first (binding)

All work is planned and tracked on the project board,
https://claude.ai/artifact/82X6JYs9eBHaVrzbLyeJht; `PLANNING/BOARD.md` has the workflow,
the Definition of Done and the data schema. Before you change code, docs or analysis:

1. Find the ticket for the work on the board, or create one with acceptance criteria.
2. Set yourself (`"claude"`) as assignee, move it to In progress and post an update
   saying what you are about to do.
3. Name the key in every commit (`FA-12: ...`) and add the commits or PR to its Links.
4. Before stopping, post an update on every ticket you touched: what changed, what is
   next, what you need. Hand finished work to In review, not Done.

Work you find along the way that is outside the ticket becomes a new ticket. Read and
write the board with the `ArtifactData` tool (see `PLANNING/BOARD.md`). If the board
cannot be reached, say so and record the same updates in your reply.

## What this repo is

A **valuation factory**: a reusable, self-improving DCF pipeline (SEC EDGAR XBRL import
→ typed statement store → two Monte Carlo DCF engines → adversarial review → published
report), whose accepted review findings compound into deterministic infrastructure
(rules → validators → red-team specimens) instead of being re-paid every run. The
standing design is `PLANNING/reusable-dcf-vision.md` (approved 2026-07-16, seven
amendments + basket amendment 2026-07-17).

Flagship analysis: **IBM v4.1** (`analysis/IBM/`, report
`analysis/IBM/report/IBM-valuation-2026-07.md`): P10 $83.84 / P50 $151.84 / P90 $258.03,
conditional on f = 0.50, price at writing $212.40. IBM reports Q2 on **July 22, 2026**
— the report's four-item watch list and the first bet-ledger entries grade then.

Two engines, both live (details in docs/ARCHITECTURE.md):
- `financial_analysis/dcf.py` — classic scenario MC (samples growth/margin/WACC/tg
  directly; presets in `scenarios.py`).
- `financial_analysis/driver_dcf.py` — driver-based MC (parameter-MC, one draw per
  path, per-path WACC with kd derived from rf, joint tg < WACC − 2% guard; per-ticker
  register in `analysis/<TICKER>/driver_model.py`).

**Current phase** (as of 2026-07-17 evening): IBM v4.1 shipped and audited; vintage
backtest harness and MC path explorer landed; the visualizer's backtest section + price
range buttons merged (PR #9), 0DTE skew exclusion merged (PR #14); DUOL first pass done
(consensus-anchored presets + capital override, PR #14 — labeled NOT a vetted driver
build); P1 of the vision (rulebook extraction, validators, red-team corpus, coverage
validator) is the active frontier, with the ORCL run queued as ticker #2. Open work is
on the project board (`PLANNING/BOARD.md`); Sprint 1 (from 2026-10-25) catches the IBM
model up with its Q2 and Q3 2026 results.

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
7. `README.md`, `context.json`, `docs/` are informative orientation; the project board
   (`PLANNING/BOARD.md`) is live state; `THOUGHTS` is a raw idea scratchpad.

## Orchestration norms (learned the hard way)

- **Subagents do not commit.** Subagents leave work uncommitted in their worktree; the
  orchestrator reviews, collects, and commits. Every commit in this repo's history went
  through the orchestrator.
- **Parallel work happens in git worktrees, one branch per worktree** (`git worktree
  add ../wt-<name> -b <branch> origin/main`). **Checking out a branch in the shared
  main tree is forbidden** — on 2026-07-17 an agent avoided clobbering another agent's
  uncommitted `p1-importer` work sitting in the main checkout only because it created
  its own worktree instead of switching branches in place. Assume the main checkout has
  someone's uncommitted state at all times.
- The local `main` ref in the main tree may lag `origin/main`; branch new work off
  **`origin/main`**, not local `main`.
- Never commit `data/` churn: `data/processed/NVDA_financials.xlsx` and
  `PANW_financials.xlsx` are tracked-but-gitignored leftovers that dirty on every
  pipeline run (ticket FA-4: `git rm --cached`).
- **Smoke-test policy:** a ticket is not done until real output is printed and
  confirmed consumable by the next stage (`PLANNING/BOARD.md` Definition of Done).
- **Token ledger:** update `PLANNING/token-ledger.md` per run (success metric #1;
  targets: frontier ticker ≤ 500k, repeat run ≤ 50k subagent tokens).
- Session handoffs live on the board: an update on each ticket you touched.
  `PLANNING/HANDOFF.md` was retired on 2026-10-25.

## Review pipeline (roles as evidenced in .claude/agents/ and PLANNING docs)

Vendored doc-review framework (`.claude/agents/FRAMEWORK.md`, from
doc-review-framework@d9d9075). Every client-grade report runs:

1. **valuation-analyst** drafts the report;
2. **valuation-skeptic** adversarial substance review, loop until sign-off (this loop
   produced the IBM v1→v3 retraction, REVIEW.md, and the 19+4 driver reviews);
3. **memo-reviewer** grades content/format against `memo-review-rubric.md` (scores,
   never edits), analyst fixes, loop until PASS — a PostToolUse hook
   (`scripts/memo-review-reminder.sh`) enforces this on any freshly produced .docx;
4. **report-auditor** — final gate: claim→source traceability, re-derives every number
   with the engine, softens over-claims (output: `analysis/IBM/report/SOURCE-AUDIT.md`).

The vision doc (L5) extends this to three tiers — frontier fan-outs for new
templates/mappings, a **charter review on every run** (ACCEPT required before
publication), exception reviews on validator/staleness triggers — with **Luc as the
accountability terminus** (✦6a): frontier runs and accepted-with-reservations runs need
his explicit sign-off; routine repeat runs publish on charter ACCEPT alone.

## Quick commands

```bash
python -m pytest                          # 302 tests, no network (~12s)
python -m scripts.analyze IBM             # full import → 9-tab xlsx
python -m scripts.visualize_dcf IBM       # single-page HTML report
python -m scripts.backtest IBM            # point-in-time vintage backtest
python -m analysis.IBM.dcf_drivers_run    # v4 headline numbers (seed 7)
```

Full verified CLI list: README.md; per-module detail: docs/ARCHITECTURE.md.
