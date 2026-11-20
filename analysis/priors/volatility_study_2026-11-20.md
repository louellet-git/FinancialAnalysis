# Volatility study: the data and the plan (2026-11-20)

Ticket `T-20261120-volatility-study`. This is a study. It builds no model. It answers two questions. Is the option
data good enough to calibrate the stochastic volatility models? What do the 5-minute bars show about roughness?
Then it gives the plan as seven board tickets and a study guide.

**Words.** A "series" is one root and one expiry date (for example SPXW 2026-12-24). "k" is the log-moneyness
ln(K / F), with F the forward of the series. "Vol point" is 0.01 of implied volatility. "Live chain" is the Alpaca
indicative chain with bid and ask. "History" is the surface of daily closes from `fa options history`. "Pass" means
that a quote passes the filter of section A.0. All times are New York time (ET) unless the text says UTC.

**Data and commands.** All data commands ran on 2026-11-20 with the Alpaca key variables in the environment of the
command (never in a file). The cache was a scratch folder, not the repo. The cost was 284 Alpaca requests in this task
(34 + 62 + 50 + 10 for the SPX and VIX history, 33 for the wide SPX history of 2026-11-19, 77 for the 5-minute bars,
18 for the 1-minute bars). The live chains of the same day were made before this task (about 50 requests).

| Data | Command | Result |
|---|---|---|
| SPX, SPY, VIX live chains | `fa options snapshot SPX --source alpaca` (and SPY, VIX); made 2026-11-20 about 16:23, 15:58, 16:22 UTC | 32,046, 12,500, 1,510 quotes |
| SPX history, calm day | `fa options history SPX --date 2024-06-07 --max-days 100 --moneyness 0.92,1.08 --refresh` | 24,282 contracts listed, 3,737 with a close, 50 requests |
| SPX history, stressed day | `fa options history SPX --date 2024-08-05 --max-days 100 --moneyness 0.9,1.1` | 30,222 listed, 5,984 with a close, 62 requests |
| SPX history, recent day | `fa options history SPX --date 2026-11-19 --max-days 120 --moneyness 0.9,1.1` | 14,768 listed, 9,647 with a close, 34 requests |
| SPX history, wide strikes | `fa options history SPX --date 2026-11-19 --max-days 36 --moneyness 0.6,1.2` (second cache) | 14,286 listed, 8,692 with a close, 33 requests |
| VIX history, three dates | `fa options history VIX --date D --moneyness 0.7,1.6 --refresh` | 458 to 2,482 listed, 148 to 327 with a close |
| VIX futures | `fa futures curve VX` | worked for 2026-11-19. `--date 2024-06-07` and `--date 2024-08-05`: no Cboe file (one try each) |
| 5-minute bars | `fa prices intraday SPY --interval 5m --from 2020-07-27 --to 2026-11-19` | 123,389 rows, 77 requests |
| 1-minute bars | `fa prices intraday SPY --interval 1m --from 2025-06-01 --to 2026-11-19` | 143,853 rows, 18 requests |
| Daily files (read only) | `data/raw/trading/SPY_1d.csv` (1993 to 2026-11-18), `_VIX_1d.csv` (1990 to 2026-11-18), Cboe VIX history file | no request |

The dates. The calm day is 2024-06-07 (VIX close 12.22). The stressed day is 2024-08-05 (VIX close 38.57, high 65.73).
The recent day is 2026-11-19 (VIX close 28.44, high 29.13). The live day 2026-11-20 is in a stressed period too:
the first VIX option forward is 27.5. The Alpaca HTTP code was 200 for all requests. No 401 or 403 happened.

The scratch code (Python with numpy, no scipy) is in the scratch folder `vol/` and is not in the repo. Section 7 lists it.
It reads the JSON snapshot files and uses its own Black-76 solver. It does not use the `fa` volatilities.

## 1. Conclusion in ten lines

1. The live SPX chain is good for calibration: 25,175 quotes with bid and ask in 52 expiries from 3 days to 5 years; the median bid-ask spread of the quotes near the money is 0.1 to 0.3 vol points.
2. The live forward from put-call parity is smooth in the expiry (residual 3 index points of 7,100). The split of the carry into rate and dividend is not identified; use the carry r - q as one curve.
3. After the filter, static arbitrage in the live mids is small: calendar violations 6 of 824, none beyond the spread. Butterfly violations are 4.7% when the strikes are 0.35% apart or wider (beyond the spread); they come from quote noise of about 0.2 vol points.
4. The history of closes is not a surface of quotes. On the two active days (2024-08-05, 2026-11-19) 24% to 46% of the wide-strike tests of monotony and convexity fail beyond one tick, and 25% to 34% of the calendar tests. The calm day is better (3% to 26%). A volume filter helps but does not remove this.
5. The history cannot give a VIX within 5%: with strikes 0.9 to 1.1 the SPX chain gives 22.9 against 28.4 (2026-11-19). With strikes 0.6 to 1.2 it gives 26.9. The live chain gives 29.99 with all strikes and 24.5 with 0.9 to 1.1: the wings carry 18% of the VIX.
6. The VIX options are enough for a joint fit on the live chain (12 expiries, 1,329 quotes, strikes 10 to 200). On 2026-11-19 the parity forwards of the VIX monthly options are within 0.17 of the VX settlement for 4 of 4 monthly expiries.
7. The ATM skew term structure of the live SPX chain has a slope of -0.40 (H = 0.10) over 5 to 574 days. It is not one power law: -0.27 below 60 days, -0.45 above 20 days. The honest range is -0.25 to -0.49.
8. The 5-minute SPY realized variance gives H = 0.147 (bootstrap sd 0.017; years 0.106 to 0.173). The result moves from 0.19 (lags 1 to 5 days) to 0.07 (lags 10 to 40 days).
9. A smooth model reproduces this. An AR(1) of log volatility with the observed lag-1 correlation (0.73) gives 0.17, and a 3-factor smooth model gives 0.10. The scaling estimate alone does not show roughness. The option skew exponent and the 1-minute check support a small H, but they do not prove it either.
10. The sequence is possible with live chains from today on (steps 1 to 6). The history of closes supports only the coarse tests, and the step 5 model of Guyon and Lekeufack needs no option history (daily SPY and VIX files are enough). The seventh ticket (a daily copy of the chains) is the one that makes the history of bid and ask.

## 2. Part A: the quality of the option data

### A.0 The filters (used in all tables)

The filters are the result of the study. Section A.7 repeats them as a list.

