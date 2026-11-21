# The S&P 10 in the week to 2026-11-20: does the explanation hold for each stock?

PRELIMINARY, for the review of the owner. Close of 2026-11-13 to close of 2026-11-20. Of record: the prices and the SEC
facts (data store 0340bc1). NOT of record: the model numbers (the code is at the Quality gate; branch
`claude/attribution-reality`), and the usual weekly deviation (a shell calculation, not yet in the engine).

**The explanation.** The 10 largest US companies fell 9.96% (value-weighted); the market (SPY) fell 8.56%. The fall was
market-wide: the return that the price asks (the implied premium) rose for all 68 S&P 100 companies with a value. The
yields rose the week before (real 10-year +28 bp), the stocks did not follow then, and they followed in this week
(Head of Economics: one fair reading, not proven). No company of the 10 filed new accounts in the week.

**The test.** If the fall is market-wide, a stock falls by its beta times the market. "Hit more" or "hit less" is the
rest. The rule, set before the table: the explanation holds when the rest is smaller than 1 usual weekly deviation of
the stock (104 weeks before the week).

| Stock | Weight | Change | Beta | Expected | Rest (points) | Rest (deviations) | Result |
|---|---:|---:|---:|---:|---:|---:|---|
| NVDA | 14.7% | -22.9% | 1.67 | -14.3% | -8.6 | -2.2 | **Hit more** |
| META | 6.1% | -12.6% | 1.26 | -10.8% | -1.8 | -0.4 | Holds |
| LLY | 4.8% | -6.7% | 0.68 | -5.8% | -0.9 | -0.2 | Holds |
| GOOG | 13.5% | -9.8% | 1.13 | -9.7% | -0.1 | 0.0 | Holds |
| AVGO | 6.3% | -12.3% | 1.57 | -13.4% | +1.2 | +0.2 | Holds |
| TSLA | 5.1% | -11.4% | 1.60 | -13.7% | +2.3 | +0.4 | Holds |
| MSFT | 14.5% | -7.5% | 1.08 | -9.3% | +1.7 | +0.5 | Holds |
| AMZN | 10.8% | -7.4% | 1.29 | -11.0% | +3.6 | +1.0 | **Hit less** |
| BRK-B | 4.8% | -3.9% | 0.73 | -6.3% | +2.4 | +1.1 | **Hit less** |
| AAPL | 19.3% | -3.8% | 1.09 | -9.3% | +5.5 | +1.6 | **Hit less** |

**Where the explanation does not hold.**
- **NVDA, hit more (8.6 points, 2.2 deviations).** Company news: its earnings report of 2026-11-17. The results beat
  its guidance (revenue $115.8B against $108.0B), but the guided gross margin is lower (71% to 72% against 74.8%), and
  the report names an interim CEO with no reason. The stock fell 14.0% on 2026-11-20; our data gives no cause for that
  day: not explained.
- **AAPL, hit less (5.5 points, 1.6 deviations).** A possible reason, not proven: its annual report of 2026-11-03 raised
  the model value from $112 to $151 a share (revenue +15%), and the price had not risen with it.
- **BRK-B, hit less (2.4 points, 1.1 deviations).** It is the only one of the 10 whose price needs a normal growth (8.6% a
  year; the others need 15% to more than 39%). So it had less to lose when the asked return rose. A reading, not proven.
- **AMZN, hit less (3.6 points, 1.0 deviation).** Not explained: the model gives no value for it (its capex takes almost
  all its operating cash flow), and it had no report in the week.

**What the valuation model adds, and its limit.** The DCF split agrees on the four stocks above (NVDA: a company part of
-15 points of its price; AAPL +16; BRK-B +5). For META, AVGO and LLY it shows a company part of -4 to -8 points, where the
beta test shows none. The two tests are not independent of the price, and the DCF split depends on a sensitivity that
the model gives each stock (9 to 45 for these 10). So the beta test is the main test here, and the DCF split is the second view.
The model uses a premium of 4.41% (an assumption with no confirmed source); the prices imply 1.2% to 1.7%.

**Answer.** The market-wide explanation holds for 6 of the 10 stocks. NVDA was hit more, with company news as the reason
for a part. AAPL, BRK-B and AMZN were hit less; two have a possible reason, and AMZN has none. This is a reading from
fundamentals, not a trade rule.

Next, after the review of the owner: the same test for the S&P 100.
Sources: `analysis/priors/sp100_week_prelim_results_2026-11-21.md` (sections 1 to 8) and the run
`fa dcf attribute --tickers-file rust/mapcheck/universe/sp100.txt --top 10 --from 2026-11-13 --to 2026-11-20 --reality`.
