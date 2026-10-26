---
id: DOC-IBM-DRIVERS
type: description
item: 10.15 Design description
status: current
date: 2026-10-25
scope: Assumption register of the IBM v4 driver model; driver_model.py governs on disagreement.
---
# IBM driver model — assumption register

Companion to `driver_model.py` and `financial_analysis/driver_dcf.py` (engine). Review
records: analysis/IBM/REVIEW.md (F-IDs), analysis/IBM/DRIVER-REVIEW.md. Tickets:
PLANNING/tickets.md §Engine tickets; segment bases: docs/HIDDEN-ASSUMPTIONS.md #5.

Notation:

| Symbol | Meaning |
|---|---|
| S/D | stochastic/deterministic driver |
| β | beta |
| kd | after-tax cost of debt (kd_spread row) |
| f | `ma_recurrence` |
| ma | `ma_ratio` |
| truncnorm | normal draw clipped to [lo, hi] after correlation (driver_dcf.py docstring) |
| tg | terminal_growth |
| τ | tax_rate |
| E/V | equity_weight |
| ERP | erp |
| t | projection year 1-10 |
| locus | published sweep of one judgment (the f-locus moves g_software's mean with f; `analysis/IBM/dcf_drivers_run.py` docstring, item 3) |

## Items (deterministic formulas)

| Item | Formula (words) | Notes |
|---|---|---|
| Revenue | Σ segments: t=1 base × (1+pin_seg); t≥2 t=1 level × (1+g_seg)^(t−1); pins (warning-implied year-1 levels; see Source ledger) | growth constant per path (T-DCF-1 ticket); **FX unmodeled** (report §3.2) |
| FCFF margin | core + mix_drift(t) − sbc(t) − f × ma | mix_drift and sbc are deterministic per-year schedules |
| WACC | E/V × (rf + β×ERP) + (1−E/V) × (rf + spread) × (1−τ) | kd = (rf + spread) × (1−τ) per path, so kd moves with rf on every path (spread fixed, kd_spread row); **flat term structure disclosed** — one WACC for years 1-10 + terminal |
| Terminal value | Gordon at the joint guard (docs/ARCHITECTURE.md, Driver-based engine), discounted 10y | — |

Equity/share: bridge in analysis/IBM/README.md.

## Driver values

| Driver | S/D | Dist | Mean | Std | Bounds/Schedule | Review |
|---|---|---|---|---|---|---|
| g_software | S | truncnorm | **5.0% = 4.0% organic + f×2.0% acquired (`SOFTWARE_ORGANIC_G + f × SOFTWARE_ACQUIRED_G`)** | 2.5% | hi 11% | F16 organic/acquired split; credit derived from f |
| g_consulting | S | normal | 1.75% | 2.5% | — | skew proxy; mixture: Engine tickets, Mixture marginal for g_consulting |
| g_infrastructure | S | truncnorm | −1.0% | 2.5% | hi +3% | decade-CAGR dispersion; hi at 1.6σ: ≈5.5% of draws clip at +3% (analytic, normal marginal: Φ(−1.6)) |
| g_financing | D | — | 0.0% | — | — | disclosures in driver_model.py rationale |
| core_margin | S | normal | 19.4% pre-SBC, pre-M&A (FY2025 endpoint, F19) | 2.0% | — | F19; endpoint basis and mix drift together |
| margin_mix_drift | D | — | 0 | — | 0 → +1.2pp by y10, linear | mechanical software-mix consequence |
| sbc_ratio | D | — | 2.54% | — | 2.54% → 3.5% by y10, linear | drift locus published (flat/3.5%/4.5%) |
| ma_ratio | S | truncnorm | 7.1% | **2.4%** | lo 0 (3.0σ) | decade-average scale; source series AR(1) ρ≈0.8 (data persistence, not engine dynamics) |
| ma_recurrence | D | — | 0.50 (`MA_RECURRENCE`, single source + assert) | — | — | paired with g_software |
| rf | S | truncnorm | 4.6% (spot 10y, Jul-2026) | 0.5% | lo 1% (formal guard) | permanent-component std |
| beta | D | — | 0.85 | — | locus 0.68–1.00 published | epistemic, not sampled |
| erp | S | truncnorm | 4.5% (Damodaran Jul-2026) | 0.75% | lo 2% | state variable; comoves with demand |
| kd_spread | D | — | −0.10% over rf | — | — | fair-value calibrated |
| tax_rate | D | — | 21% | — | — | τ=0 run published |
| equity_weight | D | — | 0.72 | — | — | price-value fixed point |
| terminal_growth | S | normal | 2.25% | 0.75% | joint guard | T-DCF-3 (derived tg) ticketed |

Segments: bases = FY2025 8-K actuals, EXCEPT Infrastructure = FY24/25 average $14,869M
(cycle-peak haircut). Year-1 pins: Software +5%, Consulting 0%, Financing 0% on FY2025
bases; Infrastructure **−1.69% on the haircut base** — level-targeted to the
warning-implied $14,618M so the cycle correction is not double-counted.

## Correlation matrix

Full matrix: `CORR` in analysis/IBM/driver_model.py (PSD min eigenvalue 0.419); every entry,
zeros included, is an assumption. Regime disclosure: rf·g_* > 0 assumes demand-driven rates; the
supply-shock case (rf·g_* = −0.15) is published by `dcf_drivers_run.py`; effect in
docs/HIDDEN-ASSUMPTIONS.md #22.

| Pair | ρ | Rationale |
|---|---|---|
| g_software · g_consulting | +0.40 | shared enterprise IT demand cycle |
| g_software · g_infrastructure | +0.35 | Z-stack coupling; Q2-26 warning: transaction-processing software fell with the hardware shift |
| g_consulting · g_infrastructure | +0.20 | weaker demand link |
| g_software · core_margin | +0.30 | software mix lifts margin |
| g_consulting · core_margin | +0.10 | low-margin segment, weak link |
| g_infrastructure · core_margin | +0.15 | operating leverage on hardware cycle |
| g_software · ma_ratio | +0.30 | M&A concentrated in software; buys its growth |
| g_consulting · ma_ratio | +0.10 | occasional consulting tuck-ins |
| rf · erp | −0.30 | risk-off: rates down, premium up |
| rf · g_software / g_consulting / g_infrastructure | +0.15 / +0.15 / +0.10 | strong nominal economy: rates and demand up together (demand-regime sign) |
| ma_ratio · rf | −0.15 | cheap money fuels deals |
| erp · g_software / g_consulting / g_infrastructure | −0.25 / −0.25 / −0.15 | risk aversion rises in weak-demand worlds |
| erp · core_margin | −0.15 | margin pressure in risk-off worlds |
| erp · ma_ratio | −0.10 | M&A slows when risk premia blow out |
| tg · g_software / g_consulting / g_infrastructure | +0.40 / +0.30 / +0.20 | long-run growth ties to realized growth (F6 tail-fattening link) |
| tg · core_margin | +0.15 | durable franchises keep both |
| tg · rf | +0.25 | shared inflation component |
| tg · erp | −0.20 | risk-off marks down long-run growth |
| tg · ma_ratio | +0.15 | high-tg worlds keep buying growth: bought growth priced into terminal (TV review) |
| ma_ratio · g_infrastructure | 0 | M&A is software-directed |
| ma_ratio · core_margin | 0 | spend timing independent of operating margin |
| core_margin · rf | 0 | no direct mechanism |

## WACC

At means: **7.06%** (MC std 0.56%). v3-to-v4 WACC bridge and per-share effect:
`analysis/IBM/report/IBM-valuation-2026-07.md` §3.3. Residual conservatism sits in β = 0.85
(beta row; trailing ~0.68; beta locus: report §4).

## Output

`python -m analysis.IBM.dcf_drivers_run` (v4 final, seed 7, 10k sims, 10y, conditional on f=0.50);
headline, attribution, loci and implied-reinvestment check in report §4.

## Source ledger

Segment data: Q4 8-K Ex. 99.1 filings 2022-2026 (report Appendix B item 8) + Jul 14 2026
warning (press: report Appendix B items 1-2; driver_model.py cites the Q2-26 8-K). Market params: 10y Treasury spot
(https://fred.stlouisfed.org/series/DGS10; observation date: July 2026, day not recorded), Damodaran ERP
(Jul 2026; report Appendix B item 9).
Bridge/margin basis: analysis/IBM/REVIEW.md.
