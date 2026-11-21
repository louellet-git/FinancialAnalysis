# The S&P 10 in the week to 2026-11-20: does the explanation hold for each stock?

PRELIMINARY, for the review of the owner. Close of 2026-11-13 to close of 2026-11-20 (all 5 days). Of record: the prices
and the SEC facts (store 0340bc1). NOT of record: the betas and the DCF numbers (code at the Quality gate, branch
`claude/attribution-reality`) and the usual weekly deviation (a shell calculation, not yet in the engine).

**Answer.** The week has two parts. Monday to Thursday (SPY -5.1%) was a market-wide fall after the rise of the yields of
the week before. Friday 2026-11-20 (SPY -3.6%, about 40% of the week) was NOT market-wide: it was a fall of the chip and
AI-hardware companies (NVDA -14.0%, AVGO -12.2%; in the S&P 100: SNDK -16.1%, MU -13.9%, AMD -13.2%, ANET -11.6%,
DELL -11.0%, AMAT -10.7%), while the yields fell and the other sectors were flat. **We do not have the cause of Friday.**
For the week, the market-wide explanation holds for most of the 10; NVDA was hit more, and AAPL less (weak).

| | Mon 16 | Tue 17 | Wed 18 | Thu 19 | **Fri 20** | Week | Rest, in deviations (SPY; equal weight) | Result |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| SPY | -1.9 | -1.0 | -0.3 | -2.0 | **-3.6** | -8.6 | | |
| S&P 500, equal weight | -1.2 | -1.4 | -0.1 | -1.8 | **-2.3** | -6.6 | | |
| NVDA | -4.1 | -0.4 | -2.9 | -3.3 | **-14.0** | -22.9 | -2.2; -3.0 | **Hit more** |
| AVGO | -3.9 | +2.5 | +3.6 | -2.1 | **-12.2** | -12.3 | +0.2; -0.4 | Holds (all on Friday) |
| TSLA | -2.7 | +0.1 | +1.0 | -2.4 | **-7.6** | -11.4 | +0.4; -0.2 | Holds |
| META | -2.4 | -1.2 | -0.4 | -3.3 | **-5.9** | -12.6 | -0.4; -1.0 | Chance level |
| AMZN | -3.3 | +0.1 | +3.8 | -3.9 | **-4.1** | -7.4 | +1.0; +0.3 | Chance level |
| MSFT | -1.1 | -2.3 | +0.4 | -2.0 | **-2.8** | -7.5 | +0.5; -0.1 | Holds |
| GOOG | -3.9 | -1.3 | +0.6 | -2.9 | **-2.6** | -9.8 | 0.0; -0.6 | Holds |
| AAPL | -1.0 | -0.3 | +0.2 | -0.9 | **-1.9** | -3.8 | +1.6; +1.0 | **Hit less** (weak) |
| BRK-B | 0.0 | -1.0 | -2.1 | -0.5 | **-0.4** | -3.9 | +1.1; +0.4 | Chance level |
| LLY | -0.8 | -1.1 | -2.3 | -2.3 | **-0.3** | -6.7 | -0.2; -0.4 | Holds |

Changes in percent. The test: a stock falls by its beta times the market; the rest is in units of the usual weekly
deviation of the rest (104 weeks). Rule set before the table: holds below 1 deviation. By chance, about 3 of 10 are outside.
SPY contains the 10 (NVDA moved it), so the equal weight of the S&P 500 is the second market.

**Friday 2026-11-20, what our data shows.**
- The S&P 10 fell about 5.2% on the day; NVDA and AVGO gave about 2.8 points of it. S&P 100 by sector (equal weight):
  computers, chips and software -7.0%; cars -5.7%; manufacturing -4.0%; finance -2.3%; health -0.1%; retail 0.0%; energy
  +0.8%; telecoms +1.4%. Software fell little (CRM -1.5%, NOW -1.4%): the fall was in hardware.
- Rates: the 10-year yield fell 12 bp (5.04% to 4.92%), the 2-year 13 bp, the real 10-year 11 bp. So it was not a rate
  shock; money went to bonds.
- Filings: no earnings 8-K of the 10 on 11-19 or 11-20. The NVDA report was on 11-17 (results above its guidance, a
  lower guided margin, an interim CEO named with no reason); the stock fell 2.9% the next day and 14.0% three days later.
- **Missing data:** the VIX close of 11-20; SEC 8-K filings of other kinds (for example a change of an officer) of 11-19
  and 11-20; news of the day (a search found none for that date); intraday prices. Asked: the Head of Economics for the
  daily view, the Head of Technology for the 8-K filings of those two days.

**Other points.** AAPL, hit less: its annual report of 2026-11-03 raised the model value from $112 to $151 a share (a
possible reason, not proven). The 10 fell 3.4 points more than the average stock in the week; the prices of 8 of the 10
need a growth that large companies rarely reached, and BRK-B (normal growth) fell least. The DCF split is a second view
from the same prices, not a second proof. This page is a reading from fundamentals, not a trade rule.

Next, after the review of the owner: the same test for the S&P 100, with the count against chance (about 32 of 100).
Sources: `analysis/priors/sp100_week_prelim_results_2026-11-21.md`; daily closes of the store; `fa dcf attribute
--tickers-file rust/mapcheck/universe/sp100.txt --top 10 --from 2026-11-13 --to 2026-11-20 --reality`.
