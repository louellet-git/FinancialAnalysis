# NVDA: what is the share worth? Report draft

**Status: draft, not reviewed.** This text is step 1 of 4 of the review (analyst). The skeptic, the reviewer and the auditor have not seen it. Do not use it alone for a trade. The values are model values, not market prices.

Version 4 of 2026-11-21 (answer to the skeptic, round 2; version 3 is commit 6eac285, version 2 is commit 41afd53, version 1 is commit 9a2e6f0). Model date 2026-11-20. Data store `1696af8` (the NVDA facts and prices are those of `a1dffb5`): SEC facts of 2026-11-20, NVDA prices to 2026-11-19, the 8-K of 2026-11-17 (fiscal Q3 FY27, quarter end 2026-10-25). The analyst read the 8-K page and traced each number (ledger section A2). The 10-Q of that quarter is not filed. Files: `analysis/NVDA/driver_model_paths.toml` (the model), `analysis/NVDA/research-ledger.md` (the outside facts, ids L1 to L26, with L2a to L2i for the 8-K page) and `analysis/NVDA/COMMANDS.md` (the command for each number).

## 1. Result

| Measure | Value for each share (USD) |
|---|---|
| P10 | 55.49 |
| P25 | 76.35 |
| **P50 (headline)** | **108.40** |
| P75 | 152.40 |
| P90 | 208.21 |
| Mean | 123.85 (standard error 0.70) |

Command: `fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml` (10,000 paths, seed 7). Two runs give the same text. **The headline rests on r = 0.5, a choice with no data** (see below). The base year-1 growth is 47.9% (the guide times the measured median beat; decision of the Head of Fundamental Research, round 3). With the guide alone (40%) the P50 is $97.74 (P10 50.95, P90 185.24, mean 111.09); section 4 has both.

Monte Carlo noise. Seed 7 with 10,000 paths is at the high end. With 100,000 paths: seed 7 gives P10 55.76, P50 107.03, P90 208.71 and mean 123.08 (SE 0.19); seed 12 gives P10 55.24, P50 106.79, P90 207.71 and mean 122.44. Seed 11 with 10,000 paths gives P50 107.12. So the P50 is about $106.8 to $108.4 and the noise is about $1.4.

**The range across r (one number for the past and the future).** A purchase of an equity security is partly an investment that keeps its value (the share r) and partly a subsidy to buyers that buys credited revenue (the share 1 - r). The stakes held are in the bridge at r x carrying value ($109.7B). The future purchases are charged at (1 - r) x the four-quarter rate (19.6% of revenue, divided by 0.79 for tax). The charge fades with the growth (same half-life as D3). r = 1 is the "fair price" frame. r = 0 is the "financing" frame. Seed 7, 10,000 paths:

| r | Net debt ($B) | Start of the charge | P10 | P50 | P90 | Mean |
|---|---|---|---|---|---|---|
| 0 (stakes out, full charge) | -21.1 | 26.3% | 50.16 | 101.99 | 201.17 | 117.42 |
| 0.25 | -48.6 | 20.1% | 52.87 | 105.17 | 204.62 | 120.63 |
| **0.5 (base)** | **-76.0** | **13.9%** | **55.49** | **108.40** | **208.21** | **123.85** |
| 0.75 | -103.5 | 7.7% | 58.14 | 111.53 | 212.07 | 127.06 |
| 1 (stakes in, no equity charge) | -130.9 | 1.5% | 60.64 | 114.10 | 220.05 | 130.56 |

The P50 moves by $12.1 over the whole range of r. The headline rests on r = 0.5, a choice with no data.

Other facts of the run:
- 18.5% of the paths have a demand collapse inside ten years (driver D14).
- Lower tail. No path is at or below $0. 1.1% of the paths are below $30 and 2.9% are below $40 (`--price 0.01 --price 30 --price 40`). The lowest values come from a collapse in year 1: 2.0 times the base revenue of $361.8B, discounted, is $648B. With net cash and stakes of $76B that is $30 a share. So the multiple of the collapse (2.0, no source) sets the floor and the lower tail. The P10 is $41.95, $47.75, $51.93, $55.49 and $66.40 for a multiple of 0.5, 1.0, 1.5, 2.0 and 4.0. A multiple of 2.0 assumes a milder bust than MU (revenue -49.5%, operating margin -37% in FY23; ledger L3 and the cache).
- **Bias of the collapse paths.** The engine subtracts the net debt on each path and cannot impair the bridge on a failed path. So a collapse path keeps the stakes at r x carrying value ($2.27 a share at r = 0.5, up to $4.54 at r = 1), and it does not carry the guarantee claim (up to $4.49 a share on the maximum of $108.5B; L12). The P10 is biased upward by up to about $2.3 (up to $4.5 at r = 1) for the first reason; the guarantees are not in the model at all.
- The terminal value is 40.1% of the enterprise value at the means and 42.1% as the mean over the paths.
- The equity value at the P50 is $2,621.9B on 24,186M shares. The enterprise value at the P50 is $2,545.9B, or 7.0 times the trailing revenue of $361.8B.
- The value at the means (every drawn number at its mean, no shock, with the expected collapse) is $117.54. A hand calculation of the same cash flows (Python, no engine, no collapse) gives $128.32. The engine gives $128.34 for the same inputs.
- Year 1 at the means: revenue $535.1B, EBIT margin 50.0% (66.0% less SBC 2.1% less charge 13.9%), reinvestment $57.8B, free cash flow to the firm $153.5B.
- Version 4 with the guide alone gave P50 $97.74, version 3 gave $85.07, version 2 gave $97.65 and version 1 gave $130.89. Section 7 shows the change step by step.

## 2. The business

