---
id: DOC-IBM-SOURCE-AUDIT
type: report
item: 10.74 Verification report
status: frozen
date: 2026-07-16
scope: Claim-to-source audit of the IBM July 2026 report against the engine, for the two owners; frozen.
---
# Source audit — IBM-valuation-2026-07 (docx + md master)

Auditor: report-auditor (final gate). Date: 2026-07-16. Audited artifact: `IBM-valuation-2026-07.docx`
(read from a scratchpad copy; tables extracted cell-by-cell via `officecli get`) against the markdown
master, `data/processed/IBM.json` (XBRL store), `analysis/IBM/driver_model.py`, DRIVERS.md, README.md (v3),
REVIEW.md, DRIVER-REVIEW.md, and a fresh engine re-run.

Engine re-run: `python -m analysis.IBM.dcf_drivers_run` (seed 7 built in, 10k sims, 10y) — executed on the
audit date; full output captured. **Every MODEL number in the report reproduced exactly.** The docx and md
master carry identical content (verified paragraph-by-paragraph and cell-by-cell).

Binding constraints respected: REVIEW.md dispositions (F1–F19) and DRIVER-REVIEW.md consolidation rulings
were not re-litigated; edits below are softens only.

## 1. Claim → tag table

Grouped where a table row or sentence carries one derivation. Status ✓ = traced/reproduced.

