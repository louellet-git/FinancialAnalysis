# Where the value of the S&P 100 went in the week to 2026-11-20

> **DRAFT.** Two data fixes are open: (1) V, BRK-B, COP and NEE have no model and V and BRK-B
> have no weight, because the point-in-time mapping does not read the 10-K filing file (fix by the
> Head of Technology in progress); (2) IBM and JNJ have no close after 2026-11-18, so the command
> uses the close of 2026-11-18 for 2026-11-20. The numbers change after the fixes. No review yet.

For: the Head of Fundamental Research. Request of the owner (Luc), 2026-11-21: put each part of the
drop of the week on a part of the valuation, for the S&P 100 and for its 10 largest companies (the
"S&P 10"). Rule: no input of the model is set from the price. The price only measures.

Question: how many points of the change of the S&P 100 from the close of 2026-11-13 to the close
of 2026-11-20 does each part of the model valuation give?

"Points" are percentage points of the market value of the list at the close of 2026-11-13
(price x shares). All numbers come from `fa dcf attribute` (section 6).

## 1. Answer

- The S&P 100 list (98 companies with two prices and a share count, $48,788.8 billion at
  2026-11-13) changed by **-9.40%** (value-weighted). The S&P 10 ($26,644.4 billion) changed by **-9.88%**.
- Premium view first, S&P 100: facts +0.01, time +0.09, rates +4.93, implied premium -10.71,
  implied growth -1.68, no model value -2.03. The sum is -9.40 points.
- Premium view first, S&P 10: facts +0.00, time +0.08, rates +5.57, implied premium -12.44,
  implied growth -1.68, no model value -1.40. The sum is -9.88 points.
- The 10-year zero yield went down. So the model value went up. The price went down. So the
  price implies a higher equity risk premium. This plug is the largest part.
- The "growth view first" reading gives the same total, with the plug on growth (S&P 100: rates
  +3.94, implied growth -11.01). The model cannot tell the two readings apart: both give the price.

## 2. The S&P 100 bridge

Points of the market value of the whole list at 2026-11-13. Source: section "(4) FULL BRIDGE" of
`analysis/priors/data/sp100_week_2026-11-20.txt`. The sector lines use the premium-first reading.

| Line | n | Weight | Change | Facts | Time | Rates | Premium | Growth | Resid. | No model | Sum |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| **All, premium first** | 98 | 100.0% | -9.40% | +0.01 | +0.09 | +4.93 | -10.71 | -1.68 | +0.00 | -2.03 | **-9.40** |
| **All, growth first** | 98 | 100.0% | -9.40% | -0.00 | +0.13 | +3.94 | -0.43 | -11.01 | -0.00 | -2.03 | **-9.40** |
| BusEq | 26 | 53.5% | -11.86% | -0.00 | +0.05 | +3.23 | -8.09 | -0.94 | +0.00 | -0.60 | -6.34 |
| Money | 14 | 7.6% | -10.14% | +0.00 | +0.00 | +0.03 | +0.00 | -0.16 | +0.00 | -0.65 | -0.77 |
| Shops | 8 | 10.9% | -4.64% | +0.00 | +0.01 | +0.19 | -0.23 | -0.03 | +0.00 | -0.44 | -0.51 |
| Hlth | 12 | 8.6% | -5.63% | +0.00 | +0.01 | +0.62 | -0.69 | -0.42 | -0.00 | +0.00 | -0.49 |
| Manuf | 10 | 4.2% | -10.23% | +0.00 | +0.00 | +0.19 | -0.54 | -0.07 | +0.00 | +0.00 | -0.43 |
| Durbl | 2 | 3.0% | -11.23% | +0.00 | +0.00 | +0.00 | +0.00 | -0.01 | +0.00 | -0.32 | -0.34 |
| Other | 9 | 3.7% | -6.41% | +0.01 | +0.01 | +0.26 | -0.51 | +0.00 | +0.00 | +0.00 | -0.24 |
| Enrgy | 3 | 2.7% | -5.64% | +0.00 | +0.00 | +0.11 | -0.24 | +0.00 | +0.00 | -0.02 | -0.15 |
| NoDur | 5 | 2.4% | -4.12% | +0.00 | +0.00 | +0.13 | -0.23 | +0.00 | -0.00 | +0.00 | -0.10 |
| Chems | 2 | 1.3% | -1.81% | +0.00 | +0.00 | +0.08 | -0.10 | +0.00 | -0.00 | +0.00 | -0.02 |
| Telcm | 4 | 1.3% | -0.68% | +0.00 | +0.00 | +0.06 | -0.06 | -0.01 | +0.00 | +0.00 | -0.01 |
| Utils | 3 | 0.8% | -0.92% | +0.00 | +0.00 | +0.04 | -0.01 | -0.04 | -0.00 | -0.01 | -0.01 |

