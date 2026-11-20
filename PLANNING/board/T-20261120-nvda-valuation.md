---
id: T-20261120-nvda-valuation
title: Value NVDA with a reviewed driver model
area: ticker:NVDA
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.1
branch: claude/nvda-valuation
created: 2026-11-20
acceptance: `analysis/NVDA/driver_model.toml` and a report give the value range of NVDA for each share with each driver and its source, the report passed the four steps of the review (analyst, skeptic, reviewer, auditor), and the owner approved it.
---
Owner word, 2026-11-20 23:21 UTC: "I want to know what NVDA is worth." It is a new ticker: the owner approves the
report at the end (`CLAUDE.md`, "Review of a valuation report").

Why a driver model and not the automatic model: the automatic model gives NVDA a value far below the price
because the level rule of the growth shock makes the noise of its history large, so the growth mean is mostly the
cell mean (`analysis/priors/dcf_investigation_2026-11-20.md`, section 4.8: company mean 10.6% against a history of
65% to 126%). A company in a state of very high growth needs a model of its drivers (data-center demand, share,
margin, the cycle of capital spend of its customers).

Data: the data store (`data/raw`), plus the newest NVDA filings and prices (request to the Head of Technology,
2026-11-20). Reported results of the fiscal third quarter of 2027 (8-K of 2026-11-17, SEC address
`www.sec.gov/Archives/edgar/data/0001045810/000104581026000088/q3fy27pr.htm`): read the filing, not a summary.
