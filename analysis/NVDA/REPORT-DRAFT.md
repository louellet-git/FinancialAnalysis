# NVDA: what is the share worth? Report draft

**Status: draft, not reviewed.** This text is step 1 of 4 of the review (analyst). The skeptic, the reviewer and the auditor have not seen it. Do not use it alone for a trade. The values are model values, not market prices.

Version 3 of 2026-11-21 (answer to the skeptic, round 1; version 2 is commit 41afd53, version 1 is commit 9a2e6f0). Model date 2026-11-20. Data store `a1dffb5`: SEC facts of 2026-11-20, NVDA prices to 2026-11-19, the 8-K of 2026-11-17 (fiscal Q3 FY27, quarter end 2026-10-25). The analyst read the 8-K page and traced each number (ledger section A2). The 10-Q of that quarter is not filed. Files: `analysis/NVDA/driver_model_paths.toml` (the model), `analysis/NVDA/research-ledger.md` (the outside facts, ids L1 to L24, with L2a to L2h for the 8-K page) and `analysis/NVDA/COMMANDS.md` (the command for each number).

## 1. Result

| Measure | Value for each share (USD) |
|---|---|
| P10 | 44.27 |
| P25 | 59.48 |
| **P50 (headline)** | **85.07** |
| P75 | 122.98 |
| P90 | 174.19 |
| Mean | 100.17 (standard error 0.60) |

Command: `fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml` (10,000 paths, seed 7). Two runs give the same text. Seed 11 gives P50 85.09. Seeds 7 and 12 with 100,000 paths give P50 84.85 and 84.85. The noise of the P50 is about $0.5. The mean is higher than the P50 because the right tail is long. Use the P50 as the headline.

- 18.5% of the paths have a demand collapse inside ten years (driver D14).
- Lower tail: no path is at or below $0. 0.5% of the paths are below $30 and 5.9% are below $40 (`--price 0.01 --price 30 --price 40`). The lowest values come from a collapse in year 1: 2.0 times the base revenue of $361.8B, discounted, is $648B. With net cash and stakes of $131B that is $32 a share. So the multiple of the collapse (2.0, no source) sets the floor and the lower tail. With a multiple of 1.0 the P10 is $35.86. With 4.0 it is $54.14.
- The terminal value is 45.5% of the enterprise value at the means and 47.7% as the mean over the paths.
- The equity value at the P50 is $2,057.5B on 24,186M shares. The enterprise value at the P50 is $1,926.6B, or 5.3 times the trailing revenue of $361.8B.
- The value at the means (every drawn number at its mean, no shock, with the expected collapse) is $94.64. A hand calculation of the same cash flows (Python, no engine, no collapse) gives $102.99. The engine gives $103.02 for the same inputs.
- Year 1 at the means: revenue $506.5B, EBIT margin 37.6% (66.0% less SBC 2.1% less charge 26.3%), reinvestment $48.2B, free cash flow to the firm $102.1B.
- Version 2 gave P50 $97.65 and version 1 gave $130.89. Section 7 shows the change step by step.

## 2. The business

NVIDIA designs processors and systems. The fiscal year ends in late January. Revenue was $215.9B in fiscal 2026 (to 2026-01-25) and $361.8B in the four quarters to 2026-10-25. The last four quarters were $68.1B, $81.6B, $96.2B and $115.8B. Each was 18% to 20% above the quarter before. Data Center was $107.6B of the last quarter and Edge Computing $8.2B (8-K, L2a). The GAAP operating margin was 66.9% in the last quarter and 15.7% in fiscal 2023 (ledger L1, L3). The company holds $53.5B of cash and marketable debt securities and $32.4B of debt (L2f). It also holds $109.7B of equity securities (L11) and had guarantees with a maximum of $108.5B on 2026-07-26 (L12). The company guides revenue of $130.0B for the next quarter, which has 14 weeks (L2d). Only the last quarter has a segment split and no history of it exists, so the model has one segment. The Q3 numbers come from the 8-K press release (unaudited), which I traced line by line. The 10-Q is not filed (L2). The press release also names a leadership transition (an interim CEO and a search) and an ongoing investigation by an independent committee of the board (L2h). The model has no input for them (weak point W16).

## 3. The drivers

Tags: DATA = a reported number or the calibration table. CONVENTION = a record of this repo. JUDGMENT = the analyst. "Stoch." = drawn one time for each path. A process has a yearly shock in addition. The reasons are in the model file.

