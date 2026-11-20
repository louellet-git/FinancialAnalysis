# Study of two conventions of the automatic DCF (2026-11-20)

Ticket `T-20261120-dcf-conventions-study`. This file continues `analysis/priors/dcf_investigation_2026-11-20.md`
(sections 4.6 and 4.7). The automatic model (`fa dcf auto`, `fa dcf screen`) is a screen, not a reviewed valuation.
No default of the model changes in this ticket. The owner decides.

**Rule of the ticket.** A convention is not chosen because it brings the value near the market price. The evidence is:
(a) the realized record of the company panel, (b) named published sources, (c) logic and consistency inside the model,
(d) the point-in-time test of revenue and free cash flow (`fa dcf pit`). The return test (`fa dcf returns`) is secondary.
**Warning:** a choice made on the return test is a fit to past returns. We show it, but we do not decide with it.

**Words.** "Ratio" is P50 / price (screen of 2026-11-19, 430 tickers of `sp500.txt` with a price, 4,000 paths, seed 7).
"Stable set" is the 55 names of the investigation. "Base" is the model of today, with no `--rules`. "CAGR10" is the
compound annual growth of revenue over 10 fiscal years. "Spend" is `acquisitions_net_of_cash` divided by revenue (a
missing value counts as 0, as the model does).

**Data.** Cache `/home/user/FinancialAnalysis/data/raw` (read only; company facts of about 1,250 tickers, GSW curve file).
Panel: `build_panel` of `panel.rs`, template `standard` (849 companies), fiscal years from 2006 to 2025. A scratch program
(`conv_study`, not in the repository; it calls `build_panel`, `load_company_years`, `model_at`, `build`) made the panel
numbers. Its output is `analysis/priors/dcf_conventions_study_2026-11-20_panel.txt`. Each model number comes from the
command that the table names.

## 0. Summary

| Question | Finding | Recommendation |
|---|---|---|
| Q1: one long-run growth | Both numbers fail the evidence. 5.13% is above the growth of large mature companies (median CAGR10 2.7%) and above the growth of the economy that the sources give (3.8% to 4.8%). 2.25% is below the growth of the economy and below the growth that the panel shows in years 8 to 12 (median 4.8%). | One number `L = lower of the 10-year zero yield and the long-run nominal growth of the economy`, for the fade level and for the mean terminal growth. Now `L = 3.8%` (option `long_run=0.038`). Ratio +0.057. |
| Q2: acquisition charge | The model gives free cash flow before acquisitions. The real cash flow after acquisitions is 1.7% of revenue lower at T+1 (3.3% for companies that spend more than 5%). One dollar of spend buys 0.42 dollar of revenue (panel). | The consistent pair: charge `w` times the mean spend, with `w` the weight of the history in the growth mean (the shrink weight), history unchanged (option `acq_w=1`). Ratio -0.107. `w` is 0.55 for IBM. IBM `f = 0.5` is near this. |

## 1. Q1: one long-run growth number

### 1.1 Evidence from the panel (command: `conv_study q1`)

The panel has 4,016 windows of 10 fiscal years (568 companies; start years 2006 to 2016). The windows overlap. Only two
decades do not overlap (start 2006 or 2007, and start 2016). So the panel has about **1.5 independent decades**, and the
second one has 35 windows. The result describes one decade (2007 to 2025) with a financial crisis, a low-rate period and an
inflation shock. **Survivorship:** the cache holds companies that exist now. A company that failed, shrank or left is
missing. This pushes all growth numbers up. We do not know by how much.

CAGR10 of revenue, by revenue at the start (all windows; share = share of windows):

| Start revenue | Windows (companies) | CAGR10 P25 / median / mean / P75 | Above the 10-year zero yield of the start year | Above 2.25% | Above 3.8% | Years 8 to 12, median (n) |
|---|---|---|---|---|---|---|
| below 1B | 773 (175) | 10.2 / 16.4 / 20.5 / 25.9% | 95% | 96% | 94% | 12.0% (555) |
| 1B to 10B | 2,010 (340) | 3.2 / 6.4 / 7.3 / 10.4% | 78% | 81% | 71% | 7.2% (1,465) |
| 10B to 30B | 765 (148) | -0.0 / 2.5 / 3.0 / 5.8% | 47% | 52% | 39% | 3.8% (549) |
| 30B to 100B | 338 (66) | 0.4 / 3.6 / 3.7 / 6.8% | 57% | 59% | 47% | 7.1% (241) |
| above 100B | 130 (22) | -1.6 / 2.2 / 1.8 / 7.4% | 44% | 50% | 38% | 4.4% (96) |
| **10B and above** | **1,233 (194)** | **-0.1 / 2.7 / 3.1 / 6.2%** | **50%** | **54%** | **41%** | **4.8% (886)** |
| stable set (55 names) | 426 (49) | 0.8 / 3.3 / 3.6 / 6.4% | 53% | 61% | 44% | 4.4% (328) |

Years 8 to 12 is the 4-year CAGR from year 8 to year 12 after the start. It exists for start years up to 2013.

Large companies against the economy. Median CAGR10 of the companies with 10B or more at the start, by start year, against
the growth of US nominal GDP over the same 10 years (BEA series, see 1.2):

