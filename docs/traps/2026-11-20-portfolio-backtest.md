# The portfolio backtest: today's ticker list, index addition and restated SEC facts

*Trap 1, survivorship and index addition:* `fa portfolio backtest` takes its tickers from a list that exists now (for
example `rust/mapcheck/universe/sp500.txt`). A company that failed, was bought or left the index before today is not in the
list. A company that joined the index after it grew is in the list for the whole test (in the run of 2026-11-20, 47 of 503
tickers have their first price after 2017-01-01). The strategies could not know this on the past dates, so the result is
too good for both strategies. The review measured an equal-weight portfolio of the whole list at about +14.4% a year
against SPY +12.35% (2018 to 2026-11-13). The command prints the equal-weight line, the number of tickers with a late first
price, and the warning.
*Trap 2, SEC facts restated (found in the review, fixed):* the first version cut only the list of fiscal years by the
filing date and took the latest filing of each year, so a year became known about 2 years after its end (about 790 days). The 550-day cap then dropped almost each ticker (0 to 19 of 119 scored in the review run) and the portfolio held cash on 766 of 2,190 days (35%); the new rule gives 57 to 93 scored and 0 days in cash (value-rank +19.02% a year, equal weight +14.56%). The value of each year came from the latest filing, so a restatement filed in 2022 entered a decision of 2020
(the review multiplied each fact filed from 2022-06-01 by 1000, and the portfolio changed from 2020-11-03). About 15% of the
annual periods with more than one filing differ between the first and the latest filing. The fix: `load_vintages` makes the
years again from the facts filed on one annual filing day or before (`CompanyFacts::filed_up_to`), once for each annual
filing day; `Vintages::known(day)` gives the latest vintage filed before the rebalance day. A fact with no filing date is not
in a vintage. The template of the filer and the list of tickers with facts still come from today's facts: a truncation test (facts cut at a past date) moved the scored set from 431 to 416 tickers and the value of the portfolio by about 1% by 2022-06.
*Also:* the price files are adjusted for splits, not for dividends (price returns, also for SPY, which is cap-weighted and
bought with no cost); 997 of 999 files in the store are like this, and the files of IBM and JNJ are adjusted for dividends.
A ticker that stops trading is sold at its last close (no failure loss). The parameter table of value-rank was estimated on
all years of the cache (in-sample).
*What to do:* read the result as a test of the machine, not as a forecast. Compare a strategy with the equal-weight line, not
only with SPY. A list of the index members for each past date (not in the data store now) removes trap 1. A strategy with
other data than prices must read it as a vintage, as `ValueRank` does.
*Evidence:* `rust/src/portfolio/cli.rs` (`LIMITS`); tests `value_rank_weights_do_not_move_with_a_fact_filed_after_the_date`,
`value_rank_weights_do_not_move_with_a_price_of_the_date_or_later`, `a_vintage_has_the_value_of_the_filings_before_the_date_not_the_restatement`
in `rust/src/portfolio/strategies.rs`.