| # | Claim (location) | Tag | Source / check | Status |
|---|---|---|---|---|
| 1 | Price at writing $212.40; prior close (Jul 13) $290.23 (header, §6) | MARKET | Hand-entered, disclosed as such in §6; README v3 | ✓ |
| 2 | Jul 14 fall −25.2%, worst single day on record, worse than Oct 19 1987 (§1, §2) | PRESS | [1][2]; README v3 §What happened | ✓ |
| 3 | Q2-26 preliminary table: rev ~$17.2B vs ~$17.0B py vs ~$17.9B street; GAAP EPS $2.27 vs ~$2.32; adj EPS $2.93 vs ~$2.80 vs $3.02 (tbl 1) | PRESS | [1][2]; identical in README v3 | ✓ |
| 4 | Management explanation ×3 incl. Krishna "but not the magnitude", Mythos deal stalls (§2) | PRESS | [1][2] | ✓ |
| 5 | Street actions: HSBC Reduce $191 [5]; BofA Buy 330→280 [6]; Argus Buy 360→280 [7]; MS EW 267→293 + "already pricing in" [4]; Oppenheimer Perform→later Outperform $350 [3] (§2) | PRESS | [3]–[7] all resolvable in Appendix B | ✓ |
| 6 | Median of targets $280 (§2) | MODEL(arith) | median{191,280,280,293,350} = 280 | ✓ |
| 7 | Headline P10 $83.84 / P50 $151.84 / P90 $258.03, conditional f=0.50 (§1, §4) | MODEL | engine re-run, exact | ✓ |
| 8 | TV/EV at means 64.1%; tg guard pinned 0.09% (§1, §3.1, §4) | MODEL | engine: 64.1%, 0.0009 | ✓ |
| 9 | f-locus P50s: 197.79 / 175.37 / 151.84 / 127.46 / 101.44 (§4 tbl 6) | MODEL | engine, exact | ✓ |
| 10 | SBC locus: 164.11 / 151.84 / 139.20 (§4) | MODEL | engine, exact | ✓ |
| 11 | Beta locus: 179.42 / 151.84 / 132.34 (§4) | MODEL | engine, exact | ✓ |
| 12 | Disclosure runs: rf·g=−0.15 → P50 151.89, tails 81.24–263.54; τ=0 → 140.72 (§3.2, §4) | MODEL | engine, exact | ✓ |
| 13 | Attribution table, 8 rows (−31.34/+31.43 … −5.45/+5.58) (§4 tbl 5) | MODEL | engine, exact all 16 values | ✓ |
| 14 | Terminal check: reinvest 3.55% of rev, terminal margin 13.55%, RR 20.8% of 17.10%, implied ROIC 10.8% > 7.06% (§4) | MODEL | engine, exact; 13.55 = 19.4+1.2−3.5−3.55 | ✓ |
| 15 | WACC at means 7.06% (MC std 0.56%); formula arithmetic (§3.3) | MODEL | 0.72×(4.6+0.85×4.5)+0.28×4.5×0.79 = 7.061%; std per DRIVERS.md/consolidation review | ✓ |
| 16 | Gap decomposition rf+17 / ERP−15 / E/V−14 / kd−17 / rounding−15 bp; +$11.5 vs 7.34%, +$17.6 vs 7.50% (§3.3) | MODEL | DRIVERS.md §WACC + DRIVER-REVIEW WACC consolidation (verified sums to −44bp = 7.50→7.06) | ✓ |
| 17 | E/V = 0.72 iterated fixed point, converges ≈0.726 (§3.2, §3.3) | MODEL | engine prints iteration path → 0.726 | ✓ |
| 18 | β 0.68 ⇒ WACC ≈6.5% (§1, §5) | MODEL(arith) | 0.72×(4.6+0.68×4.5)+0.995 = 6.51% | ✓ |
| 19 | Gap arithmetic: $60 (40%) over P50; $15 over f=0; $33 short at β 0.68; β cut lifts median by $28; f=0 ≈7% below price (§1, §2, §5) | MODEL(arith) | 60.56/39.9%; 14.61; 32.98; 27.58; 6.9% | ✓ |
| 20 | "$212 requires sub-6.5% rate or bull path on any single published lever; stacked readings not run" (§1, §5) | INTERPRETATION | hedged ("roughly", "inside this framework", stacking disclaimed); max single lever = 197.79 < 212 | ✓ |
| 21 | "Each further point of core margin ≈ $16/share"; "two points move ±$31" (§4, §5) | MODEL(arith) | ±1σ = 2.0pp = ±$31.4 | ✓ |
| 22 | Fundamentals table FY2021–25 (rev/NI/OCF/capex/SBC/acq, 30 values) (§3.2 tbl 4) | FILING | all match `IBM.json` mapped fields (e.g. FY25: 67,535 / 10,593 / 13,193 / 1,091 / 1,715 / 8,294 M) | ✓ |
| 23 | FCFF v3 margin column 18.5/14.8/19.9/18.9/16.9% (tbl 4) | FILING+MODEL | recomputed from JSON per README v3 formula (OCF + 0.79×(int exp − int inc) − capex − SBC)/rev — all 5 reproduce | ✓ |
| 24 | Capital-return arithmetic: 13.2−1.1−6.3−8.3 = −$2.5B, funded by $6.3B debt increase (§3.2) | FILING | dividends: raw_extras `PaymentsOfDividendsCommonStock` $6.25B; debt delta 61,259−54,943 = $6.32B | ✓ |
| 25 | Net-debt bridge: total debt $61.3B + after-tax pension $7.1B − cash & securities $14.4B = $54.0B; 948.7M diluted shares (§3.1) | FILING+ASSUMPTION | XBRL: debt incl. current 61,259M; pension liab 9,020M×0.79 = 7.13B; cash 13,587M + AFS current 830M = 14.42B; shares 948,675,228. **Softened** (E1): "deficit" → "liability (liability-only; surplus asset excluded)" per the signed-off v3 bridge wording | ✓ after E1 |
| 26 | Segment bases SW $29,962M / Cons $21,055M / Fin $737M; Infra FY24/25 avg $14,869M; warning-implied $14,618M; −1.69% pin; y1 pins +5/0/0 (§3.1) | FILING(8-K)+ASSUMPTION | driver_model.py constants match; sum + Other $63M = 67,535M = XBRL revenue; 15,718×0.93 = 14,618; 14,618/14,869−1 = −1.69% | ✓ |
| 27 | Parameter register, 16 rows (§3.2 tbl 3) | ASSUMPTION | each row matches DRIVERS.md post-review register + driver_model.py; f=0.50 and β=0.85 flagged as judgments in-table. SBC basis cell **softened** (E2): 1.74% → ~1.7% (XBRL: 982/57,350 = 1.71%) | ✓ after E2 |
| 28 | ma_ratio basis "FY21–25 average 7.06%" (tbl 3) | FILING | JSON recomputation 7.07% (rounding vs 8-K-sourced register; mean used is 7.1%) — within rounding | ✓ |
| 29 | g_financing "1.1% of revenue" (tbl 3) | FILING | 737/67,535 = 1.09% | ✓ |
| 30 | rf 4.6% 10y spot; ERP 4.45% Damodaran Jul-26 (4.23% Jan-26), carried 4.5%; kd spread −10bp fair-value calibrated (§3.3, tbl 3) | MARKET | [9]; DRIVER-REVIEW rf/erp/kd rows (10y ≈ 4.56–4.60%; ladder ≈4.45% pre-tax) | ✓ |
| 31 | Trailing post-crash β ~0.68 (§1, §3.3, §5, tbl 3) | MARKET/ASSUMPTION | driver_model.py beta rationale; locus published; presented as probe not fact | ✓ |
| 32 | Correlation matrix, 28 entries + "25 non-zero, 3 deliberate zeros", PSD min eigenvalue 0.419 (§3.2, App A) | ASSUMPTION+MODEL | matrix in App A == CORR in driver_model.py (all 28); numpy eigvalsh min = 0.4193; counts verified | ✓ |
| 33 | Review-driven entries MA·rf=−0.15, TG·MA=+0.15 (App A) | ASSUMPTION | DRIVERS.md + DRIVER-REVIEW matrix/TV dispositions | ✓ |
| 34 | v-history: v1 $202/"fairly valued"; F1 debt double-charge, F2 $22.3B acquisitions free, F3 8.5% unsourced; v2 $166, F15, F16; v3 $141 signed off; 19 driver reviews, 18/19 REVISE, 4 consolidations (§3.4) | FILING(internal ledger) | REVIEW.md table + findings ledger; DRIVER-REVIEW.md header + consolidation sections; $22.3B = Σ acq FY21–25 in JSON | ✓ |
| 35 | Tail caveat quote "P50s are robust; P10s are optimistic…" carried verbatim per F6 (§4) | FILING(internal) | verbatim in README v3 tail caveat; F6 disposition | ✓ |
| 36 | φ=0.6 AR(1) experiment cut v3 median ~10%, P90 ~16% (§6) | MODEL(historical) | REVIEW.md T-DCF-1 ticket | ✓ |
| 37 | v4 matrix fattened left tail vs v3 (§4) | MODEL | DRIVER-REVIEW matrix row: 0.439 vs 0.398 relative left spread | ✓ |
| 38 | FY2025 FCF $14.7B (IBM definition); pre-warning guide +~$1B → ~$15.7B bar (§1, §5) | FILING/PRESS | [8]; README v3 reconciliation | ✓ |
| 39 | Core margin 19.4% = FY2025 endpoint, pre-SBC pre-M&A basis (F19) (§5, tbl 3) | FILING+ASSUMPTION | README v3 margin basis; F19 disposition; endpoint concession stated | ✓ |
| 40 | Options snapshot 2026-07-16: implied 7.6% move, ATM IV 60.4% vs ~48%, call-bid skew, P/C OI 0.69 (§5) | MARKET | README v3 (options-extractor snapshot); "positioning leans toward recovery" hedged. Lead sentence **softened** (E3) to a reading | ✓ after E3 |
| 41 | "Price embeds cheaper cost of capital or bull resolution"; "consensus requires stacking…" (§1, §2) | INTERPRETATION | both explicitly labeled **Interpretation.** | ✓ |
| 42 | "f = 0.50 and β = 0.85 are judgments…" (§5) | ASSUMPTION | labeled **Assumption.** | ✓ |
| 43 | §6 limitations: T-DCF-1/T-DCF-3, phantom 10-K rows, FY2019–20 recast, null dividends/buybacks/prices, hand-entered items, no model-internal f | FILING(internal)+MODEL | REVIEW.md tickets; README v3 data notes; JSON confirms mapped `dividends_paid`/`repurchases_common_stock`/prices null | ✓ |