NVIDIA designs processors and systems. The fiscal year ends in late January. Revenue was $215.9B in fiscal 2026 (to 2026-01-25) and $361.8B in the four quarters to 2026-10-25. The last four quarters were $68.1B, $81.6B, $96.2B and $115.8B. Each was 18% to 20% above the quarter before. Data Center was $107.6B of the last quarter and Edge Computing $8.2B (8-K, L2a). The GAAP operating margin was 66.9% in the last quarter and 15.7% in fiscal 2023 (ledger L1, L3). The company holds $53.5B of cash and marketable debt securities and $32.4B of debt (L2f). It also holds $109.7B of equity securities (L11) and had guarantees with a maximum of $108.5B on 2026-07-26 (L12). The company guides revenue of $130.0B for the next quarter, which has 14 weeks (L2d). Only the last quarter has a segment split and no history of it exists, so the model has one segment. The Q3 numbers come from the 8-K press release (unaudited), which I traced line by line. The 10-Q is not filed (L2). The press release also names a leadership transition (an interim CEO and a search) and an ongoing investigation by an independent committee of the board (L2h). The model has no input for them (weak point W16).

## 3. The drivers

Tags: DATA = a reported number or the calibration table. CONVENTION = a record of this repo. JUDGMENT = the analyst. "Stoch." = drawn one time for each path. A process has a yearly shock in addition. The reasons are in the model file.

| Id | Driver | Value | Distribution | Source and tag |
|---|---|---|---|---|
| D1 | Base revenue | $361.807B | fixed | Four quarters to 2026-10-25 (L1, L2a). DATA (Q3: 8-K, unaudited) |
| D2 | Growth of year 1 | **47.9%**, std 12%, cut 10% to 75% | stoch. | The guide for Q4 times the measured median beat (decision of the Head of Fundamental Research, round 3). Guide (L2d, L22): $130.0B in 14 weeks is $120.7B in 13 weeks, +4.2% on Q3; at face value year 1 is +40% (a flat run rate +28%, +10% a quarter +55%). Own record (L26, 12 of 12 quarters above the guide, median +5.65%, range +2.5% to +10.5%): 1.40 x 1.0565 - 1 = 47.9%; the range of the beats gives 43.5% to 54.7%, inside one std. A base at the guide alone would be low on the same evidence. DATA + JUDGMENT. Version 1: 65%; version 4 with the guide alone: 40% |
| D3 | Half-life of the growth fade | 1.0 year, std 0.4, cut 0.4 to 3.0 | stoch. | The panel (0.6 to 0.7 years, L19) bounds it from below. The old upper bound (own momentum, 2.5 years) is gone: the sequential growth falls from +20% to +4% in the guide. JUDGMENT. Version 1: 1.25 |
| D4 | Long-run growth | 4.0% | fixed | A-001 (L18). CONVENTION |
| D5 | Yearly shock of growth (the cycle) | max(15%, 30% of the slow part), persistence 0, cut -50% to +200% | yearly shock | Own history, two busts in ten years (L3). JUDGMENT + DATA |
| D6 | Margin before SBC, year 1 | 66%, std 3% | stoch. | GAAP operating margin: Q3 66.9%, trailing 66.1%; guide for Q4 about 63.0% (gross margin 71% to 72%, opex $11.0B on $130.0B); SBC about 2.1% to 2.3% (L1c, L2b, L2d). DATA + guidance. Version 1: 68% |
| D7 | Long-run margin before SBC | 48.5%, std 8%, cut 25% to 65% (GAAP 45%) | stoch. | Middle of the own history: 31% (FY17 to FY22) and 59% (FY24 to FY26) (L3). JUDGMENT |
| D8 | Half-life of the margin fade | 3 years, std 1 | stoch. | JUDGMENT |
| D9 | Yearly shock of margin | 6 points, persistence 0.5, correlation with the growth shock 0.6 | yearly shock | Calibration table (persistence) and own history (L3). JUDGMENT |
| D10 | SBC (a cost, rule F5) | 2.12% in year 1 (trailing four quarters; Q3 SBC $2.088B from the page), to 3.5% (std 1%) in year 10 | stoch. target | L6, L2c. DATA + JUDGMENT. Version 2: 2.27% |
| D11 | Charge (rule F16), two parts in one process | (A) purchases of equity securities: (1 - r) x 19.6% of revenue / 0.79 = 12.4% at r = 0.5 (std 2%). (B) purchases of businesses: 1.5% (std 0.7%, cut 0 to 5%). Total 13.9% in year 1. It falls to the target (B) with the same half-life as D3 (mean 1.0 year, correlation 0.9 with D3) | stoch. | (A) Four quarters: $70.90B of purchases over revenue $361.8B (L2e). r is a decision with no data (section 1). (B) Nine months of FY27: acquisitions $3.031B plus Groq $2.944B (separate financing line, no overlap) = 2.0% (L2e, L7). JUDGMENT. Version 3: a ramp of 10 years from 26.3% |
| D12 | Sales to capital (reinvestment, rule F16) | 3.0 in year 1, to 4.0 (std 1.0) in year 10 | stoch. target | L4a. DATA + JUDGMENT |
| D13 | Tax rate | 21% | fixed | Statutory. Kept after the guide of 16% to 18% for FY27: the effective rate rose from 12.0% (FY24) to 13.3%, 15.1% and 17.4% (Q3); the guide is for one year and the long run is not guided (L5, L2d). CONVENTION |
| D14 | Demand collapse | hazard 2% a year (std 1%), value 2.0 times the revenue of the year before | stoch. hazard, event sampled | The engine has no regime state. JUDGMENT |
| D15 | Beta | 1.681, std 0.09, lower bound 1.0 | stoch. | Regression 2.021 shrunk; standard error 0.088 (L8). DATA. It is a DRAW; the IBM model fixes the beta as an estimation constant. This is a change to the IBM rule, made because the error of the NVDA beta is wide. Version 1: 1.676 |
| D16 | Equity risk premium | 4.41%, std 0.75%, lower bound 2% | stoch. | L17. CONVENTION |
| D17 | Risk-free rate | Hull-White on the GSW curve of 2026-11-13; 10-year zero yield 5.13% | process | L16. CONVENTION |
| D18 | Equity weight | 1.0 | fixed | The market weight is an input from the price. The debt is below 1% of any plausible value. CONVENTION |
| D19 | Terminal growth | 4.0%, std 0.75%, capped by D4 and by the risk-free rate | stoch. | A-001. CONVENTION |
| D20 | Terminal return on new capital | 35%, std 15%, cut 15% to 100% | stoch. | JUDGMENT |
| D21 | Net debt | -$76.015B (net cash and stakes at r = 0.5): debt 32.371 less cash 29.089 less marketable debt securities 24.423 less 0.5 x equity securities 109.747 | fixed | Balance sheet 2026-10-25 (L2f, L10, L11). DATA + decision on r. Version 2: -21.141; version 3: -130.888 |
| D22 | Shares | 24,186M (diluted, Q3) | fixed | L2f, L9. DATA. Version 1: 24,285M |

