# Record: quant (louellet-git)

The exact words of the person that decide or direct the work, with the clock time. Add at the end only.
Form and rules: `chiefs/MEMORY-HOWTO.md`, section 5. This repo is public: remove each secret and each
personal contact detail before you write an entry.

## 2026-11-20 19:02 UTC (chat)
Words: "You are FA-L - Head of Quant Research. Set your session title to that text, then read chiefs/ORIENTATION.md."
Done: The title is set and the page is read. No ticket is started. This entry was written at 19:12 UTC,
after the record rule came into `main` (commit bfb70af).

## 2026-11-20 21:24 UTC (chat)
Words: "Proceed with your work"
Done: Took the queue ticket with order 1, `T-20261120-option-history-root-defect`, on the branch
`claude/upbeat-cori-a4ykf5` (the one branch that this session can push).

## 2026-11-20 21:39 UTC (chat)
Words: "Speak w Head of tech about this"
Done: Sent the question about the missing Alpaca key id (HTTP 401 in the smoke test) to the Head of Technology.

## 2026-11-20 22:05 UTC (chat)
Words: "I think since you're the head of quant research, that arbitrage is going to be something of particular interest. So I think from our data, start taking over the responsibilities that are yours and beginning reconciliation / identification of arbitrage given our data. And just add stuff to the board based off of that."
Done: Started a survey of the arbitrage and reconciliation code and data of this area; tickets follow on the board.

## 2026-11-20 22:06 UTC (chat, with a screenshot of a search result: fundamental research studies the value of one company in depth with human judgment; quantitative research uses formulas, statistics and models to score or trade large sets of assets on numerical factors, with systematic rules and broad coverage)
Words: "Make sure to outline this for you and fundamental head separately for individual responsibilities"
Done: Wrote `memory/responsibilities.md` (the two areas and the boundary) and sent the outline to the Head of Fundamental Research.

## 2026-11-20 22:25 UTC (chat)
Words: "You don't need me to tell you to continue unless I tell you to stop, less tech head know"
Done: Continue with the queue after each ticket with no wait, until the person says stop. Told the Head of Technology. Started `arb-1-chain-check`.

## 2026-11-20 23:46 UTC (chat)
Words: "If there are permission requests, they go to the tech head, please let it know."
Done: Sent the open permission request (the refused run of the step-2 appendix script) and the rule to the Head of Technology.

## 2026-11-21 00:10 UTC (question dialog)
Words: "May the Quant head make a separate branch for each task, so each change is reviewed alone?" = "Yes (Recommended)"; "Run once the rates calculation that the safety system stopped (an auditor already checked those tables)?" = "No (Recommended)".
Done: New tickets get their own branch `claude/quant-<ticket>` from `origin/main`. The work already on `claude/upbeat-cori-a4ykf5` (arb-1b, arb-2, vol-1, vol-7, arb-4 step 1; each reviewed alone) goes in one pull request. The appendix run is not done. Both ask tickets move to `done/`.

## 2026-11-21 17:50 UTC
Words: "Do we have reconciliation between the VIX and option train IVs? Are we monitoring that?"
Done: Answer: no. Nothing calculates the VIX from our SPX chain, and nothing monitors it. The VIX replication is now one relation of the register below.

## 2026-11-21 17:55 UTC
Words: "I think that you should probably have a list or like a folder of arbitrage theories. That should generally always be true. And we should also have code that's running autonomously to make sure that that's the case. Fleshing this out will probably require some research to see how regular quant firms do this, and then how we should do this as well, and fitting it into our systems, / expanding our systems if need be."
Done: Ticket `T-20261121-arb-invariants` (branch `claude/quant-invariants`): research of industry practice first, then a register of no-arbitrage relations (one file for each relation) and one command that checks all of them on the newest data each day.
