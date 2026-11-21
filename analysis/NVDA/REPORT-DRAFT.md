# NVDA: what is the share worth? Report draft

**Status: draft, not reviewed.** This text is step 1 of 4 of the review (analyst). The skeptic, the reviewer and the auditor have not seen it. Do not use it alone for a trade. The values are model values, not market prices.

Version 2 of 2026-11-21 (version 1 is commit 9a2e6f0, with the base of 2026-07-26). Model date 2026-11-20. Data store `a1dffb5`: SEC facts of 2026-11-20, NVDA prices to 2026-11-19, the 8-K of 2026-11-17 (fiscal Q3 FY27, quarter end 2026-10-25). The 10-Q of that quarter is not filed. Files: `analysis/NVDA/driver_model_paths.toml` (the model) and `analysis/NVDA/research-ledger.md` (the outside facts, ids L1 to L24).

## 1. Result

| Measure | Value for each share (USD) |
|---|---|
| P10 | 51.91 |
| P25 | 70.05 |
| **P50 (headline)** | **97.65** |
| P75 | 136.20 |
| P90 | 186.58 |
| Mean | 110.86 (standard error 0.60) |

Command: `fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml` (10,000 paths, seed 7). Two runs give the same text. Seed 11 gives P50 98.68. Seeds 7 and 12 with 100,000 paths give P50 98.13 and 97.88. The noise of the P50 is about $1. The mean is higher than the P50 because the right tail is long. Use the P50 as the headline.

- 18.3% of the paths have a demand collapse inside ten years (driver D14).
- The terminal value is 38.2% of the enterprise value at the means and 39.7% as the mean over the paths.
- The equity value at the P50 is $2,361.8B on 24,186M shares. The enterprise value at the P50 is $2,340.6B, or 6.5 times the trailing revenue of $361.8B.
- The value at the means (every drawn number at its mean, no shock, with the expected collapse) is $107.20. A hand calculation of the same cash flows (Python, no engine, no collapse) gives $116.79. The engine gives $116.82 for the same inputs.
- Year 1 at the means: revenue $506.5B, EBIT margin 62.2%, tax 21%, reinvestment $48.2B, free cash flow to the firm $200.8B.
- Version 1 gave P50 $130.89 (P10 59.06, P90 298.97, mean 162.33). Section 7 shows the change step by step.

## 2. The business

NVIDIA designs processors and systems. The fiscal year ends in late January. Revenue was $215.9B in fiscal 2026 (to 2026-01-25) and $361.8B in the four quarters to 2026-10-25. The last four quarters were $68.1B, $81.6B, $96.2B and $115.8B. Each was 18% to 20% above the quarter before. Data Center was $107.6B of the last quarter and Edge Computing $8.2B (8-K, L2a). The GAAP operating margin was 66.9% in the last quarter and 15.7% in fiscal 2023 (ledger L1, L3). The company holds $53.5B of cash and marketable debt securities and $32.4B of debt (L2f). It also holds $109.7B of equity securities (L11) and had guarantees with a maximum of $108.5B on 2026-07-26 (L12). The company guides revenue of $130.0B for the next quarter, which has 14 weeks (L2d). Only the last quarter has a segment split and no history of it exists, so the model has one segment. The Q3 numbers come from the 8-K press release (unaudited). The 10-Q is not filed (L2).

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
| D10 | SBC (a cost, rule F5) | 2.27% in year 1 (three quarters; Q3 waits for the 10-Q), to 3.5% (std 1%) in year 10 | stoch. target | L6. DATA + JUDGMENT |
| D11 | Purchases of businesses (charge, rule F16) | 1.5% of revenue, std 0.7%, cut 0 to 5% | stoch. | Nine months of FY27: $3.0B to $6.0B (1.0% to 2.0% of revenue; the Groq line may overlap) (L2e, L7). JUDGMENT. Version 1: 1.0%. Purchases of equity securities are not in it (section 3, bridge) |
| D12 | Sales to capital (reinvestment, rule F16) | 3.0 in year 1, to 4.0 (std 1.0) in year 10 | stoch. target | L4a. DATA + JUDGMENT |
| D13 | Tax rate | 21% | fixed | Statutory. Kept after the guide of 16% to 18% for FY27: the effective rate rose from 12.0% (FY24) to 13.3%, 15.1% and 17.4% (Q3); the guide is for one year and the long run is not guided (L5, L2d). CONVENTION |
| D14 | Demand collapse | hazard 2% a year (std 1%), value 2.0 times the revenue of the year before | stoch. hazard, event sampled | The engine has no regime state. JUDGMENT |
| D15 | Beta | 1.681, std 0.09, lower bound 1.0 | stoch. | Regression 2.021 shrunk; standard error 0.088 (L8). DATA. Version 1: 1.676 |
| D16 | Equity risk premium | 4.41%, std 0.75%, lower bound 2% | stoch. | L17. CONVENTION |
| D17 | Risk-free rate | Hull-White on the GSW curve of 2026-11-13; 10-year zero yield 5.13% | process | L16. CONVENTION |
| D18 | Equity weight | 1.0 | fixed | The market weight is an input from the price. The debt is below 1% of any plausible value. CONVENTION |
| D19 | Terminal growth | 4.0%, std 0.75%, capped by D4 and by the risk-free rate | stoch. | A-001. CONVENTION |
| D20 | Terminal return on new capital | 35%, std 15%, cut 15% to 100% | stoch. | JUDGMENT |
| D21 | Net debt | -$21.141B (net cash): debt 32.371 less cash 29.089 less marketable debt securities 24.423 | fixed | Balance sheet 2026-10-25 (L2f, L10). DATA. Version 1: -23.220 |
| D22 | Shares | 24,186M (diluted, Q3) | fixed | L2f, L9. DATA. Version 1: 24,285M |