Cost of capital at the means: 12.55% in each year (risk-free 5.14% plus beta 1.681 times 4.41%). Year 1 starts after 2026-10-25 and the model date is 2026-11-20: the first cash flow is discounted for 339 / 365 = 0.9288 years.

**Decision on the equity securities ($109.7B) and on their purchases (Head of Fundamental Research, 2026-11-21, round 2).** The rule is the pairing of IBM F16 with ONE frame and ONE number r. A purchase is partly an investment that keeps its value (the share r) and partly a subsidy to buyers that buys credited revenue (the share 1 - r). The same r applies to the past and to the future. The stakes held are in the bridge at r x carrying value (marketable $44.2B at fair value, non-marketable $65.5B at the carrying value, mostly cost). The future purchases are charged at (1 - r) x the four-quarter rate: ($17.502B - $4.702B + $58.100B) / $361.8B = 19.6%, divided by 0.79 because the purchases are not tax deductible (the skeptic accepted this). Q3 alone (13.5%) is not used. The base r is 0.5: there are no terms and no data for r, and 0.5 says that we do not know. The range of r (0 to 1) carries the uncertainty (section 1). The charge fades with the growth that it pays for (record A-002): it has the same half-life as D3, not a ramp of 10 years. The 1.5% charge for acquisitions stays, separate (the target of the fade). The engine formula is target + (start - target) x 0.5^(t-1) for a half-life of 1.0 year: year 1 = start, year 2 = half of the excess, year 3 = a quarter. The mean of years 1 to 3 is 58% of the start. The panel cross-check (study 2.1, `analysis/priors/dcf_conventions_study_2026-11-20.md`): the spend of the next 3 years is 0.55 of the past for a spend above 5% of revenue (mean; 0.24 median; 0.39 for all by regression). So 0.58 is close to 0.55. It is a cross-check, not the rule.

Fade of the charge and the panel rows (r = 0.5; seed 7, 10,000 paths; each row is a copy of the model file, see `COMMANDS.md`):

| Case | P10 | P50 | P90 | Mean |
|---|---|---|---|---|
| **Base: ou, half-life 1.0 year (same as D3)** | **55.49** | **108.40** | **208.21** | **123.85** |
| ou, half-life 2.0 years | 53.94 | 105.55 | 203.72 | 120.85 |
| Ramp, 3 years | 55.25 | 107.96 | 215.91 | 125.45 |
| Ramp, 4 years | 54.54 | 106.91 | 214.60 | 124.41 |
| Ramp, 5 years | 53.88 | 105.84 | 213.17 | 123.39 |
| Ramp, 10 years (version 3 shape) | 51.61 | 101.24 | 206.45 | 118.84 |
| Panel row: start at 0.39 of the past (6.3%), ou 1.0 | 57.36 | 110.90 | 211.63 | 126.38 |
| Panel row: start at 0.55 of the past (8.3%), ou 1.0 | 56.85 | 110.19 | 210.73 | 125.72 |
| Panel row: start at 0.24 of the past (4.5%), ou 1.0 | 57.81 | 111.48 | 212.41 | 126.99 |
| Q3 rate (13.5%) in place of 19.6%, ou 1.0 | 56.42 | 109.60 | 209.98 | 125.13 |
| Purchases net of sales proceeds (nine months 16.6%), ou 1.0 | 55.94 | 108.99 | 209.21 | 124.50 |
| No tax gross-up, ou 1.0 | 56.09 | 109.20 | 209.58 | 124.72 |
| Acquisitions part B at 2.0%, ou 1.0 | 55.17 | 107.45 | 206.55 | 122.79 |

The panel rows set the start of the charge at 0.39, 0.55 and 0.24 times the past charged rate (12.4% at r = 0.5), plus the 1.5% of part B; then the ou fade of 1.0 year goes on. The panel factors are for the mean of years 1 to 3, so the start of these rows is a stricter reading than the panel. The slower the fade, the lower the P50: the ramp of 10 years costs $7.2 against the base.

**Pairing and stock-or-flow rules.**
- Growth credited has its cost (F16). The model charges reinvestment for each dollar of revenue growth (D12), a part of the purchases of businesses and the purchases of equity securities (D11) and SBC (D10). The terminal value reinvests growth divided by the return on new capital (D20).
- No stock with its flow (F15). The margin is the operating margin. Interest income and gains on investments are not in the cash flow, so cash, debt securities and the equity securities (at r = 0.5 of the carrying value) are in the bridge. The lease cost is in the operating income, so the lease liability is not in the bridge (L13). Purchase obligations are costs of future product, already in the gross margin (L14).
- No debt twice (F1). Interest expense is not in the cash flow. The debt of $32.4B is subtracted one time.
- SBC is a cost (F5). GAAP operating income already holds SBC. The model adds the SBC ratio to the GAAP margin and charges it as D10. The share count is the diluted count (existing awards).
- Forward disclosures (R1): section 8 and the ledger (L20 to L24).

