# NVDA: what is the share worth? Report draft

**Draft. It has not passed the four steps and has no approval of the owner. Do not use it alone for a trade.** The values are model values, not market prices.

Version 5 of 2026-11-21. Model date 2026-11-20. Data store `1804f48`: SEC facts of 2026-11-20, NVDA prices to 2026-11-19, the 8-K of 2026-11-17 (quarter end 2026-10-25), and 21 earnings releases (Q4 FY22 to Q3 FY27). The 10-Q of that quarter is not filed.

Files:
- `analysis/NVDA/driver_model_paths.toml`: the model.
- `analysis/NVDA/research-ledger.md`: the outside facts (L1 to L27).
- `analysis/NVDA/COMMANDS.md`: the command for each number, in the order of this report, with seed, paths and store commit.

## Result in brief

- **Range** (value for each share, USD; 100,000 paths, seed 7): P10 54.92, **P50 103.78**, P90 200.40, mean 119.07. The P50 of 14 seeds is 103.59 to 104.05 with 100,000 paths, and 102.70 to 105.09 with 10,000 paths.
- **Five choices and their effect on the P50.**
  1. Year-1 growth. The guide gives $98.60. The full median beat gives $109.23. The base (half the median) gives $103.78.
  2. The share r of the equity purchases that keeps its value. It runs from $98.42 (r = 0) to $109.08 (r = 1). The base is r = 0.5. No data supports it.
  3. The speed of the growth fade (D3). The base rates give $88.75 to $92.45. The base gives $103.78. A year 2 at the guide pace gives $92.02.
  4. The collapse of demand (D14). Its multiple of 2.0 has no source and sets the low tail. The P10 runs from $41.46 to $64.31 for a multiple of 0.5 to 4.0.
  5. The cost of capital. A premium of 3.5% gives $122.41 and 5.5% gives $88.29. A beta of 1.2 gives $132.05.
- **Distance to the price.** The close was $145.98 on 2026-11-19. It is at about the P75. The P50 is 28.9% below it. The model value is below the close on 75.0% of the paths.
- **Open points.** The 10-Q is not filed. The close of 2026-11-20 is not in the store.
- **How firm the range is.** The rule for the year-1 growth was set after the earlier runs (W1). Most steps raised the P50 (`COMMANDS.md`, section 7). The last change lowered it from 108.40 to 105.09 (10,000 paths). See W15.
- **Other limits:** W5, W6, W13, W14.
- **Purpose.** The owner can use this range to judge whether the price assumes more growth than the company's record and guide support. It is not a price target and not a signal to trade.

## 1. Result in detail

**Headline: 100,000 paths, seed 7.** Every number of this report has these paths and this seed, unless the text gives other values.

| Measure | Value for each share (USD) |
|---|---|
| P10 | 54.92 |
| P25 | 74.36 |
| **P50** | **103.78** |
| P75 | 146.06 |
| P90 | 200.40 |
| Mean | 119.07 (standard error 0.21) |

Command: `fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --sims 100000`. Two runs give the same text. The default run (10,000 paths) gives P10 54.72, P25 74.68, P50 105.09, P75 147.10, P90 200.29 and mean 119.81. Seed 7 is the highest of the 14 seeds with 10,000 paths. The noise of the headline P50 is about $0.5.

**The base of the year-1 growth is 44.9%.** The guide gives +41.0%. Half of the median beat of 19 pairs adds +2.78%. The rule is in section 3, below the driver table.

| Year-1 growth | P10 | P50 | P90 | Mean |
|---|---|---|---|---|
| 16.7% (a miss like Q2 FY23, -17.2%) | 42.11 | 72.86 | 131.22 | 81.76 |
| 41.0% (the guide, +3.5% to +4% a quarter after Q4) | 52.70 | 98.60 | 189.15 | 112.87 |
| 42.1% (a constant +4.2% a quarter after Q4) | 53.35 | 100.04 | 192.25 | 114.60 |
| **44.9% (base: half the median beat)** | **54.92** | **103.78** | **200.40** | **119.07** |
| 48.9% (the full median beat) | 57.21 | 109.23 | 212.65 | 125.58 |

**Year 2 at the guide pace.** At the means, year 2 is +24.4%. The guide pace is +3.5% to +4.2% a quarter. It gives +15% to +18% for year 2. A half-life of 0.58 years gives +16.4%.

With this half-life the P10 is 51.30, the P50 is 92.02 and the P90 is 169.73 (mean 103.84), against 54.92, 103.78 and 200.40 for the base. The base is $11.8 above the guide pace for year 2.

