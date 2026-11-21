# NVDA driver model: the command for each number

Version 3 of 2026-11-21 (draft, not reviewed). The numbers are the numbers of `REPORT-DRAFT.md`, in the order of the report.

Run each command from the repository root of the worktree. Build the programs one time:
`cargo build --release --manifest-path rust/Cargo.toml` and
`cargo build --release --manifest-path rust/mapcheck/Cargo.toml`. Below, `fa` is `rust/target/release/fa` and `mapcheck` is
`rust/mapcheck/target/release/mapcheck`. Each run has seed 7 and 10,000 paths (the defaults), unless the line says other.
The model is `analysis/NVDA/driver_model_paths.toml`. A run of the same model with the same seed gives the same text.

## Data

- Data store `data/raw` (the repo `louellet-git/fa-data`): commit `a1dffb5` for the numbers of this report. The store commit of
  2026-11-21 is `c486b86`; its NVDA files are the same (prices to 2026-11-19, close $145.98; SEC facts and estimates of 2026-11-20).
  The prices of other tickers are newer in `c486b86`. This changes `fa dcf auto` a little (beta 1.675 and P50 16.57 at `c486b86`;
  1.681 and 16.55 at `a1dffb5`). The model file has the beta of `a1dffb5` (1.681); the difference has no effect that matters.
- In the worktree `data/raw` is empty. Use `--cache /home/user/FinancialAnalysis/data/raw` (or a clone of the store) in each
  command that reads the cache. In the lines below `$C` is that folder.
- The 8-K page: `https://www.sec.gov/Archives/edgar/data/1045810/000104581026000088/q3fy27pr.htm`. One request with the
  User-Agent of `sec_contact.toml`:
  `curl -sS -A "$(sed -n 's/^user_agent = "\(.*\)"/\1/p' sec_contact.toml)" -o q3fy27pr.htm <address>`. The ledger section A2 has
  the table or line of the page for each number. The SEC site is public; do not request it again if you have the file.
- The 10-Q of Q3 FY27 is not filed.

```bash
C=/home/user/FinancialAnalysis/data/raw
git -C $C log -1 --format=%h
```

## 1. Headline (section 1)

```bash
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml                     # P10 44.27, P25 59.48, P50 85.07, P75 122.98, P90 174.19, mean 100.17 (SE 0.60)
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --seed 11           # P50 85.09
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --sims 100000       # P50 84.85
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --seed 12 --sims 100000   # P50 84.85
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --no-attribution --price 0.01 --price 20 --price 25 --price 30 --price 40
                                                                                       # paths below: 0.0%, 0.0%, 0.1%, 0.5%, 5.9%
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --no-attribution --pset failure.recovery_revenue_multiple=1.0   # P10 35.86
fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --no-attribution --pset failure.recovery_revenue_multiple=4.0   # P10 54.14
```

The first command prints also: paths with a collapse 18.5%, terminal value share 45.5% at the means and 47.7% over the paths,
equity value at the P50 $2,057.5B, the value at the means $94.64, the table of the means by year (year 1: revenue $506.5B, free
cash flow to the firm $102.1B), the attribution block (section 4) and the variance block (71.1% / 35.2% / 26.6% removed).
The floor of $32 is a hand calculation: `2.0 x 361.807 x 0.8960 = 648.4` ($B, the discount factor of year 1 is in the table of
the means), plus 130.888 of net cash and equity securities, over 24.186 billion shares.

The hand check of the cash flows (no repo command; one-time Python, not kept): the same ten years, the terminal value and the
bridge at the means, without the collapse (`--pset failure.hazard=0`), give $102.99 against the engine $103.02
(`fa dcf drivers NVDA --model ... --no-attribution --pset failure.hazard=0`, line "VALUE AT THE MEANS").

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
plus 1.5% = 26.3% (the start of the charge). Beta and its error: `fa dcf auto NVDA --cache $C --as-of 2026-11-20 --explain`
prints the regression beta (2.012 at `c486b86`, 2.021 at `a1dffb5`). The standard error 0.132 is `beta x sqrt((1 - R2) / (R2 x (N - 2)))`
with R2 0.477 and N 260 from a one-time script; no repo command prints it (a candidate for the `--explain` text).

## 3. Sensitivities of section 4

