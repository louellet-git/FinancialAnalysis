# What the market price implies (2026-11-20)

Ticket `T-20261120-dcf-market-implied`, part 4. This file shows the first run of `fa dcf implied` on the two lists of
the repo, with the defaults of `main` of 2026-11-20 (conventions of pull request 77, engine accuracy of pull request 81).
The result is a screen, not a reviewed valuation.

**Rule.** A market-implied value is a measurement. It is not an input of the model. No default and no rule of the
builder reads it (`docs/QUALITY-STANDARDS.md`, section 6). The price measures and ranks. This file changes no input and
no default.

## 1. Question

For each size class, sector and industry: what cost of capital, what revenue growth and what margin does the market
price imply, when the value engine is at its defaults? A person can answer with a number for each group and a count.

## 2. Method

- **Value at the means.** For a ticker, the builder makes the model and the engine runs the one path with each parameter at
  its mean and each shock at zero (`PathEngine::deterministic`). The value for each share is `V`. No seed, no simulation.
- **Three searches (bisection).** Each search moves one number and keeps all others. Each step builds the model again
  with changed `Rules`, so each cap, fade and guard of the builder applies.
  1. The mean of the equity risk premium, from -4% to +20%. Report: the implied premium and the implied cost of capital of year 1.
  2. A shift of the growth (the company mean and year 1), from -30 to +60 points. Report: the shift and the compound
     growth of years 1 to 10 of the path.
  3. A shift of the margin (start and target), from -50 to +50 points. Report: the shift and the free cash flow margin of
     year 10 (after SBC and the charge).
- **Bound.** A search with no solution in its range gives the bound and a flag (`<`: the price needs a lower number than the
  bound; `>`: a higher number). `nm`: the value does not rise or fall in the expected direction on the ends of the range
  and at the model number. A row with `nm` has no number for that search.
- **List premium.** One number: the equity risk premium for which the median of `V` / price over the list is 1.0.
- **Groups.** P25, P50 and P75 of each measure. A cell has `*` when more than 20% of its values are bounds: a quartile of
  censored values is not a measured number. Sector: Fama-French 12 name of the SIC code. Industry: the SIC code with its
  description (industries with 5 tickers or more in the command; 10 or more in this file).
- **Price.** The last close of the price cache (2026-11-18). The price also gives the equity weight of the cost of capital
  (`E / (E + debt)`), as in `fa dcf auto`. So `V` depends on the price a little.

## 3. Data and commands

Cache: the data store `louellet-git/fa-data`, commit 08361bd, data of 2026-11-19 (a read-only copy with the SIC codes).
Model date `--as-of 2026-11-19`. Code: branch `claude/dcf-market-implied` with `main` of 2026-11-20 (pull requests 77, 81).

```
fa dcf implied --tickers-file rust/mapcheck/universe/sp500.txt   --cache <store copy> --as-of 2026-11-19 --out <file>
fa dcf implied --tickers-file rust/mapcheck/universe/next500.txt --cache <store copy> --as-of 2026-11-19 --out <file>
fa dcf screen  --tickers-file rust/mapcheck/universe/sp500.txt   --cache <store copy> --as-of 2026-11-19 --out <file>   (same for next500)
```

Each `implied` command ran two times. The CSV files are the same bytes in both runs, for both lists. Time (4 threads):
5.9 s and 4.5 s for the second run of sp500 and next500 (the first run of each was slower, 31 s and 11 s, because the
files were not in the memory of the machine). The CSV files are not in the repo.

| List | Tickers | With a value | With a price | Refused |
|---|---|---|---|---|
| `sp500.txt` | 503 | 431 | 430 | 72: reit 27, bank 23, insurer 19, new 2, history too short 1 |
| `next500.txt` | 500 | 395 | 391 | 105: bank 30, reit 26, insurer 19, new 10, mapping FAIL 7, history short 6, no revenue 5, fund 1, non-usd 1 |

Searches (tickers with a price):

| List | Premium `<` / `>` / `nm` | Growth `<` / `>` / `nm` | Margin `<` / `>` / `nm` |
|---|---|---|---|
| `sp500.txt` | 57 / 11 / 24 | 8 / 20 / 25 | 0 / 29 / 0 |
| `next500.txt` | 68 / 24 / 87 | 9 / 18 / 87 | 0 / 48 / 0 |

## 4. Why the value at the means and not the P50

