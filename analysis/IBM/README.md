# IBM — post-crash valuation (July 2026)

Analysis date: **2026-07-16**. Price at writing: **$212.40** (down ~27% from the $290.23 close on July 13).

## What happened

On **July 14, 2026** IBM fell **25.2% — its worst day on record** (worse than Oct 19, 1987) after releasing preliminary Q2 results ahead of its July 22 earnings date:

| Metric | Preliminary Q2 | Street expected |
|---|---|---|
| Revenue | ~$17.2B (+1% y/y) | ~$17.9B |
| Adj. EPS | $2.93 (reported down 2% to $2.27 per some outlets) | ~$3.01–3.02 |

Management's explanation (CEO Arvind Krishna, pre-earnings call):

1. **Capex rotation** — clients shifted quarterly spend away from IBM software/infrastructure toward servers, storage, and memory chips to lock in supply-constrained AI hardware ahead of price increases. IBM anticipated some impact but "not the magnitude."
2. **Large deals slipped** — big software/consulting deals failed to close on expected timelines; this drove the majority of the shortfall.
3. **AI model disruption** — Krishna cited the release of Anthropic's Mythos, which stalled several large deals while customers re-evaluated.

Street reaction was split: HSBC cut to Reduce ($191 target); BofA cut $330→$280 (kept Buy); Argus cut $360→$280 (kept Buy); Oppenheimer *raised* to $350 and Morgan Stanley to $293 — i.e., most of the Street reads this as a timing problem, not a structural break.

Sources: [CNBC](https://www.cnbc.com/2026/07/14/ibm-warns-second-quarter-earnings-fell-short-of-expectations.html) · [Bloomberg](https://www.bloomberg.com/news/articles/2026-07-14/ibm-says-second-quarter-sales-missed-as-customers-pulled-back) · [Forbes](https://www.forbes.com/sites/petercohan/2026/07/15/ibm-stock-loses-67-billion--causes-and-recovery-outlook/) · [Fortune](https://fortune.com/2026/07/15/ibm-stock-price-crash-25-percent-analyst-reaction/) · [TradingKey](https://www.tradingkey.com/analysis/stocks/us-stocks/262031493-ibm-stock-forecast-july-15-2026-crash-q2-warning-213-support-tradingkey)

## Fundamentals (SEC EDGAR, via `scripts.analyze IBM`)

| FY | Revenue | Net income | OCF | CapEx | FCF | FCF margin |
|---|---|---|---|---|---|---|
| 2021 | $57.4B | $5.7B | $12.8B | $2.1B | $10.7B | 18.7% |
| 2022 | $60.5B | $1.6B | $10.4B | $1.3B | $9.1B | 15.0% |
| 2023 | $61.9B | $7.5B | $13.9B | $1.2B | $12.7B | 20.5% |
| 2024 | $62.8B | $6.0B | $13.4B | $1.0B | $12.4B | 19.8% |
| 2025 | $67.5B | $10.6B | $13.2B | $1.1B | $12.1B | 17.9% |

- Revenue CAGR FY2020→FY2025: **4.1%**
- Derived FCF margin (5y): **18.4% ± 1.9%**
- Net debt (FY2025 10-K): $61.3B total debt + finance leases − $13.6B cash − $0.8B securities ≈ **$46.9B**
- Shares outstanding: **0.937B**
- Dividend: ~$6.3B/yr (~3.1% yield at $212)

## Monte Carlo DCF (`python -m analysis.IBM.dcf_scenarios`)

10,000 sims, 10-year horizon, FCF margin sampled from history, terminal Gordon growth. Equity per share = (PV of FCF − net debt) / shares.

| Scenario | Growth | WACC | Terminal | Eq/sh P10 | **Eq/sh P50** | Eq/sh P90 |
|---|---|---|---|---|---|---|
| Bear — AI disruption real, growth stalls | 1% | 9.5% | 1.5% | $73 | **$111** | $165 |
| Base — mature grower, historical trend | 4% | 8.5% | 2.5% | $135 | **$202** | $309 |
| Bull — Q2 was timing, momentum resumes | 6% | 8.0% | 3.0% | $194 | **$294** | $468 |

## Read

- **The pre-crash price ($290) was the bull case.** Bull P50 is $294 — the market had been pricing IBM as a 6%+ grower with AI tailwinds.
- **Today's price ($212) is the base case.** Base P50 is $202 — the crash repriced IBM from bull to roughly fair value for a 4% mature grower, *not* to a bargain.
- **Margin of safety only appears below ~$150** (base P10 $135; midpoint of bear/base P50s ≈ $155).
- The bet at $212 is binary on the July 22 full report: if the deal slippage converts in 2H (deals delayed, not lost) the bull case re-rates toward $290+; if the capex-rotation/Mythos disruption is structural, bear P50 is $111 and there's ~45% downside.

**Verdict: fairly valued after the drop — not the discount a 27% crash suggests. Watch July 22 for (a) whether slipped deals closed, (b) full-year FCF guidance (~$13.5B was the pre-warning bar), (c) software segment growth ex-Red Hat.**

## Reproduce

```bash
python -m scripts.analyze IBM            # pull financials + prices from EDGAR/yfinance
python -m analysis.IBM.dcf_scenarios     # run the three DCF scenarios
```
