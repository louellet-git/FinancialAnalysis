# JNJ first pass: valuation and pipeline findings (2026-10-16)

**First pass, unreviewed** (IBM had 19 + 4 driver reviews; this has none). Purpose, per
Luc's review on PR #16: run a stock we had not covered and see where the pipeline and
the IBM template break. Not investment advice.

Reproduce: `python -m scripts.analyze JNJ`, then `python -m analysis.JNJ.first_pass_run`.
Register with inline rationales: `analysis/JNJ/driver_model.py`.

Why JNJ: a mature non-tech large cap that tests what IBM could not. It has a spin-off
recast (Kenvue, 2023), heavy pipeline M&A (11.6% of revenue vs IBM's 7.1%), a litigation
liability (talc), a planned Orthopaedics separation, and a low beta at a 5.1% Treasury.

## Verdict: overpriced on our conventions (low-to-moderate confidence)

| P10 | P25 | P50 | P75 | P90 | Price (Oct 16 close) | Chance too high |
|---|---|---|---|---|---|---|
| $69.99 | $88.63 | $112.64 | $141.82 | $175.38 | $255.56 | 99.3% |

The direction is robust: no single assumption closes the gap within its bounds, and the
most generous combination tested gives $210.83 with 72.1% of values still below the price.
The size is not robust (first pass; f alone spans $63.70 to $154.95). The analyst median
target is $280.50.

Inputs: WACC 7.59% at means (rf 5.10%, beta 0.65, ERP 4.20%, E/V 0.93); TV/EV 63.3%.
Net debt $41.05B = debt 49.04 + after-tax pension 5.50 + after-tax talc 7.27 (reserve
$3.7B + $5.52B Q3 charge) - cash and securities 20.76. Shares 2,440.1M. Year 1 is pinned
to the $101.2B 2026 guidance.

**Engine check:** at driver means, the engine and an independent hand-built DCF both give
$112.0194. The arithmetic is sound; the open questions are inputs and template.

| One change, rest at base | P50 |
|---|---|
| M&A recurrence f = 0 / 0.5 / 1 (growth credit moves with f) | $154.95 / $112.64 / $63.70 |
| Beta 0.45 / 0.85 | $136.02 / $95.28 |
| rf 4.6% (the July 2026 level) | $126.37 |
| Stacked: f = 0, beta 0.45, rf 4.6% | $210.83 |
| Classic engine, generic presets, no M&A or SBC charge: base / bull | $142.55 / $200.59 |

**What $255.56 requires** (solved with the parked `price_implied_mean` from `076a953`).
At base, only rf at 2.26% reaches it. With f = 0, it needs core margin 32.4% (vs 20.8%),
terminal growth 4.78% forever, or rf 3.13%. Free cash flow yields 3.0% on enterprise
value, which implies about 4.5% growth forever at a 7.59% WACC.

## Pipeline defects found

| # | Defect | Evidence | Effect |
|---|---|---|---|
| 1 | Quarterly facts inside 10-Ks become "annual" rows (no duration check) | Rows 2022-10-02, 2023-04-02, 2023-07-02, 2023-10-01 hold ~$20-21B of quarterly revenue | Any code reading 10-K rows sees fake years |
| 2 | Those rows use up `n_periods=8`, crowding out real history | Annual revenue only FY2022-25; companyfacts has FY2016+ | Too little history |
| 3 | Backtest cannot run | 0 realized vintages (needs 3 annual periods) | No calibration for JNJ |
| 4 | Recast mismatch: revenue restated ex-Kenvue, cash flow not | FY2022 revenue 79.99 vs 94.94 originally; OCF unrestated | FY2022-23 margins wrong; same class as IBM FY2019-20 (T-IMP-1) |
| 5 | `run_dcf` prints enterprise value per share as "P50 per share" | Base preset $160.07 printed vs $142.55 equity value | Overstates by net debt per share |
| 6 | Default net debt omits talc and pension | Derived $28.2B vs $41.1B bridge | Confirms HIDDEN-ASSUMPTIONS #25 on a second ticker |

## Template gaps

- **Pharma M&A:** acquired pipeline revenue turns "organic" after a year, so the IBM
  pairing likely undercounts what M&A buys. Needs a sector rule.
- **Beta:** regression betas are 0.22 (5-year monthly) and ~0 (2-year weekly); 0.65 is a
  judgment. Needs a method rule.
- **Rates:** rf rose from 4.6% to 5.10% since July, worth ~$14/share here. Refresh rf on
  every run.
- **Not modeled:** the Orthopaedics separation; whether past talc payments sit in the
  historical cash flow (stock-vs-flow, F15).
- **Narrow tails** (T-DCF-1): P90 is below the price, so 99.3% overstates certainty.

## Watch list (settles at Q4-2026 results, late January 2027, unless noted)

- FY2026 sales vs the $101.2B guide; Innovative Medicine +8.6% for 2026.
- 2027 guidance: Innovative Medicine growth vs our 5.0% decade average.
- FY2026 free cash flow (Q3 YTD ~$15.1B) vs our 20.8% core margin.
- Ovarian talc settlement reaching 95% participation (first payment 2027).

## Sources

- JNJ Q3-2026 8-K, Exhibits 99.1/99.2: https://www.sec.gov/Archives/edgar/data/200406/000020040626000189/
- JNJ Q4-2025 8-K: https://www.sec.gov/Archives/edgar/data/200406/000020040626000002/
- FY2025 10-K: https://www.sec.gov/Archives/edgar/data/200406/000020040626000016/jnj-20251228.htm
- FY2023 10-K: https://www.sec.gov/Archives/edgar/data/200406/000020040624000013/jnj-20231231.htm
- Q2-2026 10-Q (talc reserve): https://www.sec.gov/Archives/edgar/data/200406/000020040626000153/jnj-20260628.htm
- Talc settlement 8-K: https://www.sec.gov/Archives/edgar/data/0000200406/000020040626000155/exhibit991.htm
- Damodaran ERP 4.20% (Oct 1, 2026): https://pages.stern.nyu.edu/~adamodar/New_Home_Page/home.htm
- 10-year Treasury 5.10% and JNJ price: yfinance (^TNX, JNJ), 2026-10-16 close; SEC companyfacts, CIK 200406.
