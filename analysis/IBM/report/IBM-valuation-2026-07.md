# IBM — Valuation Report

**Post-crash driver-based valuation · July 2026 · Model v4**

Prepared 2026-07-16 · IBM (NYSE) · Price at writing: $212.40 · Prior close (Jul 13): $290.23

Engine: `python -m analysis.IBM.dcf_drivers_run` (10,000 simulations, seed 7, 10-year horizon). All valuation figures in this report were reproduced from that command on the date above. Assumption register: DRIVERS.md; review record: REVIEW.md and DRIVER-REVIEW.md.

## 1. Executive summary

At $212.40, IBM trades above this model’s median value under every M&A-recurrence assumption we publish. The v4 driver-based Monte Carlo DCF values IBM at **P10 $83.84 / P50 $151.84 / P90 $258.03**, conditional on the headline judgment that half of IBM’s recurring acquisition spend is a cost of staying in place (f = 0.50). Even the most generous published reading — f = 0, where all acquisition spend is treated as optional and only organic software growth is credited — puts the median at $197.79, still about 7% below the price. Reaching $212 inside this framework requires a discount rate below roughly 6.5% on any single published lever — holding every other judgment at base; stacked readings were not run. At the trailing post-crash beta of 0.68 the bottom-up WACC falls to ≈6.5% and the median is still only $179.42. **Interpretation.** The price embeds either a materially cheaper cost of capital than we can construct bottom-up, or resolution of the bull path — one price identifies a region of assumptions, not a point.

July 22, when IBM reports full Q2 results, adjudicates three falsifiable numbers: (a) whether the slipped large deals closed; (b) the FY2026 free-cash-flow guide against the ~$15.7B pre-warning bar; and (c) whether margins hold the FY2025 endpoint on which the model’s base is set. A fourth check is segment actuals against the model’s pinned year-1 growth (Software +5%, Consulting flat, Infrastructure at the warning-implied ~$14.6B).

Two caveats belong in the first paragraph a reader skims. First, terminal value is 64.1% of enterprise value at driver means — the valuation leans heavily on the Gordon inputs. Second, each simulated path holds one growth draw for ten years, so the left tail is understated; no margin-of-safety claim is made from the brackets (Section 4 callout).

## 2. What happened

On July 14, 2026, IBM fell **25.2% — its worst single day on record**, worse than October 19, 1987 — after preliminary Q2 results released ahead of the July 22 report [1][2]:

| Q2 2026 (preliminary) | Actual | Prior year | y/y | Street expected |
|---|---|---|---|---|
| Revenue | ~$17.2B | ~$17.0B | +1% | ~$17.9B |
| EPS, GAAP | $2.27 | ~$2.32 | −2% | — |
| EPS, adjusted | $2.93 | ~$2.80 | +~5% | $3.02 |

Management’s explanation (CEO Arvind Krishna, pre-earnings call): (1) clients rotated quarterly capex away from IBM software and infrastructure into servers, storage, and memory to lock in supply-constrained AI hardware ahead of price increases — IBM anticipated some impact “but not the magnitude”; (2) large software and consulting deals slipped past the quarter — the majority of the shortfall; (3) the release of Anthropic’s Mythos stalled several large deals while customers re-evaluated [1][2].

**Street reaction.** HSBC cut to Reduce with a $191 target [5]; BofA stayed at Buy, $330 → $280 [6]; Argus stayed at Buy, $360 → $280 [7]; Morgan Stanley held Equal-weight and moved $267 → $293, noting the market is “already pricing in” the upside [4]; Oppenheimer first downgraded IBM to Perform on the weak guidance, then a later note reiterated Outperform with a $350 target [3]. Median of these targets: **$280**.