- **Series.** Each (root, expiry) is one series. The 2026 chains list the SPX monthly (AM-settled, 9:30) and the SPXW (PM-settled, 16:00) on the same date for 6 dates (2026-11-20, 12-18, 2027-01-15, 02-19, 03-19, 04-16). The tables keep one series for each date (SPXW when both exist). The time to expiry is counted in minutes from the snapshot time to the settlement time, divided by 365 days.
- **Rate.** The Treasury par curve of the last published day (2026-11-19: 4.20% at 1 month, 4.30% mean over the expiries). The history uses the curve of its date.
- **Forward.** For each series: the median of K + (C - P) / df at the 5 strikes where |C - P| is smallest. C and P are mids (live) or closes (history).
- **Side.** Out-of-the-money only: calls for K at or above F, puts below F. The implied volatility is Black-76 on the forward of the series.
- **Live filter.** Bid above 0, ask above bid, and spread of at most 1 vol point (SPX, SPY) or 5 vol points (VIX; its lognormal volatilities are about 80%).
- **History filter.** Close above 0, volume at least 1, and 0.05 / vega at most 1 vol point (the tick of 0.05 may not move the volatility by more than one point).

### A.1 Quotes, strike range and spread, by expiry

Command that made the table: `rep1.py` (scratch). Counts of the live chains are after the removal of the series with a time to expiry under 0.5 day and
of the second root of a date. "Quotes" are the quotes with bid and ask (live) or with a close (history). "K/F" is the median range of the passing quotes.
"Spread" is the median spread of the out-of-the-money quotes with |k| at most 1.5 sigma sqrt(T), in vol points.

**SPX live, 2026-11-20 16:23 UTC (11:23 ET).** 52 series, 25,444 contracts, 25,175 quotes, 11,145 pass (44%).

| Days to expiry | Series | Contracts | Quotes | Pass | Pass % | K/F range of pass | Spread (vol points) |
|---|---|---|---|---|---|---|---|
| 0 to 7 | 3 | 1,780 | 1,682 | 616 | 37% | 0.79 to 1.10 | 0.18 |
| 7 to 30 | 16 | 8,640 | 8,509 | 3,569 | 42% | 0.63 to 1.14 | 0.23 |
| 30 to 91 | 11 | 5,234 | 5,214 | 2,417 | 46% | 0.53 to 1.17 | 0.25 |
| 91 to 365 | 15 | 7,984 | 7,966 | 3,852 | 48% | 0.41 to 1.36 | 0.14 |
| over 365 | 7 | 1,806 | 1,804 | 691 | 38% | 0.33 to 1.66 | 0.27 |

The series with more than 1,000 days to expiry are poor: the 2029, 2030 and 2031 expiries have 5 to 17 passing quotes near the money and spreads of 1 to 5 vol points.

**SPY live, 2026-11-20 15:58 UTC.** 30 series, 12,026 contracts, 11,541 quotes, 4,706 pass (41%). The options are American; Black-76 is used for the out-of-the-money side only.

| Days to expiry | Series | Quotes | Pass | K/F range of pass | Spread (vol points) |
|---|---|---|---|---|---|
| 0 to 7 | 3 | 932 | 297 (32%) | 0.81 to 1.09 | 0.22 |
| 7 to 30 | 8 | 2,885 | 1,075 (37%) | 0.71 to 1.15 | 0.20 |
| 30 to 91 | 4 | 1,793 | 755 (42%) | 0.67 to 1.29 | 0.15 |
| 91 to 365 | 10 | 3,537 | 1,623 (46%) | 0.44 to 1.37 | 0.12 |
| over 365 | 5 | 2,394 | 956 (40%) | 0.13 to 1.99 | 0.11 |

**VIX live, 2026-11-20 16:22 UTC.** 12 series (the series of 2026-11-25 has no call-put pair), 1,410 contracts, 1,329 quotes, 377 pass (28%) at 5 vol points.

| Days to expiry | Series | Quotes | Pass | Median spread (vol points) |
|---|---|---|---|---|
| 0 to 7 | 1 | 88 | 2 | 19.8 |
| 7 to 30 | 3 | 308 | 41 | 9.7 (weeklies), 3.1 (monthly 12-16) |
| 30 to 91 | 3 | 353 | 100 | 2.4 |
| 91 to 365 | 5 | 580 | 234 | 2.2 |

**SPX history of closes.** The strike range is the range that I asked for (0.9 to 1.1; 0.92 to 1.08 for the calm day). It is not a result.

| Date | Series | Contracts with a close | Pass | Pass % | Selection (days) |
|---|---|---|---|---|---|
| 2024-06-07 (calm) | 29 | 3,464 | 2,423 | 70% | 100 |
| 2024-08-05 (stressed) | 33 | 5,678 | 3,403 | 60% | 100 |
| 2026-11-19 (recent) | 33 | 7,541 | 4,509 | 60% | 120 |

Only 25% to 65% of the contracts that the list names have a close: 3,737 of 24,282 (2024-06-07), 5,984 of 30,222 (2024-08-05), 9,647 of 14,768 (2026-11-19).
There are no expiries beyond the selection in these files (100 or 120 days). The history contains no bid, no ask and no spread.

### A.2 The forward from put-call parity

For each chain I fitted ln F = a + b T over the series with at most 400 days (command `rep1.py`). The intercept is a spot estimate. The slope b is the carry r - q.
The residual is in index points. The implied q is the mean Treasury rate minus b.

| Chain | Series in fit | Intercept | Carry r - q | Mean Treasury r | Implied q | Residual rms (points) | Largest residual |
|---|---|---|---|---|---|---|---|
| SPX live | 46 | 7,081.2 | 3.96% | 4.30% | 0.35% | 3.0 | 8.5 |
| SPY live | 26 | 706.99 (IEX close 706.8) | 3.21% | 4.31% | 1.10% | 0.60 | 1.16 |
| SPX 2024-06-07 | 29 | 5,346.0 | 6.49% | 5.47% | -1.0% | 4.4 | 8.0 |
| SPX 2024-08-05 | 33 | 5,202.6 | 3.94% | 5.50% | 1.56% | 19.6 | 59.3 |
| SPX 2026-11-19 | 33 | 7,180.8 | 6.37% | 4.23% | -2.1% | 27.9 | 132.6 |

- The live forward is smooth in the expiry. The spread among the 5 parity estimates of one series is 0.9 to 12 points for the series under 2 months, 4 to 33 points from 2 months to 2 years, and 19 to 156 points for the 2029 to 2031 series.
- The implied q is not reliable. SPY gives 1.10% (a normal yield for the S&P 500; early exercise of the American puts lowers the forward, so this is a lower bound). SPX gives 0.35%. The difference of 0.75 points is not explained. Possible causes are the funding rate (the par curve is not the rate of box spreads) and quote noise. The calibration needs the carry, not q.
- The history forward is much noisier. A close of a call and a close of a put at one strike can be 4 hours apart. The calm day is acceptable (4.4 points). The two other days are not (20 and 28 points, with errors up to 133 points). The implied q of these dates (-2.1% and -1.0%) is not plausible.
- The close of 2026-11-19 gives a carry curve with one outlier of 133 points; the series of this file are not smooth.

### A.3 Static arbitrage on mids and closes

