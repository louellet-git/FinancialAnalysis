# No-arbitrage relations: how the industry keeps and monitors them (2026-11-21)

Ticket: `PLANNING/board/T-20261121-arb-invariants.md`, step 1. A research agent read each source on 2026-11-21.
The note `chiefs/louellet-git/quant/industry-2026-11-21.md` (here: "N0") already gives SR 26-2, the Cboe 15:45 ET
DataShop file, Wallmeier (2024), Gatheral-Jacquier, the NY Fed box rate, GSW and Keane. This note does not repeat them.
Source tags [S1] to [S22] point to the list at the end. "Our synthesis" marks a point that no source states.

## 1. How the relations are kept (the register)
- **No public source found** for the internal catalog of no-arbitrage relations of a quant firm or a market maker.
- The nearest public forms are rule files of monitoring tools. A Prometheus alerting rule has a name, an expression,
  a `for` duration, a `keep_firing_for` duration, labels and annotations [S8]. Deequ declares "unit tests for data":
  a constraint with a user threshold, computed as aggregation queries over the data set [S11].
- A regulator asks for an inventory with "sufficient information to understand model risks" (SR 26-2, in N0).
  BCBS 239 (Basel Committee, 2013) asks banks for accurate risk data (Principle 3) [S12].
- Our synthesis: the fields of the ticket (statement, formula, data, tolerance, source, check command, status) cover
  these forms. Add three fields from [S8]: a persistence count (`for`), a severity, and an owner.

## 2. How the checks run
- **Executable prices.** N0 covers bid and ask. Cboe uses the mid only to find the forward. It drops each series with a
  null quote or bid > ask [S1]. A quote is valid only with bid >= 0 and ask > bid [S1].
- **Rate in the bound.** Constantinides, Jackwerth, Savov use a rate implied by parity, not the T-bill rate. The T-bill
  rate "often leads to violations of put-call parity". Their rate is the median over pairs with moneyness 0.95 to 1.05,
  from mids. It is on average 24 bp below the T-bill rate [S3]. Cboe uses the Treasury CMT curve with a bounded
  natural cubic spline, and time in minutes over 525,600 [S1].
- **Borrow and costs.** Parity breaks on single stocks go in the direction of short-sale limits. Their size follows the
  rebate rate spread, and they persist net of costs [S13]. With costs and short-sale limits, very few S&P 500 index
  option parity breaks stay [S14]. On SPX European options, breaks are smaller than on American options, and they
  "reflect premia for liquidity (immediacy) risk" [S15].
- **Time match of legs.** Van Tassel matches VIX futures to 4 pm option quotes. VIX futures trade to 4:15 pm. On
  2018-02-05 the VIX moved much between 4:00 and 4:15 pm, so he uses the matched prices [S2]. N0 gives the 15:45 ET rule.
- **Stale quotes.** Cboe keeps two quotes for each series: the last valid quote, and the latest quote with the minimum
  spread in the 15 seconds before the time. A quote that repeats the previous bid and ask is not used [S1].
- **Data error or price.** Cboe flags an outlier when its spread is above gamma times an EMA of the spread and above a
  maximum lambda (the index rules give the values). It then uses the min-spread quote or the previous filtered quote [S1].
  CJS drop implied volatilities more than 2 standard deviations from a quadratic fit (5.7% of the data), and parity
  pairs more than 2 standard deviations from the daily median rate (300 pairs) [S3].
- Our synthesis: a break in one leg, with a wide, crossed or repeated quote, that is gone at the next copy, is a data
  error. A break that persists at bid and ask, on several strikes, and that agrees with a known cost (borrow, stress), is a price.

## 3. Surveillance and alerts
- **Speed.** EU RTS 6 (Regulation 2017/589, Article 16) asks for real-time monitoring of algorithmic trading by the
  trader and by an independent risk function. "Real-time alerts shall be generated within five seconds" [S7].
- **Persistence.** A Prometheus alert stays "pending" until its condition holds for the `for` duration [S8]. The Cboe
  index filter holds a drop of x points or more for a threshold period (example: 5 minutes); it does not filter a rise [S1].
  Futures mispricing grows with maturity and is path dependent (MacKinlay, Ramaswamy) [S16].
- **False positives.** Benjamini and Hochberg control the false discovery rate over many tests. It equals the
  family-wise rate when all null hypotheses are true and is smaller otherwise, so the tests have more power [S10].
- **Escalation.** The SRE rules: "Every page should be actionable". Alert on symptoms, not causes. Show subcritical
  problems on a dashboard [S9]. RTS 6 asks staff to start remedial action when necessary [S7]. No public source found for the form of a firm's daily break report.

