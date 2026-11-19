# JNJ first pass: the command for each number

Run each command from the repository root. Build the program one time:
`cargo build --release --manifest-path rust/Cargo.toml`. Below, `fa` is
`rust/target/release/fa`. Each run has seed 7 and 10,000 simulations (the defaults).
The model is `analysis/JNJ/driver_model.toml`. The Python runner that printed the
numbers of `FIRST-PASS.md` is deleted; its printed text is the record in
`rust/tests/fixtures/dcf/runner_JNJ.txt`, and `cargo test` compares each number with it.

## Verdict and engine check

```bash
fa dcf drivers JNJ --price 255.56
```

This gives P10 to P90, the share of paths below the price (99.3%), the terminal value
share, and the engine check against a hand DCF ($112.0194).

## One change, rest at base (table of FIRST-PASS.md)

The growth credit moves with the M&A recurrence `f`: `g_im = 4.0% + f x 2.0%` and
`g_medtech = 3.5% + f x 2.0%`.

```bash
fa dcf drivers JNJ --set ma_recurrence=0 --set g_im=0.04 --set g_medtech=0.035       # f = 0: $154.95
fa dcf drivers JNJ                                                                    # f = 0.5: $112.64
fa dcf drivers JNJ --set ma_recurrence=1 --set g_im=0.06 --set g_medtech=0.055       # f = 1: $63.70
fa dcf drivers JNJ --sensitivity beta=0.45,0.85 --sensitivity rf=0.046               # $136.02 / $95.28 / $126.37
fa dcf drivers JNJ --price 255.56 --set ma_recurrence=0 --set g_im=0.04 \
  --set g_medtech=0.035 --set beta=0.45 --set rf=0.046                               # stacked: $210.83, 72.1% below
fa dcf scenarios JNJ                                                                  # classic engine, generic presets
```

The last line reads `data/processed/JNJ.json` (use `--data-dir` for another folder).
Its numbers depend on that file. The file changed after the first pass, so the base and
bull values of the table ($142.55 / $200.59) do not come again from a new file.

## What $255.56 requires

```bash
fa dcf drivers JNJ --implied-price 255.56 --implied-driver rf --range 0.01,0.2
fa dcf drivers JNJ --set ma_recurrence=0 --set g_im=0.04 --set g_medtech=0.035 \
  --implied-price 255.56 --implied-driver core_margin --range 0,0.9
fa dcf drivers JNJ --set ma_recurrence=0 --set g_im=0.04 --set g_medtech=0.035 \
  --implied-price 255.56 --implied-driver terminal_growth --range 0,0.2
fa dcf drivers JNJ --set ma_recurrence=0 --set g_im=0.04 --set g_medtech=0.035 \
  --implied-price 255.56 --implied-driver rf --range 0.01,0.2
```

The first line gives the rf at base (2.27%); the others give, with f = 0, the core
margin (32.3%), the terminal growth (4.76%) and the rf (3.15%). The text of
`FIRST-PASS.md` has 2.26%, 32.4%, 4.78% and 3.13%: it used a parked solver of an older
commit (`076a953`), which is not in the repo. The difference is 0.02 percentage points
at most.