**Claim counts by tag (grouped rows above; sub-claims in parentheses):**
FILING 11 (≈60 individual numbers) · MARKET 5 (≈10) · PRESS 5 (≈20) · MODEL 15 (≈55 numbers re-derived) ·
ASSUMPTION 7 (register 16 rows + matrix 28 entries + structural choices) · INTERPRETATION 3 (all labeled after E3).
**Untraceable claims: 0. Non-reproducing numbers: 0** (two rounding notes: ma average 7.07% vs stated 7.06%;
SBC start-ratio fixed by E2).

## 2. Calculation spot-checks (re-derived this audit)

1. Engine headline/loci/attribution/disclosure/terminal-check/E-V iteration — all 40+ published values exact (seed 7).
2. WACC identity at means → 7.061%; β=0.68 variant → 6.51%; gap decomposition sums to −44bp.
3. FCFF v3 margins, all five years, from XBRL + README v3 formula → 18.5/14.8/19.9/18.9/16.9 exact.
4. Net-debt bridge from XBRL: 61.26 + 7.13 − 14.42 = 54.0 (against $54.0B used).
5. Debt increase FY24→FY25: 61,259 − 54,943 = $6.32B ("$6.3B" ✓); dividends $6.25B ("~$6.3B" ✓).
6. Segment sum FY2025: 29,962+21,055+15,718+737+63(Other, dropped-disclosed) = 67,535 = XBRL revenue.
7. Correlation matrix PSD: min eigenvalue 0.4193 ("0.419" ✓); 25 non-zero / 3 zeros ✓.
8. Median analyst target = $280 ✓; all price-gap arithmetic in §1/§2/§5 ✓.