| Id | Driver | Value | Distribution | Source and tag |
|---|---|---|---|---|
| D1 | Base revenue | $361.807B | fixed | Four quarters to 2026-10-25 (L1, L2a). DATA (Q3: 8-K, unaudited) |
| D2 | Growth of year 1 | 40%, std 12%, cut 10% to 75% | stoch. | Reported quarters (L1a) and the guide for Q4 (L2d, L22): $130.0B in 14 weeks is $120.7B in 13 weeks, +4.2% on Q3. A flat run rate gives +28%; +4.2% a quarter gives +42%; +4.2% that fades gives +39.5%; +10% a quarter gives +55%. DATA + JUDGMENT. Version 1: 65% |
| D3 | Half-life of the growth fade | 1.0 year, std 0.4, cut 0.4 to 3.0 | stoch. | The panel (0.6 to 0.7 years, L19) bounds it from below. The old upper bound (own momentum, 2.5 years) is gone: the sequential growth falls from +20% to +4% in the guide. JUDGMENT. Version 1: 1.25 |
| D4 | Long-run growth | 4.0% | fixed | A-001 (L18). CONVENTION |
| D5 | Yearly shock of growth (the cycle) | max(15%, 30% of the slow part), persistence 0, cut -50% to +200% | yearly shock | Own history, two busts in ten years (L3). JUDGMENT + DATA |
| D6 | Margin before SBC, year 1 | 66%, std 3% | stoch. | GAAP operating margin: Q3 66.9%, trailing 66.1%; guide for Q4 about 63.0% (gross margin 71% to 72%, opex $11.0B on $130.0B); SBC about 2.1% to 2.3% (L1c, L2b, L2d). DATA + guidance. Version 1: 68% |
| D7 | Long-run margin before SBC | 48.5%, std 8%, cut 25% to 65% (GAAP 45%) | stoch. | Middle of the own history: 31% (FY17 to FY22) and 59% (FY24 to FY26) (L3). JUDGMENT |
| D8 | Half-life of the margin fade | 3 years, std 1 | stoch. | JUDGMENT |
| D9 | Yearly shock of margin | 6 points, persistence 0.5, correlation with the growth shock 0.6 | yearly shock | Calibration table (persistence) and own history (L3). JUDGMENT |
| D10 | SBC (a cost, rule F5) | 2.12% in year 1 (trailing four quarters; Q3 SBC $2.088B from the page), to 3.5% (std 1%) in year 10 | stoch. target | L6, L2c. DATA + JUDGMENT. Version 2: 2.27% |
| D11 | Charge (rule F16), two parts in one process | (A) purchases of equity securities: 19.6% of revenue, divided by (1 - 21%) = 24.8% (std 4%), falling linearly to zero in year 10. (B) purchases of businesses: 1.5% (std 0.7%, cut 0 to 5%), constant. Total 26.3% in year 1, 1.5% in year 10 | stoch. | (A) Four quarters: $70.90B of purchases over revenue $361.8B (L2e). (B) Nine months of FY27: acquisitions $3.031B plus Groq $2.944B (separate financing line, no overlap) = 2.0% (L2e, L7). JUDGMENT. Version 2: 1.5% only |
| D12 | Sales to capital (reinvestment, rule F16) | 3.0 in year 1, to 4.0 (std 1.0) in year 10 | stoch. target | L4a. DATA + JUDGMENT |
| D13 | Tax rate | 21% | fixed | Statutory. Kept after the guide of 16% to 18% for FY27: the effective rate rose from 12.0% (FY24) to 13.3%, 15.1% and 17.4% (Q3); the guide is for one year and the long run is not guided (L5, L2d). CONVENTION |
| D14 | Demand collapse | hazard 2% a year (std 1%), value 2.0 times the revenue of the year before | stoch. hazard, event sampled | The engine has no regime state. JUDGMENT |
| D15 | Beta | 1.681, std 0.09, lower bound 1.0 | stoch. | Regression 2.021 shrunk; standard error 0.088 (L8). DATA. It is a DRAW; the IBM model fixes the beta as an estimation constant. This is a change to the IBM rule, made because the error of the NVDA beta is wide. Version 1: 1.676 |
| D16 | Equity risk premium | 4.41%, std 0.75%, lower bound 2% | stoch. | L17. CONVENTION |
| D17 | Risk-free rate | Hull-White on the GSW curve of 2026-11-13; 10-year zero yield 5.13% | process | L16. CONVENTION |
| D18 | Equity weight | 1.0 | fixed | The market weight is an input from the price. The debt is below 1% of any plausible value. CONVENTION |
| D19 | Terminal growth | 4.0%, std 0.75%, capped by D4 and by the risk-free rate | stoch. | A-001. CONVENTION |
| D20 | Terminal return on new capital | 35%, std 15%, cut 15% to 100% | stoch. | JUDGMENT |
| D21 | Net debt | -$130.888B (net cash and stakes): debt 32.371 less cash 29.089 less marketable debt securities 24.423 less equity securities 109.747 | fixed | Balance sheet 2026-10-25 (L2f, L10, L11). DATA. Version 2: -21.141 (stakes out of the bridge) |
| D22 | Shares | 24,186M (diluted, Q3) | fixed | L2f, L9. DATA. Version 1: 24,285M |