Cost of capital at the means: 12.55% in each year (risk-free 5.14% plus beta 1.681 times 4.41%). Year 1 starts after 2026-10-25 and the model date is 2026-11-20: the first cash flow is discounted for 339 / 365 = 0.9288 years.

**Decision on the equity securities ($109.7B, $4.54 a share).** The marketable ($44.2B) and the non-marketable ($65.5B) equity securities are NOT in the bridge. The rule F15 would allow them, because their gains are not in the margin. I exclude them for three reasons. (a) The purchases are large: $15.7B in Q3 alone, 13.5% of revenue. They can finance buyers of the product, which is a price discount in another form, and the model has no data on the terms. (b) Part of the stakes can be in companies whose orders are already in the revenue that the model credits, so the stake and the revenue are one bet that the model would count two times. (c) The notes of the 10-Q are not read. The model treats the purchases like this: the past purchases are paid and the stakes have value zero in the model; future purchases are not charged, except in the 1.5% of D11. If the stakes are in the bridge at book value, the P50 is $102.19 (+$4.54). If a charge of 5% of revenue is added for future purchases, the P50 is $91.01. The base case is between: it is neither the best nor the worst case for this item. It stays a weak point (W6).

**Pairing and stock-or-flow rules.**
- Growth credited has its cost (F16). The model charges reinvestment for each dollar of revenue growth (D12), a part of the purchases of businesses (D11) and SBC (D10). The terminal value reinvests growth divided by the return on new capital (D20).
- No stock with its flow (F15). The margin is the operating margin. Interest income and gains on investments are not in the cash flow, so cash and debt securities are in the bridge. The equity stakes are not in the bridge (L11). The lease cost is in the operating income, so the lease liability is not in the bridge (L13). Purchase obligations are costs of future product, already in the gross margin (L14).
- No debt twice (F1). Interest expense is not in the cash flow. The debt of $32.4B is subtracted one time.
- SBC is a cost (F5). GAAP operating income already holds SBC. The model adds the SBC ratio to the GAAP margin and charges it as D10. The share count is the diluted count (existing awards).
- Forward disclosures (R1): section 8 and the ledger (L20 to L24).

## 4. What drives the value

Change of the P50 when the mean of one drawn number moves by one standard deviation:

