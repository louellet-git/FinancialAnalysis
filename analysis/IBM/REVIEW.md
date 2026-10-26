---
id: DOC-IBM-REVIEW
type: report
item: 10.17 Evaluation report
status: frozen
date: 2026-07-16
scope: Binding record of the three-round adversarial review (F1-F19) of the IBM valuation; frozen, not to be re-litigated.
---
# Adversarial review record — IBM valuation (July 2026)

Three rounds between the analysis author and an independent skeptical reviewer, 2026-07-16. The reviewer reproduced every published number with the repo's own engine (seed 7) before judging it; the author re-ran every accepted correction the same way. **Outcome: the original conclusion was retracted and the analysis rewritten (v3).**

| | Base case P50 | Verdict |
|---|---|---|
| v1 (original) | $202 | "Fairly valued at $212" |
| v2 (round-2 rebuild) | $166 | "Market prices M&A as free" |
| **v3 (final, signed off)** | **$141** | **"$212 unreachable at any M&A-recurrence fraction under base WACC; price embeds sub-6.5% WACC or bull resolution"** |

Each step came from a specific identified error, not a change of opinion.

## Findings ledger

| # | Sev | Finding | Disposition | P50 impact |
|---|---|---|---|---|
| F1 | CRIT | FCF was neither FCFF nor FCFE: post-interest OCF discounted at WACC *and* net debt subtracted — debt charged twice. Interest income mirror-image caught in round 2 (F1a). | FIXED v3: FCFF = OCF + 0.79×(int exp − int inc) − capex − SBC | +$34 then −$8 |
| F2 | CRIT | $22.3B of 2021–25 acquisitions treated as free while crediting the growth they bought | FIXED v2/v3: explicit M&A charge, 0–100% recurrence locus published | up to −$97 |
| F3 | CRIT | WACC 8.5% ± 1% unsourced; bottom-up ≈ 7.3% | FIXED v2: CAPM build shown; 7.5% used (top of band, disclosed) | +$35 |
| F4 | MAJ | 4.1% growth was trough-based (FY2020) and inorganic | FIXED: 2019-base all-in 2.65%; organic ≈ 1.65% derived | −$25 |
| F5 | MAJ | Static shares while adding back SBC — dilution double-benefit | FIXED: SBC charged as cash cost, diluted count, flat | −$17 |
| F6 | MAJ | Independent normal draws + perfectly autocorrelated growth → left tail understated | DEFERRED (ticket below); tail caveat now on every bracket; no margin-of-safety claims | tails only |
| F7 | MAJ | Net debt ignored pension, leases, captive | FIXED via bridge; then REOPENED (F15) and re-fixed | see F15 |
| F8 | MAJ | Pre-warning FCF bar misstated ($13.5B vs actual ~$15.7B guide; FY25 actual $14.7B) | FIXED, 8-K cited, definitions reconciled | factual |
| F9 | MAJ | Analyst citations spun (Oppenheimer downgrade omitted; MS equal-weight framed as bullish); $280 Street median never confronted | FIXED; gap paragraph added, explained via the locus | factual |
| F10 | MAJ | Scenario parameters circular — reproduced the two prices already known | FIXED: external anchors (guidance, organic history, CAPM); v3 base no longer recovers the price | credibility |
| F11 | MIN | Terminal value 58–65% of EV, unreported | FIXED: TV/EV per scenario (52/60/66%) | disclosure |
| F12 | MIN | Spread clip (wacc−tg ≥ 0.1%) creates tail absurdities; mean/std contaminated | DEFERRED (ticket); mean/std no longer quoted anywhere | tails only |
| F13 | MIN | Pipeline defects: phantom 10-K rows, recast mismatch (FY2020 margin garbage), null dividends/buybacks/prices; hand-entered figures undisclosed | PARTIAL: all disclosed in Data Notes; importer fixes ticketed | disclosure |
| F14 | MIN | Missing: capital-return arithmetic (FY25 −$2.5B, debt-funded), segment mix limit, clean GAAP/adj EPS comp | FIXED: all three added | disclosure |
| F15 | MAJ | v2 bridge double-counted stocks whose flows stayed in the margin (receivables credit ≈ capitalized captive earnings; cash credited while interest income in margin; lease/pension over-charges) — net +$24/sh flattery | FIXED v3: stock-vs-flow rule applied to every line; ND $47.5B → $54.0B | −$24 |
| F16 | MAJ | Growth/M&A pairing incoherent (all-in growth credited, half the cost charged) | FIXED v3: paired rule g = 1.65% + f×1.0pp; (g, f) shown per scenario; gradient thesis stated | −$25 |
| F17 | MIN | "Market prices M&A as free" was an unidentified inverse problem (3 free parameters, 1 price) | FIXED: rephrased as region identification; WACC grid published | wording |
| F18 | MIN | Incomplete parameter vectors → not reproducible | FIXED: full vectors in README table | reproducibility |
| F19 | MIN | Base margin (5y mean 17.8%) sits above a 3-year downtrend ending at 16.9% | DISCLOSED: at FY2025 level, base P50 ≈ $129 [75, 207]; July 22 FCF guide adjudicates | −$12 conditional |

## Author positions that survived

- WACC at the top of the defensible band (7.5% vs 7.3% computed): post-crash market-value weights are themselves circular and trailing beta understates forward risk. Reviewer accepted.
- Correlation-sign pushback on F6 was **partially rejected**: macro co-movement of growth and rates applies to systematic shocks, but IBM's crash was idiosyncratic, and the dominant correlation for a TV-heavy model (g–tg, same firm) unambiguously fattens the left tail. Recorded as accepted-rejection; hence the tail caveat.
- Interest-income magnitude: reviewer's ~$0.9B estimate was high; XBRL actuals (0.05–0.75B) reduce F1a's impact from −$15/sh to ~−$8/sh. Reviewer conceded on reproduction.

## Sign-off (round 3) and its four conditions

> "v3 is … internally consistent in every dimension I attacked. … No remaining substantive objections."

1. **This deferred-ticket ledger** (below) — deferrals were accepted *because* they are documented.
2. F8/F9/F14 prose lands as committed → done in README v3.
3. Pension disclosure: liability-only, after-tax, excludes overfunded-plan surplus asset, 10-K funded-status note cited → done in bridge table.
4. Full parameter vectors in the README → done.

## Deferred-fix tickets (model/pipeline work, out of scope for this branch)

- **T-DCF-1 (from F6):** correlated sampling — single-factor correlation across (g, tg, margin) at stated loadings; AR(1) mean-reverting growth (spec: φ=0.6 reverting to 2.5% cut base P50 ~10% and P90 ~16% in reviewer's experiment). Until then, P10/P90 are decorative.
- **T-DCF-2 (from F12):** truncated/lognormal draws for WACC and tg, enforce wacc > tg + 2% jointly; remove or winsorize mean/std in `DCFResult`.
- **T-IMP-1 (from F13):** importer — drop phantom annual periods (spurious "10-K" rows with 1–2 fields), align recast revenue with recast OCF across divestiture boundaries, populate dividends/buybacks/interest income as first-class fields, persist prices to the JSON store.

## Process note

Reviewer: independent agent instructed to attack and to re-verify every author claim with the engine. All valuation numbers in this record were reproduced independently by both sides (same seed) before being accepted. The v1 → v3 trajectory (−$61/sh, verdict inverted) is the measured cost of the assumptions the original analysis did not state.