| Start year | 2007 | 2008 | 2009 | 2010 | 2011 | 2012 | 2013 | 2014 | 2015 |
|---|---|---|---|---|---|---|---|---|---|
| Companies, median CAGR10 (%) | 1.5 | 1.6 | 3.1 | 0.9 | 1.6 | 3.1 | 3.5 | 3.0 | 4.0 |
| Nominal GDP CAGR, same 10 years (%) | 3.1 | 3.4 | 4.1 | 3.6 | 4.3 | 4.8 | 5.1 | 5.3 | 5.4 |
| 10-year zero yield at the start (mean, %) | 4.3 | 3.2 | 4.1 | 3.4 | 2.0 | 1.9 | 3.1 | 2.3 | 2.3 |

In each of the 9 decades the large companies grew 1.0 to 2.7 points a year slower than nominal GDP. The yield did not
follow the growth: it was 2.0% in 2011 and the growth of the economy in the next 10 years was 4.3%.

Companies that start fast keep their growth for long (CAGR10 median; no jump in the window; revenue 1B or more at the
start): first 3 years above 12% a year: 10.6% (447 windows), and 7.5% in years 8 to 12; first 3 years at or below the yield of the start year: 1.6% (1,044 windows). So a fade of the
company mean to a low level inside 10 years is not what the record shows for fast starters. A part of this is survivorship.

### 1.2 Evidence from sources

| Source | What it says | Use |
|---|---|---|
| A. Damodaran, "To infinity and beyond: the terminal value implodes (Myth 5.2)", slides, pages.stern.nyu.edu/~adamodar/pdfiles/blog/TermValueMyth2.pdf (read as text) | "the growth rate that you can use in it is constrained to be less than equal to the growth rate of the economy in which you operate." "I use a simpler and more easily observable number as a cap on stable growth: the risk free rate". The risk-free rate is "a good proxy for the nominal growth rate in the economy". His table: 1954 to 2015, 10-year T-bond 5.93%, nominal GDP 6.67%; 1981 to 2008, 6.88% against 6.30%. | The risk-free rate is a **cap** and a proxy. The limit is the growth of the economy. In 1981 to 2008 the yield was above the growth of the economy. |
| B. Koller, Goedhart, Wessels, *Valuation*, 5th ed. (Wiley, 2010), chapter 9 (forecast period) and chapter 10, p. 227 (read in a PDF of the book) | "The explicit forecast period should be long enough that the company's growth rate is less than or equal to that of the economy. Higher growth rates would eventually make companies unrealistically large relative to the aggregate economy." The formula with `g` and no extra investment "can substantially overstate continuing value because it assumes that NOPLAT can grow without any incremental capital investment." | Growth at most the growth of the economy. A higher `g` needs more reinvestment. |
| C. BEA, nominal GDP, annual, series GDPA of FRED (downloaded 2026-11-20; `curl` of fred.stlouisfed.org/graph/fredgraph.csv?id=GDPA); CAGR computed by us | 1960 to 2025: 6.41%. 1990 to 2025: 4.81%. 2000 to 2025: 4.51%. 2007 to 2025: 4.30%. 2015 to 2025: 5.37%. | Long-run nominal growth since 1990: 4.3% to 4.8%. |
| D. CBO, *The Budget and Economic Outlook: 2026 to 2036* (February 2026), as summarized by CRFB and others in web search results. **We could not open cbo.gov (HTTP 403) and did not read the table.** | Real GDP growth 1.8% a year after 2026; PCE inflation 2.0% from 2030. | A forward nominal growth of about 3.8% (1.8 + 2.0). **Derived by us, not verified.** The owner must check it in the CBO data file. |
| E. A. Damodaran, "The one number that tells the whole story: equity risk premiums", slides (pages.stern.nyu.edu/~adamodar/pdfiles/country/ERPforEvercore2018.pdf), and the equation of 1 January 2018 | "Earnings and Cash flows grow @2.41% (set equal to risk free rate) a year forever" after year 5. The equation ends with `(k - 0.0241)`. The premium of 4.23% on 1 January 2026 is in his post "Data Update 2 for 2026: Equities get tested, and pass again!" (23 January 2026). | The implied premium assumes a growth after year 5 **equal to the risk-free rate**. |

### 1.3 Consistency inside the model

1. **The premium.** The 4.41% of the model is an implied premium of Damodaran (source E). (We could not match the value 4.41% to a publication; the post of January 2026 in source E gives 4.23%.) It is the discount rate that makes the
   index price equal to the cash flows with the growth after year 5 equal to the risk-free rate. If the model discounts a
   company at `rf + beta x 4.41%` and grows it forever below `rf`, the model values a company of average risk below the
   index price by construction. Size, with `k = 5.13% + 4.41% = 9.54%`: the Gordon multiple `(1 + g) / (k - g)` is 23.8 for
   `g = 5.13%`, 18.1 for `g = 3.8%` and 14.0 for `g = 2.25%`. So `g = 2.25%` gives 59% of the terminal multiple that the
   premium assumes. This is conservative. It is not a measured error.
2. **Two numbers.** The fade goes to the 10-year zero yield (cap, 5.13%) in year 10, then the growth steps to 2.25%. The step
   is 2.9 points. The share of companies with a long-run growth at the cap is 239 of 431 (screen column `growth_long_run`).
   For the stable set the median faded growth is 3.58%.