**The range across r.** Part of a purchase of an equity security keeps its value. This part is the share r. The other part pays a customer to buy. This part is the share 1 - r. The marketable securities ($44.2B) have marks every quarter and net gains of $25.5B in nine months. They are in the bridge at full value, and r does not apply to them. The non-marketable securities ($65.5B) are in the bridge at r times the carrying value. The future purchases are charged at (1 - r) times the four-quarter rate (19.6% of revenue, divided by 0.79 for tax). The charge fades with the growth.

| r | Net debt ($B) | Start of the charge | P10 | P50 | P90 | Mean |
|---|---|---|---|---|---|---|
| 0 (non-marketable out, full charge) | -65.4 | 26.3% | 50.47 | 98.42 | 193.89 | 113.69 |
| 0.25 | -81.7 | 20.1% | 52.69 | 101.10 | 197.16 | 116.38 |
| **0.5 (base)** | **-98.1** | **13.9%** | **54.92** | **103.78** | **200.40** | **119.07** |
| 0.75 | -114.5 | 7.7% | 57.14 | 106.49 | 203.81 | 121.76 |
| 1 (all stakes in, no equity charge) | -130.9 | 1.5% | 59.28 | 109.08 | 207.87 | 124.64 |

The P50 moves by $10.7 over the range of r. The headline rests on r = 0.5, a choice with no data.

Other facts of the run:
- 18.1% of the paths have a demand collapse in ten years (D14).
- No path is below $30. 2.8% of the paths are below $40.
- The collapse multiple has no source. It decides the lowest values. A collapse in year 1 gives 2.0 x $361.8B x 0.896 = $648B. With net cash and stakes of $98B, this is $31 a share. The P10 is $41.46, $46.83, $51.29, $54.92 and $64.31 for a multiple of 0.5, 1.0, 1.5, 2.0 and 4.0.
- A multiple of 2.0 assumes a milder bust than MU (revenue -49.5% and operating margin -37% in FY23).
- The engine cannot impair the bridge on a failed path. A collapse path keeps the stakes ($3.18 a share at r = 0.5, $4.54 at r = 1). It carries no guarantee claim (up to $4.49 a share; $108.5B on 2026-07-26, L12). So the P10 is too high by up to about $3.2.
- The terminal value is 39.9% of the enterprise value at the means. The enterprise value at the P50 is $2,411.9B, or 6.7 times the trailing revenue. The value at the means is $113.93. A hand calculation of the same cash flows (Python, collapse off) gives $124.19. The engine gives $124.22 for the same inputs.
- Year 1 at the means: revenue $524.3B, EBIT margin 50.0% (66.0% less SBC 2.1% less charge 13.9%), reinvestment $54.2B, free cash flow to the firm $152.8B.

## 2. The business

NVIDIA designs processors and systems. The fiscal year ends in late January. Revenue was $215.9B in fiscal 2026 and $361.8B in the four quarters to 2026-10-25. The last four quarters were $68.1B, $81.6B, $96.2B and $115.8B. Data Center was $107.6B of the last quarter, and Edge Computing was $8.2B (L2a). The GAAP operating margin was 66.9% in the last quarter and 15.7% in fiscal 2023.

The company holds $53.5B of cash and marketable debt securities, and has $32.4B of debt (L2f). It holds $109.7B of equity securities (L11). The company guides $130.0B of revenue for the next quarter, which has 14 weeks (L2d). The 8-K gives a segment split for Q3 only, so the model has one segment. The 8-K is unaudited. The analyst traced each number to the page (ledger A2). The press release names a leadership change and an investigation by a committee of the board (L2h). The model has no input for them (W14).

## 3. The drivers

Tags: DATA is a reported number or the calibration table. CONVENTION is a record of this repo. JUDGMENT is the analyst. "Stoch." means drawn one time for each path. A process has a yearly shock in addition. The reasons are in the model file.