| Driver | Minus 1 std | Plus 1 std |
|---|---|---|
| Growth of year 1 (D2) | -14.39 | +16.56 |
| Equity risk premium (D16) | +13.81 | -10.61 |
| Half-life of the growth fade (D3) | -9.52 | +12.34 |
| Long-run margin (D7) | -10.39 | +10.36 |
| Collapse hazard (D14) | +5.10 | -4.55 |
| Beta (D15) | +3.89 | -3.58 |
| Half-life of the margin fade (D8) | -3.34 | +3.09 |
| Terminal return on new capital (D20) | -2.27 | +1.68 |
| Others (D6, D19, D10, D11, D12) | each under 2.1 | |

Growth is the main source of the spread. When the growth draws and shocks are fixed at their means, 68% of the variance is gone. The margin removes 31% and the discount rate removes 22%.

**Sensitivity to the three most important drivers** (P50, other numbers at base):

| Growth of year 1 (D2) | 28% | **40%** | 55% |
|---|---|---|---|
| P50 | 83.26 | **97.65** | 118.47 |

| Equity risk premium (D16) | 3.5% | **4.41%** | 5.5% |
|---|---|---|---|
| P50 | 115.00 | **97.65** | 83.06 |

| Half-life of the growth fade, years (D3) | 0.6 | **1.0** | 1.25 | 2.5 |
|---|---|---|---|---|
| P50 | 88.13 | **97.65** | 105.25 | 145.29 |

Other sensitivities (P50; each changes one number with `--pset`):

| Change | P50 | Change | P50 |
|---|---|---|---|
| Long-run margin 35% / 60% (D7) | 80.21 / 112.05 | Beta 1.2 (D15) | 124.17 |
| Tax 17% (D13) | 102.69 | Mid-year cash flows | 103.46 |
| Hazard 0% / 4% (D14) | 106.33 / 88.26 | Value of collapse 1.0 / 4.0 times revenue | 95.90 / 103.27 |
| Growth shock 10% / 25% (D5) | 100.33 / 89.60 | Margin start 64% / 69% (D6) | 96.30 / 99.71 |
| Terminal return 15% / 100% (D20) | 94.75 / 100.86 | Charge 0% / 5% (D11) | 100.14 / 91.01 |
| Sales to capital 2 / 6 (D12) | 96.82 / 98.36 | Equity securities in the bridge (+$4.54) | 102.19 |
| No yearly shocks | 104.05 | Shock correlation 0.25 (a copy of the file) | 97.24 |

The cost of capital is high for a company of this size. A beta of 1.2 gives $124.17.

## 5. What the price implies

The last close in the store is **$145.98 on 2026-11-19** (L15). The close of 2026-11-20 is not in the store (the source lags one day). The price is a measurement here. No input comes from it. I made this section after the model was complete.

- The model P50 ($97.65) is 33.1% below the close. The model mean ($110.86) is 24.1% below it. The close is between the P75 ($136.20) and the P90 ($186.58). 79.2% of the paths are below the close.
- In version 1 the close ($150.96 on 2026-11-18) was inside the P25 to P75 range. After the 8-K the model value fell by 25% and the close fell by 3%. The main cause in the model is the guide for Q4 (section 7).
- The model says that, on its inputs, the share is worth less than the close in most of the paths. This is a statement about the inputs, not a finding that the price is wrong. The largest open difference is in the growth of the quarters after Q4: the guide gives +4% on Q3 in 13 weeks, and the consensus EPS (L23, not an input) needs a growth far above it.
- One number alone makes the P50 equal to $145.98, with all other numbers at base (`fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --price 145.98 --implied-price 145.98 --implied-param NAME --range LO,HI`; for the last three rows with `--shift-bounds`, so that the cut of the draw moves with the mean):

| Number | Base | Value for P50 = $145.98 |
|---|---|---|
| Half-life of the growth fade (D3) | 1.0 year | 2.53 years |
| Growth of year 1 (D2) | 40% | 71.7% (range 0.15 to 1.5) |
| Equity risk premium (D16) | 4.41% | 2.47% |
| Beta (D15) | 1.681 | 0.95 |
| Long-run margin before SBC (D7) | 48.5% (GAAP 45%) | no solution up to 78% (the cut of the process is 80%) |