```bash
M=analysis/NVDA/driver_model_paths.toml
fa dcf drivers NVDA --model $M --no-attribution --sensitivity process.growth.start=0.28,0.40,0.55     # 72.90 / 85.07 / 102.32
fa dcf drivers NVDA --model $M --no-attribution --sensitivity discount.erp=0.035,0.0441,0.055         # 101.66 / 85.07 / 71.66
fa dcf drivers NVDA --model $M --no-attribution --sensitivity process.margin.target=0.35,0.485,0.60   # 68.18 / 85.07 / 99.11
fa dcf drivers NVDA --model $M --no-attribution --sensitivity process.growth.half_life=0.6,1.0,1.25,2.5   # 76.63 / 85.07 / 91.65 / 127.89
```

One number changed (`--pset NAME=V`), P50 in the comment:

```bash
fa dcf drivers NVDA --model $M --no-attribution --pset tax.rate=0.17                  # 89.23
fa dcf drivers NVDA --model $M --no-attribution --pset discount.beta=1.2              # 110.16
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.start=0.456     # 91.26 (Q4 revenue 4% over the guide: 1.40 x 1.04 - 1)
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.half_life=0.4   # 73.89 (the cell above $100B; the cut of the draw)
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.half_life=0.6   # 76.63
fa dcf drivers NVDA --model $M --no-attribution --pset failure.hazard=0.0             # 92.31
fa dcf drivers NVDA --model $M --no-attribution --pset failure.hazard=0.04            # 76.11
fa dcf drivers NVDA --model $M --no-attribution --pset failure.recovery_revenue_multiple=1.0   # 83.51
fa dcf drivers NVDA --model $M --no-attribution --pset failure.recovery_revenue_multiple=4.0   # 91.63
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.sigma=0.10      # 87.89
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.sigma=0.25      # 77.56
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.persistence=-0.25   # 84.36
fa dcf drivers NVDA --model $M --no-attribution --pset process.growth.persistence=0.25    # 85.07
fa dcf drivers NVDA --model $M --no-attribution --pset terminal.roic=0.15             # 82.23
fa dcf drivers NVDA --model $M --no-attribution --pset terminal.roic=1.0              # 88.36
fa dcf drivers NVDA --model $M --no-attribution --pset process.s2c.target=2.0         # 84.10
fa dcf drivers NVDA --model $M --no-attribution --pset process.s2c.target=6.0         # 85.79
fa dcf drivers NVDA --model $M --no-attribution --pset process.margin.start=0.64      # 83.77
fa dcf drivers NVDA --model $M --no-attribution --pset process.margin.start=0.69      # 87.09
fa dcf drivers NVDA --model $M --no-attribution --no-shocks                           # 91.07
```

Three rows change the file (a copy in `/tmp`; `sed` makes it):

```bash
mkdir -p /tmp/nvda_var
sed 's|^equity_weight = 1.0|equity_weight = 1.0\nmid_year = true|' $M > /tmp/nvda_var/mid_year.toml                       # 89.74
sed '/^a = "growth"/,/^note/ s|^rho = 0.6|rho = 0.25|' $M > /tmp/nvda_var/shock_corr_025.toml                              # 84.65
fa dcf drivers NVDA --model /tmp/nvda_var/mid_year.toml --no-attribution
fa dcf drivers NVDA --model /tmp/nvda_var/shock_corr_025.toml --no-attribution
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
jq -r 'select(.ticker=="MU" and .form=="10-K" and .period_end>="2021-06") | [.period_end,.values.revenue,.values.operating_income]|@tsv' /tmp/all_emit.jsonl
                                                                      # FY22 30.76B, FY23 15.54B (op. income -5.745B), FY26 133.19B (99.34B)
```

Revenue after three years from $361.8B: cell above $100B 361.8 x 1.447 x 0.867 x 0.945 = $429B; cell $30B to $100B
361.8 x 1.125 x 0.995 x 1.033 = $418B; the model at the means is in the table of the headline run (year 3: $698.3B). The year-10
operating profit at the means is $1,000.1B x (50.7% - 3.5%) = $472B (year 10 of the table; SBC target 3.5%). The P50 of year-10
revenue over all paths ($803.5B) is in the mean-path table of the headline run.

## 5. The charge for equity securities and the bridge (section 3, M3)

