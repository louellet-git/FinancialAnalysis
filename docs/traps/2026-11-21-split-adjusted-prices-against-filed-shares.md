# A split-adjusted past price against as-filed shares gives a wrong market value

*Trap:* The store price files are adjusted for splits to the date of the download (`<T>/prices.meta.json`: `"adjusted": "splits"`). The share counts of the SEC facts are as filed. A value per share that uses as-filed shares, compared with an adjusted past price, is wrong by the factor of each later split. BKNG: the store close of 2018-01-05 is 72.94, the real close is about 1,823, because of the 25:1 split of 2026-04-06. A backtest that compares a vintage value per share with an adjusted past price has a split look-ahead.

*What to do:* Put the price and the shares on the same date. With F(a, b] the product of the split factors with a < ex_date <= b (`split_factor` in `rust/src/market/splits.rs`, table `<cache>/_corporate/splits.csv`): market value on day d = adj_close(d) x F(d, today] x shares(t) x F(t, d], where t is the date of the share count. The real close on day d is adj_close(d) x F(d, today]. Shares of date t, in the shares of day d, are shares(t) x F(t, d]. Also check that the table starts in 2016: Alpaca has no split before 2016, so a date before 2016 needs another source (the SEC tag).

*Evidence:* `fa splits get` (2026-11-21: BKNG 2026-04-06, factor 25), `docs/DATA-SOURCES.md` (rows "Daily prices" and "Stock splits"), `rust/src/market/splits.rs` (tests).
