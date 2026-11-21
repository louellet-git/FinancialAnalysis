# `fa dcf auto --as-of` and `fa dcf screen --as-of` read facts that were filed after the date

*Trap:* `--as-of D` of `fa dcf auto` and `fa dcf screen` sets the model date, the curve and the
timing of the discount. It does not remove the facts that were filed after D. The mapping
(`map_company_with`) takes no date, and `check_ticker` uses the date only for the warning about
an old fiscal year. So a model "as of" a past date can use a later 10-Q, 10-K or restatement, and
a test of a past date with these commands has a look-ahead. `fa dcf pit` and `fa dcf returns` are
not affected: they build each model from the fiscal years up to the base year (`model_at`).

*What to do:* for a model of a past date, use `fa dcf attribute` or `fa dcf implied --as-of D`
(or `--price-date D`): they keep only the facts filed on or before D
(`CompanyFacts::filed_up_to`, `map_inputs_pit`) and do not fill an input from the
10-K file of the cache. Do not read `fa dcf auto --as-of D` for a past D as a point-in-time value.

These two commands are also not a point-in-time test of the share basis (the split factor): for that, use
`fa dcf implied --as-of`, `fa dcf returns` or the backtest.

*Evidence:* `rust/src/dcf/auto_cli.rs` (`map_inputs`, `map_inputs_pit`), `rust/mapcheck/src/facts.rs`
(`filed_up_to`); a poison test on 2026-11-21 (NVDA facts filed after 2026-05-01 times
1000): `fa dcf implied NVDA --price-date 2026-05-01` gives the same row on the clean and on the
poisoned cache. Found by the attribution work, ticket `T-20261120-valuation-move-attribution`.
