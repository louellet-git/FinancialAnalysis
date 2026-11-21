# A split-adjusted past price against as-filed shares gives a wrong market value

*Trap:* The store price files are adjusted for splits to the date of the download (`<T>/prices.meta.json`: `"adjusted": "splits"`). The share counts of the SEC facts are as filed. A value per share that uses as-filed shares, compared with an adjusted past price, is wrong by the factor of each later split. BKNG: the store close of 2018-01-05 is 72.94, the real close is about 1,823, because of the 25:1 split of 2026-04-06. A backtest that compares a vintage value per share with an adjusted past price has a split look-ahead.

*What to do:* Put the price and the shares on the same date. With F(a, b] the product of the split factors with a < ex_date <= b (`split_factor` in `rust/src/market/splits.rs`, table `<cache>/_corporate/splits.csv`): market value on day d = adj_close(d) x F(d, today] x shares(t) x F(t, d], where t is the date of the share count. The real close on day d is adj_close(d) x F(d, today]. Shares of date t, in the shares of day d, are shares(t) x F(t, d]. 

Three more facts about the table (`<cache>/_corporate/splits.csv`):

- A split with `status` = `pending` has an ex date after the last date of the price file of the ticker (SNDK 1:8, 2026-11-20, price file to 2026-11-19). The price file is not adjusted for it. Ask `split_factor` with `to` = the last date of the price file, or call `split_factor_for_prices(splits, ticker, after, price_last)`. A call with `to` = today counts a pending split that the prices do not have.
- Alpaca has no split before 2016, and it gives price factors of spin-offs and mergers as splits (HON, DELL, HLT). The rules of `rust/src/market/splits_rules.rs` drop a row that the SEC cover share count does not confirm, and add the steps of that count that Alpaca misses (`source` = "SEC cover-count step", `ex_date_estimated` = true). The estimated ex date is the cover date: it can be up to 250 days after the true date. A share count dated between the true date and the estimate has the wrong basis.
- A row with `source` = "alpaca corporate actions; no SEC fact" or "not confirmed by SEC" has a simple factor that no SEC cover count could confirm. A person can drop it in `rust/src/market/splits_corrections.csv`.

*Evidence:* `fa splits get` (2026-11-21: BKNG 2026-04-06, factor 25; the cover count steps 25 times), `docs/DATA-SOURCES.md` (rows "Daily prices" and "Stock splits"), `rust/src/market/splits.rs` (tests).