## 4. What drives the value

Change of the P50 when the mean of one drawn number moves by one standard deviation (the attribution block of the headline command):

| Driver | Minus 1 std | Plus 1 std |
|---|---|---|
| Growth of year 1 (D2) | -15.84 | +17.32 |
| Half-life of the growth fade (D3) | -12.78 | +16.78 |
| Equity risk premium (D16) | +15.65 | -12.25 |
| Long-run margin (D7) | -12.16 | +11.34 |
| Collapse hazard (D14) | +5.25 | -6.06 |
| Beta (D15) | +4.72 | -4.29 |
| Half-life of the margin fade (D8) | -3.61 | +3.21 |
| Terminal return on new capital (D20) | -2.74 | +1.69 |
| Margin start (D6) | -2.27 | +2.21 |
| Others (D19, D10, D11 target, half-life and start, D12) | each under 1.6 | |

Growth is the main source of the spread. The headline command also prints the variance by group (block "VARIANCE OF THE VALUE BY GROUP"). When the growth draws and shocks are fixed at their means, 70% of the variance is gone. The margin removes 32% and the discount rate removes 26%.

**Sensitivity to the three most important drivers** (P50, other numbers at base; `--sensitivity`):

| Growth of year 1 (D2) | 30% | 40% (guide alone) | 43.5% | 45.6% | 47% | **47.9% (base)** | 50% | 54.7% | 55% |
|---|---|---|---|---|---|---|---|---|---|
| P50 | 85.50 | 97.74 | 102.40 | 105.04 | 107.07 | **108.40** | 111.38 | 118.16 | 118.65 |

| Equity risk premium (D16) | 3.5% | **4.41%** | 5.5% |
|---|---|---|---|
| P50 | 128.03 | **108.40** | 91.70 |

| Long-run margin before SBC (D7) | 35% | **48.5%** | 60% |
|---|---|---|---|
| P50 | 88.41 | **108.40** | 124.03 |

**Guide against result (ledger L26) and the base of D2.** The 8-K press releases of 13 quarters are in the store (`data/raw/NVDA/filings/8k`). In each of the 12 quarters with a guide and a result, the revenue was above the guide: median +5.65%, mean +6.12%, range +2.5% to +10.5%; the last 8 quarters have a median of +4.85%. **The base case uses the guide times the median beat** (decision of the Head of Fundamental Research, round 3): the company's own record is the best evidence for the revenue of Q4, and a base at the guide alone would be a base that the same evidence calls low. The beat is applied to the level of year 1 (a beat that holds in the next quarters, because each guide starts from the last result): year-1 growth = 1.40 x (1 + beat) - 1. The std of 12% covers the range of the beats and the three later quarters. Each row is one run (`--sensitivity process.growth.start`):

| Beat of the guide | None (the guide alone) | +2.5% (minimum) | +4.0% (round 1 guess) | **+5.65% (median) = base** | +10.5% (maximum) |
|---|---|---|---|---|---|
| Year-1 growth | 40.0% | 43.5% | 45.6% | **47.9%** | 54.7% |
| P50 | 97.74 | 102.40 | 105.04 | **108.40** | 118.16 |

The measured median beat adds $10.7 to the P50 against the guide alone. Limits: 12 observations of one company in a rising market; the guide of Q4 has a wider band (3%, all earlier guides 2%) and an extra week; no earlier guide had a 14-week quarter. The reader can take the P50 as $97.74 at the guide alone and $108.40 at the median beat.

**Year 2 against the guide pace (skeptic MIN 1).** At the means, year 2 is +26.0%. The guide pace (a constant +3.5% to +4.2% a quarter in 13-week quarters) gives about +15% to +18% for year 2. So D3 is above the guide pace for year 2. A half-life of 0.55 years gives year 2 at +16.4% (4% + 43.9% x 0.5^(1/0.55)); the P50 is $94.43 (the half-life of the charge stays at 1.0 year). The P50 is $95.62 at 0.6 years and $91.31 at 0.4 years.

Other sensitivities (P50; each changes one number with `--pset`, or a copy of the file; see `COMMANDS.md`):

| Change | P50 | Change | P50 |
|---|---|---|---|
| Half-life 0.4 / 1.25 / 2.5 years (D3) | 91.31 / 118.48 / 176.91 | Beta 1.2 (D15) | 138.32 |
| **Tax 17% (D13; the guide is 16% to 18%)** | **113.87** | Mid-year cash flows (a copy of the file) | 114.57 |
| Hazard 0% / 4% (D14) | 116.93 / 97.40 | Value of collapse 0.5 / 1.0 / 1.5 / 4.0 times revenue | 105.68 / 106.45 / 107.19 / 114.94 |
| Growth shock 10% / 25% (D5) | 111.96 / 99.21 | Persistence of growth shock -0.25 / +0.25 | 108.08 / 108.19 |
| Terminal return 15% / 100% (D20) | 105.04 / 112.05 | Sales to capital 2 / 6 (D12) | 107.37 / 109.25 |
| Margin start 64% / 69% (D6) | 106.90 / 110.61 | No yearly shocks | 113.91 |
| Shock correlation 0.25 (a copy of the file) | 108.04 | Acquisitions part B at 2.0% | 107.45 |

The cost of capital is high for a company of this size: a beta of 1.2 gives $138.32.

### Base rates of scale and fade (skeptic M2)

