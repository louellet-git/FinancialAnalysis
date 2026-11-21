# NVDA driver model: the command for each number

Version 4 of 2026-11-21 (draft, not reviewed). The numbers are the numbers of `REPORT-DRAFT.md`, in the order of the report.

Run each command from the repository root. Build the programs one time:
`cargo build --release --manifest-path rust/Cargo.toml` and
`cargo build --release --manifest-path rust/mapcheck/Cargo.toml`. Below, `fa` is `rust/target/release/fa` and `mapcheck` is
`rust/mapcheck/target/release/mapcheck`. Each run has seed 7 and 10,000 paths (the defaults), unless the line says other.
The model is `analysis/NVDA/driver_model_paths.toml`. A run of the same model with the same seed gives the same text.

## Data

- Data store `data/raw` (the repo `louellet-git/fa-data`): commit **`1696af8`**. Get it with
  `git clone --depth 1 https://github.com/louellet-git/fa-data data/raw` (`CLAUDE.md`, "Data"). The NVDA facts, prices
  (to 2026-11-19, close $145.98) and estimates are those of `a1dffb5` (SEC facts and estimates of 2026-11-20). The store of
  `1696af8` adds the 8-K press releases of 13 quarters in `data/raw/NVDA/filings/8k/`.
- The prices of other tickers are newer in `1696af8`. This changes `fa dcf auto` a little: beta 1.675 and P50 16.57 at `1696af8`
  against 1.681 and 16.55 at `a1dffb5`. The model file has the beta of `a1dffb5` (1.681); the difference has no effect that matters.
- In the commands below `C=data/raw` and `M=analysis/NVDA/driver_model_paths.toml`.
- The 8-K page of Q3 FY27 is in the store: `data/raw/NVDA/filings/8k/0001045810-26-000088/q3fy27pr.htm` (also on the SEC site,
  `https://www.sec.gov/Archives/edgar/data/1045810/000104581026000088/q3fy27pr.htm`). The ledger section A2 has the table or line of
  the page for each number. Do not request the SEC site: read the file.
- The 10-Q of Q3 FY27 is not filed.

```bash
C=data/raw
M=analysis/NVDA/driver_model_paths.toml
git -C $C log -1 --format=%h            # 1696af8
mkdir -p /tmp/nvda_var                  # the variant files of this document
```

## 1. Headline (section 1)

```bash
fa dcf drivers NVDA --model $M                                      # P10 55.49, P25 76.35, P50 108.40, P75 152.40, P90 208.21, mean 123.85 (SE 0.70)
fa dcf drivers NVDA --model $M --seed 11                            # P50 107.12
fa dcf drivers NVDA --model $M --sims 100000                        # P10 55.76, P50 107.03, P90 208.71, mean 123.08 (SE 0.19)
fa dcf drivers NVDA --model $M --seed 12 --sims 100000              # P10 55.24, P50 106.79, P90 207.71, mean 122.44
fa dcf drivers NVDA --model $M --no-attribution --price 0.01 --price 30 --price 40   # paths below: 0.0%, 1.1%, 2.9%
for m in 0.5 1.0 1.5 2.0 4.0; do fa dcf drivers NVDA --model $M --no-attribution --pset failure.recovery_revenue_multiple=$m; done
                                                                    # P10 41.95 / 47.75 / 51.93 / 55.49 / 66.40; P50 105.68 / 106.45 / 107.19 / 108.40 / 114.94
```

The first command prints also: paths with a collapse 18.5%, terminal value share 40.1% at the means and 42.1% over the paths,
equity value at the P50 $2,621.9B, the value at the means $117.54, the table of the means by year (year 1: revenue $535.1B, free
cash flow to the firm $153.5B), the attribution block (section 4), the variance block (69.8% / 31.6% / 26.3% removed for growth /
margin / discount) and the mean path (year-10 revenue P50 $905.8B).

The floor of $30: `2.0 x 361.807 x 0.8960 = 648.4` ($B; the discount factor of year 1 is in the table of the means), plus 76.015 of
net cash and equity securities, over 24.186 billion shares = $29.95.

