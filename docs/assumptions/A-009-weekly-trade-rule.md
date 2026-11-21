# A-009: rule of the weekly fundamental trades (side, choice, size, exit)
> **INVALID (2026-11-21, CRIT F1 of the strategy gate): split look-ahead in the backtest prices.** Its test results
> (R1, R2) are not valid until `T-20261121-split-basis-fix` and the reruns. See the findings file.
Updated: 2026-11-21. Written before any test of this exact rule. Do not tune it on a later result.

## Choice
Each Friday, with the last close before that Friday (Thursday; Wednesday when Thursday is a holiday, for
example 2026-11-26) and the SEC facts filed by that day. Never a later close:
1. **List.** The tickers of `rust/mapcheck/universe/sp500.txt` with the standard template and a model value.
2. **Signal.** The P50 / price of the automatic model, point in time (`fa portfolio backtest --strategy
   value-rank`: the vintage of the facts filed before the date, the curve and the price of the day before).
3. **Rank.** The rank of the signal inside its Fama-French 12 sector (today's SIC code). A sector with fewer
   than 5 scored tickers is ranked with "Other".
4. **Choice.** The 3 tickers with the highest rank inside their sector that are not held now, at most one for
   each sector in a week, and each in the top fifth of its sector. Fewer than 3 qualify: fewer trades.
5. **Data checks (exclude the ticker).** A mapping status FAIL; a last fiscal year older than 550 days; a flag
   of the share-count guard (0.01x to 100x); P50 at or below 0. These are data faults, not views.
6. **Side.** Buy. A short is not used before its test passes (see "When to examine it again").
7. **Size.** Each trade is 1/39 of the value of the sleeve at entry (2.56%): 3 a week for 13 weeks gives at most
   39 open positions. A lower limit of the Head of Quality and Risk wins.
8. **Exit.** At the Friday 13 weeks after the entry (the last close before that Friday). Earlier only for a data fault found
   later or a sleeve limit. No price stop and no target (the backtest had none).
9. **Hedge.** None. A short of SPY by the beta of the open positions is the first hedge, if a sleeve limit on
   market exposure needs it.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Weekly proposals | `PLANNING/board/T-20261121-weekly-trade-proposals.md` | this record |
| Strategy | `rust/src/portfolio/strategies.rs` | `ValueRank`, `top_quintile_by_sector` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Top fifth of the whole list | The gain is the choice inside a sector (run D against run B of the findings) |
| Weights by the size of the signal | No test supports it; the backtest is equal weight |
| A price stop | The backtest had no stop; a stop is a new, untested rule |
| Long and short now | The short side has no test yet |

## Why this choice
`analysis/priors/dcf_engine_backtest_2026-11-21.md`: the sector-neutral top fifth gave +3.45 points a year over
the equal weight of the same list, 2018-2026, with a parameter table cut at 2016 (t 2.06, 6 of 9 years). A
staggered entry of 3 names a week with a 13-week hold keeps about the breadth of the quarterly top fifth and
the same holding time. None of the reasons uses the price of a chosen name.

## Effect on the result
Measured once, as pre-registered (`analysis/priors/dcf_engine_backtest_2026-11-21.md`, run R1, store `1804f48`):
+21.37% a year against +14.80% for the weekly equal weight of the same list, 2018-01 to 2026-11; +6.56 points a
year, t 3.44, 8 of 9 years above. The order of items 4 and 8 in the test: the exit first, then the choice, so a
name can be chosen again on its exit day (a re-entry; in live use it is a hold, not a new trade).
The short-side test (R2, the bottom fifth inside each sector): -3.24 points a year, t -2.71.

## Who decided and when
The Head of Fundamental Research, 2026-11-21, on the word of the owner ("size of trades should be subject to the
trade and that responsibility is of the head"; sides: "Anything"; `chiefs/louellet-git/fundamental/RECORD.md`).
Review: not done (the gate of the Head of Quality and Risk before 2026-11-27).

## When to examine it again
After the pre-registered test; after 13 weeks of live record; the short side when a test of the bottom fifth
inside each sector (2018-2026, table cut at 2016) gives a return below the equal weight with t at or below -2.
