# Fund drivers (layer 3) for URA and GLD: pre-registration

Head of Economics, 2026-11-25. Status: frozen (see section 6). No data was read for this design.

## 1. Question

How much of the monthly return of each group of URA, and of GLD, is consistent with a short fixed list of outside drivers? The part that no driver explains is reported as a number. The answer is a measurement for Luc's decomposition of the two funds (layers 1 to 3), not a forecast and not a trade view.

## 2. Data and sample

1. Sample: 36 month-end returns, from the end of 2023-11 to the end of 2026-10. The month is the calendar month; a return is the log change of the last close of the month.
2. Outcomes (from layer 2, `fa fund attribution`). For each quarter, the weights are those of the N-PORT report of the previous quarter end, dated at the start of the quarter (the report becomes public about 60 days after its quarter end), and they are held constant inside the quarter. This is a description of the past, not a forecast:
   - URA-M: the US-listed miners group of URA.
   - URA-R: the US-listed reactor, fuel and equipment group of URA.
   - URA-F: the foreign-listed part of URA (about 40%). It closes only for each quarter, so it has 12 quarterly returns.
   - GLD: the fund itself.
3. Drivers (each from a source outside the fund price):
   - U: the log change of the IMF monthly uranium price (method: "determined by the largest exporter"; it differs from the Cameco month-end spot).
   - S: the SPY log return. C: the SMH (chip fund) log return.
   - R: the change of the 10-year Treasury real yield (Treasury real curve), in percentage points.
   - D: the UUP (dollar fund) log return.
   - G: the log change of the World Bank monthly gold price (layer 1 check only, not a driver; see rule 3.10).

## 3. Fixed rules

1. Models (ordinary least squares with an intercept; three drivers or fewer for each group, for 36 months):
   - URA-M on U, S, C (32 degrees of freedom).
   - URA-R on U, S, C (32 degrees of freedom).
   - URA-F on U only, quarterly (10 degrees of freedom).
   - GLD on R, D (33 degrees of freedom).
2. Standard errors: Newey-West with lag 3 (monthly models) and lag 1 (the quarterly model).
3. Out-of-sample check: an expanding window. Fit on months 1 to 24, predict month 25; refit and continue to month 36. Out-of-sample R² against the mean of the fit window. URA-F has too few quarters for this check; the page says so.
4. Placebo: shift all drivers of a model together, circularly in time. On a circle, a shift of -k is the shift of +(n - k), so only distinct shifts count: 6 to 30 months (25 shifts) for the monthly models, 2 to 10 quarters (9 shifts) for URA-F. p = (1 + the count of shifts with an in-sample R² at or above the true R²) / (1 + the number of shifts).
5. Words on the page:
   - "Consistent with the drivers" only if the out-of-sample R² is above 0 AND the placebo p value is below 0.10.
   - Else "not shown with these drivers".
   - URA-F: with 9 shifts the smallest p is 0.10, so the rule for URA-F is p at or below 0.10 (no shift reaches the true R²). The page states this resolution and that no out-of-sample check exists.
   - The page gives the count of models (four) next to the words: with four models at 10%, one false "consistent" is possible; the out-of-sample condition lowers that chance.
6. The unexplained part, as a number for each group: (a) 1 - R² in sample; (b) the out-of-sample part: the sum of the prediction errors of months 25 to 36, in percentage points, next to the return of those 12 months. URA-F: (a) only. The intercept (36 times the monthly intercept) is shown as a separate line; it is not an explanation.
7. For URA-M and URA-R, S and C move together, so each coefficient alone is unstable: the page reads S and C as one group (their R² and their contributions together), not one by one.
8. Description only (no test): the coefficients of each model in each calendar year (2024, 2025, 2026). The 2025 break of the gold and real-yield link is shown, not fitted away.
9. GLD and central-bank buying: the World Gold Council data is quarterly. The page shows the quarterly tonnes next to the quarterly GLD return as a table, with no regression.
10. The identity check of GLD (layer 1): the monthly GLD return against the World Bank gold price change. It is not a driver.