In words: the close needs one of these. A growth fade that is 2.5 times slower. A first-year growth of 72% against the guide of the first quarter. A premium of 2.5% or a beta of 0.95 (a cost of capital about 3 points lower). A margin alone cannot do it. A lower number in one place can be paid by a higher number in another place: the table is one number at a time.

The close was higher before the 8-K of 2026-11-17: $228.38 on 2026-09-30 and $206.69 on 2026-11-03 (L15). The model has no fact that explains the fall. It does not use the fall.

## 6. Comparison with the automatic screen

`fa dcf auto NVDA --cache data/raw --as-of 2026-11-20 --explain` (seed 7, 10,000 paths) gives P10 1.18, P25 5.40, **P50 16.55**, P75 43.26, P90 104.57 and mean 43.85. The screen uses the 10-K and the 10-Q of Q2 only; it does not see the 8-K. It is not a reviewed valuation (`docs/models/auto-path-model.md`). This model gives P50 $97.65. The two differ for the reasons below. I did not decompose the gap one input at a time.

| Input | Automatic screen | This model |
|---|---|---|
| Base revenue | $215.9B (fiscal 2026; the four quarters are not used) | $361.8B (four quarters to 2026-10-25) |
| Revenue of year 1 | $257.8B (+19.4%, std 63%) | $506.5B at the means (+40%) |
| Check against reports | The last reported quarter ($115.8B) times four is $463B | |
| Margin at the start | 31.9% (cash flow before SBC) | 66% operating margin before SBC; free cash flow to the firm 39.6% of revenue in year 1 |
| Margin, long run | 19.4% (shrunk toward 7.9%, the mean of all companies above $100B, weight 0.44) | 48.5% operating margin before SBC; free cash flow to the firm 35.1% of revenue in year 10 |
| Reported history | | Free cash flow 41.1% of revenue in the last four quarters (OCF 158.9 less capex 10.1, over 361.8; L2c) |
| Net debt | +$10.9B (cash only, $22.4B, 2026-07-26) | -$21.1B (cash and debt securities, $53.5B, 2026-10-25) |
| Collapse of demand | No | Yes (D14) |
| Beta, risk premium, rates, L | 1.681, 4.41%, GSW, 4.0% | the same |

The screen shrinks the history of NVDA toward the mean of all large companies. For NVDA that mean is far from the facts. The free cash flow margin was above 14% in each of the last ten fiscal years and above 40% in the last three. The screen's year-1 revenue is below the revenue that NVDA has already reported. So the screen does not fit this company. The ticket gave the same cause (`analysis/priors/dcf_investigation_2026-11-20.md`, section 4.8). This model does not use the shrink.

## 7. How the model was built (order of the runs)

**Version 1** (data of 2026-11-19, before the 8-K). I wrote the inputs and the reasons before the first run.
1. First run, without the collapse table (D14): P10 73.84, P50 145.95, P90 327.08, mean 181.30.
2. A check of the growth process alone (Python, 20,000 paths, no engine): a year of revenue growth below -30% occurs on 5.1% of the paths, and a drawdown of about 35% from the peak is at the 90th percentile. The ticket asks for a downside of the cycle. So I added D14.
3. Run with D14: P10 59.06, P50 130.89, P90 298.97, mean 162.33. D14 lowered the P50 by $15.06. The hazard and the multiple are judgments.

**Version 2** (data of 2026-11-21, with the 8-K of 2026-11-17). The base run is the version 1 model file with no change, run on the store of 2026-11-21: the same text, P50 $130.89 (the new price and the new filing change no input of the file). I then changed the inputs in the order below. Each row is one more change on top of the rows above (seed 7, 10,000 paths; the files of the steps are not kept in the repo):

