# Fund drivers (layer 3) for URA and GLD: pre-registration

Head of Economics, 2026-11-25. Status: draft for the freeze. No data was read for this design.

## 1. Question

How much of the monthly return of each group of URA, and of GLD, is consistent with a short fixed list of outside drivers? The part that no driver explains is reported as a number. The answer is a measurement for Luc's decomposition of the two funds (layers 1 to 3), not a forecast and not a trade view.

## 2. Data and sample

1. Sample: 36 month-end returns, from the end of 2023-11 to the end of 2026-10. The month is the calendar month; a return is the log change of the last close of the month.
2. Outcomes (from layer 2, `fa fund attribution`, weights from the N-PORT report of each quarter, held constant inside the quarter):
   - URA-M: the US-listed miners group of URA.
   - URA-R: the US-listed reactor, fuel and equipment group of URA.
   - URA-F: the foreign-listed part of URA (about 40%). It closes only for each quarter, so it has 12 quarterly returns.
   - GLD: the fund itself.
3. Drivers (each from a source outside the fund price):
   - U: the log change of the IMF monthly uranium price (method: "determined by the largest exporter"; it differs from the Cameco month-end spot).
   - S: the SPY log return. C: the SMH (chip fund) log return.
   - R: the change of the 10-year Treasury real yield (Treasury real curve), in percentage points.
   - D: the UUP (dollar fund) log return.
   - G: the log change of the World Bank monthly gold price (layer 1 check only, not a driver; see rule 3.6).

## 3. Fixed rules

1. Models (ordinary least squares with an intercept; three drivers or fewer for each group, for 36 months):
   - URA-M on U, S, C (32 degrees of freedom).
   - URA-R on U, S, C (32 degrees of freedom).
   - URA-F on U only, quarterly (10 degrees of freedom).
   - GLD on R, D (33 degrees of freedom).
2. Standard errors: Newey-West with lag 3 (monthly models) and lag 1 (the quarterly model).
3. Out-of-sample check: an expanding window. Fit on months 1 to 24, predict month 25; refit and continue to month 36. Out-of-sample R² against the mean of the fit window. URA-F has too few quarters for this check; the page says so.
4. Placebo: shift all drivers of a model together, circularly in time, by 6 to 30 months in both directions (50 shifts; quarterly model: 2 to 10 quarters, 18 shifts). The placebo p value is the share of shifts with an in-sample R² at or above the true R².
5. Words on the page:
   - "Consistent with the drivers" only if the out-of-sample R² is above 0 AND the placebo p value is below 0.10.
   - Else "not shown with these drivers".
   - URA-F: "consistent" needs the placebo p value below 0.10 only, and the page states that no out-of-sample check exists.
6. The unexplained part, as a number for each group: the 36-month (or 12-quarter) log return less the sum of the fitted driver contributions (coefficient times the cumulative driver change), in percentage points, with the intercept shown as a separate line. Also 1 - R².
7. Description only (no test): the coefficients of each model in each calendar year (2024, 2025, 2026). The 2025 break of the gold and real-yield link is shown, not fitted away.
8. GLD and central-bank buying: the World Gold Council data is quarterly. The page shows the quarterly tonnes next to the quarterly GLD return as a table, with no regression.
9. The identity check of GLD (layer 1): the monthly GLD return against the World Bank gold price change. It is not a driver.

## 4. What the design does not allow

- No input from the price of URA or GLD on the driver side.
- No driver is added, removed or changed after a result is seen. A change before the run of record needs a dated record in section 6.
- One run of record. A result that fails rule 3.5 is reported as "not shown", not hidden.

## 5. Limits (stated on the page)

- 36 months is a small sample; the degrees of freedom are on the page.
- The weights change only at the quarter ends (N-PORT).
- The foreign part of URA closes only for each quarter (no permitted free source of daily foreign prices).
- The IMF uranium price is a monthly average with its own method; the contract price is not in our data.
- A fit is not a cause: the page says "consistent with".

## 6. Dated records

None yet. The freeze line comes after the review of Quality and Risk.