Our median value ($151.84) sits far below that median, and the gap has a specific location: sell-side models effectively charge little or nothing for IBM’s recurring acquisition spend, and/or discount below 7%. On our published loci (Section 4), the most generous single reading — f = 0 at the base 7.06% WACC — reaches $197.79, and cutting beta to the trailing 0.68 lifts the base median by $28; no single published lever reaches $280. **Interpretation.** The consensus target requires stacking near-zero M&A recurrence with a discount rate well below our bottom-up build — each defensible only if IBM’s M&A is genuinely optional and its risk genuinely low.

## 3. Valuation framework

### 3.1 Architecture: deterministic items, explicit drivers

The v4 model separates *items* — accounting identities computed the same way on every path — from *drivers* — the model’s actual assumptions, each either stochastic (sampled) or deterministic (fixed or scheduled), with the choice itself reviewed and justified. The executable definition is `analysis/IBM/driver_model.py`; the engine is `financial_analysis/driver_dcf.py`.

| Item | Formula | Notes |
|---|---|---|
| Revenue | Σ segments: base × (1 + g_seg)^t; year 1 pinned to warning-implied levels, draws from t = 2 | growth constant per path (ticket T-DCF-1); growth is nominal all-in, FX unmodeled |
| FCFF margin | core + mix_drift(t) − sbc(t) − f × ma | mix drift and SBC are deterministic per-year schedules |
| WACC | E/V × (rf + β × ERP) + (1 − E/V) × (rf + spread) × (1 − τ) | kd derived per path from rf (no negative-spread paths); flat term structure — one WACC for years 1–10 and terminal |
| Terminal value | Gordon at min(tg, WACC − 2%), discounted 10 years | guard pinned on 0.09% of paths (seed 7) |

Equity value per share = (PV of FCFF + PV of TV − net debt $54.0B) / 948.7M diluted shares. Both figures carry over from the v3 stock-vs-flow bridge (README v3): net debt = total debt $61.3B + after-tax pension deficit $7.1B − cash and marketable securities $14.4B, with operating leases and financing receivables deliberately excluded because their flows already live in the margin; shares are held flat because stock-based compensation (SBC) is charged as a cash cost — growing the count too would double-charge dilution [8].

Segment bases are FY2025 actuals from the Q4 8-K [8] (Software $29,962M, Consulting $21,055M, Financing $737M), except Infrastructure, which uses the FY24/25 average $14,869M as a mainframe-cycle-peak haircut. Year-1 growth is pinned to the July 14 warning: Software +5%, Consulting 0%, Financing 0%, and Infrastructure −1.69% on the haircut base — level-targeted to the warning-implied $14,618M so the cycle correction is not double-counted. Draws apply from year 2.

### 3.2 The parameter register (full vector)

All rationales are reproduced in full in DRIVERS.md and as executable strings in `driver_model.py`. S = stochastic, D = deterministic. Every row is an assumption; the two central judgments are flagged.

| Driver | S/D | Dist | Mean | Std | Bounds / schedule | Basis (short) |
|---|---|---|---|---|---|---|
| g_software | S | truncnorm | 5.0% = 4.0% organic + f × 2.0% acquired | 2.5% | hi 11% | F16 decomposition; credit derived from the same f charged in the margin |
| g_consulting | S | normal | 1.75% | 2.5% | — | GenAI-cannibalization left-skew proxy; mixture ticketed |
| g_infrastructure | S | truncnorm | −1.0% | 2.5% | hi +3% | decade-CAGR dispersion, not annual volatility |
| g_financing | D | — | 0.0% | — | — | 1.1% of revenue; below materiality for sampling |
| core_margin | S | normal | 19.4% (FY2025 endpoint) | 2.0% | — | F19: three-year decline makes the endpoint, not the 5-year mean, the honest base |
| margin_mix_drift | D | schedule | 0 → +1.2pp by y10 | — | linear | mechanical software-mix consequence of the growth means |
| sbc_ratio | D | schedule | 2.54% → 3.5% by y10 | — | linear | ratio rose 1.74% → 2.54% since 2021; constant embeds a trend break |
| ma_ratio | S | truncnorm | 7.1% | 2.4% | lo 0 (3.0σ) | FY21–25 average 7.06%; decade-average dispersion at AR(1) ρ ≈ 0.8 |
| ma_recurrence | D | — | **f = 0.50 (headline judgment)** | — | — | pairing coherence; headline conditional on f; locus published |
| rf | S | truncnorm | 4.6% (10y spot, Jul 2026) | 0.5% | lo 1% (formal guard) | permanent-component uncertainty, not annual rate vol |
| beta | D | — | **0.85 (judgment)** | — | locus 0.68–1.00 published | epistemic constant; trailing ~0.68 vs higher forward risk |
| erp | S | truncnorm | 4.5% (Damodaran, Jul 2026) [9] | 0.75% | lo 2% | state variable of risk appetite; comoves with demand |
| kd_spread | D | — | −0.10% over rf | — | — | fair-value calibrated to the ~7y debt ladder |
| tax_rate | D | — | 21% | — | — | statutory shield; τ = 0 disclosure run published |
| equity_weight | D | — | 0.72 | — | — | iterated price-value fixed point (converges to ≈0.726) |
| terminal_growth | S | normal | 2.25% | 0.75% | joint guard tg < WACC − 2% | world-state outcome, correlated with realized growth |

