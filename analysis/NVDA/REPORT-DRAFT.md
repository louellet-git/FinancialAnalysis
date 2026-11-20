# NVDA: what is the share worth? Report draft

**Status: draft, not reviewed.** This text is step 1 of 4 of the review (analyst). The skeptic, the reviewer and the auditor have not seen it. Do not use it alone for a trade. The values are model values, not market prices.

Version of 2026-11-20. Model date 2026-11-19. Data store `41116d0`: SEC facts of 2026-11-19, prices to 2026-11-18. Files: `analysis/NVDA/driver_model_paths.toml` (the model) and `analysis/NVDA/research-ledger.md` (the outside facts, ids L1 to L24).

## 1. Result

| Measure | Value for each share (USD) |
|---|---|
| P10 | 59.06 |
| P25 | 85.86 |
| **P50 (headline)** | **130.89** |
| P75 | 201.88 |
| P90 | 298.97 |
| Mean | 162.33 (standard error 1.20) |

Command: `fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml` (10,000 paths, seed 7). Two runs give the same text. Seed 11 gives P50 132.35. Seeds 7 and 12 with 100,000 paths give P50 131.57 and 131.25. The noise of the P50 is about $1.5. The mean is higher than the P50 because the right tail is long. Use the P50 as the headline.

- 18.3% of the paths have a demand collapse inside ten years (driver D14).
- The terminal value is 41.8% of the enterprise value at the means and 44.5% as the mean over the paths.
- The equity value at the P50 is $3,178.8B on 24,285M shares. The enterprise value at the P50 is $3,155.6B, or 10.4 times the trailing revenue of $303.0B.
- The value at the means (every drawn number at its mean, no shock) is $148.79. A hand calculation of the same cash flows (Python, no engine, no collapse) gives $164.07. The engine gives $164.15 for the same inputs.
- Year 1 at the means: revenue $499.9B, EBIT margin 64.6%, tax 21%, reinvestment $65.6B, free cash flow to the firm $189.5B.

## 2. The business

NVIDIA designs processors and systems. The fiscal year ends in late January. Revenue was $215.9B in fiscal 2026 (to 2026-01-25) and $303.0B in the four quarters to 2026-07-26. The last four quarters were $57.0B, $68.1B, $81.6B and $96.2B. Each was 18% to 22% above the quarter before. The operating margin was 66.2% in the last quarter and 15.7% in fiscal 2023 (ledger L1, L3). The company holds $56.6B of cash and debt securities and $33.4B of debt (L10). It also holds $90.7B of equity stakes (L11) and has guarantees with a maximum of $108.5B (L12). The store has no revenue by segment, so the model has one segment. The results of the fiscal third quarter of 2027 (8-K of 2026-11-17) are not in the store (L2).

## 3. The drivers

Tags: DATA = a reported number or the calibration table. CONVENTION = a record of this repo. JUDGMENT = the analyst. "Stoch." = drawn one time for each path. A process has a yearly shock in addition. The reasons are in the model file.

