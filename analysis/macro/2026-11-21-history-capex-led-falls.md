# History: market falls led by the leaders of a capex boom while their spend still rises

**Question (2026-11-21).** From 2026-10-06 to 2026-11-19 SPY fell 8.11%. NVDA, ORCL, GOOGL and META led the fall, but their reported capex still rose (`2026-11-21-conditions-and-the-oct-nov-drawdown.md`). Which past episodes are like this, and what came after them?

Context only: not a forecast, a model input or a trade signal. Data: store `louellet-git/fa-data`, commit 1804f48.

## 1. Selection rule (written before I read the outcomes)

An episode is kept when all three conditions are true:

1. **New technology, large spend.** Investment in a technology less than about 30 years old was large for the economy and grew fast in the years before.
2. **Led by the boom.** The firms that did or supplied the spend fell more than the broad market at the start of the fall.
3. **Spend still rose.** At the start of the fall, the spend of these firms was still higher than one year before (a fact that was published at that time, or a later series for that year).

| Candidate | Result | Reason |
|---|---|---|
| British railway mania, 1845-1847 | Kept | Shares peaked Aug 1845; construction rose to 6.7% of GDP in 1847 |
| US railroads, 1873 | Out (fails 3) | Track added peaked in 1871 (7,379 mi), fell in 1872 (5,870) and 1873 (4,097): FRED `A02082USA374NNBR` |
| US railroads, 1893 | Out (fails 1, 3) | Technology about 60 years old; track added fell from 12,876 mi (1887) to 4,441 (1892): same series |
| Electricity and radio, 1929 | Kept (partial) | Utilities led the October 1929 fall; the 1930 build plan was larger than 1929. The fall became broad at once |
| Nifty Fifty, 1972-1974 | Out (fails 1, 2) | A valuation group of large firms, not a capex boom of a new technology |
| Telecom and internet, 2000-2002 | Kept | Carrier capex peaked in 2000; equipment makers led the fall |
| Growth stocks, 2022 | Kept (partial) | Leaders' capex rose in 2022, but the main cause at the time was a rate shock, not doubt about the spend |

## 2. Episodes

| | Railway mania | 1929 utilities and radio | 2000-2002 telecom | 2022 growth |
|---|---|---|---|---|
| Dates | Share peak Aug 1845; trough Apr 1850 | Dow peak 1929-09-03 (381.17); low 1932-07-08 (41.22) | Nasdaq peak 2000-03-10 (5,048.62); low 2002-10-09 (about 1,114) | SPY peak 2022-01-03 (477.71); trough 2022-10-12 (356.56, -25.4%) |
| Spend still rising | Construction 5.7% / 6.7% / 4.7% of GDP in 1846 / 1847 / 1848 | Utility plan for 1930: $1.4 bn, +8.5% on 1929 (White House, 1930-02-28) | Carrier capex $97 bn in 2000, $77 bn in 2001, $46-51 bn forecast for 2002 (RHK) | Capex 2021 to 2022, USD bn: META 18.57 to 31.43; GOOGL 24.64 to 31.49; AMZN 61.05 to 63.65; MSFT (FY Jun) 20.62 to 23.89 |
| Profits of spenders | Dividends of old lines 4.4% of par (1843) to 7.0% (1847), then 2.4% (1852) | Regulated return; regulators in NY, MA and the FTC reviewed rates in Oct 1929 | Many new carriers lost money (memory, not checked); WorldCom overstated 2001-02 earnings by $3.8 bn (known only in Jun 2002) | Large positive profits (META, GOOGL, MSFT) |
| Funding, leverage | Partly-paid shares; "calls" for capital in the downturn | Holding companies with debt and preferred stock; broker loans above $8.5 bn | Debt of new carriers (memory, not checked) | Mostly own cash (memory, not checked) |
| Rates | Bank rate up Oct 1845, Apr and Aug 1847 | Gov. bonds 3.4%, IG industrial bonds 5.1% | GSW 10Y 6.55% (2000-03-10) | GSW 10Y 1.72% (2022-01-03) to 3.97% (2022-10-12) |
| Stress at "-8%" day | n/a | n/a | 2000-04-14: VIX 33.49, Baa-10Y 2.53 pp, GSW 10Y 6.07% | 2022-01-21: VIX 28.85, Baa-10Y 1.84 pp, GSW 10Y 1.84% |
| Valuation | No index | S&P CAPE about 30 (memory, not checked) | S&P CAPE about 44 (memory, not checked) | not computed |
| What came after: leaders | Railway shares -18.2% Aug-Nov 1845; -57.5% peak to trough (Apr 1850) | DJUA year-end 1929 = 88.27: 1y -31.1%, 3y -68.6%, 5y -79.8% | Cisco -87.1% (2000-03-27 to 2002-10-11); passed its 2000 high only in Dec 2025. WorldCom bankrupt 2002-07-21 | From 2022-01-21: 1y META -52.7%, NVDA -17.9%, AMZN -31.6%, GOOGL -23.5%, MSFT -18.1%; 3y META +103%, NVDA +503%, AMZN +62%, GOOGL +52%, MSFT +45% |
| What came after: broad index | No broad index | Dow from year-end 1929 (248.48): 1y -33.8%, 3y -75.9%, 5y -58.1% | S&P 500 price return from year-end 1999: 1y -10.1%, 3y -40.1%, 5y -17.5% | SPY from 2022-01-21 (437.98): 1y -8.5%, 3y +37.7%, to 2026-11-19 +63.5% (5y not yet) |

