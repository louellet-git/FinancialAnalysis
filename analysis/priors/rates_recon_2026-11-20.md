# Rates reconciliation, step 1 (2026-11-20)

Ticket `T-20261120-arb-2-rates-recon`, step 1. This file gives the difference between the GSW curve and the Treasury
par curve, and between the corporate series (HQM, ICE BofA) and the Treasury zero curve plus the spread. It uses the
files of the data store. It checks data and conventions. It is not a trade signal.

**Rule of the ticket.** A difference between two indexes or curves is not an arbitrage. No stock price is an input.
Step 2 (FedInvest, bill rates, New York Fed) is not done.

## 1. Question
Do GSW and the Treasury par curve agree to the rounding of the sources? Do HQM and ICE BofA agree with a Treasury zero
curve plus the ICE OAS? If not, which convention or error explains the difference?

## 2. Data and commands
Differences are in basis points (bp). Cache `data/raw` of 2026-11-20, read only, with
`FA_REPO=/home/user/FinancialAnalysis`. GSW and the Treasury par file overlap on 218 days (2026-01-02 to 2026-11-13).
HQM ends at the October 2026 average. The ICE series start on 2023-11-20.

- `fa forwards check`: it downloads the Treasury CSV of 2026 and compares the last 30 days of GSW. It lists only a
  difference above 25 bp. Result: 8 tenors, 22 days each, 0 mismatches.
- The per-date difference is not in that output. I made it with `awk` over `rates/gsw_feds200628.csv` (`SVENPY01` to
  `SVENPY30`) and `rates/treasury_nominal_2026.csv` (`1 Yr` to `30 Yr`). Difference = GSW minus Treasury.
- `fa recon bonds --date 2026-11-18`, last block (HQM against our zero curve plus the mean OAS). No command runs this
  check without FedInvest prices (D3).
- Corporate series: `awk` over `bonds/fred_HQMCB*YR.csv`, `fred_BAMLC0A1CAAA.csv`, `fred_BAMLC0A2CAA.csv`,
  `fred_BAMLC0A3CA.csv`, the `*EY.csv` files, and `SVENY01` to `SVENY30` of GSW (continuous zero), changed to
  semi-annual with `2*(exp(r/2)-1)`. `awk` rounds intermediate values to 0.1 bp, so a value can differ by 0.1 bp from the exact value. Std dev = population std dev.

## 3. Evidence
### 3.1 GSW par yield minus Treasury par yield (bp), 218 days
| Tenor | Mean | Std dev | Largest (date) | Mean, last 22 days |
|---|---|---|---|---|
| 1Y | +8.5 | 2.6 | +17.8 (2026-06-08) | +6.7 |
| 2Y | -0.6 | 2.8 | -6.4 (2026-05-19) | +0.8 |
| 3Y | +0.1 | 2.0 | +4.7 (2026-10-06) | +0.7 |
| 5Y | +2.1 | 1.4 | +5.6 (2026-03-26) | +1.3 |
| 7Y | +1.0 | 2.3 | +5.1 (2026-11-02) | -0.4 |
| 10Y | +5.2 | 2.1 | +8.9 (2026-05-15) | +2.5 |
| 20Y | -2.5 | 1.1 | -5.9 (2026-09-04) | -2.4 |
| 30Y | +4.9 | 2.2 | +12.3 (2026-11-06) | +4.0 |

The largest value of all is 17.8 bp (1Y). In the last 22 days the largest is 12.3 bp (30Y), so `fa forwards check`
(limit 25 bp) agrees. The Treasury file has 2 decimals of a percent (rounding up to 0.5 bp); GSW has 4.

### 3.2 HQM spot minus (GSW zero + mean OAS of AAA, AA, A), monthly means, 2026-01 to 2026-10
The mean OAS is 52.7 bp (47.4 bp in January, 58.4 bp in October). Ten months.

| Tenor | HQM minus GSW zero | Residual after OAS, mean | Std dev | Residual 2026-10 |
|---|---|---|---|---|
| 1Y | +24.1 | -28.6 | 5.6 | -32.1 |
| 2Y | +40.0 | -12.7 | 4.2 | -18.2 |
| 3Y | +47.1 | -5.6 | 2.5 | -9.2 |
| 5Y | +52.7 | 0.0 | 1.9 | +0.8 |
| 7Y | +61.0 | +8.3 | 2.5 | +12.2 |
| 10Y | +73.0 | +20.3 | 2.1 | +24.2 |
| 20Y | +80.2 | +27.6 | 2.6 | +21.4 |
| 30Y | +103.6 | +50.9 | 7.6 | +64.2 |

