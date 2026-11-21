# A 1% share test cannot separate a small equity REIT from a mortgage REIT

*Trap:* The tags `InterestAndDividendIncomeOperating` and `InterestIncomeOperating` give revenue only to a filer with no current-assets total and with a value of 1% or more of total assets. UMH FY2010 (an equity REIT) has 2.817M of interest income, 1.49% of total assets (188.8M). It passes the test, and its revenue for FY2010 is this interest income. A mortgage REIT has 3% to 4% of total assets, so no single threshold separates the two cases.

*What to do:* Do not use the revenue of an equity REIT for an early year (UMH FY2010) without a check against its rental income. Do not raise the threshold: a mortgage REIT in a low-rate year can have less than 2%.

*Evidence:* `mapcheck values UMH`, `rust/mapcheck/src/mapper.rs` (`resolve_period_with`, the `unclassified_only` test), `financial_analysis/standard_variables.toml` (`investment_income_operating`).