| Id | Driver | Value | Distribution | Source and tag |
|---|---|---|---|---|
| D1 | Base revenue | $361.807B | fixed | Four quarters to 2026-10-25 (L1, L2a). DATA |
| D2 | Growth of year 1 | **44.9%**, std 12%, cut 10% to 75% | stoch. | Guide plus half the median beat. The rule is below this table. DATA + JUDGMENT |
| D3 | Half-life of the growth fade | 1.0 year, std 0.4, cut 0.4 to 3.0 | stoch. | The panel (0.6 to 0.7 years, L19) is the lower bound. The guide shows the growth falling from +20% to +4% a quarter. JUDGMENT |
| D4 | Long-run growth | 4.0% | fixed | Record A-001 (L18). CONVENTION |
| D5 | Yearly shock of growth (the cycle) | 15% a year, or 30% of the slow part if that is larger; persistence 0; cut -50% to +200% | yearly shock | Own history: two busts in ten years (L3). JUDGMENT + DATA |
| D6 | Margin before SBC, year 1 | 66%, std 3% | stoch. | GAAP margin Q3 66.9%, trailing 66.1%. The Q4 guide gives about 63.0% (L1c, L2b, L2d). DATA + guidance |
| D7 | Long-run margin before SBC | 48.5%, std 8%, cut 25% to 65% (GAAP 45%) | stoch. | Middle of the own history: 31% (FY17 to FY22) and 59% (FY24 to FY26) (L3). JUDGMENT |
| D8 | Half-life of the margin fade | 3 years, std 1 | stoch. | JUDGMENT |
| D9 | Yearly shock of margin | 6 points, persistence 0.5, correlation 0.6 with the growth shock | yearly shock | Calibration table and own history (L3). JUDGMENT |
| D10 | SBC (a cost, rule F5) | 2.12% in year 1, to 3.5% (std 1%) in year 10 | stoch. target | L6, L2c. DATA + JUDGMENT |
| D11 | Charge (rule F16), two parts | Part A: purchases of equity securities, (1 - r) x 19.6% / 0.79 = 12.4% of revenue (std 2%). Part B: purchases of businesses, 1.5% (std 0.7%, cut 0 to 5%). Year 1 total: 13.9%. The charge falls to part B with the half-life of D3 (correlation 0.9) | stoch. | A: $70.90B of purchases in four quarters (L2e). B: $3.031B for acquisitions and $2.944B for Groq in nine months, 2.0% of revenue (L2e, L7). JUDGMENT |
| D12 | Sales to capital (reinvestment, rule F16) | 3.0 in year 1, to 4.0 (std 1.0) in year 10 | stoch. target | L4a. DATA + JUDGMENT |
| D13 | Tax rate | 21% | fixed | Statutory. The effective rate rose from 12.0% (FY24) to 17.4% (Q3). The guide is 16% to 18% for FY27 only (L5, L2d). CONVENTION |
| D14 | Demand collapse | hazard 2% a year (std 1%), value 2.0 times the revenue of the year before | stoch. hazard | The engine has no regime state. JUDGMENT |
| D15 | Beta | 1.681, std 0.09, lower bound 1.0 | stoch. | Regression 2.021 shrunk, standard error 0.088 (L8). DATA. It is a draw. The IBM model fixes the beta |
| D16 | Equity risk premium | 4.41%, std 0.75%, lower bound 2% | stoch. | L17. CONVENTION |
| D17 | Risk-free rate | Hull-White on the GSW curve of 2026-11-13, 10-year zero yield 5.13% | process | L16. CONVENTION |
| D18 | Equity weight | 1.0 | fixed | A market weight would use the price. The debt is below 1% of any plausible value. CONVENTION |
| D19 | Terminal growth | 4.0%, std 0.75%, capped by D4 and the risk-free rate | stoch. | Record A-001. CONVENTION |
| D20 | Terminal return on new capital | 35%, std 15%, cut 15% to 100% | stoch. | JUDGMENT |
| D21 | Net debt | -$98.124B: debt 32.371, less cash 29.089, less marketable debt securities 24.423, less marketable equity securities 44.219, less 0.5 x non-marketable securities 65.528 | fixed | Balance sheet 2026-10-25 (L2f, L10, L11). DATA + decision on r |
| D22 | Shares | 24,186M (diluted, Q3) | fixed | L2f, L9. DATA |

Cost of capital at the means: 12.55% in each year (risk-free 5.14% plus beta 1.681 times 4.41%). The model date is 2026-11-20, so the first cash flow is discounted for 339 / 365 = 0.9288 years.

**The rule for D2** (decision of the Head of Fundamental Research, 2026-11-21).
- The guide for Q4 is $130.0B in 14 weeks. In 13 weeks this is $120.7B, +4.2% on Q3.
- The four quarters of year 1 are 121 + 125 + 130 + 134 = $510B over $361.8B, or +41.0%. Quarters 2 to 4 are a judgment of about +3.3%, +4.0% and +3.1% a quarter. A constant +4.2% a quarter gives +42.1%.
- The store has 19 pairs of a guide and the next result (section 4). 18 are beats and one is a miss (Q2 FY23, -17.2%, pre-announced). The median is +5.57%.
- The base is the guide. If the median of all pairs is above zero and 15 or more of 19 (79%) are beats, the base adds half the median.
- Both tests hold, so the base is 1.41 x 1.0278 - 1 = 44.9%.
- Half is a cut that keeps the base low against the company's record. From the fourth pair on, the actual beat was at or above half of the earlier median in 15 of 16 quarters. It was at or above the full earlier median in only 9 of 16.

