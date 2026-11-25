# alpha-1 stage 1: the premium in isolation and s05_ic_0dte again

Ticket: `PLANNING/board/T-20261121-alpha-1-pipeline.md`. Branch `claude/quant-alpha-1`.
Every option price in this note is a MODEL price. Nothing here is proof of profit.

## Hypothesis and gate (written 2026-11-21, before any test was run)

H1a. The implied variance of the VIX (VIX squared) is on average larger than the realised
variance of SPY over the next 21 trading days, and the premium is positive on most days
outside the stress windows (`analysis/macro/2026-11-21-stress-windows.md`). The same holds
for VIX1D against the same-day SPY move (2023 to 2026).

H1b. s05_ic_0dte (as coded on 2026-11-21, no change of its rules) keeps a positive
out-of-sample return and t > 2 of the mean trade P&L when the model price is worse: the
spread x2, the skew +50% and -50%, and a fill at the bid or ask instead of the mid.

Gate 1 (from the ticket): the premium is positive outside the stress windows, and s05 keeps
t > 2 and a positive return under the worse prices. Else: stop, and write why.

Also shown (not in the ticket): the researcher bar t > 3 and the deflated Sharpe ratio. The
ticket gate (t > 2) decides; a t between 2 and 3 is called "weak".

Rules of the test: no variant is chosen by its result on the score set. s05 stays as it is.
The out-of-sample part is the last 30% of the bars (the split of the leaderboard). Each run
goes into the trial table at the end.

Precision of the gate, also written before the first run: "outside the stress windows" means
that the date of the signal is not inside [start, end] of a window of the stress file. The
file covers 2016-01-05 to 2026-11-19, so the gate uses this period. "Positive" means mean
premium > 0 with a Newey-West t > 2. Dating limit: the threshold "t_NW > 2" for 1a is in this
note only. Git does not date it before the runs: the ticket (commit b49e7e8) says only
"positive". The stop below does not rest on it (see the gate decision). Rule from stage 2: the
hypothesis and the gate go in a separate commit before the code and the runs. The years 1993 to 2015 have no window in the file (they
contain 2000-02 and 2008): they are shown as information, not in the gate. A second view
"clean outside" (no window inside the next 21 trading days either) is information only.

## Method (what was run)

Commands: `fa trade premium` (new, `rust/src/trading/premium.rs`) and `fa trade backtest --bot
s05_ic_0dte --trials 20 [--spread-mult X] [--skew-mult X] [--mid-fill] [--sr-var V] [--end D]`.
Data: `data/raw/trading`, run 2026-11-21. SPY daily 1993-01-29 to 2026-11-18 (the file starts
in 1993; the VIX in 1990), VIX1D 2023-04-24 on, SPY hourly 2023-12-21 on. No survivorship (index).
The VIX close is known at the close of day t; the realised side uses only later days.

## Stage 1a: the premium (real index levels and real SPY prints, no option model)

30-day premium = (VIX/100)^2 minus realised variance of the next 21 trading days (sum of
squared close-to-close log returns x 252/21, no demeaning). Unit: annual variance; 0.01 is
about 10 vol points squared. t_NW: Newey-West, 20 lags, on the masked deviations. t_blk: every
21st day, plain t. pos%: share of days with a positive premium (the squared-return sample is
skewed, so the median and pos% look better than the mean).

| subset | n | mean | median | pos% | t_NW | t_blk |
|---|---|---|---|---|---|---|
| all days 1993-2026 | 8489 | 0.00972 | 0.01143 | 83.7 | 4.36 | 3.36 |
| 1993 to 2015 (no windows defined) | 5775 | 0.01073 | 0.01127 | 83.5 | 4.38 | 3.01 |
| 2016-2026 all | 2714 | 0.00756 | 0.01182 | 84.0 | 1.64 | 1.55 |
| 2016-2026 inside windows | 672 | 0.01353 | 0.02505 | 81.0 | 1.06 | 0.78 |
| **2016-2026 outside windows (gate)** | 2042 | 0.00560 | 0.01035 | 85.0 | **1.97** | 1.79 |
| 2016-2026 clean outside: USES THE FUTURE, not a result | 1833 | 0.01126 | 0.01107 | 90.9 | 13.49 | 9.61 |

