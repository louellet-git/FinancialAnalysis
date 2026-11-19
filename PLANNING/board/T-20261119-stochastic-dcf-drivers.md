---
id: T-20261119-stochastic-dcf-drivers
title: Make each element of the DCF cash flows a stochastic process where it is relevant, and test the DCF
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: For one ticker, each driver of the future cash flows has a documented process (stochastic, or fixed with the reason), the DCF runs with these processes, and a test compares the result with fixed answers.
---
Owner request, 2026-11-19: adapt and test the DCF. The goal: each element that builds the future cash flows follows a stochastic process where that is relevant. The owner set the start of this work to 2026-11-20.

Inputs that exist: the stochastic interest-rate models (`T-20261119-stochastic-rate-models`), the forward rates and zero-coupon yields (`fa forwards`), the standard variables for 1,000 companies (`rust/mapcheck`).

Rules that apply: new code is Rust. The DCF engines are legacy Python (`financial_analysis/dcf.py`, `financial_analysis/driver_dcf.py`); before a change of their function, move them to Rust (`T-20261119-python-to-rust`, step 1) and write the fixed-answer tests first (`T-20261119-engine-fixed-answer-tests`). The IBM version 4 numbers are a reviewed record and must stay the same.
