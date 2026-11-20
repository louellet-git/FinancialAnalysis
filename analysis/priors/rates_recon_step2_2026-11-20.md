# Rates reconciliation, step 2 (2026-11-20)

Ticket `T-20261120-arb-2-rates-recon`, step 2. Step 1 is `analysis/priors/rates_recon_2026-11-20.md`. This file tests the
1Y bill-basis hypothesis of step 1 and gives the rich and cheap residuals of the notes and bonds, with their persistence.
It is a measure of a model residual. It is not a trade signal.

**Rule of the ticket.** A residual that lasts and is larger than the bid-ask cost is a candidate, not an arbitrage. The
repo rate (general or special), the borrow, the financing and the tax are not known (`docs/HIDDEN-ASSUMPTIONS.md` #112).
The prices are FedInvest end-of-day bid prices (model input), not executable quotes. The repo sends no order.

## 1. Questions

1. Is the +8.5 bp difference between GSW par 1Y and Treasury par 1Y a bill basis (convention)?
2. Which notes and bonds are rich or cheap by more than the cost, does the residual last, and what is the half-life?

## 2. Data and commands

Store `data/raw` of 2026-11-20 (read only): FedInvest prices for 58 days (2026-08-27 to 2026-11-19), 340 notes and
bonds of 6 months or more in the panel, `rates/treasury_bill_rates_2026.csv`, the step-1 rate files. Rates are in
basis points (bp) of yield. Residual = observed minus fitted yield (plus = cheap, minus = rich). The curve is a
Nelson-Siegel-Svensson fit of each day to the notes and bonds (bills excluded). Cost = larger of bid-ask and 1/64
price step, one leg. Every command is in the appendix, in the order of the tables. No command downloaded a file.
`fa recon explain` was not run (see 6). `awk` rounds to 2 decimals; std dev = population std dev.

## 3. Evidence

### 3.1 Bill basis (`fa recon bills --date 2026-11-18`; the 2026-11-19 curve does not exist yet)

Mean of observed minus theoretical bill yield, bp, 47 bills:

| Test | <=3M | 3-6M | >6M | all |
|---|---|---|---|---|
| Baseline: par nodes as bond-equivalent, T+1 | -10.7 | -2.8 | -3.3 | -7.1 |
| Nodes as money-market yield (ACT/360) | -16.6 | -9.0 | -5.8 | -12.6 |
| Settlement T+2 | +2.0 | +0.4 | -1.8 | +1.0 |
| Nodes from the bill-rate table (coupon equivalent) | +0.5 | -3.7 | -5.2 | -1.7 |

The par curve is above the bill-rate table by +12 bp (1M), +10 (1.5M), +12 (2M), +8 (3M), -1 (4M), -4 bp (6M).

### 3.1b The 1Y point against the 52-week bill (218 days, 2026-01-02 to 2026-11-13, bp)

| Difference | Mean | Std dev |
|---|---|---|
| Treasury par 1Y minus 52-week bill, coupon equivalent | +0.7 | 1.1 |
| GSW par 1Y minus Treasury par 1Y | +8.5 | 2.6 |
| GSW par 1Y minus 52-week bill, coupon equivalent | +9.3 | 2.9 |

### 3.2 Fit and bills against the note curve (`fa recon rich-cheap`)

| Date | Fit RMSE (bp) | Candidates (cheap, rich) | Bill residual 6M to 11M (n); 11M and more (n), bp |
|---|---|---|---|
| 2026-09-15 | 2.03 | 14 of 401 (4, 10) | -6.6 (6); -6.3 (1) |
| 2026-10-15 | 1.77 | 18 of 400 (3, 15) | -11.6 (5); -8.5 (1) |
| 2026-11-18 | 1.89 | 14 of 400 (3, 11) | -7.3 (5); -3.5 (1) |

### 3.3 Persistence, 58 days, 18,893 security-days (`fa recon history`)

| Measure | Lag 1 | Lag 5 | Lag 10 |
|---|---|---|---|
| Autocorrelation, raw residual | 0.948 | 0.928 | 0.918 |
| Autocorrelation, residual minus own mean | 0.441 | 0.199 | 0.062 |
| Half-life from phi, raw (business days) | 13.7 | 53.0 | 102.2 |
| Half-life from phi, minus own mean (business days) | 0.8 | 2.1 | 2.5 |

The raw residual is close to a unit root (phi 0.93 to 0.95; no formal test was run, the sample is 58 days). Almost
all of it is a fixed level for each security. After the own mean is removed the residual lasts about 1 to 3 days.
The 10% most rich at a date keep 96% of the residual after 10 days (raw) but 13% (minus own mean). Mean fit RMSE: 1.90 bp
(range 1.46 to 2.50). By group, raw lag-10 autocorrelation: old 30-year bonds 0.971, high coupon 0.900, low coupon 0.816.

### 3.4 The 10 largest lasting residuals

"Lasting" = at least 38 dates, the same sign on 90% of the days, and |mean| above both the mean cost and 1.9 bp (the
fit RMSE). The rule is mine, set before the run, not tuned. 61 of 340 securities pass (60 with exact values: 29 rich, 31 cheap; the table rounds to 2 decimals). The half-life
is of the deviation from the own mean (lag-1 coefficient), not of the level.

| CUSIP | Maturity | Coupon | Mean residual (bp) | Cost (bp) | Std dev (bp) | Half-life (days) | Group |
|---|---|---|---|---|---|---|---|
| 912810PW2 | 2038-02-15 | 4.375% | -14.62 | 1.43 | 0.86 | 1.6 | old 30-year |
| 912810FT0 | 2036-02-15 | 4.500% | -13.46 | 2.50 | 3.07 | 8.9 | old 30-year |
| 912810QA9 | 2039-02-15 | 3.500% | -12.55 | 1.12 | 0.72 | 1.5 | old 30-year |
| 912810PU6 | 2037-05-15 | 5.000% | -12.15 | 1.67 | 1.15 | 1.8 | old 30-year |
| 912810PX0 | 2038-05-15 | 4.500% | -12.14 | 1.33 | 0.91 | 2.0 | old 30-year |
| 912810PT9 | 2037-02-15 | 4.750% | -11.79 | 1.58 | 1.47 | 2.6 | old 30-year |
| 912810QB7 | 2039-05-15 | 4.250% | -7.96 | 0.99 | 0.59 | 1.1 | old 30-year |
| 912810QC5 | 2039-08-15 | 4.500% | -6.14 | 0.85 | 0.62 | 1.4 | old 30-year |
| 912810QD3 | 2039-11-15 | 4.375% | -5.44 | 0.86 | 0.59 | 1.4 | old 30-year |
| 912810QE1 | 2040-02-15 | 4.625% | -5.07 | 0.88 | 0.48 | 1.1 | other |

All 10 are rich (the price is above the curve) and are bonds of 9.2 to 13.3 years (from 2026-11-18). The first cheap one is 91282CNP2
(+3.27 bp, cost 2.34). The 31 cheap securities that pass the rule (exact values) have |mean| of 1.9 to 3.3 bp: they pass, but sit near the fit RMSE (1.9 bp).

## 4. Answers

1. **Bill basis.** The hypothesis of step 1 is rejected in this form. The Treasury par 1Y is the 52-week bill in coupon
   equivalent (difference +0.7 bp, std dev 1.1), so the Treasury file has no basis error at 1Y. Bills of 6 months and more
   price 4 to 12 bp rich against the curve of the notes (3.2), and the 1Y GSW point is a coupon-curve value. So the
   +8.5 bp is a difference between the bill market and the coupon market (GSW does not use bills), not a conversion
   error. The size is consistent (3.5 to 11.6 bp) but the sample is 3 days with 1 to 6 bills: hypothesis, not proved.
   The short end differs from the bill table by +8 to +12 bp at 1M to 3M (3.1): the par curve is not the bill table
   there (open, cause not tested: the bill table is a different yield quote).
2. **Rich and cheap.** The residual lasts, but as a fixed level of the security, not as a gap that closes: after the
   own mean is removed it decays in 1 to 3 days. The lasting rich bonds are the original 30-year bonds of 2036 to 2040.
   They share one sign and a size of 5 to 15 bp, so this is a group effect (coupon, tax, strips and futures
   deliverability are not in the model), as the command states. It is not shown to be a mispriced single bond.
   The same group was found before (`docs/HIDDEN-ASSUMPTIONS.md` #112: the nine old 30-year bonds of 2036 to 2039
   are rich by 6 to 19 bp). This note adds that the level lasts for the 58 days of the store.

## 5. Candidates (each is a candidate, not an arbitrage)

For each of the 10 bonds of 3.4 the residual (5 to 15 bp) is above the one-leg cost (0.9 to 2.5 bp) and above the
fit RMSE. The side is rich. Not known for any of them: the borrow fee and
the special repo rate of the bond, the financing of the long leg, the round trip (two legs: about twice the cost),
the tax, the interpolation gap of #112 (no note or non-SOMA security in 2036 to 2042), and the model error of the fit (RMSE 2.77 bp for 10Y to 20Y on 2026-11-18, the largest for 1Y or more). No
position is advised. A residual that does not close in 58 days has no known exit: the owner must decide if a
level that lasts is a mispricing or a feature of these bonds.

## 6. What did not run, limits, open

- `fa recon explain` was not run. It downloads the New York Fed (SOFR, SOMA) and Fiscal Data files when they are
  not cached. They are not in the store, and the rule is no download. This leaves the cause of the group effect
  untested. Owner: Head of Technology (data ticket for the New York Fed files); then Quant.
- 58 days, one quarter, one regime. Unit root test not run. Survivorship: only securities in the files of the period.
- Prices are end-of-day bid prices; no volume, no intraday time. The bill sample of 3.2 is 3 dates with 1 to 6 bills. The curve is a model: residual = price gap plus
  model error. 2026-11-19 has prices but no Treasury curve in the store, so the last bill test is for 2026-11-18.
- Open, owner Quant: the 1M to 3M gap of +8 to +12 bp between the par curve and the bill table; the old 30-year
  group effect (needs the explain files); the cost of the round trip.

## A. Commands

Run from the repository root of a checkout that has `data/raw` and a built `fa` (set `FA` for another binary). In B5 the group column is for 2026-11-18 only. The script was run from the appendix text and gave the numbers of the tables
(the results of `fa` commands are in the order 3.1, 3.2 (first date), 3.1b, 3.2, 3.3, 3.4). It writes only to a `mktemp`
folder and downloads nothing. It needs the binary of the worktree (`cargo build --release --manifest-path rust/Cargo.toml --locked`).

```bash
cd "$(git rev-parse --show-toplevel)"
export FA_REPO=$PWD; FA=${FA:-rust/target/release/fa}
R=$FA_REPO/data/raw/rates; T=$(mktemp -d)
# B1. Bill-basis tests (table 3.1) and fit on 2026-11-18 (3.2)
$FA recon bills --date 2026-11-18
$FA recon rich-cheap --date 2026-11-18 --out $T/rc.csv
# B2. 52-week bill, CMT 1Y and GSW par 1Y (table 3.1b), all 218 common days
awk -F, 'NR>1{split($1,a,"/");print a[3]"-"a[1]"-"a[2]","$15}' $R/treasury_bill_rates_2026.csv | sort > $T/b52.csv
awk -F, 'NR>1{split($1,a,"/");print a[3]"-"a[1]"-"a[2]","$8}' $R/treasury_nominal_2026.csv | sort > $T/cmt1.csv
awk -F, '/^Date,BETA0/{for(i=1;i<=NF;i++)h[$i]=i;hd=1;next} hd&&$1>="2026-01-01"&&$h["SVENPY01"]!="NA"{print $1","$h["SVENPY01"]}' $R/gsw_feds200628.csv | sort > $T/g1.csv
join -t, $T/b52.csv $T/cmt1.csv | join -t, - $T/g1.csv | awk -F, '{a=($3-$2)*100;b=($4-$3)*100;c=($4-$2)*100;sa+=a;sb+=b;sc+=c;qa+=a*a;qb+=b*b;qc+=c*c;n++}END{printf "CMT1Y-52W_CE %.1f sd %.1f | GSWpar1Y-CMT1Y %.1f sd %.1f | GSWpar1Y-52W_CE %.1f sd %.1f n=%d\n",sa/n,sqrt(qa/n-(sa/n)^2),sb/n,sqrt(qb/n-(sb/n)^2),sc/n,sqrt(qc/n-(sc/n)^2),n}'
# B3. Bills by maturity against the note-fitted curve, three days (table 3.2b)
for d in 2026-09-15 2026-10-15 2026-11-18; do $FA recon rich-cheap --date $d --out $T/rc_$d.csv | grep -E "^Candidates"
 awk -F, -v d=$d 'NR>1&&$2=="bill"{y=$5;b=(y<0.25)?"<3M":(y<0.5)?"3-6M":(y<0.9)?"6-11M":">=11M";s[b]+=$15;c[b]++}END{for(k in s)printf "%s bills %s n=%d mean_res=%.1f\n",d,k,c[k],s[k]/c[k]}' $T/rc_$d.csv | sort; done
# B4. Persistence over the 58 days (table 3.3)
$FA recon history --out $T/hist.csv
# B5. Per-security record and the lasting residuals (table 3.4)
cat > $T/per.awk <<'E'
function flush(  i,m,v,num,pos,phi,hl){
 if(n==0)return; m=0;for(i=1;i<=n;i++)m+=x[i];m/=n; v=0;num=0;pos=0
 for(i=1;i<=n;i++){v+=(x[i]-m)^2; if(x[i]*m>0)pos++; if(i>1)num+=(x[i]-m)*(x[i-1]-m)}
 phi=(v>0)?num/v:0; hl=(phi>0&&phi<1)?log(0.5)/log(phi):-1
 printf "%s,%d,%.2f,%.2f,%.2f,%.2f,%.2f,%.1f\n",cur,n,m,sqrt(v/n),cs/n,pos/n,phi,hl; n=0;cs=0}
BEGIN{FS=","}
$1!=cur{flush();cur=$1}
{n++;x[n]=$3;cs+=$4}
END{flush()}
E
tail -n +2 $T/hist.csv | sort -t, -k1,1 -k2,2 | awk -f $T/per.awk > $T/per.csv
# lasting = 38 or more dates, same sign on 90% of days, |mean| above mean cost and above 1.9 bp (the fit RMSE)
awk -F, '{a=$3<0?-$3:$3; if($2>=38&&$6>=0.9&&a>$5&&a>1.9)print a","$0}' $T/per.csv | sort -t, -k1,1 -gr > $T/last.csv
echo "lasting: $(wc -l < $T/last.csv) of $(wc -l < $T/per.csv) securities"
echo "cusip,type,coupon,maturity,mean_bp,cost_bp,sd_bp,half_life_days,group"
awk -F, 'NR==FNR{if(FNR>1){c[$1]=$3;m[$1]=$4;g[$1]=$23;t[$1]=$2};next}{print $2","t[$2]","c[$2]","m[$2]","$4","$6","$5","$9","g[$2]}' $T/rc.csv $T/last.csv | head -10
awk -F, 'NR==FNR{if(FNR>1)g[$1]=$23;next}{print g[$2]}' $T/rc.csv $T/last.csv | sort | uniq -c
awk -F, '{n[($4<0)?"rich":"cheap"]++}END{for(k in n)print k,n[k]}' $T/last.csv
```