In vol points (VIX minus realised vol): all days mean 3.67; outside windows 2016-2026 mean 3.35.
"Clean outside" drops days with a window in the next 21 days: it uses the future, so it is biased.
Do not quote its t as a result; the row stays only as a warning. The windows were defined with
hindsight (after the stress was known), so "outside windows" is itself a choice made with the
future. Read the gate row (t_NW 1.97) next to the full 2016-2026 sample (t_NW 1.64, no window
choice).

Lag table for the gate row (2016-2026 outside windows; t_NW by number of lags): lag 10 2.26,
lag 20 1.97, lag 30 1.87, lag 63 1.77, lag 126 1.80. The result depends on the lag, and the
overlap of the 21-day windows asks for 20 lags or more. The t is near 2 and below it at every
lag of 20 or more. Do not call 1.97 a narrow miss that another method could rescue: the
choice of the lag moves it by 0.5.
Windows with a weak premium: 2018-Q4 (mean 0.0006, 52% positive), 2022-Q2/Q3 (0.0030, 57%),
2025-04 (mean -0.0011, median +0.033). In 2020 the mean is 0.0129 and the median 0.0375: a few
days of very large realised variance pull the mean. Short windows have too few days for a t.

1-day premium = (VIX1D open/100)^2/252 minus the squared return of the same day, 897 days from
2023-04-25. The VIX1D value is the open of the day (known at the open). t_NW 5 lags.

| convention | subset | n | mean | median | pos% | t_NW |
|---|---|---|---|---|---|---|
| close to close | all | 897 | -0.00003 | 0.00001 | 62.2 | -2.58 |
| close to close | inside windows | 81 | -0.00021 | 0.00006 | 63.0 | -1.75 |
| close to close | outside windows | 816 | -0.00001 | 0.00001 | 62.1 | -4.88 |
| open to close | all | 897 | -0.00000 | 0.00002 | 74.9 | -0.22 |
| open to close | outside windows | 816 | +0.00001 | 0.00002 | 75.5 | 3.13 |

Close to close includes the overnight move, which VIX1D (an index of the session after the
open) does not price in full; open to close leaves out the overnight move and the first minutes.
The two conventions bracket the answer, so the honest result is: about zero, and it depends on the
convention (it does not show a premium).

Reading: the 30-day premium exists over 33 years (t_NW 4.36); in 2016-2026 outside the windows
t_NW is 1.97, below 2. The 1-day premium is negative close to close (the median is positive
only because squared returns are skewed) and positive only against the session alone, outside
the windows. Limit: VIX1D is a Yahoo series; its open print may be stale.

## Stage 1b: s05_ic_0dte again (MODEL prices; rules unchanged; $2,000 account, 3% risk, $0.65 per contract)

Price knobs: `pricing.rs` had none. I added `set_price_knobs(spread_mult, skew_mult, mid_fill)`
(defaults 1, 1, false: the old numbers do not change, see R2). Flags: `--spread-mult`,
`--skew-mult`, `--mid-fill`. The default fill is ALREADY the worse side (long legs pay the
ask, short legs get the bid; half-spread 0.5% of mid, $0.01 to $0.05), so "worse side" is the
baseline, not a new run. New runs: spread x2, skew slope and curve x1.5 and x0.5, and the mid
fill (a better price, a reference). The commission is in all runs.