3. **Terminal reinvestment (`auto.rs`, `[reinvestment] mode = "margin"`).** The terminal cash flow is `FCFF(10) x (1 + g)`.
   There is no reinvestment term. The margin of year 10 is the shrunk mean of `(OCF - capex) / revenue` of the history. So it
   already pays the capex for the growth that the history had (stable set: median 3.6% growth). We checked the effect with a
   hand calculation (not run in the engine). Assume `ROIC` 10% to 15% and the stable-set margin 14.3%:
   - Terminal growth 2.25%: the margin pays for 1.35 points of growth that the terminal value does not use. The terminal cash
     flow is about **12% (ROIC 15%) to 21% (ROIC 10%) too low**. This is conservative.
   - Terminal growth 5.13% (`terminal_rf=1`): the terminal value uses 1.5 points of growth that the margin did not pay for. The
     terminal cash flow is about **13% to 24% too high**. This is the "aggressive-growth formula" that source B warns about.
   - Terminal growth 3.8%: the gap is about zero for the stable set (the faded growth of year 10 is near 3.6%).
   So the margin mode is consistent with a terminal growth near the growth of year 10, and `terminal_rf=1` is not.

### 1.4 Alternatives and their measured effects

Commands: `fa dcf screen --tickers-file rust/mapcheck/universe/sp500.txt --cache data/raw --as-of 2026-11-19 --rules R`;
`fa dcf pit --cache data/raw --rules R`; `fa dcf returns --cache data/raw --rules R`. "TV share" is the median of the share of
the terminal value in the enterprise value (`PathResult::tv_share`, new column `tv_share` of the screen CSV).

| Alternative | `--rules` R | Ratio, all [stable] | Change (all) | TV share | PIT revenue T+3 err | PIT free cash flow T+3 err | 3-year rank corr (t) |
|---|---|---|---|---|---|---|---|
| Base | none | 0.489 [0.523] | | 0.610 | +1.6% | +1.5% | +0.123 (4.5) |
| Fade to the terminal growth | `long_cap=0.0225` | 0.459 [0.498] | -0.030 | 0.600 | +2.1% | +1.5% | +0.122 (4.4) |
| Terminal growth = yield | `terminal_rf=1` | 0.603 [0.641] | +0.114 | 0.709 | +1.6% | +1.5% | +0.125 (4.5) |
| **Middle: lower of yield and 3.8%** | `long_run=0.038` (same as `terminal_growth=0.038,long_cap=0.038` while the yield is above 3.8%) | **0.546 [0.593]** | **+0.057** | 0.668 | +1.7% | +1.5% | +0.127 (4.5) |
| Two-stage to 2.25% (fade ends in year 10) | `long_cap=0.0225,fade_years=10` | 0.446 [0.485] | -0.043 | 0.594 | +2.3% | +1.6% | +0.118 (4.2) |
| Two-stage to 3.8% | `terminal_growth=0.038,long_cap=0.038,fade_years=10` | 0.532 [0.593] | +0.043 | 0.664 | +1.9% | +1.5% | +0.124 (4.3) |
| Fade ends in year 10 to the yield | `fade_years=10` | 0.477 [0.523] | -0.012 | 0.609 | +1.8% | +1.5% | +0.119 (4.3) |

**The point-in-time test cannot tell the alternatives apart.** Its horizon is 3 years, and the fade ramp starts at zero in year
1. The coverage of P10 to P90 at T+3 is 86.9% (revenue) and 85.5% (free cash flow) for all of them, within 0.2 point. The return
test also cannot tell them apart (3-year rank correlation 0.118 to 0.127; the standard error is 0.027 to 0.029).

**A 10-year point-in-time test of the revenue (`conv_study long`).** It tests the level of the fade directly. Base years 2010
to 2016 (5 years up to T and 10 after; 1,625 company-years with revenue 1B or more; 672 with 10B or more). The curve is the
GSW curve of the base date. Nearly one independent decade. Survivorship applies. The 2021 to 2023 inflation is in these years.

| Level of the fade | Real below P10 / above P90 | Real below P50 | Median (real / P50 - 1), 1B and above | Same, 10B and above |
|---|---|---|---|---|
| 10-year zero yield of the date (the rule; 1.7% to 3.4% in these years) | 1.9% / 8.4% | 29% | +24.3% | +16.4% |
| 2.25% | 1.8% / 8.6% | 29% | +24.6% | +16.4% |
| 3.8% | 2.2% / 7.6% | 35% | +18.5% | +12.2% |
| 5.13% (fixed) | 2.6% / 6.7% | 38% | +15.3% | +9.7% |
| No cap (company mean stays) | 3.8% / 5.3% | 44% | +7.2% | +8.0% |

The model P50 of year 10 is below the real revenue in every case. A lower level of the fade gives a larger error. The cap
is not the main cause: for the 196 company-years where the company mean is below the cap the error is +40% for all levels (the
shrink to the cell mean). At year 5 the errors are -2% to -3% for the 1B and above group. This test supports a level above
2.25% for the fade, and it shows that the yield of the date was a poor proxy for the growth (see 1.1). It does not test the
terminal growth after year 10.

### 1.5 Recommendation for Q1

