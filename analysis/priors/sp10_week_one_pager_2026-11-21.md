# The S&P 10 in the week to 2026-11-20: does the explanation hold for each stock?

PRELIMINARY, for the review of the owner. Close of 2026-11-13 to close of 2026-11-20 (all 5 days). Of record: the prices
and the SEC facts (store 0340bc1). NOT of record: the betas and the DCF numbers (code at the Quality gate, branch
`claude/attribution-reality`) and the test sums (shell calculations; `analysis/priors/sp10_week_tests_2026-11-21.md`).

**Answer.** Two things explain the 10 stocks: the market, and the chip sector. Monday to Thursday (SPY -5.1%) was a
market-wide fall after the rise of the yields of the week before. Friday 2026-11-20 (SPY -3.6%, about 40% of the week) was
a fall of the chip and AI-hardware companies, while the yields fell and the other sectors were flat. With the market and
the sector, no stock of the 10 is left outside the level of chance. **One thing is not explained: the cause of the chip
fall of Friday. We tested for it and found no cause in our data.**

| | Mon 16 | Tue 17 | Wed 18 | Thu 19 | **Fri 20** | Week | Rest, in deviations (SPY; equal weight) |
|---|---:|---:|---:|---:|---:|---:|---:|
| SPY | -1.9 | -1.0 | -0.3 | -2.0 | **-3.6** | -8.6 | |
| S&P 500, equal weight | -1.2 | -1.4 | -0.1 | -1.8 | **-2.3** | -6.6 | |
| 6 other chip companies | -5.4 | +0.2 | +1.1 | -2.7 | **-11.0** | -17.0 | |
| NVDA | -4.1 | -0.4 | -2.9 | -3.3 | **-14.0** | -22.9 | -2.2; -3.0 |
| AVGO | -3.9 | +2.5 | +3.6 | -2.1 | **-12.2** | -12.3 | +0.2; -0.4 |
| TSLA | -2.7 | +0.1 | +1.0 | -2.4 | **-7.6** | -11.4 | +0.4; -0.2 |
| META | -2.4 | -1.2 | -0.4 | -3.3 | **-5.9** | -12.6 | -0.4; -1.0 |
| AMZN | -3.3 | +0.1 | +3.8 | -3.9 | **-4.1** | -7.4 | +1.0; +0.3 |
| MSFT | -1.1 | -2.3 | +0.4 | -2.0 | **-2.8** | -7.5 | +0.5; -0.1 |
| GOOG | -3.9 | -1.3 | +0.6 | -2.9 | **-2.6** | -9.8 | 0.0; -0.6 |
| AAPL | -1.0 | -0.3 | +0.2 | -0.9 | **-1.9** | -3.8 | +1.6; +1.0 |
| BRK-B | 0.0 | -1.0 | -2.1 | -0.5 | **-0.4** | -3.9 | +1.1; +0.4 |
| LLY | -0.8 | -1.1 | -2.3 | -2.3 | **-0.3** | -6.7 | -0.2; -0.4 |

Changes in percent. The rest of a stock = its change less its beta times the market, in units of the usual weekly
deviation of the rest (104 weeks). SPY contains the 10 (NVDA moved it), so the equal weight of the S&P 500 is the second
market. By chance alone, about 3 of 10 stocks are outside 1 deviation.

**Each reason, with its test and its result** (the tests: the evidence file).
- **NVDA, hit more than the market (3.0 deviations). PROVEN: it is the fall of its sector.** The 7 chip companies of the
  S&P 100 fell 17.8% on average; MU (-23.6%) and AMD (-20.5%) fell as much with no report. NVDA alone, against the other
  chips: -5.9 points, 1.5 deviations. "Its own report of 11-17 is the reason": REFUTED as the main reason (the chips rose
  the day after the report and fell three days later).
- **AAPL, hit less (1.0 to 1.6 deviations). TESTED, NO CAUSE FOUND; it is at the level of chance.** "The price had not
  risen with its annual report": REFUTED (it rose 7.3% from 11-03 to 11-13 with a flat market). New facts do not rank the
  falls (79 companies, r +0.06). AAPL does not fall less than its beta in falling weeks (4 weeks: mean rest -1.3%).
- **BRK-B and AMZN, hit less against SPY only.** Against a market that does not contain them: +0.4 and +0.3 deviations.
  No effect to explain. "Less growth needed, less to lose" (BRK-B): REFUTED as a cause of its own; across 82 companies
  the needed growth ranks the falls (r -0.47), but not outside the technology sector (r -0.09): it is the sector.
- **The 10 fell more than the average stock (-9.96% against -6.56%). PROVEN as a sector effect:** 6 of the 10 are in the
  sector of computers, chips and software (-11.9%).

**Friday 2026-11-20: tested, no cause found.** Tested: earnings filings of the 10 on 11-19 and 11-20 (none); rates (the
10-year yield fell 12 bp, the real yield 11 bp: not a rate shock); the NVDA report (three days before). Sectors of the
S&P 100 that day: computers and chips -7.0%, cars -5.7%, health -0.1%, retail 0.0%, energy +0.8%, telecoms +1.4%;
software fell little (CRM -1.5%). Missing data, asked for: SEC 8-K filings of other kinds of 11-19 and 11-20 for the chip
companies (Head of Technology); the daily macro view and the calendar (Head of Economics); the VIX close; news of the day.

This page is a reading from fundamentals, not a trade rule. Next, after the review of the owner: the S&P 100.
