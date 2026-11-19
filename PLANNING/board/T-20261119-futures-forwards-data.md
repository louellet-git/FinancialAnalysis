---
id: T-20261119-futures-forwards-data
title: Get futures prices (the forward rates are done)
area: engine
status: in-progress
owner: louellet-git
branch: claude/futures-prices
created: 2026-11-19
acceptance: One `fa` command prints the futures prices for a date, each with its source and its date. The code is Rust and it calls no Python code.
---
Owner request, 2026-11-19: a branch that gets futures and forward data. Merge to `main` when it is done.

Scope of the first step: interest-rate data, because the next work is the bond comparison (`T-20261119-bond-recon`). Forward rates come from official sources or from the Treasury curve that the repo has. Futures prices need a source whose terms permit this use; the ticket records the source and its limits.

## State, 2026-11-19

Done (Rust, `rust/src/forwards.rs`): `fa forwards [--date D]` and `fa forwards check [--days N]`.
Source: Federal Reserve Board GSW file `feds200628.csv` (daily, from 1961-06-14).

Not done: futures prices. No free source was found whose terms permit automated use.
- CME Group website data: the CME Data Terms of Use allow personal, non-commercial use in
  the form presented on the website and forbid data mining with scripts, software, spiders
  and robots. Do not scrape it.
- Databento (CME Globex data, licensed distributor): needs an API key and gives a one-time
  credit of USD 125 for historical data. After the credit it is paid. Terms not read in full.
- Yahoo endpoints: not approved by the owners.
- Third-party gateways that resell Yahoo data: terms not published.

Owner decision needed: pay for a licensed source (for example Databento, key in an
environment variable), or accept no futures data. The acceptance line for the futures
command is not met.

Owner decision, 2026-11-19: use each free source that gives data of good quality. The work continues on the branch `claude/futures-prices`.