1. **Use one number `L` for the fade level and for the mean terminal growth: `L = min(10-year zero yield, long-run nominal
   growth of the economy)`.** Today `L = min(5.13%, 3.8%) = 3.8%`. Option: `--rules long_run=0.038` (new; off by default).
2. **Reasons.** (a) Both sources (A, B) cap the growth at the growth of the economy. (b) Source A uses the risk-free rate only as
   a proxy for that growth; in 2026 the proxy (5.13%) is above every measure of the growth of the economy (3.8% forward, 4.3% to
   4.8% in the past). (c) The panel does not support 5.13% for large companies (median CAGR10 2.7%, 41% of windows above 3.8%).
   (d) The 10-year test rejects 2.25% against 3.8%. (e) With 3.8% the margin of year 10 pays for about the growth that the
   terminal value uses (section 1.3); with 5.13% it does not.
3. **What stays inconsistent.** The premium of 4.41% assumes `g = rf` after year 5 (source E). With `L = 3.8%` the model is still
   conservative by 1.3 points against that assumption. We accept this: `rf` is a cap, not a forecast (source A).
4. **What a person must decide.**
   - Is the ceiling the risk-free rate (source A, `terminal_rf=1`, ratio 0.603) or the growth of the economy (source B, `long_run`)? The
     two give a ratio difference of 0.06.
   - The value of the long-run nominal growth. We used 3.8% from source D, which we could not read. Check the CBO table and then
     update this number each year. Do not keep it as a constant in the code.
   - Whether to keep 2.25% as a conservative convention. It is below every measure of the growth of the economy that we found.
5. **What stays uncertain.** One independent decade of data; survivorship; the 10-year test mixes the level of the fade with the
   shrink of the company mean; the reinvestment estimates are a hand calculation with assumed `ROIC`; source D is a
   secondary report.

## 2. Q2: the acquisition charge

### 2.1 Spend in the panel (command: `conv_study q2`)

Spend = `acquisitions_net_of_cash` / revenue. "Regular" = mean spend 2% or more and spend above 0.5% in at least 70% of the years.
Of the company-years, 52% to 67% have a reported value (a missing value counts as 0, so the numbers are low).

| Size of the year | Company-years | Median | Mean | P75 | P90 | Share of years above 2% |
|---|---|---|---|---|---|---|
| below 1B | 1,900 | 0.00% | 20.2% (outliers) | 1.5% | 18.3% | 23.5% |
| 1B to 10B | 6,046 | 0.04% | 5.3% | 2.9% | 13.6% | 28.3% |
| 10B to 30B | 2,599 | 0.00% | 2.9% | 1.4% | 6.4% | 21.4% |
| 30B to 100B | 1,055 | 0.02% | 2.2% | 0.7% | 4.8% | 17.1% |
| above 100B | 409 | 0.05% | 1.2% | 0.9% | 3.0% | 15.9% |

Companies with 5 or more years, by size of the median revenue (mean spend over all years of the company):

| Size | Companies | Median / mean of the company means | Mean spend 2% or more | Regular | CV of the spend of regular companies (median) |
|---|---|---|---|---|---|
| 1B to 10B | 421 | 3.3% / 6.2% | 59.6% | 19.7% (83) | 1.22 |
| 10B to 30B | 152 | 1.2% / 2.9% | 40.1% | 12.5% (19) | 1.20 |
| 30B to 100B | 65 | 0.9% / 2.3% | 32.3% | 7.7% (5) | 1.80 |
| above 100B | 23 | 0.3% / 1.1% | 21.7% | 8.7% (2) | 1.12 |
| 10B and above | 240 | 1.1% / 2.6% | 36.2% | 10.8% (26) | |

**Lumpiness.** The spend is lumpy (CV 1.1 to 1.8). Examples (`conv_study co`): IBM spend 2021 to 2025: 5.7, 3.9, 8.2, 5.2, 12.3%;
JNJ 2021 to 2025: 0.1, 22.1, 0.0, 17.1, 18.6%. The mean of the last 5 years is the model charge (`acq_charge`). Does the mean
last? Mean spend of the next 3 years against the mean of the past 5 years (company-years with 5 past and 3 next years):

| Past 5-year mean | Companies of 10B and above: mean past | mean next 3 | ratio of means | median of ratios |
|---|---|---|---|---|
| below 0.5% | 0.07% | 0.79% | 11.4 (tiny base) | 1.21 |
| 0.5% to 2% | 1.14% | 1.96% | 1.72 | 0.55 |
| 2% to 5% | 3.25% | 3.57% | 1.10 | 0.46 |
| above 5% | 12.24% | 6.70% | **0.55** | **0.24** |
| all (regression) | next = 1.39% + 0.39 x past (n 2,299) | | | |

High spenders spend about half as much in the next 3 years (the mean), and the typical high spender spends a quarter
(the median).

### 2.2 How much growth is bought

Panel regression, revenue growth of year `t` on `a(t) = spend(t) / revenue(t-1)` and `a(t-1)`. Company effects, and year
effects where stated. Standard errors clustered by company (in brackets). Jumps above 60% and `a` above 50% are out.