The hand check of the cash flows (no repo command; one-time Python, not kept): the same ten years, the terminal value and the
bridge at the means, without the collapse, give $128.32 against the engine's $128.34:

```bash
fa dcf drivers NVDA --model $M --no-attribution --pset failure.hazard=0     # line "VALUE AT THE MEANS": $128.3427
```

### The range of r (section 1)

One number r for the stakes and the charge. Net debt = -(21.141 + r x 109.747) $B; start of the charge = 1.5% + (1 - r) x 19.6% / 0.79.
For r = 1 the start is the fixed number 0.015.

```bash
rvar() { r=$1; nd=$(awk -v r=$r 'BEGIN{printf "%d", -(21.141+r*109.747)*1e9}')
  if [ "$r" = "1" ]; then st='0.015'; else st=$(awk -v r=$r 'BEGIN{printf "{ mean = %.5f, std = 0.02, lo = 0.015, hi = 0.40 }", 0.015+(1-r)*0.196/0.79}'); fi
  sed -e "s|^net_debt = .*|net_debt = $nd.0|" -e "/^\[process.charge\]/,/^half_life/ s|^start = .*|start = $st|" $M; }
for r in 0 0.25 0.5 0.75 1; do rvar $r > /tmp/nvda_var/r_$r.toml; fa dcf drivers NVDA --model /tmp/nvda_var/r_$r.toml --no-attribution; done
# P10 / P50 / P90 / mean:
# r = 0     50.16 /  101.99 / 201.17 / 117.42
# r = 0.25  52.87 /  105.17 / 204.62 / 120.63
# r = 0.5   55.49 /  108.40 / 208.21 / 123.85   (the base; the same as the headline)
# r = 0.75  58.14 / 111.53 / 212.07 / 127.06
# r = 1     60.64 / 114.10 / 220.05 / 130.56
```

## 2. The drivers (section 3): where the inputs come from

```bash
mapcheck values NVDA --cache $C                                  # the standard variables (annual, to 2026-01-25 and the 10-Q of 2026-07-26)
mapcheck NVDA --detail --cache $C --as-of 2026-11-20             # mapping checks; the `nested` lines (taxes payable etc. are inside accrued)
mapcheck NVDA --cache $C --periods 20 --emit /tmp/nvda_emit.jsonl
jq -r 'select(.form=="10-Q") | [.period_end, .values.revenue, .values.operating_income, .values.sbc_total] | @tsv' /tmp/nvda_emit.jsonl
```

Trailing four quarters to 2026-10-25: Q4 FY26 = fiscal year 215.938 less the first three quarters (44.062 + 46.743 + 57.006) =
68.127; Q1 FY27 81.615; Q2 FY27 96.221; Q3 FY27 115.844 (page). The sum is 361.807. The page is the source of Q3, the
balance sheet of 2026-10-25, the outlook and the cash flow lines of the ledger (A2). The tags typed in from the company facts:

```bash
zcat $C/NVDA/company_facts.json.gz | jq -r '.facts["us-gaap"].PaymentsToAcquireEquitySecuritiesFvNi.units.USD[]
  | select(.end=="2026-01-25" or .end=="2025-10-26") | [.start,.end,.val,.form] | @tsv'     # 4.702 (nine months FY26), 17.502 (FY26)
zcat $C/NVDA/company_facts.json.gz | jq -r '.facts["us-gaap"].GuaranteeObligationsMaximumExposure.units.USD[] | [.end,.val]|@tsv'
```

Four-quarter purchases of equity securities: (17.502 - 4.702) + 58.100 = 70.90; over revenue 361.807 = 19.6%; divided by 0.79 = 24.8%;
times (1 - r) = 0.5: 12.4%; plus 1.5% = 13.9% (the start of the charge, `process.charge.start = 0.13905`). Beta and its error:
`fa dcf auto NVDA --cache $C --as-of 2026-11-20 --explain` prints the regression beta (2.012 at `1696af8`, 2.021 at `a1dffb5`). The
standard error 0.132 is `beta x sqrt((1 - R2) / (R2 x (N - 2)))` with R2 0.477 and N 260 from a one-time script; no repo command
prints it (a candidate for the `--explain` text).

### The fade of the charge and the panel rows (section 3)

