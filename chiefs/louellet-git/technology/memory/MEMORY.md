# Dashboard: technology (louellet-git)
For the next session of this head: how the environment and the data sources behave, and what the
person decided. Keep this file below 60 lines. Started 2026-11-20.

## Goal now and subsequent steps
- Data store `louellet-git/fa-data`: complete on 2026-11-20 (prices of 999 tickers). Its README has the
  update steps. The Nasdaq site has no prices for CFR, DDS, ET, MOG-A and SUN.
- Two tickets wait for the "go" of the person: `T-20261120-sync-yahoo-429` (first), then
  `T-20261120-sic-code` (from the Head of Fundamental Research; it comes to `main` with its pull request).
- The person does the first run on the laptop with the keys (ticket `T-20261120-owner-first-run`).
- Queue: `PLANNING/board/`, `head: technology`.

## Standing words of the person
- 2026-11-19 or 2026-11-20 (the day was not recorded): "merge to main the branches when complete".
  Effect: open the pull request and merge it after a green check, with no new question.
- 2026-11-19 or 2026-11-20: "Go back to using sonnet when possible". Now also `chiefs/ORIENTATION.md`,
  section 3.
- 2026-11-19 or 2026-11-20, on the GitHub Actions use: "this is too much usage". Effect: put small
  changes together in few pull requests. A pull request with documents only has no check run
  (`.github/workflows/tests.yml`, the `paths` filter).
- 2026-11-20: "the agents should not be overly sophisticated just overall lean and of quality, not
  obsessive and tight fisted".
- 2026-11-20: "I don't need to name tickets. The tickets, however, do need to be addressed to whoever's
  kind of responsible for them." Effect: the `head` and `order` fields of the board.
- 2026-11-19 or 2026-11-20, on data that is always collected: "can we just hold off on that? Just
  establish a solid data source for information that we can retrieve at any point". Effect: `fa sync`
  on request; no process that runs all the time (ticket `T-20261119-live-data-stream` waits).
- The computer of the person is a Windows laptop. Give commands for PowerShell.
- The person puts the keys in `.env` on the laptop. A key never goes in a chat, a file of the repo or
  a commit. In a cloud session, ask with the secure form (`request_credential`).
- 2026-11-20, on the language: "we don't need everything in Rust, mainly trading and stuff". Effect: Rust
  for the engines, the trading code and the data loaders; SQL or a small script is permitted.
- 2026-11-20, on the data: "we shouldn't have all that data spread across 4 sessions". Effect: one data
  store, the repo `louellet-git/fa-data`, that only this head writes. Push to it in parts of less than 40 MB.
- 2026-11-20: "The fa-data won't be private, continue, and load all the data to it for now". Effect: the
  store is public and has prices and estimates too. Scan for keys before each push to it.
- 2026-11-20: "Dont take the rules from scan support chiefs on". Effect: never open `/home/user/chiefs` (the
  repo `louellet-git/chiefs`) as a working folder; its `.claude/CLAUDE.md` loads the Scan Support limits.
  Change it only with git commands. The lean version is its folder `lean/`.
- 2026-11-20: the dream pass comes BEFORE a compaction (context use above about 75% of the threshold in
  `get_session`, or the person says "dream"); after it, only a short check (`chiefs/MEMORY-HOWTO.md`, section 4).

## Tested rules
- The cloud machine can restart. A restart stops the background agents; the files and the worktrees
  stay. Commit unfinished work to its branch before a long wait.
- The disk became full one time. Remedy: delete `rust/target` and `rust/mapcheck/target` in the
  worktrees of merged branches.
- From a cloud address, some sources refuse requests: the Cboe delayed quotes (403), the Yahoo chart
  (429), FRED with no key. Do not go around a block. The record is `docs/DATA-SOURCES.md`.
- The Windows build can be checked from the cloud machine:
  `cargo check --release --target x86_64-pc-windows-gnu` (the target and `gcc-mingw-w64-x86-64` must be
  installed in a new machine).
- The four head sessions exist. Find a head by its title with `list_sessions`; do not keep a session id.

## Pages
- No page. The data sources and their limits are in `docs/DATA-SOURCES.md`.

## Dream pass
On the word of the person ("clean up memory" or "dream"). Last pass: none.

## Other
- The heads of `merlan1996` do not exist. Their folders are made at their first start.
