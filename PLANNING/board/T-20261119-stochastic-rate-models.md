---
id: T-20261119-stochastic-rate-models
title: Build stochastic interest-rate models with calibration to realized rates and realized volatility
area: engine
status: in-progress
owner: louellet-git
branch: claude/stochastic-rate-models
created: 2026-11-19
acceptance: One `fa` command calibrates each model to a rate history for a time interval and a sampling frequency, with more than one calibration method, and one `fa` command simulates paths from a calibrated model. For each model, a test shows that the calibration finds the parameters of simulated data, and a test compares the simulation with a known formula.
---
Owner request, 2026-11-19: a set of stochastic interest-rate models. The models need different methods of calibration to realized volatility and to realized rates, for different time intervals and sampling frequencies. Speed and quality are necessary. The code is Rust.

The models are an input of the DCF work (`T-20261119-stochastic-dcf-drivers`): the discount rate and other elements of the cash flows will follow stochastic processes.