One run, not reproducible, different date. `fa recon bonds --date 2026-11-18` downloaded FedInvest by mistake (D5); its files
are not in the store. It compares one day (2026-11-18) with the October HQM. Our zero curve gave: 1Y -45.9, 2Y -26.6, 3Y -11.8,
5Y +0.8, 7Y +12.1, 10Y +28.2, 20Y +22.1, 30Y +70.1 bp. Difference from the October column: 0 to 14 bp (largest 1Y 13.8,
2Y 8.4). The mean OAS is 63.7 bp on 2026-11-18 and 58.4 bp for October, so about 5 bp of the match is a date effect.

### 3.3 ICE effective yield minus OAS, against Treasury par, daily, 221 days to 2026-11-18
Cell = mean of (EY - OAS - Treasury par of the tenor) in bp, std dev in brackets.

| Rating | Mean OAS | vs 3Y | vs 5Y | vs 7Y | vs 10Y |
|---|---|---|---|---|---|
| AAA | 39 | +60.2 (13.8) | +49.8 (10.6) | +35.0 (6.5) | +18.5 (3.5) |
| AA | 54 | +38.3 (10.7) | +28.0 (7.4) | +13.1 (3.7) | -3.4 (3.3) |
| A | 67 | +32.1 (11.2) | +21.8 (7.9) | +7.0 (4.1) | -9.6 (3.2) |
| BBB | 101 | +32.2 (9.9) | +21.9 (6.6) | +7.1 (2.9) | -9.5 (3.7) |
| BB | 174 | +10.8 (5.5) | +0.5 (2.5) | -14.4 (3.8) | -30.9 (8.1) |
| B | 309 | +7.5 (4.1) | -2.9 (2.3) | -17.7 (5.4) | -34.2 (9.9) |
| CCC | 1012 | +1.6 (6.7) | -8.8 (5.1) | -23.6 (5.5) | -40.1 (8.7) |

The implied benchmark is not one Treasury tenor. The tenor with the smallest std dev gets shorter as the rating
falls: 10Y for AAA and AA, 7Y to 10Y for A and BBB, 5Y for BB, B and CCC. This test is weak: for AA the std dev is 3.3 at
10Y against 3.7 at 7Y, so the tenor is not fixed.

## 4. What explains each difference
1. **GSW, 1Y (+8.5 bp).** Hypothesis, not tested: the Treasury 1Y point comes from bills (bill basis). GSW uses coupon bonds only
   and gives a coupon-equivalent yield. The bill file is needed to test it (step 2).
2. **GSW, 10Y and 30Y (+5 bp), 20Y (-2.5 bp).** GSW is a smooth six-parameter Svensson curve. The Treasury curve
   uses quoted on-the-run prices at fixed points. Hypothesis, not tested: a smooth curve does not follow a kink at the 20Y point (the 20Y bias is a stable -2.5 bp, std dev 1.1).
   Hypothesis, not tested: fit form. Neither source is shown to be wrong (hypothesis, not tested).
3. **Rounding.** Up to 0.5 bp from the 2 decimals of the Treasury file. Small against the std dev of 1.1 to 2.8 bp.
4. **HQM, from -29 bp (1Y) to +51 bp (30Y).** The test adds one flat spread (53 bp) to all tenors. The HQM spread to
   the zero curve is 24 bp at 1Y and 104 bp at 30Y. The ICE OAS is a mean over an index of mixed maturity, so it fits
   the middle (5Y) and not the ends. Convention (index against tenor). Other causes that I did not isolate: a
   different bond set, an OAS over a spot curve that removes option value, a monthly average against a daily curve.
5. **ICE EY - OAS.** The effective yield is the yield of an index with its own duration; the OAS is a spread over
   the spot curve. A longer-duration index (AAA) matches a longer Treasury tenor, as seen. Index duration is not in the
   store, so this is not confirmed. Open.