| Id | Driver | Value | Distribution | Source and tag |
|---|---|---|---|---|
| D1 | Base revenue | $302.969B | fixed | Four quarters to 2026-07-26 (L1). DATA |
| D2 | Growth of year 1 | 65%, std 20%, cut 20% to 100% | stoch. | Reported quarters (L1a). A flat run rate gives +27%. A fade of the sequential growth gives +63% to +76%. DATA + JUDGMENT |
| D3 | Half-life of the growth fade | 1.25 years, std 0.5, cut 0.4 to 3.0 | stoch. | Between the panel (0.6 to 0.7 years, L19) and the own momentum (2.5 years). JUDGMENT |
| D4 | Long-run growth | 4.0% | fixed | A-001 (L18). CONVENTION |
| D5 | Yearly shock of growth (the cycle) | max(15%, 30% of the slow part), persistence 0, cut -50% to +200% | yearly shock | Own history, two busts in ten years (L3). JUDGMENT + DATA |
| D6 | Margin before SBC, year 1 | 68%, std 3% | stoch. | Q2 FY27 68.35%, trailing 67.60% (L1c, L6). DATA |
| D7 | Long-run margin before SBC | 48.5%, std 8%, cut 25% to 65% (GAAP 45%) | stoch. | Middle of the own history: 31% (FY17 to FY22) and 59% (FY24 to FY26) (L3). JUDGMENT |
| D8 | Half-life of the margin fade | 3 years, std 1 | stoch. | JUDGMENT |
| D9 | Yearly shock of margin | 6 points, persistence 0.5, correlation with the growth shock 0.6 | yearly shock | Calibration table (persistence) and own history (L3). JUDGMENT |
| D10 | SBC (a cost, rule F5) | 2.39% in year 1, to 3.5% (std 1%) in year 10 | stoch. target | L6. DATA + JUDGMENT |
| D11 | Purchases of businesses (charge, rule F16) | 1.0% of revenue, std 0.5%, cut 0 to 4% | stoch. | L7. JUDGMENT |
| D12 | Sales to capital (reinvestment, rule F16) | 3.0 in year 1, to 4.0 (std 1.0) in year 10 | stoch. target | L4a. DATA + JUDGMENT |
| D13 | Tax rate | 21% | fixed | Statutory. The effective rate is 15% to 17% (L5). CONVENTION |
| D14 | Demand collapse | hazard 2% a year (std 1%), value 2.0 times the revenue of the year before | stoch. hazard, event sampled | The engine has no regime state. JUDGMENT |
| D15 | Beta | 1.676, std 0.09, lower bound 1.0 | stoch. | Regression 2.013 shrunk; standard error 0.088 (L8). DATA |
| D16 | Equity risk premium | 4.41%, std 0.75%, lower bound 2% | stoch. | L17. CONVENTION |
| D17 | Risk-free rate | Hull-White on the GSW curve of 2026-11-13; 10-year zero yield 5.13% | process | L16. CONVENTION |
| D18 | Equity weight | 1.0 | fixed | The market weight is an input from the price. The debt is below 1% of any plausible value. CONVENTION |
| D19 | Terminal growth | 4.0%, std 0.75%, capped by D4 and by the risk-free rate | stoch. | A-001. CONVENTION |
| D20 | Terminal return on new capital | 35%, std 15%, cut 15% to 100% | stoch. | JUDGMENT |
| D21 | Net debt | -$23.220B (net cash) | fixed | L10. DATA |
| D22 | Shares | 24,285M (diluted) | fixed | L9. DATA |

Cost of capital at the means: 12.53% in each year (risk-free 5.14% plus beta 1.676 times 4.41%).

**Pairing and stock-or-flow rules.**
- Growth credited has its cost (F16). The model charges reinvestment for each dollar of revenue growth (D12), a part of the purchases of businesses (D11) and SBC (D10). The terminal value reinvests growth divided by the return on new capital (D20).
- No stock with its flow (F15). The margin is the operating margin. Interest income and gains on investments are not in the cash flow, so cash and debt securities are in the bridge. The equity stakes are not in the bridge (L11). The lease cost is in the operating income, so the lease liability is not in the bridge (L13). Purchase obligations are costs of future product, already in the gross margin (L14).
- No debt twice (F1). Interest expense is not in the cash flow. The debt of $33.4B is subtracted one time.
- SBC is a cost (F5). GAAP operating income already holds SBC. The model adds the SBC ratio to the GAAP margin and charges it as D10. The share count is the diluted count (existing awards).
- Forward disclosures (R1): section 8 and the ledger (L20 to L24).

## 4. What drives the value

Change of the P50 when the mean of one drawn number moves by one standard deviation:

