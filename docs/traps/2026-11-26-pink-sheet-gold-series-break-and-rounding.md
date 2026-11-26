# The World Bank gold series changed its source in June 2025 and has whole dollars

*Trap:* The Pink Sheet column "Gold" is a monthly average of daily rates. Before June 2025 the
daily rate is the London afternoon fixing (99.5% fine). From June 2025 it is a spot average of
daily rates. The two parts are not one definition. The file also gives whole dollars (October 2026:
4105). A test that needs a smaller step than 1 USD, or a daily price, cannot use this series.
A monthly average is not a month-end price: it lags a trend.

*What to do:* Use the series for monthly decomposition only (fee, premium, long trend). Do not
match it to a fund price of one day. For a test across June 2025, say that the source changed.
`fa commodity show gold` prints the Description line of the file and warns when the text changes.

*Evidence:* `rust/src/market/commodity.rs` (`read_series`, `GOLD_DEFINITION_START`); sheet
"Description" of `CMO-Historical-Data-Monthly.xlsx`, downloaded 2026-11-26.