| Sample | Observations (companies) | `b0` (same year) | `b1` (year before) | Sum |
|---|---|---|---|---|
| all, company effects | 7,595 (778) | 0.25 (0.03) | 0.20 (0.03) | 0.45 |
| all, company and year effects | 7,595 (778) | 0.21 (0.03) | 0.20 (0.03) | 0.41 |
| 1B and above, company and year effects | 6,849 (720) | 0.22 (0.03) | 0.21 (0.03) | **0.42** |
| 10B and above | 2,925 (351) | 0.14 (0.06) | 0.27 (0.06) | 0.41 |
| 1B to 10B | 3,924 (536) | 0.24 (0.03) | 0.19 (0.03) | 0.43 |
| 1B and above, spenders only | 4,066 (574) | 0.23 (0.03) | 0.21 (0.03) | 0.44 |

The coefficient of the second lag is 0.00 to 0.01 (SE 0.02 to 0.05). One dollar of spend buys **0.42 dollar of revenue**, half in the
year of the spend and half in the next year. That is a price of 2.4 times revenue.

Check against a source: PitchBook, *2025 Annual Global M&A Report*: median EV / revenue 1.7 times (global) and 2.2 times (US) in
2025. That is 0.59 and 0.45 dollar of revenue for each dollar. The panel value 0.42 is near the US value. **We did not open the PDF
(HTTP 403); we read these two numbers in a search summary.** The IBM review (F4, F16) has a much lower value: a growth credit of 1.0
point for a spend of 7.1% of revenue, so **0.14** (software deals at high multiples). The value changes by a factor of three
between companies.

Organic growth of acquirers against non-acquirers (revenue 1B or more; organic = growth - `b0 a(t)` - `b1 a(t-1)` with the 1B-and-above
estimate):

| Group | Observations (companies) | Total growth, median | Organic growth, median | Organic, points against the median of the year |
|---|---|---|---|---|
| 1B to 10B, regular acquirers | 739 (96) | 7.4% | 4.0% | -1.9 |
| 1B to 10B, spend below 0.5% | 947 (124) | 6.7% | 6.7% | +1.3 |
| 10B and above, regular acquirers | 300 (40) | 4.8% | 2.2% | -3.6 |
| 10B and above, spend below 0.5% | 904 (106) | 3.3% | 3.3% | -2.6 |

Regular acquirers grow faster in total (+0.7 to +1.5 points) and slower in organic growth (-1.1 to -2.7 points). A part of their
growth is bought.

### 2.3 What the model does now with an acquirer

The growth history keeps the bought growth (jumps above 60% with cash for businesses are out, `growth_organic`). The shrink to the
cell mean removes a part of it. The weight of the history in the growth mean (`w`, printed by `fa dcf auto T --explain`, rule of
"growth: company mean") is 0.50 to 0.56 for most large and mid companies: IBM 0.553, JNJ 0.562, ABT 0.505, AMGN 0.519, ROP 0.561,
BRO 0.500, KO 0.558, CSCO 0.479; MSFT 0.841; PFE 0.175.

Ten acquirers (chosen by a rule before we looked at the results: revenue 5B or more, latest year 2024 or later, spend above 0.5%
in at least 4 of the last 5 years, highest median spend). Numbers: growth of year 1 / company mean `mu` / charge `c` (% of revenue),
from `model_at` of the last year. Columns: default; `acq_w` (history stays); `acq_f=0,acq_mult=0.42` (organic only); `acq_f=0.5,acq_mult=0.42`;
`acq_f=1` (full charge).

| Ticker | Revenue | Median spend | History (last 4 growth values) | Default | `acq_w` | f=0 | f=0.5 | f=1 |
|---|---|---|---|---|---|---|---|---|
| ROP | 7.9B | 41.6% | 12.3, 13.9, 15.0, 11.1 | 11.3 / 11.1 / 0 | 11.3 / 11.1 / 23.6 | -2.8 / -1.3 / 0 | 4.2 / 4.8 / 21.0 | 11.3 / 11.1 / 42.1 |
| BRO | 5.9B | 18.5% | 22.8, 12.9, 19.1, 17.1 | 14.9 / 13.3 / 0 | 14.9 / 13.3 / 23.2 | 0.4 / 3.7 / 0 | 7.4 / 8.2 / 23.2 | 14.9 / 13.3 / 46.5 |
| EQT | 8.6B | 16.6% | -23.7, -7.9, 144.6, 0.2 | 9.1 / 9.1 / 0 | 9.1 / 9.1 / 0.3 | 9.0 / 9.0 / 0 | 9.1 / 9.1 / 9.1 | 9.1 / 9.1 / 18.3 |
| WCN | 9.5B | 15.6% | 6.1, 11.2, 11.2, 17.2 | 9.5 / 10.2 / 0 | 9.5 / 10.2 / 9.6 | 4.1 / 5.2 / 0 | 6.8 / 7.7 / 8.7 | 9.5 / 10.2 / 17.4 |
| APH | 23.1B | 14.2% | 51.7, 21.3, -0.5, 16.1 | 15.7 / 6.7 / 0 | 15.7 / 6.7 / 2.0 | 13.8 / 6.5 / 0 | 14.8 / 6.6 / 6.1 | 15.7 / 6.7 / 12.2 |
| IR | 7.7B | 14.0% | 5.7, 5.2, 16.2, 14.8 | 9.0 / 9.6 / 0 | 9.0 / 9.6 / 9.0 | 3.8 / 5.5 / 0 | 6.3 / 7.4 / 8.5 | 9.0 / 9.6 / 17.0 |
| BSX | 20.1B | 12.7% | 19.9, 17.6, 12.3, 6.7 | 11.3 / 9.2 / 0 | 11.3 / 9.2 / 8.3 | 6.2 / 5.0 / 0 | 8.8 / 7.1 / 8.0 | 11.3 / 9.2 / 15.9 |
| AME | 7.4B | 12.6% | 6.6, 5.2, 7.3, 10.9 | 7.7 / 8.0 / 0 | 7.7 / 8.0 / 10.2 | 3.6 / 3.6 / 0 | 5.6 / 5.8 / 9.1 | 7.7 / 8.0 / 18.1 |
| TDG | 10.6B | 11.6% | 19.5, 11.2, 20.6, 21.3 | 12.8 / 11.1 / 0 | 12.8 / 11.1 / 9.8 | 6.9 / 6.6 / 0 | 9.9 / 8.9 / 9.5 | 12.8 / 11.1 / 19.1 |
| SSNC | 6.3B | 11.0% | 6.6, 6.9, 4.2, 4.6 | 6.8 / 6.9 / 0 | 6.8 / 6.9 / 6.7 | 3.2 / 3.8 / 0 | 5.0 / 5.3 / 6.0 | 6.8 / 6.9 / 11.9 |

