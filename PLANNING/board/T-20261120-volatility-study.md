---
id: T-20261120-volatility-study
title: Study the data and the plan for the stochastic volatility models
area: engine
status: in-progress
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