Source: `[[fade]]` of `analysis/priors/process_priors_2026-11-19.toml` (companies with growth of 50% or more in the start year; cache of 2026-11-20), and the facts of the cache (`mapcheck`, all 10-K rows, 1,976 companies with us-gaap facts). The cells have survivor bias: the cache holds the companies that exist now, so the cells are too high, not too low.

**Composition of the cell above $100B.** Its 12 start years with revenue above $100B and growth of 50% or more (10-K rows of the cache) are: oil in 2021 and 2022 (XOM 2021; CVX 2021 and 2022; MPC 2021; PSX 2021 and 2022; VLO 2021 and 2022: 8 starts), CI in 2019 (a merger: +216%), MU in FY26 (+256%), and NVDA itself in FY25 and FY26. So the cell is not a base rate of organic demand: two thirds of it is the oil price of 2021-22, which fell back in 2023. It is a base rate of a price-driven jump, and the fall of years 2 and 3 is the end of the oil price.

| Item | Cell above $100B revenue | Cell $30B to $100B | This model at the means |
|---|---|---|---|
| Starts (companies with growth of 50% or more) | 14 | 42 | |
| Median growth, year 1 (n) | +44.7% (12) | +12.5% (39) | +47.9% |
| Median growth, year 2 (n) | -13.3% (11) | -0.5% (38) | +26.0% |
| Median growth, year 3 (n) | -5.5% (11) | +3.3% (36) | +15.0% |
| Revenue after three years, from $361.8B | $429B | $418B | $775B |
| Largest revenue in the cache | AMZN $716.9B (year to 2025-12-31); WMT $713.2B (year to 2026-01-31) | | Year 10: $1,130B at the means; P50 over all paths $906B |
| Largest GAAP operating profit in the cache | AAPL $158.1B (year to 2026-09-26); MSFT $155.2B (2026-06-30); NVDA $130.4B (2026-01-25) | | Year 10 at the means: $533B (revenue $1,130B x (50.7% - 3.5%)) |
| GAAP operating margin of company-years above $100B revenue | MU 74.6% (FY26), NVDA 62.4% and 60.4%, MSFT 41.6% to 46.8%, META 34.7% to 42.2%, GOOGL up to 32.1%, AAPL up to 33.0% | | Year 10: 47.2% (D7 target 45%) |
| A bust in the cache | MU: revenue $30.8B (FY22), $15.5B (FY23), -49.5%; operating margin -37% in FY23; $133.2B in FY26 | | Collapse hazard 2% a year; shock 15% |

Why D3 and D7 sit above the base rate. (a) The cells are small (11 company-years above $100B), they are mostly oil, and they hold busts. The model has busts too, in 18.5% of the paths (collapse) and in the shocks, but the median path has none. (b) NVDA reports +17.9% to +20.4% a quarter for the last five quarters, and the guide is +4.2% in 13 weeks, not a fall; the guide was beaten in 12 of 12 quarters (L26). (c) The base rate says growth falls to zero or below in years 2 and 3; the model has +26% and +15%. This is the largest difference with the base rate. A model that follows the cell has a half-life of 0.4 years (the cell phi of 0.161 gives 0.38; the cut of the draw is 0.4): the P50 is $91.31. With 0.6 years (the panel of all sizes) it is $95.62. The base case with 1.0 year is $108.40. So D3 is above the base rate by $13 to $17 of P50 (P50 $95.62 at 0.6 years and $91.31 at 0.4 years, against $108.40). This is the price of reading the guide and the sequential growth as evidence. D7 (45% GAAP) is at the level of the best large company of the cache with a long history (MSFT); only MU (one year) and NVDA are higher. The year-10 numbers of the model are far above anything in the cache: revenue 1.6 times AMZN, operating profit 3.4 times AAPL. The cache has no company with such numbers, so the model has no base rate for them. The P50 of year-10 revenue ($906B) is 1.26 times AMZN.

## 5. What the price implies

The last close in the store is **$145.98 on 2026-11-19** (L15). The close of 2026-11-20 is not in the store (the source lags one day). The price is a measurement here. No input comes from it. I made this section after the model was complete.

- The model P50 ($108.40) is 25.7% below the close. The model mean ($123.85) is 15.2% below it. The close is between the P50 and the P75 ($152.40). 72.0% of the paths are below the close. Across r (0 to 1) the P50 is $101.99 to $114.10. With the guide alone (year-1 growth 40%) the P50 is $97.74, 33.0% below the close.
- In version 1 the close ($150.96 on 2026-11-18) was inside the P25 to P75 range. In versions 2, 3 and 4 with the guide alone it was between the P75 and the P90. With the beat in the base it is again below the P75.
- The model says that, on its inputs, the share is worth less than the close in most of the paths. This is a statement about the inputs, not a finding that the price is wrong. The largest open difference is in the growth of the quarters after Q4: the guide gives +4% on Q3 in 13 weeks (and has been beaten by 5.65% on the median; the base now includes the beat), and the consensus EPS (L23, not an input) needs a growth far above it.
- One number alone makes the P50 equal to $145.98, with all other numbers at base (`--price 145.98 --implied-price 145.98 --implied-param NAME --range LO,HI`; for the rows marked `*` with `--shift-bounds`, so that the cut of the draw moves with the mean):

| Number | Base | Value for P50 = $145.98 |
|---|---|---|
| Growth of year 1 (D2) * | 47.9% | 71.6% (range 0.15 to 1.5) |
| Half-life of the growth fade (D3) | 1.0 year | 1.85 years |
| Equity risk premium (D16) * | 4.41% | 2.91% |
| Beta (D15) * | 1.681 | 1.12 |
| Long-run margin before SBC (D7) * | 48.5% (GAAP 45%) | 74.1% (GAAP 70.6%) |

