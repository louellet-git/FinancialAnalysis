# Reality check: implied numbers of the week 2026-11-13 to 2026-11-20 (10 largest US companies)

Sources: `sp10.csv` and `sp10.txt` (run folder `week_2026-11-20_main-b01b66a`); the store `/home/user/fa-data-split`
(SEC company facts through `mapcheck check --emit`, prices, 8-K files). Tools: jq and awk only. All growth is nominal
and compound. The implied numbers are measurements. No model input comes from the price.

## A. Implied 10-year growth against the own history

`growth10` = (model revenue of year 10 / base revenue)^(1/10) - 1 (`auto_implied.rs`, line 196). The base is the last
fiscal year (FY). "YTD" = the quarters after the base year against the same quarters one year before (10-Q rows).

| Ticker | Base FY end | Revenue $B | Own 1y | 3y | 5y | 10y | YTD | Implied D1 | Implied D2 | Year-10 revenue at D2 | Implied premium D1 / D2 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| AAPL | 2026-09-26 | 478.8 | 15.1% | 7.7% | 5.5% | 8.3% | - | 16.4% | 15.0% | $1,937B (4.0x) | 1.11% / 1.45% |
| NVDA | 2026-01-25 | 215.9 | 65.5% | 100.0% | 66.9% | 45.7% | +98.7% (9 months) | 33.5% | 29.3% | $2,827B (13.1x) | -2.20% / 0.07% |
| MSFT | 2026-06-30 | 331.8 | 17.8% | 16.1% | 14.6% | 13.8% | +18.8% (1 quarter) | 17.8% | 16.1% | $1,473B (4.4x) | 1.73% / 2.19% |
| GOOG | 2025-12-31 | 402.8 | 15.1% | 12.5% | 17.2% | 18.3% | +23.6% | 19.6% | 17.5% | $2,014B (5.0x) | 1.17% / 1.65% |
| AMZN | 2025-12-31 | 716.9 | 12.4% | 11.7% | 13.2% | 21.0% | +16.0% | none | none | - | none |
| AVGO | 2025-11-02 | 63.9 | 23.9% | 24.4% | 21.7% | 19.1% (9y) | +55.0% | 30.7% | 28.2% | $764B (12.0x) | none |
| META | 2025-12-31 | 201.0 | 22.2% | 19.9% | 18.5% | 27.3% | +28.8% | 25.3% | 22.8% | $1,572B (7.8x) | -0.04% / 1.03% |
| TSLA | 2025-12-31 | 94.8 | -2.9% | 5.2% | 24.6% | 37.1% | +12.5% | >39.0% | >39.0% | >$2,561B (27x) | none |
| LLY | 2025-12-31 | 65.2 | 44.7% | 31.7% | 21.6% | 12.6% | +46.4% | 28.7% | 27.1% | $717B (11.0x) | none |
| BRK-B | 2025-12-31 | 371.4 | 0.0% | 7.1% | 8.6% | 5.8% | +8.6% | 9.8% | 8.6% | $845B (2.3x) | 2.66% / 3.16% |

Notes. The model premium is 4.41%. TSLA: the growth search has no solution (bound above +0.60); 39.04% is the value at
the bound, thus a floor, not a measurement. AMZN: no view ("the value does not rise with the growth"). AVGO: the store
starts at FY2016, thus 9 years. The largest revenue in the list today is AMZN $716.9B and WMT $713.2B.
NVDA: the base is 10 months old. Nine months of FY2027 are $293.7B (release). With the guidance midpoint ($130.0B),
FY2027 is about $423.7B (+96%; a derived number, not a fact). Then the other 9 years need 23.5% a year at D2 (27.9% at D1).

## Base rate (rule set before the calculation)

Rule: companies of `sp500.txt` (list of 2026-11-19, one row for each CIK). End year = last fiscal year in the store
(end on or after 2025-01-01). Base year = the fiscal year 10 years before (9.88 to 10.12 years). Set = base revenue of
$50 billion or more. KR, VLO, JNJ and SYY have no mapped revenue in the base year; their base value comes from the raw
tags `SalesRevenueGoodsNet` and `RefiningAndMarketingRevenue`.

| Set | n | Median | 75th | 90th | Maximum |
|---|---|---|---|---|---|
| Base revenue >= $50B (main set; same 50 rows as "largest 50") | 50 | 3.7% | 7.5% | 9.9% | 20.9% (AMZN) |
| Same, without AAPL MSFT GOOG AMZN BRK-B | 45 | 3.3% | 6.4% | 9.1% | 11.0% (UNH) |
| Base $10B to $50B | 141 | 4.0% | 6.2% | 8.6% | 27.3% (META) |
| Base $1B to $10B | 225 | 8.0% | 11.8% | 16.4% | 45.7% (NVDA) |
| All sizes | 445 | 6.0% | 10.5% | 16.6% | 67.4% (KKR) |
| Pooled: all windows with base years 2007 to 2016, base >= $50B (windows overlap, no patch) | 349 | 2.6% | 6.9% | 9.7% | 23.7% (AMZN 2012-2022) |

