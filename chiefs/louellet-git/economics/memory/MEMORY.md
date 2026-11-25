# Dashboard: economics (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Updated 2026-11-25.
Rules: `chiefs/ORIENTATION.md`, section 8. Decisions of the person: `../RECORD.md`.

## Goal now (week of 2026-11-25; order: PLANNING/board/EPICS.md, last section)
- E2.S3 runs, no new design: (1) premium test run of record: `fa macro premium-test` (main, #175).
  Input = Fundamental Research bridge file, branch claude/premium-series-bridge-record 098e552 (PR 214),
  analysis/priors/data/premium_series_sp500_2019_bridge_record_main-6c59388/sp500_bridge.csv; code main
  6c59388, store 0340bc1. Before the run: add the FR record to section 6 of the premium design; DFII10/T10YIE
  from `fa rates derived --out DIR`; VIX from the store. Limits: parameters in-sample, today's names, PR 200
  revenue-tag fix. (2) First real run of `fa macro view` (#191) after the FR reader
  (`fa dcf attribute --common file --common-file FILE`) passes its gate; windows from
  `fa dcf premium-series --windows-out FILE`.
- JST (design 31ed025): record R1 before any score. Local data /home/user/jst-local (never in a repo).
- E3.S5 (Economics part): one command for the period table and the weekly correlations of any fund
  (URA/GLD limit g/d), and a news-day test. After the runs above.
- Keep a reserve for Luc's questions (three reports in two days last week). Pace about 12% of the week a day.
- At the close of each ticket: `fa usage record && fa usage show`; put the table in the done message (E6.S5).

## Done on main (2026-11-21 to 11-23)
- Designs/commands: view design #153, premium test #175, view command #191; briefs #116 #118 #207;
  notes for Luc #193 (rate rise, daily view, chip fall), AI build-out report #216, URA and GLD pages #221.
- URA N-PORT update (holdings dated, limit e done): branch econ/ura-nport.

## Team and routing
- Agents: macro-economist, market-historian, rates-credit-quant (Rust), model-documenter, rust-engineer,
  auditor; general-purpose or web-fetch for web research ("pages only, no data files").
- Head of Staff (session FA-L - Head of Staff): a done message for each result; anything for Luc goes there first.
- Quality and Risk gates each document for Luc (auditor + document reviewer; risk-compliance for a conflict
  of interest such as Anthropic). Technology alone writes the data store; ask it for any missing series.

## Tested rules
- No FRED copies in the store (FRED terms); real yields from the Treasury real curve (store has 2023-2026).
  NFCI and BAA10Y are excluded (terms). SPF: private copy only. Cboe: no automatic reads.
- Owner report style: chiefs/louellet-git/staff/memory/owner-report-style.md (title, subtitle, answer first,
  tables with a "See" line, "consistent with", a limits table with owner and date that match the board).
- Mark each press fact we did not read at the primary source "secondary"; give the week rule of each
  correlation (ISO week, all series present, which weeks a year has).
- Readings, not claims: "the data fits ..."; no forecast or trade view.
- PDFs: pandoc + playwright (scratchpad r1.css, Letter, 0.4in/0.5in margins).
- `data/raw` as a symlink breaks git (.gitkeep): pass --cache /home/user/fa-data-ro instead.
- Push to an open PR only to correct it; batch small document changes (owner pays for CI).
- The head merges its own PR after the gate and a green check.