Cost of capital at the means: 12.55% in each year (risk-free 5.14% plus beta 1.681 times 4.41%). Year 1 starts after 2026-10-25 and the model date is 2026-11-20: the first cash flow is discounted for 339 / 365 = 0.9288 years.

**Decision on the equity securities ($109.7B, $4.54 a share) and on their purchases.** The decision is from the Head of Fundamental Research (2026-11-21). The rule is the pairing of IBM F16. If a purchase of an equity security finances a customer, the revenue that it buys is credited in the growth, so its cost must be charged. If it is an investment at a fair price, the stake keeps its value. So the model does two things. (1) The stakes are in the bridge at their carrying value: marketable $44.2B (fair value) and non-marketable $65.5B (carrying value, mostly cost). They are assets of NVDA. Their gains are not in the margin (F15). (2) The future purchases are a cost of the business. The charge is the mean of the purchases of equity securities over revenue in the last four quarters: $70.90B over $361.8B = 19.6% (the page gives $58.1B for nine months and $15.7B for Q3; Q4 FY26 comes from the company facts, $12.8B; Q3 alone is 13.5% and is not used). The purchases are not tax deductible, so the charge is divided by 0.79 (24.8%). The charge falls linearly to zero in ten years, like the charge of record A-002 along the fade of the credited growth. The 1.5% charge for acquisitions stays, separate (D11, part B). If the purchases are fair-price investments, this charge is a cost that is too high; if they are discounts for buyers, it may still be too low. Sensitivity (seed 7, 10,000 paths):

| Case | P10 | P50 | P90 | Mean |
|---|---|---|---|---|
| Version 2: stakes out of the bridge, charge 1.5% only | 51.91 | 97.65 | 186.58 | 110.86 |
| Stakes in the bridge, no charge at all | 58.22 | 105.40 | 198.21 | 119.84 |
| Stakes in the bridge, charge 1.5% only | 56.46 | 102.32 | 191.38 | 115.50 |
| Stakes in the bridge, charge on the Q3 rate (13.5%, divided by 0.79) | 47.67 | 90.40 | 181.15 | 105.53 |
| **Base: stakes in the bridge, charge on the four-quarter rate (19.6%, divided by 0.79)** | **44.27** | **85.07** | **174.19** | **100.17** |
| Base, purchases net of sales proceeds (nine months 16.6%) | 46.00 | 87.78 | 177.62 | 102.87 |
| Base, no gross-up for tax (19.6% + 1.5%) | 46.58 | 88.72 | 178.91 | 103.81 |
| Base, but the stakes out of the bridge | 39.73 | 80.53 | 169.66 | 95.63 |
| Base, acquisitions part B at 2.0% | 44.09 | 84.39 | 172.73 | 99.42 |

The Q3 case is `--pset process.charge.start=0.1865`. The cases with a changed process or bridge are copies of the model file that `COMMANDS.md` describes. The exact commands are in `COMMANDS.md`.

**Pairing and stock-or-flow rules.**
- Growth credited has its cost (F16). The model charges reinvestment for each dollar of revenue growth (D12), a part of the purchases of businesses and the purchases of equity securities (D11) and SBC (D10). The terminal value reinvests growth divided by the return on new capital (D20).
- No stock with its flow (F15). The margin is the operating margin. Interest income and gains on investments are not in the cash flow, so cash, debt securities and equity securities are in the bridge. The lease cost is in the operating income, so the lease liability is not in the bridge (L13). Purchase obligations are costs of future product, already in the gross margin (L14).
- No debt twice (F1). Interest expense is not in the cash flow. The debt of $32.4B is subtracted one time.
- SBC is a cost (F5). GAAP operating income already holds SBC. The model adds the SBC ratio to the GAAP margin and charges it as D10. The share count is the diluted count (existing awards).
- Forward disclosures (R1): section 8 and the ledger (L20 to L24).

## 4. What drives the value

Change of the P50 when the mean of one drawn number moves by one standard deviation (the attribution block of the headline command):

| Driver | Minus 1 std | Plus 1 std |
|---|---|---|
| Growth of year 1 (D2) | -12.17 | +13.55 |
| Equity risk premium (D16) | +13.12 | -9.68 |
| Long-run margin (D7) | -10.09 | +10.11 |
| Half-life of the growth fade (D3) | -8.43 | +10.75 |
| Collapse hazard (D14) | +4.17 | -4.89 |
| Beta (D15) | +3.81 | -3.29 |
| Half-life of the margin fade (D8) | -3.26 | +3.14 |
| Start of the charge (D11, part A) | +2.81 | -2.80 |
| Terminal return on new capital (D20) | -2.14 | +1.60 |
| Margin start (D6) | -2.04 | +2.03 |
| Others (D19, D10, D11 part B, D12) | each under 1.4 | |

