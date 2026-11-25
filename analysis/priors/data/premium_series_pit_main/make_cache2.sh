#!/usr/bin/env bash
# make_cache2.sh MODE D OUT   (MODE = trunc | poison). A one-time test script (shell, jq, awk, gzip).
# Builds a cache copy from the data store for the sp100 list and SPY: all data after D removed (trunc) or made false (poison).
# Options by environment:
#   GSW_CUT=YYYY-MM-DD  remove the GSW rows after this date in every mode (the par fit is then used after it)
#   DBASIS=1            with trunc: the cache on the share basis of D: the splits with an ex-date after D are removed from
#                       splits.csv, last_date of prices.meta.json is the last row on or before D, and each price on or
#                       before D is multiplied by the product of the factors of the removed splits of the ticker
set -euo pipefail
MODE=$1; D=$2; OUT=$3
SRC=${SRC:-/home/user/fa-data-split}
LIST=${LIST:-/home/user/wt-run/rust/mapcheck/universe/sp100.txt}
GSW_CUT=${GSW_CUT:-}; DBASIS=${DBASIS:-0}
D1=$(date -d "$D + 1 day" +%F)
rm -rf "$OUT"; mkdir -p "$OUT/rates" "$OUT/_corporate" "$OUT/prices"
if [ "$DBASIS" = 1 ]; then
  awk -F, -v d="$D" 'NR==1 || $2 <= d' "$SRC/_corporate/splits.csv" > "$OUT/_corporate/splits.csv"
else
  cp "$SRC/_corporate/splits.csv" "$OUT/_corporate/"
fi
cp "$SRC/_corporate/splits.meta.json" "$OUT/_corporate/" 2>/dev/null || true
G="$SRC/rates/gsw_feds200628.csv"
awk -F, -v d="$D" -v mode="$MODE" -v cut="$GSW_CUT" 'BEGIN{OFS=","}
  /^Date,/ {for(i=1;i<=NF;i++) h[i]=$i; print; next}
  $1 ~ /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/ {
    if (cut != "" && $1 > cut) next
    if ($1 > d) { if (mode=="trunc") next
      for(i=2;i<=NF;i++){ if(h[i] ~ /^(BETA1|BETA2|BETA3|TAU1|TAU2)$/) continue
        if($i != "NA" && $i != "") $i = sprintf("%.10g", $i + 5) } } }
  {print}' "$G" > "$OUT/rates/gsw_feds200628.csv"
for f in "$SRC"/rates/treasury_*.csv; do
  awk -F, -v d="$D" -v mode="$MODE" 'BEGIN{OFS=","}
    $1 ~ /^[0-9]{2}\/[0-9]{2}\/[0-9]{4}$/ { split($1,a,"/"); iso=a[3]"-"a[1]"-"a[2]
      if (iso > d) { if (mode=="trunc") next
        for(i=2;i<=NF;i++) if($i != "" && $i != "N/A") $i = sprintf("%.10g", $i + 5) } }
    {print}' "$f" > "$OUT/rates/$(basename "$f")"
done
for f in "$SRC"/prices/fedinvest_*.csv; do
  fd=$(basename "$f" .csv); fd=${fd#fedinvest_}
  if [ "$MODE" = trunc ] && [ "$fd" \> "$D" ]; then continue; fi
  cp "$f" "$OUT/prices/"
done
one() {
  t=$1; s="$SRC/$t"; o="$OUT/$t"; [ -d "$s" ] || { echo "no folder $t" >&2; return 0; }
  mkdir -p "$o"
  if [ "$MODE" = trunc ] && [ "$DBASIS" = 1 ]; then
    k=$(awk -F, -v t="$t" -v d="$D" 'NR>1 && $1==t && $2>d {p*=$5; n++} BEGIN{p=1} END{printf "%.10g", p}' "$SRC/_corporate/splits.csv")
    jq -c --arg d "$D" --argjson k "$k" 'map(select(.date <= $d) | .open*=$k | .high*=$k | .low*=$k | .close*=$k)' "$s/prices.json" > "$o/prices.json"
    last=$(jq -r 'last.date' "$o/prices.json"); n=$(jq 'length' "$o/prices.json")
    jq --arg l "$last" --argjson n "$n" '.last_date=$l | .rows=$n' "$s/prices.meta.json" > "$o/prices.meta.json"
  else
    cp "$s/prices.meta.json" "$o/"
    if [ "$MODE" = trunc ]; then jq -c --arg d "$D" 'map(select(.date <= $d))' "$s/prices.json" > "$o/prices.json"
    else jq -c --arg d "$D" 'map(if .date > $d then .close *= 10 else . end)' "$s/prices.json" > "$o/prices.json"; fi
  fi
  if [ -f "$s/company_facts.json.gz" ]; then
    if [ "$MODE" = trunc ]; then
      zcat "$s/company_facts.json.gz" | jq -c --arg d "$D" '.facts |= map_values(map_values(.units |= map_values(map(select(.filed <= $d)))))' | gzip -c > "$o/company_facts.json.gz"
    else
      zcat "$s/company_facts.json.gz" | jq -c --arg d "$D" '.facts |= map_values(map_values(.units |= map_values(map(if .filed > $d then .val *= 1000 else . end))))' | gzip -c > "$o/company_facts.json.gz"
    fi
  fi
  if [ -f "$s/sec_8k_earnings.json" ]; then
    if [ "$MODE" = trunc ]; then
      jq --arg d "$D" '.filings |= map(select(.filed <= $d)) | if (.latest_periodic.filed // "") > $d then del(.latest_periodic) else . end' "$s/sec_8k_earnings.json" > "$o/sec_8k_earnings.json"
    else
      jq --arg d "$D" --arg p "$D1" '.filings |= map(if .filed > $d then .filed = $p | .report_date = $p | .items = "2.02,9.01" else . end) | if (.latest_periodic.filed // "") > $d then .latest_periodic.filed = $p else . end' "$s/sec_8k_earnings.json" > "$o/sec_8k_earnings.json"
    fi
  fi
  if [ -d "$s/filings" ] && [ "$MODE" = poison ]; then cp -r "$s/filings" "$o/"; fi
}
export -f one; export SRC OUT MODE D D1 DBASIS
( grep -v '^#' "$LIST"; echo SPY ) | xargs -P 4 -I{} bash -c 'one {}'
echo "built $MODE $D in $OUT (GSW_CUT=$GSW_CUT DBASIS=$DBASIS)"
