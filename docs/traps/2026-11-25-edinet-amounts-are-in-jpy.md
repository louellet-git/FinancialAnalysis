# EDINET amounts are in JPY, in the full unit, with no conversion

*Trap:* An EDINET report gives each amount in yen, not in thousands or millions (the CSV form holds the value of the fact). ITOCHU revenue for the year ended 2025-03-31 is 14,724,234,000,000 yen. The standard variables of `8001.T` are in JPY. A model that reads them as USD is wrong by a factor of about 150. The EDINET periods have the form label `EDINET-120` and the currency `JPY`; `fa` refuses them (`Company::usd_only`), and its annual filter reads form `10-K` only.

*What to do:* Do not convert in `mapcheck`. A conversion needs a dated rate (Fed H.10) and its own ticket, after the first real read. The per-share and share-count variables need care: ITOCHU made a five-for-one split on 2026-01-01, and its 2026 report restates the earlier per-share numbers.

*Evidence:* `rust/mapcheck/src/edinet.rs` (test `the_standard_variables_come_from_the_facts_in_jpy`); `rust/src/company.rs` (test `an_edinet_period_in_jpy_is_refused_on_the_import_path`).
