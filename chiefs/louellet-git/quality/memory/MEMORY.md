# Dashboard: quality (louellet-git)
What the next session of this head must know. Below 60 lines. Started 2026-11-20.
Form and purpose: `chiefs/MEMORY-HOWTO.md`. Memory of this head: the patterns of the reviews.

## Standing words of the person
- 2026-11-20 21:44 UTC: balance realistic risk with potential risk. Internal processes and
  internal documents: do not be constricting and do not slow the work. Risk in trades: strict.
  (`RECORD.md`; rule in `docs/QUALITY-STANDARDS.md`, section 3, "Proportion".)
- 2026-11-20: the agents are lean and of good quality, "not obsessive and tight fisted".
- Few pull requests; a pull request with documents only has no check run (`chiefs/ORIENTATION.md`, section 6).
- The computer of the person is a Windows laptop: give commands for PowerShell.
- 2026-11-20 23:22 UTC: choose the model of each agent run for the job (permission, not a duty); the
  files stay Sonnet. The validator, auditor and code reviewer found each MAJ: keep Sonnet for that work.
  Haiku fits a small mechanical check (a byte comparison, a narrow diff).

## Goal now
- Do the gate reviews that the other heads ask for; one verdict with all findings in one message.
- Open for the person (asked 2026-11-20): open the pull request of my branch? Start my queue?

## How a review runs here (tested 2026-11-20 and 2026-11-21)
- One detached worktree for each review (`git worktree add --detach ../wt-qa-<name> <commit>`); build
  the release binary before the agents start; each agent has its own `CARGO_TARGET_DIR` in scratch.
- For a refactor or a merge: build the base commit too, and ask for a byte comparison of the outputs
  (`cmp`). Cheap and strong; it passed each time.
- Data: the store moves during the day. Clone it one time (`GIT_LFS_SKIP_SMUDGE=1 git clone --depth 1
  https://github.com/louellet-git/fa-data /home/user/qa-fa-data`); for a second fixed commit use
  `git -C /home/user/qa-fa-data worktree add --detach <path> <commit>`. Name the store commit in each
  brief and verdict. A fetch that only adds files does not disturb a running review.
- Alpaca from a cloud session: dummy key variables give HTTP 401 (the proxy does not add the key id).
- SEC pages: a zero-padded CIK path gives HTTP 301; use `curl -L` or the CIK with no zeros, with the
  contact of `sec_contact.toml`. One request for one page is not a bulk download.
- In each brief: no download of many tickers (only the Head of Technology starts one), no network
  unless the request needs it.
- An agent's verdict can disagree with its own levels (PASS-WITH-NOTES with a MAJ). Set the verdict
  from the levels, after "Proportion".

## Review patterns
- Tail of a model change: a rule right for the median can be wrong for the tail (ROP P50 $441 to -$88).
  Ask for the tail class and the count of P50 <= 0.
- A record that rejects alternatives with numbers: ask the validator to run each alternative
  (accuracy review: -2.9% stated, -0.4% run).
- A backtest: ask for a poison test of each data source it reads, not only the prices (the SEC
  restatements entered value-rank). A large change of a result after a fix: isolate code against data.
- A fit or calibration: ask for the convergence status and the parameters on a bound (vol-1).
- A findings file: check the statistic behind a headline number (the value at the means is not the
  mean; market-implied review).
- A new mapping tag that is a fallback: check non-target companies and old periods, not only the
  latest year (the status check sees the latest year only; mapping review, 2026-11-21).
- QA agents often rate wording, a document conflict or a text claim MAJ. Apply "Proportion": MIN when
  no value, data or trade can change. Each time I changed a level, I said so in the verdict.
- Precedent: in cycle 2, a limit that is measured and written in the record passes, with a ticket as
  a condition (no third cycle). The automatic model is a screen.
- A valuation report is not limited to two cycles: step 2 repeats until the skeptic accepts (`CLAUDE.md`).
- A valuation with a charge for future spending: check that the frames agree (an asset held at full value
  and each future purchase a total loss is two frames; NVDA round 2). Ask for the range across frames.
- Haiku did well on narrow checks with a fixed list (byte comparisons, a 5-point diff check); it found
  the same defect as the Sonnet validator in vol-1 cycle 2. Keep Sonnet for open attacks.

## Dream pass (before a compaction, above about 75% in `get_session`, or on "dream")
Last pass: 2026-11-21 01:25 UTC (context 81%); first pass 00:30 UTC.