The correlation matrix over the eight stochastic drivers is reproduced in full in Appendix A (25 non-zero entries plus three deliberate zeros, each an assumption with a rationale row in DRIVERS.md; positive semi-definite, minimum eigenvalue 0.419). One regime disclosure matters: rf·g > 0 assumes demand-driven rates; the supply-shock sensitivity (rf·g = −0.15) is published by the engine — the median is nearly unchanged ($151.89) and the tails widen.

The fundamentals grounding these parameters (SEC EDGAR via `scripts.analyze IBM`; margin basis defined in README v3):

| FY | Revenue | Net income | OCF | CapEx | SBC | Acquisitions | FCFF v3 margin |
|---|---|---|---|---|---|---|---|
| 2021 | $57.4B | $5.7B | $12.8B | $2.1B | $1.0B | $3.3B | 18.5% |
| 2022 | $60.5B | $1.6B | $10.4B | $1.3B | $1.0B | $2.3B | 14.8% |
| 2023 | $61.9B | $7.5B | $13.9B | $1.2B | $1.1B | $5.1B | 19.9% |
| 2024 | $62.8B | $6.0B | $13.4B | $1.0B | $1.3B | $3.3B | 18.9% |
| 2025 | $67.5B | $10.6B | $13.2B | $1.1B | $1.7B | $8.3B | 16.9% |

FY2025 capital-return arithmetic frames the bear case: OCF $13.2B − capex $1.1B − dividends $6.3B − acquisitions $8.3B = **−$2.5B**, funded by a $6.3B debt increase. The dividend-plus-M&A model is currently debt-funded.

### 3.3 Discount rate

At driver means: 0.72 × (4.6% + 0.85 × 4.5%) + 0.28 × (4.6% − 0.10%) × (1 − 21%) = **7.06%** (Monte Carlo std 0.56%). Every input is externally sourced: rf 4.6% is the 10-year Treasury spot; ERP is Damodaran’s July 2026 implied premium (4.45%, carried as 4.5% in the register) [9]; kd is derived per path from rf at a fair-value-calibrated −10bp spread; E/V = 0.72 is the iterated price-value fixed point, not the circular post-crash market weight. The v3 “7.5% top-of-band” position is retired, not back-solved — the gap decomposes as rf +17bp, ERP −15bp, E/V −14bp, kd −17bp, and retirement of the old top-of-band rounding −15bp. The WACC level alone is worth ≈ +$11.5/share versus 7.34% and +$17.6/share versus 7.50%. Residual conservatism lives transparently in β = 0.85 (versus trailing ~0.68), with the beta locus published in Section 4.

### 3.4 How the model got here: v1 to v4 under adversarial review

