---
id: T-20261125-forecast-freeze
title: Freeze the forecasts of today for each company, to score them on the fiscal 2026 reports
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.021
branch: claude/t1-forecast-freeze
created: 2026-11-25
acceptance: A dated file on main has, for each company of the standard template, the P10, P50 and P90 of revenue and free cash flow for the 3 years after its last fiscal year, with the engine commit, the store commit, the table, the rules and the seed; the scoring rule is written beside it before any result exists.
---
Words of the person (`chiefs/louellet-git/staff/RECORD.md`, 2026-11-25 about 20:33 UTC): "Fix the middle and freeze
(Recommended)". Part (a), this week: save the forecasts of today and write the scoring rule now. Part (b), the work on
the middle, is for the week after 2026-12-02 (on `T-20261121-blind-projection-test`). The person decides later which
test is the pass for trades; this ticket does not change the rule "No trades until T1 passes".

Plan: one option `fa dcf pit --freeze FILE` (Rust, with a test), one run on the data store, the file and its rule in
`analysis/priors/forecast_freeze_2026-11-25/`. No agent.
