---
id: T-20261120-volatility-study
title: Study the data and the plan for the stochastic volatility models
area: engine
status: review
owner: louellet-git
branch: claude/volatility-study
created: 2026-11-20
acceptance: A findings file gives the quality of the option data for calibration (quotes for each expiry, arbitrage checks, the forward), a first roughness estimate from the 5-minute bars, and a sequence of tickets for the models with the test of each one.
---
Owner request, 2026-11-20: investigate the recommendations. The owner wants to learn
the more advanced stochastic volatility models by testing them (Heston and Bates, SABR
and local volatility, a two-factor Bergomi model, rough Bergomi, the quintic model and
the path-dependent model, then the joint fit to SPX and VIX).

The data is free and on request since 2026-11-20: SPX, SPY and VIX option chains of
today and of past dates (Alpaca, from February 2024), the VIX history from 1990, VIX
futures, and 5-minute bars from July 2020.

It is a study. It adds no model. The plan goes on the board as tickets.

Result, 2026-11-20 (the findings are in `analysis/priors/volatility_study_2026-11-20.md`):

- The live chains support the six steps. The history of closes supports only coarse tests (section A.7 of the findings).
- The 5-minute SPY bars give a Hurst exponent of 0.147 (error 0.017). A smooth two- or three-factor model gives 0.10 to 0.17 with the same estimator: the estimate alone does not show roughness.
- Seven tickets are on the board: `T-20261120-vol-1-heston-bates` to `T-20261120-vol-7-chain-copy`.
- One defect is not corrected and is in ticket 7: the roots SPX and SPXW of one date are mixed in `fill_from_mids`.
- Open: the live VIX test (the VIX value at the time of the snapshot is not in a file).
- Set the status to `review` when the pull request is open.