**The decision on the equity securities** (Head of Fundamental Research, 2026-11-21).
- One frame and one number r apply to the stakes and the purchases (IBM rule F16).
- The marketable securities are at full value. They have marks and gains.
- The non-marketable securities are at r times the carrying value.
- A future purchase is charged at (1 - r) times the four-quarter rate: ($17.502B - $4.702B + $58.100B) / $361.8B = 19.6%, divided by 0.79 because tax does not cover the purchases. On the page basis ($4.952B for nine months FY26, not the XBRL $4.702B) the charge start is 13.86%, against 13.90% on the XBRL basis (both 13.9% at one decimal).
- The base r is 0.5. There are no terms and no data for r. The value 0.5 says that the model does not know.
- The charge fades with the growth that it pays for (record A-002). It has the half-life of D3. In years 1 to 3 its mean is 58% of the start. The panel gives 0.55 (study 2.1, `analysis/priors/dcf_conventions_study_2026-11-20.md`). This is a cross-check.

Fade of the charge and the panel rows (r = 0.5; each row is a copy of the model file, see `COMMANDS.md`):

| Case | P10 | P50 | P90 | Mean |
|---|---|---|---|---|
| **Base: half-life 1.0 year (same as D3)** | **54.92** | **103.78** | **200.40** | **119.07** |
| Half-life 2.0 years | 53.46 | 101.06 | 196.03 | 116.21 |
| Ramp, 10 years | 51.75 | 98.32 | 194.02 | 114.01 |
| Panel row: start at 0.55 of the past | 56.31 | 105.61 | 202.88 | 120.88 |
| Q3 rate (13.5%) in place of 19.6% | 55.88 | 105.02 | 202.12 | 120.31 |
| Acquisitions (B) at 2.0% | 54.55 | 102.88 | 198.68 | 118.06 |

The panel row sets the start at 0.55 times the past charged rate (12.4%), plus the 1.5% of part B. A slower fade gives a lower P50. The ramp of 10 years costs $5.5. The rows with a ramp of 3 and 5 years, the panel factors 0.39 and 0.24, the net purchases and the case without tax gross-up are in `COMMANDS.md`.

**Pairing and stock-or-flow rules.**
- Growth has its cost (F16). The model charges reinvestment for each dollar of growth (D12), a part of the purchases of businesses and of equity securities (D11), and SBC (D10). The terminal value reinvests growth divided by the return on new capital (D20).
- No stock with its flow (F15). The margin is the operating margin. So the cash flow has no interest income and no gains on investments. The bridge has cash, debt securities and the equity securities. The lease cost is in the operating income, so the lease liability is not in the bridge (L13). Purchase obligations are costs of future product (L14).
- No debt twice (F1). The cash flow has no interest expense. The debt of $32.4B is subtracted one time.
- SBC is a cost (F5). GAAP operating income already has SBC. The model adds the SBC ratio to the GAAP margin and charges it as D10. The share count is the diluted count.
- Forward disclosures (R1): section 8 and the ledger (L20 to L24).

## 4. What drives the value

The attribution is the change of the P50 when the mean of one drawn number moves by one standard deviation.

| Driver | Minus 1 std | Plus 1 std |
|---|---|---|
| Growth of year 1 (D2) | -15.08 | +16.94 |
| Equity risk premium (D16) | +14.83 | -11.28 |
| Half-life of the growth fade (D3) | -11.33 | +14.76 |
| Long-run margin (D7) | -11.21 | +11.10 |
| Collapse hazard (D14) | +5.09 | -5.04 |
| Beta (D15) | +4.26 | -3.85 |

The other drivers (D8, D20, D6, D19, D10, D11, D12) each move the P50 by less than $3.4. Growth is the main source of the spread. When the growth draws and shocks are fixed at their means, 70% of the variance is gone. The margin removes 32% and the discount rate removes 24%. The headline command prints these shares (block "VARIANCE OF THE VALUE BY GROUP").

**Sensitivities** (P50; each row changes one number; see `COMMANDS.md`):

