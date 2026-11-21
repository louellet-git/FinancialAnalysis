# Dashboard: fundamental (louellet-git)
For the next session of this head: the judgments about value that are not in the code, and what the
person decided. Keep this file below 60 lines. Started 2026-11-20. Dream pass: 2026-11-20 23:25 UTC.

## Goal now and subsequent steps
2026-11-20 (`RECORD.md`): (1) explain why the market fell in the week of 2026-11-16 and where the value
went, one stock at a time, then a general model; (2) what NVDA is worth; (3) an engine with studied
assumptions, and the price-implied values by stock, sector and industry. The person does not choose
each assumption: this head decides, with a reason that does not use the price.
1. `T-20261120-nvda-valuation` (branch `claude/nvda-valuation`): driver model on the path engine;
   four-step review; the owner approves. Waits for the new NVDA filing and prices (Technology).
2. `T-20261120-valuation-move-attribution`: split a price change into rates, implied premium and
   implied growth; NVDA first, then the sectors. Needs price and curve of a past date in `fa dcf implied`.
3. In review at Quality: the register (`claude/dcf-assumption-register`; after the verdict move the
   conventions ticket to `done/`) and the market-implied command (`claude/dcf-market-implied`).
4. Backlog: `T-20261120-dcf-acq-charge-limits`; the five records of Quality (premium, rf, horizon,
   beta, SBC) wait for the owner's word to that head.

## Decisions of this head (records A-001 to A-003; no reason uses the price)
- One long-run growth L = min(10-year zero yield, 4.0%): FOMC longer-run real growth + PCE inflation
  (SEP 2026-09-16). FOMC and not CBO: we can read it; ALFRED keeps each vintage. Pull request 77.
- Acquisition charge = w x mean spend, fading with the credited growth above L. Pull request 77.
- A base-year decline enters the growth mean as 0% (no fitted number). Pull request 81.
- Market-implied search uses the value at the means because it is deterministic. It is NOT the expected
  value: the mean is higher (median value at the means / mean 0.89). Statistic matters: sp500 V/price 0.63, mean/price 0.77, P50/price 0.477.
- First market result (data of 2026-11-18): the S&P 500 list implies a premium of 2.32%, the next 500
  list 3.12% (model 4.41%). Energy, Telecom, NoDur above the price; BusEq far below; Utils negative.

## Lessons (tested)
- A paired rule must stay paired in EACH year and in ONE frame (F1 of 77: ROP -$91; NVDA N1/N2: a charge that
  faded slower than its growth, and stakes at full value with each new purchase a total loss). Before I
  give a pairing rule in a brief, check it year by year and frame by frame.
- Check an in-sample band error out of sample before a correction (margin bands: 95% in, 80% out).
- Write no claim in an output text or a brief before a run supports it (V1 of pull request 81: I wrote
  "compare the mean with a price"). An agent can write numbers that no run made (V2): the record gets
  the command of each number, and I rerun two of them before the gate.
- Same error two times (V1 of 81, F1 of the implied file): a claim on the mean or the expected value
  with no run. Before a claim on a statistic, run `fa dcf auto` and read the mean line.

## Standing words of the person
- Immediate goal: the stochastic DCF engine, for stocks only; no option work in this area.
- 2026-11-21: "Ensure that work we do is repeatable and contributes to the dcf engine": each analysis has
  a COMMANDS.md with the store commit; each ticker judgment that can be a rule goes to
  `T-20261121-dcf-engine-from-tickers`.
- Merge rule: the head merges its own pull request after a gate pass and a green check
  (`chiefs/ORIENTATION.md`, section 6), with no new question.
- A merged ticket moves to `PLANNING/board/done/` (`status: done`, `merged:`); never delete a ticket.
- Agents lean and of quality. Few pull requests (CI cost). Windows laptop: commands for PowerShell.
- Merge `origin/main` once before a review request; a change of the output of a command of a
  different head: say it in the pull request title and send that head one message.

## Tested rules
- Cache = data store (`git clone --depth 1 https://github.com/louellet-git/fa-data data/raw`). No bulk
  download. Check: `fa dcf auto IBM --as-of 2026-11-19 --rules long_run=off,acq_w=0` gives P50 $132.87.
- Parallel agents: one worktree each (`../wt-<name>`), `--cache` to the store, read only.
- A reviewer asks: no push to a branch under review until the verdict. Send all corrections in one push.
- cbo.gov refuses the cloud machine (HTTP 403); federalreserve.gov answers.

## Pages
- `responsibilities.md`: the area of this head and its boundary with the Head of Quant Research (in main).