A DCF with a risk-adjusted rate gives the present value of the EXPECTED cash flows. So the price compares with an
expected value, not with the median over the draws (decision on the ticket, 2026-11-20; the reason does not use the price).
The path at the means is a deterministic estimate of the expected value. The P50 is lower when the distribution is
skewed to the right, which it is here.

Value at the means against the P50 of `fa dcf auto` (10,000 paths, seed 7, data of 2026-11-19, defaults of `main`):

| Ticker | `V` at the means | P50 |
|---|---|---|
| AAPL | $144.87 | $132.89 |
| ABT | $48.44 | $42.45 |
| ADBE | $271.01 | $248.47 |
| IBM | $134.95 | $95.84 |
| KO | $49.35 | $36.38 |

The median of P50 / price of `fa dcf screen` (4,000 paths) is 0.476 for sp500 and 0.579 for next500. The median of
`V` / price of `fa dcf implied` is 0.63 and 0.76. They are two different measures. The screen number is the lower one.
Do not compare a number of one with a number of the other.

## 5. Results

Each cell: P25 P50 P75. `n`: tickers with a value. The three difference columns are implied less model, in points of
percent (a number in brackets after a cell is the count of that measure when it is lower than `n`). The last column is the
value at the list premium over the price.

### 5.1 List premium

| List | Tickers in the median | Median `V` / price at the model premium (4.41%) | Premium for a median of 1.0 |
|---|---|---|---|
| `sp500.txt` | 430 | 0.634 | 2.32% |
| `next500.txt` | 391 | 0.759 | 3.12% |

### 5.2 `sp500.txt`: all, size, sector

| Group | n | `V` / price | Cost of capital | 10-year growth | Margin of year 10 | `V(list premium)` / price |
|---|---|---|---|---|---|---|
| all | 431 | 0.34 0.63 1.20 | -3.21 -1.35 1.10 (400) | -1.24 2.44 8.63 (406) | -1.41 4.27 15.76 | 0.55 1.00 1.85 |
| size <1B | 1 | 0.90 | -0.47 | 1.38 | 5.11 | 1.41 |
| size 1-10B | 155 | 0.25 0.59 1.06 | -3.29 -1.49 0.46 (134) | -0.77 4.71 10.25 (137) | -0.75 6.82 22.13 | 0.42 0.97 1.63 |
| size 10-30B | 162 | 0.30 0.60 1.20 | -3.32 -1.84 0.86 (156) | -1.18 2.62 8.50 (157) | -1.39 4.60 13.96 | 0.51 0.89 1.87 |
| size 30-100B | 78 | 0.46 0.84 1.30 | -2.61 -0.84 1.43 (75) | -1.37 1.07 5.10 (77) | -1.27 1.30 9.64 | 0.74 1.20 1.81 |
| size >100B | 35 | 0.45 1.36 2.36 | -3.31 1.25 4.10* (34) | -4.79 -1.13 9.70 (34) | -2.93 -0.46 6.87 | 0.67 1.67 3.51 |
| BusEq | 97 | 0.16 0.37 0.60 | -6.05 -3.29 -1.47 (81) | 4.71 10.06 18.72 (89) | 6.67 18.89 46.29* | 0.25 0.65 0.96 |
| Chems | 16 | 0.33 0.65 1.15 | -2.90 -1.87 0.45 (15) | -0.91 2.67 5.56 (15) | -1.24 4.00 11.00 | 0.50 1.04 1.72 |
| Durbl | 7 | 1.29 3.10 4.49 | 1.49 3.67 12.70* | -14.43 -3.65 -1.62* | -4.60 -4.12 -2.54 | 2.00 4.66 7.07 |
| Enrgy | 15 | 1.57 1.71 2.32 | 2.07 2.60 4.56 | -4.25 -2.98 -2.32 | -8.10 -4.61 -3.70 | 2.40 2.70 3.63 |
| Hlth | 40 | 0.30 0.58 0.88 | -5.02 -1.73 -0.32* (39) | 0.35 2.36 9.80 (39) | 1.16 6.43 18.00 | 0.48 0.79 1.41 |
| Manuf | 50 | 0.43 0.73 0.95 | -2.76 -1.28 -0.19 (48) | 0.41 2.63 6.47 (49) | 0.51 4.21 8.50 | 0.70 1.13 1.58 |
| Money | 29 | 0.73 1.11 1.45 | -0.89 0.36 1.67* | -3.95 -0.77 1.37* (27) | -2.81 -0.87 5.11 | 1.26 1.59 2.23 |
| NoDur | 25 | 0.80 1.48 1.86 (24) | -1.04 1.00 2.30 (24) | -2.07 -1.41 1.03 (24) | -3.44 -1.65 2.09 (24) | 1.19 2.00 2.46 (24) |
| Other | 71 | 0.50 0.77 1.23 | -2.41 -1.13 1.04 (70) | -1.67 2.16 6.38 (70) | -2.31 1.90 10.18 | 0.79 1.13 2.07 |
| Shops | 39 | 0.50 0.82 1.34 | -2.66 -0.66 1.01 (38) | -1.34 1.87 5.40 (38) | -0.83 2.78 6.15 | 0.77 1.25 1.90 |
| Telcm | 8 | 1.13 1.70 2.56 | 0.64 2.21 3.52* | -3.29 -2.10 -0.58 | -6.14 -4.13 -1.36 | 1.47 2.35 3.61 |
| Utils | 34 | -0.75 -0.10 0.42 | -2.75 -2.40 -0.61* (26) | 1.89 6.10 13.74 (25) | 2.80 7.72 15.40 | -0.79 0.08 0.74 |

