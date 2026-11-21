# Cboe delayed quotes are not read

*Trap:* The Cboe page prohibits automatic downloads of the delayed quotes (2026-11-21). No command reads them. Old store files of Cboe pulls have no `source` field.

*What to do:* Use Alpaca for a new chain. Read an old file as a Cboe file when it has no `source`.

*Evidence:* `rust/src/market/options_cli.rs` (tests `the_source_cboe_is_refused_and_alpaca_is_the_default`, `the_option_chain_code_has_no_cboe_address`), ticket T-20261121-cboe-off.
