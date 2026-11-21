# Runs for the chip piece (week 2026-11-13 to 2026-11-20). Binary of claude/attribution-reality, store 0340bc1, --curve auto.
# sp500.csv: fa dcf attribute --tickers-file rust/mapcheck/universe/sp500.txt --from 2026-11-13 --to 2026-11-20 --cache <store> --out sp500.csv
# chips.txt: the 16 tickers of sp500.csv with the industry code 3674. chips.csv, chips_run.txt: the same command on chips.txt.
# chips.js: a one-time calculation (node) of the expected fall by beta, the deviations and the correlations; it reads the store prices.
# deviations_sp10.js: the one-time calculation of the deviations of the S&P 10 pages.
# Not reviewed. SWKS has no close of 2026-11-20 in the store (stale price) and is left out of the sums.