Tests (code `an.py`, function names `arb_checks` and `cal_checks`): (a) monotony and slope of the call-equivalent price C(K) (puts are converted with C = P + df (F - K)): C decreases in K, and the fall is at most df per point of strike; (b) butterfly convexity on triples of
neighbouring strikes; (c) calendar: the total variance w = sigma^2 T does not fall from one expiry to the next at the same k (k from -0.10 to 0.06 in steps of 0.01, linear interpolation in k, only where both smiles have a quote within 0.015).
A violation is any breach of more than 0.001 index points (monotony, butterfly) or any fall of w (calendar). A "beyond-spread" violation is larger than the sum of the half-spreads
of the points that it uses (live) or than the tick 0.05 for each point (history). For the calendar test, a violation counts when sqrt(w) of the later expiry is 1 vol point
(scaled to its time) below the earlier one. "Wide" means that the strikes are at least 0.35% of F apart (25 points on SPX): the test is less sensitive to the price tick.

Counts: violations / tests, with the beyond-spread count in brackets.

| Chain | Test | Raw (all quotes) | After the filter | After the filter, wide strikes |
|---|---|---|---|---|
| SPX live | monotony and slope | 1,459 / 12,401 (282) | 1,250 / 11,094 (282) | 13 / 5,025 (1) |
| | butterfly | 5,406 / 12,349 (779) | 4,948 / 11,043 (785) | 1,740 / 4,974 (235) |
| | calendar | 7 / 862 (0) | 6 / 824 (0) | |
| SPY live | monotony and slope | 597 / 5,498 (174) | 331 / 4,676 (124) | 95 / 3,297 (35) |
| | butterfly | 2,451 / 5,468 (527) | 2,012 / 4,646 (489) | 1,282 / 3,267 (283) |
| | calendar | 1 / 493 (0) | 0 / 493 (0) | |
| VIX live | monotony, butterfly, calendar | 37 / 612 (0), 204 / 600 (1), 1 / 114 (1) | | |
| SPX 2024-06-07 | monotony and slope | 630 / 2,440 (489) | 620 / 2,394 (489) | 52 / 846 (29) |
| | butterfly | 1,075 / 2,411 (806) | 1,059 / 2,365 (806) | 275 / 817 (216) |
| | calendar | 49 / 373 (15) | 48 / 362 (14) | |
| SPX 2024-08-05 | monotony and slope | 1,936 / 3,481 (1,862) | 1,891 / 3,370 (1,846) | 414 / 1,273 (408) |
| | butterfly | 1,707 / 3,448 (1,589) | 1,661 / 3,337 (1,573) | 592 / 1,240 (574) |
| | calendar | 192 / 538 (183) | 191 / 532 (182) | |
| SPX 2026-11-19 | monotony and slope | 2,444 / 4,559 (2,308) | 2,418 / 4,476 (2,302) | 306 / 1,222 (296) |
| | butterfly | 2,170 / 4,526 (1,950) | 2,129 / 4,443 (1,946) | 531 / 1,189 (502) |
| | calendar | 147 / 542 (133) | 147 / 541 (133) | |

What the table shows.

- The filter does not remove the violations. It removes the quotes with a wide spread, but a violation beyond the spread has another cause: the quotes of one snapshot are not at one time. The indicative feed gives, in one chain, quotes that differ by 0.3 to 1.3 index points from a convex curve (the first 50 violations of the series 2026-12-18 have a size of 0.01 to 1.3 points and half-spreads of 0.06 to 1.2).
- The live chain has no calendar arbitrage (0 beyond-spread cases of 824) and no monotony problem at wide strikes (1 of 5,025). The butterfly noise is 4.7% of the wide triples (235 of 4,974). The calibration must therefore use weights and a smooth model, not a test that rejects the surface.
- The history is worse in each test. Beyond-spread monotony violations at wide strikes are 3.4% (calm day) against 0.02% (live), and 24% to 32% on the two active days. Butterfly: 26% (calm), 46% (stressed), 42% (recent) against 4.7% (live). Calendar: 4% (calm), 34% (stressed), 25% (recent) against 0% (live).
- A volume filter helps the history, but slowly (recent day, butterfly beyond one tick, narrow strikes): volume at least 1: 44%; at least 10: 41%; at least 50: 37%; at least 200: 35%; at least 1,000: 27% (251 quotes left). Monotony: 51%, 38%, 28%, 20%, 6%. On the calm day the figures fall from 34% to 7% (butterfly) and from 20% to 2% (monotony) for volume 1,000 (170 quotes left).
- Reason (not measured): the SPX moves 1% in one day; a call close 4 hours old is wrong by delta times 70 points, which is more than the price difference of two strikes 5 points apart.

### A.4 The smile by expiry and the term structure of the at-the-money skew

Method (code `an.py`, function `smile_fit`): a quadratic fit of the implied volatility on k for the quotes that pass, in the window |k| at most c sigma sqrt(T) with c = 1 (floor 0.01),
with at least 6 quotes and 2 on each side. The ATM level is the constant, the skew is the slope d sigma / d k at k = 0. The table gives c = 1.

**SPX live, 2026-11-20.**

| Expiry | Days | ATM vol | Skew d sigma / d k | Standard error of the skew | Quotes in the window | Half-window |
|---|---|---|---|---|---|---|
| 2026-11-25 | 5.2 | 0.2760 | -1.452 | 0.008 | 94 | 0.033 |
| 2026-11-30 | 10.2 | 0.2445 | -1.262 | 0.006 | 118 | 0.042 |
| 2026-12-10 | 20.2 | 0.2528 | -1.129 | 0.007 | 107 | 0.059 |
| 2026-12-21 | 31.2 | 0.2391 | -0.969 | 0.005 | 74 | 0.070 |
| 2027-01-22 | 63.2 | 0.2196 | -0.744 | 0.005 | 81 | 0.092 |
| 2027-02-19 | 91.2 | 0.2139 | -0.645 | 0.003 | 221 | 0.106 |
| 2027-05-21 | 181.9 | 0.2076 | -0.469 | 0.003 | 85 | 0.147 |
| 2027-11-19 | 363.9 | 0.2033 | -0.307 | 0.001 | 120 | 0.203 |
| 2028-06-16 | 573.9 | 0.1975 | -0.246 | 0.001 | 126 | 0.249 |

The ATM level is not monotone for short expiries (24.5% to 27.6% between 3 and 14 days: the calendar of events). It falls to 19.8% at 1.6 years.

**The term structure of the skew.** The model of the rough volatility literature is |skew(T)| = A T^(H - 1/2) (Gatheral, Jaisson, Rosenbaum 2018; Fukasawa 2021). I regressed ln |skew| on ln T over all series between 5 days and 2 years.

