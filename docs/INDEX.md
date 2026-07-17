# Documentation index

Every document in the repo, one line each. **Status**: `normative` = binds future work
(change only via review of equal weight) · `informational` = kept accurate, does not
bind · `transient` = live state, expected to change/disappear. **Freshness** = last
substantive update (verified via git/content, 2026-07-17).

| Path | Purpose (one line) | Status | Freshness |
|---|---|---|---|
| `CLAUDE.md` | Agent onboarding contract: normativity map, orchestration norms, pointer table | informational | 2026-07-17 |
| `README.md` | Pipeline overview, commands, xlsx structure, engines, IBM headline, roadmap pointer | informational | 2026-07-17 |
| `context.json` | Machine-readable orientation: summary, phase state, entry points, normativity pointers | informational | 2026-07-17 |
| `docs/ARCHITECTURE.md` | Code-verified data flow, module map, the two engines, backtest harness | informational | 2026-07-17 |
| `docs/HIDDEN-ASSUMPTIONS.md` | Trap registry — 22 verified traps with evidence pointers | informational | 2026-07-17 |
| `docs/INDEX.md` | This catalogue | informational | 2026-07-17 |
| `docs/visualizer-design-brief.md` | Contract for cosmetic restyles of the HTML company report (hard constraints included) | informational | 2026-07-16 |
| `docs/design-specs/2026-07-16-ratings-desk-blueprint.md` | ACCEPTED visualizer design spec (applied; "Ledger serif" alternative not applied) | informational | 2026-07-16 |
| `docs/superpowers/plans/*` (3 files) | Historical implementation plans from the Tickets 1–6 build-out (June 2026) | informational (historical) | 2026-06-03/05 |
| `docs/superpowers/specs/*` (3 files) | Historical design specs from the same build-out | informational (historical) | 2026-06-04/05 |
| `PLANNING/reusable-dcf-vision.md` | The valuation-factory roadmap: 7 layers, amendments ✦1–✦7, phasing, metrics | **normative** | 2026-07-17 (✦1 basket amendment) |
| `PLANNING/RULES.md` | Rulebook (L4 ratchet): R1 + extraction backlog; every rule → validator + red-team specimen | **normative** | 2026-07-17 |
| `PLANNING/tickets.md` | Current open tickets (consolidated) + closed Tickets 1–6 historical record + Ticket 7 spec | informational (open list) / historical | 2026-07-17 |
| `PLANNING/HANDOFF.md` | Live open-work state for the next session — delete items as completed, delete file when empty | **transient** | 2026-07-17 |
| `PLANNING/token-ledger.md` | Cost per orchestrated run (success metric #1) + compression targets | informational | 2026-07-17 |
| `analysis/IBM/README.md` | IBM v3 analysis record (frozen: Bear $37 / Base $141 / Bull $232; fresh runs drift — HIDDEN-ASSUMPTIONS #14) | **normative** (frozen record) | 2026-07-16 |
| `analysis/IBM/REVIEW.md` | v1→v3 adversarial findings ledger F1–F19 + deferred tickets; dispositions bind | **normative** | 2026-07-16 |
| `analysis/IBM/DRIVERS.md` | Human assumption register for the v4 driver model (driver_model.py governs on divergence) | **normative** (summary of register) | 2026-07-17 (RPO footnotes) |
| `analysis/IBM/DRIVER-REVIEW.md` | 19 round-1 reviews → dispositions → 4 consolidations → RPO re-look; rejections bind | **normative** | 2026-07-17 |
| `analysis/IBM/research-ledger.md` | Frozen source-tagged evidence (RPO block) — **expires at the Q2-2026 10-Q** | **normative** (with expiry) | 2026-07-17 |
| `analysis/IBM/report/IBM-valuation-2026-07.md` | Report markdown master (content-identical to the shipped .docx) | informational (shipped artifact) | 2026-07-17 (v4.1) |
| `analysis/IBM/report/SOURCE-AUDIT.md` | Report-auditor PASS record: claim→source table, softens E1–E3, coverage flags C1–C5 | **normative** (audit record) | 2026-07-16 |
| `.claude/agents/FRAMEWORK.md` | Vendoring provenance of the doc-review framework (@d9d9075) + pipeline order | informational | 2026-07-16 |
| `.claude/agents/valuation-analyst.md` | Analyst agent: drafts reports; REVIEW.md dispositions bind it | informational (agent def) | 2026-07-16 |
| `.claude/agents/valuation-skeptic.md` | Skeptic agent: reproduces every number with the engine before judging | informational (agent def) | 2026-07-16 |
| `.claude/agents/memo-reviewer.md` | Craft reviewer for .docx (reviews a COPY — officecli mutates on open) | informational (agent def) | 2026-07-16 |
| `.claude/agents/memo-review-rubric.md` | The 7-category scoring rubric memo-reviewer applies | informational | 2026-07-16 |
| `.claude/agents/report-auditor.md` | Final gate: claim→source taxonomy, re-derivation, softening | informational (agent def) | 2026-07-16 |
| `.claude/skills/officecli/SKILL.md` | officecli usage for .docx/.xlsx/.pptx (vendored) | informational | 2026-07-16 |
| `THOUGHTS` | Luc's raw idea scratchpad (requirements.txt upkeep; news-shock scanning idea) | informational (raw notes) | pre-2026-07 |

Not documentation but worth knowing: `financial_analysis/driver_dcf.py` and
`analysis/IBM/driver_model.py` carry extensive normative docstrings/rationales — treat
those strings as part of the assumption register, not as comments.
