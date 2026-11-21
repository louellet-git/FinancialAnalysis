# Record: quality (louellet-git)

The exact words of the person that decide or direct the work, with the clock time. Add at the end only.
Form and rules: `chiefs/MEMORY-HOWTO.md`, section 5. This repo is public: remove each secret and each
personal contact detail before you write an entry.

## 2026-11-20 19:12 UTC (chat)
Words: "You are FA-L - Head of  quality and risk. Set your session title to that text, then read chiefs/ORIENTATION.md."
Done: The words arrived at 19:03 UTC. The title is `FA-L - Head of Quality and Risk` (the exact text of the orientation table). Orientation read. No ticket started.

## 2026-11-20 21:44 UTC (chat)
Words: "As the head of quality and risk, I understand that it's very important that you capture any risk inherent in the system we have. I just want to make sure that you're not slowing things down. So make sure you balance, realistic risk with potential risk. So when we're talking like internal processes or internal documentation, just balance what is realistic, what's not. I've had cases where cybersecurity agents are a bit too constricting. So we want to make sure we're not constricting. When it comes down to risk involved with trades, that's a very different story."
Done: a rule of proportion in `docs/QUALITY-STANDARDS.md`, section 3, and in the risk officer agent file; a standing word in the dashboard. The terms check of the public data store is dropped (the owner decided; no realistic risk to a trade).

## 2026-11-20 23:07 UTC (chat)
Words: "I approve you changing your reviewer agents becoming haiku agents Unless you feel it should otherwise be sonnet"
Done: Haiku for `document-reviewer`, `data-quality-analyst` and `test-engineer` (their reviews of today found MIN items: format, data checks, test plans). Sonnet stays for `model-validator`, `auditor`, `code-reviewer` (they found each MAJ of today) and `risk-compliance-officer` (judgement, seldom run). `docs/TEAMS.md` line 4 says it.

## 2026-11-20 23:22 UTC (chat)
Words: "Don't over commit to one model, since the scope of the job will change is what I meant. Tell the tech that. Keep the rule lean and not a need but permission to."
Done: the three agent files are Sonnet again; `docs/TEAMS.md` line 4 gives the permission to choose a different model for one run. A message to the Head of Technology for `chiefs/ORIENTATION.md`, section 3.

## 2026-11-21 01:47 UTC (dialog)
Words: two answers: "Yes, start" (the model-documenter writes the 5 NVDA assumption records from the next draft) and "Open it" (one pull request for the document changes of this branch).
Done: the records start with the next NVDA draft; the branch gets a document review, then one pull request.

## 2026-11-21 01:49 UTC (correction and source note; no new words of the person)
- Correction of the entry of 2026-11-20 21:44: the words of that entry do not mention the terms check. The decision that the data store stays public is the owner's word in `chiefs/louellet-git/technology/RECORD.md` (2026-11-20 20:01). To drop my own recommendation of a terms check of the store was my decision under "Proportion"; check 3 of the risk officer (terms of a new source) is unchanged.
- Source of the four new checks of the skill `model-validation` (steps 3 and 7): the findings of my gate reviews of 2026-11-20 and 2026-11-21 (the conventions, accuracy, portfolio and vol-1 verdicts; the ledger rows), recorded in my dashboard under "Review patterns". No word of the person.

## 2026-11-21 03:16 UTC (chat)
Words: "I need the report to be a trade recommendation, based on fundamentals. Trade recommendations should be sub 3 pages."
Done: a request to the Head of Fundamental Research for an NVDA trade recommendation of less than 3 pages, from the reviewed report (baf6358). A row for "Trade recommendation" in `docs/QUALITY-STANDARDS.md`, sections 4 and 5. A trade gets the strict levels (entry of 2026-11-20 21:44).

## 2026-11-21 03:47 UTC (relayed verbatim by the Head of Fundamental Research; not yet confirmed to me in chat)
Words: "And every week you will need to propose 3 trades to begin on the live trader, beginning next Friday. A valuation engine will be required. Potentially a hedging engine when and if that's needed."
Done: live-pilot limits added to `proposal-sleeve-limits.md`; a gate form for the weekly proposal sent to the Head of Fundamental Research; a question to the owner on live against paper for the first weeks.

## 2026-11-21 ~03:55 UTC (dialog)
Words (four answers): live start "Paper 4 weeks, then live"; sleeve limits "Not sure"; PR 102 "Merge now"; Deere lease equipment "I'm not sure".
Done: the weekly process runs on paper from 2026-11-27 for 4 weeks; live from about 2026-12-25 only after the owner approves limits. The proposed limits apply to paper as an interim rule (no real money). The Deere and CNH lease lines stay out (the interim rule) until the owner decides. PR 102 merged.

## 2026-11-21 04:29 and 04:30 UTC (relayed verbatim by the Head of Fundamental Research)
Words: "Ensure that before you have trades and we begin the weekly trades, that you have effectively back tested your future state blind projections using the DCF engine and using prior data. A recommendation should work at any time and point and when it doesn't, the model should have already priced it in such that it does work." And: "Backtesting is integral to our work here".
Done: the blind-projection tests T1 to T4 (ticket `T-20261121-blind-projection-test`) are a condition of the strategy gate before the first weekly trade, paper included. A row "Backtest" in `docs/QUALITY-STANDARDS.md`, section 4.

## 2026-11-21 05:33 UTC (my decision; no word of the person)
The public copy of a split table from the Alpaca corporate-actions API: allowed as derived facts only (ticker, ex_date, ratio, factor, kind, sec_check, source), each checked against the SEC split tag where one exists; no raw Alpaca payload and no Alpaca-only field in the public store. Alpaca terms for redistribution are not read; this is written in `docs/DATA-SOURCES.md`. The person can reverse this.
