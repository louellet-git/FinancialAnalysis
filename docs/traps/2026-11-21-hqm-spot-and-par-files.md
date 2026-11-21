# FRED HQMCB*YR is the spot rate, and T10Y3M from the Treasury starts in 1990

*Trap:* The FRED series `HQMCB1YR` to `HQMCB30YR` are the Treasury HQM monthly average spot
rates, not the par yields. The Treasury par file (`hqm_qh_pars.xls`) has only 2, 5, 10 and 30
years, so it cannot replace the 8 maturities of `fa bonds hqm`. A reader who takes the par file
gets other numbers (Oct 2026, 10 years: spot 6.02, par 5.92).

*What to do:* Use the Treasury spot files (`hqm_84_88.xls` to `hqm_24_28.xls`). `fa bonds hqm get`
downloads them. The files give the date as a year block and a month column, not as a date.

*Evidence:* `rust/src/hqm.rs` (`parse_hqm_spot`, `compare_files`); `fa bonds hqm-compare` on the
nine files: 514 months and 8 maturities, all equal to the FRED files (2026-11-21).