### 5.3 `next500.txt`: all, size, sector

| Group | n | `V` / price | Cost of capital | 10-year growth | Margin of year 10 | `V(list premium)` / price |
|---|---|---|---|---|---|---|
| all | 395 | 0.09 0.76 1.51 (391) | -2.87 -0.22 2.57* (323) | -3.56 -0.26 6.89 (304) | -2.15 2.72 19.16 (391) | 0.12 1.00 1.97 |
| size <1B | 59 | -0.77 -0.15 0.12 | -12.38 -9.96 -2.48* (28) | 2.65 10.60 27.88 (18) | 27.07 50.00 50.00* | -1.08 -0.18 0.14 |
| size 1-10B | 257 | 0.22 0.74 1.30 (255) | -2.97 -0.73 1.56 (220) | -2.75 1.35 7.36 (210) | -2.05 3.20 14.21 | 0.32 0.95 1.69 |
| size 10-30B | 62 | 1.05 1.60 2.40 (61) | 0.36 2.24 4.56 (59) | -3.89 -2.73 -0.27 (60) | -3.64 -1.68 -0.20 (61) | 1.39 2.04 3.03 |
| size 30-100B | 16 | 2.38 3.64 10.04 (15) | 2.73 4.64 6.10* (15) | -10.24 -5.45 -2.64 (15) | -3.74 -2.39 -1.63 (15) | 3.04 4.37 12.54 |
| size >100B | 1 | 35.10 | 9.96* | -26.29* | -2.87 | 44.69 |
| BusEq | 88 | -0.23 0.17 0.58 | -9.98 -3.15 -2.01* (61) | 1.39 9.35 16.78 (52) | 6.73 19.76 44.16* | -0.33 0.21 0.75 |
| Chems | 9 | 0.69 1.10 1.55 | -1.38 0.41 1.54 | -1.48 -1.01 3.81 | -2.08 -0.67 3.59 | 0.94 1.46 1.97 |
| Durbl | 9 | 0.66 1.42 1.72 | -0.02 1.93 2.50 (8) | -3.22 -2.18 0.35 (8) | -3.12 -1.50 3.24 | 0.90 1.87 2.27 |
| Enrgy | 13 | 2.27 2.89 5.02 (12) | 3.70 6.82 8.38* (12) | -9.31 -6.81 -4.68 (12) | -20.82 -13.27 -5.83 (12) | 2.95 3.74 6.27 (12) |
| Hlth | 51 | -0.18 0.11 1.03 | -3.56 -0.70 1.16* (31) | -2.69 0.24 5.37 (28) | -0.27 8.98 50.00* | -0.26 0.17 1.32 |
| Manuf | 51 | 0.49 0.76 1.14 (50) | -2.36 -1.11 0.79 (48) | -0.91 2.93 7.47 (48) | -0.59 2.79 8.66 (50) | 0.65 1.01 1.45 |
| Money | 37 | -0.03 0.90 1.72 | -3.88 0.60 4.65* (31) | -14.89 -2.62 2.35 (26) | -3.73 1.22 27.81 | -0.03 1.21 2.27 |
| NoDur | 15 | 0.84 2.21 3.83 | -0.80 2.99 3.85* | -4.84 -3.97 1.53 | -5.75 -2.85 1.18 | 1.07 2.83 4.33 |
| Other | 59 | 0.49 0.85 1.72 | -1.25 -0.21 2.91 (53) | -3.73 -0.12 3.27 (51) | -3.46 0.71 9.16 | 0.67 1.15 2.31 |
| Shops | 43 | 0.74 1.37 2.46 (42) | -0.50 1.75 4.54 (40) | -4.33 -1.73 1.13 (40) | -2.36 -1.47 1.05 (42) | 0.93 1.79 3.21 (42) |
| Telcm | 4 | 0.02 1.41 3.91 | -6.41 -0.85 3.87* | -3.84 7.61 22.35* | -10.22 -0.54 17.80* | 0.06 1.82 4.90 |
| Utils | 16 | -0.31 1.31 2.72 (15) | 0.23 2.44 4.13 (11) | -4.35 -2.36 -0.39 (11) | -10.42 -2.06 12.18 (15) | -0.25 1.66 3.47 (15) |