| Driver | Minus 1 std | Plus 1 std |
|---|---|---|
| Growth of year 1 (D2) | -32.06 | +37.20 |
| Half-life of the growth fade (D3) | -26.23 | +34.17 |
| Equity risk premium (D16) | +19.43 | -15.11 |
| Long-run margin (D7) | -15.35 | +14.29 |
| Collapse hazard (D14) | +7.70 | -7.70 |
| Beta (D15) | +5.57 | -5.18 |
| Half-life of the margin fade (D8) | -5.04 | +4.24 |
| Terminal return on new capital (D20) | -3.50 | +2.08 |
| Others (D6, D19, D10, D11, D12) | each under 2.6 | |

Growth is the main source of the spread. When the growth draws and shocks are fixed at their means, 81% of the variance is gone. The margin removes 29% and the discount rate removes 21%.

**Sensitivity to the three most important drivers** (P50, other numbers at base):

| Growth of year 1 (D2) | 30% | 45% | **65%** | 85% |
|---|---|---|---|---|
| P50 | 82.41 | 98.83 | **130.89** | 168.09 |

| Half-life of the growth fade, years (D3) | 0.6 | **1.25** | 2.5 |
|---|---|---|---|
| P50 | 99.81 | **130.89** | 223.45 |

| Equity risk premium (D16) | 3.5% | **4.41%** | 5.5% |
|---|---|---|---|
| P50 | 154.95 | **130.89** | 110.14 |

Other sensitivities (P50; each changes one number with `--pset`):

| Change | P50 | Change | P50 |
|---|---|---|---|
| Long-run margin 35% / 60% (D7) | 106.28 / 150.81 | Beta 1.2 / 1.0 (D15) | 166.78 / 184.81 |
| Tax 17% (D13) | 137.57 | Mid-year cash flows | 138.40 |
| Hazard 0% / 4% (D14) | 144.66 / 116.11 | Value of collapse 1.0 / 4.0 times revenue | 127.79 / 138.08 |
| Growth shock 10% / 25% (D5) | 133.75 / 120.74 | Persistence of growth shock -0.25 / +0.25 | 129.79 / 130.79 |
| Terminal return 15% / 100% (D20) | 126.51 / 135.69 | Charge 0% / 3% (D11) | 133.00 / 125.46 |
| Sales to capital 2 / 6 (D12) | 129.38 / 131.98 | Equity stakes in the bridge (+$3.74) | 134.63 |
| No yearly shocks | 137.80 | Shock correlation 0.25 (a copy of the file) | 130.55 |

The cost of capital is high for a company of this size. A beta of 1.2 gives $166.78.

## 5. What the price implies

The last close in the store is **$150.96 on 2026-11-18** (L15). The closes of 2026-11-19 and 2026-11-20 are not in the store. The price is a measurement here. No input comes from it. I made this section after the model was complete.

- The model P50 ($130.89) is 13.3% below the close. The model mean ($162.33) is 7.5% above it. The close is between the P25 and the P75. 59.2% of the paths are below the close.
- The model does not say that the price is wrong. The price is inside the range.
- One number alone makes the P50 equal to $150.96, with all other numbers at base (`fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --price 150.96 --implied-price 150.96 --implied-param NAME --range LO,HI`):

| Number | Base | Value for P50 = $150.96 |
|---|---|---|
| Half-life of the growth fade (D3) | 1.25 years | 1.56 years |
| Growth of year 1 (D2) | 65% | 76% |
| Long-run margin before SBC (D7) | 48.5% (GAAP 45%) | 60.1% (GAAP 56.6%) |
| Equity risk premium (D16) | 4.41% | 3.64% |
| Beta (D15) | 1.676 | 1.380 |

The close needs one of these: growth that fades 25% slower, a long-run GAAP operating margin of about 57%, or a cost of capital about 1.3 points lower. Each one is inside the range of the draws of the model.

The close was higher before the 8-K of 2026-11-17: $228.38 on 2026-09-30 and $206.69 on 2026-11-03 (L15). The model has no fact that explains the fall. It does not use the fall.