## 4. Standard relations for our instruments
| Relation | Statement (our notation) | Source | Note |
|---|---|---|---|
| SPX parity, box rate | `C - P = df (F - K)`; box gives `df` | [S15], [S17], N0 | Box rate to 3 years, minutely; box minus Treasury is a convenience yield [S17]. |
| SPY American bounds | `S - D - K <= C - P <= S - K df` | [S20] abstract | Standard result. `fa options check` tests the upper bound. Borrow matters [S13]. |
| Strike, calendar | monotony, convexity, calendar | N0 (Gatheral-Jacquier) | Already in `fa options check`. |
| VIX from SPX chain | Cboe method against published VIX | [S1] | F at min `|C - P|` of mids; K0 <= F; stop after two consecutive zero-bid strikes. |
| VIX not computable | K0 put or call null or crossed | [S1] | Cboe then republishes the last valid value. Our check must do the same. |
| VIX options vs VX | parity forward = VX of the same expiry | [S19] | Cboe uses "the VIX futures contract with a corresponding expiration" for margin. Settle: SOQ (VRO). |
| VX upper bound | `VX(T) <= sqrt(variance swap forward)` from SPX | [S2] | Jensen; lower bound: volatility swap forward. Deviations widen in stress [S2]. |
| SPX vs SPY | SPY forward / SPX forward stable | [S21] | SPY pays dividends quarterly (Jan, Apr, Jul, Oct); expense 0.0945%. No 1/10 ratio found in the part read. |
| Futures basis | `F = S exp((r - q) T)` inside a band | [S16] | The band widens with maturity [S16]. |
| Treasury smoothness | residual of a smooth fit | N0 (GSW), [S18] | Noise about 3 bp against a 2 bp bid/ask (working paper); it rises in crises. A liquidity measure, not an arbitrage. |
| Bills vs SOFR, EFFR | spread, not a bound | [S22] | Bills substitute for repo; +$100bn net bills, repo +1.3 bp. EFFR - IORB was -1 bp each day 2025-12 to 2026-04. |
| Fed funds futures | `100 - price` = mean EFFR of the month | [S20b] | 25 bp steps; `P(hike) = (EFFR end - EFFR start) / 25 bp`. `fa futures check` tests ZQ against EFFR (5 bp). |

## 5. Data quality
- **Crossed and locked.** The options plan of 2009 requires members "reasonably to avoid displaying Locked and Crossed
  Markets" and to reconcile them [S6]. So a crossed quote in an NBBO feed is an exception. Alpaca quotes are indicative.
- **Outliers in a price series.** Brownlees-Gallo keep a price when `|p_i - mean_i(k)| < 3 s_i(k) + gamma`. The mean is a 10%
  trimmed mean of k neighbours; gamma is a multiple of the tick. Their choice: k = 60, gamma = 0.02 [S5].
- **Quote rules.** Barndorff-Nielsen et al. (2009) delete quotes with a negative spread and quotes with a spread above 50
  times the daily median. The paper refused the agent (HTTP 403); these rules come from a secondary text [S4].
- **Coarse option filters.** CJS: 14 to 365 days, zero volume removed, implied volatility 5% to 100%, moneyness 0.7 to
  1.3, duplicates removed. Their data went from 1,976,805 quotes to 411,030 [S3].

