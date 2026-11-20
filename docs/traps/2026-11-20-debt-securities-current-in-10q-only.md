# NVDA has the current investments in a company tag in the 10-K

*Trap:* For the year end 2026-01-25, NVDA reports the current investments in the 10-K with the company tag `nvda_MarketableSecuritiesAndEquitySecuritiesFVNI` (51.951B, with equity securities). The standard tag `DebtSecuritiesCurrent` (39.065B for the same date) exists only in the 10-Q filings. The annual row of fiscal 2026 has no `short_term_investments`. The quarterly rows 2026-04-26 and 2026-07-26 have it (37.098B, 34.143B).

*What to do:* Use the quarterly row for the net debt of NVDA after 2026-01-25. Do not use cash alone (10.6B at the year end). Do not add the company tag to the registry: it includes equity securities.

*Evidence:* `mapcheck values NVDA`, `mapcheck filing NVDA --statement balance`, `financial_analysis/standard_variables.toml` (`short_term_investments`, last tag).