- **Order effect** (the line "the other order", plug before rates): premium first gives rates
  +4.41, premium -10.28, growth -1.60; growth first gives rates +3.38, premium -0.37, growth -10.50.
- **Rounding.** Each cell has 2 decimals. So a row of cells can differ from its sum by up to 0.02
  points. A recalculation from the CSV (part x weight) gives, premium first: facts +0.005, time
  +0.087, rates +4.927, premium -10.707, growth -1.677, no model -2.034. These add to -9.399, the
  change of the list.
- BusEq (computers, chips, software) has 53.5% of the weight. It gives -6.34 of the -9.40 points.
- The largest contributions (points): NVDA -1.86, GOOG -0.73, MSFT -0.60, AMZN -0.44 (no model
  value), MU -0.43, AVGO -0.43, META -0.43, AAPL -0.41, AMD -0.37 (no model value), TSLA -0.32
  (no model value).

## 3. The S&P 10, company by company

The S&P 10 is the list `analysis/priors/data/sp10_2026-11-13.txt`: the 10 largest companies of
the S&P 100 by market value at the close of 2026-11-13. GOOG carries the weight of Alphabet (GOOG
and GOOGL). Points of the market value of the S&P 10 at 2026-11-13, premium-first reading. Source:
the per-ticker lines of section "(4) FULL BRIDGE" of `sp10_week_2026-11-20.txt`. The weight and
the last column come from `sp10_week_2026-11-20.csv`. The residual is 0.00 in each row.

| Ticker | Weight | Change | Facts | Time | Rates | Premium | Growth | No model | Sum | Implied value, D1 to D2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| NVDA | 14.84% | -22.92% | +0.00 | +0.00 | +0.25 | -3.66 | +0.00 | +0.00 | -3.40 | premium -2.20% to 0.07% |
| GOOG | 13.62% | -9.75% | +0.00 | +0.02 | +1.31 | -2.65 | +0.00 | +0.00 | -1.33 | premium 1.17% to 1.65% |
| MSFT | 14.64% | -7.51% | +0.00 | +0.02 | +1.22 | -2.33 | +0.00 | +0.00 | -1.10 | premium 1.73% to 2.19% |
| AMZN | 10.95% | -7.44% | +0.00 | +0.00 | +0.00 | +0.00 | +0.00 | -0.81 | -0.81 | no view |
| AVGO | 6.38% | -12.28% | +0.00 | +0.00 | +0.23 | +0.00 | -1.02 | +0.00 | -0.78 | 10-year growth 30.66% to 28.17% |
| META | 6.20% | -12.60% | +0.00 | +0.01 | +0.10 | -0.89 | +0.00 | +0.00 | -0.78 | premium -0.04% to 1.03% |
| AAPL | 19.51% | -3.85% | +0.00 | +0.02 | +2.06 | -2.84 | +0.00 | +0.00 | -0.75 | premium 1.11% to 1.45% |
| TSLA | 5.19% | -11.38% | +0.00 | +0.00 | +0.00 | +0.00 | +0.00 | -0.59 | -0.59 | no view |
| LLY | 4.84% | -6.68% | +0.00 | +0.01 | +0.33 | +0.00 | -0.66 | +0.00 | -0.32 | 10-year growth 28.70% to 27.10% |
| WMT | 3.84% | -0.11% | +0.00 | +0.00 | +0.07 | -0.07 | +0.00 | +0.00 | -0.00 | premium -2.12% to -1.79% |
| **All** | 100.0% | **-9.88%** | +0.00 | +0.08 | +5.57 | -12.44 | -1.68 | -1.40 | **-9.88** | |

- **The columns add to the "All" line.** The sums of the rows are: facts +0.00, time +0.08, rates
  +5.57, premium -12.44, growth -1.68, no model -1.40. These are the cells of the "All" line. The
  sum column of the rows adds to -9.86. The difference of 0.02 points to -9.88 comes from the
  rounding of each cell to 2 decimals.
- **Check at full precision** (CSV part x weight): time +0.077, rates +5.572, premium -12.445,
  growth -1.675, no model -1.405. These add to -9.876, the change of the S&P 10 (-9.876%).
- **Order effect** (plug before rates): rates +4.59, premium -11.55, growth -1.59. Growth first:
  rates +3.62, growth -12.23; in the other order rates +3.12, growth -11.73.

The three largest contributors:

- **NVDA, -3.40 points.** The price fell 22.92%. The lower rates gave +0.25 points. The implied
  premium went from -2.20% to 0.07% and gave -3.66 points.