## 3. Statement-coverage flag pass (FY2025, for the P1 coverage audit — informational, not gating)

Scanned the FY2025 10-K period: mapped schema fields + all raw_extras concepts > ~0.5% of revenue ($0.34B).
Material items the model neither uses nor §6 explains, or worth an importer note:

- **FLAG C1 — Remaining performance obligation $71.0B** (`RevenueRemainingPerformanceObligation`, 105% of revenue).
  Contracted backlog is neither a model input (segment growth is assumption-driven) nor mentioned in §6. No report
  claim contradicts it; it plausibly bears on the July-22 "slipped deals" question. Recommend P1 decide use-or-disclose.
- **FLAG C2 — Overfunded-plan surplus asset $7.54B** (`DefinedBenefitPlanAssetsForPlanBenefitsNoncurrent`, 11% of revenue).
  Deliberately excluded from the net-debt bridge (README v3, sign-off condition 3 — binding). The report's §3.1 word
  "deficit" contradicted that liability-only construction → soften E1 applied. No further action; choice is signed off.
- **FLAG C3 — FY2025 income tax is a net benefit** (`income_tax_expense` = −$242M; effective rate ≈ −2.3% vs the 21%
  statutory shield used). Margins are OCF-based and the τ=0 disclosure run is published, and README v3 discloses the
  near-zero cash tax rate, but the report itself does not carry that one-line disclosure. Not contradictory (τ=0 run
  covers the direction that hurts). Informational.
- **FLAG C4 — Deferred revenue $16.1B current / $4.3B non-current** (24%/6% of revenue). The margin formula has no
  ΔWC term (historical OCF embeds WC swings; the margin-formula review's "ΔWC disclosure line" did not land in the
  report). Below contradiction threshold; note for P1.
- **FLAG C5 — Importer note:** `PaymentsOfDividendsCommonStock` ($6.25B) exists in raw_extras although the mapped
  `dividends_paid` is null — §6's "hand-entered" disclosure is accurate for the mapped schema, and the hand-entered
  ~$6.3B is confirmed by the raw concept. Cheap T-IMP-1 win.
- Checked and NOT flagged (used or accounted for): financing receivables $7.7B + lease receivables ($4.3B) — excluded
  with F15 rationale; operating leases $3.35B — excluded with rationale; goodwill/intangibles (stock items, flow-based
  model; goodwill acquired $5.9B consistent with the M&A charge); sparse pre-2018 10-K rows — covered by §6 recast
  disclosure; ShortTermBorrowings $6.42B — inside the $61.3B total-debt concept used.

## 4. Softens applied (both .docx and .md master; officecli issues 0, validation passed)

| # | Location | Before | After | Why |
|---|---|---|---|---|
| E1 | §3.1 bridge sentence | "…+ after-tax pension deficit $7.1B −…" | "…+ after-tax pension liability $7.1B (liability-only; the overfunded-plan surplus asset is deliberately excluded) −…" | $7.1B is the gross liability × (1−21%); XBRL shows a $7.54B overfunded-plan surplus asset. "Deficit" read as net funded status — stronger than the signed-off liability-only construction it cites. |
| E2 | §3.2 register, sbc_ratio basis | "ratio rose 1.74% → 2.54% since 2021" | "ratio rose ~1.7% → 2.54% since 2021" | XBRL: FY2021 SBC 982M / rev 57,350M = 1.71%; 1.74% does not reproduce from the store (2.54% endpoint reproduces exactly). Trend claim unaffected. |
| E3 | §5 options lead | "**The options market is pricing the event, not the verdict.**" | "**Our reading of the options market: it is pricing the event, not the verdict.**" | "The market prices…" is the taxonomy's flagged INTERPRETATION class; was phrased as fact. Now phrased as a reading; supporting data unchanged. |

No other edits. No content restructured; no model value changed; no signed-off disposition re-opened.

## Audit verdict: PASS

Every material claim traces to a tagged source; every MODEL number reproduced from a fresh seed-7 engine run;
every FILING number checked against `data/processed/IBM.json` (or its disclosed hand-entered source); all PRESS
claims resolve to Appendix B; all INTERPRETATIONs are labeled as readings after E3. Coverage flags C1–C5 are
handed to the P1 coverage audit; none gates this report.
