# Dashboard: fundamental (louellet-git)
For the next session of this head: the judgments about value that are not in the code, and what the
person decided. Keep this file below 60 lines. Started 2026-11-20. Dream pass: 2026-11-21 03:40 UTC.

## Goal now and subsequent steps (`RECORD.md` has the words; the board has the state of each item)
1. FIRST (2026-11-21 14:45, "hold off on other work"): valuation attribution of the week to 2026-11-20, 3 pages,
   S&P 10 first, then S&P 100. `T-20261121-sp100-week-attribution`, branch `claude/sp100-week-attribution`.
   Code: `fa dcf attribute --curve par` and the full bridge. Waits for the 2026-11-20 closes (Technology).
2. PAUSED, work committed, status on each ticket: the trade-level engine (`claude/trade-level-backtest`), the
   T1 revenue fix (`claude/t1-revenue-band`; the hold-out `--cut 2011 --base-to 2014` is still unused).
3. Stock DCF rank: no evidence of an edge on split-corrected prices (PR 136); no weekly trade from it.
4. NVDA (PR 106) merged: Luc approved the merge, NOT the value ("this doesnt mean I approve that the valuation is correct"). JST design frozen by Economics.
5. Rule of the person: each pause puts the state on the board and the report says where the work is.

## Decisions of this head (records A-001 to A-003; no reason uses the price)
- One long-run growth L = min(10-year zero yield, 4.0%) (FOMC SEP). Acquisition charge = w x expected
  spend, fading with the credited growth above L. A base-year decline enters as 0%.
- 2026-11-21: `acq_second` on; `acq_persist` on (1.75% + 0.3925 x past, fit on pit half 0, scored on
  half 1 and cut 2015 half 1); `acq_keep` off, open question: is growth at L paid by reinvestment only?
- The value at the means is deterministic and is NOT the expected value (the mean is higher).
- Market (2026-11-18): S&P list implies a premium of 2.32%, next 500 3.12% (model 4.41%).
- A trade side needs: close outside P25-P75; the side holds at a premium of 3.5% and 5.5%; the gap is
  not a property of the whole list (same frame). Written after the NVDA price was known: gate checks it.

## Lessons (tested)
- A paired rule must stay paired in EACH year and in ONE frame. Check it year by year before a brief.
- Check an in-sample error out of sample, and check whether a data window was selected.
- A backtest of the model uses a parameter table cut before its first year (in-sample table: +0.9
  points a year of look-ahead, 2026-11-21).
- Compare a stock strategy with the equal weight of the SAME scored list, by year, with a t value.
- 2026-11-21, after the split fix: the stock-level DCF rank shows NO EVIDENCE of an edge (A-009 R1-hold -1.10, t -0.58);
  the market-level premium passes 1893-2016 (PR 133). Weekly stock trades have no backed rule.
- The price files are split-adjusted to TODAY. A past value per share against a past price needs the share basis of
  the vintage (CRIT F1, 2026-11-21: BKNG 25x; R1 +6.56 became +0.71). A truncation test cannot see it.
- No claim before a run supports it; rerun two numbers of an agent before the gate.
- Do not edit code with a perl one-line substitution that has `|` in it: use the Edit tool.

## Standing words of the person
- 2026-11-21: "Backtesting is integral to our work here": each model change and strategy has a point-in-time
  backtest written before its run; trades only from a backtested rule; blind projections tested (T1-T4).
- The stochastic DCF engine, for stocks only; no option work in this area. Agents lean and of quality.
- Each analysis has COMMANDS.md with the store commit; each ticker judgment that can be a rule goes to
  `T-20261121-dcf-engine-from-tickers`. Few pull requests (CI cost). Windows laptop: PowerShell.
- The head merges its own PR after a gate pass and a green check (ORIENTATION section 6).
- A merged ticket moves to `PLANNING/board/done/`; never delete a ticket.
- Trade recommendations: fundamentals, under 3 pages, "paper" unless Luc says live.
- From time to time: an industry-practice note (`industry-<date>.md`, 1 page, ORIENTATION section 3).

## Tested rules and contacts
- Cache = data store (`GIT_LFS_SKIP_SMUDGE=1 git clone --depth 1 .../fa-data data/raw`); store 1804f48.
- Heads (send_message): Technology session_015ReypMwF8aVDzTBhLCbEv1; Quality session_015ReypKkZtAZBTkhHTskxJg;
  Quant session_015ReypKReqsgzkZY2hCsPdF; Economics session_015ReypPG2WmExK3KRHNj1e2.
- A reviewer asks: no push to a branch under review until the verdict.
- Data length: SEC facts from about 2009; most prices from 2016-11-18; SPY and IBM from 1962; GSW from 1961.
- cbo.gov refuses the cloud machine (HTTP 403); federalreserve.gov answers.

## Pages
- `responsibilities.md`: the area of this head and its boundary with the Head of Quant Research.
