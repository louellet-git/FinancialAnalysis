# Documentation index

Every document in the repo, its purpose, and its normativity. Inventory verified
2026-07-17. Normativity classes: **binding** (governs decisions; conflicts resolved per
the CLAUDE.md normativity map), **live** (working state — edit/delete as work
completes), **informative** (orientation; code governs on conflict), **historical**
(frozen record — do not edit, supersede), **generated/audited** (produced by the
pipeline or an audit gate; regenerate, don't hand-edit).

| Doc | Purpose | Normativity |
|---|---|---|
| `CLAUDE.md` | Agent orientation: repo identity, normativity map, orchestration norms, review-pipeline roles | binding (process) |
| `README.md` | Human-facing overview: verified CLI commands, engines, xlsx tabs, layout, tests | informative |
| `context.json` | Machine-readable project summary + phase state for agents | informative |
| `THOUGHTS` | Raw idea scratchpad (2 items, consolidated into tickets.md OPEN TICKETS) | live |
| `docs/ARCHITECTURE.md` | Code-verified module map, data flow, the two engines' mechanics | informative (code governs) |
| `docs/HIDDEN-ASSUMPTIONS.md` | Trap registry: 20 non-obvious assumptions/landmines with evidence pointers | informative (code governs) |
| `docs/INDEX.md` | This file | informative |
| `docs/visualizer-design-brief.md` | Contract for cosmetic restyles of the HTML report (structure + hard constraints) | binding (for visualizer restyles) |
| `docs/design-specs/2026-07-16-ratings-desk-blueprint.md` | Accepted visual design spec applied to `visualize_dcf.py` (tokens, chart chrome) | binding (visualizer styling) |
| `docs/superpowers/plans/2026-06-03-financial-analysis-pipeline.md` | Original pipeline implementation plan | historical |
| `docs/superpowers/plans/2026-06-05-financial-statement-automation.md` | Ticket 4/4.5 implementation plan | historical |
| `docs/superpowers/plans/2026-06-05-ticket-4-6-market-data-enrichment.md` | Ticket 4.6 implementation plan | historical |
| `docs/superpowers/specs/2026-06-04-expanded-financial-schema-design.md` | Expanded schema design | historical |
| `docs/superpowers/specs/2026-06-05-financial-statement-automation-design.md` | Statement-automation design | historical |
| `docs/superpowers/specs/2026-06-05-ticket-4-6-market-data-enrichment.md` | Market-data enrichment design | historical |
| `PLANNING/reusable-dcf-vision.md` | The valuation-factory standing initiative: 7-layer architecture, 7 amendments + basket amendment, phasing P1–P4, success metrics | binding (design) |
| `PLANNING/RULES.md` | Canonical rulebook (L4): R1 + the P1 extraction backlog; rules bind at declared scope, deprecation needs equal-weight review | binding |
| `PLANNING/tickets.md` | OPEN TICKETS (consolidated, with source pointers) + historical closed tickets 1–6 + Ticket 7 spec | live (open) / historical (closed) |
| `PLANNING/HANDOFF.md` | Session handoff — live open-work state; delete items as completed, delete file when empty | live |
| `PLANNING/token-ledger.md` | Cost-per-run ledger (success metric #1) + targets; update per run | live |
| `PLANNING/proposals/2026-10-15-investor-verdict-layer.md` | Proposal: investor verdict layer (verdict band, price-implied drivers, convergence-aware returns, scored ledger) + first shipped slice | proposed (for review) |
| `analysis/IBM/README.md` | IBM v3 analysis: fundamentals, net-debt bridge, v3 scenarios ($37/$141/$232), data notes | historical (v3 — superseded by the v4 report for headline numbers; bridge/data notes still referenced by v4) |
| `analysis/IBM/DRIVERS.md` | v4 assumption register companion table (drivers, items, correlations) | binding, EXCEPT that `driver_model.py` rationale strings govern over its prose (stated in its own header) |
| `analysis/IBM/DRIVER-REVIEW.md` | Round-1 driver/formula/matrix reviews (19 + 4 consolidations) → dispositions | binding (recorded dispositions; do not re-litigate) |
| `analysis/IBM/REVIEW.md` | v1→v3 adversarial review: findings F1–F19, dispositions, deferred tickets T-DCF-1/2, T-IMP-1 | binding (recorded dispositions; do not re-litigate) |
| `analysis/IBM/research-ledger.md` | Frozen source-tagged facts consumed by the model (RPO; expiry at Q2-2026 10-Q) | binding (evidence record, with expiry) |
| `analysis/IBM/report/IBM-valuation-2026-07.md` | The v4.1 published report (markdown master of the .docx) | historical (frozen published artifact) |
| `analysis/IBM/report/SOURCE-AUDIT.md` | Final audit gate record: claim→source table, engine re-run verification | generated/audited (frozen) |
| `.claude/agents/FRAMEWORK.md` | Vendored review-framework manifest + pipeline description | binding (process) |
| `.claude/agents/valuation-analyst.md` | Analyst agent definition (drafts reports) | binding (agent spec) |
| `.claude/agents/valuation-skeptic.md` | Adversarial reviewer agent definition | binding (agent spec) |
| `.claude/agents/memo-reviewer.md` | .docx craft reviewer (scores, never edits) | binding (agent spec) |
| `.claude/agents/memo-review-rubric.md` | Scoring rubric for memo-reviewer | binding (agent spec) |
| `.claude/agents/report-auditor.md` | Final audit gate agent (claim traceability + soften) | binding (agent spec) |
| `.claude/skills/officecli/SKILL.md` | officecli docx tooling skill | informative (tooling) |
