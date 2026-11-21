# S&P 500 Chip Companies: Fall Against Expectation

*Week of 2026-11-13 to 2026-11-20: did the chip companies fall as much as was expected of them?*

Background: the chip companies made the largest part of the fall of Friday (page 1). 16 companies of the S&P 500 have
the SEC industry code of semiconductors. "Expected" has two measures here: the beta of the stock times the fall of the
market, and the fall of the model value if the stock had the same change as the median S&P 500 company.

No. The chip companies fell much more than expected. They fell 16.4% on average. Their betas give 9.4%, and the
market-wide change of the model value gives 4.4%. Each of the 15 companies fell more than its two expected values.

**Table 1: Each chip company against its expected fall** (percent; the largest difference first).
See: 9 of 14 are more than 1 deviation below the beta line (chance gives about 2), and the fall is larger where the
needed growth is higher.

| Stock | Change | Expected by beta | Difference, in deviations | Expected by value | Needed growth, 11-13 to 11-20 |
|---|---:|---:|---:|---:|---:|
| NVDA | -22.9 | -11.0 | **-2.7** | -4.0 | 33.5% to 29.3% |
| FSLR (solar) | -19.7 | -6.5 | **-1.8** | -3.6 | 19.7% to 16.4% |
| MU | -23.6 | -10.6 | **-1.5** | -3.5 | 29.8% to 24.9% |
| MPWR | -17.9 | -10.4 | **-1.5** | -3.5 | 41.8% to 38.3% |
| AMAT | -18.3 | -9.9 | **-1.4** | -4.7 | 23.8% to 20.5% |
| MRVL | -22.0 | -11.2 | **-1.3** | -3.7 | 40.3% to 36.1% |
| AMD | -20.5 | -10.6 | **-1.3** | no value | above 40% |
| ADI | -13.1 | -8.0 | **-1.3** | -4.6 | 22.1% to 19.8% |
| TXN | -13.1 | -7.1 | **-1.3** | -3.6 | 26.6% to 24.2% |
| ON | -14.4 | -9.3 | -0.8 | -4.5 | 12.6% to 9.8% |
| NXPI | -11.3 | -8.2 | -0.6 | -5.3 | 13.6% to 11.6% |
| INTC | -14.0 | -9.3 | -0.5 | no value | no value |
| AVGO | -12.3 | -10.3 | -0.4 | -4.4 | 30.7% to 28.2% |
| MCHP | -10.1 | -9.6 | -0.1 | -5.9 | 13.9% to 11.9% |
| Q (new company) | -12.6 | no beta | | -5.3 | 9.1% to 6.9% |
| **Mean of the group** | **-16.4** | **-9.4** | | **-4.4** | |

Expected by beta: beta times the equal-weight S&P 500 (-6.56%); the difference is in weekly deviations (104 weeks).
Expected by value: the lower yields, and the cut of expected growth of the median S&P 500 company, in the model of the stock.

**What to see**

- **The group as a whole.** The 485 other companies of the S&P 500 fell 6.3% on average. The chips fell 16.4%, which
  is 7 points more than their betas give. Weighted by market value, the group fell 19.5%.
- **The size goes with the needed growth.** Correlation -0.63 (13 companies). The 6 companies that needed a growth of
  24% or more each year fell 18.6% on average; the 7 others fell 14.2%.
- **In valuation terms.** The price change of the median chip company is a cut of expected growth 2.4 times that of
  the median S&P 500 company (4.8 points against 2.0 points).
- **The exceptions.** AVGO and TXN need a high growth but fell 12% to 13%. FSLR makes solar panels, not chips.
- **The same fall outside the code.** Chip equipment and AI hardware fell as much: LRCX -18.9%, KLAC -16.7%,
  DELL -19.3%, SMCI -19.4%, ANET -15.0%. This is consistent with the reason on page 1 (the AI build-out).

Status of the numbers: preliminary; not audited. SWKS has no close of 11-20 in our data and is left out. Sources:
`data/week_2026-11-20_chips/`.
