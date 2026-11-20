# Dashboard: quality (louellet-git)
What the next session of this head must know. Below 60 lines. Started 2026-11-20.
Form and purpose: `chiefs/MEMORY-HOWTO.md`. Memory of this head: the patterns of the reviews.

## Standing words of the person
- 2026-11-20 21:44 UTC: balance realistic risk with potential risk. Internal processes and
  internal documents: do not be constricting and do not slow the work. Risk in trades: strict.
  (`RECORD.md`; rule in `docs/QUALITY-STANDARDS.md`, section 3, "Proportion".)
- 2026-11-20: the agents are lean and of good quality, "not obsessive and tight fisted".
- GitHub Actions cost (date not recorded, 2026-11-19 or 2026-11-20): few pull requests; a pull
  request with documents only has no check run. Rules: `chiefs/ORIENTATION.md`, section 6.
- The computer of the person is a Windows laptop: give commands for PowerShell.
- 2026-11-20 23:22 UTC: choose the model of each agent run for the job (permission, not a duty); the files
  stay Sonnet. Today: the validator, auditor and code reviewer found each MAJ (keep Sonnet for that work);
  format, data checks and test plans found MIN only (Haiku fits a small job of that kind).

## Goal now
- Do the gate reviews that the other heads ask for; send one verdict with all findings in one
  message to the author. Then the queue (`head: quality`, lowest `order` first).

## How a review runs here (tested 2026-11-20)
- One detached worktree for each review: `git worktree add --detach ../wt-qa-<name> <commit>`.
  Build the release binary there before the agents start; give each agent its own
  `CARGO_TARGET_DIR` in the scratch folder, so that two agents do not rebuild one binary.
- Data: clone the store one time to `/home/user/qa-fa-data`
  (`GIT_LFS_SKIP_SMUDGE=1 git clone --depth 1 https://github.com/louellet-git/fa-data <path>`,
  1.5 GB) and link it as `data/raw` of the worktree. Agents read it and never write it.
- The store has no SIC code in the 505 earnings list files (until the next `fa sync`), and no
  price file for CFR, DDS, ET, MOG-A, SUN. Tell the agents: these are not new findings.
- Alpaca from a cloud session: dummy key variables give HTTP 401 (the proxy does not add the key id; 2026-11-20). A real Alpaca run needs the person or a key.
- Write in each brief that the agent must not download many tickers (only the Head of Technology
  starts that). A gate request can ask for such a run; do not give it to an agent.

## Review patterns
- 2026-11-20 conventions review: a charge or a rule that is right for the median company can be wrong for the tail (heavy buyers: ROP P50 $441 to -$88). Ask for the tail class and the count of P50 <= 0 in each model-change brief. It found the MAJ; cycle 2 fixed 11 of 13.
- 2026-11-20 accuracy review: a record defended a default with numbers for the rejected alternatives,
  and those numbers did not reproduce (-2.9% stated, -0.4% run). Ask the validator to run each
  rejected alternative of a record.
- Precedent, 2026-11-20: in cycle 2, a remaining limit that is measured and written in the record
  passes, with a ticket as a condition (no third cycle). The automatic model is a screen.

## Pages
- No page.

## Dream pass
Before a compaction (above about 75% of the threshold in `get_session`) or on "dream".
Last pass: none.
