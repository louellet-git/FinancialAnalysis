#!/usr/bin/env bash
# run_all.sh: the point-in-time tests of `fa dcf premium-series` on the code of main. A one-time test driver.
set -uo pipefail
W=$(cd "$(dirname "$0")" && pwd); cd "$W"
SRC=/home/user/fa-data-split; B=/home/user/wt-run/rust/target/release/fa
LIST=/home/user/wt-run/rust/mapcheck/universe/sp100.txt
mkdir -p out caches; : > cmp.txt
run() { # cache start end name
  (cd /home/user/wt-run && $B dcf premium-series --tickers-file $LIST --start $2 --end $3 --cache $(case $1 in /*) echo $1;; *) echo $W/$1;; esac) --bridge --per-ticker $W/out/$4.tickers.csv --out $W/out/$4.csv > $W/out/$4.log 2>&1) || echo "RUN FAILED $4" >> cmp.txt
}
same() { # name-a name-b expect(identical|differs) label
  if [ ! -s out/$1.csv ] || [ ! -s out/$2.csv ]; then echo "$4: a file is missing: FAIL" >> cmp.txt; return; fi
  if cmp -s out/$1.csv out/$2.csv; then r=identical; else r=differs; fi
  echo "$4: $1.csv against $2.csv: $r (expected: $3) $([ "$r" = "$3" ] && echo PASS || echo FAIL)" >> cmp.txt
}
link_cache() { # out: a cache of symbolic links to the store, with its own rates folder
  rm -rf "$1"; mkdir -p "$1/rates"
  for e in "$SRC"/*; do b=$(basename "$e"); [ "$b" = rates ] || ln -s "$e" "$1/$b"; done
  cp "$SRC"/rates/* "$1/rates/"
}
# 1. Five anchors: truncation and poison against the store.
for a in "2020-03-13 2020-03-20" "2022-06-10 2022-06-17" "2025-03-28 2025-04-04" "2020-04-03 2020-04-10" "2022-07-22 2022-07-29"; do
  set -- $a; S=$1; D=$2
  [ -n "${SKIP_BUILD:-}" ] || ./make_cache2.sh trunc $D caches/trunc_$D >> build.log 2>&1
  [ -n "${SKIP_BUILD:-}" ] || ./make_cache2.sh poison $D caches/poison_$D >> build.log 2>&1
  run $SRC $S $D base_$D; run caches/trunc_$D $S $D trunc_$D; run caches/poison_$D $S $D poison_$D
  same base_$D trunc_$D identical "truncation $D"; same base_$D poison_$D identical "poison $D"
done
# 2. The share basis of D (2020-03-20: AAPL and TSLA split on 2020-08-31, and 2 or more later splits in the list).
[ -n "${SKIP_BUILD:-}" ] || DBASIS=1 ./make_cache2.sh trunc 2020-03-20 caches/dbasis_2020-03-20 >> build.log 2>&1
run caches/dbasis_2020-03-20 2020-03-13 2020-03-20 dbasis_2020-03-20
same base_2020-03-20 dbasis_2020-03-20 identical "share basis of D 2020-03-20 (byte test)"
# 3. The par path: the GSW file ends on 2025-03-21, so 2025-03-28 (the switch week) and 2025-04-04 use the par fit.
D=2025-04-04; S=2025-03-21; CUT=2025-03-21
link_cache caches/parbase; awk -F, -v c=$CUT '!($1 ~ /^[0-9]{4}-/ && $1 > c)' $SRC/rates/gsw_feds200628.csv > caches/parbase/rates/gsw_feds200628.csv
[ -n "${SKIP_BUILD:-}" ] || GSW_CUT=$CUT ./make_cache2.sh trunc $D caches/partrunc >> build.log 2>&1
[ -n "${SKIP_BUILD:-}" ] || GSW_CUT=$CUT ./make_cache2.sh poison $D caches/parpoison >> build.log 2>&1
run caches/parbase $S $D par_base; run caches/partrunc $S $D par_trunc; run caches/parpoison $S $D par_poison
same par_base par_trunc identical "par path truncation $D"; same par_base par_poison identical "par path poison $D"
# 4. Controls: a false value ON or BEFORE D must change the output (the copy reaches the engine), one path each.
# 4a. Treasury par rows of the 10 days to D, +5 points (par path).
link_cache caches/c_par; cp caches/parbase/rates/gsw_feds200628.csv caches/c_par/rates/
for f in $SRC/rates/treasury_*.csv; do awk -F, -v d=$D 'BEGIN{OFS=","} $1 ~ /^[0-9]{2}\/[0-9]{2}\/[0-9]{4}$/ { split($1,a,"/"); iso=a[3]"-"a[1]"-"a[2]; if (iso <= d && iso > "2025-03-25") for(i=2;i<=NF;i++) if($i != "" && $i != "N/A") $i = sprintf("%.10g", $i + 5) } {print}' $f > caches/c_par/rates/$(basename $f); done
run caches/c_par $S $D c_par; same par_base c_par differs "control: Treasury rows before D (par path)"
# 4b. GSW rows of the 10 days to D, +5 points (GSW path, 2025-04-04).
link_cache caches/c_gsw; awk -F, 'BEGIN{OFS=","} /^Date,/ {for(i=1;i<=NF;i++) h[i]=$i; print; next} $1 ~ /^[0-9]{4}-/ && $1 <= "2025-04-04" && $1 > "2025-03-25" { for(i=2;i<=NF;i++){ if(h[i] ~ /^(BETA1|BETA2|BETA3|TAU1|TAU2)$/) continue; if($i != "NA" && $i != "") $i = sprintf("%.10g", $i + 5) } } {print}' $SRC/rates/gsw_feds200628.csv > caches/c_gsw/rates/gsw_feds200628.csv
run caches/c_gsw 2025-03-28 2025-04-04 c_gsw; same base_2025-04-04 c_gsw differs "control: GSW rows before D"
# 4c. Facts filed in the 400 days to D, times 1000, for 5 tickers.
link_cache caches/c_facts
for t in AAPL MSFT JPM XOM WMT; do rm caches/c_facts/$t; mkdir caches/c_facts/$t; for e in $SRC/$t/*; do ln -s $e caches/c_facts/$t/; done; rm caches/c_facts/$t/company_facts.json.gz
  zcat $SRC/$t/company_facts.json.gz | jq -c '.facts |= map_values(map_values(.units |= map_values(map(if .filed <= "2025-04-04" and .filed > "2024-03-01" then .val *= 1000 else . end))))' | gzip -c > caches/c_facts/$t/company_facts.json.gz; done
run caches/c_facts 2025-03-28 2025-04-04 c_facts; same base_2025-04-04 c_facts differs "control: facts before D"
# 4d. Closes of the 5 days to D, times 2, for 5 tickers.
link_cache caches/c_close
for t in AAPL MSFT JPM XOM WMT; do rm caches/c_close/$t; mkdir caches/c_close/$t; for e in $SRC/$t/*; do ln -s $e caches/c_close/$t/; done; rm caches/c_close/$t/prices.json
  jq -c 'map(if .date <= "2025-04-04" and .date > "2025-03-30" then .close *= 2 else . end)' $SRC/$t/prices.json > caches/c_close/$t/prices.json; done
run caches/c_close 2025-03-28 2025-04-04 c_close; same base_2025-04-04 c_close differs "control: closes before D"
# 4e. SPY: the row of D removed (the anchor of the Friday moves).
link_cache caches/c_spy; rm caches/c_spy/SPY; mkdir caches/c_spy/SPY; for e in $SRC/SPY/*; do ln -s $e caches/c_spy/SPY/; done; rm caches/c_spy/SPY/prices.json
jq -c 'map(select(.date != "2025-04-04"))' $SRC/SPY/prices.json > caches/c_spy/SPY/prices.json
run caches/c_spy 2025-03-28 2025-04-04 c_spy; same base_2025-04-04 c_spy differs "control: SPY row of D removed"
# 4f. The stale rule: the close of D removed for 5 tickers (their close is then older than the SPY close).
link_cache caches/c_stale
for t in AAPL MSFT JPM XOM WMT; do rm caches/c_stale/$t; mkdir caches/c_stale/$t; for e in $SRC/$t/*; do ln -s $e caches/c_stale/$t/; done; rm caches/c_stale/$t/prices.json
  jq -c 'map(select(.date != "2025-04-04"))' $SRC/$t/prices.json > caches/c_stale/$t/prices.json; done
run caches/c_stale 2025-03-28 2025-04-04 c_stale; same base_2025-04-04 c_stale differs "control: stale close at D (5 tickers)"
echo DONE >> cmp.txt
