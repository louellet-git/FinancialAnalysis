---
id: T-20261121-economics-premium-test-cmd
title: Build the command fa macro premium-test for the frozen premium design
area: engine
status: review
owner: louellet-git
requested_by: louellet-git
head: economics
order: 8
branch: econ/premium-test-cmd
created: 2026-11-21
acceptance: `fa macro premium-test` implements the design frozen at 55f2e92, its tests pass, and the code gate (code-reviewer, model-validator) passes before the run of record.
---
The regression command of `T-20261121-economics-premium-macro`. Input: the CSV of `fa dcf premium-series`
(Fundamental Research). Agreed with the Head of Technology: FRED files from data/local, VIX from data/raw.
