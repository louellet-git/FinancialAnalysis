# Frozen Forecasts: Scoring Rule

*The rule to score the forecasts of 2026-11-25 against the real values that the companies file later. Written before
the forecasts were made and before any real value exists.*

Background: the one-time score of the projection test (T1) failed on 2026-11-25. No historical sample is blind now.
The person chose to save the forecasts of today and to score them on the reports of fiscal 2026 and later
(`chiefs/louellet-git/staff/RECORD.md`, 2026-11-25, "Fix the middle and freeze"). This file is that rule. It does not
say which test lifts the rule "No trades until T1 passes": the person decides that.

## What is frozen

- File: `forecasts.csv` in this folder. One row for each company of the standard template that has a forecast.
- The forecast: the model of main as it is on 2026-11-25, with its default rules and its default parameter table
  (`analysis/priors/process_priors_2026-11-19.toml`), at the end of the LAST fiscal year of the company in the data
  store (the base year). 2000 paths, seed 7. For each of the 3 years after the base year (T+1, T+2, T+3): the P10,
  the P50 and the P90 of the revenue and of the free cash flow (the definition of `fa dcf pit`).
- The command, the rules and the counts are in the comment lines at the top of the file. The engine commit and the
  store commit are in section "Record" below.
- A company is in the file only if its last fiscal year ends on or after 2025-01-01, the 5 years up to it are in
  sequence, and the model accepts it.

## The scoring rule

1. The real value of a year is the value that `fa dcf pit` reads for that fiscal year from a later data store: the
   revenue, and the free cash flow by the same definition and the same default rules.
2. A row enters the score of horizon T+h only if the annual report of year T+h was filed AFTER 2026-11-25 (the date of
   the freeze). A report filed on or before 2026-11-25 can be public already, so it does not enter. The
   score gives the number of rows that this rule takes out.
3. The forecasts do not change. A company whose report is late or missing does not enter. A company that has a
   different number of years in the later store (a restatement, a merger) enters if its year T+h has a value.
4. The cells: for each horizon, the share of the real values below P10, inside P10 to P90, and above P90; and the
   share below P50. The rows "all" and the rows by size (the classes of `fa dcf pit`).
5. The pass rule is the rule of T1, unchanged: on the row "all", for the revenue and for the free cash flow, at each
   horizon that is scored, the share inside is 75% to 85% and each tail is 5% to 15%.
6. The order: T+1 first, when most reports of the fiscal years that end in 2026 are filed (about 2027-04-01); T+2
   and T+3 one year and two years later. Each horizon is scored one time. The result of each is reported as it is,
   pass or fail.
7. Label: "blind". The model and the rule were fixed before the real values existed.
8. A model that is changed after 2026-11-25 (for example the work on the middle) does not replace this file. It can
   be frozen in a new folder with its own date, before the reports that it is scored on are filed.

## Limits stated before the score

- The real values of the fiscal year 2026 are partly known today in the quarterly reports. The model does not use
  quarterly data, and the model was not chosen for this file (it is the model of main), so this does not move the
  forecasts. The person can see this as a weaker point of T+1.
- The middle of the model was too high on the hold-out of 2011 to 2014 (T1). The same error can show here.
- The free cash flow of the real data can change with a change of the data mapping. A change of the mapping after
  2026-11-25 is listed in the score.

## Record

Filled in when the file is made: date and time (UTC), the engine commit, the store commit, the command, the counts.
