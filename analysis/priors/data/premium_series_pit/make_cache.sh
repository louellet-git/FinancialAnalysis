#!/usr/bin/env bash
# make_cache.sh MODE D OUT   (MODE = trunc | poison)
# Builds a cache copy from the store for the sp100 list and SPY.
set -euo pipefail
MODE=$1; D=$2; OUT=$3
SRC=/home/user/fa-data-split
LIST=/home/user/wt-premium/rust/mapcheck/universe/sp100.txt
D1=$(date -d "$D + 1 day" +%F)          # the poison date for 8-K filings (the day after D)
rm -rf "$OUT"; mkdir -p "$OUT/rates" "$OUT/_corporate"
cp "$SRC/_corporate/splits.csv" "$SRC/_corporate/splits.meta.json" "$OUT/_corporate/"

# --- rates: GSW (YYYY-MM-DD in column 1, data rows start with a digit) ---
G="$SRC/rates/gsw_feds200628.csv"
if [ "$MODE" = trunc ]; then
  awk -F, -v d="$D" '!($1 ~ /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/ && $1 > d)' "$G" > "$OUT/rates/gsw_feds200628.csv"
else
  # +5 points on BETA0 and on every yield or rate column; BETA1..3 and TAU1..2 stay.
  awk -F, -v d="$D" 'BEGIN{OFS=","}
    /^Date,/ {for(i=1;i<=NF;i++) h[i]=$i; print; next}
    $1 ~ /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/ && $1 > d {
      for(i=2;i<=NF;i++){ if(h[i] ~ /^(BETA1|BETA2|BETA3|TAU1|TAU2)$/) continue;
        if($i != "NA" && $i != "") $i = sprintf("%.10g", $i + 5) }
      print; next }
    {print}' "$G" > "$OUT/rates/gsw_feds200628.csv"
fi
# --- rates: Treasury files (MM/DD/YYYY in column 1) ---
for f in "$SRC"/rates/treasury_*.csv; do
  b=$(basename "$f")
  awk -F, -v d="$D" -v mode="$MODE" 'BEGIN{OFS=","}
    $1 ~ /^[0-9]{2}\/[0-9]{2}\/[0-9]{4}$/ {
      split($1,a,"/"); iso=a[3]"-"a[1]"-"a[2];
      if (iso > d) { if (mode=="trunc") next;
        for(i=2;i<=NF;i++) if($i != "" && $i != "N/A") $i = sprintf("%.10g", $i + 5) } }
    {print}' "$f" > "$OUT/rates/$b"
done
cp "$SRC/rates/bls_cpi.json" "$OUT/rates/" 2>/dev/null || true
[ "$MODE" = trunc ] && rm -f "$OUT/rates/bls_cpi.json"   # not read; present data, removed in the cut
# --- FedInvest files (prices/fedinvest_YYYY-MM-DD.csv) ---
mkdir -p "$OUT/prices"
for f in "$SRC"/prices/fedinvest_*.csv; do
  fd=$(basename "$f" .csv); fd=${fd#fedinvest_}
  if [ "$MODE" = trunc ]; then [ "$fd" \> "$D" ] && continue; fi
  cp "$f" "$OUT/prices/"
done

# --- ticker folders ---
for t in $(grep -v '^#' "$LIST") SPY; do
  s="$SRC/$t"; o="$OUT/$t"; [ -d "$s" ] || { echo "no folder $t" >&2; continue; }
  mkdir -p "$o"
  cp "$s/prices.meta.json" "$o/"        # kept AS IT IS (the split basis reads its last_date)
  if [ "$MODE" = trunc ]; then
    jq -c --arg d "$D" 'map(select(.date <= $d))' "$s/prices.json" > "$o/prices.json"
  else
    jq -c --arg d "$D" 'map(if .date > $d then .close *= 10 else . end)' "$s/prices.json" > "$o/prices.json"
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
      jq --arg d "$D" '.filings |= map(select(.filed <= $d))
        | if (.latest_periodic.filed // "") > $d then del(.latest_periodic) else . end' "$s/sec_8k_earnings.json" > "$o/sec_8k_earnings.json"
    else
      jq --arg d "$D" --arg p "$D1" '.filings |= map(if .filed > $d then .filed = $p | .report_date = $p | .items = "2.02,9.01" else . end)
        | if (.latest_periodic.filed // "") > $d then .latest_periodic.filed = $p else . end' "$s/sec_8k_earnings.json" > "$o/sec_8k_earnings.json"
    fi
  fi
  # 10-K filing folders: every one in the list is filed after all three test dates (accession years 25 and 26,
  # the 25 ones filed 2025-12-18). The cut removes them; the poison copy keeps them unchanged.
  if [ -d "$s/filings" ] && [ "$MODE" = poison ]; then cp -r "$s/filings" "$o/"; fi
  [ -f "$s/splits.csv" ] && cp "$s/splits.csv" "$s/splits.meta.json" "$o/" 2>/dev/null || true
done
echo "built $MODE $D in $OUT"