## 5. Defects
D1. **No per-date output.** `fa forwards check` lists only differences above 25 bp. A mean bias of 8.5 bp at 1Y is
not visible. Correction (not done): print the mean, std dev and largest value for each tenor. Test: a sample with a
constant 10 bp bias gives 0 mismatches before and a 10 bp mean after.
D2. **The check reads the network.** It downloads the Treasury CSV and ignores `rates/treasury_nominal_2026.csv`. It
cannot test a past window that the store holds. Correction: read the store first, then the network.
D3. **The curve-level check needs FedInvest.** `recon::hqm_rows` needs a bootstrapped curve. The only command
(`fa recon bonds`) downloads FedInvest prices. Missing code: a command that bootstraps a zero curve from the par file
for any store date and calls `hqm_rows`. The 3.2 test could then run on every date with our own curve.
D4. **No ICE check exists.** Missing code: a check by rating and tenor, with the index duration as input.
D5. **Side effect of my run (my error).** `fa recon bonds --date 2026-11-18` downloaded FedInvest prices and wrote
three files into the data folder of the other checkout: `data/raw/prices/fedinvest_2026-11-18.csv`,
`data/recon/bonds_2026-11-18.csv`, `data/recon/bonds_2026-11-18_curve.csv`. The ticket allowed only the Treasury CSV.
I did not know that the command downloads. The permission system refused my delete. The orchestrator must remove them
(the folders `data/raw/prices` and `data/recon` did not exist before). The second source in 3.2 comes from this run.

## 6. Conventions and open points
GSW par is coupon-equivalent, GSW zero is continuous, HQM and ICE yields are semi-annual. I did not read the HQM and ICE definitions (no page fetched): this is from the data and general knowledge, not confirmed. The owner decides; no assumption record. Open, with owner:
- 1Y bill basis: Quant, with the bill file of step 2. ICE index duration: Quant (a new source: ask first).
- Zero curve from the par file for all store dates (D3), and removal of the D5 files: Orchestrator.

## 7. Limits
- One year of Treasury data (2026), 218 days. No other year or regime is tested. The GSW zero is a proxy for our zero
  curve. HQM has 10 monthly points: the std devs are weak.
- Not done: FedInvest prices (except the one run of D5), bills, New York Fed, rich and cheap, persistence, bid-ask
  cost, repo and borrow. Nothing here is a trade candidate. No unit root test: the differences are levels, not a fit.
- The numbers come from `awk` scratch scripts. The auditor must run them again.

## A. Commands

Run from a folder of a checkout that has `data/raw` (a worktree has none; use `/home/user/FinancialAnalysis`). Bash. Output is in the order of 3.1, 3.2, 3.3;
the script was run again and gave the table numbers (the 1Y std dev in 3.2 prints 5.7 after the 0.1 bp rounding; the exact value is 5.6). Two `fa` lines are not in the script:
`FA_REPO=/home/user/FinancialAnalysis rust/target/release/fa forwards check` (section 2, network) and `fa recon bonds --date 2026-11-18` (one run, not reproducible: it downloads, and its files are not in the store).