| Chain | Series | Range (days) | Slope, c = 1 (regression se; bootstrap 5% to 95%) | H = slope + 1/2 | Slope c = 0.5 | Slope c = 1.5 | Slope, T up to 60 d | Slope, T from 20 d |
|---|---|---|---|---|---|---|---|---|
| SPX live | 46 | 5 to 574 | -0.398 (0.010; -0.421 to -0.376) | 0.10 | -0.384 | -0.412 | -0.265 | -0.446 |
| SPY live | 26 | 5 to 574 | -0.402 (0.015; -0.436 to -0.369) | 0.10 | -0.389 | -0.422 | -0.269 | -0.485 |
| SPX 2026-11-19 closes | 31 | 5 to 120 | -0.249 (0.046; -0.301 to -0.189) | 0.25 | -0.092 | -0.273 | -0.187 | -0.303 |
| SPX 2024-08-05 closes | 29 | 7 to 87 | -0.270 (0.053; -0.390 to -0.156) | 0.23 | -0.077 | -0.259 | -0.232 | -0.484 |
| SPX 2024-06-07 closes | 26 | 5 to 84 | -0.131 (0.102; -0.277 to +0.050) | 0.37 | -0.106 | -0.139 | -0.109 | -0.280 |

How noisy it is.

- The regression error (0.01) is not the real error. The residual of ln |skew| is 8%, but it is not independent between expiries. The better measure is the change of the slope with the choices of the analyst: window c (0.5 to 1.5: -0.38 to -0.41), the range of expiries (up to 60 days: -0.27; from 20 days: -0.45), the chain (SPX and SPY agree within 0.01 to 0.04). A fair statement is "-0.25 to -0.49", that is H between 0.01 and 0.25, central value 0.10.
- The curve is not a straight line in ln T. The skew falls more slowly below 60 days than above. The data therefore do not show one power law over the whole range.
- The history gives -0.13 to -0.27 with an error of 0.05 to 0.10 and a bootstrap range that includes 0 on the calm day. It is not usable for this estimate. The closes of different times and the small range (to 120 days) give a flat and noisy skew.
- The SPY skew has an American-option error and a shorter list of expiries; it is a check, not a second source.

### A.5 The SPX chain against the VIX index

The method is the official one of Cboe. The Cboe documents that I read on 2026-11-20 (text taken from the PDF with `pdftotext`):

- Cboe, "Cboe Volatility Index Methodology" (the current one): `https://cdn.cboe.com/resources/indices/Volatility_Index_Methodology_Cboe_Volatility_Index.pdf`. The universe: AM-settled SPX options and PM-settled SPXW options that expire at the end of the week, without the SPXW of the date of an AM-settled SPX. The option price is the mid of bid and ask. Once two puts (or calls) with consecutive strikes have a bid of zero, no lower (higher) strike is used. SPX expires at 9:30, SPXW at 16:00. The constant maturity is 30 days with the "30 day bracket method".
- Cboe, "Cboe Volatility Index Mathematics Methodology" (the formulas): `https://cdn.cboe.com/resources/indices/Cboe_Volatility_Index_Mathematics_Methodology.pdf` (listed by the first document; I did not read it. I used the formula of the first document: sigma^2 = (2/T) sum (dK / K^2) e^(RT) Q(K) - (1/T) (F/K0 - 1)^2, and the combination of two terms with the minutes N30 = 43,200 and N365 = 525,600).

My implementation is in `vixf.py`. The terms: the last eligible expiry up to 30 days and the first one after 30 days. The strike sets are those of the quotes that exist (for the closes there is no zero-bid rule: every strike with a close counts, and dK is the half distance of the neighbours).
It uses the Treasury curve with linear interpolation (Cboe uses a bounded cubic spline). The test of the method itself on the Cboe example was not run (the sample data of the document is a picture in the PDF).

| Date | Near term | Next term | VIX from the chain | Published VIX | Difference |
|---|---|---|---|---|---|
| 2026-11-20 live, 11:23 | SPX 2026-12-18, 27.9 d, 566 strikes (2,000 to 10,000) | SPXW 2026-12-24, 34.2 d, 198 strikes | 29.99 | not available at that time. Close of 11-19: 28.44. First VIX option forward (weekly of 11-24): 27.50 | see below |
| 2026-11-19 closes, strikes 0.9 to 1.1 | SPX 2026-12-18, 251 strikes | SPXW 12-24, 57 strikes | 22.92 | 28.44 | -5.52 |
| 2026-11-19 closes, strikes 0.6 to 1.2 | 414 strikes (4,300 to 8,500) | 76 strikes | 26.88 | 28.44 | -1.56 |
| 2024-08-05 closes, strikes 0.9 to 1.1 | SPXW 2024-08-30, 187 strikes | SPXW 09-06, 69 strikes | 24.68 | 38.57 | -13.89 |
| 2024-06-07 closes, strikes 0.92 to 1.08 | SPXW 2024-07-05, 129 strikes | SPXW 07-12, 82 strikes | 10.93 | 12.22 | -1.29 |

The effect of the strike range, measured on the live chain (same quotes, strikes cut around 7,100):

| Strikes used | all | 0.7 to 1.3 | 0.8 to 1.2 | 0.9 to 1.1 | 0.95 to 1.05 |
|---|---|---|---|---|---|
| VIX from the chain | 29.99 | 28.52 | 27.25 | 24.52 | 20.86 |

- The live chain is consistent with the VIX but I cannot close the test. The VIX at 11:23 ET is not in any file. The value 29.99 lies between the close of 11-19 (28.44) and a plausible spot (the VX settlement of 11-19 was 24.54 for December, so a spot of 28 to 30 fits a weekly forward of 27.5 in a backwardated curve). The plausible reading is that the live chain agrees with the VIX within 1 to 2 points. This is not a proof. To close the test, one more live snapshot at the close, with the VIX close of the same day, is needed. I did not make it (the market was open for the rest of this task).
- The history of closes gives a VIX that is too low by 1.3 (calm), 5.5 (recent, narrow) and 13.9 (stressed). The main cause is the strike range (the wing is cut): on the live chain, the cut to 0.9 to 1.1 alone removes 5.5 points, the same size as the miss of 2026-11-19 (5.5). With strikes 0.6 to 1.2 the miss falls to 1.6 (5%). The rest is the time mismatch of the closes and the missing strikes.
- Rule for a history that must give a VIX: keep strikes from 0.5 to 1.3 and only the expiries up to 40 days. The cost on 2026-11-19 was 33 requests.

### A.6 The VIX options and the VIX futures

**Live chain, 2026-11-20 16:22 UTC (code `vixopt.py`).** 13 expiries, 1,377 quotes with bid and ask.