| Step | Change | P10 | P50 | P90 | Mean |
|---|---|---|---|---|---|
| 0 | Base run (version 1 file) | 59.06 | 130.89 | 298.97 | 162.33 |
| 1 | Base revenue $361.8B, growth start 40% (std 12%), year 1 after 2026-10-25, model date 2026-11-20 | 54.59 | 107.20 | 211.54 | 123.32 |
| 2 | Margin start 66%, SBC start 2.27% | 53.74 | 105.82 | 209.23 | 121.91 |
| 3 | Bridge of 2026-10-25 (net debt -21.141), shares 24,186M, beta 1.681 | 53.82 | 105.93 | 209.43 | 122.03 |
| 4 | Half-life 1.0 year (std 0.4) | 52.35 | 98.69 | 188.47 | 111.94 |
| 5 | Charge 1.5% (std 0.7%, cut at 5%) = final | 51.91 | 97.65 | 186.58 | 110.86 |

The tax rate (21%) and the treatment of the equity securities did not change. Reading the table: step 1 is the largest change (-$23.7). It has two causes. The guide for Q4 shows +4.2% on Q3 in 13 weeks, against +20% in each of the last four quarters. And the base is one quarter later, so the year-1 revenue is about the same in dollars ($507B against $500B) but the high-growth quarters now lie in the past. Step 4 is a judgment that follows from the guide (the old upper bound of the half-life is void). Without step 4 the P50 would be $105.93 (with the charge at 1.0%). I made the steps by the evidence of the 8-K and not by the price: the close was not used in any step. A skeptic can still ask whether I would have made step 4 if the P50 had gone up; the answer is that the ledger and the model file give the reason for it (the bound (b) of the half-life is no longer true).

## 8. Forward disclosures (rule R1)

| Disclosure | Decision | Reason |
|---|---|---|
| Remaining performance obligation, $3.2B on 2026-07-26 (L20) | Excluded | About 1% of the trailing revenue. It is not for product orders. |
| Purchase obligations, $22.7B at 2026-01-25 (L21) | Plausibility check only | They show supply bought, not orders received. |
| Outlook for Q4 FY27 (L2d, L22): revenue $130.0B +-3% in 14 weeks, gross margin 71% to 72%, opex about $11.0B, tax 16% to 18%, no China data center compute | **Used** | Evidence for D2 (first quarter of year 1, +4.2% on Q3 in 13 weeks), D3 (the speed of the slow-down) and D6 (margin of the guided quarter about 63.0% GAAP). The tax guide is not used for D13 (reason in section 3). The guide is one quarter: the other three quarters of year 1 are in the std of D2. The past of the company against its guides is not in the store. |
| Consensus EPS and targets (L23, L24) | Cross-check only | Not an input. Consensus EPS for the four quarters after Q3 is +70% over the last four reported quarters, and the model has revenue +40% in year 1 with a lower margin. This is the largest difference with the street. |

## 9. Weak points that I found

