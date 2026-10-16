# Proposal: an investor verdict layer (2026-10-15)

Status: **proposed, for Luc's review.** Author: Merlan (merlan1996), drafted with Claude.
Shipped with it, as a first additive slice: `financial_analysis/verdict.py`,
`scripts/verdict.py`, `tests/test_verdict.py`. The engines, registers, presets and
published reports are untouched.

## The idea in one paragraph

Today the pipeline stops at "here is the value range." An investor needs "here is what
to do, how sure we are, what I could make or lose, and what would prove us wrong." The
proposal adds that layer on top of the driver engine, following established industry
methods, and adds three things we found no mainstream rating publishing: the exact
assumptions today's price requires, returns under realistic partial convergence, and a
scored record of how well calibrated every call was. It is the investor-facing form of
the vision's L6 (gap attribution + bet ledger).

## Built on industry standards

| Standard | Source | How we apply it | What we add |
|---|---|---|---|
| Fair value with a margin of safety that widens with uncertainty | Morningstar Equity Research Methodology: bull/bear cases at 25% probability each | Verdict cutoffs at our own P25 / P75 | Uncertainty measured from 10,000 paths and checked by backtest, not set by judgment |
| Price-implied expectations (reverse DCF) | Mauboussin & Rappaport, *Expectations Investing* | Solve each stochastic driver for the price | Automatic per driver, sorted into testable bets vs stances, each with a settlement date |
| Clear valuation method and risks | FINRA Rule 2241 | Already true: rationale strings in `driver_model.py` | Every number reproducible from code + seed |
| Opinion separated from fact | CFA Institute Standard V(B) | Judgments labeled (f = 0.50, beta = 0.85) | Each judgment published with its locus |
| Quality / forensic screens | Altman Z'', Beneish M, Piotroski F | (follow-up slice) | Used as vetoes against value traps, not return predictors |
| Scoring probabilistic forecasts | Brier score (Good Judgment Project) | Score every call at its horizon | A published calibration record; sell-side 12-month targets are met only 38% of the time (Bradshaw, Brown & Huang 2013) |

## What ships in this PR (all computed by `python -m scripts.verdict IBM --price 212.40 --dividend-yield 0.0313`)

**1. Verdict band.** Deciding number: the chance the price is too high (share of
simulated values below the price). Below 25% = Strong opportunity; 25-40% Opportunity;
40-60% No edge; 60-75% Overpriced; above 75% = Strongly overpriced. The outer cutoffs
are Morningstar's 25%/75% calibration; the middle ones are judgment, to be tuned by
backtest. Quartiles are also the honest anchor for this engine, since P10/P90 are
"decorative" until T-DCF-1.

| P10 | P25 | P50 | P75 | P90 | Price (Jul 2026) | Chance too high | Verdict |
|---|---|---|---|---|---|---|---|
| $83.84 | $112.57 | $151.84 | $200.39 | $258.03 | $212.40 | 79.2% | Strongly overpriced |

