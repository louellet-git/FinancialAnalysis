---
id: T-20261119-dcf-process-calibration
title: Estimate the process parameters of growth and margin from the company data
area: engine
status: review
owner: louellet-git
branch: claude/dcf-calibration
created: 2026-11-19
acceptance: One command gives, from the cached company data, the mean reversion and the volatility of revenue growth and of the margins by company size and sector, with a test on synthetic data that has known parameters.
---
This ticket supports `T-20261119-stochastic-dcf-drivers`. The path engine needs
parameters for each process: the speed of the return to the long-run value, the
long-run value, and the size of the shocks. The cache has the standard variables of
about 1,000 companies (`rust/mapcheck`). This ticket estimates the parameters from
this panel, so that a model starts from measured values and not from a guess.

It also measures the forecast bands: for past dates, how often the later real value
was inside the band of the model (`fa backtest`).

State, 2026-11-19. Done: the panel (`rust/src/dcf/panel.rs`), the estimators and the table
(`rust/src/dcf/calibrate.rs`, `fa dcf calibrate`, `analysis/priors/process_priors_2026-11-19.toml`), the tests on
synthetic panels, and the band coverage (`rust/src/dcf/calib_bands.rs`). The traps are #170 to #184 of
`docs/HIDDEN-ASSUMPTIONS.md`. Open: the cache has no SIC code, so no sector cells; a company like ANT is
outside the data (#182); one AR(1) does not describe fast growers (#176, #177). The path engine decides which
form of process it uses.