```bash
cd "$(git rev-parse --show-toplevel)"
R=data/raw/rates; B=data/raw/bonds; T=$(mktemp -d)
# A1. GSW par and Treasury par (3.1)
awk -F, '/^Date,BETA0/{for(i=1;i<=NF;i++)h[$i]=i;hd=1;next} hd&&$1>="2026-01-01"{printf "%s",$1;split("01 02 03 05 07 10 20 30",t," ");for(k=1;k<=8;k++)printf ",%s",$h["SVENPY" t[k]];print ""}' $R/gsw_feds200628.csv | sort > $T/gsw.csv
awk -F, 'NR>1{split($1,a,"/");print a[3]"-"a[1]"-"a[2]","$8","$9","$10","$11","$12","$13","$14","$15}' $R/treasury_nominal_2026.csv | sort > $T/tr.csv
join -t, $T/gsw.csv $T/tr.csv | grep -v NA > $T/j.csv
cat > $T/stat.awk <<'E'
BEGIN{split("1Y 2Y 3Y 5Y 7Y 10Y 20Y 30Y",n," ")}
{for(k=1;k<=8;k++){d=($(k+1)-$(k+9))*100;s[k]+=d;q[k]+=d*d;d2=d<0?-d:d;if(d2>m[k]){m[k]=d2;ms[k]=d;md[k]=$1};c[k]++}}
END{for(k=1;k<=8;k++)printf "%s n=%d mean=%.2f sd=%.2f max=%.1f on %s\n",n[k],c[k],s[k]/c[k],sqrt(q[k]/c[k]-(s[k]/c[k])^2),ms[k],md[k]}
E
echo "== 3.1 all days"; awk -F, -f $T/stat.awk $T/j.csv
echo "== 3.1 last 22 days"; awk -F, '$1>="2026-10-14"' $T/j.csv | awk -F, -f $T/stat.awk
# A2. HQM against GSW zero + OAS (3.2)
awk -F, '/^Date,BETA0/{for(i=1;i<=NF;i++)h[$i]=i;hd=1;next} hd&&$1>="2026-01-01"&&$h["SVENY01"]!="NA"{m=substr($1,1,7);split("01 02 03 05 07 10 20 30",t," ");for(k=1;k<=8;k++){r=$h["SVENY" t[k]]/100;s[m,k]+=2*(exp(r/2)-1)*1e4};c[m]++}END{for(m in c){printf "%s",m;for(k=1;k<=8;k++)printf " %.1f",s[m,k]/c[m];print ""}}' $R/gsw_feds200628.csv | sort > $T/gz.txt
for n in BAMLC0A1CAAA BAMLC0A2CAA BAMLC0A3CA; do awk -F, 'NR>1&&$2!=""&&$1>="2026-01-01"{m=substr($1,1,7);s[m]+=$2*100;c[m]++}END{for(m in s)print m,s[m]/c[m]}' $B/fred_$n.csv | sort > $T/o_$n.txt; done
paste -d' ' $T/o_BAMLC0A1CAAA.txt $T/o_BAMLC0A2CAA.txt $T/o_BAMLC0A3CA.txt | awk '{print $1,($2+$4+$6)/3}' > $T/oas.txt
for t in 1 2 3 5 7 10 20 30; do awk -F, -v t=$t 'NR>1&&$1>="2026-01-01"{printf "%s %.1f\n",substr($1,1,7),$2*100}' $B/fred_HQMCB${t}YR.csv > $T/h$t.txt; done
paste -d' ' $T/h1.txt $T/h2.txt $T/h3.txt $T/h5.txt $T/h7.txt $T/h10.txt $T/h20.txt $T/h30.txt | awk '{printf "%s",$1;for(i=2;i<=NF;i+=2)printf " %s",$i;print ""}' > $T/hq.txt
join $T/gz.txt $T/hq.txt | join - $T/oas.txt > $T/m.txt
echo "== 3.2 (spread = HQM - GSW zero; resid = spread - mean OAS)"
awk 'BEGIN{split("1Y 2Y 3Y 5Y 7Y 10Y 20Y 30Y",n," ")}{for(k=1;k<=8;k++){sp=$(9+k)-$(1+k);r=sp-$18;S[k]+=sp;R[k]+=r;Q[k]+=r*r;if($1=="2026-10"){ro[k]=r}};c++;oa+=$18}END{for(k=1;k<=8;k++)printf "%s months=%d spread_mean=%.1f resid_mean=%.1f resid_sd=%.1f oct_resid=%.1f\n",n[k],c,S[k]/c,R[k]/c,sqrt(Q[k]/c-(R[k]/c)^2),ro[k];print "mean OAS",oa/c}' $T/m.txt
awk '$1=="2026-01"||$1=="2026-10"{print "OAS",$1,$2}' $T/oas.txt
# A3. ICE EY - OAS against Treasury par (3.3)
awk -F, '{print $1","$4","$5","$6","$7}' $T/tr.csv | sort > $T/trp.csv
echo "== 3.3"
for g in AAA:BAMLC0A1CAAA AA:BAMLC0A2CAA A:BAMLC0A3CA BBB:BAMLC0A4CBBB BB:BAMLH0A1HYBB B:BAMLH0A2HYB CCC:BAMLH0A3HYC; do r=${g%%:*}; n=${g#*:}
 join -t, <(awk -F, 'NR>1&&$2!=""&&$1>="2026-01-01"{print $1","$2}' $B/fred_${n}EY.csv|sort) <(awk -F, 'NR>1&&$2!=""&&$1>="2026-01-01"{print $1","$2}' $B/fred_$n.csv|sort) | join -t, - $T/trp.csv > $T/ice_$r.csv
 awk -F, -v r=$r '{b=$2-$3;for(k=1;k<=4;k++){d=(b-$(3+k))*100;s[k]+=d;q[k]+=d*d};n++;oas+=$3*100}END{printf "%s n=%d OAS=%.0f",r,n,oas/n;split("3Y 5Y 7Y 10Y",t," ");for(k=1;k<=4;k++)printf " %s %+.1f(%.1f)",t[k],s[k]/n,sqrt(q[k]/n-(s[k]/n)^2);print ""}' $T/ice_$r.csv; done
```
