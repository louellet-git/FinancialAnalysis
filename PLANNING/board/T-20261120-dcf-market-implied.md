---
id: T-20261120-dcf-market-implied
title: Show what the market price implies for each stock, industry and sector
area: engine
status: review
owner: louellet-git
head: fundamental
order: 1.2
branch: claude/dcf-market-implied
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

**State, parked on the word of the owner, 2026-11-20.** Parts 1, 2 and 3 are built on the branch
(`rust/src/dcf/auto_implied.rs`, 10 new tests, 564 + 17 + 1 tests pass, two runs give the same bytes).
Run of the 218 tickers of the partial cache (A to CME, old defaults): 135 with a price, median value / price
0.67, list premium 2.16% (model 4.41%). Open, in this order:
1. The search uses the value at the means. It is above the simulated P50 (AAPL 134.53 against 128.84, ABT
   36.40 against 29.84, ADBE 289.35 against 270.91). Decide if the search must use the P50 with a fixed seed.
2. 14 of 135 tickers have a negative value and get no implied premium or growth (flag `nm`).
3. Sector and industry groups: `--groups FILE` works; the SIC code waits for `T-20261120-sic-code`.
4. The findings file (part 4) and the gate (model-validator, code-reviewer) wait for the data store of the
   Head of Technology. Merge `main` and the conventions branch first: this branch has the old defaults.

**Decision of the Head of Fundamental Research, 2026-11-20 (open point 1), corrected after review cycle 1.** The
search keeps the value at the means, because it is deterministic and stable. It is NOT the expected value. A DCF with
a risk-adjusted discount rate gives the present value of the expected cash flows, but the path at the means is not
that expected value: for most tickers it lies between the P50 and the simulated mean (median 0.89 of the mean; data of
2026-11-19, defaults of `main`, `fa dcf auto`, seed 7, 10,000 paths: AAPL $144.87 against P50 $132.89 and mean $153.05;
IBM $134.95 against $95.84 and $152.78). The mean is unstable for some tickers (one tail holds a large share). So the
size of the gap to the price depends on the statistic: value at the means / price median 0.63 (sp500), mean / price
about 0.77, P50 / price 0.477. The P50 / price of `fa dcf screen` is a different, lower measure. The findings file
says this. Follow-up for the engine ticket: report the mean of the simulated value with its standard error (done in
`fa dcf auto`, pull request 81).

**State, 2026-11-20 22:10 UTC.** Part 3b done: sector (Fama-French 12) and industry (SIC) groups from the cache
(`mapcheck::sec::company_sic`), a `*` on a group cell with more than 20% bounds. Check run on `sp500.txt` and
`next500.txt` (data store 08361bd, data of 2026-11-19, OLD defaults: a check of the code, not the result):
two runs give the same bytes; 6 s for each list. Open: merge `main` after the conventions merge, run again,
the findings file (part 4), the gate.

**State, 2026-11-20 23:00 UTC.** Part 4 done: findings file `analysis/priors/market_implied_2026-11-20.md` (first run on
`sp500.txt` and `next500.txt`, defaults of `main` with pull requests 77 and 81, data store 08361bd, data of 2026-11-19;
two runs give the same bytes). `main` merged into the branch (conflicts in `auto.rs` and four documents resolved; the
growth and margin shifts apply after the floor rule). Code: `rust/src/dcf/auto_implied.rs`,
`rust/src/dcf/auto_implied_tests.rs`. Open: the gate (skeptic, reviewer, auditor) before a person uses a table.