| Driver | Low case | P50 | Base | P50 | High case | P50 |
|---|---|---|---|---|---|---|
| Growth of year 1 (D2) | 16.7% (miss) | 72.86 | 44.9% | 103.78 | 48.9% (full median) | 109.23 |
| Equity risk premium (D16) | 5.5% | 88.29 | 4.41% | 103.78 | 3.5% | 122.41 |
| Long-run margin (D7) | 35% | 85.31 | 48.5% | 103.78 | 60% | 119.06 |
| Half-life of the growth fade (D3) | 0.4 years | 88.75 | 1.0 year | 103.78 | 2.5 years | 163.14 |
| Beta (D15) | | | 1.681 | 103.78 | 1.2 | 132.05 |
| Tax (D13) | | | 21% | 103.78 | 17% | 108.96 |
| Collapse hazard (D14) | 4% | 93.91 | 2% | 103.78 | 0% | 112.50 |

Other rows (P50): collapse multiple 0.5 gives $101.53 and 4.0 gives $109.98. Growth shock 25% gives $95.13 and 10% gives $107.31. Terminal return 15% gives $100.69 and 100% gives $107.28. Sales to capital 2 gives $102.76 and 6 gives $104.58. Margin start 64% gives $102.34 and 69% gives $105.94. No yearly shocks gives $110.45. Mid-year cash flows give $109.65. A shock correlation of 0.25 gives $103.54. Acquisitions (B) at 2.0% give $102.88.

**Guide against result (ledger L26).** The store has 21 earnings releases. They give 19 pairs of a guided revenue and the revenue of the next quarter. The guide of Q4 FY27 has no result yet. The result of Q4 FY22 has no guide in the store. The release of 2022-08-08 (`0001045810-22-000133`) is a preliminary result of Q2 FY23 ($6.70B). This report uses the final release of 2022-08-24 (`0001045810-22-000136`). The company pre-announced the miss.

| Window | Pairs | Beats | Median | Mean | Minimum | Maximum |
|---|---|---|---|---|---|---|
| All pairs (Q1 FY23 to Q3 FY27) | 19 | 18 | +5.57% | +5.61% | -17.23% | +22.79% |
| Before Q4 FY24 | 7 | 6 | +2.32% | +4.74% | -17.23% | +22.79% |
| Q4 FY24 to Q3 FY27 | 12 | 12 | +5.65% | +6.13% | +2.47% | +10.52% |

The limits: the 19 pairs are from one company with one upswing and one bust. The guide of Q4 has a wider band (3%, and 2% in all earlier guides) and an extra week.

### Base rates of scale and fade

The source is `[[fade]]` of `analysis/priors/process_priors_2026-11-19.toml` (companies with growth of 50% or more in the start year) and the facts of the cache (`mapcheck`, all 10-K rows, 1,976 companies). The cells have survivor bias. The cells are too high, not too low.

The cache has 12 start years above $100B (10-K rows, `COMMANDS.md` section 4). The prior cell `[[fade]]` counts 14 starts (its `n_start`; row "Starts" below). The growth rows give their own n. Of the 12 in the cache, eight are oil companies in 2021 and 2022 (XOM, CVX, MPC, PSX, VLO). The others are CI 2019 (a merger), MU FY26 and NVDA FY25 and FY26. So this cell is not a base rate of organic demand. It is a base rate of a price jump. The fall in years 2 and 3 is the end of the oil price.

| Item | Cell above $100B | Cell $30B to $100B | Model at the means |
|---|---|---|---|
| Starts (`n_start` of the prior cell) | 14 | 42 | |
| Median growth, year 1 (n) | +44.7% (12) | +12.5% (39) | +44.9% |
| Median growth, year 2 (n) | -13.3% (11) | -0.5% (38) | +24.4% |
| Median growth, year 3 (n) | -5.5% (11) | +3.3% (36) | +14.2% |
| Revenue after three years, from $361.8B | $429B | $418B | $745B |
| Largest revenue in the cache | AMZN $716.9B (year to 2025-12-31); WMT $713.2B (year to 2026-01-31) | | Year 10: $1,080B at the means; P50 over all paths $861B |
| Largest GAAP operating profit in the cache | AAPL $158.1B (year to 2026-09-26); MSFT $155.2B; NVDA $130.4B (FY26) | | Year 10 at the means: $509B ($1,080B x (50.7% - 3.5%)) |
| GAAP operating margin, company-years above $100B | MU 74.6% (FY26), NVDA 62.4% and 60.4%, MSFT 41.6% to 46.8%, META 34.7% to 42.2%, GOOGL up to 32.1%, AAPL up to 33.0% | | Year 10: 47.2% (D7 is 45%) |
| A bust in the cache | MU: revenue $30.8B (FY22) to $15.5B (FY23), -49.5%; operating margin -37% | | Collapse hazard 2% a year; shock 15% |