W1. **The growth start is the largest driver and rests on one guided quarter.** The mean of 40% follows the guide of +4.2% on Q3 in 13 weeks. The quarters after Q4 have no evidence in the store. NVDA beat the EPS consensus by 6% to 10% in each of the last four quarters (L23), so a low guide is possible: I could not measure it (the guide history is not in the store). The street EPS needs a much higher growth. At a start of 55% the P50 is $118.47. The press-release numbers were given to me by the orchestrator and I did not read the page (ledger section E).
W2. **The long-run margin (D7) and the half-lives (D3, D8) are judgments.** The only data is the own history (31% before the step, 59% after). The half-life moved from 1.25 to 1.0 in version 2 on the evidence of the guide (section 7, step 4). Section 4 shows the sensitivities.
W3. **The cost of capital is 12.55% for ten years and in the terminal value.** The beta of 1.68 comes from five years of weekly returns. A company with $1T of revenue can have a lower beta. A beta of 1.2 adds $26.5 to the P50. I kept the repo method.
W4. **The cycle is a wide shock plus a table that the engine made for failure (D14).** The engine has no regime. The hazard (2%) and the value of the collapse (2.0 times revenue) have no data of the repo. The two busts of the history (FY20, FY23) were mild in a year of revenue. The mean-path table of the output counts failed paths as zero revenue. Do not read it as a forecast.
W5. **The model has one segment.** The 8-K gives Data Center ($107.6B) and Edge Computing ($8.2B) for Q3 only, with growth rates and no history, and no customer concentration. A segment process needs the history.
W6. **Two items are outside the bridge: equity securities $109.7B (+$4.54 a share) and guarantees of $108.5B on 2026-07-26 (up to -$4.49).** The purchases of equity securities were $15.7B in Q3 alone (13.5% of revenue). If they finance customers, the model does not charge the cost: the pairing rule F16 has a gap here. A charge of 5% of revenue gives P50 $91.01. The decision and its reasons are in section 3. The note on guarantees (10-Q) is not read.
W7. **Two mapping gaps in `mapcheck`.** (a) The standard variable `short_term_investments` has no value for NVDA after 2025-10-26, because the filer uses the tag `DebtSecuritiesCurrent`. `mapcheck values NVDA` shows cash of $22.4B only. The screen net debt is wrong by $34.1B ($1.41 a share) and it does not see the 8-K. (b) The tag `PaymentsToAcquireBusinessTwoNetOfCashAcquired` ($13.0B in fiscal 2026) has no standard variable. I typed the numbers from the company facts file and from the 8-K. The fix is a change of `standard_variables.toml` with tests. The orchestrator decides.
W8. **Tax at 21% is above the effective rate (17.4% in Q3, guide 16% to 18% for FY27).** The P50 is $102.69 at 17%. The engine has no draw for the tax rate.
W9. **The terminal return on new capital (D20) is a judgment.** The P50 is $94.75 at 15% and $100.86 at 100%.
W10. **The timing is simple.** The balance sheet is of 2026-10-25 and the model date is 2026-11-20. Year 1 holds all cash flows after the balance sheet, discounted from the end of year 1 (0.93 years). Cash paid to holders since 2026-10-25 is not removed. The extra week of Q4 is removed (13 of 14 weeks). Mid-year timing gives P50 $103.46. `s2c` (D12) and the SBC of Q3 are not updated: the press release does not give them.
W11. **The right tail is long.** The P90 of revenue in year 10 is $1.6T in the mean-path table.
W12. **The panel that bounds D3 has survivor bias.** It is a bound only.
W13. **The growth-margin shock correlation (0.6) rests on two events.** With 0.25 the P50 is $97.24.
W14. **I did not read the press release page, the 10-K text or the notes.** The Q3 numbers came from the orchestrator (read by a fetch tool on 2026-11-21); the older numbers are XBRL facts. The ledger (section E) lists what is not read. The beta draw also differs from the IBM rule (beta fixed).
W15. **The model changed by $33 between version 1 and version 2** (P50 $130.89 to $97.65) for one filing. This shows how much the result depends on the growth input of the near quarters. A reader must not take the P50 as a stable number before the 10-Q and the next guide.

## 10. Open items and remaining steps

1. The Head of Technology adds the 10-Q of the fiscal third quarter of 2027 and the close of 2026-11-20 to the store. Then update D10 (SBC of Q3), D12 (receivables, inventory), D21, D22 and the ledger (check each row of section A2 of the ledger against the 10-Q). Read the guarantee note and the terms of the equity securities. Find the past guides of the company and measure the beat. Split revenue by segment if a history exists.
2. Orchestrator: decide W7 (new tags in `standard_variables.toml`).
3. Orchestrator: write `docs/models/` and `docs/assumptions/` records for D3, D7, D14 and D20. I did not write them.
4. The ticket acceptance names `analysis/NVDA/driver_model.toml` (old engine form). I wrote the path-engine form only.
5. Skeptic, reviewer, auditor, then the approval of the owner.

## 11. Commands (release build, in the worktree)

```
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --price 145.98 --implied-price 145.98 --implied-param process.growth.half_life --range 0.5,2.9
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --sensitivity process.growth.start=0.28,0.40,0.55
fa dcf auto NVDA --cache data/raw --as-of 2026-11-20 --explain
mapcheck values NVDA --cache data/raw
fa dcf drivers IBM          # P10 83.84, P50 151.84, P90 258.03 (unchanged)
```
(I ran these with `--cache /home/user/FinancialAnalysis/data/raw`, because the worktree `data/raw` is empty.)