OOS = the last 30% of the bars (R1 and R3 to R6 from 2026-01-07, R2 from 2025-12-05).
Sharpe = daily equity returns x sqrt(252). t = mean trade P&L in dollars / its standard error
(the size grows with equity, so t of P&L / max loss is also given). DSR: N = 20 trials as a lower
bound (6 runs here + 14 earlier from the bot note). N is too low: s05 was the best of about 21
bots, and about 14 of its own variants were seen in-sample, so N is 20 to more than 100. The DSR
at N = 20 is an UPPER bound; a larger N gives a lower DSR. SR0 from the variance V of the daily Sharpe ratios of the
trials. Two choices of V: 1/T (0.0046, the sampling variance of one Sharpe ratio under no edge)
and the variance of the 6 runs here (0.00096; low, the runs are close variants). The table gives
DSR for V = 1/T / V = 0.00096. The better figure is V = 1/T (R1: DSR 0.32); the default 1/T is a
proxy, and the V from the 6 close variants is too low. Neither is a measured V of all trials.

| run | change | OOS return | OOS max DD | OOS trades | OOS Sharpe | t(P&L) | t(P&L/maxloss) | DSR |
|---|---|---|---|---|---|---|---|---|
| R1 | none (data to 2026-11-18) | +38.8% | -10.7% | 57 | 1.59 | 1.35 | 1.29 | 0.32 / 0.76 |
| R2 | none, data to 2026-10-09 | +26.7% | -10.7% | 51 | 1.30 | 1.06 | 1.01 | 0.21 / 0.65 |
| R3 | spread x2 | +10.1% | -12.7% | 54 | 0.67 | 0.52 | 0.75 | 0.09 / 0.40 |
| R4 | skew x1.5 | +37.0% | -11.0% | 56 | 1.54 | 1.30 | 1.21 | 0.30 / 0.74 |
| R5 | skew x0.5 | +37.9% | -15.2% | 61 | 1.46 | 1.18 | 1.35 | 0.27 / 0.70 |
| R6 | mid fill (reference, better) | +57.3% | -15.2% | 66 | 1.86 | 1.59 | 1.74 | 0.43 / 0.84 |

Whole period (2023-12-21 to 2026-11-18, in-sample included), return / trades / t(P&L): R1 +98.0%
/ 106 / 1.99; R2 +84.0% / 99 / 1.85; R3 +44.6% / 92 / 1.27; R4 +95.4% / 103 / 1.96; R5 +114.4%
/ 117 / 2.00; R6 +149.3% / 139 / 2.24.

Reproduction of the leaderboard: R2 gives the ALL numbers of `LEADERBOARD.md` (+84%, 99
trades; Sharpe 1.21 against 1.19, max DD -10.7% against -11%). The OOS numbers differ
(here +26.7%, 51 trades, Sharpe 1.30; leaderboard +40%, 56 trades, Sharpe 1.69). Cause: the
cut. These runs start the bars at 2023-11-10 (the file of the store here), the leaderboard run
at 2023-12-21, so the 70/30 cut falls three weeks earlier. A cut three weeks earlier moves the
OOS Sharpe from 1.30 to 1.78 (same rules, same data). So the OOS result is not one number:
report OOS Sharpe as a range, 1.30 to 1.78 for the baseline (R2 data), and note that a cut chosen
by the date is a source of noise the size of the effect.
CORRECTION (2026-11-21, review of the grids): the cause stated above is wrong. The store file
`SPY_1h.csv` starts 2023-12-21, as the table of this note says ("2023-12-21 to 2026-11-18"); the start
2023-11-10 is only in the old bot note `s05_ic_0dte.md`. The cut differs because the END of the data
differs (R2 ends 2026-10-09, the leaderboard run 2026-11-18). The range also depends on the step and the
span of the cuts: `fa trade backtest --bot s05_ic_0dte --cut-scan` (default cut +/- 42 days) gives, on
`--end 2026-10-09`, 1.30 to 1.68 at step 7 and 1.30 to 1.81 at step 1 with span 28; on the full data
(to 2026-11-18) 1.43 to 1.87 at step 1 with span 42 and 1.44 to 1.87 at step 7. No one of these is "the"
range: each is a sensitivity of the OOS Sharpe to the cut, with its step and span. The text above stays
as the history of the first reading.

