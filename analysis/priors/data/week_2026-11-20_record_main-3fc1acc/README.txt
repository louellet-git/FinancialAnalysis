# RUN OF RECORD of the base rates and of the reality check, week 2026-11-13 to 2026-11-20. Code: main 3fc1acc (pull
# request 199, after the gate of the Head of Quality and Risk). Store: fa-data 0340bc1. Commands:
#   fa dcf baserates --tickers-file rust/mapcheck/universe/sp500.txt --as-of 2026-11-20 --cache <store> --out baserates_sp500.csv
#   fa dcf attribute --tickers-file rust/mapcheck/universe/sp100.txt --top 10 --reality
#       --baserate-list rust/mapcheck/universe/sp500.txt --from 2026-11-13 --to 2026-11-20 --cache <store> --out sp10_reality.csv
# baserates_sp500.csv is byte-identical to analysis/priors/data/baserates_sp500_2026-11-20/baserates_sp500.csv.
# The bridge of sp10_reality.txt is the bridge of week_2026-11-20_record_main-7e88cfe/sp10.txt (-9.96%, the same blocks).
# The path of the cache in the two sp10_reality files is written as <cache>.
# NOT in a run of record: the weekly premium series of the list (1.20% and 1.67%), and the deviations and the expected
# fall by beta of pages 1 and 3 (one-time scripts in week_2026-11-20_chips/).
# The automatic model is a screen, not a reviewed valuation. An implied number is a measurement, not an input.
