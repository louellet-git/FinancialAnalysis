# History: market falls led by the leaders of a capex boom while their spend still rises

**Main finding.** The rule keeps 3 episodes (1845, 2000, 2022). In all 3, the spend still rose in the year that the shares of the leaders turned, and the fall continued.
The 2 older falls became deep after the spend turned down; in 2022 the leaders recovered in 3 years. Three cases are a story, not a base rate.

**Question (2026-11-21).** From 2026-10-06 to 2026-11-19 SPY fell 8.11%. NVDA, ORCL, GOOGL and META led the fall, but their reported capex still rose (`2026-11-21-conditions-and-the-oct-nov-drawdown.md`). Which past episodes are like this, and what came after them? Context only: not a forecast, a model input or a trade signal. Store: `louellet-git/fa-data`, commit 1804f48.

## 1. Selection rule

Order of the work: the rule and the candidates come from the first version of this note; this revision applies the rule with evidence. An episode is kept only when all three conditions are true:

1. **New technology, large spend.** Investment in a technology less than about 30 years old was large for the economy and grew fast in the years before.
2. **Led by the boom.** The firms that did or supplied the spend fell more than the broad market at the start of the fall.
3. **Spend still rose.** At the start of the fall, the spend of these firms was higher than one year before (published at that time, or a later series for that year).

| Candidate | 1 | 2 | 3 | Result |
|---|---|---|---|---|
| British railways, 1845 | Yes: industry began 1830 [S1]; construction 5.7% of GDP in 1846 (Mitchell 1964, quoted in [S1]) | Yes: 1845-08-08 to end Nov, established railways -18.4%, non-railways -4.3% [S2] | Yes, proxy: paid-up capital of new railways £3.7m (end 1844), £15.6m (end 1845) [S3] | Kept |
| US railroads, 1873 | - | - | No: track added 7,379 mi (1871), 5,870 (1872), 4,097 (1873) [S4] | Out |
| US railroads, 1893 | No: about 60 years old | - | No: 12,876 mi (1887), 4,441 (1892) [S4] | Out |
| Electricity, 1929 | **No**: central stations from 1879-1882 [S5], so about 47 years old | Yes: end Sep to end Nov 1929, utilities -55%, industrials -48% [S6] | Yes: utility construction $2,395m (1928), $2,687m (1929), 1947-49 dollars [S7] | Out |
| Nifty Fifty, 1973 | No: about 50 large "one-decision" growth stocks, P/E 50 or more common [S8]; not a technology spend | - | - | Out |
| Telecom, 2000 | Yes: commercial internet from 1989-1995 [S9]; communication-equipment investment $70.8bn (1996) to $98.7bn (1999), 1.0% of GDP [S10] | Yes: Lucent -75% (Mar 2000 to Feb 2001) [S11]; S&P 500 at most -49.7% in its whole fall (1,527.46 to 768.83) [S12] | Yes: $98.7bn (1999), $123.3bn (2000) [S10]; carrier capex $97bn (2000) [S13] | Kept |
| Growth/cloud, 2022 | Yes: capex of META, GOOGL, AMZN, MSFT about $69bn (2019) to $125bn (2021), 0.5% of GDP [store, S10] | Yes: 2022-01-03 to 01-21, SPY -8.3%; META -10.4%, GOOGL -10.1%, MSFT -11.6%, AMZN -16.3%, NVDA -22.4% [store] | Yes: all four higher in 2022 (row "Spend" below) | Kept |

## 2. Episodes (Analogy, not a forecast)

| | Railways 1845 | Telecom 2000 | Growth 2022 |
|---|---|---|---|
| Dates | Share peak Aug 1845; trough Apr 1850 [S1] | Nasdaq Composite peak 5,048.62 (2000-03-10); intraday low 1,108.49 (2002-10-10) [S12] | SPY 477.71 (2022-01-03) to 356.56 (2022-10-12), -25.4% [store] |
| Spend | 5.7 / 6.7 / 4.7% of GDP in 1846 / 1847 / 1848 [S1] | Comm. equipment $123.3bn (2000), $109.1bn (2001), $83.7bn (2002) [S10]; carrier capex $97bn (2000), $77bn (2001) [S13] | USD bn, 2021 to 2022: META 18.57 to 31.43; GOOGL 24.64 to 31.48; AMZN 61.05 to 63.65; MSFT (FY Jun) 20.62 to 23.89 [store] |
| Returns of spenders | Dividends of old lines 4.4% of par (1843), 7.0% (1847), 2.4% (1852) [S1] | WorldCom: $3.8bn false entries, found Jun 2002; Chapter 11 on 2002-07-21 [S14] | Capex/OCF 2022: GOOGL 0.34, MSFT 0.27, META 0.62, AMZN 1.36 [store] |
| Funding | Partly-paid shares; "calls" for capital in the downturn [S1, S3] | Carrier investment "mostly financed with debt"; industry "owed a trillion dollars" (2002) [S15] | Own cash, except AMZN (ratio above 1) |
| Rates | Bank rate up Oct 1845 [S16]; 5.5% by Aug 1847 [S1] | GSW 10Y 6.55% (2000-03-10) | GSW 10Y 1.72% (01-03) to 3.97% (10-12): a rate shock, not a rule condition, the main difference |
| Stress at "-8%" day | n/a | 2000-04-14: VIX 33.49, Baa-10Y 2.53 pp | 2022-01-21: VIX 28.85, Baa-10Y 1.84 pp |
| After: leaders | Railways -18.2% Aug-Nov 1845; -57.5% to Apr 1850 [S1] | Cisco -52%, Nortel -56% by Feb 2001 [S11]; Cisco lost 80% in the fall [S12] | From 2022-01-21: 1y (to 2023-01-23) META -52.7%, NVDA -17.9%, AMZN -31.6%, GOOGL -23.4%, MSFT -18.1%; 3y +103%, +503%, +62%, +52%, +45% [store] |
| After: market | Non-railways 1,158 (Aug 1845) to 1,126 (1850), -2.8% [S2] | S&P 500 from year-end 1999: 1y -10.1%, 3y -40.1%, 5y -17.5% [S17] | SPY from 437.98 (2022-01-21): 1y -8.5%, 3y +37.7%, to 2026-11-19 +63.5% [store] |

