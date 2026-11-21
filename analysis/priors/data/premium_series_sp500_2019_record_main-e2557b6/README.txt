# RUN OF RECORD of the weekly implied premium of the S&P 500 list (S1), 2019-01-04 to 2026-11-20, 412 weeks.
# PARAMETERS IN-SAMPLE: the parameter table (process_priors_2026-11-19.toml) is fitted on all years of the cache; it is
# not cut. The data of each row are point in time (tests: ../premium_series_pit_main/result.md, on the S&P 100 list);
# the parameters are not.
# Code: main e2557b6 (rust/ identical on main c4fc285; after pull requests 190, 199 and 200). Store: fa-data 0340bc1.
# Command: fa dcf premium-series --tickers-file rust/mapcheck/universe/sp500.txt --start 2019-01-04 --end 2026-11-20
#          --cache <store> --out sp500_s1.csv        (no --bridge: the s2 columns are empty; 46 minutes on 4 threads)
# Header (the contract with the Head of Economics): friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,
#   ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change (decimals; empty = missing).
# Result: median 4.19%; highest 7.77% (2020-03-20); lowest 2.50% (2026-11-13); 2026-11-20: 3.14%, and 94.9% of the 411
# weeks before it are higher (21 are lower). Tickers with a value: 360 to 430 of 503.
# SWITCH WEEK of the curve source: 2026-11-20 (the first row with the par fit; all rows before it use GSW). The step of
# zero10 and of s1 in that week mixes two curve fits (docs/models/premium-series.md, Limits).
# Limits: a list of today's names (survivorship); a ticker with the mapping status FAIL at an anchor has no value there;
# S1 sets the median of value / price to 1 by design; the automatic model is a screen, not a reviewed valuation.
# An implied number is a measurement, not an input.
