# Documentation index

Every document in the repo, its purpose, and its normativity. Inventory verified
2026-11-19. Normativity classes: **binding** (governs decisions; conflicts resolved per
the CLAUDE.md normativity map), **live** (working state — edit/delete as work
completes), **informative** (orientation; code governs on conflict), **historical**
(frozen record — do not edit, supersede), **generated/audited** (produced by the
pipeline or an audit gate; regenerate, don't hand-edit).

| Doc | Purpose | Normativity |
|---|---|---|
| `CLAUDE.md` | Rules for agents: purpose, source precedence, work rules, review steps | binding (process) |
| `README.md` | Overview for persons: commands, engines, trading research, xlsx tabs, layout | informative |
| `THOUGHTS` | Raw idea scratchpad (2 items, consolidated into tickets.md OPEN TICKETS) | live |
| `docs/ARCHITECTURE.md` | Code-verified module map, data flow, the two engines' mechanics | informative (code governs) |
| `docs/HIDDEN-ASSUMPTIONS.md` | Trap registry: 31 non-obvious assumptions/landmines with evidence pointers | informative (code governs) |
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
| `PLANNING/board/` | Open work, one file for each ticket; rules in its `README.md` | live |
| `PLANNING/tickets.md` | Backlog without tickets, and the Ticket 7 specification | live |
| `PLANNING/tickets-closed.md` | Closed tickets 1 to 6 (June 2026), original text | historical |
| `PLANNING/token-ledger.md` | Cost-per-run ledger (success metric #1) + targets; update per run | live |
| `PLANNING/proposals/2026-10-15-investor-verdict-layer.md` | Proposal: investor verdict layer as five small pieces, each with a value test; piece 2 (verdict band) shipped first | proposed (for review) |
| `analysis/IBM/README.md` | IBM v3 analysis: fundamentals, net-debt bridge, v3 scenarios ($37/$141/$232), data notes | historical (v3 — superseded by the v4 report for headline numbers; bridge/data notes still referenced by v4) |
| `analysis/IBM/DRIVERS.md` | v4 assumption register companion table (drivers, items, correlations) | binding, EXCEPT that `driver_model.py` rationale strings govern over its prose (stated in its own header) |
| `analysis/IBM/DRIVER-REVIEW.md` | Round-1 driver/formula/matrix reviews (19 + 4 consolidations) → dispositions | binding (recorded dispositions; do not re-litigate) |
| `analysis/IBM/REVIEW.md` | v1→v3 adversarial review: findings F1–F19, dispositions, deferred tickets T-DCF-1/2, T-IMP-1 | binding (recorded dispositions; do not re-litigate) |
| `analysis/IBM/research-ledger.md` | Frozen source-tagged facts consumed by the model (RPO; expiry at Q2-2026 10-Q) | binding (evidence record, with expiry) |
| `analysis/JNJ/FIRST-PASS.md` | JNJ first pass (unreviewed): potential verdict, engine-vs-hand-DCF check, six pipeline defects, template gaps | informative (first pass) |
| `analysis/IBM/report/IBM-valuation-2026-07.md` | The v4.1 published report (markdown master of the .docx) | historical (frozen published artifact) |
| `analysis/IBM/report/SOURCE-AUDIT.md` | Final audit gate record: claim→source table, engine re-run verification | generated/audited (frozen) |
| `trading/research/LEADERBOARD.md` | Results and limits of the 22 option bots (model prices) | informative (research record) |
| `trading/research/s*.md` | One research note for each option bot | informative (research record) |