Reading: the return stays positive in every price run (OOS +10% at the lowest). The t of the
mean trade P&L is below 2 in every OOS run, also in the baseline (1.35), so it is not a
result of the worse prices. With 57 OOS trades and kurtosis 13.9, the standard error is
large. Power: for t > 2 the mean trade P&L must be about $30 (the mean is $19.42). So the result
is "not proven", not "no edge": the test cannot tell a $19 mean from $30 or from 0. The spread x2 takes out 74% of the OOS mean P&L ($19.42 to $4.91): the strategy is
sensitive to the spread more than to the skew. The note of the bot says the 10 best trades
make 86% of the P&L.

## Trial table (every run; no run failed)

| id | what | parameters | result |
|---|---|---|---|
| 1a-1 | `fa trade premium` | horizon 21, lag 20 | 30-day outside t_NW 1.97; 1-day cc mean negative |
| R1 to R6 | s05 runs above | as in the table | as in the table |
| prior | s05 variants of the bot note | about 14 (note: "roughly 14") | in the note |
Trials for the DSR of s05: 6 + 14 = 20. A first print with `--trials 21` gave the same t and
DSR 0.306: it is the same run with another N, not a new variant. Nothing was chosen by a
result; the rules of s05 and the grid of runs did not change after the first run.

## Gate decision

**Gate 1 is NOT passed. Stop.** The primary reason is 1b: s05 OOS t is 1.35 against the ticket
bar t > 2 (item 3). The 1a miss (item 1) is only secondary: its threshold is not dated before
the runs (see "Precision of the gate") and its t depends on the lag.
1. Premium (1a): outside the stress windows (2016-2026) the mean 30-day premium is positive
   (0.0056, 3.35 vol points, 85% of days), but t_NW is 1.97, below 2 (t_blk 1.79). Not a result that another method
   rescues: t_NW is 2.26 at lag 10, 1.87 at lag 30, 1.77 at lag 63, and the full 2016-2026
   sample has 1.64. The 33-year premium is clear (t_NW 4.36); in 2016-2026 it is thin.
2. The 1-day premium (VIX1D against the same-day move) is about zero, and depends on the
   convention: negative close to close (t_NW -2.58, it includes the overnight move) and about
   zero open to close. The premium that s05 sells is not shown in the index data.
3. s05 (1b): OOS t of the mean trade P&L is 1.35 in the baseline, not above 2, and 0.52 with
   the spread x2. The return stays positive (+10.1% worst). DSR is 0.32 (V = 1/T, the better figure) or 0.76 (V from the runs), at N = 20. N is too low
   (20 to more than 100), so these are upper bounds: under 0.95 in both. The researcher bar (t > 3) is far off.
4. Limits: model prices; 57 OOS trades; one regime (2023-2026); a few large wins carry the P&L;
   OOS Sharpe 1.30 to 1.78 by the choice of the cut (R2 data; see the CORRECTION: the range depends on the end of the data, the step and the span). With the power given above, the result is
   "not proven", not "no edge".
If the owner still wants stage 2: measure on the real chain copies (a) the real credit of the
$2-wide ATM iron fly at 10:30 against the model credit (gate 2: at least 80%; the model
needs at least $1.40), (b) the real half-spread of each leg (the model uses $0.01 to $0.05; the
spread x2 run takes away most of the edge), (c) the real VIX1D-implied ATM price against the
model price. Without that the 1b result cannot rise above a screen.

## Open items (not done now)

- `metrics.rs` `daily_returns` drops the return across the IS/OOS cut (negligible: one day).
- `pricing.rs` price knobs are process-wide atomics; pass a knob struct in a later change.
- Choose N for the DSR from a count of all bots and variants seen, and measure V from all trials.
- Stage 2: the hypothesis and the gate go in a separate commit before the code and the runs.
