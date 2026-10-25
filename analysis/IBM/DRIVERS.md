---
id: DOC-IBM-DRIVERS
type: description
item: 10.15 Design description
status: current
date: 2026-10-25
scope: Reviewer-facing assumption register of the IBM v4 driver model; driver_model.py governs on disagreement.
---
# IBM driver model — assumption register

Companion to `driver_model.py` and `financial_analysis/driver_dcf.py` (engine). Review
record: analysis/IBM/DRIVER-REVIEW.md.

Notation: S/D = stochastic/deterministic driver; f = `ma_recurrence`; ma = `ma_ratio`;
truncnorm = truncated normal (bounds in the Bounds column).

## Items (deterministic formulas)

| Item | Formula (words) | Notes |
|---|---|---|
| Revenue | Σ segments: base × (1+g_seg)^t; **year 1 pinned** to Q2-warning-implied levels, draws from t=2 | growth constant per path (T-DCF-1 ticket); **FX: growth is nominal all-in, FX unmodeled** |
| FCFF margin | core + mix_drift(t) − sbc(t) − f × ma | mix_drift and sbc are deterministic per-year schedules |
| WACC | E/V × (rf + β×ERP) + (1−E/V) × (rf + spread) × (1−τ) | kd derived per path from rf (no negative-spread paths); **flat term structure disclosed** — one WACC for years 1-10 + terminal |
| Terminal value | Gordon at min(tg, WACC−2%), discounted 10y | — |

Equity/share = (PV FCFF + PV TV − net debt $54.0B) / 948.7M diluted (bridge: analysis/IBM/README.md).

## Drivers (post-review values; rationales live in driver_model.py)

| Driver | S/D | Dist | Mean | Std | Bounds/Schedule | Review |
|---|---|---|---|---|---|---|
| g_software | S | truncnorm | **5.0% = 4.0% organic + f×2.0% acquired (derived from MA_RECURRENCE)** | 2.5% | hi 11% | F16 organic/acquired split; credit derived from f |
| g_consulting | S | normal | 1.75% | 2.5% | — | skew proxy; mixture ticketed |
| g_infrastructure | S | truncnorm | −1.0% | 2.5% | hi +3% | decade-CAGR dispersion; hi at 1.6σ, bound mass disclosed |
| g_financing | D | — | 0.0% | — | — | disclosures in driver_model.py rationale |
| core_margin | S | normal | 19.4% (FY2025 endpoint, F19) | 2.0% | — | F19; endpoint basis and mix drift together |
| margin_mix_drift | D | schedule | 0 → +1.2pp by y10 | — | linear | mechanical software-mix consequence |
| sbc_ratio | D | schedule | 2.54% → 3.5% by y10 | — | linear | drift locus published (flat/3.5%/4.5%) |
| ma_ratio | S | truncnorm | 7.1% | **2.4%** | lo 0 (3.0σ) | std 2.4% = decade-average scale at AR(1) ρ≈0.8 |
| ma_recurrence | D | — | 0.50 (`MA_RECURRENCE`, single source + assert) | — | — | paired with g_software; headline conditional on f |
| rf | S | truncnorm | 4.6% (spot 10y, Jul-2026) | 0.5% | lo 1% (formal guard) | permanent-component std |
| beta | D | — | 0.85 | — | locus 0.68–1.00 published | epistemic, not sampled |
| erp | S | truncnorm | 4.5% (Damodaran Jul-2026) | 0.75% | lo 2% | state variable; comoves with demand |
| kd_spread | D | — | −0.10% over rf | — | — | fair-value calibrated |
| tax_rate | D | — | 21% | — | — | τ=0 run published |
| equity_weight | D | — | 0.72 | — | — | price-value fixed point |
| terminal_growth | S | normal | 2.25% | 0.75% | joint guard | std 0.75%; T-DCF-3 (derived tg) ticketed |

Segments: bases = FY2025 8-K actuals, EXCEPT Infrastructure = FY24/25 average $14,869M
(cycle-peak haircut). Year-1 pins: Software +5%, Consulting 0%, Financing 0% on FY2025
bases; Infrastructure **−1.69% on the haircut base** — level-targeted to the
warning-implied $14,618M so the cycle correction is not double-counted.

## Correlation matrix (post-review)

Full matrix: `CORR` in analysis/IBM/driver_model.py (PSD min eigenvalue 0.419).
Entries: ma_ratio·rf = −0.15; terminal_growth·ma_ratio = +0.15 (bought growth
priced into terminal). Regime disclosure: rf·g_* > 0 assumes demand-driven rates; the
supply-shock sensitivity (rf·g_* = −0.15) is published by `dcf_drivers_run.py` — P50
nearly unchanged, tails widen.

## WACC

At means: 0.72 × (4.6% + 0.85 × 4.5%) + 0.28 × (4.6% − 0.10%) × (1 − 21%) = **7.06%**
(MC std 0.56%). Gap attribution 7.50% → 7.06% and the WACC-level value per share (vs
7.34% and vs 7.50%): report §3.3. Residual conservatism sits in β = 0.85
(trailing ~0.68; locus 0.68–1.00, Output below).

## Output (v4 final, seed 7, 10k sims, 10y, conditional on f=0.50)

**P10 $83.84 / P50 $151.84 / P90 $258.03** · TV/EV at means 64.1% · tg guard pinned 0.09%.

Attribution leaders (±1σ, correlations intact): core_margin ∓31 ·
terminal_growth −19/+26 · rf +23/−19 · erp +23/−19 · g_software −18/+21 · ma_ratio ±18.7.

Published loci (`python -m analysis.IBM.dcf_drivers_run`, engine governs — these are the
current run's values): ma_recurrence f=0→1 (paired, derived from
`SOFTWARE_ORGANIC_G + f × SOFTWARE_ACQUIRED_G`): P50 $197.79→$101.44 ·
SBC drift flat/3.5%/4.5%: $164.11/$151.84/$139.20 · β 0.68/0.85/1.00:
$179.42/$151.84/$132.34 · supply-shock correlations and τ=0 disclosure runs ·
implied terminal reinvestment rate 20.8%, ROIC 10.8% > WACC (no free growth at means).

## Deferred tickets

T-DCF-1 AR(1) growth (largest known distortion) · T-DCF-3 derived per-path tg ·
mixture/lognormal marginals · Harris-Pringle re-levered β / target weights ·
importer segment capture (segment history is hand-entered from 8-Ks, sourced).

## Source ledger

Segment data: Q4 8-K Ex. 99.1 filings 2022-2026 + Jul 14 2026 warning 8-K (links: report
Appendix B items 1–2, 8). Market params: 10y Treasury spot, Damodaran ERP (Jul 2026).
Bridge/margin basis: analysis/IBM/README.md (v3) + REVIEW.md.