This is the fourth version of the valuation, and each step down was an identified error, not a change of opinion. v1 concluded “fairly valued at $212” on a $202 median — retracted after review found debt charged twice, $22.3B of acquisitions treated as free while their growth was credited, and an unsourced 8.5% WACC (findings F1–F3 in the adversarial-review ledger, REVIEW.md). v2’s rebuild ($166) surfaced a net-debt bridge that double-counted stocks whose flows stayed in the margin (finding F15) and an incoherent growth/M&A pairing (finding F16). v3 ($141, signed off) fixed both and published full parameter vectors. v4 rebuilt the model driver-by-driver: 19 independent driver reviews — eighteen of nineteen demanded revisions; all were resolved — plus four consolidation reviews moved the median to $151.84 — chiefly by replacing the top-of-band 7.5% WACC with the fully sourced 7.06% build, offset by an endpoint-based margin, an SBC ramp, and a higher M&A charge (DRIVER-REVIEW.md). The binding dispositions ledger is REVIEW.md; nothing retracted there is resurrected here.

## 4. Results

**Headline (engine-verified, seed 7, 10k sims, conditional on f = 0.50): P10 $83.84 / P50 $151.84 / P90 $258.03.** TV/EV at means 64.1%; terminal-growth guard pinned on 0.09% of paths.

> **Tail caveat — applies to every bracket in this report.** Each path holds one growth draw for all ten years; mean reversion is unmodeled (ticketed model improvement T-DCF-1, the largest known distortion), so the left tail is understated even after the v4 correlation matrix fattened it relative to v3. “P50s are robust; P10s are optimistic. No margin-of-safety claim is made from these tails” (README v3 tail caveat, per the F6 disposition — the adversarial review’s tail-sampling finding; carried verbatim). With TV/EV at 64.1%, the valuation leans heavily on the Gordon inputs. Until AR(1) growth lands, treat P10/P90 as decorative.

**Attribution** (P50 delta at mean ± 1σ, same seed, correlations intact — engine output):

| Driver | −1σ | +1σ |
|---|---|---|
| core_margin | −$31.34 | +$31.43 |
| terminal_growth | −$19.27 | +$26.34 |
| rf | +$23.29 | −$18.96 |
| erp | +$22.68 | −$18.54 |
| g_software | −$18.18 | +$21.19 |
| ma_ratio | +$18.75 | −$18.71 |
| g_consulting | −$9.87 | +$11.66 |
| g_infrastructure | −$5.45 | +$5.58 |

The model’s verdict is margin-led: two points of core margin move the value by ±$31, more than any demand or rate driver.

**Published loci** (each varies one judgment, everything else at base; engine output):

| Locus | Setting | P50 |
|---|---|---|
| M&A recurrence (growth credit paired: g_software mean = 4.0% + f × 2.0%) | f = 0.00 | $197.79 |
| | f = 0.25 | $175.37 |
| | **f = 0.50 (headline)** | **$151.84** |
| | f = 0.75 | $127.46 |
| | f = 1.00 | $101.44 |
| SBC ratio | flat 2.54% | $164.11 |
| | **ramp to 3.5% (base)** | **$151.84** |
| | ramp to 4.5% | $139.20 |
| Beta | 0.68 (trailing, post-crash) | $179.42 |
| | **0.85 (base)** | **$151.84** |
| | 1.00 | $132.34 |

Disclosure runs: supply-shock correlation regime (rf·g = −0.15) P50 $151.89 with wider tails ($81.24–$263.54); no debt tax shield (τ = 0) P50 $140.72.

**Terminal-value sanity check** (engine-verified, at means): terminal reinvestment = f × ma_ratio = 3.55% of revenue against a 13.55% terminal FCFF margin, a 20.8% reinvestment rate (of pre-reinvestment cash earnings, 17.10% of revenue); sustaining tg = 2.25% on that reinvestment implies an incremental ROIC of 10.8% — above the 7.06% WACC and plausible rather than heroic, so the terminal value no longer carries growth the model isn’t paying for.

## 5. Reading the price, and July 22

