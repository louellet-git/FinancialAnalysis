# Dashboard: quality (louellet-git)
What the next session of this head must know. Below 60 lines. Started 2026-11-20.
Form and purpose: `chiefs/MEMORY-HOWTO.md`. Memory of this head: the patterns of the reviews.

## Standing words of the person
- 2026-11-20 21:44 UTC: balance realistic risk with potential risk. Internal processes and
  internal documents: do not be constricting and do not slow the work. Risk in trades: strict.
  (`RECORD.md`; rule in `docs/QUALITY-STANDARDS.md`, section 3, "Proportion".)
- 2026-11-20: lean agents of good quality, "not obsessive and tight fisted". Few pull requests; PowerShell for Windows.
- 2026-11-20 23:22 UTC: choose the model of each agent run for the job (permission, not a duty); the
  files stay Sonnet. The validator, auditor and code reviewer found each MAJ: keep Sonnet for that work.
  Haiku fits a small mechanical check (a byte comparison, a narrow diff).

## Goal now
- Do the gate reviews that the other heads ask for; one verdict with all findings in one message. Head of Staff
  (PR 161): send it only done/blocked/Luc-decision/conflict items; its message is not an owner approval.
- Owner 2026-11-21: paper orders only (controls 0-8, PR 149); NVDA merge not a value approval; Alpaca shared store
  accepted; "docs for me are in pdf format" (a document for Luc also goes as a PDF).
- Until 2026-11-25 11:00 Toronto (token limit 71% used): gates in the order of EPICS.md last section; one agent a gate
  where enough, Haiku for narrow checks; at most 2 builds at a time (15 GB machine; OOM on 2026-11-21).
- HANDOVER (2026-11-21 21:05 UTC): every gate of 2026-11-21 has a verdict (ledger). Open: review the premium-series
  PIT results (61fe8de) before the series run of record; PR 198 waits for the Fundamental head's yes on the skill text;
  9 stray HQM files in /home/user/qa-fa-data-e06/bonds/treasury_hqm (local, untracked). A new session needs Luc's word.

## How a review runs here (tested 2026-11-20 and 2026-11-21)
- One detached worktree for each review (`git worktree add --detach ../wt-qa-<name> <commit>`); build
  the release binary before the agents start; each agent has its own `CARGO_TARGET_DIR` in scratch.
- A refactor or a merge: build the base too and ask for a byte comparison (`cmp`). Cheap and strong.
- Data: the store is private from 2026-11-21: attach it first (add_repo louellet-git/fa-data, read). It moves during the day. Clone it one time (`GIT_LFS_SKIP_SMUDGE=1 git clone --depth 1
  https://github.com/louellet-git/fa-data /home/user/qa-fa-data`); for a second fixed commit use
  `git -C /home/user/qa-fa-data worktree add --detach <path> <commit>`. Name the store commit in each
  brief and verdict. A fetch that only adds files does not disturb a running review.
- No bulk download; harness in scratch only; delete target dirs (disk).
- An agent's verdict can disagree with its levels. Set the verdict from the levels, after "Proportion".

## Review patterns
- Tail of a model change: ask for the tail class and the count of P50 <= 0 (ROP P50 $441 to -$88).
- A record that rejects alternatives with numbers: the validator runs each one (-2.9% stated, -0.4% run).
- A backtest: ask for a poison test of each data source it reads, not only the prices (the SEC
  restatements entered value-rank). A large change of a result after a fix: isolate code against data.
- A fit or calibration: ask for the convergence status and the parameters on a bound (vol-1).
- A findings file: check the statistic behind a headline (the value at the means is not the mean).
- A new mapping tag that is a fallback: check non-target companies and old periods, not only the
  latest year (the status check sees the latest year only; mapping review, 2026-11-21).
- QA agents often rate wording, a document conflict or a text claim MAJ. Apply "Proportion": MIN when
  no value, data or trade can change. Each time I changed a level, I said so in the verdict.
- Cycle 2 precedent: pass with written conditions; I check the fix diff myself. A valuation report has no 2-cycle limit.
- A valuation with a charge for future spending: check that the frames agree (an asset held at full value
  and each future purchase a total loss is two frames; NVDA round 2). Ask for the range across frames.
- A weighted sum over a list: ask for the top weights and the excluded part (one 1000x share count of
  WAT made 39% of the weight; attribution review).
- Model change: model doc and records current; record number in its head's range (Fund A-001+, Quant 100+, Econ 160+, Tech 200+, QR 230-249).
- A harness or a backtest: ask the validator to plant a look-ahead (1 day and 40 days) in a scratch copy;
  the author's own mutant only showed the long case (backtest environment, 2026-11-21).
- A model change: ask for the P50 change by class (margin, size, no spend) in % of value, not only the
  score in % of revenue (acquisition intercept: thin margins -25%).
- A design before a run: one primary test, a rejection rule, a fixed window count; review it before data.
- Split-basis trap (skill point-in-time-test, build step 6): BKNG 25x; truncation and poison miss it.
## Dream pass (before a compaction, above about 75% in `get_session`, or on "dream")
Last pass: 2026-11-21 04:10 UTC; earlier 01:25 and 00:30 UTC.
