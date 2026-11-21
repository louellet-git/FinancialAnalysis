# Long-history test of the market-level DCF: design

Ticket: T-20261121-economics-long-history. Date: 2026-11-21. Head: Economics.
For the test of T-20261121-dcf-engine-backtest.

## 1. Purpose

The Head of Fundamental Research tests a market-level DCF on the longest history. The owner asked for
data "as long as you have", also from times before or outside the US stock market. This file gives
the data list, the periods, the later returns and a simple baseline premium. It is context. It is not
a model input and not a trade signal. A head that wants a macro input writes an assumption record.

## 2. Data list

Status: all rows are requested from the Head of Technology (ticket T-20261121-economics-data). Shiller
first. Nothing is in `data/raw` yet. Public-copy terms: "to check" for each row.

| Series | Source and address | Freq. | Start | What it gives | Traps | Terms |
|---|---|---|---|---|---|---|
| Shiller `ie_data.xls` | Yale, Robert Shiller online data (shillerdata.com) | Month | 1871 | S&P composite price, dividends, earnings, CPI, GS10 | Cowles reconstruction before 1926. Annual dividends and earnings are interpolated to months. Earnings come about 3 months late. CPI is monthly from 1913 and annual before. | to check |
| BAA and AAA yields | FRED (series BAA, AAA), Moody's | Month | 1919 | Corporate credit spread | Moody's is the owner of the data. Rating rules change over time. | to check |
| TB3MS | FRED | Month | 1934 | 3-month bill rate (cash return) | Bill rates are near zero in some years. | to check |
| GS10 | FRED | Month | 1953 | 10-year Treasury yield | Shiller GS10 covers the earlier years. | to check |
| CPIAUCNS | FRED | Month | 1913 | CPI, not seasonally adjusted (deflator) | Release lag of about 1 month. Values are revised little. | to check |
| Short rates before 1934 | NBER Macrohistory database | Month | about 1857 | Call money and commercial paper rates | Rates are not the same kind as bills. Check each series. | to check |
| GSW curve | Store copy, `fa rates` | Day | 1961 | Fitted zero yields | Lag of about 1 week. Values are fitted, not traded. | in the store |
| Jorda-Schularick-Taylor database | macrohistory.net | Year | 1870 | 17 countries to 2020: equity, bill, bond returns, CPI, dividends | Annual only. Revisions between releases. | to check |
| Bank of England "A millennium of macroeconomic data" | Bank of England website | Year | about 1700 | UK consol yield, CPI | Early values are estimates. Use for the rate side only. | to check |
| Damodaran implied ERP | Damodaran online data | Year | 1960 | Implied equity risk premium | It is the output of a model. Use as a cross-check only. | to check |

## 3. Periods

The periods are fixed before any run. We do not move them after a result.

| Period | Dates | Reason | What it tests |
|---|---|---|---|
| 1 | 1871-1913 | Gold standard, low inflation | The rule with stable money. The index is a reconstruction. |
| 2 | 1914-1928 | War and the 1920s | A war shock to inflation and rates |
| 3 | 1929-1949 | Depression, war, rate controls (the Fed held yields 1942-1951) | A rate that is not a market price |
| 4 | 1950-1965 | Low inflation | A calm base case |
| 5 | 1966-1982 | High inflation | Real rates and real cash flow when prices rise |
| 6 | 1982-2007 | Disinflation | Falling rates and rising prices |
| 7 | 2008-2021 | Rates near zero, QE | A discount rate near the floor |
| 8 | 2022-now | Inflation back | A new rise of rates. The later returns are not yet known. |

Outside the US stock market:

- **JST countries.** Same method on other markets. It is an out-of-sample test with less survivorship
  bias. Each country has its own wars, defaults and closures.
- **UK consols.** They give a discount rate before any stock index. Use them for the rate side only.

## 4. Measures

(a) **Later real excess return** at month t, for horizon H of 60 or 120 months:

- TR is the monthly total return: (price_s + dividend_s) / price_(s-1) - 1.
  The Shiller dividend column D is an annual rate: use dividend_s = D_s / 12.
- B is the monthly bill return.
- G_TR = product of (1 + TR_s) and G_B = product of (1 + B_s), for s = t+1 to t+H.
- Let c = CPI_t / CPI_(t+H). Real annual total return: `RT = (G_TR * c) ^ (12/H) - 1`.
  Real annual bill return: `RB = (G_B * c) ^ (12/H) - 1`.
- Real excess return: `X(t,H) = RT - RB`. Report RT also.
- Before 1934, use the NBER short rate as the bill return. State this in the result.

(b) **Baseline premium.** The rules are fixed before the run:

1. P1 = dividend yield + long-run real dividend growth - real 10-year yield.
2. P2 = (1 / CAPE10) - real 10-year yield.

Real 10-year yield = GS10 less the trailing 10-year CPI change (confirmed by the Head of Fundamental Research, 2026-11-21). The long-run real
dividend growth is a fixed number from data before t only.

(c) **Engine premium.** The implied premium of the engine (Fundamental Research).

## 5. Test

1. Regress X(t,H) on the premium, in the full sample and in each period.
2. Overlapping returns. Use Newey-West standard errors with lag = H in months. Add a check on
   non-overlapping windows.
3. Independent points are few: about 15 ten-year windows in 150 years, and fewer in each period.
   A result for one period is weak evidence. Do not rank the periods by their t-statistics.

Known traps:

- **Look-ahead.** Lag earnings by 3 months and CPI by 1 month. Use the data as of each date.
  Shiller D and E are annual values interpolated to months. Thus a value of D or E at month t
  uses data of the full year. Rule: a test value at month t uses only the annual D and E of the
  years that ended before t (plus the 3-month lag for E). If a run does not follow this rule,
  it states the look-ahead that remains. (Agreed with the Head of Fundamental Research, 2026-11-21.)
  Shiller's file is a later revision. This is a limit.
- **Survivorship.** The US is a winner market. This is why the JST test is in the plan.
- **Reconstruction.** The index before 1926 is a reconstruction, not a traded price.
- **Regime chosen after the result.** The periods in section 3 are fixed. A new split needs a new record.
- **Overlap of premium and return.** A slow premium and a long horizon give a high but false fit.

## 6. Split of the work

| Who | Gives |
|---|---|
| Economics | The data request, the periods, the later returns, the baseline premium |
| Fundamental Research | The engine and its implied premium |
| Quality and Risk | The review. The model-validator checks the test. |

## 7. Open points for the person

None now. If a source has terms that stop a public copy, the data stays out of the store. The test
uses a local copy only if the person agrees.

## What we did not do

We did not run `fa`, download data, or test any number. No value in this file is a measured result.
