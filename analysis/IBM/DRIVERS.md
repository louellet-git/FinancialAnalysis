# IBM driver-based model — assumption register (pre-review)

Companion to `driver_model.py` (the executable definition) and `financial_analysis/driver_dcf.py`
(the engine). **Every row here is a reviewable assumption.** Top-level items are deterministic
vetted formulas; only drivers are sampled, jointly, through the full correlation matrix below.
Status: **PRE-REVIEW** — per-driver, per-formula, and correlation reviews pending; consolidation
reviews per item follow. Their outcomes land in DRIVER-REVIEW.md.

## Items (deterministic formulas — the X, Y, Z)

| Item | Formula (words) | Code |
|---|---|---|
| Revenue | Σ over segments of: base FY2025 segment revenue compounding at that segment's growth driver (constant per path — disclosed limitation, AR(1) is ticketed) | `revenue_paths` |
| FCFF margin | core pre-SBC pre-M&A FCFF margin − SBC ratio − recurrence × M&A ratio | `fcff_margin` |
| WACC | E/V × (rf + β × ERP) + (1 − E/V) × after-tax Kd | `wacc_capm` |
| Terminal value | Gordon on final-year FCFF at min(tg, WACC − 2%) per path, discounted 10y | `terminal_value` |

Equity/share = (PV of FCFF + PV of TV − net debt $54.0B) / 948.7M diluted (v3 bridge, unchanged).

## Segment history (the revenue drivers' data basis)

$M, from Q4 8-K press releases (URLs in the source ledger below). Segments sum to reported
totals; "Other" ($63M FY2025, 0.1%) dropped on materiality.

| Segment | FY2021 | FY2022 | FY2023 | FY2024 | FY2025 | FY2025 mix |
|---|---|---|---|---|---|---|
| Software | 24,141 | 25,037 | 26,308 | 27,085 | 29,962 | 44.4% |
| Consulting | 17,844 | 19,107 | 19,985 | 20,692 | 21,055 | 31.2% |
| Infrastructure | 14,188 | 15,288 | 14,593 | 14,020 | 15,718 | 23.3% |
| Financing | 774 | 645 | 741 | 713 | 737 | 1.1% |

Recast caveats (bind the reviewers): FY2021 is post-Kyndryl continuing-ops; Watson Health
recast moved $715M out of FY2021 Software; Jan-2024 segment changes moved Security Services
Software→Consulting (FY2023 recast: SW 25,011 / Cons 20,884) — growth rates across those
boundaries are not perfectly like-for-like. Q2-2026 warning segment detail: Software +5%
(vs ~10% guide), Consulting flat, Infrastructure −7%.

## Drivers (the A, B, C — stochastic OR deterministic, each reasoned)

See `driver_model.py` for the live values; the rationale strings there are normative. Summary:

| Driver | S/D | Dist | Mean | Std | Bounds | One-line basis |
|---|---|---|---|---|---|---|
| g_software | **S** | normal | 6.0% | 2.5% | — | organic ~5% + half-paid acquired ~1%; guide 10%, warning +5% |
| g_consulting | **S** | normal | 2.0% | 2.0% | — | +7.1/+5/−1/+2 history; GenAI two-sided |
| g_infrastructure | **S** | normal | −1.0% | 3.0% | — | z-cycle sawtooth; guide down low singles |
| g_financing | **D** | — | 0.0% | — | — | 1.1% of revenue: immaterial |
| core_margin | **S** | normal | 19.75% | 2.0% | — | v3 FCFF-after-SBC 17.8% + SBC ratio ~2.0% |
| sbc_ratio | **D** | — | 2.5% | — | — | management-set, slow trend; set at FY2025 endpoint not the mean |
| ma_ratio | **S** | truncnorm | 6.6% | 3.0% | ≥0 | lumpy 3.9–12.3% history |
| ma_recurrence | **D** | — | 0.50 | — | — | a judgment (v3 F2/F16): explored via locus, not sampled |
| rf | **S** | truncnorm | 4.3% | 0.5% | ≥1% | 10y Treasury |
| beta | **D** | — | 0.85 | — | — | estimation constant; sampling it double-counts ERP vol |
| erp | **S** | truncnorm | 4.75% | 0.75% | ≥2% | risk-appetite process |
| kd_after_tax | **D** | — | 4.1% | — | — | contractual, observable |
| equity_weight | **D** | — | 0.765 | — | — | market E/V at $212 (circularity disclosed) |
| terminal_growth | **S** | normal | 2.25% | 0.5% | joint guard | tg < WACC − 2% enforced per path |

