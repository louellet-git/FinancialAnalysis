# ANT first pass: value range at the IPO (2026-11-19)

**First pass. Not reviewed. Do not use it alone for a trade. Claude models made it, and
Anthropic makes Claude.** No skeptic, reviewer or auditor examined this work.

Reproduce: `fa dcf drivers ANT --price 83 --price 105 --path` (seed 7, 10,000 simulations,
10 years). The command for each number is in `COMMANDS.md`.
Driver reasons: `driver_model.toml` (and `driver_model.py`, the older copy). Facts: `research-ledger.md` (final prospectus).
"Press" marks a news source, which has lower quality than the prospectus.

## Value range for each share

| P10 | P25 | P50 | P75 | P90 | Paths below $83.00 | Paths below $105.00 |
|---|---|---|---|---|---|---|
| $12.82 | $19.30 | $30.97 | $49.54 | $74.96 | 92.2% | 96.0% |

- $83.00 is the IPO price. ANT had no trade at 11:15 New York time on 2026-11-19.
  $105.00 is the last opening indication (press, Yahoo Finance). It is not a trade.
- WACC at means 11.24% (rf 5.07% + beta 1.40 x ERP 4.41%; equity weight 1.0). The
  terminal value is 98.6% of EV at means: the cash flow of years 1 to 5 is negative.
- Equity value at P50: $562B. EV at P50: $331B, 9.4 times trailing revenue. At $83.00:
  market value $1,505B, EV 36 times trailing revenue.
- Engine check: at the driver means, the engine and a hand DCF both give $31.6971. The
  tails are too narrow (repo rule F6): do not read P10 or P90 as a safety margin.

At means, revenue is $105.0B / $151.2B / $217.7B / $541.8B in years 1 / 3 / 5 / 10, and
the FCFF margin after SBC is -32.0% / -18.3% / -4.6% / +16.0%. Year-10 revenue: P10
$241B, P50 $537B, P90 $1,119B. The best 10-year result in the repo data from a $30B start
is AMZN (27.4% a year): $397B from this base. Damodaran has $1,011B. Thus the P50 is
above the best precedent and below the public forecasts.

## What the market price implies

Bisection on the engine P50. One driver changes; the other drivers stay at base.

| Necessary for | $83.00 (IPO) | $105.00 (indication) | Base |
|---|---|---|---|
| (a) Growth from year 2, for 9 years | 37.4% a year | 41.4% a year | 20% |
| Year-10 revenue with (a) | $1,833B | $2,376B | $542B |
| (b) Mature FCFF margin after SBC | 46.9% | 60.0% | 16% |
| (c) Beta (cost of equity) | 0.24 (6.11%) | 0.07 (5.39%) | 1.40 (11.24%) |
| Joint: growth, with margin 19% and beta 1.10 | 30.7% ($1,171B) | 34.6% ($1,522B) | - |

In plain words: $83.00 needs about $1.2 trillion of revenue in year 10, with a 19% margin
after SBC and a 9.9% cost of equity. No company has had revenue of $1 trillion. With the
Damodaran revenue ($1,011B) and margin (19%) and the repo discount rate, the P50 is
$59.08. Damodaran has $1.475T of equity ($81.32 on this share count): his cost of
capital is 9.74% to 9.13% and his terminal growth is 4.90%.

## Key drivers

| Driver | Mean | Spread | Source | Effect on P50 |
|---|---|---|---|---|
| Growth from year 2 | 20% | std 8% | Judgment: between the base rate and the forecasts | -1 std $-10.12; +1 std $+17.10. Mean 10% / 30%: $19.12 / $53.92 |
| Mature FCFF margin before SBC | 22% (16% after SBC) | std 6% | Judgment: peers 12% to 16% now; Damodaran 19.0% | -1 std $-11.55; +1 std $+11.63 |
| Beta | 1.40 | locus 1.10 / 1.70 | Judgment: peer median 1.38 | $37.19 / $26.53 |
| ERP | 4.41% | std 0.75% | Damodaran, 2026-11-01 | -1 std $+4.73; +1 std $-3.62 |
| rf | 5.07% | std 0.5% | 10-year Treasury, 2026-11-18 | -1 std $+2.06; +1 std $-1.81 |
| Year-1 revenue (pinned) | $105B (+198%) | none | Run rate $81B at 2026-10-31; press forecasts | $80.6B / $125B: $26.73 / $34.44 |
| Margin ramp | -7% in year 1, mature in year 8 | none | 9M2026 cash flow; committed spend | Mature in year 6 / 10: $34.31 / $27.24 |
| SBC, new grants only | 25%, to 6% in year 8 | none | Grants of 2026; GOOGL 6.2% | Year 1 at 15%: $33.33 |
| Terminal growth | 2.25% | std 0.75% | Repo convention | -1 std $-1.48; +1 std $+1.73 |

Three drivers explain most of the spread: growth, the mature margin, and the cost of
equity (beta and ERP). Horizon of 15 years: $48.84 with growth 20% (year-15 revenue
$1,348B, no precedent); $36.67 with growth 16.7% (year-15 revenue $910B).

## Bridge

1. Shares 18,137.6M: the fully diluted count of the prospectus (treasury stock method at
   $83.00). Class A 7,585.3M, Class N 7,766.5M, options 1,712.1M, RSUs 1,073.7M.
2. The Amazon rollover notes are 1,547.7M of these shares. Thus the note liability
   ($109.7B at fair value; principal about $5.6B) is not in the net debt.
3. The unvested awards are in the count. Thus the SBC driver is new grants only.
4. Net cash $230.7B ($12.72 a share): cash and investments after the offering $232.4B
   less finance leases $1.7B. Operating leases and commitments stay in the margin.
5. Other choices: no subsidiary shares (846.8M), $32.49. Leases not started ($451.2B) as
   debt at 7.07%, out of the margin, $27.74. Damodaran debt $472.3B added, margin not
   changed (counts the cost two times; a lower limit), $4.93.

## The five largest limits

1. **Constant growth after year 1.** Real growth fades. With the same year-10 revenue, a
   path from 38.9% down to 2.25% gives $31.81 against $31.70 (+$0.11): more early revenue
   comes at negative margins. The stop of growth in year 10 has a larger effect (see the
   15-year values). The year-1 pin has no spread.
2. **No XBRL.** A person or an agent typed each value from the prospectus text. The 9M2026
   values are unaudited. Press numbers feed the year-1 pin and the forecast comparison.
3. **No beta history.** Beta 1.40 is a judgment (locus $37.19 to $26.53).
4. **Commitment risk.** Committed spend to 2030 is $451.6B: 73% of revenue at mean growth,
   86% at P10 growth. With equal spend each year, it gives margins before SBC of -24% and
   -9% in years 2 and 3; the model has -3% and +1% ($-1.85 a share, not in the base). The
   engine cannot show low growth with fixed costs, and it has no failure probability.
5. **Share count.** The count is fixed at $83.00. The board can issue up to 2% more shares
   each time the price goes up 50% (L258). Donation matching adds shares. No charge here.

Sources: prospectus (URL in the ledger); Damodaran ERP page and Anthropic slides; U.S.
Treasury; Yahoo Finance live blog and stockanalysis.com (price status); peer data, base
rates and press forecasts from the market notes of this ticket (not in the repo).