| Series | Days | Contracts | Quotes | Pairs (call and put) | Strikes | Parity forward | Spread of the 5 estimates |
|---|---|---|---|---|---|---|---|
| VIXW 2026-11-24 | 3.9 | 100 | 88 | 38 | 10 to 100 | 27.500 | 0.075 |
| VIXW 2026-12-01 | 10.9 | 100 | 87 | 37 | 10 to 100 | 26.627 | 0.121 |
| VIXW 2026-12-09 | 18.9 | 100 | 90 | 40 | 10 to 100 | 25.392 | 0.112 |
| VIX 2026-12-16 | 25.9 | 140 | 131 | 61 | 10 to 200 | 25.363 | 0.144 |
| VIXW 2026-12-23 | 32.9 | 100 | 92 | 42 | 10 to 100 | 24.120 | 0.054 |
| VIX 2027-01-20 | 60.9 | 140 | 135 | 65 | 10 to 200 | 24.511 | 0.072 |
| VIX 2027-02-17 | 88.9 | 130 | 126 | 61 | 10 to 200 | 24.161 | 0.113 |
| VIX 2027-03-17 | 116.9 | 120 | 116 | 56 | 10 to 200 | 23.920 | 0.120 |
| VIX 2027-04-21 | 151.9 | 120 | 117 | 57 | 10 to 200 | 23.830 | 0.114 |
| VIX 2027-05-18 | 178.9 | 120 | 118 | 58 | 10 to 200 | 23.572 | 0.130 |
| VIX 2027-06-16 | 207.9 | 120 | 114 | 54 | 10 to 200 | 23.308 | 0.103 |
| VIX 2027-07-21 | 242.9 | 120 | 115 | 55 | 10 to 200 | 23.377 | 0.175 |

(The series VIXW 2026-11-25 has 48 quotes and no pair.) The live VIX forward curve is in backwardation from 27.5 to 23.4. The ln F fit of the 12 series has a residual of 0.71 points (the shape is not linear).
The median spread near the money is 2 to 3 vol points for the monthly options and 9 to 20 vol points for the weeklies (VIX volatilities are about 80%).

