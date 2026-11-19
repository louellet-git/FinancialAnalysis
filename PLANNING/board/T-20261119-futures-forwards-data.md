---
id: T-20261119-futures-forwards-data
title: Get futures and forward-rate data
area: engine
status: in-progress
owner: louellet-git
branch: claude/futures-forwards-data
created: 2026-11-19
acceptance: One `fa` command prints the forward rates for a date, and one `fa` command prints the futures prices for a date, each with its source and its date. The code is Rust and it calls no Python code.
---
Owner request, 2026-11-19: a branch that gets futures and forward data. Merge to `main` when it is done.

Scope of the first step: interest-rate data, because the next work is the bond comparison (`T-20261119-bond-recon`). Forward rates come from official sources or from the Treasury curve that the repo has. Futures prices need a source whose terms permit this use; the ticket records the source and its limits.
