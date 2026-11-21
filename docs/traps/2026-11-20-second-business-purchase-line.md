# A second line for the purchase of businesses has its own tag

*Trap:* A filer can show two lines for the purchase of businesses in the investing section. NVDA (fiscal 2026) has `PaymentsToAcquireBusinessesNetOfCashAcquired` (1.535B) and `PaymentsToAcquireBusinessTwoNetOfCashAcquired` (13.0B). The registry takes the first tag that has a fact. If the two tags are in one variable, the second line is lost. If a reader adds them, a filer that tags one item with both tags is counted two times.

*What to do:* The variable `acquisitions_net_of_cash_second` has the Two tag. It is a part of the investing total (`rollup` of the investing cash flow). To get all purchases of businesses, add the second variable only when its value differs from the first, or when the first has no value. A filer can rename the tag between 10-Ks, and then both tags have the same value for one year: 15 of 60 filers with both tags have 21 such periods (EQT FY2021 1,030,239,000 in both; QSR FY2023 17M; MGY FY2022 90.126M). The investing rollup (`rollup_parts` in `rust/mapcheck/src/checks.rs`) skips the second part when it equals the first. 71 filers in the cache have the Two tag; 67 of them also have the first tag. Four have only the Two tag (ZBH, MATX, LUMN, WT): for them `acquisitions_net_of_cash` has no value.

*Evidence:* `mapcheck filing NVDA --statement cash` (both lines are in the cash flow statement), `financial_analysis/standard_variables.toml` (`acquisitions_net_of_cash_second`).

*Done 2026-11-21:* the automatic model adds the second line in `YearData::acq_spend` (rule `acq_second`, default on). MATX, LUMN and WT have the second line only before the last 5 years of the charge, so their charge is still 0.
