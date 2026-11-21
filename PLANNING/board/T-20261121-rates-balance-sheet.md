---
id: T-20261121-rates-balance-sheet
title: The effect of a change of the yield curve on the assets and liabilities of a company (debt, investments, pensions), next to the effect on the discount rate
area: engine
status: backlog
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.007
branch: claude/premium-series
created: 2026-11-21
acceptance: (1) This design is committed before any measurement; (2) a measurement for the S&P 100 (no model change): for each company, the change of the market value of its debt, of its interest-bearing investments and of its pension obligation for the curve change of the week to 2026-11-20, from SEC facts, as points of its equity value, next to the rates part of `fa dcf attribute`; (3) a decision on a model change, with its own written test (a backtest) before it changes any value.
---
Owner words, 2026-11-21 about 19:10 UTC: "I think we also need to make sure that we're properly evaluating the impact of rates on
a company by getting an appropriate value of how it's assets and liabilities are affected by the change in curves."

What the engine does now (the defect to measure): the rates part of `fa dcf attribute` changes only the discount rate of
the free cash flow (the 10-year zero yield, `long_flat` mode; the shape of the curve is not used). The bridge from the
enterprise value to the equity value uses book values (debt, cash and investments of the last balance sheet), and they do
not move with the curve. So a fall of the rates does not raise the value of a fixed-rate debt (a loss for the equity), does
not raise the value of a bond portfolio (a gain), does not change a pension obligation, and does not change the interest
of a floating-rate debt or of the cash (short rates).

Design (Head of Fundamental Research, written before any measurement):
1. **Data (SEC facts of the store, point in time).** For each S&P 100 company: the carrying amount and the fair value of the
   debt (`LongTermDebtFairValue` and the like), the maturity ladder (`LongTermDebtMaturitiesRepaymentsOfPrincipalInNext...`),
   a fixed or floating split where it is tagged; the debt securities held, at fair value, with their maturities
   (`AvailableForSaleSecuritiesDebtMaturities...`, held-to-maturity); the defined-benefit obligation, the plan assets and the
   discount rate. A tag that the store does not have is a gap to report, not a guess. A new standard variable goes through
   `financial_analysis/standard_variables.toml` (the Head of Technology).
2. **Measurement.** The duration of each block from its ladder (else a stated default, the same for all companies), the
   curve change of each maturity from the par fit of 2026-11-13 and 2026-11-20 (not only the 10-year point), and the change
   of the market value of each block, as points of the equity market value of 2026-11-13. Then the sum against the rates part
   of the bridge, by company and for the S&P 10 and S&P 100.
3. **Decision.** If the balance-sheet effect is material for a company or a group (threshold fixed now: more than 0.25
   points of its equity value in this week, or more than 10% of its rates part), write a model change: the bridge at market
   value with the curve of the date, and the term structure of the curve in the discount (the `long` mode). Each change gets
   an assumption record and a point-in-time backtest written before its run (rule of this area). No value changes before
   that test passes.
4. **Rules.** No input is set from the price; the measurement uses SEC facts and the Treasury curve only; banks, insurers and
   REITs (no template now) are listed with the size of their debt and investments, as the largest gap.

**Measurement (2026-11-21 about 20:00 UTC; an agent, not reviewed):** `analysis/priors/rates_balance_sheet_measure_2026-11-21.md`.
The curve change of the week moves the market value of the debt, the debt securities held and the pensions by: S&P 10
-0.031 points (0.6% of its rates part +5.57), S&P 100 -0.093 points (1.9% of +4.93); range over the stated assumptions
-0.021 to -0.037 and -0.067 to -0.109. Both groups are BELOW the threshold of item 3. But 29 of 86 non-financial S&P 100
companies are ABOVE it, the largest leveraged ones: T -1.87, DUK -1.54, CMCSA -1.45, VZ -1.23, SO -1.15, NEE -0.89 points
of their equity value. Gaps: no fixed or floating split is tagged for any company; 65 of 86 have no debt fair value at the
date; 78 of 86 have no pension tags. Decision by the rule of item 3: a model change is needed for the company level (the
bridge at market value), not for the S&P 10 or S&P 100 sums. It is item 4 of `T-20261121-dcf-engine-integration`, with
an assumption record and a point-in-time backtest written before its run.
Commit note: commit c347c62 ("Board: rates effect ...") also holds the code of the beta, premium-sensitivity and 8-K
columns of `fa dcf attribute` (`T-20261121-premium-company-part`, an agent); the message does not say it. Tests pass
(846 + 17 + 1).

**Board note (2026-11-21 about 20:50 UTC):** the measurement is done; the next step (the bridge at market value) is item 4 of `T-20261121-dcf-engine-integration`. Back to the backlog.
