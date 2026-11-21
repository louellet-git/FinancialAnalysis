# NVDA: trade recommendation (fundamentals)

**Recommendation: no trade.** Instrument: NVDA common stock. Account: paper (no live trade unless Luc
says live). Luc decides. Date: 2026-11-21; the data stops at the close of 2026-11-19 (the close is stale).
The value rests on one model and on judgment inputs (r, D3, D14, the premium of REPO tier). A side from
it would be weak evidence.
Re-examine at the 10-Q of Q3 FY2027 (about 2026-12-04: 40 days after 2026-10-25, the SEC rule for a large
accelerated filer) and at the Q4 FY2027 release (store estimate 2027-02-24, `earnings_dates.json`).

Evidence: `analysis/NVDA/REPORT-DRAFT.md` at commit `baf6358` (four-step review complete; auditor
PASS-WITH-NOTES) and its `COMMANDS.md` (data store `1804f48`, seed 7, 100,000 paths unless stated). This is
not a price target.

## 1. Basis

| Item | Value | Source |
|---|---|---|
| Last close in the store | $145.98 (2026-11-19) | report section 5 |
| P10 / P25 / P50 / P75 / P90 | $54.92 / $74.36 / $103.78 / $146.06 / $200.40 | report section 1 |
| Mean; value at the means | $119.07 (SE 0.21); $113.93 | report section 1; `COMMANDS.md` section 1 |
| Rank of the close (paths below it) | 75.0% (74.7% to 75.2% over 14 seeds) | `--price 145.98`; gate run |

What the price implies (one number at a time, the rest at base; report section 5): year-1 growth 72.3%
(base 44.9%); or a half-life of the growth fade (D3) of 2.08 years (base 1.0); or a premium of 2.72%
(base 4.41%); or a beta of 1.04 (base 1.681). A long-run margin alone cannot reach the close.

The drivers, as P50 ranges (report "Result in brief"): the fade D3 ($88.75 at 0.4 years to $163.14 at 2.5
years); the premium ($122.41 at 3.5%, $88.29 at 5.5%); year-1 growth ($98.60 to $109.23); the share r of the
equity purchases ($98.42 to $109.08); the collapse multiple (P10 $41.46 to $64.31).

## 2. Why no trade

**The reason that holds.** The price agrees with a reviewed value of the largest judgment. The fade that
the price implies (D3 = 2.08 years) is inside the reviewed range of D3 (0.4 to 3.0 years), and D3 = 2.5
gives a P50 of $163.14, above the close. So the model cannot say that the price is too high or too low.

**The three tests (written after the price was known; see section 6).**
- **A, margin (close outside P25 to P75): a tie.** The close is at the 75.0% rank. The P75 moves from
  $145.41 to $146.60 over the 14 seeds, so A gives a short in 5 of 14 seeds. One reasoned change flips it:
  a premium of 4.5%, a beta of 1.8, r = 0 or D3 = 0.8. Of 13 reasoned bands, 7 give a short.
- **B, the side holds over the named ranges of D3, r, the premium and year-1 growth.** It fails for both
  sides. A short fails at D3 = 2.5 (P50 $163.14) and at a premium of 3.5% (the close at the 62.5% rank,
  inside the band). A buy fails at the base and at each range limit: no P50 is above the close except
  D3 = 2.5.
- **C, the gap is not a property of the whole list: a veto until a list in the same frame exists.** The
  two frames give opposite calls. In the driver frame, the value at the means ($113.93, 0.78 of the price)
  gives a premium of about 3.2%, against 2.41% for the S&P list. In the automatic frame, the premium that
  the price implies is -1.32% for NVDA, so NVDA is dearer than the list. List figures: store `1804f48`,
  median value at the means / price 0.649, premium 2.41%, BusEq median 0.38 (`fa dcf implied`; the findings
  file `analysis/priors/market_implied_2026-11-20.md` has the earlier store `08361bd`: 0.634, 2.32%, 0.37).

## 3. The trade

None. Size 0. No entry, no stop, no target, and no time exit. The paper book records no position.
If a later re-examination gives a side, the write-up goes to `trading/journal/` (form of
`trading/journal/README.md`):
- **Size and stop:** none before the sleeve limits exist (`PLANNING/board/T-20261120-paper-desk.md`,
  decision 2: the Head of Quality and Risk proposes them).
- **Entry and target:** from the dated rule of a side (section 6), not from this file.
- **A short has no horizon for convergence.** The P90 ($200.40) is 37.3% above the close
  (200.40 / 145.98 - 1), and 10% of the paths are above it.

## 4. What changes the answer

Each item starts a new reviewed run, not a change of this file.
1. **The 10-Q of Q3 FY2027:** convertible debt, accrued purchase consideration, the notes (W13). It can move
   the net debt (-$98.124B) and the stakes ($109.7B of equity securities).
2. **The Q4 FY2027 release:** the guide is $130.0B for 14 weeks (+4.2% on Q3 in 13 weeks). A result far
   from it moves D3, the largest driver.
3. **The close moves:** the band of the dated rule of a side (section 6) applies, with its tie rule.
4. **A list in the same frame as NVDA:** then test C can be done.
5. **The governance items** (a leadership change, an investigation by a board committee; W14).

## 5. Risks and costs (for a later trade)

- **Short:** the borrow cost is not in the store; the open loss has no limit; no horizon for convergence.
- **Long:** the collapse of demand (D14) sets the low tail: P10 $54.92, busts in 18.1% of the paths.
- **Options:** none. The owner keeps this area to the stock ("stick to underlyings").
- **Costs:** the commission and the spread at market quotes are not measured. The portfolio backtest uses
  10 bp a side.

## 6. How firm this is

- **The rule of section 2 was written after the price was known, was used on one case only, and has no test
  power.** It must not be used for a trade again before it is written as a dated rule and reviewed (a record
  in `docs/assumptions/` or a rule in `PLANNING/RULES.md`: the band and its reason, a tie rule over seeds,
  the sims and seed, the statistic, the ranges of the judgments, the frame of C). It is not tuned on NVDA.
- **The year-1 growth rule was set after the earlier runs (W1).** Most model changes raised the P50, and
  the last one lowered it (W15).
- **The premium of 4.41% is a REPO-tier convention** (the implied premium of Damodaran). The auditor could
  not open its primary source.
- **The model has one segment (W5).**
- **The close is stale.** The ticket `T-20261120-valuation-move-attribution` quotes two news summaries
  (read on 2026-11-20, not checked): on 2026-11-20 the S&P 500 fell about 3.7%, the Nasdaq Composite about
  5.3%, and NVDA about 14%. An estimate: 145.98 x 0.86 = $125.54, inside the band. Test A must be done again
  with the real close.

## 7. Review

The gate of a trade recommendation (`docs/QUALITY-STANDARDS.md`, section 5; the row is on the branch of the
Head of Quality and Risk): `model-validator`, `auditor`, `risk-compliance-officer`. Then Luc decides. The
report and this recommendation are one decision.