Growth is the main source of the spread. The headline command also prints the variance by group (block "VARIANCE OF THE VALUE BY GROUP"). When the growth draws and shocks are fixed at their means, 71% of the variance is gone. The margin removes 35% and the discount rate removes 27%.

**Sensitivity to the three most important drivers** (P50, other numbers at base; `--sensitivity`):

| Growth of year 1 (D2) | 28% | **40%** | 55% |
|---|---|---|---|
| P50 | 72.90 | **85.07** | 102.32 |

| Equity risk premium (D16) | 3.5% | **4.41%** | 5.5% |
|---|---|---|---|
| P50 | 101.66 | **85.07** | 71.66 |

| Long-run margin before SBC (D7) | 35% | **48.5%** | 60% |
|---|---|---|---|
| P50 | 68.18 | **85.07** | 99.11 |

Other sensitivities (P50; each changes one number with `--pset`, or a copy of the file; see `COMMANDS.md`):

| Change | P50 | Change | P50 |
|---|---|---|---|
| Half-life 0.6 / 1.25 / 2.5 years (D3) | 76.63 / 91.65 / 127.89 | Beta 1.2 (D15) | 110.16 |
| **Tax 17% (D13; the guide is 16% to 18%)** | **89.23** | Mid-year cash flows (a copy of the file) | 89.74 |
| **Q4 revenue 4% over the guide (year-1 growth 45.6%)** | **91.26** | Margin start 64% / 69% (D6) | 83.77 / 87.09 |
| Hazard 0% / 4% (D14) | 92.31 / 76.11 | Value of collapse 1.0 / 4.0 times revenue | 83.51 / 91.63 |
| Growth shock 10% / 25% (D5) | 87.89 / 77.56 | Persistence of growth shock -0.25 / +0.25 | 84.36 / 85.07 |
| Terminal return 15% / 100% (D20) | 82.23 / 88.36 | Sales to capital 2 / 6 (D12) | 84.10 / 85.79 |
| No yearly shocks | 91.07 | Shock correlation 0.25 (a copy of the file) | 84.65 |

The guide-beat row sets the level of all four quarters of year 1 4% higher (1.40 x 1.04 - 1 = 45.6%). The beats of the EPS consensus were 8.3%, 10.0%, 6.2% and 8.5% in the last four quarters (L23). Revenue beats are not in the store. The cost of capital is high for a company of this size: a beta of 1.2 gives $110.16.

### Base rates of scale and fade (skeptic M2)

Source: `[[fade]]` of `analysis/priors/process_priors_2026-11-19.toml` (companies with growth of 50% or more in the start year; cache of 2026-11-20), and the facts of the cache (`mapcheck`, all 10-K rows, 1,976 companies with us-gaap facts). The cells have survivor bias: the cache holds the companies that exist now, so the cells are too high, not too low.

| Item | Cell above $100B revenue | Cell $30B to $100B | This model at the means |
|---|---|---|---|
| Starts (companies with growth of 50% or more) | 14 | 42 | |
| Median growth, year 1 (n) | +44.7% (12) | +12.5% (39) | +40.0% |
| Median growth, year 2 (n) | -13.3% (11) | -0.5% (38) | +22.0% |
| Median growth, year 3 (n) | -5.5% (11) | +3.3% (36) | +13.0% |
| Revenue after three years, from $361.8B | $429B | $418B | $698B |
| Largest revenue in the cache | AMZN $716.9B (year to 2025-12-31); WMT $713.2B (year to 2026-01-31) | | Year 10: $1,000B at the means; P50 over all paths $804B |
| Largest GAAP operating profit in the cache | AAPL $158.1B (year to 2026-09-26); MSFT $155.2B (2026-06-30); NVDA $130.4B (2026-01-25) | | Year 10 at the means: $472B (revenue $1,000B x (50.7% - 3.5%)) |
| GAAP operating margin of company-years above $100B revenue | MU 74.6% (FY26), NVDA 62.4% and 60.4%, MSFT 41.6% to 46.8%, META 34.7% to 42.2%, GOOGL up to 32.1%, AAPL up to 33.0% | | Year 10: 47.2% (D7 target 45%) |
| A bust in the cache | MU: revenue $30.8B (FY22), $15.5B (FY23), -49.5%; operating margin -37% in FY23; $133.2B in FY26 | | Collapse hazard 2% a year; shock 15% |

