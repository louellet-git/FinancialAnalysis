# The Pink Sheet link changes each month and the gold column has two letters

*Trap:* The address of `CMO-Historical-Data-Monthly.xlsx` has a hash that changes with each monthly
update (the 2026-11-03 file: `74e8be41...-0050012026`). A stored address stops working. The gold
column is `BR`; the reader of the Atlanta Fed file (`futures::mpt::cells`) keeps one letter only and
reads column `B` (crude oil). The first sheet of the file is hidden ("Mismatch Details"), so
"the first sheet" is not the price sheet. The file has "…" in a cell with no value.

*What to do:* Read the link from the commodity page each time. Find the sheet by name and the column
by its title and unit, as `read_series` does. Do not use `cells` for a wide sheet.

*Evidence:* `rust/src/market/commodity.rs` (`file_link`, `row_cells`, tests
`gold_column_two_letters_and_missing_cell`, `link_has_a_hash_and_the_host_is_known`); real run 2026-11-26.
