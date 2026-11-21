---
id: T-20261121-quant-model-docs
title: Model documents and assumption records for the quant models
area: docs
status: in-progress
owner: louellet-git
head: quant
order: 2.9
branch: claude/quant-model-docs
created: 2026-11-21
requested_by: louellet-git
acceptance: Each merged quant model has a document in `docs/models/` (at most 120 lines) and each assumption a record in `docs/assumptions/` (at most 60 lines, with its source), as CLAUDE.md "Model documents" and docs/QUALITY-STANDARDS.md section 4 say. A document-reviewer pass.
---
Owner request through the Head of Technology, 2026-11-21. Record numbers of the quant area: A-100 to A-159.

Part 1 (this ticket, merged models):
- Rates: the fitted Treasury curve and rich and cheap (`fa recon bonds`), the short-rate models (`fa models`). Records A-100 to A-119. The credit spread and cost of debt (`fa bonds`, `--rating`) are out: the Head of Technology writes them with its change on `claude/fred-primary` (A-200 onward).
- Options: the static arbitrage check (`fa options check`), the implied volatility and the parity forward, the futures check (`fa futures check`), the records of the Heston and Bates choices. Records A-120 to A-139.
- Option bots: the backtest and the model price of the option legs (`fa trade backtest`), the paper trader (`fa trade paper`). Records A-140 to A-159.

Part 2 (on their own branches, when they move): `fa algo` (PR 131), `fa trade premium` (alpha-1), SABR (vol-2).
