# Driver review — round 1 verdicts and author dispositions

19 independent reviews (one per driver incl. deterministic ones, one per item formula, one
for the correlation matrix), all instructed to challenge the stochastic-vs-deterministic
choice, parameters vs sources, and consistency with the binding v3 ledger (REVIEW.md).
**Verdicts: 18 REVISE / 1 SOUND.** Several reviewers reproduced P50 $160 with the engine
before judging. Consolidation reviews per item follow the fixes below.

## Driver verdicts → dispositions

| Driver | Verdict | Key findings (condensed) | Disposition |
|---|---|---|---|
| g_software | REVISE | Mean 6% double-credits acquired growth ~1–1.25pp ≈ $9–12/sh (F16: organic ~3.75–4%, half-paid credit ~1pp); "5–11%" history cherry-picks recast bases; 10y of 11%+ implausible | **ACCEPT**: mean 5.0%, truncnorm hi=11%; history footnote corrected |
| g_consulting | REVISE | Symmetric normal wrong-way for GenAI-cannibalization left tail; mixture proposed (implied mean ~1.75%); recast base disclosed | **ACCEPT-MOD**: mean 1.75%, std 2.5% (skew proxy; mixture dist = engine ticket); recast footnote |
| g_infrastructure | REVISE | Constant-per-path punishes sawtooth: 10y-CAGR dispersion ≈1.5–2.7%, not annual 8.4%; 2.3% of paths compound ≥+5%/yr forever — never happened | **ACCEPT-MOD**: std 2.5%, truncnorm hi=+3%; rationale rewritten as 10y-CAGR dispersion; mean −1% kept, embed disclosed |
| g_financing | **SOUND** | Materiality math verified; bridge-consistency (F15) confirmed; flat > −1.2% history read | Two disclosure lines added (tg grows flat segment in terminal; infra volume link severed) |
| core_margin | REVISE | 5y-mean basis contradicts F19 (declining 3 straight yrs) → mean should be FY2025 level 19.4%; mix independence omits deterministic +1.2pp software-mix drift by y10 (+$10–15) — implement BOTH together | **ACCEPT**: mean 19.4%; new deterministic mix-drift schedule 0→+1.2pp (linear) added to the margin formula |
| sbc_ratio | REVISE | Constant 2.5% embeds unexplained trend break (ratio rose 1.74→2.54 accelerating); ramp to 3.5% = −$13/sh, historical pace = −$26; 2.54→2.5 rounding flattering; MSFT at 4.3% | **ACCEPT**: deterministic per-year schedule 2.54%→3.5% by y10; SBC-drift locus (2.54 flat / 3.5 / 4.5) published |
| ma_ratio | REVISE | Mean 6.6% is flattering-backward (simple avg 7.06%, endpoint 12.3% accelerating; endpoint rule applied inconsistently); right tail too thin (lognormal); clip mass 1.4% OK | **ACCEPT-MOD**: mean 7.1%; std 1.8% per margin-formula reviewer's decade-average argument (conflicts with fat-tail demand — adjudicated: level ↑ and dispersion honest for a 10y average; lognormal = engine ticket) |
| ma_recurrence | REVISE | "Fake precision" rationale backwards (sampling would *widen* honestly); correct defense is per-path pairing coherence; f hardcoded in TWO places (silent-drift risk); headline must be conditional-on-f | **ACCEPT**: deterministic kept with pairing rationale; single MA_RECURRENCE constant + assert; conditional labeling |
| rf | REVISE | One permanent draw ≠ "rates are stochastic" — reframe as permanent-component uncertainty; mean stale: 10y ≈ 4.56–4.60% (Jul 2026); lo=1% decorative | **ACCEPT**: mean 4.6% (spot, cited); framing rewritten; std 0.5% kept under permanent-component reading; lo labeled formal guard |
| beta | REVISE | "Double-counting ERP" rationale technically wrong (loading vs price of risk compound legitimately); regime question unanswered (trailing ~0.68 post-crash, forward arguably higher); β absent from attribution; 7.34% ≠ v3's 7.5% silently | **ACCEPT-MOD**: 0.85 deterministic kept on epistemic grounds (rationale corrected); β locus 0.68–1.00 published; v3 top-of-band position explicitly retired (build now fully sourced) |
| erp | REVISE | Rationale thinnest in file — must state state-variable/correlation justification; 4.75% unsourced, 30–50bp above Damodaran (4.23% Jan-26, ~4.45% Jul-26); dispersion math sane | **ACCEPT**: mean 4.5% (Damodaran cited); rationale rewritten; std 0.75% kept |
| kd_after_tax | REVISE | 4.1% fails provenance (looks pre-tax; fair-value math → ~3.5% after-tax); fixed kd while rf floats → negative credit spreads on 3.8% of paths | **ACCEPT-MOD**: kd now DERIVED per path: (rf − 0.10%) × (1−21%), spread fair-value-calibrated (blended ~7y ladder yields ≈ 4.45% pre-tax at rf 4.56%); τ=0 scenario published |
| equity_weight | REVISE | Disclosure ≠ treatment: iterate to price-value fixed point (E/V≈0.72, +$8–9); core-debt swap rejected as cherry-pick (correct); non-stationary as debt grows | **ACCEPT-MOD**: E/V iterated at means to fixed point in the analysis layer; non-stationarity + Harris-Pringle relever = ticket |
| terminal_growth | REVISE | Stochastic-vs-beta asymmetry must be defended; guard inert (report binding freq); mean 2.25% survives its own mix-weighted-fade math; std 0.5% is tightest-to-impact — widen | **ACCEPT-MOD**: std 0.75%; asymmetry defended in rationale (tg = world-state outcome, correlated with realized growth — not an estimation constant); derived-per-path tg = ticket T-DCF-3; pinned fraction now reported by engine |