D3 and D7 sit above the base rate for these reasons.
- The cells are small (above $100B: 14 starts in the prior cell, 11 with a year-2 growth), they are mostly oil, and they hold busts. The model has busts in 18.1% of the paths, but the median path has none.
- NVDA reported +17.9% to +22.0% a quarter for the last five quarters. The guide is +4.2% in 13 weeks, not a fall. The company beat the guide in 18 of 19 quarters.
- The cells say that growth falls to zero or below in years 2 and 3. The model has +24% and +14%. This is the largest difference with the base rate.
- A half-life of 0.4 years follows the cell. The P50 is $88.75. A half-life of 0.6 years follows the panel of all sizes. The P50 is $92.45. The base gives $103.78. So D3 is above the base rate by $11.3 to $15.0.
- D7 (45% GAAP) is at the level of MSFT, the best large company with a long history in the cache. Only MU (one year) and NVDA are higher.
- The year-10 numbers of the model are far above the cache. Revenue is 1.5 times AMZN and operating profit is 3.2 times AAPL. The model has no base rate for them.

## 5. What the price implies

The last close in the store is **$145.98 on 2026-11-19** (L15). The close of 2026-11-20 is not in the store. The analyst saw the price while the versions were made. That no input comes from the price rests on the review record, not on this text alone.

- The P50 ($103.78) is 28.9% below the close. The mean ($119.07) is 18.4% below it. The close is at about the P75 ($146.06).
- The model value is below the close on 75.0% of the paths.
- Across r the P50 is $98.42 to $109.08. At the guide (41.0%) the P50 is $98.60, 32.5% below the close. At the full median beat it is $109.23, 25.2% below.
- This is a statement about the inputs. It is not a finding that the price is wrong.
- The largest open difference is the growth after Q4. The guide gives +4% on Q3 in 13 weeks. The consensus EPS (L23, not an input) needs much more.

One number alone makes the P50 equal to $145.98, with all other numbers at base. The rows marked `*` use `--shift-bounds`.

| Number | Base | Value for P50 = $145.98 |
|---|---|---|
| Growth of year 1 (D2) * | 44.9% | 72.3% |
| Half-life of the growth fade (D3) | 1.0 year | 2.08 years |
| Equity risk premium (D16) * | 4.41% | 2.72% |
| Beta (D15) * | 1.681 | 1.04 |
| Long-run margin before SBC (D7) * | 48.5% (GAAP 45%) | no solution up to 78% |

The close needs one of these. A year-1 growth of 72% is far above the base (45%) and the full median beat (49%). A fade twice as slow needs a different view of the company. A premium of 2.7% or a beta of 1.0 means a cost of capital about 2.8 points lower (1.681 x 1.69 = 2.84). A margin alone cannot reach the close. A lower number in one place can be paid by a higher number in another place.

NVDA fell from $206.69 (2026-11-03) to $155.46 (2026-11-17), or -24.8%, before the release. The price path is consistent with a release after the close of 2026-11-17 (no file in the store gives the time). The price fell from $155.46 to $145.98 (-6.1%) in the two sessions after it. The model has no fact for the fall before the release. The release has a result 7.3% above the guide and a guide of +4.2% on Q3. The second fact may explain the later days. The model does not use the fall.

## 6. Comparison with the automatic screen

`fa dcf auto NVDA --cache data/raw --as-of 2026-11-20 --explain` (seed 7, 10,000 paths) gives P10 1.18, P25 5.40, **P50 16.55**, P75 43.26, P90 104.57 and mean 43.85 (store `a1dffb5`). At store `1804f48` the beta is 1.675, the P50 is 16.57 and the mean is 44.00. With the code of `main` merged at `a7f030d` (store `1804f48`), the screen gives P10 2.58, P25 6.81, P50 17.98, P75 44.80, P90 106.22 and mean 45.40. The cause is in the code of `main` (pull requests 101 to 104); this model does not change. The screen uses the 10-K and the 10-Q of Q2. It does not see the 8-K. It is not a reviewed valuation (`docs/models/auto-path-model.md`). This model gives P50 $103.78. The gap was not split by input.