- **GOOG, -1.33 points.** The rates gave +1.31 points. The implied premium went from 1.17% to 1.65%
  and gave -2.65 points.
- **MSFT, -1.10 points.** The rates gave +1.22 points. The implied premium went from 1.73% to 2.19%
  and gave -2.33 points.

## 4. What each piece is

- **Facts.** The model inputs from the SEC 10-K and 10-Q filings made in the week. An earnings
  press release is not in it. Two S&P 100 companies (CSCO, DIS) filed in the week; no S&P 10
  company filed. The part is +0.01 points (premium first).
- **Time.** The roll of the valuation date from 2026-11-13 to 2026-11-20. The cash flows come one
  week nearer, so the value goes up a little.
- **Rates.** The yield curve of 2026-11-20 in place of the curve of 2026-11-13. The curve is the
  NSS fit to the Treasury par yield curve. Its 10-year zero yield was 5.1288% on 2026-11-13 and
  4.9029% on 2026-11-20. A lower discount rate gives a higher value.
- **Implied premium.** The equity risk premium that makes the model value equal to the price.
  The part is the change of this premium in the week. It is the plug of the bridge.
- **Implied growth.** The growth shift that makes the model value equal to the price, at the
  model premium of 4.41%. The bridge uses it where the premium view has no value (AVGO, LLY).
- **No model value.** The command gives no split, so the full change of the company goes in this
  column. In the S&P 100: 20 companies, 19.8% of the weight, -2.03 points. The reasons (refusal
  column of the CSV):
  - "template bank: no model": JPM, BAC, MS, GS, WFC, C, AXP, SCHW, COF, BNY, USB.
  - "template insurer: no model": BLK. "template reit: no model": AMT.
  - "mapping status FAIL": NEE, COP (data fix (1)). V and BRK-B have the same refusal and no weight.
  - No view: AMZN, TSLA, AMD, INTC, PLTR. The model has a value, but the CSV flags
    `premium-view:none growth-view:none`. The two searches find no exact premium and no exact
    growth shift that give the price. The refusal column is empty, so the files do not give the
    reason. In the S&P 10, AMZN and TSLA give the -1.40 points of this column.

## 5. Limits

- **The split is an accounting split with the model. It is not a cause.** The files do not say
  why the price went down. This document does not add news or causes.
- **The value is the value at the means of one path.** It is NOT the expected value of the Monte
  Carlo engines.
- **A market-implied value is a measurement, not an input.** The implied premium and the implied
  growth come from the price. No input of the model comes from the price.
- **A low implied premium gives a steep reading.** Below an implied premium of 1% at D1, a small
  change of the premium moves the value much. The txt file flags five S&P 10 names in the premium
  view: NVDA (-2.20% at D1), META (-0.04%) and WMT (-2.12%) are below 1%; MSFT (premium part -15.9%
  of the price) and GOOG (-19.5%) have a part above 15% of the price. In the growth view it flags
  NVDA, AVGO and META (a part above 15%).
- **The order changes the split.** Over the two readings and the two orders, the rates part of the
  S&P 100 is +3.38 to +4.93 points, and the plug (premium + growth) is -12.38 to -10.87 points.
  For the S&P 10: rates +3.12 to +5.57 points, plug -14.12 to -11.73 points. The facts, the time
  and the no-model parts change by 0.07 points or less.
- **Coverage.** The premium view covers 69.2% of the S&P 100 weight. The growth view covers 11.0%
  more. 19.8% has no model value (section 4).
- **Automatic model.** It is a screen, not a reviewed valuation. No step of the four-step review
  is done.

## 6. Data and commands

- Data store: commit `f634633` of `louellet-git/fa-data`. Curve source: `--curve par`.
- Branch: `claude/sp100-week-attribution`. Run date: 2026-11-21.
- Dates: close of 2026-11-13 (D1) to close of 2026-11-20 (D2). The price is the last close on or
  before each date.
- The list `sp10_2026-11-13.txt` has the ranking of the market value at 2026-11-13. Its header
  names store `58e54a2`, not `f634633`.

```bash
rust/target/release/fa dcf attribute --tickers-file rust/mapcheck/universe/sp100.txt --from 2026-11-13 --to 2026-11-20 --cache <store f634633> --curve par --out analysis/priors/data/sp100_week_2026-11-20.csv
rust/target/release/fa dcf attribute --tickers-file analysis/priors/data/sp10_2026-11-13.txt --from 2026-11-13 --to 2026-11-20 --cache <store f634633> --curve par --out analysis/priors/data/sp10_week_2026-11-20.csv
```

Outputs: `analysis/priors/data/sp100_week_2026-11-20.txt` and `.csv`;
`analysis/priors/data/sp10_week_2026-11-20.txt` and `.csv`.
