# Dashboard: fundamental (louellet-git)
For the next session of this head: the judgments about value that are not in the code, and what the
person decided. Keep this file below 60 lines. Started 2026-11-20.

## Goal now and subsequent steps
The person wants (2026-11-20, `RECORD.md`): an engine with assumptions that are studied, decided and
written in the assumption records, and a fixed procedure that shows what the market price implies for
each stock, industry and sector. The person does not choose each assumption: this head decides.
1. `T-20261120-dcf-conventions-decision`: apply the two decisions below, pass the gate, merge.
2. `T-20261120-dcf-market-implied`: `fa dcf implied` (value at the means, three searches, group medians).
3. `T-20261120-dcf-assumption-register`: one table in the code, `fa dcf assumptions`, a test that ties
   each default to its record, the missing records, the skill for an assumption study.
4. Sector medians need the SIC code: ticket `T-20261120-sic-code` (head: technology).

## Decisions of this head (the reasons are in the records; no reason uses the price)
- 2026-11-20, long-run growth: one number `L` = lower of the 10-year zero yield and `G`. `G` = FOMC
  longer-run real growth + longer-run PCE inflation (SEP of 2026-09-16: 2.0% + 2.0% = 4.0%). Why FOMC
  and not CBO (3.8%): we can read the page, and ALFRED keeps each vintage for a point-in-time test.
- 2026-11-20, acquisitions: charge `w` x mean spend (`acq_w`), `w` = weight of the history in the growth
  mean. Known weak point: heavy buyers are over-charged in the point-in-time test.
- 2026-11-20, market-implied values: the path at the means (`PathEngine::deterministic`), not the P50,
  so that the result has no random part. A market-implied value is a measurement, never an input.

## Standing words of the person
- 2026-11-20: "the agents should not be overly sophisticated just overall lean and of quality, not
  obsessive and tight fisted".
- 2026-11-19 or 2026-11-20, on the GitHub Actions use: "this is too much usage". Effect: few pull
  requests. A pull request with documents only has no check run.
- The computer of the person is a Windows laptop. Give commands for PowerShell.

## Tested rules
- A new machine has an empty `data/raw`. A full sync of 1,114 tickers (`--only sec,prices,gsw`) takes
  about 70 minutes; start it first. Prices come from the Nasdaq source (10 years); Yahoo answers 429.
- This session can push a ticket branch with a different name (`claude/<ticket>`), as the repo rule asks.
- The quant agent can run code and tests while the cache downloads, but not the screen or the tests on
  the panel. Send it the data runs as a second message.
- cbo.gov refuses a request from the cloud machine (HTTP 403). federalreserve.gov answers.

## Pages
- No page.

## Dream pass
On the word of the person ("clean up memory" or "dream"). Last pass: none.