The present, for comparison (store): VIX 28.44 (2026-11-19), Baa-10Y 1.52 pp (11-18), GSW 10Y 5.135% (11-13). Capex divided by operating cash flow, SEC company facts: MSFT 0.68 (Jul-Sep 2026), META 0.86 and GOOGL 1.04 (Jan-Sep 2026), AMZN 1.54 (Jan-Sep 2026), ORCL 1.23 (Jun-Aug 2026). A ratio above 1 means the capex needs cash from outside operations.

## 3. What the set says

1. In three of the four episodes, spend kept rising for one to two years after the share prices of the leaders turned (1846-1847, 1930, 2000). A rising spend did not stop the fall.
2. In the three older episodes, the turn of the spend came later and the large losses of the leaders came after that turn (railways after 1847, utilities after 1930, telecom in 2001-2002).
3. The worst cases (1845, 1929, 2000) had external funding with leverage: calls on partly-paid shares, holding-company debt, carrier debt. The spenders of 1845 and 2000 had low or falling returns on the new capital.
4. In 2022 the spenders had large profits and funded the spend mostly from cash. The leaders fell more than the index for one year, then rose more. The cut of META capex in 2023 (31.43 to 27.27, SEC company facts) came at the same time as the rise. This is one case only.
5. The present has a mix. The level of profits is like 2022. The capex of AMZN, GOOGL and ORCL is above their operating cash flow, which is nearer to the external funding of the older episodes. The 10Y rate (GSW 5.1%) is between 2000 and 2022. The Baa-10Y spread (1.52 pp) is lower than at the "-8%" day of 2000 (2.53) and 2022 (1.84).

## 4. Limits

- Four episodes, two of them partial. This is a story, not a base rate. No episode is the same as the present.
- Hindsight: the GDP shares of 1846-1848 (Mitchell, 1964) and the RHK capex series (2002) were not known at the start of each fall. The WorldCom fraud was known only in 2002. The "-8%" day for 2022 avoids the use of the later peak; the older episodes use peaks or year-ends, which a person knows only later.
- Survivorship: only US and UK markets. Cisco and the 2022 names still exist; WorldCom, Lucent and most railway companies of 1845 do not. The store has CSCO prices only from 2016, so the Cisco numbers come from a secondary source.
- All returns are price returns (no dividends). 1845 has no broad index; VIX starts in 1990.

## 5. Sources

- Store files: `SPY/prices.json`, `META/prices.json`, `NVDA/prices.json`, `AMZN/prices.json`, `GOOGL/prices.json`, `MSFT/prices.json`; `vix/VIX_History.csv`; `bonds/fred_BAA10Y.csv`; `rates/gsw_feds200628.csv` (SVENY10); `<TICKER>/company_facts.json.gz`. The "-8%" day is the first SPY close 8% or more below its running peak.
- G. Campbell (2014), "Government Policy during the British Railway Mania and the 1847 Commercial Crisis", in *British Financial Crises since 1825*, OUP: https://pureadmin.qub.ac.uk/ws/files/18161511/RailwayCrisis.pdf
- NY Fed, Liberty Street Economics (2015-06): https://libertystreeteconomics.newyorkfed.org/2015/06/crisis-chronicles-railway-mania-the-hungry-forties-and-the-commercial-crisis-of-1847
- FRED, NBER series `A02082USA374NNBR`: https://fred.stlouisfed.org/data/A02082USA374NNBR
- H. Bierman, "The 1929 Stock Market Crash", EH.net: https://eh.net/encyclopedia/the-1929-stock-market-crash/
- Hoover news conference, 1930-02-28: https://www.presidency.ucsb.edu/documents/the-presidents-news-conference-701
- Wikipedia (secondary): Dow Jones Utility Average, DJIA, S&P 500 (annual returns), Wall Street crash of 1929.
- Lightwave, 2002-05-01 (RHK figures): https://lightwaveonline.com/business/earnings-statements/article/16648149/surviving-the-crunch-in-capital-expenditures
- Cisco: https://www.nasdaq.com/articles/25-years-later-cisco-finally-recovers-dot-com-crash ; https://finance.yahoo.com/news/cisco-stock-still-isnt-back-to-its-2000-high-chart-of-the-week-123018268.html
- WorldCom: https://money.cnn.com/2002/07/19/news/worldcom_bankruptcy/index.htm ; https://abcnews.go.com/amp/Business/worldcom-admits-38-billion-accounting-trick/story?id=87084
- Nasdaq peak and low: https://kiplinger.com/article/investing/t058-c016-s002-3-lessons-for-investors-from-the-tech-bubble.html