| Input | Automatic screen | This model |
|---|---|---|
| Base revenue | $215.9B (fiscal 2026) | $361.8B (four quarters to 2026-10-25) |
| Revenue of year 1 | $257.8B (+19.4%) | $524.3B at the means (+44.9%) |
| Margin, long run | 19.4% (shrunk toward the 7.9% mean of all companies above $100B) | 48.5% operating margin before SBC; free cash flow 35.1% of revenue in year 10 |
| Net debt | +$10.9B (cash only, 2026-07-26) | -$98.1B (cash, debt securities, equity securities, 2026-10-25) |
| Equity purchases | not charged | charged at 0.5 x 19.6% / 0.79, falling with the growth |
| Other | SBC 4.14%; no collapse of demand | SBC 2.12%; collapse of demand (D14); free cash flow 41.1% of revenue in the last four quarters (L2c) |

The screen shrinks the history of NVDA toward the mean of all large companies. For NVDA this mean is far from the facts. The free cash flow margin was above 14% in each of the last ten fiscal years and above 40% in the last three. The screen's revenue for year 1 is below the revenue that NVDA has already reported. So the screen does not fit this company (`analysis/priors/dcf_investigation_2026-11-20.md`, section 4.8). This model does not use the shrink.

## 7. How the model changed

Each row is one version. The P50 is for 10,000 paths. The step tables and the commands are in `COMMANDS.md`, section 7.

| Version | Main change | P50 |
|---|---|---|
| 1 (data of 2026-11-19) | Base on the 10-Q of Q2; year-1 growth 65%; collapse table (D14) added after a first run (P50 145.95 without it) | 130.89 |
| 2 | 8-K of Q3: base revenue $361.8B, year-1 growth 40%, half-life 1.0 year | 97.65 |
| 3 | Equity securities in the bridge at full value; purchases charged on a ramp of 10 years | 85.07 |
| 4 | One number r = 0.5; the charge fades with the growth; year-1 growth from the guide times the beat of 12 pairs (47.9%) | 108.40 |
| 5 | Marketable securities at full value; year-1 growth = guide plus half the median beat of 19 pairs (44.9%) | 105.09 (103.78 with 100,000 paths) |

The collapse table (D14) has this reason. The brief (ledger L27; from the session, not in git) asked for a downside of the cycle. The ticket also names the cycle of the capital spend of the customers as a driver (ticket line 21). The check of the growth process alone found a year below -30% growth on only 5.1% of the paths (`COMMANDS.md`). So the collapse table was added. The first run gave P50 145.95. That value is near the close by chance. No input was set from it.

No input comes from the price (section 5). The Head of Fundamental Research decided r, the fade and the base of D2. The balance sheet of the 8-K press release of 2026-11-17 (EX-99.1) confirmed the start of D12 (3.0).

## 8. Forward disclosures (rule R1)

| Disclosure | Decision | Reason |
|---|---|---|
| Remaining performance obligation, $3.2B on 2026-07-26 (L20) | Excluded | About 1% of the revenue. It is not for product orders. |
| Purchase obligations, $22.7B at 2026-01-25 (L21) | Plausibility check only | They show supply bought, not orders received. |
| Outlook for Q4 FY27 (L2d, L22): revenue $130.0B +-3% in 14 weeks, gross margin 71% to 72%, opex about $11.0B, tax 16% to 18%, no China data center compute | **Used** | It is evidence for D2, D3 and D6. The tax guide is not used for D13: it is for one year only. At 17% the P50 is $108.96. The record of the guides is in L26. The rule is in section 3. |
| Consensus EPS and targets (L23, L24) | Cross-check only | Not an input. The consensus EPS of the next four quarters is +70% over the last four. The model has revenue +44.9% in year 1 with a lower margin. This is the largest difference with the street. |

## 9. Weak points