**2. What the price requires** (one stochastic driver moved at a time, others at base,
seed 7; search range = the driver's bounds, else mean ± 6σ):

| Driver | Base | Price-implied | Reading | Type |
|---|---|---|---|---|
| core_margin | 19.4% | 23.3% | ~4 pts above FY2025 after three years of decline | Testable bet (each quarter's FCF) |
| g_consulting | 1.75% | 11.5% | Far above FY22-25 history (+7.1/+5/-1/+2%) | Testable bet (segment results) |
| g_software | 5.0% | not reachable within bounds (hi 11%) | - | Testable bet |
| g_infrastructure | -1.0% | not reachable within bounds (hi 3%) | - | Testable bet |
| ma_ratio | 7.1% | not reachable within bounds (lo 0) | Even zero acquisition spend does not reach the price | Testable bet |
| rf | 4.6% | 3.47% | Market-wide rates | Stance |
| erp | 4.5% | 2.71% | Far below Damodaran's 4.45% (Jul 2026) | Stance, market-wide |
| terminal_growth | 2.25% | 3.69% | Forever, for a mature company | Stance, untestable in an investor's horizon |

For IBM the actionable reading is: the price needs a ~4-point margin recovery (or
consulting growth far above its history); the next earnings reports test it. Stances
shared by every stock are not mispricing evidence (vision L6 basket decomposition).

**3. Returns under partial convergence.** The log price-to-value gap closes a fraction
k per year (Capozza & Israelsen 2010: 15-30% of the gap closes per year, 8,845 stocks,
1997-2006); dividends compound on top. Return per year, 3-year hold:

| Scenario | k = 0.15 | k = 0.30 |
|---|---|---|
| P10 | -8.5% | -15.9% |
| P50 | -1.2% | -4.2% |
| P90 | +5.7% | +7.6% |

Chance of losing money over 3 years: 59.3% (k = 0.15) to 68.3% (k = 0.30), lower than
the 79.2% chance the price is too high because prices close only part of the gap and
dividends keep paying. This horizon probability is the one a track record should score.
Benchmark: the S&P 500 compounded about 10% a year over 1928-2025 (Damodaran data, via
secondary summaries), so even IBM's good case trails the index.

## Proposed next slices (not in this PR)

1. Grade the July 22 IBM watch list and re-run IBM on current data: first ledger entries.
2. Wire the verdict section into `scripts/visualize_dcf.py`.
3. Screens module (Altman Z'', Beneish M, Piotroski F, ROIC vs WACC) from imported data.
4. Scored ledger: record each call (git-timestamped), grade at horizon, Brier score and
   calibration chart; pre-registered kill criteria per testable bet.
5. Calibrate the middle verdict cutoffs with `backtest.py`; then ORCL end to end.
6. Later research: options-implied (Breeden-Litzenberger) vs fundamental distribution;
   filing-triggered re-grading (gives THOUGHTS item 2 a disciplined form).

## Caveats

- Horizon is 1-5 years; not a trading tool, not personal advice.
- IBM inputs are as of the July 2026 report; the RPO ledger facts have expired.
- Single-driver solves ignore combinations of smaller moves; a joint view is a follow-up.
- Shared limits of the engine carry through (T-DCF-1 tails; growth-phase tickers).

## Questions for Luc

- Audience: outside investors, or our own investing first?
- Comfortable with explicit verdicts, and with publishing misses as well as hits?
- Before, alongside, or after the P1 work (validators, red-team corpus, coverage)?
- Should the SPY/QQQ trading-bot branch move to its own repo?

## Sources

- Morningstar Equity Research Methodology (text dated 23 Oct 2020): https://s25.q4cdn.com/634000362/files/doc_downloads/1/Morningstar-Equity-Research-Methodology-2023-06-14.pdf
- Mauboussin & Rappaport, *Expectations Investing* (excerpt): https://www.michaelmauboussin.com/s/EIexcerpt.pdf
- FINRA Rule 2241: https://www.finra.org/rules-guidance/rulebooks/finra-rules/2241
- CFA Institute Standard V(B): https://www.cfainstitute.org/standards/professionals/code-ethics-standards/standards-of-practice-v-b
- Bradshaw, Brown & Huang (2013), Review of Accounting Studies: https://ideas.repec.org/a/spr/reaccs/v18y2013i4d10.1007_s11142-012-9216-5.html
- Capozza & Israelsen (2010), via CXO Advisory: https://www.cxoadvisory.com/fundamental-valuation/speeddeterminants-of-stock-price-reversion/
- Piotroski follow-up (Creighton): https://researchworks.creighton.edu/esploro/outputs/journalArticle/Value-Stocks-and-Accounting-Screens-Has/991006117468602656
- Good Judgment Project summary: https://aiimpacts.org/?p=1283
- Historical returns 1928-2025: https://awealthofcommonsense.com/2026/01/historical-returns-for-stocks-bonds-cash-housing-gold-2025/
