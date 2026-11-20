---
id: T-20261025-bridge-per-ticker-rule
title: Add bridge-per-ticker rule and validator
area: planning
status: backlog
owner: louellet-git
head: quality
order: 4
branch:
created: 2026-10-25
acceptance: RULES.md carries a rule requiring a reviewed net-debt bridge per basket ticker, and a validator fails a ticker without one.
---
The default capital-structure derivation differs from the reviewed IBM bridge (candidate RULES.md R2).
Input from the mapping audit, 2026-11-19: the default net debt now uses the standard variable `total_debt`. The two estimates of total debt disagree by more than 2% for IBM, JNJ, HD, CVX and CSCO (`python -m scripts.mapping_audit --panel`). These tickers need a reviewed bridge first.
Input from the Rust test framework, 2026-11-19: on 312 US companies the estimates of total debt disagree for 47 (15%). `rust/mapcheck/target/release/mapcheck TICKER --detail` shows the estimates for a ticker.
