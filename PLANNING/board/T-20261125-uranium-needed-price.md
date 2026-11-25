---
id: T-20261125-uranium-needed-price
title: Read the uranium price that the price of a miner needs, from the value tables of the technical reports (no new code)
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 6.05
story: E3.S5
branch: claude/uranium-needed-price
created: 2026-11-25
acceptance: One page for the person in his report style. Step 1, a table: for each URA miner that was checked, which technical report has an after-tax value at several uranium prices (the document, its date, its page). Step 2, for each name that has such a table: the flat uranium price at which the sum of the mine values, the cash and the uranium held, less the debt, equals the market value at a stated date, with the spot price next to it. Each number has its source and page. The limits are said plainly. One audit of the numbers passes before the page goes to the person.
---
Why: the person read the design of the miner value model (`analysis/priors/miner_value_model_design_2026-11-25.md`) and
chose, in a dialog of the Head of Staff on 2026-11-25: "Small step first (Recommended)" (record of the Head of Staff). The
choices were: this small step; the build of the model; stop. He decides on the build after he sees the number.

Scope: read only. No evaluator code, no build, no bulk download, no valuation and no trade view. The number is a
measurement: the price of the commodity that the stock price needs. It is never an input.
Step 1 (the check, first): UEC, then NexGen, Denison, Cameco and Energy Fuels, as far as the documents are easy to get.
Step 2 (the measurement): by interpolation in the table of each report; no extrapolation outside a table.
Limits to say on the page: one date, by hand; each report has its own rate, cost year and date, so the mines are not on
one basis; resources outside a plan get no value, so the needed price is too high; a table that does not reach the
needed price gives no answer.
Cost: about 1% to 2% of the weekly token limit, from the reserve of the week (not from T1). One analyst agent, one audit.

Board note (16:41 UTC, 2026-11-25): created and in progress (louellet-git, branch claude/uranium-needed-price).

Board note (16:50 UTC, 2026-11-25): the page and its sources file are written (`analysis/priors/uranium_needed_price_2026-11-25.md`,
PDF in `analysis/priors/pdf/`, one page; `uranium_needed_price_sources_2026-11-25.md`). Result of the check: 1 of 5
miners (NexGen) prints its mine value at several uranium prices. Result of the measurement: the close of NexGen of
2026-11-20 (US$6.40) needs a flat uranium price of about US$84 a pound (US$80 to US$85 by the exchange rate and the
debt number; US$70 with the costs of the 2021 study); spot US$91.55 and long-term US$98.00 on 2026-10-30. Agents: one
research agent (30 tool calls) and one check of the NexGen filing. NEXT: the audit of the numbers (Quality), then the
Head of Staff, then the person. The design ticket `T-20261123-miner-value-model-design` is merged (pull request 231):
move it to `done/` with the pull request of this ticket.