### 5.4 Industries with 10 tickers or more

| List | Industry (SIC) | n | `V` / price | Cost of capital | 10-year growth | Margin of year 10 |
|---|---|---|---|---|---|---|
| sp500 | 7372 Services-Prepackaged Software | 20 | 0.03 0.28 0.50 | -5.41 -3.18 -2.36 (14) | 7.67 11.52 17.37* (15) | 13.18 20.82 40.76 |
| sp500 | 3674 Semiconductors & Related Devices | 16 | 0.11 0.25 0.45 | -7.72 -5.38 -3.25 (12) | 8.80 18.72 24.13 (15) | 17.69 49.91 50.00* |
| sp500 | 7389 Services-Business Services, NEC | 16 | 0.46 0.87 1.45 | -2.02 0.41 1.34 (15) | -1.76 -0.37 6.43 (15) | -5.20 1.80 22.30 |
| sp500 | 4911 Electric Services | 15 | -0.53 0.09 0.73 | -2.42 -2.20 -0.42* (12) | 0.35 6.01 6.85 (11) | 1.81 3.60 13.15 |
| sp500 | 4931 Electric & Other Services Combined | 12 | -1.18 -0.59 0.35 | -2.69 -2.44 -2.13* (8) | 5.20 8.94 20.73 (8) | 5.04 9.71 13.72 |
| sp500 | 2834 Pharmaceutical Preparations | 11 | 0.49 0.55 0.76 | -3.61 -2.70 -0.93 | 1.06 2.45 4.51 | 3.20 10.48 18.24 |
| sp500 | 3841 Surgical & Medical Instruments & Apparatus | 10 | 0.26 0.45 0.78 | -7.04 -3.68 -0.97* | 1.69 3.81 14.78 | 3.76 6.62 14.42 |
| next500 | 7372 Services-Prepackaged Software | 40 | -0.41 -0.02 0.43 | -11.29 -3.48 -2.63* (23) | 6.27 10.32 17.36 (18) | 12.73 25.73 45.44* |
| next500 | 2834 Pharmaceutical Preparations | 24 | -0.30 -0.02 0.51 | -4.26 -1.92 0.19* (11) | -1.82 3.52 13.48 (10) | 5.77 26.80 50.00* |
| next500 | 3674 Semiconductors & Related Devices | 13 | 0.01 0.11 0.65 | -13.06 -9.29 -1.93* (10) | 3.69 22.80 32.38* (9) | 7.56 42.79 50.00* |
| next500 | 6282 Investment Advice | 11 | 0.08 0.79 1.24 | -3.76 -0.40 2.16 (9) | -4.18 0.27 5.11 (8) | -3.10 2.95 24.91 |
| next500 | 1311 Crude Petroleum & Natural Gas | 10 | 2.39 2.89 4.77 | 4.48 6.82 8.56* | -9.16 -6.81 -4.98 | -25.00 -16.06 -10.40 |
| next500 | 2836 Biological Products, (No Diagnostic Substances) | 10 | -0.20 0.08 0.59 | -10.31 -1.30 0.13* (5) | -1.31 1.80 9.83 (4) | 14.28 49.59 50.00* |
| next500 | 7389 Services-Business Services, NEC | 10 | -0.33 0.79 1.55 | -4.17 -0.73 2.21* (9) | -5.29 1.80 2.64 (7) | -4.51 2.17 18.15 |

## 6. What the tables show