Start points and index types differ: 1845 uses cap-weighted capital-gain indices [S2]; 2000 starts at year-end 1999, before the peak, with the S&P 500 price index; 2022 starts at the "-8%" day (first SPY close 8% or more below its running peak), with SPY closes. No return includes dividends.

Present (store): VIX 28.44 (2026-11-19), Baa-10Y 1.52 pp (11-18), GSW 10Y 5.135% (11-13). Capex/OCF (SEC company facts; the windows differ): one quarter for MSFT (Jul-Sep 2026, 0.68) and ORCL (Jun-Aug 2026, 1.23); nine months (Jan-Sep 2026) for META 0.86, GOOGL 1.04, AMZN 1.54.

## 3. What the set says

1. In 3 of 3 kept episodes, the spend rose in the year of the turn (1845 to 1847; 2000; 2022). The rise did not stop the fall.
2. The spend then fell: railways after 1847; telecom in 2001 ($123.3bn to $109.1bn, BEA; $97bn to $77bn, RHK). After 2022, only META and AMZN cut in 2023 (META 31.43 to 27.27; AMZN 63.65 to 52.73, store).
3. In 2 of 3 (1845, 2000), the trough came after the spend turned down, and the spend used external funding: calls on partly-paid shares, carrier debt.
4. In 2022, three of four spenders funded capex from operating cash. The leaders fell more than SPY for one year, then rose more.
5. The present: profits like 2022; capex/OCF above 1 at AMZN, GOOGL and ORCL. Inference: this is nearer to the external funding of 1845 and 2000. The 10Y (5.1%) is between 2000 and 2022. The Baa-10Y (1.52 pp) is below both "-8%" days.

## 4. Limits

- Three episodes; none is the same as the present.
- The 30-year limit decides 1929. With a 50-year limit, 1929 is kept: from year-end 1929, DJUA (88.27) -68.6% in 3 years; Dow (248.48) -75.9% [S18].
- Hindsight: the Mitchell, BEA and RHK series came after each start; the 1845 and 2000 peaks were known only later. Paid-up capital is a proxy for spend.
- Survivorship: US and UK only. The 2022 names exist; WorldCom does not.

## 5. Sources (all opened 2026-11-21)

Store: `<TICKER>/prices.json`, `<TICKER>/company_facts.json.gz`, `vix/VIX_History.csv`, `bonds/fred_BAA10Y.csv`, `rates/gsw_feds200628.csv`; FRED copies in `data/raw/history/`.
[S1] https://pureadmin.qub.ac.uk/ws/files/18161511/RailwayCrisis.pdf (Campbell 2014, pp. 3-5, 9, 19)
[S2] https://mpra.ub.uni-muenchen.de/61115/1/MPRA_paper_61115.pdf (Campbell 2012)
[S3] https://mpra.ub.uni-muenchen.de/29839/1/MPRA_paper_29839.pdf (Campbell 2010, Table 4)
[S4] https://fred.stlouisfed.org/data/A02082USA374NNBR (local file, not in the store; requested from the Head of Technology, 2026-11-21)
[S5] https://en.wikipedia.org/wiki/Pearl_Street_Station
[S6] https://eh.net/encyclopedia/the-1929-stock-market-crash/ (Bierman)
[S7] https://fred.stlouisfed.org/data/A0292AUSA380NNBR (local file, not in the store; requested from the Head of Technology, 2026-11-21)
[S8] https://en.wikipedia.org/wiki/Nifty_Fifty
[S9] https://en.wikipedia.org/wiki/History_of_the_Internet
[S10] https://fred.stlouisfed.org/graph/fredgraph.csv?id=C275RC1A027NBEA ; https://fred.stlouisfed.org/graph/fredgraph.csv?id=GDPA (local file, not in the store; requested from the Head of Technology, 2026-11-21)
[S11] https://knowledge.wharton.upenn.edu/article/a-stumbling-lucent-tries-to-get-back-on-track/ (2001-02-22)
[S12] https://en.wikipedia.org/wiki/Closing_milestones_of_the_S%26P_500 ; https://en.wikipedia.org/wiki/Closing_milestones_of_the_Nasdaq_Composite ; https://en.wikipedia.org/wiki/Dot-com_bubble
[S13] https://lightwaveonline.com/business/earnings-statements/article/16648149/surviving-the-crunch-in-capital-expenditures (RHK)
[S14] https://en.wikipedia.org/wiki/MCI_Inc.
[S15] https://en.wikipedia.org/wiki/Telecoms_crash
[S16] https://libertystreeteconomics.newyorkfed.org/2015/06/crisis-chronicles-railway-mania-the-hungry-forties-and-the-commercial-crisis-of-1847
[S17] https://en.wikipedia.org/wiki/S%26P_500 (annual returns)
[S18] https://en.wikipedia.org/wiki/Dow_Jones_Utility_Average ; https://en.wikipedia.org/wiki/Dow_Jones_Industrial_Average
