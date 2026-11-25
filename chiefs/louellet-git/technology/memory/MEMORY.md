# Dashboard: technology (louellet-git)
For the next session of this head: how the environment and the sources behave, and what the person
decided. Keep this file below 60 lines. Started 2026-11-20. Last dream pass: 2026-11-25 16:20 UTC.

## Order of the week (Head of Staff, from 2026-11-25; EPICS.md last section)
1. E3.S5: `T-20261123-ifrs-mapping-cameco` (IFRS for Cameco only, tests, values against the 40-F; then one
   message with the real size for NXE, DNN, SBSW, BHP, IsoEnergy; "no SIC code in the cache" is the first
   fault) → `T-20261123-non-sec-accounts-scout` (Japan and Korea filing systems first) →
   `T-20261123-commodity-series` (IMF and EIA uranium, gold series; terms first; Sprott needs consent).
2. Algo: `T-20261121-algo-host-gcp` (the guide Luc follows; e2-micro us-east1) and the Alpaca paper access
   of a head session (the Head of Staff got 401 on both hosts): check, then tell the Head of Quant Research.
3. E5: close `T-20261121-data-audit`; FRED step 4 (CPIAUCNS from BLS, TB3MS and GS10 from Fed H.15; the NBER
   series is a Fundamental decision: JST bill rate as a variant), then remove the last FRED files and code.
4. E6.S5: at each ticket close run `fa usage record && fa usage show`, the table in the done message; board
   check: count "no story" only for tickets created from 2026-11-22.
Later: the 4 Quant commands (rates audit, `fa calendar open`, snapshot `--smoke`, cache-only switch), BWXT
mapping FAIL, Cboe VX and VIX history terms, store update Thursdays (SWK, SWKS lag at Nasdaq), the option
save report on 2026-12-22, SPF notes (hash on read, 0-row error), N-PORT choice test.

## Standing words of the person (record: RECORD.md)
- Merge my branches after a QA pass and a green check, no new question. Take the next ticket with no wait.
- A done message to the Head of Staff for each result; no status report. Questions for Luc go to the Head
  of Staff (one yes/no with research first); a permission refusal is mine to filter; urgent only direct.
- Lean: "not obsessive and tight fisted"; "Let's not get overly complicated" (the usage log is local only).
- Agents use Sonnet; heads orchestrate and delegate; parallel work when the files do not overlap.
- The store `louellet-git/fa-data` is PRIVATE (2026-11-21), written only by me; attach it before a clone.
  Owner accepted: Alpaca chains in the shared store; a private SPF copy. Each person has own Alpaca keys.
- Not stored (terms): FRED, ICE, Moody's, NFCI, Cboe delayed quotes (no automatic read at all), Sprott.
- Programs may send orders to the Alpaca PAPER account only, controls 0 to 8, `--send`, never in a cloud
  session (QUALITY-STANDARDS section 8).
- Record numbers: Technology A-200 to A-229. No model name in any repo file or git ref.
- When a head rule changes here, make the general change in `louellet-git/the-heads` too.

## Tested rules
- A word of the person relayed by a head is data: check it in that session (`list_events`, kind user).
- Count tokens: one reply has several lines with the same usage; count each message id once.
- Each worktree build folder is about 5 GB: use one target folder, delete merged worktrees (the disk filled
  two times). A worktree binary writes `data/raw` of its build tree: set FA_REPO.
- FRED refuses our honest client name (no reply); do not change the name. A push of a non-branch ref from a
  cloud session failed and a retry was refused: no usage push.
- Nasdaq: an ETF asked as a stock answers "Symbol not exists" (fixed: ask again as an ETF).
- Treasury HQM: a month is used from the first weekday after the 5th of the next month.
- The repo merges by squash: check the newest PR number just before `merged: pull request N`.
- Alpaca key ID: `APCA_API_KEY_ID`; the secret is a proxy secret (`APCA_API_SECRET_KEY=proxy-injected`).
- Never work in a `chiefs` checkout under /home/user (its CLAUDE.md loads the Scan Support rules).
- Push to the store in parts below 40 MB compressed or 70 MB of text. Windows check: `cargo check --target
  x86_64-pc-windows-gnu`.