- The model values the median ticker below its price: `V` / price is 0.63 (sp500) and 0.76 (next500). To make the median
  ticker equal its price, the premium must fall from 4.41% to 2.32% (sp500) and to 3.12% (next500). The list premium is
  one number for a list. It is not a forecast of the market premium.
- Software, semiconductors and other business equipment (BusEq) have a low `V` / price (median 0.37 in sp500). The price
  needs a lower cost of capital (3 points less), or more growth (10 points a year more), or a higher margin of year 10
  (19 points more) than the model has. The margin cells for BusEq and the semiconductor industries have `*`: they are
  mostly at the bound of +50 points, so their quartiles are lower bounds.
- Energy, durable goods, telecom and non-durable goods have a `V` / price above 1 (median 1.7 for energy in sp500). The
  price needs a higher cost of capital, or less growth, or a lower margin than the model has.
- Larger tickers have a higher `V` / price (size >100B: 1.36; 1-10B: 0.59 in sp500). The model and the price rank the
  size classes in the same way in both lists, except the size classes with few tickers.
- The sector order of `V` / price has the same ends in the two lists (energy and non-durable goods high; BusEq low;
  health care also low in next500). The middle sectors change places.

**What a person must NOT read into these tables.** They are not a signal and not a price target. A low `V` / price
does not say that a stock is expensive: it says that the model with its defaults gives a lower value than the price. The
model can be wrong for the group (see section 7). The table does not say which of the three numbers (cost of capital,
growth, margin) the market uses: each search moves one number and keeps the others, so the three results are three
different explanations of the same gap, not a sum. Do not use the table to change an input or a default.

## 7. Limits

- **Free-cash-flow DCF is the wrong tool for some classes.** The bank, insurer, reit and fund templates are refused
  (sp500: 69 tickers; next500: 76). The group "Money" has the financial companies that the template let through (asset
  managers, brokers, exchanges). Their cash flow is not a good measure of value. Read `Money` with this limit.
- **Utilities are not refused.** The utility groups have a negative terminal cash flow or a negative value for many
  tickers; the cause is not tested here (a capex that is large against the margin is one possible cause). The median
  `V` / price of Utils is -0.10 (sp500) and the P25 is
  negative; 26 of 34 tickers have a premium number. Read the utility cells as "the model does not fit", not as a market
  view. The same is true, in a smaller part, for pharma and biotech with a negative margin in the next500 list
  (`V` / price median -0.02).
- **`nm` and bounds.** `nm` rows (24 and 87 for the premium search) have no number: the terminal cash flow is negative,
  so the value rises with the premium. They are not in the cells of the difference columns (see the counts in
  brackets). The next500 list has 87 such rows of 391 (22%), so its difference columns describe a part of the list.
  Rows at a bound are in the cells at the bound value; a `*` marks a cell with more than 20% of them.
- **Growth and margin ranges.** A bound at +60 or +50 points can be a clamp of the engine (the limits of the growth and
  of the margin), not only a bound of the search.
- **Price.** The price is the last close of the cache (2026-11-18), not the close of the model date. The equity weight
  uses the price. A growth shift above the long-run level changes the fade, and with it the acquisition charge
  (`acq_fade`), because the builder applies its rules to the shifted mean.
- **Survivorship.** The lists are the members of the universe files of today. They have companies that did well enough to
  be in an index. A list of 2016 would give other numbers.
- **The code of today.** The SIC code is the code of the filer today (trap #170). This command values today only, so the
  limit does not apply to it. The result is not a history of groups.
- **Data date.** The data is of 2026-11-19 (store commit 08361bd). The numbers change with the data and with the defaults.
- **Size class.** A class with 1 ticker (<1B in sp500; >100B in next500) is one ticker, not a median.
- **One run.** This is one run. No skeptic, reviewer or auditor has looked at it. A number here is not recalculated by
  the auditor.

## 8. What stays open

- The mean of the simulated value with its standard error (the engine ticket) as a check of the path at the means.
  Owner: Head of Fundamental Research.
- A rule for capex of regulated utilities, or a refusal of the utility class. Owner: the owner of the conventions decision.
- A decision on the free-cash-flow DCF for the financial companies of the group `Money`. Owner: the owner.
- A history of groups (SIC code at the date of the model). Owner: Head of Technology.
- A check by a skeptic and an auditor before a person uses a table for a decision.
- A run with the close of the model date as the price. Owner: Head of Technology (the price cache).
