---
id: T-20261119-stochastic-dcf-drivers
title: Make each element of the DCF cash flows a stochastic process where it is relevant, and test the DCF
area: engine
status: review
owner: louellet-git
branch: claude/stochastic-dcf
created: 2026-11-19
acceptance: For one ticker, each driver of the future cash flows has a documented process (stochastic, or fixed with the reason), the DCF runs with these processes, and a test compares the result with fixed answers.
---
Owner request, 2026-11-19: adapt and test the DCF. The goal: each element that builds
the future cash flows follows a stochastic process where that is relevant. The owner
started this work on 2026-11-19, after the move of the engines to Rust
(`rust/src/dcf/`).

Design:

1. A new path engine in `rust/src/dcf/`. A driver model selects it in its data file. A
   model without this selection gives the same numbers as before (the IBM version 4
   numbers are a reviewed record).
2. Revenue growth of each segment is a mean-reverting process for each year of a path
   (this removes the limit "growth is one constant value for each path").
3. The margin goes to a target margin, with shocks that have a correlation with the
   growth shocks.
4. Reinvestment can come from the growth (sales-to-capital ratio), so that growth costs
   cash.
5. Taxes use the loss carry-forward for a company with losses.
6. The discount rate of each year comes from a stochastic short-rate path that agrees
   with the yield curve (`rust/src/ratemodels/`, `fa forwards`).
7. An optional failure probability for a young company.
8. Each driver has a documented process: stochastic, or fixed with the reason.

Tests: fixed answers with all shocks off (closed form), limit cases that give the old
engine, simulated moments against the formulas, discount factors against the curve.

The process parameters come from data where possible: ticket
`T-20261119-dcf-process-calibration`.

State, 2026-11-19 (not yet committed or merged): the path engine is built (`rust/src/dcf/paths.rs`,
`process.rs`, `pathrate.rs`, `paths_cli.rs`), with 27 tests in `paths_tests.rs` for the seven test groups
of the design. The command is `fa dcf drivers TICKER --model driver_model_paths.toml`. The application
files are `analysis/ANT/driver_model_paths.toml` (first pass, not reviewed) and
`analysis/IBM/driver_model_paths.toml` (comparison, not a record). Open: the calibrated values for the
PLACEHOLDER numbers (ticket `T-20261119-dcf-process-calibration`), a review of the ANT path model, and
a second-pass text of ANT. Traps: `docs/HIDDEN-ASSUMPTIONS.md` #155 to #169.
