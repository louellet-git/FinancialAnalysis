# Session handoff — open work as of 2026-07-17 ~9:30pm EST (session closed clean)

Live work-state for the next session/agent. Delete items as completed; delete the file
when empty. Onboarding: read CLAUDE.md first, then docs/HIDDEN-ASSUMPTIONS.md.

## Done (all merged to main: PRs #5–#14)

IBM v4/v4.1 + audited report (gate-to-gate) · driver engine + path explorer · vintage
backtest · importer completeness (RPO/dividends/estimates) · everything-visualizer +
blueprint restyle + range buttons + backtest section (IBM 14/18 in band, 78% vs 80%
nominal) · DUOL first pass (#14 — the #10 commit had merged into viz-final-pass, not
main; cherry-picked back) · 0DTE skew exclusion (#14, `_SKEW_MIN_DTE`) · docs overhaul,
reconciled from the two parallel sets (#11 + #12: verified spine, 31-trap registry).

## Process rule (learned tonight, cost ~400k duplicate tokens)

Two sessions ran the same handoff item in parallel (both doc overhauls) and raced in
the shared main checkout. Before starting an item: check `gh pr list` and for active
sibling sessions; CLAIM the item by editing this file (item → "in progress, session X,
worktree Y") and push the claim; work only in `wt-*` worktrees.

## Decision open for Luc

- **DUOL depth**: first pass is consensus-anchored screening only (mechanical backtest
  0/6 in band — bands decorative). Promote to a driver-model frontier run (needs
  sign-off, vision ✦6) or leave as screening; order vs ORCL is Luc's call.

## Newly opened by the docs verification (assign before/with ORCL)

1. **v3 reproducibility drift** (HIDDEN-ASSUMPTIONS #14): `analysis.IBM.dcf_scenarios`
   derives margins live from the re-imported store and now prints $43/$149/$243 vs the
   signed-off $37/$141/$232. Decide: pin the v3 inputs as constants (preserving the
   record) or stamp the output with a reproduces-as-of note. v4 immune (pinned).
2. **filed_date fabrication** (= period_end on rows without a matched revenue fact) →
   fold into T-IMP-1; backtest point-in-time logic must not trust sparse rows.
3. **Bridge-per-ticker rule**: the default capital-structure derivation is a different
   net-debt definition than the reviewed IBM bridge — every basket ticker needs a
   reviewed bridge before its gap is comparable (candidate RULES.md R2 + validator).

## Queued next (approved plan)

4. P1 remainder: validators + red-team corpus + statement-coverage validator
   (RULES.md R1 + extraction backlog; audit flags C1–C5 seed the coverage audit).
5. ORCL frontier run (ticker #2, cheapened pipeline, token-metered) → basket
   (IBM/ORCL/MSFT/ACN/CSCO) → cross-sectional gap decomposition (vision ✦1 amendment).
6. **July 22: IBM Q2 report** — run `scripts.analyze IBM` that day (captures the
   options IV crush); grade the report §5 watch list (deal conversion, FCF guide vs
   $15.7B, margin level, RPO print); first bet-ledger entries. The 2026 mechanical
   vintage (FY2026 revenue P50 $70.3B vs consensus $71.1B) also grades.

## Standing housekeeping

Worktrees wt-* are stale/disposable (wt-path-explorer holds `main` — remove before any
local main checkout). Never commit the tracked-but-ignored churn (2 xlsx + 4 .pyc;
candidate `git rm --cached`). Token ledger: PLANNING/token-ledger.md, update per run.
