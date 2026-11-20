---
id: T-20261120-model-docs-backlog
title: Write the model documents and the assumption records of the other models
area: docs
status: backlog
owner: louellet-git
branch:
created: 2026-11-20
acceptance: Each model of the list has a model document (at most 120 lines) and each assumption of it has a record (at most 60 lines), and the table of the model document names each record.
---
Owner request, 2026-11-20: each model has one short model document, and each assumption
is a choice with one record. Templates: `docs/models/TEMPLATE.md` and
`docs/assumptions/TEMPLATE.md`. The first examples are `docs/models/auto-path-model.md`
and `docs/assumptions/A-001-terminal-growth.md`.

Models, one at a time, with the agent `model-documenter`:
1. The automatic path model: the other records of its table (equity risk premium,
   risk-free rate, horizon, beta, SBC).
2. The driver engine (`rust/src/dcf/driver.rs`) and the IBM driver model. Point to
   `analysis/IBM/REVIEW.md` and `DRIVER-REVIEW.md`.
3. The classic engine (`rust/src/dcf/classic.rs`).
4. The path engine (`rust/src/dcf/paths.rs`).
5. The bond reconciliation (`fa recon`).
6. The rate models (`fa models`).
7. The option backtester (`fa trade backtest`).

A person or a review decides each choice. The agent writes the record and does not make
the choice.
