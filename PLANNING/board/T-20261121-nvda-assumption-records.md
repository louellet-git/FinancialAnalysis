---
id: T-20261121-nvda-assumption-records
title: Write the assumption records of the NVDA valuation drivers
area: docs
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: quality
order: 1.5
branch: claude/nvda-assumption-records
created: 2026-11-21
acceptance: Each of the NVDA drivers D3, D7, D11, D14 and D20 has a record in `docs/assumptions/` (at most 60 lines) that gives the value, the source, the alternatives with their P50 and the decision, and the document reviewer passed them.
---
Request of the Head of Fundamental Research (NVDA review, step 2, MIN 7); the person agreed on 2026-11-21.
Source: `analysis/NVDA/driver_model_paths.toml` and `analysis/NVDA/REPORT-DRAFT.md` of commit 5ffe7a0 on
`claude/nvda-valuation`. The values can move until the skeptic accepts; a record then follows its value in
the same pull request as the value.