```bash
for n in 3 4 5 10; do
  sed -e '/^\[process.charge\]/,/^rationale/ s|^half_life = .*|shape = "ramp"\nramp_years = '$n'|' \
      -e '/^\[\[corr\]\]$/{N;/process.charge.half_life/{N;N;N;N;d}}' $M > /tmp/nvda_var/ramp_$n.toml
  fa dcf drivers NVDA --model /tmp/nvda_var/ramp_$n.toml --no-attribution; done          # P50 107.96 / 106.91 / 105.84 / 101.24
sed '/^\[process.charge\]/,/^rationale/ s|^half_life = .*|half_life = { mean = 2.0, std = 0.40, lo = 0.40, hi = 3.00 }|' $M > /tmp/nvda_var/ou_2.toml
fa dcf drivers NVDA --model /tmp/nvda_var/ou_2.toml --no-attribution                      # P50 105.55 (ou, half-life 2.0 years)
# the panel rows: start = 1.5% + f x 12.405% (f = 0.39, 0.55, 0.24), ou 1.0 year
fa dcf drivers NVDA --model $M --no-attribution --pset process.charge.start=0.06338      # f = 0.39: P50 110.90
fa dcf drivers NVDA --model $M --no-attribution --pset process.charge.start=0.08323      # f = 0.55: P50 110.19
fa dcf drivers NVDA --model $M --no-attribution --pset process.charge.start=0.04477      # f = 0.24: P50 111.48
# rate variants (r = 0.5, ou 1.0): Q3 rate 13.55% (0.015 + 0.5 x 13.55% / 0.79), net of sales proceeds 16.55%, no tax gross-up
fa dcf drivers NVDA --model $M --no-attribution --pset process.charge.start=0.10075      # P50 109.60
fa dcf drivers NVDA --model $M --no-attribution --pset process.charge.start=0.11976      # P50 108.99
fa dcf drivers NVDA --model $M --no-attribution --pset process.charge.start=0.113        # P50 109.20
fa dcf drivers NVDA --model $M --no-attribution --pset process.charge.target=0.02        # acquisitions 2.0%: P50 107.45, mean 122.79
```

The net rate: (58.100 - 9.493) / 293.681 = 16.55% (nine months; proceeds from sales of equity securities 9.493). The Q3 rate:
15.696 / 115.844 = 13.55%. The correlation of the two half-lives is 0.9 in the model file (`[[corr]]`); 0.95 is not positive definite.

## 3. Sensitivities of section 4

```bash
fa dcf drivers NVDA --model $M --no-attribution --sensitivity process.growth.start=0.30,0.40,0.435,0.456,0.47,0.479,0.50,0.547,0.55
                                                                                    # 85.50 / 97.74 (guide alone) / 102.40 / 105.04 / 107.07 / 108.40 (base) / 111.38 / 118.16 / 118.65
fa dcf drivers NVDA --model $M --no-attribution --sensitivity discount.erp=0.035,0.0441,0.055                        # 128.03 / 108.40 / 91.70
fa dcf drivers NVDA --model $M --no-attribution --sensitivity process.margin.target=0.35,0.485,0.60                  # 88.41 / 108.40 / 124.03
fa dcf drivers NVDA --model $M --no-attribution --sensitivity process.growth.half_life=0.4,0.6,0.66,1.0,1.25,2.5     # 91.31 / 95.62 / 97.14 / 108.40 / 118.48 / 176.91
```

The half-life of 0.66 years gives year 2 at +16.6% (4% + 36% x 0.5^(1/0.66)), the guide pace of +15% to +18% (a constant +3.5% to
+4.2% a quarter in 13-week quarters: 121 + 125 + 130 + 134 in year 1, then year 2 / year 1 - 1; a hand calculation).

One number changed (`--pset NAME=V`), P50 in the comment:

```bash
fa dcf drivers NVDA --model $M --no-attribution --pset tax.rate=0.17                  # 113.87
fa dcf drivers NVDA --model $M --no-attribution --pset discount.beta=1.2              # 138.32
fa dcf drivers NVDA --model $M --no-attribution --pset failure.hazard=0.0             # 116.93
fa dcf drivers NVDA --model $M --no-attribution --pset failure.hazard=0.04            # 97.40
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.sigma=0.10      # 111.96
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.sigma=0.25      # 99.21
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.persistence=-0.25   # 108.08
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.persistence=0.25    # 108.19
fa dcf drivers NVDA --model $M --no-attribution --pset terminal.roic=0.15             # 105.04
fa dcf drivers NVDA --model $M --no-attribution --pset terminal.roic=1.0              # 112.05
fa dcf drivers NVDA --model $M --no-attribution --pset process.s2c.target=2.0         # 107.37
fa dcf drivers NVDA --model $M --no-attribution --pset process.s2c.target=6.0         # 109.25
fa dcf drivers NVDA --model $M --no-attribution --pset process.margin.start=0.64      # 106.90
fa dcf drivers NVDA --model $M --no-attribution --pset process.margin.start=0.69      # 110.61
fa dcf drivers NVDA --model $M --no-attribution --no-shocks                           # 113.91
```

Two rows change the file (a copy in `/tmp`; `sed` makes it):

```bash
sed 's|^equity_weight = 1.0|equity_weight = 1.0\nmid_year = true|' $M > /tmp/nvda_var/mid_year.toml                       # 114.57
sed '/^a = "growth"/,/^note/ s|^rho = 0.6|rho = 0.25|' $M > /tmp/nvda_var/shock_corr_025.toml                              # 108.04
fa dcf drivers NVDA --model /tmp/nvda_var/mid_year.toml --no-attribution
fa dcf drivers NVDA --model /tmp/nvda_var/shock_corr_025.toml --no-attribution
```

### The guide against the result (section 4, ledger L26)

Each 8-K press release has the sentence "Revenue is expected to be $X billion, plus or minus Y%" in its Outlook, and the line
"Revenue" in its income statement. The pairs (the guide of one release against the revenue of the next):

```bash
cd data/raw/NVDA/filings/8k
for d in $(ls); do f=$d/$(ls $d)
  txt=$(sed -e 's/<[^>]*>/ /g' -e 's/&#160;/ /g; s/&nbsp;/ /g' $f | tr -s ' \n\t' ' ')
  g=$(echo "$txt" | grep -o 'Revenue is expected to be \$[0-9.]* billion, plus or minus [0-9]*%' | head -1 | sed 's/.*\$\([0-9.]*\) billion, plus or minus \([0-9]*\)%/\1 \2/')
  rv=$(echo "$txt" | grep -o 'Revenue \$ [0-9,]* \$' | head -1 | sed 's/Revenue \$ \([0-9,]*\) \$/\1/; s/,//')
  echo "$f $g $rv"; done |
awk '{n++; file[n]=$1; g[n]=$2; pm[n]=$3; r[n]=$4}
     END{for(i=1;i<n;i++){beat=(r[i+1]/1000/g[i]-1)*100; printf "%s guide %s (+-%d%%) reported %.3f beat %+.1f%%\n", file[i], g[i], pm[i], r[i+1]/1000, beat}}'
cd - > /dev/null
```

The output has 12 lines (13 releases; the last guide is for Q4 FY27 and is not reported). The beats: +10.5, +8.5, +7.3, +7.9, +4.9,
+2.5, +3.9, +5.6, +4.8, +4.6, +5.7, +7.3 (%). Sort and take the median of the 12 numbers: +5.65% (mean +6.12%, minimum +2.5%,
maximum +10.5%). The sensitivity: year-1 growth = 1.40 x (1 + beat) - 1.

```bash
fa dcf drivers NVDA --model $M --no-attribution --sensitivity process.growth.start=0.435,0.479,0.547   # 102.40 / 108.40 / 118.16 (beats +2.5%, +5.65% = the base, +10.5%); the guide alone is 0.40: 97.74
```

## 4. Base rates of scale and fade (section 4)