## Correlation matrix (8 stochastic drivers — every entry an assumption, zeros included)

Non-zero entries and rationale (matrix is symmetric; PSD verified by the engine):

| Pair | ρ | Rationale |
|---|---|---|
| g_software · g_consulting | +0.40 | shared enterprise IT demand cycle |
| g_software · g_infrastructure | +0.35 | Z-stack coupling — the Q2 warning showed TP software falling WITH the hardware shift |
| g_consulting · g_infrastructure | +0.20 | weaker demand link |
| g_software · core_margin | +0.30 | software mix lifts margin |
| g_consulting · core_margin | +0.10 | low-margin segment |
| g_infrastructure · core_margin | +0.15 | operating leverage |
| g_software · ma_ratio | +0.30 | M&A buys software growth |
| g_consulting · ma_ratio | +0.10 | occasional consulting tuck-ins |
| rf · erp | −0.30 | risk-off: rates down, premium up |
| rf · g_software / g_consulting / g_infrastructure | +0.15 / +0.15 / +0.10 | strong nominal economy raises both |
| erp · g_software / g_consulting / g_infrastructure | −0.25 / −0.25 / −0.15 | risk aversion in weak-demand worlds |
| erp · core_margin | −0.15 | margin pressure in risk-off worlds |
| erp · ma_ratio | −0.10 | M&A slows when premia blow out |
| tg · g_software / g_consulting / g_infrastructure | +0.40 / +0.30 / +0.20 | long-run growth ties to realized growth (the F6 tail-fattening link) |
| tg · core_margin | +0.15 | durable franchises keep both |
| tg · rf | +0.25 | shared inflation component |
| tg · erp | −0.20 | risk-off marks down long-run growth |

Explicit ZEROS (also assumptions): ma_ratio·g_infrastructure (M&A is software-directed),
ma_ratio·core_margin (spend timing ⊥ operating margin), ma_ratio·rf and ma_ratio·tg
(financing conditions second-order to strategy), core_margin·rf (no direct mechanism).

## Pre-review output (seed 7, 10k sims, 10y)

Per share: **P10 $90 / P50 $160 / P90 $265** · TV/EV at means 63%.
Attribution (P50 delta at ±1σ, correlations intact): core_margin ∓31/+32 ·
g_software −21/+26 · ma_ratio +23/−23 · erp +24/−19 · rf +18/−15 · tg −13/+16 ·
g_consulting −9/+10 · g_infrastructure −7/+10.

Cross-check vs v3 aggregate model (P50 $141 [85, 223]): the driver P50 is ~$19 higher —
mix-weighted segment growth (~3.1% all-in charged basis) sits above v3's paired 2.15%;
the reviews must adjudicate whether the segment means respect the v3 pairing discipline
(growth credited only where its M&A cost is charged). Tails are wider (correlations working).

## Open questions this register poses to its reviewers

1. Does each segment growth mean double-credit acquired growth given ma_recurrence=0.5?
   (The v3 F16 pairing rule, now per segment.)
2. sbc_ratio deterministic at 2.5%: is a rising trend better modeled as deterministic drift?
3. beta deterministic post-crash: does the regime change justify 0.85, or stochastic beta?
4. Are the tg·growth loadings (+0.4/+0.3/+0.2) strong enough to honor F6, or still thin-tailed?
5. Clip-after-correlation (engine): ma_ratio lo=0 sits 2.2σ from the mean — acceptable?
6. Growth constant per path remains (T-DCF-1 AR(1) unimplemented) — material at segment level?

## Source ledger

Segment data: Jan 24 2022, Jan 25 2023, Jan 24 2024, Jan 29 2025, Jan 28 2026 Q4 8-K
Ex. 99.1 filings (sec.gov, URLs in the research record); Q2-2026 warning: Jul 14 2026 8-K
Ex. 99.1. Margin/bridge/WACC basis: analysis/IBM/README.md (v3) + REVIEW.md dispositions.