More facts. In the 445 last windows, no company with base revenue above $22.8B (CNC) had 22.8% or more. For 27% or
more, the largest base is $17.9B (META). For 29% or more, it is $5.0B (NVDA). In all 3,179 pooled windows, the largest
base with 25% or more is $48.1B (AMZN 2011-2021, 25.6%). A published study (Mauboussin) was not necessary and is not cited.

## C. One line for each company (implied growth at D2)

A growth above every base rate is not impossible. It is only rare: no company of this size did it in the data.

| Ticker | Above own 5-year growth? | Position in the main set (n = 50) |
|---|---|---|
| AAPL | Yes (15.0% against 5.5%) | Above the 90th percentile; 2 of 50 are higher |
| NVDA | No (29.3% against 66.9%; own 10y 45.7%) | Above the maximum (20.9%) and above all pooled windows |
| MSFT | Yes (16.1% against 14.6%) | Above the 90th percentile; 2 of 50 are higher |
| GOOG | Equal (17.5% against 17.2%; own 10y 18.3%) | Above the 90th percentile; 2 of 50 are higher (one is GOOG) |
| AMZN | No implied value | - (own 10y 21.0% is the maximum of the set) |
| AVGO | Yes (28.2% against 21.7%) | Above the maximum and above all pooled windows |
| META | Yes (22.8% against 18.5%; own 10y 27.3%) | Above the maximum; one pooled window is higher (23.7%) |
| TSLA | Yes (floor 39.0% against 24.6%) | Above the maximum; no solution in the range |
| LLY | Yes (27.1% against 21.6%; own 3y 31.7%) | Above the maximum and above all pooled windows |
| BRK-B | Equal (8.57% against 8.63%) | Between the 75th and the 90th percentile; 9 of 50 are higher |

## B. Company news in the window

Only NVDA has an 8-K item 2.02 in the window (all 10 lists checked on 2026-11-20; AAPL 11-02 and BRK-B 11-09 are
before it). The release is in the store: `NVDA/filings/8k/0001045810-26-000088/q3fy27pr.htm` (filed 2026-11-17).

| Item (quarter ended 2026-10-25) | Result | Guidance of 2026-08-26 | Guidance for the next quarter |
|---|---|---|---|
| Revenue | $115.8B; +20% on the quarter, +103% on the year | $108.0B, plus or minus 2% | $130.0B, plus or minus 3%; "includes an additional week" |
| Gross margin (GAAP / non-GAAP) | 74.8% / 74.9% (prior quarter 75.0%) | 74.0%, plus or minus 0.5 points | 71.0% to 72.0% |
| Operating expenses (GAAP / non-GAAP) | $9.2B / $8.8B | $9.2B / $9.0B | $11.0B / $10.7B |
| Data Center revenue | $107.6B; +21% on the quarter, +110% on the year | - | no China Data Center compute revenue assumed |
| GAAP net income; diluted EPS | $65.3B; $2.70 | - | - |

Other facts of the release. Revenue was 7.3% above the guidance midpoint. The guidance midpoint is +12.2% on the
quarter and +90.8% on the year (prior-year quarter $68.1B); it has 14 weeks. The share repurchase authorization
increased by $150 billion in September 2026. The release names Colette Kress "interim CEO" and chief financial officer;
the release of 2026-08-26 named Jensen Huang "founder and CEO". The release gives no reason. NVIDIA announced an
agreement to acquire Hugging Face.
Prices (store, close): NVDA -2.9% on 11-18 (the day after the release) and -14.0% on 11-20. SPY -3.6% and AVGO -12.2%
on 11-20. The store has no file that gives a cause for 11-20. The run shows no new facts for NVDA (`new_facts` false):
the 10-Q of the quarter is not in the store.

## Limits

- Survivorship: the list is the list of today. Companies that left it (acquired, private, failed) are not in the set. Companies with a new SEC number have no base year (DIS, CI, DOW). This moves the base rate up, not down.
- The last window (2015-2025) is one period. The pooled windows overlap; they are not independent.
- Fiscal years differ (base years end from 2015-09 to 2016-09). Revenue is the latest filed value (restated). Acquired revenue is in the growth (CVS, UNH, AVGO). Spin-offs lower it (GE, AIG, HPQ).
- Mapping gap: 51 tickers have no mapped `revenue` in the base year (for example JNJ, KR, VLO, PFE, MRK, KO); only the 4 above $50B are patched in the main set. The PM base value ($73.9B) includes excise taxes; without PM the median is 3.9%.
- The implied growth is the growth of one path at the means, at the model premium 4.41% and the model margin. A higher margin or a lower premium gives a lower implied growth. The run is not final (README of the run folder).