- **W1. Year-1 growth rests on one guided quarter and 19 pairs.** The base is the guide plus half the median beat (44.9%). The Head of Fundamental Research decided the rule. The order is not provable from git, because the rule and the result are in one commit. The record is one company with one upswing and one bust. It has 18 beats and one large miss. At the guide the P50 is $98.60. At a constant +4.2% a quarter it is $100.04. At the full median it is $109.23. At a miss like Q2 FY23 it is $72.86. The guide of Q4 has a wider band and 14 weeks. No evidence for the quarters after Q4 is in the store except the record.
- **W2. D7, D3 and D8 are judgments.** D3 and D7 are above the base rate of scale (section 4). Year 2 at the means (+24.4%) is above the guide pace (+15% to +18%). The P50 at the guide pace is $92.02.
- **W3. The cost of capital is 12.55%.** It is the same in all ten years and in the terminal value. The beta of 1.68 comes from five years of weekly returns. A company with $1T of revenue can have a lower beta. A beta of 1.2 gives $132.05.
- **W4. The cycle is a wide shock plus a table that the engine made for failure (D14).** The engine has no regime. The hazard (2%) and the multiple (2.0) have no data except MU. The multiple sets the low tail. The mean-path table of the output counts failed paths as zero revenue. It is not a forecast.
- **W5. The model has one segment.** The 8-K gives Data Center and Edge Computing for Q3 only. It gives no history and no customer concentration.
- **W6. The value of r and of the stakes.** The value r = 0.5 has no data. The P50 runs from $98.42 (r = 0) to $109.08 (r = 1). The marketable stakes ($44.2B) are at full value. If they fall with the demand that the model credits, the P50 is too high by up to $1.83 a share. The terms of the non-marketable stakes are in the 10-Q notes, which the analyst did not read. A future purchase is charged at (1 - r) whether it would be marketable or not. The stakes held are split. The mix of the future purchases is not known. The engine cannot impair the bridge on a failed path, so the P10 is too high by up to about $3.2, and the guarantees (up to $4.49 a share) are not in the model.
- **W7. Two gaps in `mapcheck`.** The standard variable `short_term_investments` has no value for NVDA after 2025-10-26, because the filer uses the tag `DebtSecuritiesCurrent`. The screen net debt is wrong by $34.1B ($1.41 a share). The tag `PaymentsToAcquireBusinessTwoNetOfCashAcquired` ($13.0B in fiscal 2026) has no standard variable. The line "Groq, Inc." ($2.944B) is in the financing section. Purchases of equity securities have no standard variable. The analyst typed the numbers from the page and the company facts. The orchestrator decides.
- **W8. Tax at 21% is above the effective rate** (17.4% in Q3). The P50 is $108.96 at 17%. The engine has no draw for the tax rate.
- **W9. The terminal return (D20) is a judgment.** The P50 is $100.69 at 15% and $107.28 at 100%.
- **W10. The timing is simple.** Year 1 has all cash flows after the balance sheet, discounted from the end of year 1 (0.93 years). Cash paid to holders since 2026-10-25 is not removed. The share count is the Q3 weighted diluted count. The period-end count is lower after the buyback of $22.1B (under 1%), so the count is conservative. The extra week of Q4 is removed. Mid-year timing gives P50 $109.65.
- **W11. The right tail is long.** The P90 of year-10 revenue is $1.8T, far above the cache.
- **W12. Three small judgments.**
  - The panel that bounds D3 has survivor bias.
  - The correlation 0.6 between the growth and margin shocks rests on two events. With 0.25 the P50 is $103.54.
  - The correlation of the two half-lives is 0.9. The engine cannot share one number between two processes. The value 0.95 is not positive definite.
- **W13. The 10-Q is not filed.** The Q3 numbers are from the 8-K (unaudited). Convertible debt and accrued purchase consideration are not in net debt (L2i). The analyst did not read the 10-K text or the notes.
- **W14. The press release names governance items.** They are a leadership change and an investigation by a committee of the board (L2h). The model has no input for them. They may raise the discount rate or the collapse hazard. The size is unknown.
- **W15. The result is not stable. Most changes raised the P50, and the last one lowered it.** One filing moved the P50 by $33 (version 1 to version 2). The decisions on the stakes moved it by -$12.6 and then +$12.7. The beat of 12 pairs added $10.7, and the marketable stakes at full value added $0.9. The change that lowered the P50 came from the longer window (half the median of 19 pairs in place of the full median of 12). Evidence that lowers the P50 and is not in the base:
  - the base rate of years 2 and 3 (the model has +24% and +14%);
  - the guide pace for year 2 ($92.02);
  - a collapse milder than MU (a multiple of 1.0 gives P10 $46.83);
  - the guarantees (up to $4.49 a share);
  - a miss like Q2 FY23 ($72.86).

  The P50 is not a stable number before the 10-Q and the next guide.

## 10. Open items and remaining steps

1. The Head of Technology adds the 10-Q of Q3 FY27 and the close of 2026-11-20 to the store. Then check each row of ledger section A2 against the 10-Q. Read the guarantee note, the terms of the equity securities, the convertible debt and the investigation. Add the pair of Q4 FY27 to L26.
2. The orchestrator decides on W7 (new tags in `standard_variables.toml`) and on an engine feature that impairs the bridge on a failed path (W6).
3. Draft records of the assumptions for D3, D7, D11, D14 and D20 are A-004 to A-008 on the branch `claude/nvda-assumption-records`.
4. The ideas for the automatic model are in the ticket `PLANNING/board/T-20261121-dcf-engine-from-tickers.md`, section "Notes of the NVDA analyst, 2026-11-21".
5. The next steps are the auditor and the approval of the owner.
