# Fund drivers (layer 3 of URA and GLD)

Code: `rust/src/macro_test/fund_drivers.rs`. Command: `fa macro fund-drivers [--cache DIR] [--out FILE]`.
Design (frozen, with records R1 to R3): `analysis/macro/2026-11-25-fund-drivers-design.md`. This document does not
change the design. If the two disagree, the design is correct.

## What it measures

How much of the monthly return of each part of URA, and of GLD, is consistent with a short list of outside drivers.
It also gives the part that no driver explains, as a number. It is a measurement. It is not a forecast, not a cause,
and not a trade view.

## Outcomes and drivers

| Outcome | Frequency | Drivers |
|---|---|---|
| URA-M (miners, record R3) | 36 months | U, S, C |
| URA-R (reactors, fuel, equipment, record R3) | 36 months | U, S, C |
| URA-F (foreign part) | 12 quarters | U |
| GLD | 36 months | R, D |

U is the IMF uranium price (log change). S is SPY, C is SMH, D is UUP (log returns of Nasdaq closes).
R is the change of the 10-year Treasury real yield, in percentage points. G (World Bank gold) is for the GLD
identity check only. All returns are log returns. Rates are decimals in the files and percentage points in R.

Group return of a month: the sum of (weight x month return) of the members, divided by the sum of their weights.
The weights come from the N-PORT report of the previous quarter end. They are buy and hold inside the quarter: in
months 2 and 3 each weight has drifted with its own price. A holding that enters joins at the next quarter.
The group lists are the constants `URA_M` and `URA_R` (record R3).

## Method

- OLS with an intercept. Newey-West standard errors: lag 3 (monthly), lag 1 (quarterly).
- Out of sample (monthly models): fit months 1 to 24, predict month 25, refit each month. R2 is against the mean of the fit window.
- Placebo: all drivers shift together on a circle. Only distinct shifts count: 6 to 30 (25 shifts) for monthly models,
  2 to 10 (9 shifts) for URA-F. p = (1 + count of shifts with R2 at or above the true R2) / (1 + number of shifts).
- Words: "consistent with the drivers" only if the out-of-sample R2 is above 0 and p is below 0.10.
  URA-F: p at or below 0.10, and no out-of-sample check.
- Unexplained part: (a) 1 - R2; (b) the sum of the out-of-sample errors of months 25 to 36 in percentage points, next to the return of those months.
  The intercept times 36 is a separate line.
- S and C are read together (R2 with and without them).
- Yearly coefficients (2024, 2025, 2026): description only.
- Sensitivity (record R1): URA-F without Daewoo E&C (ISIN KR7047040001). The words use the fit with all holdings.

## Data rules

- U is read at run time through a reader and is not stored (record R2). The command prints the read date and the SHA-256 of the series bytes.
  No IMF reader exists yet. Then the command prints "U not available", and the URA models are not run.
- The command reads the cache only. It sends no request and no order.
- The last quarter of URA-F needs the N-PORT report of the quarter end. If it is not yet public, URA-F uses fewer quarters, and the count is printed.

## Limits

36 months is a small sample. The weights change at quarter ends only. The foreign part closes quarterly.
The IMF and World Bank prices are monthly averages and lag a month-end return by about half a month.
A fit is not a cause.