## Formula + matrix verdicts → dispositions

| Target | Verdict | Key findings | Disposition |
|---|---|---|---|
| Revenue formula | REVISE | Reconciliation exact (verified); FY2025 base is a cycle-peak for Infrastructure (+12%, FX tailwind); year 1 ignores known H1-26 actuals; FX unmodeled undisclosed | **ACCEPT**: Infrastructure base = FY24/FY25 average ($14,869M); year-1 segment growth pinned to warning actuals (SW +5%, Cons 0%, Infra −7%), draws start t=2 (engine support); FX register row |
| Margin formula | REVISE | Single ma_ratio draw held 10y converts lumpiness into permanent level (std should be decade-average scale); F19 inexpressible in time-free formula; completeness otherwise acceptable per F15 | **ACCEPT**: ma_ratio std ↓ (above); F19 via core_margin endpoint + mix-drift schedule (above); ΔWC disclosure line |
| WACC formula | REVISE | rf sampled while kd frozen → negative spread paths (3.8%); flat term structure OK-disclose; register hides 7.34% vs v3 7.5% (~$6/sh of the $19 gap misattributed) | **ACCEPT**: kd derived from rf path (above); term-structure disclosure; gap attribution split honestly in DRIVERS.md |
| TV formula | REVISE | Guard inert (1/200k) but must report pinned fraction; tg·ma_ratio=0 breaks F16 in terminal (cost-without-growth); no ROIC anchor at TV/EV 63% | **ACCEPT**: pinned fraction in result; corr(tg, ma_ratio)=+0.15 added; implied-reinvestment check published |
| Correlation matrix | REVISE | Demand block sound; rf·g_* sign regime-dependent (publish supply-shock sensitivity rf·g=−0.15); least defensible zero: ma_ratio·rf → −0.15; PSD headroom large; F6 tail now fatter than v3 (0.439 vs 0.398 relative left spread) — remaining debt is AR(1), not the matrix | **ACCEPT**: ma_ratio·rf=−0.15, tg·ma_ratio=+0.15 added, PSD re-verified; supply-shock sensitivity run published |

## Engine changes this round (driver_dcf.py)

1. `Driver.schedule` — deterministic per-year value paths ((years,1) broadcast) for sbc_ratio ramp and margin mix-drift.
2. `kd` derived per path from rf + deterministic spread (new `kd_spread`, `tax_rate` roles replace `kd_after_tax`).
3. `SegmentSpec.year1_growth` — pin year 1 to known actuals, draws from t=2.
4. `DriverDCFResult.tg_pinned_fraction` — guard-binding frequency reported.
5. Margin formula gains the mix-drift term: margin = core + mix_drift − sbc − f×ma.

## Deferred (ticketed, disclosed)

- T-DCF-1 (AR(1)/mean-reverting growth) — now the single largest known distortion (both the infrastructure and correlation reviewers converged on it; measured −10% P50 in v3 experiments).
- T-DCF-3 — terminal growth derived per path from faded mix-weighted segment draws.
- Mixture / lognormal marginals (g_consulting skew, ma_ratio right tail).
- Harris-Pringle re-levered β with target capital structure.

## Consolidation reviews (round 2 — one reviewer per item, reviewing the completed work-package)

