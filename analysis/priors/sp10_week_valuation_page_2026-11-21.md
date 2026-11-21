# S&P 10: The Fall by Valuation Block

*Week of 2026-11-13 to 2026-11-20: what changed in fact, and what the market price implies.*

Background: the model gives each stock a value from its reported numbers, the bond yields and a required return.
"Actual" is the change of that value from new facts, time and yields. "Implied" is the remaining change of the price:
the model reads it as a change of the required return, or of the expected growth. This is page 2 of 3.

The 10 largest companies lost 9.96% of their market value. The reported numbers of the companies did not change, and
lower bond yields added 5.8 points. The full fall is in the implied part, and most of it is common to all stocks.

**Table 1: The change of the market value of the S&P 10 in blocks** (points; each column adds to the total).
See: the actual blocks are positive, and the implied blocks make the fall. The implied number of a stock has two readings.

| Block | Read as required return | Read as growth |
|---|---:|---:|
| **Actual** | **+5.89** | **+3.74** |
| New company facts (filings) | 0.00 | 0.00 |
| Time (one week) | +0.08 | +0.14 |
| Lower bond yields (10-year: 5.13% to 4.90%) | +5.81 | +3.60 |
| **Implied from the market** | **-14.46** | **-12.31** |
| Required return, common part | -15.89 | |
| Required return, company part | +3.09 | |
| Expected growth, common part | -1.01 | -7.65 |
| Expected growth, company part | -0.65 | -4.66 |
| **No model value (AMZN, TSLA)** | **-1.39** | **-1.39** |
| **Change of the market value** | **-9.96** | **-9.96** |

**Table 2: Each stock** (points of the S&P 10; the rows add to Table 1, read as required return).
See: the company column shows where a stock differs. NVDA and META lost more than the common part. AAPL, MSFT and
GOOG lost less.

| Stock | Weight | Change | Actual | Implied, common | Implied, company | No model | Points | Required return | Needed growth |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| NVDA | 14.7% | -22.9% | +0.25 | -1.35 | **-2.28** | | -3.37 | -2.20% to 0.07% | 33.5% to 29.3% |
| GOOG | 13.5% | -9.8% | +1.31 | -3.83 | +1.20 | | -1.32 | 1.17% to 1.65% | 19.6% to 17.5% |
| MSFT | 14.5% | -7.5% | +1.22 | -3.62 | +1.31 | | -1.09 | 1.73% to 2.19% | 17.8% to 16.1% |
| AMZN | 10.8% | -7.4% | | | | -0.81 | -0.81 | no value | no value |
| AVGO | 6.3% | -12.3% | +0.23 | -0.55 | -0.45 | | -0.78 | no value | 30.7% to 28.2% |
| META | 6.1% | -12.6% | +0.11 | -0.40 | -0.48 | | -0.77 | -0.04% to 1.03% | 25.3% to 22.8% |
| AAPL | 19.3% | -3.8% | +2.06 | -5.89 | **+3.08** | | -0.74 | 1.11% to 1.45% | 16.4% to 15.0% |
| TSLA | 5.1% | -11.4% | | | | -0.58 | -0.58 | no value | above 39% |
| LLY | 4.8% | -6.7% | +0.34 | -0.46 | -0.19 | | -0.32 | no value | 28.7% to 27.1% |
| BRK-B | 4.8% | -3.9% | +0.37 | -0.80 | +0.25 | | -0.18 | 2.66% to 3.16% | 9.8% to 8.6% |
| **S&P 10** | 100% | **-9.96%** | **+5.89** | **-16.90** | **+2.44** | **-1.39** | **-9.96** | | |

The last two columns: the value of 11-13 to the value of 11-20. Required return: above bonds; AVGO and LLY have no value,
and their implied part is read as growth. Needed growth: the revenue growth of each year for 10 years that the price needs.

**What the implied numbers mean**

- **The common part.** The required return of the median S&P 100 company rose 0.76 points in the week. Each stock gets
  this rise as its common part; the company part is the rest. The required return rose for each stock with a value.
- **NVDA differs.** Its required return rose 2.27 points, three times the common rise. Its needed growth fell 4.2
  points; that of the other stocks fell 1.2 to 2.5 points.
- **AAPL differs.** Its required return rose 0.34 points, less than one half of the common rise. Its value is very
  sensitive to the required return, thus the small difference is a large company part (+3.08).

**Table 3: The implied numbers after the fall against history.**
See: the required return is still low, and 5 of 9 stocks need more growth than a large company ever had.

| Measure | 11-13 | 11-20 | Comparison |
|---|---:|---:|---|
| Required return above bonds, S&P 100 list | 1.20% | 1.67% | 2019 to 2026: median 3.56%; 98% of the weeks are higher |
| The same, from the index dividends (October, November) | 1.37% | 1.64% | Since 1871: 24% of the months are lower |
| Stocks above the best 10-year growth of a large company | 5 of 9 | 5 of 9 | Best case 21.0%, median 3.9% (46 cases) |

Reading from fundamentals: the fall is a correction of prices that were expensive against the last 8 years, and they
are still expensive after it. It is not a buy signal for the group. The reason of the market is on the first page.

Status of the numbers: preliminary (engine code in review; an automatic screen, not a reviewed valuation). An implied
number is a measurement, not an input. Sources: `data/week_2026-11-20_main-b01b66a/`.
