# CLAUDE.md — agent onboarding contract

Read this first. It tells you what this repo is, which documents are NORMATIVE (they bind
your work), and the operating norms that keep multi-agent work from corrupting the record.
Everything here was verified against code/data on 2026-07-17; where any doc conflicts
with the code or the engine output, **the code/engine governs** — note the discrepancy,
don't propagate it.

## What this repo is

A **valuation factory** in the making (see `PLANNING/reusable-dcf-vision.md`):

1. **Data pipeline** — SEC EDGAR companyfacts → typed schema → JSON store; yfinance
   prices/earnings/options/forward-consensus as dated snapshots; 9-tab xlsx export.
2. **Two DCF engines** — the aggregate `StochasticDCF` (samples growth/margin/WACC/tg
   directly) and the driver-based `DriverDCF` (only *drivers* are stochastic, correlated;
   items are vetted formulas). The driver engine is normative for IBM.
3. **An adversarial review machine** — analyst/skeptic/memo-reviewer/report-auditor
   agents whose accepted findings compound into rules and validators instead of being
   re-paid every run.
4. **Client-grade reports** — .docx via officecli, audited claim-by-claim.
5. **The factory roadmap** — rulebook, validators, red-team corpus, gap attribution,
   bet ledger (7 layers, currently in P1).

Current flagship output: IBM v4 driver valuation — **P10 $83.84 / P50 $151.84 /
P90 $258.03, conditional on f = 0.50** (seed 7, 10k sims; reproduces exactly).

## NORMATIVITY MAP — what binds you

| Artifact | Status | What it binds |
|---|---|---|
| `analysis/<T>/driver_model.py` **rationale strings** | **NORMATIVE assumption register** | Every parameter's stochastic-vs-deterministic choice and source. Change a value ⇒ change its rationale ⇒ that is a review-worthy event, not an edit. |
| `analysis/IBM/DRIVERS.md` | Human summary of the register | Follows driver_model.py; if they diverge, driver_model.py governs. |
| `analysis/IBM/REVIEW.md` (F1–F19) + `DRIVER-REVIEW.md` dispositions | **BINDING** | Signed-off dispositions and rejections bind all future work. **Never resurrect a retracted claim** (v1 "fairly valued at $212", the 4.1% kd, the 7.5% top-of-band WACC, the RPO parameter changes — all retired/rejected). Never drop a disclosed caveat (tail caveat, F19 conditional, data notes). Re-litigating a disposition requires a review of equal weight. |
| `PLANNING/reusable-dcf-vision.md` | **NORMATIVE roadmap** | 7 layers, amendments ✦1–✦7, phasing P1–P4. Managed as a standing initiative; charter review on every run is non-negotiable; Luc is the accountability terminus (✦6a). |
| `PLANNING/RULES.md` | Rulebook (L4 ratchet) | R1 + the extraction backlog. Every accepted finding must generate a validator and a red-team specimen. |
| `analysis/IBM/report/SOURCE-AUDIT.md` | Audit record | The claim→source table and the three softens (E1–E3) are part of the shipped report's provenance; don't undo them. |
| **The ENGINE** (`dcf_drivers_run.py` etc., seed 7) | Governs over stale doc numbers | Any published number must reproduce from the stated script/seed. Doc numbers that don't reproduce are stale docs, not engine bugs — see `docs/HIDDEN-ASSUMPTIONS.md` #14 for the known v3 store-drift case. |
| `README.md`, `context.json`, `docs/*` | Informational | Kept current, but never override the above. |

