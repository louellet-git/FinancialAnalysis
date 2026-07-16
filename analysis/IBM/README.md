# IBM — post-crash valuation (July 2026), v3

Analysis date: **2026-07-16**. Price at writing: **$212.40** (down ~27% from the $290.23 close on July 13).

> **v3 note:** this analysis was rewritten after a three-round adversarial review that identified material errors in the original version (debt double-counting, no charge for recurring M&A, unsourced WACC, an inconsistent net-debt bridge). The full exchange, every finding, and the deferred-fix ledger are in [REVIEW.md](REVIEW.md). The original conclusion — "fairly valued at $212" — did not survive.

## What happened

On **July 14, 2026** IBM fell **25.2% — its worst day on record** (worse than Oct 19, 1987) after preliminary Q2 results released ahead of the July 22 report:

| Q2 2026 (preliminary) | Actual | Prior year | y/y | Street expected |
|---|---|---|---|---|
| Revenue | ~$17.2B | ~$17.0B | +1% | ~$17.9B |
| EPS, GAAP | $2.27 | ~$2.32 | −2% | — |
| EPS, adjusted | $2.93 | ~$2.80 | +~5% | $3.02 |

Management's explanation (CEO Arvind Krishna, pre-earnings call): (1) clients rotated quarterly capex away from IBM software/infrastructure into servers, storage, and memory to lock in supply-constrained AI hardware ahead of price increases — IBM anticipated some impact "but not the magnitude"; (2) large software/consulting deals slipped past the quarter — the majority of the shortfall; (3) the release of Anthropic's Mythos stalled several large deals while customers re-evaluated.

**Street reaction** — targets: HSBC Reduce/$191; BofA Buy, $330→$280; Argus Buy, $360→$280; Morgan Stanley **Equal-weight**, $267→$293, noting the market is "already pricing in" the upside; Oppenheimer **first downgraded IBM to Perform** on the weak guidance, then a later note reiterated Outperform/$350. Median of these targets: **$280**.

Our base value ($141, below) sits far below that median. The gap is explained by the pivotal judgment in this analysis: sell-side models effectively charge nothing for IBM's recurring acquisition spend and/or discount below 7.5%. On our locus (see valuation), the consensus $280 requires f≈0 recurrence *and* a sub-7% WACC — each defensible only if IBM's M&A is genuinely optional and its risk genuinely low.

