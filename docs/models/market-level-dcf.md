# Market-level dividend DCF (the premium that the S&P composite price implies)

Ticket: `T-20261121-dcf-engine-backtest`, plan item 3. Code: `rust/src/dcf/market.rs`, `market_score.rs`.
## Purpose
For each month t, find the equity risk premium (erp) for which a dividend DCF with the fade of the
automatic model gives the S&P composite price of month t. This is a measurement: the price is the
input of the search, and no model input comes from the price. With `--score` it also scores the premium against the later real excess return by the frozen design
(`analysis/macro/2026-11-21-long-history-design.md`); the run of record is 3f5a6e3.

## Data
`macro/shiller/shiller_monthly.csv` of the data store (Shiller, `ie_data.xls`). Columns used:
price, dividend (annual rate), cpi, gs10 (percent), estimate. Also `macro/fred/CPIAUCNS.csv` (from 1913-01).

## Design (fixed before the first run)
All values are real. The money unit is the money of month t (the CPI known at t, CPIK(t)).
1. Known dividend. Month t of year Y, from 1936: the dividend of December of year Y-1 if the month
   is April or later, else of December of year Y-2. Reason: the annual figure is complete only
   about 3 months after the year ends. Before 1936: always December of year Y-2 (a lag of 12 months
   or more), because the annual figures were published late. The first row is 1883-01.
2. Known CPI, CPIK(t). Before 1913-01: the Shiller CPI of December of the last ended year (a step
   series). At 1913-01: the Shiller December 1912. From 1913-02: CPIAUCNS of month t-1 (FRED,
   option `--cpi`; the command fails without it if a row needs it). A month without a CPIAUCNS
   value gives status `no_cpi` (October 2025 has no value). The gs10 and the price are those of
   month t.
3. D_real(t) = D_known x CPIK(t) / CPI(own December of D_known). That December CPI is the Shiller
   value before 1913 and CPIAUCNS from 1913. The real price is the price (money of t).
4. Real rate: rr(t) = gs10(t) / 100 - ((CPIK(t) / CPIK(t-120))^(1/10) - 1).
5. Start growth g0: the annualized real growth of the known annual dividend over the last 10 known
   years. Clamp to -10% and +20%.
6. Fade: the real growth goes linearly from g0 in year 1 to L in year N, then stays L.
   Defaults: L = 2.0% (`--long-real`, record A-001), N = 10 (`--fade-years`, at least 2).
7. Value: V = sum of D_k / (1 + r)^k for k = 1 to N, plus D_N (1 + L) / (r - L) / (1 + r)^N, with
   r = rr + erp.
8. Implied erp: the root of V = price by bisection on -10% to +30%. The lower bound is raised to
   L - rr + 1e-9, so that r > L. Without a root the row has erp_implied = "none" and a status
   code: `no_root_low` (the price is above the value at the lower bound), `no_root_high` (below
   the value at 30%), `no_root_rate` (the raised bound is 30% or more), `no_dividend` (the
   dividend is not positive).
9. Gordon baseline: erp_gordon = D_real (1 + L) / price + L - rr. With g0 = L the DCF value equals
   the Gordon value, so the two premiums are equal.

## Difference from A-001
A-001 caps L at the 10-year yield. This model does not: real rates were negative in many years.

## Output
CSV, one row a month from the first month with 10 known years of dividend and 120 months of CPIK:
month, price_real, d_known_real, g0, rr, erp_implied, erp_gordon, estimate, status. The stdout
summary gives, by decade, the months, the no-root months, and the median, P10 and P90 of erp_implied.

## Limits
- Dividends only. Buybacks are not in the cash flow. They were large after 1982, so the premium of
  later decades is higher than a total-payout premium.
- The index is the S&P composite as it is now. It has survivorship.
- Before 1926 the series is the Cowles reconstruction.
- The annual dividends are interpolated to months in the source. The model reads only December.
- The last months of the file are estimates (the `estimate` column).
- The variants L = 1.0% and 3.0%, and N = 5 and 20, run once each after the base. No choice among them.

## Command
`fa dcf market --shiller FILE --cpi FILE --out FILE [--long-real G] [--fade-years N]`; to score: add `--tb3ms FILE --nber FILE --gs10 FILE --months-out FILE --score FILE` (the command of record: `analysis/priors/market_score_complete_2026-11-21.command.txt`).