Nothing gets deleted in this repo's records: superseded content is updated in place or
marked superseded with a pointer (REVIEW.md's whole value is that the errors stay visible).

## Orchestration norms (learned the hard way)

- **Subagents do NOT commit.** The orchestrator commits, on feature branches off
  origin/main; PRs merge to main. No pushes/commits unless Luc asked.
- **Parallel file work uses git worktrees** (`C:\Users\ouell\FinAnalysis\wt-*`). A
  shared-tree branch switch once misplaced a commit; don't switch branches under a
  concurrent agent. Existing `wt-*` trees are stale/disposable (HANDOFF.md).
- **Branch awareness**: finished work may live unmerged on another branch (the backtest
  harness sat on `factory-builds` for hours before PR #7 merged it). Check
  `git log --all -- <path>` before declaring something missing.
- **Review a COPY for .docx reads** — officecli normalizes (mutates) a .docx on open.
  `cp doc /tmp/review.docx` first. officecli is at `~/.local/bin`
  (`export PATH="$HOME/.local/bin:$PATH"`).
- **A PostToolUse hook fires whenever a .docx is produced** (any tool: officecli,
  pandoc, python-docx, libreoffice) demanding a memo-reviewer pass before commit/merge.
  A SessionStart hook checks officecli presence (check-only; install is manual
  `bash scripts/setup-officecli.sh --install`).
- **Fixed seeds (7) everywhere.** `SCENARIO_SEED=7`, driver runs seed 7, backtest seed 7.
  Published numbers must be reproducible; never publish from an unseeded run.
- **Background/timed tasks can be killed mid-run** (session/token limits). Persist
  resume-state in files as you go — `PLANNING/HANDOFF.md` is the pattern (live work-state,
  delete items as completed, delete the file when empty). Interrupted fan-outs resume,
  never restart (vision ✦6b), and publication is hard-blocked while any load-bearing
  review is incomplete.
- **Token ledger**: update `PLANNING/token-ledger.md` per orchestrated run (success
  metric #1).

## The review pipeline (`.claude/agents/`)

`valuation-analyst` (drafts from README/REVIEW/engine; dispositions bind it) ↔
`valuation-skeptic` (adversarial; reproduces every number with the engine before judging)
→ `memo-reviewer` (craft rubric, target=good minimum; scores, never edits) →
`report-auditor` (final gate: claim→source taxonomy, re-derives every number, softens
over-claims; PASS required to ship). `FRAMEWORK.md` records the vendoring provenance
(doc-review-framework@d9d9075).

Review scopes: **frontier** (full per-driver fan-out — new templates/mappings),
**exception** (targeted — validator violations, staleness, new evidence; the RPO re-look
is the model: ~160k tokens vs 2.6M frontier), **charter** (every run, the applied
rules+assumptions as a package, must ACCEPT before publication).

## Conventions

- **Tests**: pytest, no network, synthetic fixtures; 309 collected post-merge
  2026-07-17 (~3 min — the backtest tests are the slow ones). Run `pytest -q` for the
  current count. Every ticket closes with a live smoke test.
- **Never commit**: the tracked-but-gitignored churn (NVDA/PANW xlsx + 4 .pyc — see
  HIDDEN-ASSUMPTIONS #18), `.env`, anything under `data/`.
- **LF/CRLF warnings are normal** (`core.autocrlf=true`). Windows PowerShell 5.1 is the
  primary shell: no `&&`, no ternary, UTF-16 default encoding — prefer Git Bash for
  POSIX scripts.
- **Python 3.12**; run everything from the repo root as modules (`python -m scripts.analyze IBM`).

## Pointer table — every document

| Path | What it is |
|---|---|
| `README.md` | Pipeline overview, commands, xlsx structure, IBM headline |
| `context.json` | Machine-readable orientation summary + phase state |
| `CLAUDE.md` | This file |
| `docs/ARCHITECTURE.md` | Code-verified data flow, module map, the two engines, backtest |
| `docs/HIDDEN-ASSUMPTIONS.md` | Trap registry (22 entries, each with evidence) |
| `docs/INDEX.md` | Catalogue of every doc with normativity + freshness |
| `docs/visualizer-design-brief.md` | Visual contract for the HTML report restyle |
| `docs/design-specs/2026-07-16-ratings-desk-blueprint.md` | Accepted visualizer design spec |
| `docs/superpowers/{plans,specs}/*` | Historical build plans/specs (Tickets 1–6 era) |
| `PLANNING/reusable-dcf-vision.md` | **Normative** factory roadmap (7 layers, ✦1–✦7) |
| `PLANNING/RULES.md` | **Normative** rulebook (R1 + extraction backlog) |
| `PLANNING/tickets.md` | Closed Tickets 1–6 (history) + current open tickets |
| `PLANNING/HANDOFF.md` | **Transient** live work-state (delete-as-completed) |
| `PLANNING/token-ledger.md` | Cost per orchestrated run |
| `analysis/IBM/README.md` | IBM v3 analysis (frozen record; published numbers are frozen — see HIDDEN-ASSUMPTIONS #14) |
| `analysis/IBM/REVIEW.md` | **Binding** v1→v3 findings ledger F1–F19 + deferred tickets |
| `analysis/IBM/DRIVERS.md` | Human register mirroring driver_model.py (v4) |
| `analysis/IBM/DRIVER-REVIEW.md` | **Binding** 19+4 review verdicts, consolidations, RPO re-look |
| `analysis/IBM/research-ledger.md` | Frozen evidence with expiry (RPO — expires at Q2-26 10-Q) |
| `analysis/IBM/report/IBM-valuation-2026-07.md` | Report markdown master (docx sibling) |
| `analysis/IBM/report/SOURCE-AUDIT.md` | Audit PASS record: claim→source table, softens E1–E3, coverage flags C1–C5 |
| `.claude/agents/*.md` | Review-pipeline agent definitions + rubric + FRAMEWORK.md provenance |
| `.claude/skills/officecli/SKILL.md` | officecli usage (docx/xlsx/pptx) |
| `THOUGHTS` | Luc's raw idea scratchpad (informational) |