```bash
sed -n 2210,2260p analysis/priors/process_priors_2026-11-19.toml    # [[fade]] size ">100B", threshold 0.5: n_start 14; steps k=1,2,3: n 12, 11, 11; p50 0.447, -0.133, -0.055
sed -n 2081,2130p analysis/priors/process_priors_2026-11-19.toml    # [[fade]] size "30-100B", threshold 0.5: n_start 42; n 39, 38, 36; p50 0.125, -0.005, 0.033
mapcheck --cache $C --emit /tmp/all_emit.jsonl                       # all tickers of the cache, 10-K and 10-Q rows (about 45 s); 1,976 with us-gaap facts
jq -r 'select(.form=="10-K" and .values.revenue!=null) | [.values.revenue, .ticker, .period_end] | @tsv' /tmp/all_emit.jsonl | sort -gr | head -3
                                                                      # AMZN 716.9B (2025-12-31), WMT 713.2B (2026-01-31), WMT 681.0B
jq -r 'select(.form=="10-K" and .values.operating_income!=null) | [.values.operating_income, .ticker, .period_end] | @tsv' /tmp/all_emit.jsonl | sort -gr | head -4
                                                                      # AAPL 158.1B (2026-09-26), MSFT 155.2B (2026-06-30), AAPL 133.1B, NVDA 130.4B (2026-01-25)
jq -r 'select(.form=="10-K" and .values.revenue>=100e9 and .values.operating_income!=null) | [(.values.operating_income/.values.revenue*1000|round/10), .ticker, .period_end] | @tsv' /tmp/all_emit.jsonl | sort -gr | head -12
                                                                      # MU 74.6, NVDA 62.4, NVDA 60.4, MSFT 46.8 ...
jq -r 'select(.ticker=="MU" and .form=="10-K" and .period_end>="2021-06") | [.ticker,.period_end,.values.revenue,.values.operating_income]|@tsv' /tmp/all_emit.jsonl
                                                                      # FY22 30.76B, FY23 15.54B (op. income -5.745B), FY26 133.19B (99.34B)
# the composition of the cell above $100B: company-years with revenue of $100B or more and growth of 50% or more (10-K rows)
jq -r 'select(.form=="10-K" and .values.revenue!=null) | [.ticker,.period_end,.values.revenue]|@tsv' /tmp/all_emit.jsonl | sort -k1,1 -k2,2 |
  awk -F'\t' '{ if($1==pt && $3>=100e9 && pr>0 && $3/pr-1>=0.5) printf "%s %s rev %.0fB growth %.0f%%\n",$1,substr($2,1,4),$3/1e9,($3/pr-1)*100; pt=$1; pr=$3}'
                                                                      # CI 2019, CVX 2021 and 2022, MPC 2021, MU 2026, NVDA 2025 and 2026, PSX 2021 and 2022, VLO 2021 and 2022, XOM 2021
```

Revenue after three years from $361.8B: cell above $100B 361.8 x 1.447 x 0.867 x 0.945 = $429B; cell $30B to $100B
361.8 x 1.125 x 0.995 x 1.033 = $418B; the model at the means is in the table of the headline run (year 3: $774.9B). The year-10
operating profit at the means is $1,130.2B x (50.7% - 3.5%) = $533B (year 10 of the table; SBC target 3.5%).

## 5. What the price implies (section 5)

Close $145.98 on 2026-11-19 (`data/raw/NVDA/prices.json`, last row). The close of 2026-11-20 is not in the store. Earlier closes
(2026-11-03 $206.69, 2026-11-17 $155.46, 2026-11-18 $150.96): `jq -r '.[] | select(.date>="2026-11-03") | [.date,.close]|@tsv' data/raw/NVDA/prices.json`.

```bash
P=145.98
fa dcf drivers NVDA --model $M --no-attribution --price $P                           # paths below the close: 72.0%
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param process.growth.half_life --range 0.5,2.9    # 1.8513
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param process.growth.start --range 0.15,1.5 --shift-bounds   # 0.7161
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param discount.erp --range 0.01,0.07 --shift-bounds        # 0.0291
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param discount.beta --range 0.5,2.0 --shift-bounds        # 1.1162
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param process.margin.target --range 0.30,0.78 --shift-bounds   # 0.7406
```

## 6. Automatic screen (section 6)

```bash
fa dcf auto NVDA --cache $C --as-of 2026-11-20 --explain     # a1dffb5: P10 1.18, P25 5.40, P50 16.55, P75 43.26, P90 104.57, mean 43.85; 1696af8: P50 16.57, mean 44.00
```