```bash
mkdir -p /tmp/nvda_var
ch() { sed "/^\[process.charge\]/,/^ramp_years/ s|^start = .*|start = $1|" $M; }
ch '0.0' | sed '/^\[process.charge\]/,/^ramp_years/ s|^target = .*|target = 0.0|' > /tmp/nvda_var/charge_0.toml   # no charge: 105.40
ch '0.015' > /tmp/nvda_var/charge_15.toml                                                                       # 1.5% only: 102.32
ch '{ mean = 0.2245, std = 0.04, lo = 0.05, hi = 0.40 }' > /tmp/nvda_var/charge_net.toml                        # net of sales proceeds (16.55% / 0.79 + 1.5%): 87.78
ch '{ mean = 0.211, std = 0.04, lo = 0.05, hi = 0.40 }' > /tmp/nvda_var/charge_nogross.toml                     # no tax gross-up (19.6% + 1.5%): 88.72
sed 's|^net_debt = .*|net_debt = -21141000000.0|' $M > /tmp/nvda_var/stakes_out.toml                            # stakes out of the bridge: 80.53
for f in charge_0 charge_15 charge_net charge_nogross stakes_out; do fa dcf drivers NVDA --model /tmp/nvda_var/$f.toml --no-attribution; done
fa dcf drivers NVDA --model $M --no-attribution --pset process.charge.start=0.1865   # Q3 rate (15.696 / 115.844 = 13.55%; / 0.79 + 1.5%): 90.40, mean 105.53
fa dcf drivers NVDA --model $M --no-attribution --pset process.charge.target=0.02    # acquisitions 2.0%: 84.39, mean 99.42
```

The net rate: (58.100 - 9.493) / 293.681 = 16.55% (nine months; proceeds from sales of equity securities 9.493).
The base (four-quarter rate) is `process.charge.start = 0.2631` in the model file: 19.6% / 0.79 + 1.5%.

## 6. What the price implies (section 5)

Close $145.98 on 2026-11-19 (`data/raw/NVDA/prices.json`, last row). The close of 2026-11-20 is not in the store.

```bash
P=145.98
fa dcf drivers NVDA --model $M --no-attribution --price $P                           # paths below the close: 83.8%
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param process.growth.half_life --range 0.5,2.9    # no solution
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param process.growth.start --range 0.15,1.5 --shift-bounds   # 0.8510
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param discount.erp --range 0.01,0.07 --shift-bounds        # 0.0213
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param discount.beta --range 0.5,2.0 --shift-bounds        # 0.8217
fa dcf drivers NVDA --model $M --no-attribution --price $P --implied-price $P --implied-param process.margin.target --range 0.30,0.78 --shift-bounds   # no solution
```

## 7. Automatic screen (section 6)

```bash
fa dcf auto NVDA --cache $C --as-of 2026-11-20 --explain     # a1dffb5: P10 1.18, P25 5.40, P50 16.55, P75 43.26, P90 104.57, mean 43.85; c486b86: P50 16.57, mean 44.00
```

## 8. Order of the runs (section 7)

Each line makes one file from the one before. Run `fa dcf drivers NVDA --model FILE --no-attribution` on each (P50 in the comment).

```bash
T=/tmp/nvda_var
git show 9a2e6f0:analysis/NVDA/driver_model_paths.toml > $T/v1.toml        # version 1: P50 130.89 (P10 59.06, P90 298.97, mean 162.33)
git show 41afd53:analysis/NVDA/driver_model_paths.toml > $T/v2.toml        # version 2: P50 97.65
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
# step 9 = the model file of this version: 85.07
```

The first run of version 1 without the collapse table (P50 145.95) is the version 1 file with the `[failure]` table removed. The
Python check of the growth process alone (a year below -30% on 5.1% of the paths) is a one-time script and is not kept.

## 9. The engine check

```bash
fa dcf drivers IBM          # P10 83.84, P50 151.84, P90 258.03 (unchanged by this work)
```

## What is not repeatable by a repo command

- The standard error of the beta (one-time script).
- The hand check of the cash flows (one-time Python, formula in the model file).
- The check of the growth process alone (one-time Python).
- The numbers of the 8-K page: the page is the source (ledger A2); they are not in the store as data. A request to the Head of
  Technology: the 8-K tables as data (ticket `T-20261121-dcf-engine-from-tickers`, candidate 7).