Sources: [CNBC](https://www.cnbc.com/2026/07/14/ibm-warns-second-quarter-earnings-fell-short-of-expectations.html) · [Bloomberg](https://www.bloomberg.com/news/articles/2026-07-14/ibm-says-second-quarter-sales-missed-as-customers-pulled-back) · [Benzinga (Oppenheimer downgrade)](https://www.benzinga.com/markets/large-cap/26/07/60493640/whats-going-on-with-ibm-stock-on-thursday-2) · [MarketScreener (MS)](https://www.marketscreener.com/news/morgan-stanley-raises-price-target-on-ibm-to-293-from-267-keeps-equalweight-rating-ce7f5edcd18af12d) · [StreetInsider (HSBC)](https://www.streetinsider.com/Analyst+Comments/HSBC+Downgrades+IBM+(IBM)+to+Reduce,+Stretched+Valuation/26764503.html) · [Investing.com (BofA](https://www.investing.com/news/analyst-ratings/bofa-cuts-ibm-stock-price-target-on-revenue-miss-software-weakness-93CH-4791206), [Argus)](https://www.investing.com/news/analyst-ratings/argus-cuts-ibm-stock-price-target-to-280-on-weak-q2-guidance-93CH-4795696) · [IBM Q4-2025 8-K](https://www.sec.gov/Archives/edgar/data/51143/000005114326000004/ibm-20260128xex991.htm)

## Fundamentals (SEC EDGAR, via `scripts.analyze IBM`)

| FY | Revenue | Net income | OCF | CapEx | SBC | Acquisitions | FCFF v3 margin* |
|---|---|---|---|---|---|---|---|
| 2021 | $57.4B | $5.7B | $12.8B | $2.1B | $1.0B | $3.3B | 18.5% |
| 2022 | $60.5B | $1.6B | $10.4B | $1.3B | $1.0B | $2.3B | 14.8% |
| 2023 | $61.9B | $7.5B | $13.9B | $1.2B | $1.1B | $5.1B | 19.9% |
| 2024 | $62.8B | $6.0B | $13.4B | $1.0B | $1.3B | $3.3B | 18.9% |
| 2025 | $67.5B | $10.6B | $13.2B | $1.1B | $1.7B | $8.3B | 16.9% |

\* **FCFF v3** = OCF + (1−21%)×(interest expense − interest income) − capex − SBC, *before* any M&A charge. Mean **17.8% ± 1.8%** (sampling std floored at 2.0%). The 21% statutory tax shield is a modeling choice; IBM's FY2025 cash tax rate was near zero. Interest income per year from XBRL `InterestIncomeOther`.

- Revenue CAGR FY2019→FY2025 (recast base): **2.65%** all-in, *including* the contribution of $22.3B of acquisitions over 2021–25. FY2020-based CAGR (4.1%) is trough-based and not used. Estimated acquired-growth contribution: $4.46B/yr average spend at 6–9× software sales ≈ $0.5–0.75B revenue/yr ≈ **~1.0pp**, implying organic ≈ **1.65%**.
- **Margin trend warning (F19):** FCFF v3 margins have declined three straight years (19.9% → 18.9% → 16.9%) and the Q2 warning implies FY2026 below FY2025. The base case samples around the 5-year mean (17.8%); **if the FY2025 level is the new normal, base P50 falls from $141 to ~$129** [75, 207]. The July 22 FCF guide adjudicates which is right.
- **IBM-defined FCF reconciliation:** IBM reported FY2025 FCF of **$14.7B** (OCF excluding financing receivables, − capex) and guided FY2026 FCF up "about $1B" → **~$15.7B pre-warning bar** ([8-K](https://www.sec.gov/Archives/edgar/data/51143/000005114326000004/ibm-20260128xex991.htm)). Our model FCFF (~$12.4B pre-M&A-charge) differs by: +$3.2B financing-receivables build (IBM excludes it; we keep consolidated OCF), −$1.7B SBC (we charge it; IBM doesn't), +$1.0B after-tax net interest (we add back; IBM doesn't).
- **Capital-return arithmetic:** FY2025: OCF $13.2B − capex $1.1B − dividends $6.3B − acquisitions $8.3B = **−$2.5B**, funded by a $6.3B debt increase. The dividend-plus-M&A model is currently debt-funded — directly relevant to the bear case and the net-debt trajectory.

## Net-debt bridge (stock-vs-flow consistent)

Rule: an item may appear in the bridge **or** have its income/expense in the margin — never both.

| Item | $B | Note |
|---|---|---|
| Total debt incl. current + finance leases | 61.3 | 8-K / 10-K |
| + Pension/postretirement deficit, after-tax | +7.1 | $9.0B liability × (1−21%); liability-only (excludes the overfunded US qualified plan surplus asset) — the conservative, extractability-based choice; 10-K retirement-benefits note |
| − Cash & equivalents + marketable securities | −14.4 | its interest income is stripped from the margin (net-interest FCFF) |
| Operating leases | 0 | NOT added: full rent already burdens OCF in perpetuity |
| Financing receivables | 0 | NOT credited: the captive's income stays in the flows; crediting the stock too would double-count (the capitalized captive earnings ≈ the entire $11.7B credit — review finding F15) |
| **Adjusted net debt** | **54.0** | |

Shares: **948.7M diluted** (FY2025 10-K), held flat — SBC is charged as a cash cost, so growing the count too would double-charge dilution.

## Discount rate (bottom-up)

rf 4.3% + β 0.85 × ERP 4.75% → cost of equity ≈ 8.3%; after-tax cost of debt ≈ 4.1% on $61.3B; market-value weights E/V ≈ 76.5% (at $212 — itself circular post-crash, noted) → **WACC ≈ 7.3%, used 7.5% ± 0.75%** (top of the defensible band: trailing beta from the pre-warning regime plausibly understates forward risk). Sensitivity below.

## Monte Carlo DCF v3 (`python -m analysis.IBM.dcf_scenarios`)

10,000 sims, 10-year horizon, seed 7. Equity/share = (PV of FCFF − net debt $54.0B) / 948.7M. Growth and M&A charge are **paired**: charging fraction f of the $4.46B/yr average M&A spend credits g = organic 1.65% + f × 1.0pp acquired growth. The recurrence gradient across scenarios encodes a thesis: bull = organic AI momentum (deals return without buying growth); bear = disruption, where M&A becomes defensive spend that buys no growth.

Full parameter vectors (means ± stds; all normal draws, independent — see tail caveat):

| Scenario | g | f (M&A charged) | Net margin | WACC | Terminal g | **P50** | P10–P90† | TV/EV |
|---|---|---|---|---|---|---|---|---|
| Bear — AI disruption real | 1.00% ± 2% | 100% (+2pp erosion) | 9.2% ± 2% | 8.25% ± 0.75% | 1.5% ± 0.5% | **$37** | $6–77 | 52% |
| Base — organic + half-paid M&A | 2.15% ± 2% | 50% | 14.5% ± 2% | 7.50% ± 0.75% | 2.25% ± 0.5% | **$141** | $85–223 | 60% |
| Bull — guidance-anchored recovery | 4.50% ± 2% | 25% | 16.1% ± 2% | 7.25% ± 0.75% | 2.5% ± 0.5% | **$232** | $150–362 | 66% |

† **Tail caveat, applies to every bracket:** inputs are sampled independently and each path's growth is perfectly autocorrelated (one draw for 10 years); both understate the left tail. P50s are robust; P10s are optimistic. No margin-of-safety claim is made from these tails. Base TV/EV is 60% — the valuation leans heavily on the Gordon inputs. Year 1 applies g to the FY2025 base although H1-26 is already known at ~+1% (~−$3–4/sh, unmodeled).

**M&A-recurrence locus** (base WACC 7.5%, paired growth): f=0% → $177 · 25% → $160 · 50% → $141 · 75% → $122 · 100% → $102.

**WACC grid** (base pairing f=50%): 6.25% → $203 · 6.50% → $188 · 6.75% → $174 · 7.00% → $162 · 7.25% → $151 · 7.50% → $141. Reproducing $212 requires WACC ≈ **6.1%**.

## Read

**At $212, the market price is unreachable at any M&A-recurrence fraction under base WACC and paired growth** (maximum $177 at f=0). Reproducing $212 requires at least one of: WACC ≈ 6.1% (vs 7.3–7.5% bottom-up), perpetual all-in growth above 4% with M&A still half-charged, or the bull scenario resolving on July 22. That is one consistent reading among several — one price identifies a region of assumptions, not a point.

**Verdict: Bear $37 / Base $141 / Bull $232.** The stock trades above even the 0%-recurrence base value; the price embeds either a materially lower discount rate than we can construct bottom-up, or bull-case resolution.

**July 22 watch-list (falsifiable):** (a) did the slipped deals close (bull gate); (b) FY2026 FCF guide vs the ~$15.7B pre-warning bar (IBM definition); (c) FCFF margin: 5y-mean 17.8% vs FY2025-level 16.9% — at the latter, base P50 ≈ $129; (d) software growth ex-Red Hat (segment exposure of the warning — our model is one consolidated revenue line and cannot see mix, a structural limitation).

Options-market context (from the `options-extractor` branch, snapshot 2026-07-16): the Jul-24 expiry prices a **7.6% earnings move** (ATM IV 60.4% vs ~48% adjacent), skew mildly call-bid, P/C OI 0.69 — positioning leans toward recovery.

## Data notes (known defects, disclosed)

- The store contains phantom "10-K" rows (2012-06-30, 2018-01-01, 2019-01-01) with one or two stray fields; they carry no FCF and don't enter the model.
- FY2019–20 mix recast continuing-ops revenue with unrecast consolidated OCF → the FY2020 "28.2% margin" is spurious; **margin windows longer than 5 years are unusable** until the importer aligns recasts. The 5y window was used because it's the longest clean one, not by selection — and it sits *above* the 3y mean, i.e. against our own conclusion.
- `dividends_paid`, share repurchases (post-2019) and the JSON `prices` array are null/empty in the current pipeline; the price ($212.40), pre-crash close ($290.23), dividend (~$6.3B), analyst targets, and guidance figures are hand-entered from the cited sources, not pipeline outputs.
- Importer fixes are ticketed — see the deferred ledger in [REVIEW.md](REVIEW.md).

## Reproduce

```bash
python -m scripts.analyze IBM            # pull financials + prices + option chain
python -m analysis.IBM.dcf_scenarios     # v3 scenarios, loci, sensitivity grids
```
