# ANT first pass: the command for each number

Run each command from the repository root. Build the program one time:
`cargo build --release --manifest-path rust/Cargo.toml`. Below, `fa` is
`rust/target/release/fa`. Each run has seed 7 and 10,000 simulations (the defaults).
The model is `analysis/ANT/driver_model.toml`. The numbers are the numbers of
`FIRST-PASS.md`. The Python runner that made them is deleted; its printed text is the
record in `rust/tests/fixtures/dcf/runner_ANT.txt`, and `cargo test` compares each
number with it.

`--shift-bounds` moves the bounds of a driver with its mean. The runner did this for
each changed mean. `--keep-year1-margin` keeps the margin of year 1 when the mature
margin (`core_margin`) changes.

## Value range, engine check, key drivers

```bash
fa dcf drivers ANT --price 83 --price 105 --path
```

This gives the P10 to P90 values, the paths below each price, the WACC inputs, the share
of the terminal value, the equity value and EV at P50, the market value at each price,
the revenue and margin at the means in years 1, 3, 5 and 10, the year-10 revenue P10,
P50 and P90, the engine check against a hand DCF, and the effect of each driver
(minus 1 std and plus 1 std).

## What the market price implies (table of FIRST-PASS.md)

Row (a), growth from year 2. Row (b), mature margin (core margin, before SBC). Row (c), beta.
Change `83` to `105` for the second column.

```bash
fa dcf drivers ANT --shift-bounds --implied-price 83 --implied-driver g_total --range 0,0.8
fa dcf drivers ANT --shift-bounds --keep-year1-margin --implied-price 83 --implied-driver core_margin --range 0.10,1.0
fa dcf drivers ANT --shift-bounds --implied-price 83 --implied-driver beta --range 0,3
```

Row "Joint": growth, with the margin 19% after SBC (25% before SBC) and beta 1.10.

```bash
fa dcf drivers ANT --shift-bounds --keep-year1-margin --set beta=1.10 --set core_margin=0.25 \
  --implied-price 83 --implied-driver g_total --range 0,0.8
```

The mature margin after SBC is the core margin minus 6%. The Damodaran case (revenue
$1,011B in year 10, margin 19% after SBC; P50 $59.08):

```bash
fa dcf drivers ANT --shift-bounds --keep-year1-margin --set core_margin=0.25 --set g_total=0.28612935
```

## Key drivers (table of FIRST-PASS.md)

```bash
fa dcf drivers ANT --shift-bounds --sensitivity g_total=0.10,0.30            # growth 10% / 30%
fa dcf drivers ANT --sensitivity beta=1.10,1.70                              # beta locus
fa dcf drivers ANT --year1-revenue Total=80.6e9                              # year-1 revenue $80.6B
fa dcf drivers ANT --year1-revenue Total=125e9                               # year-1 revenue $125B
fa dcf drivers ANT --ramp margin_mix_drift=-0.29:0:6 --ramp sbc_ratio=0.25:0.06:6    # mature in year 6
fa dcf drivers ANT --ramp margin_mix_drift=-0.29:0:10 --ramp sbc_ratio=0.25:0.06:10  # mature in year 10
fa dcf drivers ANT --ramp sbc_ratio=0.15:0.06:8                              # SBC year 1 at 15%
fa dcf drivers ANT --shift-bounds --sensitivity rf=0.0407,0.0607
fa dcf drivers ANT --shift-bounds --keep-year1-margin --sensitivity core_margin=0.16,0.28,0.34
```

The last two lines are not rows of the table; the others give the "Mean 10% / 30%",
the beta locus and the margin-ramp cells.

## Horizon of 15 years

```bash
fa dcf drivers ANT --years 15                                                # growth 20%: $48.84
fa dcf drivers ANT --years 15 --shift-bounds --set g_total=0.16674398        # growth 16.7%: $36.67
```

## Bridge (items 4 and 5 of FIRST-PASS.md)

```bash
fa dcf drivers ANT --shares 17290746913                                      # no subsidiary shares: $32.49
fa dcf drivers ANT --shares 12372801314 --net-debt -120935000000             # outstanding shares only: $36.53
fa dcf drivers ANT --net-debt -172094000000                                  # leases not started as debt: $27.74
fa dcf drivers ANT --net-debt 241623000000                                   # Damodaran debt added: $4.93
```

## Limits 1 and 4 of FIRST-PASS.md

```bash
fa dcf drivers ANT --fade --commitments 70.7e9:451.6e9:4.25:0.31
```

`--fade`: growth falls from 38.9% in year 2 to 2.25% in year 10 with the same year-10
revenue; the hand DCF is $31.81 against $31.70. `--commitments FIRST:TOTAL:YEARS:OTHER`:
committed spend of $70.7B in year 1 and $451.6B to the end of year 4.25 (2030-12-31),
the rest in equal parts, and 31% of revenue for all other cash costs. It gives the share
of revenue (73% at mean growth, 86% at P10 growth), the margins before SBC of years 1 to
4 (model against committed spend) and the effect of $-1.85 a share, which is not in the
base. The base rate (AMZN, 27.4% a year) is a hand calculation: $35.282B x 1.274^10.
