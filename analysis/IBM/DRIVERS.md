# IBM driver-based model — assumption register (POST-REVIEW r1, consolidated)

Companion to `driver_model.py` (the executable, NORMATIVE definition — its rationale
strings govern) and `financial_analysis/driver_dcf.py` (engine). Review record:
DRIVER-REVIEW.md (19 round-1 reviews → dispositions → 4 consolidation reviews; the
reopened items are implemented here). **Status: CONSOLIDATED** — headline is
conditional on f = 0.50 (see ma_recurrence).

## Items (deterministic formulas)

| Item | Formula (words) | Notes |
|---|---|---|
| Revenue | Σ segments: base × (1+g_seg)^t; **year 1 pinned** to Q2-warning-implied levels, draws from t=2 | growth constant per path (T-DCF-1 ticket); **FX: growth is nominal all-in, FX unmodeled** (register row per revenue-formula review) |
| FCFF margin | core + mix_drift(t) − sbc(t) − f × ma | mix_drift and sbc are deterministic per-year schedules |
| WACC | E/V × (rf + β×ERP) + (1−E/V) × (rf + spread) × (1−τ) | kd derived per path from rf (no negative-spread paths); **flat term structure disclosed** — one WACC for years 1-10 + terminal |
| Terminal value | Gordon at min(tg, WACC−2%), discounted 10y | pinned fraction reported: 0.0009 at seed 7 |

Equity/share = (PV FCFF + PV TV − net debt $54.0B) / 948.7M diluted (v3 bridge).

## Drivers (post-review values; rationales live in driver_model.py)

| Driver | S/D | Dist | Mean | Std | Bounds/Schedule | Review outcome |
|---|---|---|---|---|---|---|
| g_software | S | truncnorm | **5.0% = 4.0% organic + f×2.0% acquired (derived from MA_RECURRENCE)** | 2.5% | hi 11% | double-credit fixed; pairing single-sourced |
| g_consulting | S | normal | 1.75% | 2.5% | — | skew proxy; mixture ticketed |
| g_infrastructure | S | truncnorm | −1.0% | 2.5% | hi +3% | decade-CAGR dispersion; hi at 1.6σ — bound mass disclosed |
| g_financing | D | — | 0.0% | — | — | SOUND (only one); disclosures added |
| core_margin | S | normal | 19.4% (FY2025 endpoint, F19) | 2.0% | — | endpoint basis + mix drift together |
| margin_mix_drift | D | schedule | 0 → +1.2pp by y10 | — | linear | mechanical software-mix consequence |
| sbc_ratio | D | schedule | 2.54% → 3.5% by y10 | — | linear | trend break removed; drift locus published |
| ma_ratio | S | truncnorm | 7.1% | **2.4%** | lo 0 (3.0σ) | consolidation reopen: AR(1) ρ≈0.8 decade-average scale |
| ma_recurrence | D | — | 0.50 (`MA_RECURRENCE`, single source + assert) | — | — | pairing-coherence rationale; headline conditional on f |
| rf | S | truncnorm | 4.6% (spot 10y, Jul-2026) | 0.5% | lo 1% (formal guard) | permanent-component framing |
| beta | D | — | 0.85 | — | locus 0.68–1.00 published | epistemic grounds (corrected rationale) |
| erp | S | truncnorm | 4.5% (Damodaran Jul-2026) | 0.75% | lo 2% | sourced; state-variable rationale |
| kd_spread | D | — | −0.10% over rf | — | — | fair-value calibrated (conflict #2 upheld) |
| tax_rate | D | — | 21% | — | — | τ=0 scenario published |
| equity_weight | D | — | 0.72 | — | — | price-value fixed point (converges, verified) |
| terminal_growth | S | normal | 2.25% | 0.75% | joint guard | std widened; T-DCF-3 (derived tg) ticketed |

Segments: bases = FY2025 8-K actuals, EXCEPT Infrastructure = FY24/25 average $14,869M
(cycle-peak haircut). Year-1 pins: Software +5%, Consulting 0%, Financing 0% on FY2025
bases; Infrastructure **−1.69% on the haircut base** — level-targeted to the
warning-implied $14,618M so the cycle correction is not double-counted (consolidation
reopen, fixed).

## Correlation matrix (post-review)

As pre-review (see git history for the full rationale table) PLUS two review-driven
entries: **ma_ratio·rf = −0.15** (debt-financed M&A slows when rates spike — the least
defensible zero, fixed) and **terminal_growth·ma_ratio = +0.15** (bought growth priced
in the terminal — F16 in perpetuity). PSD verified: min eigenvalue 0.419. Regime
disclosure: rf·g_* > 0 assumes demand-driven rates; the supply-shock sensitivity
(rf·g_* = −0.15) is published by `dcf_drivers_run.py` — P50 nearly unchanged, tails widen.

## WACC level — honest attribution (consolidation reopen, closed)

At means: 0.72 × (4.6% + 0.85 × 4.5%) + 0.28 × (4.6% − 0.10%) × 0.79 = **7.06%**
(Monte-Carlo std 0.56%). The v3 "7.5% top-of-band" position is **retired**, not
back-solved: every input is now externally sourced. Gap attribution 7.50% → 7.06%
(≈ −44bp net): rf 4.3→4.6 (+17bp on WACC), ERP 4.75→4.5 (−15bp), E/V 0.765→0.72
iterated (−14bp), kd 4.1%→3.56% derived (−17bp), retirement of the old build's
top-of-band rounding (−15bp, the former 7.34→7.50 step). WACC level alone is worth
≈ +$11.5/sh vs 7.34% and +$17.6/sh vs 7.50%. Residual
conservatism lives transparently in β = 0.85 (vs trailing ~0.68) with the published locus.

## Output (v4 final, seed 7, 10k sims, 10y, conditional on f=0.50)

**P10 $83.84 / P50 $151.84 / P90 $258.03** · TV/EV at means 64.1% · tg guard pinned 0.09%.

Attribution leaders (±1σ, correlations intact, post-consolidation run): core_margin ∓31 ·
terminal_growth −19/+26 · rf +23/−19 · erp +22/−18 · g_software −18/+21 · ma_ratio ±18.7
(rose with the consolidation-mandated std 2.4%).

Published loci (`python -m analysis.IBM.dcf_drivers_run`, engine governs — these are the
current run's values): ma_recurrence f=0→1 (paired, derived from
`SOFTWARE_ORGANIC_G + f × SOFTWARE_ACQUIRED_G`): P50 $197.79→$101 ·
SBC drift flat/3.5%/4.5%: $164.11/$151.84/$139.20 · β 0.68/0.85/1.00:
$179.42/$151.84/$132.34 · supply-shock correlations and τ=0 disclosure runs ·
implied terminal reinvestment: RR 20.8%, ROIC 10.8% > WACC (no free growth at means).

## Deferred tickets

T-DCF-1 AR(1) growth (largest known distortion) · T-DCF-3 derived per-path tg ·
mixture/lognormal marginals · Harris-Pringle re-levered β / target weights ·
importer segment capture (segment history is hand-entered from 8-Ks, sourced).

## Source ledger

Segment data: Q4 8-K Ex. 99.1 filings 2022-2026 + Jul 14 2026 warning 8-K (URLs in the
research record and README). Market params: 10y Treasury spot, Damodaran ERP (Jul 2026).
Bridge/margin basis: analysis/IBM/README.md (v3) + REVIEW.md. Review record: DRIVER-REVIEW.md.