## 4. What the design does not allow

- No input from the price of URA or GLD on the driver side.
- No driver is added, removed or changed after a result is seen. A change before the run of record needs a dated record in section 6.
- One run of record. A result that fails rule 3.5 is reported as "not shown", not hidden.

## 5. Limits (stated on the page)

- 36 months is a small sample; the degrees of freedom are on the page.
- The weights change only at the quarter ends (N-PORT).
- The foreign part of URA closes only for each quarter (no permitted free source of daily foreign prices).
- The IMF uranium price is a monthly average with its own method; the fund return is month-end to month-end, so the average lags by about half a month and lowers the measured link with uranium. The contract price is not in our data.
- A fit is not a cause: the page says "consistent with".

## 6. Dated records

**Freeze.** The design is frozen at commit a8d550e plus this line (Head of Quality and Risk, PASS, 2026-11-25, about 22:50 UTC). No data was read before the freeze.

**Record R1 (2026-11-26, before the run; Head of Quality and Risk, OK).** URA-F is also reported without Daewoo E&C (its quarter-to-2026-04-30 part comes from one fund valuation with no second price). The rule-3.5 words use the fit with all holdings; the fit without it is a sensitivity line only.

**Record R2 (2026-11-26, before the run; source decision of the Head of Staff for Luc; Head of Quality and Risk, OK).** (1) G: the World Bank monthly gold series (CC BY 4.0, source named) for all months; before 2025-06 it is a monthly average of the London afternoon fixing. No change to its role (rule 3.10). Its average lags a month-end return by about half a month, the same as U; the page names this limit. (2) U: the IMF monthly uranium series is read at run time through Technology's reader and is not stored (its origin is a third party, NUEXCO/TradeTech). At the run of record, its SHA-256 and read date are written into this section, so the run stays fixed if the IMF revises a value. The page names the source, shows our results only (never the series values or a chart of them), and says in one line that a new run can change if the IMF revises a value. (3) The EIA annual uranium prices are a cross-check on the page only (description, no test). No driver, model, test or rule changes.

**Record R3 (2026-11-26, before the run; Head of Quality and Risk, OK).** The group of each US-listed holding of URA (the 16 lines of rust/src/market/fund_ticker_map.csv), by business, fixed for all quarters. URA-M (mining, exploration and royalties): CCJ, NXE, DNN, UEC, UUUU, URG, UROY, ISOU, EU. URA-R (reactors, fuel and equipment): LEU, OKLO, SMR, NNE, LTBR, IMSR, NKLR. A group return of a month = the sum of (weight x month return) of its members / the sum of their weights. The weights come from the N-PORT report of the previous quarter end (rule 2.2) and are buy-and-hold inside the quarter: month 1 uses the start weights, and in months 2 and 3 each weight has drifted with its price (the same rule as layer 2). A holding that enters in a quarter joins its group at the next quarter. Cameco is in URA-M: its largest business is mining, but it also has fuel services and a share of Westinghouse (reactor services; the size of the share comes from Quality and Risk and is checked at Cameco's own report before the page uses it). This ties URA-M to the reactor theme; the page states the choice.

**Record R4 (2026-11-26, before the run; found in the code review, before any layer-3 data was read; Head of Quality and Risk, OK).** URA-F uses the 12 quarters with public N-PORT reports, 2023-07-31 to 2026-07-31 (the same 12 quarters as the URA layer-2 page), one quarter earlier than the monthly window. Cause: the N-PORT report of 2026-10-31 is not public yet, so the quarters inside the monthly window give only 11 quarters, 8 distinct shifts and a smallest p of 1/9, and rule 3.5 could never be met. U for a quarter is the sum of its three monthly log changes (IMF months 2023-08 to 2026-07). Distinct shifts 2 to 10 = 9; smallest p = 0.10; rule 3.5 for URA-F stays p <= 0.10. The command stops with an error if it has fewer than 12 quarters. The page states that URA-F ends one quarter before the monthly models.