**The price sits above every published median.** At the base f = 0.50 the market pays $60 (40%) over the model’s median; at f = 0 — conceding that IBM’s entire acquisition program is optional — the gap is still $15. The cheapest published route toward $212 is the discount rate: at trailing β 0.68 the WACC build lands at ≈6.5% and the median reaches $179.42, still $33 short. Stacked generosity (f = 0 *and* trailing beta) was not run and is not claimed; within the published grid, $212 requires a sub-6.5% discount rate or the bull path. **Assumption.** To be explicit about boundaries: f = 0.50 and β = 0.85 are judgments, and both loci are published precisely because reasonable readers can sit elsewhere on them.

**The options market is pricing the event, not the verdict.** As of the 2026-07-16 snapshot (README v3): the Jul-24 expiry implies a **7.6% earnings move** (ATM IV 60.4% versus ~48% on adjacent expiries), skew is mildly call-bid, and put/call open interest is 0.69 — positioning leans toward recovery.

**July 22 watch list (falsifiable, with the numbers):**

1. **Did the slipped deals close?** Management attributed the majority of the Q2 shortfall to slipped large deals [1]. Closure is the bull gate; failure to close converts “timing” into demand loss.
2. **FY2026 FCF guide versus ~$15.7B.** IBM reported FY2025 FCF of $14.7B (its definition) and guided FY2026 up “about $1B” pre-warning [8]. A guide materially below ~$15.7B quantifies the damage in management’s own currency.
3. **Margin at the FY2025 endpoint.** The v4 base already concedes the adversarial review’s margin-trend finding (F19) — core margin is set at the FY2025 endpoint (19.4% pre-SBC, pre-M&A basis), not the flattering 5-year mean. July 22 tests whether even that holds: each further point of core margin is worth ≈ $16/share (attribution above).
4. **Segment actuals versus the year-1 pins.** Software +5%, Consulting flat, Infrastructure ~$14.6B implied for the year. Software materially below +5% undercuts the g_software mean from above; Infrastructure above the pin suggests the cycle haircut was too harsh.

## 6. Data and limitations

Honest limits, all disclosed in the underlying artifacts:

- **Model debt (ticketed model improvements).** T-DCF-1: growth is one draw per path — AR(1) mean reversion is the largest known distortion (a reviewer experiment with φ = 0.6 cut the v3 median ~10% and P90 ~16%). T-DCF-3: terminal growth should derive per path from faded segment draws; today it is correlationally, not structurally, anchored. Mixture/lognormal marginals (consulting left skew, M&A right tail) and Harris–Pringle re-levered beta with target weights are ticketed. FX is unmodeled; the WACC term structure is flat.
- **Pipeline defects (disclosed).** The data store contains phantom “10-K” rows (2012–2019 stubs) that carry no FCF and don’t enter the model. FY2019–20 recast mismatch makes margin windows longer than five years unusable. Dividends, post-2019 buybacks, and the prices array are null in the pipeline: the price ($212.40), pre-crash close ($290.23), dividend (~$6.3B), analyst targets, and guidance figures are hand-entered from the cited sources, not pipeline outputs. Importer fixes are ticketed model improvements (T-IMP-1).
- **Structural.** Segment history is hand-entered from the 8-Ks (sourced). The headline is conditional on f = 0.50 by construction; there is no model-internal way to estimate f — it is the analysis’s central judgment, which is why the locus is published.

**Reproduce:** `python -m scripts.analyze IBM` (financials, prices, option chain) · `python -m analysis.IBM.dcf_drivers_run` (v4 headline, loci, attribution, disclosure runs) · `python -m analysis.IBM.dcf_scenarios` (v3 scenarios, retained for the review trail).

## Appendix A — Correlation matrix (8 stochastic drivers)

Entries are Gaussian-copula rank correlations; every entry including the zeros is an assumption (rationale rows in DRIVERS.md). PSD verified, minimum eigenvalue 0.419. Abbreviations: SW = g_software, CN = g_consulting, IN = g_infrastructure, CM = core_margin, MA = ma_ratio, TG = terminal_growth.