Why D3 and D7 sit above the base rate. (a) The cells are small (11 company-years above $100B) and they hold busts. The model has busts too, in 18.5% of the paths (collapse) and in the shocks, but the median path has none. (b) NVDA reports +17.9% to +20.4% a quarter for the last five quarters, and the guide is +4.2% in 13 weeks, not a fall. (c) The base rate says growth falls to zero or below in years 2 and 3; the model has +22% and +13%. This is the largest difference with the base rate. A model that follows the cell has a half-life of 0.4 years (the cell phi of 0.161 gives 0.38; the cut of the draw is 0.4): the P50 is $73.89. With 0.6 years (the panel of all sizes) it is $76.63. The base case with 1.0 year is $85.07. So D3 is above the base rate by $11 to $13 of P50. This is the price of reading the guide and the sequential growth as evidence. D7 (45% GAAP) is at the level of the best large company of the cache with a long history (MSFT); only MU (one year) and NVDA are higher. The year-10 numbers of the model are far above anything in the cache: revenue 1.4 times AMZN, operating profit 3.0 times AAPL. The cache has no company with such numbers, so the model has no base rate for them. The P50 of year-10 revenue ($804B) is 1.12 times AMZN.

## 5. What the price implies

The last close in the store is **$145.98 on 2026-11-19** (L15). The close of 2026-11-20 is not in the store (the source lags one day). The price is a measurement here. No input comes from it. I made this section after the model was complete.

- The model P50 ($85.07) is 41.7% below the close. The model mean ($100.17) is 31.4% below it. The close is between the P75 ($122.98) and the P90 ($174.19). 83.8% of the paths are below the close.
- In version 1 the close ($150.96 on 2026-11-18) was inside the P25 to P75 range. In version 2 the close was between the P75 and the P90. In version 3 it is still between them, but the P50 is lower by $12.6, because the stakes are in the bridge (+$4.5) and the purchases of equity securities are charged (-$17.2).
- The model says that, on its inputs, the share is worth less than the close in most of the paths. This is a statement about the inputs, not a finding that the price is wrong. The largest open difference is in the growth of the quarters after Q4: the guide gives +4% on Q3 in 13 weeks, and the consensus EPS (L23, not an input) needs a growth far above it.
- One number alone makes the P50 equal to $145.98, with all other numbers at base (`--price 145.98 --implied-price 145.98 --implied-param NAME --range LO,HI`; for the rows marked `*` with `--shift-bounds`, so that the cut of the draw moves with the mean):

| Number | Base | Value for P50 = $145.98 |
|---|---|---|
| Growth of year 1 (D2) * | 40% | 85.1% (range 0.15 to 1.5) |
| Equity risk premium (D16) * | 4.41% | 2.13% |
| Beta (D15) * | 1.681 | 0.82 |
| Half-life of the growth fade (D3) | 1.0 year | no solution up to 2.9 years (P50 $127.89 at 2.5) |
| Long-run margin before SBC (D7) * | 48.5% (GAAP 45%) | no solution up to 78% |

In words: the close needs a first-year growth of 85% against a guide of +4% on Q3, or a premium near 2% (a cost of capital about 4 points lower), or a beta below 1. A margin or a slower fade alone cannot do it. The table is one number at a time: a lower number in one place can be paid by a higher number in another place.

The close was higher before the 8-K of 2026-11-17: $228.38 on 2026-09-30 and $206.69 on 2026-11-03 (L15). The model has no fact that explains the fall. It does not use the fall.

## 6. Comparison with the automatic screen

`fa dcf auto NVDA --cache data/raw --as-of 2026-11-20 --explain` (seed 7, 10,000 paths) gives P10 1.18, P25 5.40, **P50 16.55**, P75 43.26, P90 104.57 and mean 43.85. The screen uses the 10-K and the 10-Q of Q2 only; it does not see the 8-K. It is not a reviewed valuation (`docs/models/auto-path-model.md`). This model gives P50 $85.07. The two differ for the reasons below. I did not decompose the gap one input at a time.

| Input | Automatic screen | This model |
|---|---|---|
| Base revenue | $215.9B (fiscal 2026; the four quarters are not used) | $361.8B (four quarters to 2026-10-25) |
| Revenue of year 1 | $257.8B (+19.4%, std 63%) | $506.5B at the means (+40%) |
| Check against reports | The last reported quarter ($115.8B) times four is $463B | |
| Margin at the start | 31.9% (cash flow before SBC) | 66% operating margin before SBC; free cash flow to the firm 20.2% of revenue in year 1 (after the charge of 26.3%) |
| Margin, long run | 19.4% (shrunk toward 7.9%, the mean of all companies above $100B, weight 0.44) | 48.5% operating margin before SBC; free cash flow to the firm 35.1% of revenue in year 10 |
| Reported history | | Free cash flow 41.1% of revenue in the last four quarters (OCF 158.9 less capex 10.1, over 361.8; L2c) |
| SBC | 4.14% (mean of three years) | 2.12% (trailing four quarters), to 3.5% |
| Net debt | +$10.9B (cash only, $22.4B, 2026-07-26) | -$130.9B (cash, debt securities and equity securities, 2026-10-25) |
| Purchases of equity securities | not charged | charged (19.6% of revenue, to zero in 10 years) |
| Collapse of demand | No | Yes (D14) |
| Beta, risk premium, rates, L | 1.681, 4.41%, GSW, 4.0% | the same |

