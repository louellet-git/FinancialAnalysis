---
id: T-20261121-economics-view-engine
title: Design and build the Economics view engine (market ERP and long-run growth per date, not from the attributed prices)
area: engine
status: review
owner: louellet-git
requested_by: louellet-git
story: E2.S3
head: economics
order: 9
branch: econ/view-engine-cmd
created: 2026-11-21
acceptance: A frozen design, then a command that writes `date,d1,erp_change,growth_change,source,growth_parts` (point in time, no stock price of the attribution) that `fa dcf attribute --view-file` (planned) of Fundamental Research reads.
---
Owner words (via the Head of Fundamental Research, 2026-11-21): "For attribution, there are numbers in the valuation
that are a direct result of the math, but there are some that are implied. The implied components require the view
of the economists." and "The economists require their own engine in that regard."
Plan: layer A, survey levels (Philadelphia Fed SPF 10-year forecasts; CFO survey if terms allow); layer B, weekly
changes from the macro block of the premium test, only if that frozen test gives a "yes". Data request sent to the
Head of Technology. Dependent: Fundamental Research (`--common file`).
