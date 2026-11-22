# RUN OF RECORD of the weekly implied premium of the S&P 100 list (S1), 2019-01-04 to 2026-11-20, 412 weeks.
# PARAMETERS IN-SAMPLE: the parameter table (process_priors_2026-11-19.toml) is fitted on all years of the cache; it is
# not cut. The data of each row are point in time (tests: ../premium_series_pit_main/result.md); the parameters are not.
# Code: main e2557b6 (rust/ identical on main c4fc285; after pull requests 190, 199 and 200). Store: fa-data 0340bc1.
# Command: fa dcf premium-series --tickers-file rust/mapcheck/universe/sp100.txt --start 2019-01-04 --end 2026-11-20
#          --cache <store> --out sp100_s1.csv        (no --bridge: the s2 columns are empty)
# Result: median 3.56%; highest 6.67% (2020-03-20); lowest 1.20% (2026-11-13); 2026-11-20: 1.67%, and 98.1% of the 411
# weeks before it are higher (8 are lower). Switch week of the curve source: 2026-11-20 (the par fit).
# Against the earlier file (../premium_series_sp100_2019_not-of-record/sp100_s1.csv): 20 rows of 2019 differ, because of
# the fix of the old revenue tags (pull request 200); the values of 2026-11-13 and 2026-11-20 are the same.
# Limits: a list of today's names (survivorship); S1 sets the median of value / price to 1 by design; the automatic model
# is a screen, not a reviewed valuation. An implied number is a measurement, not an input.
