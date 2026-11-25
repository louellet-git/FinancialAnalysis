# RUN OF RECORD of the weekly implied premium (S1) and of the weekly bridge (S2) of the S&P 500 list, 2019-01-04 to
# 2026-11-20, 412 weeks. For the frozen test of the Head of Economics (its rule 7.2 needs s2_rates).
# PARAMETERS IN-SAMPLE: the parameter table (process_priors_2026-11-19.toml) is fitted on all years of the cache; it is
# not cut. The data of each row are point in time (tests: ../premium_series_pit_main/result.md); the parameters are not.
# Code: main 6c59388 (after pull request 213: the stale-price rule runs in the bridge). Store: fa-data 0340bc1.
# Command: fa dcf premium-series --tickers-file rust/mapcheck/universe/sp500.txt --start 2019-01-04 --end 2026-11-20
#          --cache <store> --bridge --out sp500_bridge.csv        (74 minutes on 4 threads; no warning)
# Header: friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,
#   s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change (decimals; an empty field is missing; the first row has no s2).
# Checks: the columns 1 to 10 are identical to ../premium_series_sp500_2019_record_main-e2557b6/sp500_s1.csv (412 rows);
# the seven s2 parts add to s2_change in each row (largest difference 1e-16); the week to 2026-11-20 has s2_change
# -0.0858, the list change of `fa dcf attribute` for that week; the correlation of s2_rates with the weekly change of
# zero10 is -0.85 (411 rows, with the switch week).
# SWITCH WEEK of the curve source: 2026-11-20 (the first row with the par fit). Its s2_rates and s2_premium mix the GSW
# curve at the previous anchor and the par curve at the current anchor (docs/models/premium-series.md, Limits).
# Limits: a list of today's names (survivorship); a ticker with the mapping status FAIL at an anchor has no value there;
# the automatic model is a screen, not a reviewed valuation. An implied number is a measurement, not an input.