The screen shrinks the history of NVDA toward the mean of all large companies. For NVDA that mean is far from the facts. The free cash flow margin was above 14% in each of the last ten fiscal years and above 40% in the last three. The screen's year-1 revenue is below the revenue that NVDA has already reported. So the screen does not fit this company. The ticket gave the same cause (`analysis/priors/dcf_investigation_2026-11-20.md`, section 4.8). This model does not use the shrink.

## 7. How the model was built (order of the runs)

**Version 1** (data of 2026-11-19, before the 8-K). I wrote the inputs and the reasons before the first run.
1. First run, without the collapse table (D14): P10 73.84, P50 145.95, P90 327.08, mean 181.30.
2. A check of the growth process alone (Python, 20,000 paths, no engine): a year of revenue growth below -30% occurs on 5.1% of the paths, and a drawdown of about 35% from the peak is at the 90th percentile. The ticket asks for a downside of the cycle. So I added D14.
3. Run with D14: P10 59.06, P50 130.89, P90 298.97, mean 162.33.

**Version 2** (data of 2026-11-21, with the 8-K). The base run is the version 1 file with no change: the same text, P50 $130.89. Each row is one more change on top of the rows above:

| Step | Change | P10 | P50 | P90 | Mean |
|---|---|---|---|---|---|
| 0 | Base run (version 1 file) | 59.06 | 130.89 | 298.97 | 162.33 |
| 1 | Base revenue $361.8B, growth start 40% (std 12%), year 1 after 2026-10-25, model date 2026-11-20 | 54.59 | 107.20 | 211.54 | 123.32 |
| 2 | Margin start 66%, SBC start 2.27% | 53.74 | 105.82 | 209.23 | 121.91 |
| 3 | Bridge of 2026-10-25 (net debt -21.141), shares 24,186M, beta 1.681 | 53.82 | 105.93 | 209.43 | 122.03 |
| 4 | Half-life 1.0 year (std 0.4) | 52.35 | 98.69 | 188.47 | 111.94 |
| 5 | Charge 1.5% (std 0.7%, cut at 5%) = version 2 | 51.91 | 97.65 | 186.58 | 110.86 |

Step 1 is the largest change (-$23.7): the guide for Q4 shows +4.2% on Q3 in 13 weeks, against +20% in each of the last four quarters, and the base is one quarter later. Step 4 follows from the guide (the old upper bound of the half-life is void). No step used the price.

**Version 3** (answer to the skeptic, after the page was read). Each row is one more change on top of version 2:

| Step | Change | P10 | P50 | P90 | Mean |
|---|---|---|---|---|---|
| 6 | Version 2 (the file of commit 41afd53) | 51.91 | 97.65 | 186.58 | 110.86 |
| 7 | SBC start 2.12% (Q3 SBC from the page; trailing four quarters) | 51.99 | 97.76 | 186.70 | 110.96 |
| 8 | Equity securities ($109.7B) in the bridge (decision of the Head of Fundamental Research) | 56.53 | 102.29 | 191.23 | 115.50 |
| 9 | Purchases of equity securities charged (19.6% / 0.79, to zero in 10 years) = **version 3** | 44.27 | 85.07 | 174.19 | 100.17 |

The decision on the stakes and the charge is not mine: it is from the Head of Fundamental Research. Step 9 lowers the P50 by $17.2 and step 8 raises it by $4.5. The s2c start (3.0) was checked against the balance sheet of the page and not changed.

## 8. Forward disclosures (rule R1)

| Disclosure | Decision | Reason |
|---|---|---|
| Remaining performance obligation, $3.2B on 2026-07-26 (L20) | Excluded | About 1% of the trailing revenue. It is not for product orders. |
| Purchase obligations, $22.7B at 2026-01-25 (L21) | Plausibility check only | They show supply bought, not orders received. |
| Outlook for Q4 FY27 (L2d, L22): revenue $130.0B +-3% in 14 weeks, gross margin 71% to 72%, opex about $11.0B, tax 16% to 18%, no China data center compute | **Used** | Evidence for D2 (first quarter of year 1, +4.2% on Q3 in 13 weeks), D3 (the speed of the slow-down) and D6 (margin of the guided quarter about 63.0% GAAP). The tax guide is not used for D13: the guide is for one year; the effective rate rose from 12.0% (FY24) to 13.3%, 15.1% and 17.4%; at 17% the P50 is $89.23. The guide is one quarter: the other three quarters of year 1 are in the std of D2. The past of the company against its guides is not in the store. The sensitivity "4% over the guide" is above. |
| Consensus EPS and targets (L23, L24) | Cross-check only | Not an input. Consensus EPS for the four quarters after Q3 is +70% over the last four reported quarters, and the model has revenue +40% in year 1 with a lower margin. This is the largest difference with the street. |