## Sources (all read 2026-11-21)
- [S1] Cboe Global Indices, "Cboe Volatility Index Mathematics Methodology", v5.0, revised 2026-02-26. https://cdn.cboe.com/resources/indices/Cboe_Volatility_Index_Mathematics_Methodology.pdf (full text; the WebFetch tool got it).
- [S2] Van Tassel, "The Law of One Price in Equity Volatility Markets", NY Fed Staff Report 953, 2020. https://www.newyorkfed.org/medialibrary/media/research/staff_reports/sr953.pdf (full text).
- [S3] Constantinides, Jackwerth, Savov, "The Puzzle of Index Option Returns", draft 2009-11-20 (RAPS 2013). https://kops.uni-konstanz.de/server/api/core/bitstreams/d9eb4523-5c7e-45ad-9f79-7365493dd758/content (full text).
- [S4] Barndorff-Nielsen, Hansen, Lunde, Shephard, "Realised kernels in practice", Econometrics Journal 2009. https://scholar.harvard.edu/sites/scholar.harvard.edu/files/RKpractice-ECTJ.pdf (403); rules via https://arxiv.org/pdf/2108.00480.
- [S5] Brownlees, Gallo, "Financial econometric analysis at ultra-high frequency: data handling concerns", CSDA 51(4), 2006. https://labdisia.disia.unifi.it/area01/pubblicazioni_DS/wp/2006/wp2006_03.pdf (full text).
- [S6] SEC, Release 34-60405, Options Order Protection and Locked/Crossed Market Plan, 2009-07-30. https://www.govinfo.gov/content/pkg/FR-2009-08-06/pdf/E9-18763.pdf (full text).
- [S7] Commission Delegated Regulation (EU) 2017/589 (RTS 6), Article 16. https://www.legislation.gov.uk/eur/2017/589/article/16/adopted (article text).
- [S8] Prometheus, "Alerting rules", docs v3.9. https://prometheus.io/docs/prometheus/3.9/configuration/alerting_rules/ (page).
- [S9] Ewaschuk (ed. Beyer), "Monitoring Distributed Systems", Google SRE book ch. 6, 2016. https://sre.google/sre-book/monitoring-distributed-systems/ (page).
- [S10] Benjamini, Hochberg, "Controlling the false discovery rate", JRSS B 57(1), 1995. https://www.dcscience.net/Benjamini-Hochberg-1995-FDR.pdf (abstract).
- [S11] Schelter et al., "Automating large-scale data quality verification", PVLDB 11(12), 2018. https://www.vldb.org/pvldb/vol11/p1781-schelter.pdf (abstract).
- [S12] Basel Committee, "Principles for effective risk data aggregation and risk reporting" (BCBS 239), 2013. https://www.bis.org/publ/bcbs239.htm (landing page and summaries; PDF not read).
- [S13] Ofek, Richardson, Whitelaw, "Limited arbitrage and short sales restrictions", JFE 74(2), 2004. https://www.nber.org/papers/w9423 (abstract).
- [S14] Ackert, Tian, "Efficiency in index options markets and trading in stock baskets", JBF 25, 2001. https://www.frbatlanta.org/research/publications/wp/1999/05.aspx (abstract).
- [S15] Kamara, Miller, "Daily and intradaily tests of European put-call parity", JFQA 30(4), 1995. https://ideas.repec.org/a/cup/jfinqa/v30y1995i04p519-539_00.html (abstract).
- [S16] MacKinlay, Ramaswamy, "Index-futures arbitrage and the behavior of stock index futures prices", RFS 1(2), 1988. https://ideas.repec.org/p/fth/pennfi/30-87.html (working paper abstract).
- [S17] van Binsbergen, Diamond, Grotteria, "Risk-free interest rates", JFE 143(1), 2022. https://lbsresearch.london.edu/id/eprint/1821/ (abstract).
- [S18] Hu, Pan, Wang, "Noise as information for illiquidity", JF 68(6), 2013. https://www.nber.org/papers/w16468 (abstract; working paper extract).
- [S19] Cboe, "VIX Options Product Specifications". https://www.cboe.com/tradable_products/vix/vix_options/specifications (page).
- [S20] Merton, "Theory of rational option pricing", Bell Journal 4(1), 1973. https://ideas.repec.org/a/rje/bellje/v4y1973ispringp141-183.html (abstract only; the bound formula is the standard result).
- [S20b] CME Group, "Understanding the CME Group FedWatch Tool Methodology", 2023. https://www.cmegroup.com/articles/2023/understanding-the-cme-group-fedwatch-tool-methodology.html (HTTP 503; search extract only).
- [S21] SPDR S&P 500 ETF Trust, prospectus 485BPOS, 2024. https://sec.gov/Archives/edgar/data/884394/000119312524016801/d109104d485bpos.htm (first 100,000 characters).
- [S22] Anbil, Anderson, Cordes, Ruprecht, "Repo markets and the Fed's balance sheet", FEDS Notes, 2026-08-26. https://www.federalreserve.gov/econres/notes/feds-notes/repo-markets-and-the-feds-balance-sheet-implications-for-monetary-policy-implementation-20260826.html (page).

## What this means for us
1. **Copy the rule form.** Each register file gets the ticket fields plus `for` (copies a break must last), severity and
   owner [S8]. Add the "limits of use" line of N0: indicative quotes give a measurement, not a trade.
2. **Copy the Cboe quote gate before each relation.** Drop a leg with a null quote, bid > ask, or a zero bid or ask; flag
   bid = ask [S1], [S6]. A relation with a dropped leg is "not tested", not "pass". Count the dropped legs each day.
3. **Use the rate of the chain.** Take the median parity rate of pairs at moneyness 0.95 to 1.05 as the rate in the
   SPX bounds [S3]. Compare it with the Treasury fit as a separate relation, not as an input.
4. **Separate hard and soft relations.** Relations inside one chain copy (strike, calendar, parity, box) are hard.
   Relations across sources (SPX against SPY, our VIX against the published VIX, VX against VIX options) are soft. Our
   15:27 ET copy has no underlying at the same minute, so a soft break is a measurement with a time-gap field [S2].
5. **Persistence and false positives.** Call a break only after 2 consecutive daily copies (our synthesis, from [S8],
   [S16]). Report each class as a count against its own history. Use Benjamini-Hochberg when a rule needs a p-value [S10].
6. **Report, do not page.** All breaks go into the dated report (a dashboard). An alert goes out only for a symptom:
   a stale or empty copy, or a break rate far above its history [S9].
7. **Skip** real-time monitoring (RTS 6, five seconds [S7]), intraday filters (EMA of spreads [S1]), trade-level error
   rules, and arbitrage claims on Alpaca quotes. They need NBBO and intraday data that we do not have.
8. **First new relation:** the VIX from our SPX copy with the Cboe steps [S1], against the published VIX. Set its
   tolerance from the observed gaps, not before. Next: VX against the VIX option parity forward of the same expiry [S19],
   and VX below the square root of the SPX variance swap forward [S2].
