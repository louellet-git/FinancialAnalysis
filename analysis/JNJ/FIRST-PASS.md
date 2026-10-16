# JNJ — first-pass valuation and pipeline findings (2026-10-16)

**Status: first pass, unreviewed.** One session, no driver/formula/correlation review
fan-out (IBM had 19 + 4). Purpose, per Luc's review on PR #16: run a stock we had not
covered, to see where the pipeline and the IBM template break. Not investment advice.

Reproduce: `python -m scripts.analyze JNJ` (data), `python -m analysis.JNJ.first_pass_run`
(numbers below). Register: `analysis/JNJ/driver_model.py` (rationales inline).

## Why JNJ

US-GAAP, mature large cap, outside the IBM/ORCL tech basket, and it stresses the template
in new ways: a spin-off recast (Kenvue, 2023, the same class as IBM's Kyndryl landmine),
heavy pipeline M&A (11.6% of revenue FY21-25 vs IBM's 7.1%), a litigation liability
(talc) for the net-debt bridge, a planned Orthopaedics separation, and a defensive
low-beta stock in a 5.1% rate environment.

## Potential verdict

**Overpriced on our conventions, low-to-moderate confidence.** Price $255.56
(2026-10-16 close) vs P50 $112.64; 99.3% of simulated values are below the price. The
gap is too large for any single assumption to close: even the most generous combination
tested (acquisitions treated as free, beta 0.45, July's 4.6% Treasury) gives P50
$210.83, with 72.1% of values still below the price. The *direction* is therefore more
robust than the *size*, which inherits every first-pass caveat below. Analysts' median
target is $280.50 (range $190-$320); reaching it requires a discount rate well below a
bottom-up build at today's 5.1% Treasury, or growth and M&A returns well above JNJ's
recent record.

## Headline (seed 7, 10,000 paths)

| P10 | P25 | P50 | P75 | P90 | Price | Chance too high | TV/EV at means |
|---|---|---|---|---|---|---|---|
| $69.99 | $88.63 | $112.64 | $141.82 | $175.38 | $255.56 | 99.3% | 63.3% |

WACC at means 7.59% (rf 5.10%, beta 0.65, ERP 4.20%, E/V 0.93). Net debt $41.05B
(debt 49.04 + after-tax pension 5.50 + after-tax talc 7.27 - cash and securities
20.76; talc = $3.7B reserve per the Q2-2026 10-Q + the $5.52B Q3 litigation charge). Shares 2,440.1M diluted. Year-1 pinned to the October 2026 guidance
($101.2B reported sales).

**Engine cross-check:** with every driver at its mean, the engine and an independent
hand-built DCF both give $112.0194 per share. The arithmetic is sound; the open
questions are inputs and template.

| Locus (one change, rest at base) | P50 |
|---|---|
| M&A recurrence f = 0 / 0.25 / 0.5 / 0.75 / 1 (growth credit moves with f) | $154.95 / $134.58 / $112.64 / $89.15 / $63.70 |
| Acquired-growth credit 2% / 4% / 6% at f = 0.5 | $112.64 / $121.90 / $131.60 |
| Beta 0.45 / 0.65 / 0.85 | $136.02 / $112.64 / $95.28 |
| rf 4.6% (July 2026 level) | $126.37 |
| Stacked: f = 0, beta 0.45, rf 4.6% | $210.83 (72.1% below price) |
| Classic engine, generic presets, no M&A or SBC charge: bear / base / bull | $84.84 / $142.55 / $200.59 (bridge net debt) |

**What the price requires** (bisection with the parked `price_implied_mean` from
`076a953`):

| Setting | Value the price needs |
|---|---|
| Base (f = 0.5), one driver at a time | only rf at 2.26%; growth, margin, M&A, ERP and terminal growth cannot reach it within their bounds |
| f = 0 | core margin 32.4% (vs 20.8%), or terminal growth 4.78% forever, or rf 3.13% |
| f = 0 and beta 0.45 | core margin 27.7%, or Innovative Medicine growth 10.3% a year for a decade, or terminal growth 3.76%, or rf 3.93% |

Market cross-check: free cash flow $19.7B (FY2025) is a 3.0% yield on enterprise
value; at a 7.59% WACC that implies about 4.5% perpetual growth with no charge for M&A
or stock pay.

## Pipeline problems found (for the importer / validators backlog)

1. **Quarterly figures stored as annual rows.** The importer keys 10-K facts by end date
   without checking duration, so quarterly values inside JNJ's 10-Ks become "10-K" rows
   (2022-10-02, 2023-04-02, 2023-07-02, 2023-10-01, each with about $20-21B of
   quarterly revenue). Evidence: `importer.py` period keying, `xbrl_api.extract_series`.
2. **Those rows crowd out real history.** `n_periods=8` per concept is consumed by the
   quarterly rows, so annual revenue survives only for FY2022-FY2025, although
   companyfacts holds FY2016+. Generalizes HIDDEN-ASSUMPTIONS #2-#3.
3. **Backtest unusable.** Zero realized vintages (each needs 3 point-in-time annual
   periods); JNJ cannot be calibrated until 1-2 are fixed.
4. **Recast mismatch again.** Revenue is recast ex-Kenvue (FY2022 79.99 vs 94.94
   originally) but operating cash flow is not, so FY2022-23 margins are contaminated.
   The classic engine's derived margin (21.6%) uses them. IBM's FY2019-20 landmine is
   general, not IBM-specific (T-IMP-1).
5. **`scripts/run_dcf.py` prints enterprise value per share as "P50 per share".** No net
   debt is subtracted (JNJ base preset: $160.07 printed vs $142.55 equity value with
   the bridge). The visualizer subtracts it correctly.
6. **Default net debt differs from the reviewed bridge.** It derives to $28.2B (no talc,
   no pension) vs the $41.1B bridge, confirming HIDDEN-ASSUMPTIONS #25 on a second
   ticker; generic presets also apply silently (#20).

## Template problems (model layer)

- **M&A pairing transfers poorly to pharma.** Acquired pipeline revenue counts as
  "organic" one year after closing, so a 2pp acquired-growth credit likely understates
  what M&A buys. f alone moves the value from $63.70 to $154.95. Needs a pharma-specific
  rule before any JNJ number is decision-grade.
- **Beta has no method rule.** Regression betas are 0.22 (5-year monthly) and about 0
  (2-year weekly). The 0.65 used here is a judgment; a bottom-up or industry-beta rule is
  needed for every ticker.
- **Rates move the answer.** The Treasury rose from 4.6% (IBM report, July) to 5.10%;
  that alone is about $14 per share here. Every run needs a dated rf refresh.
- **Not modeled:** the Orthopaedics separation (segment kept whole); talc as a
  stock-only bridge item (past payments in historical cash flow are not separated: a
  stock-vs-flow risk, IBM F15); acquired in-process R&D timing.
- **Overconfident tails.** One growth draw per path (T-DCF-1) makes the distribution
  narrow: P90 ($175.38) sits below the price, so the 99% figure overstates certainty.

## Watch list (testable bets)

| Bet | Our base | Settles |
|---|---|---|
| FY2026 sales vs the $101.2B guide; Innovative Medicine +8.6% for 2026 | year-1 pins | Q4-2026 results (late January 2027) |
| Innovative Medicine growth after 2026 | 5.0% a year average | FY2027 guidance (January 2027), then each year |
| Free cash flow margin | 20.8% before stock pay and M&A | FY2026 cash flow (Q3 YTD ~$15.1B) |
| M&A spend and what it buys | 11.6% of revenue, half charged | each deal; acquired-growth disclosure |
| Ovarian talc settlement reaches 95% participation | $5.5B in the bridge | settlement process (first payment 2027) |

## Sources

- SEC companyfacts API (CIK 200406), cached 2026-10-16.
- JNJ Q3-2026 8-K, Exhibits 99.1 and 99.2 (2026-10-13): sales, guidance, special items: https://www.sec.gov/Archives/edgar/data/200406/000020040626000189/
- JNJ Q4-2025 8-K, Exhibit 99.1 (2026-01-21): FY2025 operational vs adjusted operational growth: https://www.sec.gov/Archives/edgar/data/200406/000020040626000002/
- JNJ FY2025 and FY2023 Form 10-K: segment sales, capex: https://www.sec.gov/Archives/edgar/data/200406/000020040626000016/jnj-20251228.htm and https://www.sec.gov/Archives/edgar/data/200406/000020040624000013/jnj-20231231.htm
- JNJ Q2-2026 Form 10-Q (talc reserve ~$3.7B present value): https://www.sec.gov/Archives/edgar/data/200406/000020040626000153/jnj-20260628.htm
- JNJ 8-K (2026-07-27), proposed ovarian talc resolution, $5.5B: https://www.sec.gov/Archives/edgar/data/0000200406/000020040626000155/exhibit991.htm
- Damodaran implied ERP 4.20% (October 1, 2026): https://pages.stern.nyu.edu/~adamodar/New_Home_Page/home.htm
- 10-year Treasury 5.10% and price $255.56: yfinance (^TNX, JNJ), 2026-10-16 close.