**History and futures, 2026-11-19.** The only date for which the VX settlement file exists. The weekly contracts in the file have the price of the neighbouring monthly (HIDDEN-ASSUMPTIONS #73), so only the monthly expiries are compared.

| Expiry | VX settlement | VIX option parity forward (closes) | Difference | Spread of the 5 estimates |
|---|---|---|---|---|
| 2026-12-16 | 24.5357 | 24.523 | -0.013 | 0.475 |
| 2027-01-20 | 23.989 | 23.819 | -0.170 | 0.365 |
| 2027-02-17 | 23.7349 | 23.737 | +0.002 | 0.231 |
| 2027-03-17 | 23.5396 | 23.450 | -0.090 | 0.158 |

The futures agree with the parity forwards of the options to 0.2 points (0.7%) on a stressed day (VIX 28.4), with closes only. The VX file has contracts to 2027-07-21 but the options of 2026-11-19 reach only 2027-03-17 in the history (my selection and the thin trading).
For the dates 2024-06-07 and 2024-08-05 the Cboe site gave no settlement file (the file of a past date is not kept), so the check is not possible there. The parity forward of 2024-08-05 has a spread of 0.9 to 2.5 points: it is useless on that day.

**Quote counts for a joint fit.**

| Date | VIX series | Quotes | Strikes | Pass (5 vol points) |
|---|---|---|---|---|
| 2026-11-20 live | 12 | 1,329 | 10 to 200 | 377 (28%) |
| 2026-11-19 closes | 8 | 306 | 20 to 45 | 165 |
| 2024-08-05 closes | 7 | 260 | 27 to 60 | 111 |
| 2024-06-07 closes | 5 | 121 | 10 to 19 | 36 |

The live chain is enough for a joint fit (7 monthly expiries with 2 to 3 vol points of spread, to 8 months). The history has 5 to 8 series with 20 to 40 strikes: enough for a coarse test of a fit, not for the stability study of the wings.

### A.7 Conclusion of Part A

**Which step the live chain supports.**

| Step | Live chain | History of closes |
|---|---|---|
| 1. Heston, Bates (Fourier) | Yes. 46 SPX series from 5 days to 5 years, 11,145 passing quotes. Fit by expiry, weights 1 / spread. | Only a coarse fit of the ATM level and the skew. Noise of 1 to 4 vol points. Not for parameter stability. |
| 2. SABR, local volatility | SABR for each expiry: yes (the smiles have 50 to 220 quotes). Local volatility: yes after an arbitrage-free smoothing (SVI or Andreasen-Huge); the butterfly noise of 4.7% needs it. | SABR by expiry: coarse. Local volatility: no (convexity and calendar fail on 25% to 46% of the tests). |
| 3. Two-factor Bergomi | Yes. The forward variance curve comes from the VIX strip of each expiry (strikes 2,000 to 10,000 on the SPX monthly). The ATM skew curve is clean to 2 years. | No. The VIX from closes is 5% to 36% low (A.5). |
| 4. Rough Bergomi | Yes for the skew power law on 5 to 574 days (the exponent -0.25 to -0.49, A.4) and the VIX options for the check. | No for the skew (error 0.05 to 0.10). The 5-minute bars (Part B) are the history that exists. |
| 5. Quintic OU, Guyon-Lekeufack | Quintic: yes (SPX and VIX smiles at one time). Guyon-Lekeufack: needs the daily SPX return and an at-the-money volatility history. The files `SPY_1d.csv` (1993) and `_VIX_1d.csv` (1990) are enough (the paper uses the VIX as the volatility); no option history is needed. | Not needed for the Guyon-Lekeufack fit (the daily files are enough). |
| 6. Joint SPX and VIX fit | Yes. The VIX chain has 7 monthly expiries with spreads of 2 to 3 vol points; the VX settlement files exist for the recent days only. Same-time SPX and VIX quotes are needed (A.5). | Only 5 to 8 VIX series with 20 to 40 strikes; the futures agree with the option parity to 0.2 on the one date that has a file. A coarse test only. |

**The filters that a calibration must use.**

1. Keep the root and the expiry together (series). Separate the SPX AM series from the SPXW PM series, and drop the SPX monthly on its own expiry date. Count the time in minutes from the snapshot time to the settlement time.
2. Save the time of the snapshot in the file (it is missing now). The VIX test of A.5 needs it.
3. The forward of each series: parity median over the 5 strikes with the smallest |C - P|. Then fit a smooth carry curve and drop a series whose forward is more than 10 points from the curve (the live residual is 3 points rms).
4. Out-of-the-money quotes only.
5. Live: bid above 0, ask above bid, spread at most 1 vol point (VIX options: 5 vol points). This keeps 44% of the SPX quotes. Weight the quotes with 1 / spread in volatility.
6. History: close above 0, volume at least 50 (at least 1,000 on a stressed day), and 0.05 / vega at most 1 vol point. Do not test arbitrage on closes of different times: it fails by construction.
7. Do not reject a surface for butterfly violations of the live chain. Smooth it, or fit a model that has no arbitrage by construction.
8. Cut the expiries beyond 2 years (5 to 17 quotes near the money, spread 1 to 5 vol points). Keep the strikes where the quote exists; the SPX monthly has strikes from 0.3 to 1.7 times F.
9. Request wide strikes (0.5 to 1.3) for the SPX history when a VIX must be built from it.

## 3. Part B: roughness from the 5-minute bars

### B.1 Data and method

Data: the SPY 5-minute bars from Alpaca (IEX feed), 2020-07-27 to 2026-11-19. Each day has 78 bars (the median; the minimum is 72): 1,588 days in the file, 1,575 with at least 60 bars, 1,574 in the daily file. The 1-minute bars: 2025-06-02 to 2026-11-19, 368 days, a median of 390 bars each day.

The daily realized variance RV is the sum of the squared log returns of the bars of the session (the first return is from the open of the first bar). The overnight return is not in it. The log volatility proxy is x(t) = 0.5 ln RV(t).

The estimator of Gatheral, Jaisson and Rosenbaum (2018): m(q, D) = mean of |x(t + D) - x(t)|^q. In a model with Hurst exponent H, m(q, D) is proportional to D^(q H). I regress ln m(q, D) on ln D for D = 1 to 20 days and
q in 0.5, 1, 1.5, 2, 3, then take H from zeta(q) = q H (the table gives zeta(2) / 2, and the slope through the origin of all q: they differ by at most 0.01). The standard error is the standard deviation of 300 block bootstraps (blocks of 60 days; 30 days for the 1-minute sample).
I also fitted the noise-aware form m(2, D) = a + b D^(2H) over D = 1 to 30 (a is twice the variance of the measurement noise of x; grid search for H). This is the idea of the critics; it is not their estimator.

### B.2 Results

Code: `hb.py`, `hb2.py` to `hb7.py` in the scratch folder.

| Measure of the volatility | Period | Days | H, lags 1 to 20 | Standard error | Other lags |
|---|---|---|---|---|---|
| RV from 5-minute bars (IEX) | 2020-07-27 to 2026-11-19 | 1,574 | 0.147 | 0.017 (5% to 95%: 0.134 to 0.192) | lags 1 to 5: 0.190; 2 to 10: 0.165; 5 to 30: 0.094; 10 to 40: 0.073 |
| the same, one year each | 2020 / 21 / 22 / 23 / 24 / 25 / 26 | 109 to 251 | 0.173 / 0.153 / 0.159 / 0.128 / 0.149 / 0.148 / 0.106 | not computed (the bootstrap of the full sample is 0.017) | |
| the same, noise-aware fit | 2020-07-27 to 2026-11-19 | 1,574 | 0.12 (bootstrap mean 0.141) | 0.022 | the noise sd of x is 0.034 |
| RV 1-minute | 2025-06-02 to 2026-11-19 | 368 | 0.140 (lags 1 to 10) | 0.026 | lags 1 to 5: 0.182 |
| RV 5-minute, same days, one grid | same | 368 | 0.120 | 0.025 | lags 1 to 5: 0.164 |
| RV 5-minute, average of 5 offsets | same | 368 | 0.119 | 0.024 | |
| RV 15-minute, average of 15 offsets | same | 368 | 0.108 | 0.023 | |
| RV 30-minute, average of 30 offsets | same | 368 | 0.107 | 0.025 | |
| Parkinson range estimator (daily high and low) | 1993-01-29 to 2026-11-18 | 8,510 | 0.076 | 0.008 | since 2020-07-27: 0.097 (0.013); 1993 to 2009: 0.049 |
| Garman-Klass | same | 8,510 | 0.081 | 0.009 | since 2020-07-27: 0.106; 1993 to 2009: 0.054 |
| Rogers-Satchell | same | 8,510 | 0.027 | 0.008 | since 2020-07-27: 0.066 |
| ln absolute close-to-close return | 1993 to 2026 | 8,509 | 0.007 | 0.003 | |
| ln VIX close (an implied measure) | 1990-01-02 to 2026-11-18 | 9,291 | 0.365 | 0.010 | since 2020-07-27: 0.337 |

What the table shows.

- The 5-minute estimate (0.15) and the old literature value (0.1) are close. Its sampling error is small (0.017) but the estimate depends on the lags: 0.19 for 1 to 5 days and 0.07 for 10 to 40 days. A rough process would give the same H at all lags. The lines of the log-log regression are curved.
- The 1-minute RV gives the same H as the 5-minute RV on the same days (0.140 against 0.120; the difference is smaller than the error 0.025). The RV from coarser sampling gives a slightly smaller H (0.107 at 30 minutes). So the measurement noise of RV in this sample is small. The signature plot supports this: the mean RV rises 18% from 30-minute to 1-minute sampling (annualized volatility 9.37% to 10.18%). This is a small rise: weak noise, or a negative autocorrelation of the 1-minute returns. The estimated noise of ln RV(1-minute against 5-minute average) is 0.08 in x.
- The less precise the daily estimator, the smaller H. The range estimators give 0.03 to 0.11, the absolute return 0.007 (almost white noise). This is the effect that Cont and Das describe. An estimate of H is a property of the estimator and the sample, not only of the process.
- The VIX, which is a 30-day average, gives 0.34 to 0.37. It is smoother by construction. Not comparable to the RV estimate.

### B.3 The problem of the estimate

The critique. Cont and Das (2024) show that a volatility with H = 1/2 can look rough in the realized volatility, because of the estimation error of RV. Fukasawa, Takabatake and Westphal (2022) say that the scaling method suffers from the estimation error of the latent volatility, and they propose a corrected estimator; their result is that volatility is rough, even rougher than 0.1. The two groups disagree on the data. I did not implement either corrected estimator.

My tests with simulations (code `hb5.py`, `hb6.py`; 100 paths of 1,574 days each, the same estimator).

| True process of x | Noise sd of x | Mean H estimate | Spread |
|---|---|---|---|
| fractional Brownian motion, H = 0.5 | 0 / 0.08 / 0.25 | 0.497 / 0.484 / 0.406 | 0.025 |
| H = 0.3 | 0 / 0.08 / 0.25 | 0.301 / 0.287 / 0.216 | 0.02 |
| H = 0.1 | 0 / 0.08 / 0.25 | 0.100 / 0.093 / 0.058 | 0.01 |
| AR(1), rho = 0.5 (smooth) | 0 / 0.08 | 0.081 / 0.080 | 0.01 |
| AR(1), rho = 0.7 (smooth) | 0 / 0.08 | 0.168 / 0.163 | 0.015 |
| AR(1), rho = 0.85 | 0 / 0.08 | 0.294 / 0.272 | 0.02 |
| 2-factor AR(1) (rho 0.4 and 0.97, half of the variance each) | 0.08 | 0.114 | 0.011 |
| 3-factor AR(1) (rho 0.3, 0.8, 0.98) | 0.08 | 0.097 | 0.012 |

- A white measurement noise of 0.08 (the size of the RV noise in this sample) lowers H by 0.01 to 0.02 only. It does not turn H = 0.5 into 0.15. Noise of 0.25 (the size of a range estimator) lowers it by 0.09. So the pure noise story explains the range estimators but not the 5-minute RV result.
- The observed autocorrelation of x at lag 1 is 0.73 (lags 2, 3, 5, 10, 20, 40, 60: 0.64, 0.56, 0.51, 0.40, 0.32, 0.26, 0.135). A smooth AR(1) with rho = 0.7 gives H of 0.17. A smooth multi-factor model gives 0.10 to 0.11. These are the values of the data. So the scaling regression on daily data cannot separate a rough process from a smooth process that mean-reverts fast. The scaling in D is not a power law in the data either (the curvature above).
- The daily increment of x has a variance of 0.124 (sd 0.35) while x has sd 0.48. One daily step is large against the level. This is the mark of fast mean reversion, but also of jumps in the news.

### B.4 What the IEX feed does to the realized variance

- The free plan gives the IEX exchange only. The IEX volume of SPY is 1.1% (2020) to 2.8% (2026) of the consolidated volume (median of the daily ratio with the volume of `SPY_1d.csv`). The 5-minute bars have 78 of 78 bars on the usual day, so each interval has an IEX trade. A bar close is the last IEX trade, not the last trade of the market.
- The IEX return of the day is almost the official open-to-close return (correlation 0.999, sd of the difference 0.0004). The IEX RV is 0.89 times the mean squared official open-to-close return and 0.94 times the Parkinson estimator on the full sample (by year: 0.92 to 0.96 for Parkinson). The level of RV is a little low. The correlation of ln RV(IEX) with ln Parkinson (official) is 0.69 to 0.89 by year.
- A constant factor does not change H. A factor that changes in time does. The IEX share moves by a factor of 2.6 over the sample; the ratio to Parkinson is stable (0.92 to 0.96). I take the effect on H as small, but I did not test it with the consolidated bars (Yahoo refused earlier, #90).
- The 1-minute and 5-minute results agree. This does not prove that the IEX sampling is harmless: both are IEX. A different source of bars (consolidated) is needed for that.

### B.5 What the data can show and what it cannot

- It shows that the daily realized variance of SPY has short-lag increments that scale with an exponent near 0.15, with a small sampling error, in each year of the sample, and with the 1-minute and 5-minute data.
- It shows that the option skew term structure, which is independent data, has a slope of -0.25 to -0.49 over 5 to 574 days, i.e. H between 0.01 and 0.25 under the rough model.
- It does not show that the process is rough. A smooth two- or three-factor model gives the same estimate (0.10 to 0.17) with the same data. The estimate changes with the lag range (0.19 to 0.07). The corrected estimators of the literature disagree on the answer.
- It does not give a precise H. A fair range is 0.07 to 0.19 (lags), and 0.05 to 0.25 including the option skew.
- It does not give the overnight part and does not give the true spot volatility. Daily data of 6.3 years has only one regime of stress (2022, 2025 spring, 2026).
- Consequence for the plan. The rough models (steps 4 to 6) fit the SPX smile and are worth the test. The choice between a rough and a smooth model must be made on the smile and the VIX, not on the RV scaling. Ticket 4 includes the comparison with a smooth two-factor model.

## 4. Study guide

All entries were checked by a web search on 2026-11-20 (title, authors, year, journal, pages as the search result gave them). I read the full text of none of them, except the parts that I name in sections A.5 and the Heston values (Chan 2018). The Wilmott and Risk papers are behind a paywall: for these the page numbers come from a catalogue record. Where a detail was not confirmed it says so.

You know the SOA INV-201 level: Black-Scholes, Ito, risk-neutral pricing, short-rate models, Heston basics. The prerequisites below are what is missing.

**Step 1. Heston and Bates with Fourier pricing.** Prerequisites: the characteristic function of a log price, the Riccati equations of an affine model, the Fourier inversion formulas, the branch problem of the complex logarithm.

1. S. L. Heston (1993), "A closed-form solution for options with stochastic volatility with applications to bond and currency options", Review of Financial Studies 6(2), 327-343. The model and the pricing formula.
2. H. Albrecher, P. Mayer, W. Schoutens, J. Tistaert (2007), "The little Heston trap", Wilmott 2007(1), 83-92. The stable form of the characteristic function. And R. Lord, C. Kahl (2010), "Complex logarithms in Heston-like models", Mathematical Finance (volume and pages not confirmed by my search; a 2006 Tinbergen working paper TI 2006-065 exists).
3. P. Carr, D. Madan (1999), "Option valuation using the fast Fourier transform", Journal of Computational Finance 2(4), 61-73. And D. S. Bates (1996), "Jumps and stochastic volatility: exchange rate processes implicit in Deutsche Mark options", Review of Financial Studies 9(1), 69-107 (the jumps of the Bates model). Book: J. Gatheral (2006), "The Volatility Surface: A Practitioner's Guide", Wiley, chapters 1 to 3 (stochastic and local volatility, Heston, the implied volatility surface; the chapter list is from the Wiley catalogue).

**Step 2. SABR and local volatility.** Prerequisites: Dupire's equation, the implied volatility as an expansion in the expiry, arbitrage-free interpolation of a smile.

1. P. S. Hagan, D. Kumar, A. S. Lesniewski, D. E. Woodward (2002), "Managing smile risk", Wilmott Magazine, July 2002, 84-108. The SABR model and its formula. And J. Oblój (2008), "Fine-tune your smile: correction to Hagan et al.", arXiv:0708.0998.
2. B. Dupire (1994), "Pricing with a smile", Risk 7(1) (the volume is from a Wikipedia citation of one source; the Risk page confirms the title and January 1994).
3. J. Gatheral, A. Jacquier (2014), "Arbitrage-free SVI volatility surfaces", Quantitative Finance 14(1), 59-71. And J. Andreasen, B. Huge (2011), "Volatility interpolation", Risk 24(3), 76-80 (the issue and pages are from one catalogue record).

**Step 3. Two-factor Bergomi (forward variance).** Prerequisites: variance swaps and the forward variance curve, the log-contract replication (the VIX formula), Gaussian factors and their moments.

1. L. Bergomi (2005), "Smile dynamics II", Risk 18 (October 2005), 67-73. The two-factor model.
2. L. Bergomi (2016), "Stochastic Volatility Modeling", Chapman and Hall/CRC. The chapters on forward variance models, the two-factor model and the calibration to VIX futures and options (the chapter list is from the publisher page).
3. The Cboe VIX methodology documents (section A.5) for the strip formula.

**Step 4. Rough Bergomi, simulation, and the roughness estimate.** Prerequisites: fractional Brownian motion and the Volterra kernel, the Hurst exponent, the hybrid simulation scheme, Monte Carlo variance reduction.

1. J. Gatheral, T. Jaisson, M. Rosenbaum (2018), "Volatility is rough", Quantitative Finance 18(6), 933-949 (arXiv:1410.3394). The estimate of H from the moments of the increments.
2. C. Bayer, P. Friz, J. Gatheral (2016), "Pricing under rough volatility", Quantitative Finance 16(6), 887-904. The rough Bergomi model.
3. M. Bennedsen, A. Lunde, M. S. Pakkanen (2017), "Hybrid scheme for Brownian semistationary processes", Finance and Stochastics 21(4), 931-965 (arXiv:1507.03004). And R. McCrickerd, M. S. Pakkanen, "Turbocharging Monte Carlo pricing for the rough Bergomi model" (arXiv:1708.02563; the journal is not confirmed by my search). For the critique: R. Cont, P. Das (2024), "Rough volatility: fact or artefact?", Sankhya B 86(1), 191-223 (arXiv:2203.13820); M. Fukasawa, T. Takabatake, R. Westphal (2022), "Consistent estimation for fractional stochastic volatility model under high-frequency asymptotics", Mathematical Finance 32(4), 1086-1132 (arXiv:1905.04852, first titled "Is volatility rough?"); M. Fukasawa (2021), "Volatility has to be rough", Quantitative Finance 21(1), 1-8.

**Step 5. The quintic Ornstein-Uhlenbeck model and the path-dependent volatility of Guyon and Lekeufack.** Prerequisites: polynomial functions of a Gaussian process and their moments, the VIX as the square root of a conditional expectation, the linear regression of volatility on past returns.

1. E. Abi Jaber, C. Illand, S. Li (2023), "The quintic Ornstein-Uhlenbeck volatility model that jointly calibrates SPX and VIX smiles", Risk (arXiv:2212.10917; the date of the Risk issue is from a search summary).
2. J. Guyon, J. Lekeufack (2023), "Volatility is (mostly) path-dependent", Quantitative Finance 23(9), 1221-1258.
3. Background for the existence and positivity of the Guyon-Lekeufack model: R. Andres, B. Jourdain (arXiv:2408.02477, as listed by the search). I did not read it.

**Step 6. The joint fit to SPX and VIX smiles.** Prerequisites: the VIX as a function of the SPX path, the joint calibration problem, the fit measure in vol points.

1. J. Guyon (2020), "The joint S&P 500/VIX smile calibration puzzle solved", Risk, April 2020 (SSRN record).
2. J. Gatheral, P. Jusselin, M. Rosenbaum (2020), "The quadratic rough Heston model and the joint S&P 500/VIX smile calibration problem", Risk (arXiv:2001.01789).
3. A. Jacquier, C. Martini, A. Muguruza (2018), "On VIX futures in the rough Bergomi model", Quantitative Finance 18(1), 45-61. And S. E. Rømer (2022), "Empirical analysis of rough and classical stochastic volatility models to the SPX and VIX markets", Quantitative Finance 22(10), 1805-1838. Rømer reports that classical two-factor models fit the joint SPX and VIX data well without roughness: it is the comparison that ticket 6 makes.

**Reference values for the tests.** The Heston call price of the test in ticket 1 (S = 100, K = 100, r = q = 0, v0 = 0.0175, theta = 0.0398, kappa = 1.5768, vol of variance 0.5751, rho = -0.5711): 5.785155450 for T = 1 and 22.318945791 for T = 10.
Source: T. L. Chan, "Singular Fourier-Pade series expansion of European option prices", arXiv:1706.06709v3 (November 2017; the PDF says "To appear in Quantitative Finance"; the published volume was not checked), equation 105 and appendix B. I read these numbers in the PDF.

## 5. Defects, doubts and what was not done

- **Defect (not corrected): roots are mixed.** `fill_from_mids` (`rust/src/market/option_history.rs`) groups the quotes of a chain by the expiry date only. In the 2026 chain 6 dates have two roots (SPX AM and SPXW PM), and the quotes of the second root replace the first in the same strike slot. The forward and the volatility of the snapshot for those dates mix two settlement times. The same code is used for the history. The study did not use the snapshot volatilities; it grouped by (root, expiry) in its own code. Ticket 7 includes the correction (group by root; a test with two roots on one date). I did not correct it because it does not block the commands.
- **Defect to know: no snapshot time.** The snapshot has the date only. The study used the time of the file. The VIX test (A.5) and the time to expiry need the time.
- **The rate.** The par Treasury curve is used as a continuously compounded rate. The carry r - q of the chains shows that it is not the rate of the options (A.2). I did not test the SOFR curve (`fa rates nyfed`).
- **The VIX test of the live chain is open** (A.5). One more snapshot at the close with the VIX close would close it. The market was open for the rest of the task.
- **The VIX method is my reading of the Cboe document.** The 30-day bracket detail (the last expiry up to 30 days and the first after) and the days that count as end of week (Friday, or Thursday before a closed Friday) are my reading. The check on the official sample data was not run.
- **The tolerance of the arbitrage tests is my choice** (0.001 index points; the tick of 0.05; 1 vol point). The counts depend on it. The order of the results does not.
- **Part B uses one symbol and one source.** The IEX effect on the roughness is not tested against consolidated bars. The corrected estimators of Fukasawa and co-authors and of Cont and Das were not implemented. The "noise-aware" fit is my simple version.
- **The 400 request limit.** The task used 284 requests, and the earlier live chains about 50. No 401 or 403 occurred. One Cboe request for each past VX date was refused by the missing file, not by the host.
- **Not done:** SPY history; NDX, RUT or other index chains; any model; the VIX futures history of single contracts (`fa futures history` was not tried).

## 6. The tickets

The seven tickets are in `PLANNING/board/T-20261120-vol-<n>-<slug>.md` (status `backlog`):

| File | Title |
|---|---|
| `T-20261120-vol-1-heston-bates.md` | Price and fit the Heston and Bates models with Fourier methods |
| `T-20261120-vol-2-sabr-localvol.md` | Fit SABR smiles and a local volatility surface |
| `T-20261120-vol-3-bergomi-2f.md` | Fit a two-factor Bergomi forward variance model |
| `T-20261120-vol-4-rough-bergomi.md` | Simulate rough Bergomi and estimate the roughness |
| `T-20261120-vol-5-quintic-pdv.md` | Fit the quintic OU model and the path-dependent volatility model |
| `T-20261120-vol-6-joint-spx-vix.md` | Fit SPX and VIX smiles together and compare the models |
| `T-20261120-vol-7-chain-copy.md` | Copy the option chains each day to keep bid and ask |

## 7. Scratch code (not in the repo)

Folder `vol/` of the scratchpad of this session. `vc.py` (load, Black-76, forward, treasury), `an.py` (filter, arbitrage tests, smile fit), `rep1.py` (tables A.1 to A.4; `python3 -I rep1.py SPX_live`),
`vixf.py` (the VIX formula), `vixtrunc.py`, `vixwide.py`, `vixopt.py` (A.6), `volmin.py` (volume filter), `hb.py` and `hb1.py` to `hb7.py` (Part B), `PROGRESS.md`.
A person who wants the numbers again needs the same Alpaca key, the commands of the first table, and these files.
