---
id: T-20261123-miner-value-model-design
title: Design a value model for a mining company (net asset value), one page, before any build
area: engine
status: backlog
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 6.1
story: E3.S5
branch:
created: 2026-11-23
acceptance: A design of one page is committed before any run. It gives the inputs (reserves and resources, the price curve of the commodity, the costs, the mine life), the value formula (a net asset value), the data source of each input, and the test that will judge the model, with its pass rule written before the first run. The person reads the design before a build starts.
---
Why: the person read the URA page of the Head of Economics (`analysis/macro/2026-11-23-ura-one-pager.md`) and said, on its
limits section: "On the limitations section can we fill that out/begin work on removing limitations." (2026-11-23, relayed by
the Head of Staff). The first limit of that page is a limit of the model: the engine values the present cash flows of a
company from its last filings. The value of a miner comes from its reserves and from the price of the commodity.

Evidence of the limit (`fa dcf implied`, main 6c59388, store a798b9b, close of 2026-11-20): UEC has a model value of
-14.16 a share at a price of 5.30, and the search has no solution. UUUU has a model value below zero. CCJ, NXE and DNN have
"no data" (Canadian filers, form 40-F, no us-gaap facts). OKLO has no revenue and no model. Only LEU (Centrus, an operating
company) has implied numbers.

Scope of this ticket: the design only. No code, no run, no value of a company.
1. The inputs and the source of each: reserves and resources by mine and by class (the filings: Regulation S-K 1300
   tables for a US filer; NI 43-101 reports for a Canadian filer), the production plan and the mine life, the cash cost
   and the capital cost, the price of the commodity (a spot price and a forward or term price), the tax and the royalty.
2. The formula: the net asset value of each mine (the discounted cash flow of the mine plan at a price curve), plus the
   value of resources outside the plan, less the net debt and the corporate costs. The discount rate and its record.
3. What the model does not do, and its place next to `fa dcf auto` (a separate evaluator, as for a bank or a fund).
4. The data that Technology must add (the reserve tables, a commodity price series, the facts of the 40-F filers).
5. The test that judges the model, written before a run: for example the value of each mine against the value in the
   company's own technical report at the same price, and a point-in-time rule for the inputs.
Rules: the price of a stock sets no input. A market-implied number is a measurement. No trade view.

Not before: the reset of the weekly token limit (Wednesday 2026-11-25), and after the order of the week (the S&P 100 part
after the review of the person, then the T1 correction).

Board note (15:22 UTC, 2026-11-23): created from the word of the person. Not started.