In words: the close needs one of these, one at a time: a first-year growth of 72% (the base is 48%, the maximum beat gives 55%), or a growth fade 1.85 times slower, or a premium near 2.9% or a beta near 1.1 (a cost of capital about 2 points lower), or a long-run GAAP margin of 70.6% (the highest in the cache is MU's 74.6% in one year). The table is one number at a time: a lower number in one place can be paid by a higher number in another place.

NVDA fell from $206.69 (2026-11-03) to $155.46 (2026-11-17), that is -24.8% in the two weeks before the release (the release came after the close of 2026-11-17), and from $155.46 to $145.98 (-6.1%) in the two sessions after it ($150.96 on 2026-11-18, $145.98 on 2026-11-19). It was $228.38 on 2026-09-30 (L15). The model has no fact that explains the fall before the release. The release has a result 7.3% above the guide and a guide of +4.2% on Q3 in 13 weeks: the second fact may explain the days after it. I do not use the fall.

## 6. Comparison with the automatic screen

`fa dcf auto NVDA --cache data/raw --as-of 2026-11-20 --explain` (seed 7, 10,000 paths) gives P10 1.18, P25 5.40, **P50 16.55**, P75 43.26, P90 104.57 and mean 43.85 (store `a1dffb5`; at `c486b86` and later the beta is 1.675 and the P50 16.57). The screen uses the 10-K and the 10-Q of Q2 only; it does not see the 8-K. It is not a reviewed valuation (`docs/models/auto-path-model.md`). This model gives P50 $108.40. The two differ for the reasons below. I did not decompose the gap one input at a time.

| Input | Automatic screen | This model |
|---|---|---|
| Base revenue | $215.9B (fiscal 2026; the four quarters are not used) | $361.8B (four quarters to 2026-10-25) |
| Revenue of year 1 | $257.8B (+19.4%, std 63%) | $535.1B at the means (+47.9%) |
| Check against reports | The last reported quarter ($115.8B) times four is $463B | |
| Margin at the start | 31.9% (cash flow before SBC) | 66% operating margin before SBC; free cash flow to the firm 28.7% of revenue in year 1 (after the charge of 13.9%) |
| Margin, long run | 19.4% (shrunk toward 7.9%, the mean of all companies above $100B, weight 0.44) | 48.5% operating margin before SBC; free cash flow to the firm 35.1% of revenue in year 10 |
| Reported history | | Free cash flow 41.1% of revenue in the last four quarters (OCF 158.9 less capex 10.1, over 361.8; L2c) |
| SBC | 4.14% (mean of three years) | 2.12% (trailing four quarters), to 3.5% |
| Net debt | +$10.9B (cash only, $22.4B, 2026-07-26) | -$76.0B (cash, debt securities and 0.5 of the equity securities, 2026-10-25) |
| Purchases of equity securities | not charged | charged at 0.5 x 19.6% / 0.79, falling with the growth |
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

**Version 3** (round 1; the page was read):

| Step | Change | P10 | P50 | P90 | Mean |
|---|---|---|---|---|---|
| 7 | SBC start 2.12% (Q3 SBC from the page; trailing four quarters) | 51.99 | 97.76 | 186.70 | 110.96 |
| 8 | Equity securities ($109.7B) in the bridge at full value | 56.53 | 102.29 | 191.23 | 115.50 |
| 9 | Purchases of equity securities charged (19.6% / 0.79, a ramp of 10 years) = **version 3** | 44.27 | 85.07 | 174.19 | 100.17 |

**Version 4** (round 2; new decision of the Head of Fundamental Research). Each row is one more change on top of the rows above:

| Step | Change | P10 | P50 | P90 | Mean |
|---|---|---|---|---|---|
| 10 | One r = 0.5: stakes in the bridge at 0.5 of the carrying value, charge at 0.5 of the rate (start 13.9%), ramp of 10 years | 47.71 | 91.57 | 183.23 | 106.58 |
| 11 | The charge fades with the growth (ou, same half-life as D3, correlation 0.9) = **version 4 with the guide alone** | 50.95 | 97.74 | 185.24 | 111.09 |
| 12 | Year-1 growth 47.9% = the guide times the measured median beat (decision of the Head of Fundamental Research, round 3) = **version 4** | 55.49 | 108.40 | 208.21 | 123.85 |

Step 10 raises the P50 by $6.5 (a smaller charge and fewer stakes), step 11 by $6.2 (a faster fade) and step 12 by $10.7 (the beat of the guide). The rate in the version 3 and 4 steps comes from the page and the company facts; no step used the price. The decision on r and on the fade is not mine: it is from the Head of Fundamental Research. The s2c start (3.0) was checked against the balance sheet of the page and not changed. The correlation 0.95 between the two half-lives is not possible (the matrix is not positive definite with the other correlations); I used 0.9.

## 8. Forward disclosures (rule R1)

| Disclosure | Decision | Reason |
|---|---|---|
| Remaining performance obligation, $3.2B on 2026-07-26 (L20) | Excluded | About 1% of the trailing revenue. It is not for product orders. |
| Purchase obligations, $22.7B at 2026-01-25 (L21) | Plausibility check only | They show supply bought, not orders received. |
| Outlook for Q4 FY27 (L2d, L22): revenue $130.0B +-3% in 14 weeks, gross margin 71% to 72%, opex about $11.0B, tax 16% to 18%, no China data center compute | **Used** | Evidence for D2 (first quarter of year 1, +4.2% on Q3 in 13 weeks), D3 (the speed of the slow-down) and D6 (margin of the guided quarter about 63.0% GAAP). The tax guide is not used for D13: the guide is for one year; the effective rate rose from 12.0% (FY24) to 13.3%, 15.1% and 17.4%; at 17% the P50 is $113.87. The guide is one quarter: the other three quarters of year 1 are in the std of D2. The past of the guides is measured (L26): 12 of 12 above the guide, median +5.65%. The base uses the guide times the median beat (47.9%); the guide alone (40%) is a sensitivity (section 4). |
| Consensus EPS and targets (L23, L24) | Cross-check only | Not an input. Consensus EPS for the four quarters after Q3 is +70% over the last four reported quarters, and the model has revenue +47.9% in year 1 with a lower margin. This is the largest difference with the street. |

## 9. Weak points that I found

W1. **The growth start is the largest driver and rests on one guided quarter and the record of 12 beats.** The base is the guide times the median beat (47.9%). The beat is a record of 12 quarters of one company in a rising market, and the guide of Q4 has a wider band (3%) and 14 weeks. At the guide alone (40%) the P50 is $97.74; at the minimum beat (43.5%) $102.40; at the maximum beat (54.7%) $118.16. The quarters after Q4 have no evidence in the store except the beat record.
W2. **The long-run margin (D7) and the half-lives (D3, D8) are judgments.** D3 and D7 are above the base rate of scale (section 4), and the cell above $100B is mostly oil. Year 2 at the means (+26.0%) is above the guide pace (+15% to +18%); the P50 at the guide pace for year 2 is $94.43.
W3. **The cost of capital is 12.55% for ten years and in the terminal value.** The beta of 1.68 comes from five years of weekly returns. A company with $1T of revenue can have a lower beta. A beta of 1.2 gives $138.32. I kept the repo method. The beta is a draw here and fixed in the IBM model.
W4. **The cycle is a wide shock plus a table that the engine made for failure (D14).** The engine has no regime. The hazard (2%) and the value of the collapse (2.0 times revenue) have no data of the repo except MU (one bust in the cache, revenue -49.5% and margin -37%; a multiple of 2.0 assumes a milder bust). The multiple sets the lower tail: no path is below $0, 1.1% are below $30. The mean-path table of the output counts failed paths as zero revenue. Do not read it as a forecast.
W5. **The model has one segment.** The 8-K gives Data Center ($107.6B) and Edge Computing ($8.2B) for Q3 only, with growth rates and no history, and no customer concentration.
W6. **r = 0.5 has no data.** The treatment of the equity securities and of their purchases rests on one number r with no terms (section 1). The P50 goes from $101.99 (r = 0) to $114.10 (r = 1). The stakes held ($109.7B) and the purchases ($70.9B in four quarters) are large. The terms are in the 10-Q notes, not read. **The engine cannot impair the bridge on a failed path:** a collapse path keeps the stakes at r x carrying value ($2.27 a share) and carries no guarantee claim (up to $4.49 a share; $108.5B on 2026-07-26). The P10 is biased up by up to about $2.3.
W7. **Two mapping gaps in `mapcheck`.** (a) The standard variable `short_term_investments` has no value for NVDA after 2025-10-26, because the filer uses the tag `DebtSecuritiesCurrent`. `mapcheck values NVDA` shows cash of $22.4B only. The screen net debt is wrong by $34.1B ($1.41 a share) and it does not see the 8-K. (b) The tag `PaymentsToAcquireBusinessTwoNetOfCashAcquired` ($13.0B in fiscal 2026) has no standard variable, and the line "Groq, Inc." ($2.944B) is in the financing section. There is no standard variable for purchases of equity securities. I typed the numbers from the page and the company facts. The orchestrator decides.
W8. **Tax at 21% is above the effective rate (17.4% in Q3, guide 16% to 18% for FY27).** The P50 is $113.87 at 17%. The engine has no draw for the tax rate.
W9. **The terminal return on new capital (D20) is a judgment.** The P50 is $105.04 at 15% and $112.05 at 100%.
W10. **The timing is simple.** The balance sheet is of 2026-10-25 and the model date is 2026-11-20. Year 1 holds all cash flows after the balance sheet, discounted from the end of year 1 (0.93 years). Cash paid to holders since 2026-10-25 is not removed. The share count is the weighted diluted count of Q3 (24,186M). The period-end count is lower after the buyback of $22.1B in the quarter (under 1%), so the count is conservative. The extra week of Q4 is removed (13 of 14 weeks). Mid-year timing gives P50 $114.57.
W11. **The right tail is long.** The P90 of revenue in year 10 is $1.6T in the mean-path table, and the year-10 numbers are far above the cache (section 4).
W12. **The panel that bounds D3 has survivor bias.** It is a bound only.
W13. **The growth-margin shock correlation (0.6) rests on two events.** With 0.25 the P50 is $108.04.
W14. **The 10-Q is not filed.** The Q3 numbers come from the 8-K (unaudited). I read the page and traced each number (ledger A2; two inferences are marked). Convertible debt and accrued purchase consideration (footnote B of the page) are not in net debt and are open for the 10-Q (L2i). I did not read the 10-K text or the notes.
W15. **The result is not stable.** One filing moved the P50 by $33 (version 1 to version 2), the decisions on the stakes by -$12.6 and then +$12.7 (versions 3 and 4), and the beat of the guide by +$10.7. A reader must not take the P50 as a stable number before the 10-Q and the next guide.
W16. **Governance items are outside the model.** The page names a leadership transition (interim CEO, search for a new CEO) and an ongoing investigation by an independent committee of the board (ledger L2h). It gives no detail. The model has no input for them. They are a risk of the discount rate or of the collapse hazard that I cannot size.
W17. **The correlation of the charge half-life with D3 is 0.9, not 1.** The engine cannot share one number between two processes, and 0.95 is not positive definite with the other correlations.

## 10. Open items and remaining steps

1. The Head of Technology adds the 10-Q of the fiscal third quarter of 2027 and the close of 2026-11-20 to the store. Then check each row of section A2 of the ledger against the 10-Q, read the guarantee note, the terms of the equity securities (to inform r), the convertible debt and the accrued purchase consideration, and the investigation. Add the pair Q4 FY27 to the guide table (L26).
2. Orchestrator: decide W7 (new tags in `standard_variables.toml`). See the next section. Decide on an engine feature to impair the bridge on a failed path (W6).
3. Orchestrator: write `docs/models/` and `docs/assumptions/` records for D3, D7, D11, D14 and D20. I did not write them.
4. The ticket acceptance names `analysis/NVDA/driver_model.toml` (old engine form). I wrote the path-engine form only.
5. Skeptic (third run), reviewer, auditor, then the approval of the owner.

## 11. What the automatic model can learn

Each line is a judgment of this model that could be a rule of `fa dcf auto`. Ticket `T-20261121-dcf-engine-from-tickers` has the first list (items 1 to 7 below are its items). I agree or not, with the evidence. The gate for each rule is the model-change flow (a test, a record, a measured effect).

1. **Base revenue = trailing four quarters** (ticket 1). Agree. Evidence: the screen has year-1 revenue $257.8B, below the $361.8B already reported. Needs a point-in-time rule (quarters filed on or before the model date) and a fiscal-calendar rule: the Q4 of NVDA has 14 weeks, so the length of each period in days must be an input (candidate 8).
2. **Margin shrink by size and history** (ticket 2). Agree with a limit. Evidence: the screen gives 19.4% against a reported 41.1% (above 40% for three years). But MU shows a margin of 74.6% in one year after -37% three years earlier, so the weight must grow with the number of years above the cell mean, not with the level alone. A test on the point-in-time panel is needed.
3. **Base rates of scale** (ticket 3). Agree as a check, not as a cap. Evidence: the cell above $100B has 11 company-years, two thirds of them oil in 2021-22, and holds busts; the model's year 2 and year 3 growth is far above it. Use the cells above $30B pooled, and print the largest revenue and profit of the panel next to year 10 (here 1.4 times and 3.0 times). A warning flag is enough. A rule must also print the composition of the cell.
4. **Purchases of equity securities, one number r** (tickets 4 and 6). Agree, with the new frame. Evidence: $70.9B in four quarters, 19.6% of revenue. The rule is one r for the stakes in the bridge and for the charge, so that no choice is half a frame (the version 3 choice was); r = 0.5 as the default when there are no terms, with the range shown. The effect is $11 of P50 over r = 0 to 1. Needs standard variables for purchases and proceeds of equity securities and for the balance sheet lines (a new tag set). For most companies the value is near zero, so the rule is safe.
5. **The second acquisition line** (ticket 5). Agree. Evidence: $13.0B in fiscal 2026 under a second tag is not in `mapcheck values`. A limit: the "Groq, Inc." line is in the financing section under a company label. A rule cannot find it. Say so in the output (a warning when the acquisition spend of the year is far above its mean).
6. **Cash-like securities in the bridge** (ticket 6). Agree. Evidence: `DebtSecuritiesCurrent` hides $34.1B ($1.41 a share) in the screen. Equity stakes: see line 4.
7. **Company guidance as evidence for year 1** (ticket 7). Agree, and the first step is done here. Evidence: the 8-K press releases of 13 quarters are in the store, and the guide was beaten in 12 of 12 quarters (median +5.65%, +2.5% to +10.5%). A rule can read the sentence "Revenue is expected to be $X billion, plus or minus Y%" (a shell pipeline does it, `COMMANDS.md` section 10) and use the beat history of the company as the correction; the effect here is +$10.7 of P50, and the base now uses it (decision of the Head, round 3). Limit: one company, 12 quarters, a rising market. A study over all companies with an outlook sentence is needed before a rule.
8. **Period length in days.** New. Evidence: Q4 FY27 has 14 weeks ($130.0B is $120.7B in 13 weeks). The screen has no input for the length of the period.
9. **SBC ratio from the trailing four quarters when growth is high.** New. Evidence: the screen uses 4.14% (mean of three years); the trailing ratio is 2.12% and the last quarter 1.80%.
10. **A collapse regime for companies with a bust in their history, and an engine that impairs the bridge on a failed path.** New, and a study first. Evidence: MU, revenue -49.5% in FY23. The hazard and the multiple here have no source; the multiple sets the lower tail (floor $30). The engine subtracts the net debt on each path, so a collapse keeps the stakes and ignores the guarantees. The panel can measure the share of company-years with revenue -30% or worse after a year of growth of 50% or more. Without that measure the rule is not ready.
11. **The charge fades with the growth** (record A-002). Agree with the decision of the Head. Evidence: the ou fade with the half-life of the growth has a mean of 58% in years 1 to 3, close to the panel factor 0.55 (study 2.1); a ramp of 10 years is $7.2 of P50 lower. The engine cannot share one half-life between two processes: a candidate feature (a process that takes the half-life of another).
12. **Equity weight from the price.** The screen takes the equity weight from the price (0.991). This model fixes it at 1.0 so that no input comes from the price. The effect is small (under 0.1 point of cost of capital), but the rule "no input from the price" is cleaner. Disagree with the screen here; low priority.
13. **A beta that is a draw.** This model draws the beta with its standard error (0.088); the screen and IBM fix it. Evidence: a beta of 1.2 moves the P50 by $30. A draw widens the bands by little. Not a priority, but a test of the band coverage of the point-in-time test would show it.

## 12. Commands

The file `analysis/NVDA/COMMANDS.md` has the command for each number of this report, in the order of the report, with the seed, the number of paths and the data store commit. Run from the repository root with the release build (`fa` is `rust/target/release/fa`, and the cache is the data store, `--cache data/raw` for `fa dcf auto`).
