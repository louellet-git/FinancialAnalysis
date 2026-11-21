# NVDA: trade recommendation (fundamentals)

**Recommendation: no trade.** Instrument: NVDA common stock. Horizon: none. Account: paper (no live
trade unless Luc says live). Date: 2026-11-21. Owner of the sleeve: the Head of Fundamental Research.
Re-examine at the 10-Q of Q3 FY2027 (due by 2026-12-04) and at the Q4 FY2027 release (store
estimate 2027-02-24; `earnings_dates.json`, "report date of a year ago + 364 days").

Evidence: `analysis/NVDA/REPORT-DRAFT.md` at commit `baf6358` (four-step review complete; auditor
PASS-WITH-NOTES). This file adds no model number. Each value below is from that report and its
`COMMANDS.md` (data store `1804f48`, seed 7, 100,000 paths unless stated). This is not a price target.

## 1. Basis

| Item | Value | Source |
|---|---|---|
| Last close in the store | $145.98 (2026-11-19) | report L15 |
| P10 / P25 / P50 / P75 / P90 | $54.92 / $74.36 / $103.78 / $146.06 / $200.40 | report section 1 |
| Mean | $119.07 (SE 0.21) | report section 1 |
| Paths with a value below the close | 75.0% | `--price 145.98` |
| P50 over 14 seeds | $103.59 to $104.05 | report section 1 |

The close is at the P75 of the model. It is $0.08 below it.

What the price implies (one number at a time, the rest at base; report section 5):
- year-1 growth 72.3% (base 44.9%; the guide gives 41.0%); or
- a half-life of the growth fade of 2.08 years (base 1.0); or
- an equity risk premium of 2.72% (base 4.41%); or
- a beta of 1.04 (base 1.681).
A long-run margin alone cannot reach the close.

The drivers that carry the value, as P50 ranges (report "Result in brief"):
1. The speed of the growth fade: $88.75 to $92.45 at the base rates, against $103.78.
2. The equity risk premium: $122.41 at 3.5%, $88.29 at 5.5%.
3. Year-1 growth: $98.60 at the guide, $109.23 at the full median beat.
4. The share r of the equity purchases that keeps its value: $98.42 (r = 0) to $109.08 (r = 1).
5. The collapse of demand: the P10 is $41.46 to $64.31 for a multiple of 0.5 to 4.0.

## 2. Why no trade

The rule for a side (set by the Head of Fundamental Research on 2026-11-21, after the price was
known; it does not use the price to set an input):

| Test | For a buy | For a sell (short) | NVDA now |
|---|---|---|---|
| A. Margin | close at or below the P25 | close at or above the P75 | Close $145.98, P75 $146.06: the close is at the edge, with no margin. Fails. |
| B. Not only the premium | the side holds at a premium of 3.5% and of 5.5% | the same | P50 $122.41 and $88.29: both below the close. Holds for a short. |
| C. Not a property of the whole list | the gap is larger than the gap of the list | the same | Fails (see below). |

Test C: at the premium of 4.41%, most of the market is also below its price. In the S&P 500 list,
the median value at the means / price is 0.634. The premium that makes the list equal to its price
is 2.32% (`analysis/priors/market_implied_2026-11-20.md`, data of 2026-11-18). In business
equipment (BusEq) the median is 0.37. Thus "the value is below the price at 4.41%" is true for most
large stocks. It is not a fact about NVDA alone.

The two frames are not the same. The list numbers come from the automatic model with the value at
the means. NVDA uses the reviewed driver model with the P50. So 2.72% (NVDA) against 2.32% (list)
is not a measure that NVDA is cheap or dear relative to the list. No reviewed frame for a relative
call exists now.

A short on these numbers is a bet that the premium of the whole market goes up. It is not a bet on
NVDA. This sleeve has no rule for that bet.

## 3. The trade

None. Size 0. No entry, no stop, no target. The paper book records no position.

If a later re-examination gives a side, the write-up goes to `trading/journal/` (form of
`trading/journal/README.md`), and these items apply:
- **Size.** The paper desk has no sleeve limits yet (`PLANNING/board/T-20261120-paper-desk.md`,
  decision 2: the Head of Quality and Risk proposes them). No size is set before those limits exist.
- **Entry.** A limit at the band edge of the reviewed run of that date (P25 for a buy, P75 for a sell).
- **Exit.** The target is the P50 of that run. The review dates are the 10-Q and the next release.
  A stop needs the sleeve limits. For a short, the P90 ($200.40 now) is 37.3% above the close
  (200.40 / 145.98 - 1), and 10% of the paths are above it. So the open loss of a short has a
  fat tail.

## 4. What changes the answer

These facts start a re-examination. Each one is a new reviewed run, not a change of this file.
1. **The 10-Q of Q3 FY2027.** It gives convertible debt, the accrued purchase consideration and the
   notes (W13). It can move the net debt (-$98.124B) and the stakes ($109.7B of equity securities).
2. **The Q4 FY2027 release.** The guide is $130.0B for 14 weeks (+4.2% on Q3 in 13 weeks). A result
   or a guide far from this moves the fade (D3), the largest driver.
3. **The close moves outside the band.** Below $74.36 (P25) or well above $146.06 (P75), with tests
   B and C.
4. **A relative frame.** A reviewed value of the list in the same model and statistic as NVDA. Then
   test C can be done.
5. **The governance items** (a leadership change and an investigation by a board committee, W14).
   The model has no input for them.

## 5. Risks and costs (for a later trade)

- **Short:** the borrow cost is not in the store (no source); the open loss has no limit; the
  price can stay above the value for a long time.
- **Long:** the collapse of demand (D14) sets the low tail: P10 $54.92, with busts in 18.1% of
  the paths.
- **Options:** none. The owner keeps this area to the stock ("stick to underlyings").
- **Costs:** the commission and the spread of a trade at market quotes are not measured here.
  The portfolio backtest uses 10 bp a side.

## 6. How firm this is

- **The year-1 growth rule was set after the earlier runs (W1).** Most model changes raised the
  P50, and the last one lowered it (W15).
- **The premium of 4.41% is a REPO-tier convention.** It is the implied premium of Damodaran. The
  auditor could not open its primary source, and it is the second-largest driver.
- **The model has one segment (W5).** It has no history of Data Center and no customer
  concentration.
- **The data stops at 2026-11-19.** The close of 2026-11-20 is not in the store. The news said
  that NVDA fell about 14% that day (ticket `T-20261120-valuation-move-attribution`; not
  checked). A fall of 14% gives 145.98 x 0.86 = $125.54, still inside the P25 to P75 band ($74.36 to $146.06), so the
  answer stays "no trade". Test A must be done again with the real close.
- **The rule of section 2 is new.** It was written after the price was known. The gate checks it.

## 7. Review

Gate (Head of Quality and Risk, pull request 102 row):
1. `model-validator`: does the value support the side and the size? It also checks the stop and
   the horizon against the paths.
2. `auditor`: each number.
3. `risk-compliance-officer`: model risk, no order, the size limit.

Then Luc decides. The report and this recommendation are one decision.