## 6. Comparison with the automatic screen

`fa dcf auto NVDA --cache data/raw --as-of 2026-11-19 --explain` (seed 7, 10,000 paths) gives P10 1.18, P25 5.40, **P50 16.56**, P75 43.34, P90 104.73 and mean 43.95. The screen is not a reviewed valuation (`docs/models/auto-path-model.md`). This model gives P50 $130.89. The two differ for the reasons below. I did not decompose the gap one input at a time.

| Input | Automatic screen | This model |
|---|---|---|
| Base revenue | $215.9B (fiscal 2026; the four quarters are not used) | $303.0B (four quarters to 2026-07-26) |
| Revenue of year 1 | $257.8B (+19.4%, std 63%) | $499.9B at the means (+65%) |
| Check against reports | The last reported quarter ($96.2B) times four is $385B | |
| Margin at the start | 31.9% (cash flow before SBC) | 68% operating margin before SBC; free cash flow to the firm 37.9% of revenue in year 1 |
| Margin, long run | 19.4% (shrunk toward 7.9%, the mean of all companies above $100B, weight 0.44) | 48.5% operating margin before SBC; free cash flow to the firm 35.6% of revenue in year 10 |
| Reported history | | Free cash flow 41.9% of revenue in the last four quarters (OCF 134.4 less capex 7.4, over 303.0) |
| Net debt | +$10.9B (cash only, $22.4B) | -$23.2B (cash and debt securities, $56.6B) |
| Collapse of demand | No | Yes (D14) |
| Beta, risk premium, rates, L | 1.676, 4.41%, GSW, 4.0% | the same |

The screen shrinks the history of NVDA toward the mean of all large companies. For NVDA that mean is far from the facts. The free cash flow margin was above 14% in each of the last ten fiscal years and above 40% in the last three. The screen's year-1 revenue is below the revenue that NVDA has already reported. So the screen does not fit this company. The ticket gave the same cause (`analysis/priors/dcf_investigation_2026-11-20.md`, section 4.8). This model does not use the shrink.

## 7. How the model was built (order of the runs)

I wrote the inputs and the reasons before the first run.
1. First run, without the collapse table (D14): P10 73.84, P50 145.95, P90 327.08, mean 181.30.
2. A check of the growth process alone (Python, 20,000 paths, no engine): a year of revenue growth below -30% occurs on 5.1% of the paths, and a drawdown of about 35% from the peak is at the 90th percentile. The ticket asks for a downside of the cycle. So I added D14.
3. Final run with D14: P50 130.89. No other input changed after the first run. Only the text and the sensitivity runs changed.

D14 lowered the P50 by $15.06. The hazard and the multiple are judgments (section 3).

## 8. Forward disclosures (rule R1)

| Disclosure | Decision | Reason |
|---|---|---|
| Remaining performance obligation, $3.2B (L20) | Excluded | 1.1% of the trailing revenue. It is not for product orders. |
| Purchase obligations, $22.7B at 2026-01-25 (L21) | Plausibility check only | They show supply bought, not orders received. |
| Guidance of the company (L22) | Not used; waits | The 8-K is not in the store. It is the main test of D2. |
| Consensus EPS and targets (L23, L24) | Cross-check only | Not an input. Consensus EPS for the next four quarters is +66% over the last four reported quarters. Model revenue growth for year 1 is +65%. |

## 9. Weak points that I found