## 9. Weak points that I found

W1. **The growth start is the largest driver and rests on one guided quarter.** The mean of 40% follows the guide of +4.2% on Q3 in 13 weeks. The quarters after Q4 have no evidence in the store. NVDA beat the EPS consensus by 6% to 10% in each of the last four quarters (L23), so a low guide is possible: I could not measure it. At a start of 45.6% (Q4 4% over the guide) the P50 is $91.26; at 55% it is $102.32. The street EPS needs much higher growth.
W2. **The long-run margin (D7) and the half-lives (D3, D8) are judgments.** D3 and D7 are above the base rate of scale (section 4). The half-life moved from 1.25 to 1.0 in version 2 on the evidence of the guide.
W3. **The cost of capital is 12.55% for ten years and in the terminal value.** The beta of 1.68 comes from five years of weekly returns. A company with $1T of revenue can have a lower beta. A beta of 1.2 gives $110.16. I kept the repo method. The beta is a draw here and fixed in the IBM model.
W4. **The cycle is a wide shock plus a table that the engine made for failure (D14).** The engine has no regime. The hazard (2%) and the value of the collapse (2.0 times revenue) have no data of the repo except MU (one bust in the cache). The multiple sets the lower tail: no path is below $0, 0.5% are below $30. The mean-path table of the output counts failed paths as zero revenue. Do not read it as a forecast.
W5. **The model has one segment.** The 8-K gives Data Center ($107.6B) and Edge Computing ($8.2B) for Q3 only, with growth rates and no history, and no customer concentration.
W6. **The charge for purchases of equity securities (D11 part A) is one of the largest costs of the model: $133B in year 1 (26.3% of revenue).** It is a decision of the Head of Fundamental Research, with the pairing rule F16 as reason. It lowers the P50 by $17. If the purchases are fair-price investments, the charge is too high (stakes in the bridge with no charge: P50 $105.40). If they are discounts for buyers, the effect may be larger and the stakes may be worth less than the carrying value. The terms are in the 10-Q notes, not read. The guarantees ($108.5B on 2026-07-26, up to -$4.49 a share) are not in the bridge.
W7. **Two mapping gaps in `mapcheck`.** (a) The standard variable `short_term_investments` has no value for NVDA after 2025-10-26, because the filer uses the tag `DebtSecuritiesCurrent`. `mapcheck values NVDA` shows cash of $22.4B only. The screen net debt is wrong by $34.1B ($1.41 a share) and it does not see the 8-K. (b) The tag `PaymentsToAcquireBusinessTwoNetOfCashAcquired` ($13.0B in fiscal 2026) has no standard variable, and the line "Groq, Inc." ($2.944B) is in the financing section. There is no standard variable for purchases of equity securities. I typed the numbers from the page and the company facts. The orchestrator decides.
W8. **Tax at 21% is above the effective rate (17.4% in Q3, guide 16% to 18% for FY27).** The P50 is $89.23 at 17%. The engine has no draw for the tax rate.
W9. **The terminal return on new capital (D20) is a judgment.** The P50 is $82.23 at 15% and $88.36 at 100%.
W10. **The timing is simple.** The balance sheet is of 2026-10-25 and the model date is 2026-11-20. Year 1 holds all cash flows after the balance sheet, discounted from the end of year 1 (0.93 years). Cash paid to holders since 2026-10-25 is not removed. The share count is the weighted diluted count of Q3 (24,186M). The period-end count is lower after the buyback of $22.1B in the quarter (under 1%), so the count is conservative. The extra week of Q4 is removed (13 of 14 weeks). Mid-year timing gives P50 $89.74.
W11. **The right tail is long.** The P90 of revenue in year 10 is $1.7T in the mean-path table, and the year-10 numbers are far above the cache (section 4).
W12. **The panel that bounds D3 has survivor bias.** It is a bound only.
W13. **The growth-margin shock correlation (0.6) rests on two events.** With 0.25 the P50 is $84.65.
W14. **The 10-Q is not filed.** The Q3 numbers come from the 8-K (unaudited). I read the page and traced each number (ledger A2). I did not read the 10-K text or the notes.
W15. **The result is not stable.** One filing moved the P50 by $33 (version 1 to version 2), and the decision on the stakes moved it by $12.6 (version 2 to version 3). A reader must not take the P50 as a stable number before the 10-Q and the next guide.
W16. **Governance items are outside the model.** The page names a leadership transition (interim CEO, search for a new CEO) and an ongoing investigation by an independent committee of the board (ledger L2h). It gives no detail. The model has no input for them. They are a risk of the discount rate or of the collapse hazard that I cannot size.

