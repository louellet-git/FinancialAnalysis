# The S&P 10 in the week to 2026-11-20: does the explanation hold for each stock?

PRELIMINARY, for the review of the owner. Close of 2026-11-13 to close of 2026-11-20. Of record: the prices and the SEC
facts (data store 0340bc1). NOT of record: the betas and the DCF numbers (the code is at the Quality gate; branch
`claude/attribution-reality`) and the usual weekly deviation (a shell calculation, not yet in the engine).

**Answer.** The market-wide explanation holds for most of the 10, and it does not hold for NVDA. NVDA was hit more
(2.2 to 3.0 deviations), with company news as the reason for a part. AAPL was hit less (1.0 to 1.6 deviations). META (hit
more), BRK-B and AMZN (hit less) are at the level of chance. One more fact: the 10 as a group fell more (-9.96%) than the
average S&P 500 stock (-6.56%, equal weight), so the fall was heavier in the largest names.

**The explanation.** The fall was market-wide: the return that the price asks (the implied premium) rose for all 68 S&P
100 companies with a value. The yields rose the week before (real 10-year +28 bp), and the stocks followed in this week
(Head of Economics: one fair reading, not proven). No company of the 10 filed new accounts in the week.

**The test.** A stock falls by its beta times the market; "hit more" or "hit less" is the rest. Rule, set before the
table: the explanation holds when the rest is smaller than 1 usual weekly deviation OF THE REST (104 weeks before).
Two markets: SPY (-8.56%), which contains the 10 (NVDA alone moved it), and the equal weight of the S&P 500 (-6.56%), which
the 10 do not move. By chance alone, about 3 of 10 stocks are outside 1 deviation.

| Stock | Weight | Change | Beta | Rest against SPY (points; deviations) | Rest against equal weight (points; deviations) | Result |
|---|---:|---:|---:|---:|---:|---|
| NVDA | 14.7% | -22.9% | 1.67 | -8.6; -2.2 | -11.9; -3.0 | **Hit more** (both) |
| META | 6.1% | -12.6% | 1.26 | -1.8; -0.4 | -4.3; -1.0 | Chance level |
| LLY | 4.8% | -6.7% | 0.68 | -0.9; -0.2 | -2.2; -0.4 | Holds |
| GOOG | 13.5% | -9.8% | 1.13 | -0.1; 0.0 | -2.3; -0.6 | Holds |
| AVGO | 6.3% | -12.3% | 1.57 | +1.2; +0.2 | -2.0; -0.4 | Holds |
| TSLA | 5.1% | -11.4% | 1.60 | +2.3; +0.4 | -0.9; -0.2 | Holds |
| MSFT | 14.5% | -7.5% | 1.08 | +1.7; +0.5 | -0.4; -0.1 | Holds |
| AMZN | 10.8% | -7.4% | 1.29 | +3.6; +1.0 | +1.0; +0.3 | Chance level |
| BRK-B | 4.8% | -3.9% | 0.73 | +2.4; +1.1 | +0.9; +0.4 | Chance level |
| AAPL | 19.3% | -3.8% | 1.09 | +5.5; +1.6 | +3.3; +1.0 | **Hit less** (both, weak) |

**Where the explanation does not hold.**
- **NVDA, hit more.** Company news: its earnings report of 2026-11-17. The results beat its guidance (revenue $115.8B
  against $108.0B), but the guided gross margin is lower (71% to 72% against 74.8%), and the report names an interim CEO
  with no reason. The stock fell 14.0% on 2026-11-20; our data gives no cause for that day: not explained.
- **AAPL, hit less.** A possible reason, not proven: its annual report of 2026-11-03 raised the model value from $112 to
  $151 a share (revenue +15%), and the price had not risen with it.
- **The 10 as a group.** They fell 3.4 points more than the average stock. The prices of 8 of the 10 need a growth that
  large companies rarely reached (15% to more than 39% a year), so they lose more when the asked return rises. BRK-B needs
  8.6% and fell least. A reading, not proven.

**The second view (the DCF split) and its limit.** It agrees on NVDA (a company part of -15 points of its price) and AAPL
(+16). It depends on a sensitivity that the model gives each stock (9 to 45 for these 10), and it comes from the same
prices: it is not a second proof. The model uses a premium of 4.41% (an assumption with no confirmed source); the prices
imply 1.2% to 1.7%. This page is a reading from fundamentals, not a trade rule.

Next, after the review of the owner: the same test for the S&P 100, with the count against chance (about 32 of 100).
Sources: `analysis/priors/sp100_week_prelim_results_2026-11-21.md`; `fa dcf attribute --tickers-file
rust/mapcheck/universe/sp100.txt --top 10 --from 2026-11-13 --to 2026-11-20 --reality`; the same command on `sp500.txt`
for the equal weight. The deviations against the equal weight use the beta and the deviation measured against SPY.