## 7. Order of the runs (section 7)

Each line makes one file from the one before. Run `fa dcf drivers NVDA --model FILE --no-attribution` on each (P50 in the comment).

```bash
T=/tmp/nvda_var
git show 9a2e6f0:analysis/NVDA/driver_model_paths.toml > $T/v1.toml        # version 1: P50 130.89 (P10 59.06, P90 298.97, mean 162.33)
git show 41afd53:analysis/NVDA/driver_model_paths.toml > $T/v2.toml        # version 2: P50 97.65
git show 6eac285:analysis/NVDA/driver_model_paths.toml > $T/v3.toml        # version 3: P50 85.07
sed -e 's|^base_revenue = .*|base_revenue = 361807000000.0|' \
    -e 's|^start = { mean = 0.65, std = 0.20, lo = 0.20, hi = 1.00 }|start = { mean = 0.40, std = 0.12, lo = 0.10, hi = 0.75 }|' \
    -e 's|^stub_years = .*|stub_years = 0.9288|' $T/v1.toml > $T/step1.toml                                        # 107.20
sed -e 's|^start = { mean = 0.68, std = 0.03, lo = 0.55, hi = 0.75 }|start = { mean = 0.66, std = 0.03, lo = 0.55, hi = 0.75 }|' \
    -e 's|^start = 0.0239|start = 0.0227|' $T/step1.toml > $T/step2.toml                                          # 105.82
sed -e 's|^net_debt = .*|net_debt = -21141000000.0|' -e 's|^shares = .*|shares = 24186000000.0|' \
    -e 's|^beta = { mean = 1.676,|beta = { mean = 1.681,|' $T/step2.toml > $T/step3.toml                           # 105.93
sed 's|^half_life = { mean = 1.25, std = 0.50, lo = 0.40, hi = 3.00 }|half_life = { mean = 1.0, std = 0.40, lo = 0.40, hi = 3.00 }|' $T/step3.toml > $T/step4.toml   # 98.69
sed '/^\[process.charge\]/,/^half_life/ s|^target = .*|target = { mean = 0.015, std = 0.007, lo = 0.0, hi = 0.05 }|' $T/step4.toml > $T/step5.toml            # 97.65 (= v2)
sed 's|^start = 0.0227|start = 0.0212|' $T/v2.toml > $T/step7.toml                                                # 97.76
sed 's|^net_debt = .*|net_debt = -130888000000.0|' $T/step7.toml > $T/step8.toml                                  # 102.29
# step 9 = $T/v3.toml: 85.07
sed -e 's|^net_debt = .*|net_debt = -76015000000.0|' \
    -e '/^\[process.charge\]/,/^ramp_years/ s|^start = .*|start = { mean = 0.13905, std = 0.02, lo = 0.015, hi = 0.40 }|' $T/v3.toml > $T/step10.toml   # 91.57
# step 11 = the model file of this version with the guide alone: 97.74
sed 's|^start = { mean = 0.479, std = 0.12, lo = 0.10, hi = 0.75 }|start = { mean = 0.40, std = 0.12, lo = 0.10, hi = 0.75 }|' $M > $T/step11.toml
# step 12 = the model file of this version (year-1 growth 47.9% = 1.40 x 1.0565 - 1): 108.40
```

The first run of version 1 without the collapse table (P50 145.95) is the version 1 file with the `[failure]` table removed. The
Python check of the growth process alone (a year below -30% on 5.1% of the paths) is a one-time script and is not kept.

## 8. The engine check

```bash
fa dcf drivers IBM          # P10 83.84, P50 151.84, P90 258.03 (unchanged by this work)
```

## What is not repeatable by a repo command

- The standard error of the beta (one-time script).
- The hand check of the cash flows (one-time Python, formula in the model file).
- The check of the growth process alone (one-time Python).
- The 8-K numbers as data: the guide table (section 3) reads the HTML with a shell pipeline that depends on the text of the page.
  A request to the Head of Technology: the 8-K tables as data (ticket `T-20261121-dcf-engine-from-tickers`, candidate 7).