## 10. Open items and remaining steps

1. The Head of Technology adds the 10-Q of the fiscal third quarter of 2027 and the close of 2026-11-20 to the store. Then check each row of section A2 of the ledger against the 10-Q, read the guarantee note, the terms of the equity securities and the investigation, and find the past guides to measure the beat.
2. Orchestrator: decide W7 (new tags in `standard_variables.toml`). See the next section.
3. Orchestrator: write `docs/models/` and `docs/assumptions/` records for D3, D7, D11, D14 and D20. I did not write them.
4. The ticket acceptance names `analysis/NVDA/driver_model.toml` (old engine form). I wrote the path-engine form only.
5. Skeptic (second run), reviewer, auditor, then the approval of the owner.

## 11. What the automatic model can learn

Each line is a judgment of this model that could be a rule of `fa dcf auto`. Ticket `T-20261121-dcf-engine-from-tickers` has the first list (items 1 to 7 below are its items). I agree or not, with the evidence. The gate for each rule is the model-change flow (a test, a record, a measured effect).

1. **Base revenue = trailing four quarters** (ticket 1). Agree. Evidence: the screen has year-1 revenue $257.8B, below the $361.8B already reported. Needs a point-in-time rule (quarters filed on or before the model date) and a fiscal-calendar rule: the Q4 of NVDA has 14 weeks, so the length of each period in days must be an input (candidate 8).
2. **Margin shrink by size and history** (ticket 2). Agree with a limit. Evidence: the screen gives 19.4% against a reported 41.1% (above 40% for three years). But MU shows a margin of 74.6% in one year after -37% three years earlier, so the weight must grow with the number of years above the cell mean, not with the level alone. A test on the point-in-time panel is needed.
3. **Base rates of scale** (ticket 3). Agree as a check, not as a cap. Evidence: the cell above $100B has 11 company-years and holds busts; the model's year 2 and year 3 growth is far above it. Use the cells above $30B pooled, and print the largest revenue and profit of the panel next to year 10 (here 1.4 times and 3.0 times). A warning flag is enough.
4. **Purchases of equity securities as a cost, paired with the stakes in the bridge** (ticket 4 and 6). Agree. Evidence: $70.9B in four quarters, 19.6% of revenue; the charge moves the P50 by -$17 and the stakes by +$4.5. The two must switch together: stakes in the bridge only when the purchases are charged. Needs standard variables for purchases and proceeds of equity securities (a new tag set) and the balance sheet lines. For most companies the value is near zero, so the rule is safe.
5. **The second acquisition line** (ticket 5). Agree. Evidence: $13.0B in fiscal 2026 under a second tag is not in `mapcheck values`. A limit: the "Groq, Inc." line is in the financing section under a company label. A rule cannot find it. Say so in the output (a warning when the acquisition spend of the year is far above its mean).
6. **Cash-like securities in the bridge** (ticket 6). Agree. Evidence: `DebtSecuritiesCurrent` hides $34.1B ($1.41 a share) in the screen. Equity stakes are a decision: see line 4.
7. **Company guidance as evidence for year 1** (ticket 7). Agree as a study, not yet as a rule. Evidence: one guide moved the P50 by $23 here. But it is one observation, and the beat history is not in the store (EPS beats of 6% to 10% only). First step: the 8-K text as data, then the beat study.
8. **Period length in days.** New. Evidence: Q4 FY27 has 14 weeks ($130.0B is $120.7B in 13 weeks). The screen has no input for the length of the period.
9. **SBC ratio from the trailing four quarters when growth is high.** New. Evidence: the screen uses 4.14% (mean of three years); the trailing ratio is 2.12% and the last quarter 1.80%.
10. **A collapse regime for companies with a bust in their history.** New, and a study first. Evidence: MU, revenue -49.5% in FY23. The hazard and the multiple here have no source; the multiple sets the lower tail (floor $32). The panel can measure the share of company-years with revenue -30% or worse after a year of growth of 50% or more. Without that measure the rule is not ready.
11. **Equity weight from the price.** The screen takes the equity weight from the price (0.991). This model fixes it at 1.0 so that no input comes from the price. The effect is small (under 0.1 point of cost of capital), but the rule "no input from the price" is cleaner. Disagree with the screen here; low priority.
12. **A beta that is a draw.** This model draws the beta with its standard error (0.088); the screen and IBM fix it. Evidence: a beta of 1.2 moves the P50 by $25. A draw widens the bands by little. Not a priority, but a test of the band coverage of the point-in-time test would show it.

## 12. Commands

The file `analysis/NVDA/COMMANDS.md` has the command for each number of this report, in the order of the report, with the seed, the number of paths and the data store commit. Run from the repository root with the release build (`fa` is `rust/target/release/fa`, and the cache is the data store, `--cache data/raw` for `fa dcf auto`).