**TERMINAL VALUE — CONSOLIDATION: ACCEPTED.** "All dispositions implemented and verified by
execution: tg std 0.75% with world-state rationale; pinned fraction reported (0.0009, seed 7);
tg·ma_ratio=+0.15 in matrix, PSD passing; implied-reinvestment check printed (RR 20.8%, ROIC
10.8% > WACC — no cost-without-growth at means). Year-10 terminal margin 13.55% (core 19.4 +
mix 1.2 − SBC 3.5 − f·M&A 3.55) is internally consistent with tg at segment means. tg
attribution (−19/+26) is second only to core_margin at TV/EV 64.1%; judged honest because tg
comoves with growth/rates/M&A draws, but the anchoring is correlational, not structural —
T-DCF-3 (derived per-path tg) remains the binding open ticket."

**MARGIN — CONSOLIDATION: REOPENED (ma_ratio std) → FIXED.** "core_margin (19.4% endpoint +
mix-drift), sbc_ratio ramp, ma_recurrence pairing/assert, and the mix-drift margin formula are
faithfully implemented and verified (means path 13.31%→13.55%; broadcast tests pass; mix-drift's
+1.2pp is consistent with a 9.3pp software-share shift, and does not double-count the SBC ramp).
REOPENED: ma_ratio std 1.8% understates decade-average uncertainty — 1.8% is the AR(1)
decade-average only at ρ≈0.6; the 12.3% accelerating FY2025 endpoint argues ρ≈0.8, i.e. std
≈2.4%, and at 1.8% a decade at the endpoint level is a 2.9σ event." **Author fix: std 2.4%
implemented; lo=0 now 3.0σ out.**

**REVENUE — CONSOLIDATION: REOPENED (3 items) → FIXED.** "Segment dispositions verified in code
and accepted; g_financing disclosures present. Reopened: the Infrastructure year-1 pin (−7%)
applied to the FY24/25-average base double-counts the cycle correction, understating year-1
Infrastructure by ~$790M versus warning-implied actuals; the FX register row was never added
(DRIVERS.md remains pre-review); and the pairing assert covers only the ma_recurrence leg —
g_software's f-credit is hardcoded in two files, so changing MA_RECURRENCE drifts silently."
**Author fixes: pin level-targeted (−1.69% on the haircut base → warning-implied $14,618M);
g_software mean now DERIVED (`SOFTWARE_ORGANIC_G + MA_RECURRENCE × SOFTWARE_ACQUIRED_G`) and
the locus slope derives from the same constants; DRIVERS.md rewritten post-review with the FX
register row.**

**WACC — CONSOLIDATION: REOPENED (documentation) → FIXED.** "Implementation verified — kd
derived from rf (identity test), E/V 0.72 fixed-point self-consistent, WACC at means 7.06%, MC
std 0.56%, no negative-spread paths (constant −10bp disclosed). Conflicts #2/#3 upheld: spread
fair-value-calibrated; top-of-band retirement justified (+$11.5/sh vs 7.34%, +$17.6 vs 7.50% at
means; conservatism lives transparently in β 0.85 vs trailing 0.68). Reopened solely on
documentation: DRIVERS.md table still pre-revision and lacks the promised gap attribution and
term-structure disclosure." **Author fix: DRIVERS.md rewritten — post-review table, 7.50→7.06
gap attribution, term-structure and FX disclosures.**

**Round-2 close-out (v4 final, seed 7, 10k):** P10 $83.84 / P50 $151.84 / P90 $258.03,
TV/EV 64.1%, tg pinned 0.09%. All reopened items fixed and re-verified (220 tests).

## Reviewer conflicts adjudicated by the author (consolidators' rulings above: #1 overturned to std 2.4%, #2 and #3 upheld)

1. **ma_ratio**: driver reviewer wants mean ≥7.1 AND fatter right tail; margin-formula reviewer wants dispersion shrunk to decade-average scale. Resolution: mean 7.1%, std 1.8% — level honesty and averaging honesty; the tail lives in the ticket.
2. **kd spread**: WACC-formula reviewer proposed +0.89% (preserves old 4.1%); kd reviewer showed 4.1% fails provenance and market yield ≈ 4.45% pre-tax. Resolution: spread −0.10% (fair-value calibrated); the old level is retired with the position.
3. **WACC level**: with sourced inputs (rf 4.6, ERP 4.5, β 0.85, kd derived, E/V iterated ~0.72) the build lands ~7.1–7.2% — BELOW both 7.34% and v3's 7.5%. The v3 top-of-band position is retired explicitly rather than back-solved.
