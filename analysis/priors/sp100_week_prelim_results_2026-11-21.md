# Preliminary results: the week to 2026-11-20 (S&P 100 and S&P 10)

PRELIMINARY. Not reviewed. Not a report. Ticket `T-20261121-sp100-week-attribution`. Code: `claude/premium-series` a993e68
(at the Quality gate) with main b01b66a; data store 0340bc1; curve `--curve par`. Close of 2026-11-13 to close of 2026-11-20.
The split is an accounting split with the model, not a cause. The value is the value at the means, not the expected value.
A market-implied value is a measurement, not an input. Points = percentage points of the list value at 2026-11-13.

## 1. What changed
SPY -8.56% (754.37 to 689.81). S&P 100 (100 companies, value-weighted) -9.22%. S&P 10 -9.96%.

## 2. What contributed (premium view first; the common change is the S&P 100 median, +0.76 points of premium)
| Piece | S&P 100 | S&P 10 |
|---|---|---|
| Facts (new 10-Q or 10-K) | +0.01 | +0.00 |
| Time | +0.09 | +0.08 |
| Rates (10-year zero 5.13% to 4.90%) | +5.08 | +5.81 |
| Implied premium, common part | -12.64 | -15.89 |
| Implied premium, company part | +1.79 | +3.09 |
| Implied growth, common part | -1.22 | -1.01 |
| Implied growth, company part | -0.39 | -0.65 |
| No model value | -1.94 | -1.39 |
| Total | -9.22 | -9.96 |

S&P 100 by sector (points): BusEq -6.09 (51.3% of the value, -11.87%), Money -0.84, Shops -0.49, Hlth -0.48, Manuf -0.41,
Durbl -0.32, Other -0.32, Enrgy -0.14, NoDur -0.09, Chems -0.02, Telcm -0.01, Utils -0.01. The implied premium rose for 68
of the 68 tickers with a premium value at both dates. Seven companies had an earnings 8-K in the window (DIS, HD, INTU, LOW,
NVDA, PANW, WMT): -15.35% as a group, -1.82 points, of which company premium -1.07.

S&P 10 by company (points of the S&P 10): NVDA -3.37 (weight 14.7%, -22.9%; company premium -2.28; earnings 8-K 11-17),
GOOG -1.32, MSFT -1.09, AMZN -0.81 (no model), AVGO -0.78, META -0.77, AAPL -0.74 (common premium -5.89, company +3.08,
rates +2.04), TSLA -0.58 (no model), LLY -0.32, BRK-B -0.18.

## 3. Do the implied pieces make sense
- Implied premium of the S&P 100 list against its own history (412 Fridays from 2019-01-04; not the run of record): median
  3.56%, highest 6.55% (2020-03-20). 2026-11-13: 1.20%, the lowest of all weeks. 2026-11-20: 1.67%, lower than 98% of
  the weeks before it. Yearly mean: 4.5% (2019), 4.4% (2022), 3.1% (2024), 2.8% (2025), 2.4% (2026).
- Implied 10-year revenue growth at the model premium of 4.41%, after the fall, against the base rate (50 companies with
  revenue of $50B or more ten years ago: median 3.7%, 90th percentile 9.9%, maximum 20.9%): TSLA above 39%, NVDA 29.3%,
  AVGO 28.2%, LLY 27.1%, META 22.8% (above every case); GOOG 17.5%, MSFT 16.1%, AAPL 15.0% (above the 90th percentile);
  BRK-B 8.6%.
- Rates on the balance sheet (debt, bonds held, pensions): S&P 10 -0.03 points, S&P 100 -0.09 points; small for the sums,
  above the threshold for 29 leveraged companies (T -1.87 points of its own value).
- NVDA release of 2026-11-17: revenue $115.8B (+103% on the year; guidance $108.0B); next quarter $130.0B; gross margin
  guided to 71% to 72% from 74.8%; the release names an "interim CEO" and gives no reason. Closes: 155.46, 150.96, 145.98,
  125.53.
- The view of the economists: no file yet (design in review).

## 4. Preliminary reading
The fall came from a higher return asked by the market (the implied premium), common to all names, while lower rates
added value. Before the fall the implied premium of the list was the lowest of 8 years; after it, it is still in the
lowest 2%. The prices of 8 of the S&P 10 still need a growth that large companies rarely or never reached. So the
evidence points to a correction of rich prices, not to a buy opportunity for the list. This is a level reading from
fundamentals, not a timing rule and not a backtested trade rule.

## 5. Open
The Quality gate of the code; the premium series run of record; the economics view; the engine build of the base rates
and the reality check (`claude/attribution-reality`); the 2-month S&P 10 run on the new list with BRK-B.
Sources: `analysis/priors/data/week_2026-11-20_main-b01b66a/`, `analysis/priors/sp10_reality_check_evidence_2026-11-21.md`,
`analysis/priors/rates_balance_sheet_measure_2026-11-21.md`, `analysis/priors/data/premium_series_sp100_2019_not-of-record/`.