For these companies there is no "history without the acquisition years": each year has spend. The column f=0 is the history without
the bought growth, with `m = 0.42`. It cuts the growth of year 1 by 0.1 to 14.5 points. For ROP and BRO it gives a growth near zero or
below: with `m = 0.42` the model takes out more growth than the history has. **The single `m` of the panel is too high for serial
buyers of high-multiple businesses.** A rule that needs `m` is not safe for them.

### 2.4 Candidate rules

All three are pairs. A pair is consistent when the growth that the model credits is paid for by the spend that the model charges.

| Rule | Formula | Data it needs | Option |
|---|---|---|---|
| (i) Organic only | No charge. Each growth value of the history loses `m` times the spend ratio (half in the year, half in the next year). | spend, revenue, `m` (not in the data) | `acq_f=0,acq_mult=m` |
| (ii) Normal spend with full history | Charge `f` times the mean spend ratio of the last 5 years. History unchanged. With `f = 1`: the full charge. | spend, revenue | `acq_charge=1` (f = 1), `acq_f=f,acq_mult=0` |
| (ii') Charge what the shrink credits | Charge `w` times the mean spend ratio. History unchanged. `w` is the weight of the history in the growth mean. | spend, revenue, the shrink weight (in the model) | `acq_w=1` (new) |
| (iii) IBM pair | Charge `f` times the mean spend ratio. Each growth value of the history loses `(1 - f) m` times the spend ratio. | spend, revenue, `m` | `acq_f=f,acq_mult=m` |

**Free cash flow after acquisitions** is the cash flow that an owner gets: `OCF - capex + (1 - t) interest - interest income - SBC -
acquisitions`. The point-in-time test uses it when any of `acq_charge`, `acq_f` or `acq_w` is on (`fcff_of_rules`). The model without a
charge is measured with `acq_f=0,acq_mult=0` (no change to the model, only to the real value).

Measured (screen of 2026-11-19; `fa dcf pit`, `fa dcf returns` with the same `--rules`). Error = median of (real - P50) / revenue of T, in % (negative: the model is too high). Inside = share of real values in P10 to P90 (target 80%).

| Rule (`--rules`) | Ratio, all [stable] | Change | P50 at or below 0 | Free cash flow after acquisitions, error T+1 / T+2 / T+3 | Inside P10 to P90, T+1 / T+2 / T+3 | Revenue error T+1 / T+2 / T+3 | 3-year rank corr (t) |
|---|---|---|---|---|---|---|---|
| Base (`acq_f=0,acq_mult=0`) | 0.489 [0.523] | | 8.8% | -1.7 / -1.4 / -0.6 | 77.3 / 78.0 / 78.4 | -0.4 / +0.7 / +1.6 | +0.123 (4.5) |
| (i) `acq_f=0,acq_mult=0.42` | 0.428 [0.454] | -0.061 | 9.0% | -1.6 / -1.2 / -0.4 | 77.2 / 77.5 / 77.9 | +0.5 / +2.5 / +4.5 | +0.120 (4.0) |
| (i) `acq_f=0,acq_mult=0.14` (IBM `m`) | 0.464 [0.502] | -0.025 | 8.8% | -1.6 / -1.3 / -0.5 | 77.2 / 77.9 / 78.2 | -0.1 / +1.2 / +2.6 | +0.123 (4.4) |
| (iii) `acq_f=0.25,acq_mult=0.42` | 0.383 [0.437] | -0.106 | 9.7% | -0.9 / -0.5 / +0.4 | 76.7 / 77.3 / 77.2 | +0.3 / +2.1 / +3.9 | +0.098 (3.1) |
| (iii) `acq_f=0.5,acq_mult=0.42` | 0.358 [0.408] | -0.131 | 12.5% | -0.2 / +0.2 / +1.1 | 75.4 / 76.1 / 76.1 | +0.1 / +1.6 / +3.1 | +0.070 (2.4) |
| (iii) `acq_f=0.5,acq_mult=0.14` (IBM `m`) | 0.370 [0.446] | -0.119 | 12.5% | -0.3 / +0.2 / +1.1 | 75.5 / 76.3 / 76.4 | -0.2 / +1.0 / +2.1 | +0.072 (2.5) |
| (iii) `acq_f=0.75,acq_mult=0.42` | 0.343 [0.401] | -0.146 | 17.4% | +0.3 / +0.7 / +1.8 | 73.6 / 74.7 / 74.3 | -0.2 / +1.1 / +2.3 | +0.051 (1.8) |
| (ii) `acq_f=0.5,acq_mult=0` | 0.373 [0.446] | -0.116 | 12.1% | -0.3 / +0.1 / +1.1 | 75.6 / 76.5 / 76.5 | -0.4 / +0.7 / +1.6 | +0.073 (2.6) |
| (ii) full: `acq_f=1` (= `acq_charge=1`) | 0.309 [0.378] | -0.180 | 22.5% | +0.7 / +1.4 / +2.4 | 71.5 / 72.7 / 72.4 | -0.4 / +0.7 / +1.6 | +0.036 (1.4) |
| **(ii') `acq_w=1`** | **0.382 [0.446]** | **-0.107** | **12.1%** | **-0.4 / 0.0 / +0.8** | **76.3 / 77.1 / 77.4** | **-0.4 / +0.7 / +1.6** | **+0.091 (2.7)** |

Companies with a past spend above 5% of revenue (1,650 company-years), error of free cash flow after acquisitions, T+1 / T+2 / T+3:
base -3.3 / -2.9 / -2.4; `acq_f=0.25`: +0.1 / +0.7 / +1.5; `acq_f=0.5`: +2.8 / +3.3 / +4.6; `acq_w`: +1.8 / +2.2 / +3.1; full charge: +7.9 / +8.8 / +10.5.
For the group 2% to 5% (981 company-years): base -2.0 / -1.8 / -0.9; `acq_f=0.5`: -0.2 / 0.0 / +1.0; `acq_w`: -0.5 / -0.2 / +0.7.

Reading.
1. **The model without a charge overstates the cash that the owner gets.** Median -1.7% of revenue at T+1; the share of real values
   below P10 is 17.3% (target 10%). For companies that spend more than 5% the error is -3.3% and 27% of the real values are below P10.
2. **A charge of about half the mean spend makes the median error zero** (`acq_f` 0.4 to 0.5 over all companies; about 0.25 for the
   companies with spend above 5%, because the median spender spends much less than the mean). The coverage (inside P10 to P90) gets
   worse with the charge (78% to 72%): the band moves down and more real values are above P90.
3. **The revenue is best predicted with the full history** (error +1.6% at T+3; +4.5% for organic only; +8.1% for acquirers with
   spend above 5% under organic only). The acquirers keep buying, so the total revenue includes bought growth. So rule (i) predicts
   the total revenue worse. Rule (i) has no charge, so it also predicts the cash after acquisitions worse. Rule (i) is the "no value
   from acquisitions" convention, not a forecast.
4. **The charge lowers the rank correlation with later returns** (0.123 at no charge, 0.091 for `acq_w`, 0.036 for the full charge).
   This is secondary evidence, and a choice made on it would be a fit to past returns. We do not use it. It says only that the screen
   ranks the buyers lower, and the market did not do that in the sample.

### 2.5 The reviewed IBM decision against the panel

The IBM review (F2, F16, `analysis/IBM/REVIEW.md`) pairs a charge `f x` 7.06% with a growth credit `f x` 1.0 point, and sets `f = 0.5`.
It is a judgment, and the review published the locus of `f` from 0 to 1. We did not change these files.

| Test | Result | `f` |
|---|---|---|
| Spend of IBM, mean 2021 to 2025 | 7.07% (panel: 5.74, 3.88, 8.22, 5.24, 12.28); the review has 7.06% | |
| Persistence in the panel (10B and above): next 3 years = 1.39% + 0.39 x past | IBM next 3 years: 4.15%, which is 0.59 of the past mean | 0.59 |
| Same, class above 5% | ratio of means 0.55; median of ratios 0.24 | 0.55 (mean), 0.24 (median) |
| Mean spend of IBM 2008 to 2018 (panel), as a share of the 2021 to 2025 mean | 3.1% / 7.07% | 0.44 |
| Shrink weight of the growth history of IBM (`fa dcf auto IBM --explain`) | `w = 0.553` | 0.55 |
| Point-in-time zero error of free cash flow after acquisitions | all companies about 0.45; spend above 5%: about 0.25 | 0.25 to 0.45 |
| Revenue per dollar: IBM 0.14 (F16); panel 0.42 | the credit per dollar of IBM is one third of the panel value | not `f` |

IBM `f = 0.5` lies inside the band 0.25 to 0.59 that the panel gives, and it is near the shrink weight 0.55. JNJ (hand model, charge about 5.8%
of revenue = 0.5 x 11.6%, the mean of the last 5 years): the shrink weight is 0.562, so `acq_w` gives 6.5%. The automatic model for IBM gives
P50 $132.87 with no charge, $78.64 with `acq_w=1` (change -$54.2), and **$35.69 with the full charge (change -$97.2)**. The review
F2 lists "up to -$97" for the locus from 0 to 1. The automatic model reproduces that change at `f = 1`. (`fa dcf auto IBM --as-of 2026-11-19
--sims 10000 --seed 7 --rules R`. This is not a comparison with the hand model P50 of $151.84: the two models differ in many inputs.)

### 2.6 Recommendation for Q2

1. **The general rule is the pair (ii'): charge `w` times the mean spend ratio of the last 5 years, and keep the growth history
   (`--rules acq_w=1`).** `w` is the weight of the history in the growth mean, which the model already has.
2. **Reasons.** (a) It is consistent by construction. The shrink already credits `w` of the history growth, so the charge is the same
   share of the spend. The unknown `m` cancels, and the data of the company has no bought revenue, so this is the only pair that
   needs no number from outside the data. (b) It gives 0.55 for IBM and 0.56 for JNJ: the same as the hand decisions (`f = 0.5`).
   (c) In the point-in-time test, the error of the free cash flow after acquisitions is -0.4 / 0.0 / +0.8% (base: -1.7 / -1.4 / -0.6)
   and the revenue test does not change. (d) It uses the standard variables only.
3. **Cost.** The median ratio falls by 0.107 (to 0.382). 12% of the tickers have P50 at or below zero (base 8.8%). The return-test
   correlation falls from 0.123 to 0.091 (secondary evidence).
4. **Do not use rule (i) as a general rule.** It needs `m`, which is 0.14 for IBM and 0.42 for the panel, and it gives a growth near
   or below zero for serial buyers (ROP, BRO). Use it only as a measurement of the "no value from acquisitions" convention.
5. **What a person must decide.**
   - Whether the default stays "no charge" (the screen overstates the owner cash flow by 1.7% of revenue, 3.3% for heavy buyers), or
     becomes `acq_w`. A default change needs a review of the same weight as the IBM review (rule 3 of `CLAUDE.md`).
   - Whether acquisitions have zero value for the owner. Source (Koller, Goedhart, Wessels, 5th ed., chapter 21, p. 445): "most empirical
     research shows that one-third or more of acquiring companies destroy value for their shareholders". If a deal has zero net present
     value, rule (i) is the right convention; if it has negative value, a charge is right. The data cannot say which.
   - The mean of the last 5 years as the base of the charge. Spend is lumpy; JNJ has three deals of 17% to 22% of revenue in its 5-year mean.
6. **What stays uncertain.** `m` by company; the net present value of the deals; the median against the mean in the point-in-time
   test (a median test gives `f` near 0.25 to 0.45, the mean of the spend gives 0.55); 33% to 48% of company-years have no reported
   spend and count as 0; survivorship; the 5-year window; the shrink weights come from the table of 2026-11-19.

## 3. Code changes (all options are off by default)

| Change | File | Test |
|---|---|---|
| `--rules long_run=X`: fade level and mean terminal growth = lower of the 10-year zero yield and `X` (`long_cap`, `terminal_rf` win) | `rust/src/dcf/auto.rs` (`Rules::long_run`) | `one_long_run_growth_is_the_lower_of_the_yield_and_the_number` |
| `--rules acq_f=F,acq_mult=M`: the IBM pair | `auto.rs` (`Rules::acq_f`, `acq_mult`, `chain`) | `the_paired_acquisition_rule_charges_a_share_and_takes_out_the_rest_of_the_bought_growth` |
| `--rules acq_w=1`: charge `w` times the mean spend, history unchanged | `auto.rs` (`Rules::acq_w`) | `the_charge_can_follow_the_weight_of_the_growth_history` |
| `fa dcf pit`: real free cash flow after acquisitions when a charge rule is on; new table "by acquisition spend of the last 5 years" | `auto_pit.rs` (`fcff_of_rules`, `PitObs::acq_past`) | the existing PIT tests (field added) |
| `fa dcf screen --out`: new columns `tv_share`, `acq_ratio` at the end | `auto_cli.rs` | `csv_line` test (column count) |

The help text of `fa dcf auto --help` lists the new options. The default model and the reviewed numbers did not change:
`fa dcf drivers IBM` gives $83.84 / $151.84 / $258.03 (seed 7).

## 4. Commands that made the numbers

```bash
cargo build --release --manifest-path rust/Cargo.toml
fa dcf screen --tickers-file rust/mapcheck/universe/sp500.txt --cache data/raw --as-of 2026-11-19 --out OUT.csv [--rules R]
fa dcf pit --cache data/raw [--rules R]           # 2,000 paths, seed 7
fa dcf returns --cache data/raw [--rules R]       # 1,000 paths, seed 7
fa dcf auto IBM --cache data/raw --as-of 2026-11-19 --explain --sims 10000 [--rules R]
```

R values: `long_cap=0.0225`; `terminal_rf=1`; `long_run=0.038`; `long_cap=0.0225,fade_years=10`;
`terminal_growth=0.038,long_cap=0.038,fade_years=10`; `fade_years=10`; `acq_f=F,acq_mult=M`; `acq_w=1`. The medians of the screen
are over the tickers with a P50 and a price (430). The panel and the 10-year tests are the scratch program `conv_study` (modes
`q1`, `q2`, `long`, `co`); its output is in `analysis/priors/dcf_conventions_study_2026-11-20_panel.txt`.