W1. **The growth start is the largest driver and rests on four quarters.** The third quarter (reported 2026-11-17) and the guidance for the fourth quarter are not read. They can move D1 and D2. This is the first update to make.
W2. **The long-run margin (D7) and the half-lives (D3, D8) are judgments.** The only data is the own history (31% before the step, 59% after). Section 4 shows the sensitivities.
W3. **The cost of capital is 12.5% for ten years and in the terminal value.** The beta of 1.68 comes from five years of weekly returns. A company with $1T of revenue can have a lower beta. A beta of 1.2 adds $36 to the P50. I kept the repo method.
W4. **The cycle is a wide shock plus a table that the engine made for failure (D14).** The engine has no regime. The hazard (2%) and the value of the collapse (2.0 times revenue) have no data of the repo. The two busts of the history (FY20, FY23) were mild in a year of revenue. The mean-path table of the output counts failed paths as zero revenue. Do not read it as a forecast.
W5. **The model has one segment.** It has no Data Center share and no customer concentration. The earnings release has the data.
W6. **Two items are outside the bridge: equity stakes $90.7B (+$3.74 a share) and guarantees $108.5B (up to -$4.47).** I did not read the note of the 10-Q. Part of the demand can be financed by NVDA itself (stakes, guarantees). The model charges 1% of revenue for purchases of businesses, not a financing of customers. This is a gap in the pairing rule F16.
W7. **Two mapping gaps in `mapcheck`.** (a) The standard variable `short_term_investments` has no value for NVDA after 2025-10-26, because the filer uses the tag `DebtSecuritiesCurrent`. `mapcheck values NVDA` shows cash of $22.4B only. The screen net debt is wrong by $34.1B ($1.41 a share). (b) The tag `PaymentsToAcquireBusinessTwoNetOfCashAcquired` ($13.0B in fiscal 2026) has no standard variable. I typed the numbers from the company facts file. The fix is a change of `standard_variables.toml` with tests. The orchestrator decides.
W8. **Tax at 21% is above the effective rate (15% to 17%).** The P50 is $137.57 at 17%. The engine has no draw for the tax rate.
W9. **The terminal return on new capital (D20) is a judgment.** The P50 is $126.51 at 15% and $135.69 at 100%.
W10. **The timing is simple.** The balance sheet is of 2026-07-26 and the model date is 2026-11-19. Year 1 holds all cash flows after the balance sheet, discounted from the end of year 1 (0.68 years). Cash paid to holders since 2026-07-26 is not removed. Mid-year timing gives P50 $138.40.
W11. **The right tail is long.** The P90 of revenue in year 10 is $2.7T in the mean-path table.
W12. **The panel that bounds D3 has survivor bias.** It is a bound only.
W13. **The growth-margin shock correlation (0.6) rests on two events.** With 0.25 the P50 is $130.55.
W14. **I did not read the text and notes of the filings.** All company numbers are XBRL facts. The ledger (section E) lists what is not read. The beta draw also differs from the IBM rule (beta fixed).

## 10. Open items and remaining steps

1. The Head of Technology adds the 10-Q of the fiscal third quarter of 2027, the 8-K of 2026-11-17 and the closes of 2026-11-19 and 2026-11-20 to the store. Then update D1, D2, D6, D10, D21, D22 and the ledger. Read the guidance and the guarantee note. Split revenue by segment if the release allows.
2. Orchestrator: decide W7 (new tags in `standard_variables.toml`).
3. Orchestrator: write `docs/models/` and `docs/assumptions/` records for D3, D7, D14 and D20. I did not write them.
4. The ticket acceptance names `analysis/NVDA/driver_model.toml` (old engine form). I wrote the path-engine form only.
5. Skeptic, reviewer, auditor, then the approval of the owner.

## 11. Commands (release build, in the worktree)

```
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --price 150.96 --implied-price 150.96 --implied-param process.margin.target --range 0.30,0.64
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --sensitivity process.growth.start=0.30,0.45,0.65,0.85
fa dcf auto NVDA --cache data/raw --as-of 2026-11-19 --explain
mapcheck values NVDA --cache data/raw
fa dcf drivers IBM          # P10 83.84, P50 151.84, P90 258.03 (unchanged)
```
(I ran these with `--cache /home/user/FinancialAnalysis/data/raw`, because the worktree `data/raw` is empty.)