| | SW | CN | IN | CM | MA | rf | erp | TG |
|---|---|---|---|---|---|---|---|---|
| **SW** | 1 | .40 | .35 | .30 | .30 | .15 | −.25 | .40 |
| **CN** | .40 | 1 | .20 | .10 | .10 | .15 | −.25 | .30 |
| **IN** | .35 | .20 | 1 | .15 | 0 | .10 | −.15 | .20 |
| **CM** | .30 | .10 | .15 | 1 | 0 | 0 | −.15 | .15 |
| **MA** | .30 | .10 | 0 | 0 | 1 | −.15 | −.10 | .15 |
| **rf** | .15 | .15 | .10 | 0 | −.15 | 1 | −.30 | .25 |
| **erp** | −.25 | −.25 | −.15 | −.15 | −.10 | −.30 | 1 | −.20 |
| **TG** | .40 | .30 | .20 | .15 | .15 | .25 | −.20 | 1 |

Review-driven entries: MA·rf = −0.15 (debt-financed M&A slows when rates spike — the least defensible zero, fixed) and TG·MA = +0.15 (bought growth priced in the terminal — F16 in perpetuity). Regime disclosure: rf·g > 0 assumes demand-driven rates; the supply-shock sensitivity (rf·g = −0.15) is published by the engine.

## Appendix B — Sources

External:

1. CNBC, “IBM warns second-quarter earnings fell short of expectations” (Jul 14, 2026) — https://www.cnbc.com/2026/07/14/ibm-warns-second-quarter-earnings-fell-short-of-expectations.html
2. Bloomberg, “IBM says second-quarter sales missed as customers pulled back” (Jul 14, 2026) — https://www.bloomberg.com/news/articles/2026-07-14/ibm-says-second-quarter-sales-missed-as-customers-pulled-back
3. Benzinga, Oppenheimer downgrade and later reiteration — https://www.benzinga.com/markets/large-cap/26/07/60493640/whats-going-on-with-ibm-stock-on-thursday-2
4. MarketScreener, Morgan Stanley target change — https://www.marketscreener.com/news/morgan-stanley-raises-price-target-on-ibm-to-293-from-267-keeps-equalweight-rating-ce7f5edcd18af12d
5. StreetInsider, HSBC downgrade — https://www.streetinsider.com/Analyst+Comments/HSBC+Downgrades+IBM+(IBM)+to+Reduce,+Stretched+Valuation/26764503.html
6. Investing.com, BofA target cut — https://www.investing.com/news/analyst-ratings/bofa-cuts-ibm-stock-price-target-on-revenue-miss-software-weakness-93CH-4791206
7. Investing.com, Argus target cut — https://www.investing.com/news/analyst-ratings/argus-cuts-ibm-stock-price-target-to-280-on-weak-q2-guidance-93CH-4795696
8. IBM Q4-2025 8-K, Exhibit 99.1 (Jan 28, 2026; FY2025 results, segment detail, FY2026 FCF guide) — https://www.sec.gov/Archives/edgar/data/51143/000005114326000004/ibm-20260128xex991.htm — plus Q4 8-K Ex. 99.1 filings 2022–2026 for segment history
9. A. Damodaran, implied equity risk premium (July 2026 update; 4.45% cited, 4.23% Jan 2026) — https://pages.stern.nyu.edu/~adamodar/

Internal (repo artifacts; all numbers above trace to these):

- `analysis/IBM/DRIVERS.md` — consolidated v4 assumption register
- `analysis/IBM/driver_model.py` — normative executable model definition
- `analysis/IBM/README.md` (v3) — crash narrative, fundamentals, net-debt bridge, options snapshot, data notes
- `analysis/IBM/REVIEW.md` — v1→v3 adversarial review ledger (19 findings, dispositions, deferred tickets)
- `analysis/IBM/DRIVER-REVIEW.md` — v4 driver-level reviews (19 round-1 + 4 consolidation) and dispositions
