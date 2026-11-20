---
id: T-20261120-dcf-market-implied
title: Show what the market price implies for each stock, industry and sector
area: engine
status: backlog
owner: louellet-git
head: fundamental
order: 1.2
branch:
created: 2026-11-20
acceptance: `fa dcf implied` gives, for each ticker with a price, the cost of capital, the revenue growth and the margin that the price implies, with the model value of each, and the medians by sector, industry and size; two runs give the same numbers; a findings file shows the first run.
---
Owner request, 2026-11-20: "we use mark to market to see what the market is implying across industries,
sectors, stocks, and that's where the engine comes in. Deterministic way that we can do that."

Rule: a market-implied value is a measurement. It is not an input of the model
(`docs/QUALITY-STANDARDS.md`, section 6).

Parts, one at a time:
1. For one ticker, a fixed procedure with no random part (the path at the means,
   `PathEngine::deterministic`): the cost of capital that gives the price; the revenue growth of 10
   years that gives the price; the margin that gives the price. Each search changes one number and keeps
   the others at the model values. A result outside the search range is a bound, not a number.
2. For a list of tickers: one CSV row for each ticker, and the medians and quartiles by sector, industry
   (SIC code) and size, with the count of each group.
3. The premium that the whole list implies (one number), and the values of each group against it.
4. A findings file of the first run (skill `investigation-report`).

Needs: the SIC code of each ticker (ticket `T-20261120-sic-code`, head: technology). Part 1 and the size
groups do not need it.
